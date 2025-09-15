struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Standard texture-based binding (used by Flow, Pellets, Slime Mold)
@group(0) @binding(0)
var display_tex: texture_2d<f32>;
@group(0) @binding(1)
var display_sampler: sampler;

// Render parameters for texture-based simulations
@group(0) @binding(2)
var<uniform> texture_render_params: RenderParams;



// Gray Scott uses textures instead of storage buffers
@group(0) @binding(3)
var simulation_data: texture_2d<f32>;
@group(0) @binding(4)
var simulation_sampler: sampler;
@group(0) @binding(5)
var<storage, read> lut_data: array<u32>;
@group(0) @binding(6)
var<uniform> params: SimulationParams;

@group(0) @binding(7)
var<uniform> render_params: RenderParams;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

// Gray Scott specific structures
struct SimulationParams {
    feed_rate: f32,
    kill_rate: f32,
    delta_u: f32,
    delta_v: f32,
    timestep: f32,
    width: u32,
    height: u32,
    nutrient_pattern: u32,
    is_nutrient_pattern_reversed: u32,
    cursor_x: f32,
    cursor_y: f32,
    cursor_size: f32,
    cursor_strength: f32,
}

struct UVPair {
    u: f32,
    v: f32,
}

struct RenderParams {
    filtering_mode: u32, // 0 = nearest, 1 = linear, 2 = lanczos
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

// Calculate how many tiles we need based on zoom level
fn calculate_tile_count(zoom: f32) -> i32 {
    let visible_world_size = 2.0 / zoom;
    // Match the Rust-side calculation exactly
    let tiles_needed = i32(ceil(visible_world_size / 2.0)) + 6;
    let min_tiles = select(5, 7, zoom < 0.1);
    return min(max(tiles_needed, min_tiles), 1024);
}

// Calculate the starting tile offset based on camera position
fn calculate_tile_start(camera_pos: vec2<f32>, zoom: f32) -> vec2<i32> {
    let tile_center = vec2<i32>(
        i32(round(camera_pos.x / 2.0)),
        i32(round(camera_pos.y / 2.0))
    );
    
    let tile_count = calculate_tile_count(zoom);
    let half_tiles = tile_count / 2;
    
    return vec2<i32>(
        tile_center.x - half_tiles,
        tile_center.y - half_tiles
    );
}



// Vertex shader for infinite instanced rendering
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    
    let tile_count = calculate_tile_count(camera.zoom);
    let tile_start = calculate_tile_start(camera.position, camera.zoom);
    
    let grid_x = i32(instance_index % u32(tile_count)) + tile_start.x;
    let grid_y = i32(instance_index / u32(tile_count)) + tile_start.y;
    
    var world_pos = vec2<f32>(
        pos[vertex_index].x + f32(grid_x) * 2.0,
        pos[vertex_index].y + f32(grid_y) * 2.0
    );
    
    var out: VertexOutput;
    out.position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = uv[vertex_index];
    return out;
}

// Fragment shader for texture-based simulations (Flow, Pellets, Slime Mold)
@fragment
fn fs_main_texture(in: VertexOutput) -> @location(0) vec4<f32> {
    var base_color: vec4<f32>;
    
    if (texture_render_params.filtering_mode == 0u) {
        // Nearest neighbor
        base_color = textureSample(display_tex, display_sampler, in.uv);
    } else if (texture_render_params.filtering_mode == 1u) {
        // Linear (bilinear interpolation)
        base_color = textureSample(display_tex, display_sampler, in.uv);
    } else {
        // Lanczos filtering
        let tex_dims = textureDimensions(display_tex);
        let tex_x = in.uv.x * f32(tex_dims.x);
        let tex_y = in.uv.y * f32(tex_dims.y);
        
        let lanczos_a = 2.0; // Lanczos window size
        let radius = i32(lanczos_a);
        
        var color_sum = vec4<f32>(0.0);
        var weight_sum = 0.0;
        
        // Sample in a radius around the target pixel
        for (var dy = -radius; dy <= radius; dy++) {
            for (var dx = -radius; dx <= radius; dx++) {
                let sample_x = i32(floor(tex_x)) + dx;
                let sample_y = i32(floor(tex_y)) + dy;
                
                // Clamp to texture bounds
                let clamped_x = u32(clamp(f32(sample_x), 0.0, f32(tex_dims.x - 1u)));
                let clamped_y = u32(clamp(f32(sample_y), 0.0, f32(tex_dims.y - 1u)));
                
                // Calculate distance from target pixel
                let dist_x = (f32(sample_x) - tex_x);
                let dist_y = (f32(sample_y) - tex_y);
                
                // Calculate Lanczos weights
                let weight_x = lanczos_kernel(dist_x, lanczos_a);
                let weight_y = lanczos_kernel(dist_y, lanczos_a);
                let weight = weight_x * weight_y;
                
                // Sample the pixel
                let sample_uv = vec2<f32>(
                    f32(clamped_x) / f32(tex_dims.x),
                    f32(clamped_y) / f32(tex_dims.y)
                );
                let sample_color = textureSample(display_tex, display_sampler, sample_uv);
                
                color_sum += sample_color * weight;
                weight_sum += weight;
            }
        }
        
        // Normalize by total weight
        base_color = color_sum / weight_sum;
    }
    
    // Don't discard transparent pixels, just return them as-is
    // This allows the background to show through transparent areas
    return base_color;
}

// Lanczos kernel function
fn lanczos_kernel(x: f32, a: f32) -> f32 {
    if (abs(x) >= a) {
        return 0.0;
    }
    if (abs(x) < 0.0001) {
        return 1.0;
    }
    let x_pi = x * 3.14159265359;
    let x_pi_a = x_pi / a;
    return (sin(x_pi) * sin(x_pi_a)) / (x_pi * x_pi_a);
}

// Convert from sRGB (gamma-corrected) to linear RGB
fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

// Fragment shader for texture-based simulations (Gray Scott)
fn fs_main_storage(in: VertexOutput) -> vec4<f32> {
    var u_interpolated: f32;
    
    if (render_params.filtering_mode == 0u) {
        // Nearest neighbor
        let sample = textureSample(simulation_data, simulation_sampler, in.uv);
        u_interpolated = sample.x; // R channel contains u value
    } else if (render_params.filtering_mode == 1u) {
        // Linear (bilinear interpolation)
        let sample = textureSample(simulation_data, simulation_sampler, in.uv);
        u_interpolated = sample.x; // R channel contains u value
    } else {
        // Lanczos filtering
        let tex_dims = textureDimensions(simulation_data);
        let tex_x = in.uv.x * f32(tex_dims.x);
        let tex_y = in.uv.y * f32(tex_dims.y);
        
        let lanczos_a = 2.0; // Lanczos window size
        let radius = i32(lanczos_a);
        
        var u_sum = 0.0;
        var weight_sum = 0.0;
        
        // Sample in a radius around the target pixel
        for (var dy = -radius; dy <= radius; dy++) {
            for (var dx = -radius; dx <= radius; dx++) {
                let sample_x = i32(floor(tex_x)) + dx;
                let sample_y = i32(floor(tex_y)) + dy;
                
                // Clamp to texture bounds
                let clamped_x = u32(clamp(f32(sample_x), 0.0, f32(tex_dims.x - 1u)));
                let clamped_y = u32(clamp(f32(sample_y), 0.0, f32(tex_dims.y - 1u)));
                
                // Calculate distance from target pixel
                let dist_x = (f32(sample_x) - tex_x);
                let dist_y = (f32(sample_y) - tex_y);
                
                // Calculate Lanczos weights
                let weight_x = lanczos_kernel(dist_x, lanczos_a);
                let weight_y = lanczos_kernel(dist_y, lanczos_a);
                let weight = weight_x * weight_y;
                
                // Sample the pixel
                let sample_uv = vec2<f32>(
                    f32(clamped_x) / f32(tex_dims.x),
                    f32(clamped_y) / f32(tex_dims.y)
                );
                let sample = textureSample(simulation_data, simulation_sampler, sample_uv);
                let u = sample.x; // R channel contains u value
                
                u_sum += u * weight;
                weight_sum += weight;
            }
        }
        
        // Normalize by total weight
        u_interpolated = u_sum / weight_sum;
    }
    
    // Use interpolated u value for LUT lookup
    let lut_index = u32(clamp(u_interpolated * 255.0, 0.0, 255.0));
    let r_srgb = f32(lut_data[lut_index]) / 255.0;
    let g_srgb = f32(lut_data[lut_index + 256u]) / 255.0;
    let b_srgb = f32(lut_data[lut_index + 512u]) / 255.0;
    
    // Convert from sRGB to linear RGB
    let r = srgb_to_linear(r_srgb);
    let g = srgb_to_linear(g_srgb);
    let b = srgb_to_linear(b_srgb);
    let a = 1.0;
    
    let base_color = vec4<f32>(r, g, b, a);
    
    if (base_color.a <= 0.0) {
        discard;
    }
    
    return base_color;
}

// Main fragment entry point for Gray-Scott (calls storage version)
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return fs_main_storage(in);
} 