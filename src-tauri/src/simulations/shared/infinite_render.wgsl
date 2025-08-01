struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) grid_fade_factor: f32,
};

// Standard texture-based binding (used by Flow, Pellets, Slime Mold)
@group(0) @binding(0)
var display_tex: texture_2d<f32>;
@group(0) @binding(1)
var display_sampler: sampler;

// Background color uniform for fade effect
@group(0) @binding(2)
var<uniform> background_color: vec4<f32>;

// Gray Scott uses storage buffers instead of textures
@group(0) @binding(3)
var<storage, read> simulation_data: array<UVPair>;
@group(0) @binding(4)
var<storage, read> lut_data: array<u32>;
@group(0) @binding(5)
var<uniform> params: SimulationParams;

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

// Calculate how many tiles we need based on zoom level
fn calculate_tile_count(zoom: f32) -> i32 {
    let visible_world_size = 2.0 / zoom;
    // Add more padding to prevent gaps between tiles
    let tiles_needed = i32(ceil(visible_world_size / 2.0)) + 8;
    let min_tiles = select(7, 9, zoom < 0.1);
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

// Calculate fade factor based on zoom level
fn calculate_fade_factor(zoom: f32) -> f32 {
    let fade_start = 0.05;
    let fade_end = 0.005;
    
    if (zoom >= fade_start) {
        return 1.0;
    } else if (zoom <= fade_end) {
        return 0.0;
    } else {
        let t = (zoom - fade_end) / (fade_start - fade_end);
        return t;
    }
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
    
    let grid_fade_factor = calculate_fade_factor(camera.zoom);
    
    var world_pos = vec2<f32>(
        pos[vertex_index].x + f32(grid_x) * 2.0,
        pos[vertex_index].y + f32(grid_y) * 2.0
    );
    
    var out: VertexOutput;
    out.position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = uv[vertex_index];
    out.grid_fade_factor = grid_fade_factor;
    return out;
}

// Fragment shader for texture-based simulations (Flow, Pellets, Slime Mold)
@fragment
fn fs_main_texture(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(display_tex, display_sampler, in.uv);
    
    if (in.grid_fade_factor <= 0.0) {
        return background_color;
    }
    
    let final_color = vec4<f32>(base_color.rgb, base_color.a * in.grid_fade_factor);
    
    if (final_color.a <= 0.0) {
        discard;
    }
    
    return final_color;
}

// Fragment shader for storage buffer-based simulations (Gray Scott)
@fragment
fn fs_main_storage(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_x = u32(in.uv.x * f32(params.width));
    let tex_y = u32(in.uv.y * f32(params.height));
    let index = tex_y * params.width + tex_x;
    
    let uv_pair = simulation_data[index];
    let u = uv_pair.u;
    let v = uv_pair.v;
    
    let lut_index = u32(clamp(u * 255.0, 0.0, 255.0));
    let r = f32(lut_data[lut_index]) / 255.0;
    let g = f32(lut_data[lut_index + 256u]) / 255.0;
    let b = f32(lut_data[lut_index + 512u]) / 255.0;
    let a = 1.0;
    
    let base_color = vec4<f32>(r, g, b, a);
    
    if (in.grid_fade_factor <= 0.0) {
        return background_color;
    }
    
    let final_color = vec4<f32>(base_color.rgb, base_color.a * in.grid_fade_factor);
    
    if (final_color.a <= 0.0) {
        discard;
    }
    
    return final_color;
} 