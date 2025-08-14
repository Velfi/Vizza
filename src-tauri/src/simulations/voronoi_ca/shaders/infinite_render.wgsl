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

// VCA-specific texture-based binding
@group(0) @binding(0)
var display_tex: texture_2d<f32>;
@group(0) @binding(1)
var display_sampler: sampler;

// Render parameters for texture-based simulations
@group(0) @binding(2)
var<uniform> texture_render_params: RenderParams;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

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

// Fragment shader for VCA texture-based rendering
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
