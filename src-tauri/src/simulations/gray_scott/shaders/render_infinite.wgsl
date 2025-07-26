struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) grid_fade_factor: f32,
};

@group(0) @binding(0)
var<storage, read> simulation_data: array<UVPair>;
@group(0) @binding(1)
var<storage, read> lut_data: array<u32>;
@group(0) @binding(2)
var<uniform> params: SimulationParams;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

// Calculate how many tiles we need based on zoom level
fn calculate_tile_count(zoom: f32) -> i32 {
    // At zoom 1.0, we need at least 5x5 tiles
    // As zoom decreases (zooming out), we need more tiles
    // Each tile covers 2.0 world units, so we need enough tiles to cover the visible area
    let visible_world_size = 2.0 / zoom; // World size visible on screen
    let tiles_needed = i32(ceil(visible_world_size / 2.0)) + 6; // +6 for extra padding at extreme zoom levels
    let min_tiles = select(5, 7, zoom < 0.1); // More tiles needed at extreme zoom out
    // Allow more tiles for proper infinite tiling, but cap at reasonable limit
    return min(max(tiles_needed, min_tiles), 200); // Cap at 200x200 for performance
}

// Calculate the starting tile offset based on camera position
fn calculate_tile_start(camera_pos: vec2<f32>, zoom: f32) -> vec2<i32> {
    // Each tile is 2.0 world units, so divide camera position by 2.0 to get tile coordinates
    // Use round instead of floor for better centering
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
// When zoomed out too far, tiles become too small to render individually
// and should fade to the average color of the simulation
fn calculate_fade_factor(zoom: f32) -> f32 {
    // Start fading when zoom gets below 0.05
    // Complete fade when zoom gets below 0.005
    let fade_start = 0.05;
    let fade_end = 0.005;
    
    if (zoom >= fade_start) {
        return 1.0; // Full opacity
    } else if (zoom <= fade_end) {
        return 0.0; // Complete fade to average
    } else {
        // Smooth transition between fade_start and fade_end
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
    // Create a quad that covers the full screen area
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
    
    // Calculate dynamic grid size based on zoom
    let tile_count = calculate_tile_count(camera.zoom);
    let tile_start = calculate_tile_start(camera.position, camera.zoom);
    
    // Calculate grid position for this instance
    let grid_x = i32(instance_index % u32(tile_count)) + tile_start.x;
    let grid_y = i32(instance_index / u32(tile_count)) + tile_start.y;
    
    // Calculate fade factor based on zoom level
    let grid_fade_factor = calculate_fade_factor(camera.zoom);
    
    // Calculate world position for this tile
    var world_pos = vec2<f32>(
        pos[vertex_index].x + f32(grid_x) * 2.0, // Each tile is 2.0 world units
        pos[vertex_index].y + f32(grid_y) * 2.0
    );
    
    var out: VertexOutput;
    out.position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = uv[vertex_index];
    out.grid_fade_factor = grid_fade_factor;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert UV coordinates to texture coordinates
    let tex_x = u32(in.uv.x * f32(params.width));
    let tex_y = u32(in.uv.y * f32(params.height));
    let index = tex_y * params.width + tex_x;
    
    // Sample simulation data
    let uv_pair = simulation_data[index];
    let u = uv_pair.u;
    let v = uv_pair.v;
    
    // Apply LUT to get color
    // LUT data is in planar format: [r0...r255, g0...g255, b0...b255]
    let lut_index = u32(clamp(u * 255.0, 0.0, 255.0));
    let r = f32(lut_data[lut_index]) / 255.0;           // Red channel: indices 0-255
    let g = f32(lut_data[lut_index + 256u]) / 255.0;    // Green channel: indices 256-511
    let b = f32(lut_data[lut_index + 512u]) / 255.0;    // Blue channel: indices 512-767
    let a = 1.0;
    
    let base_color = vec4<f32>(r, g, b, a);
    
    // When completely faded (grid_fade_factor = 0), render a solid color
    // representing the average of the simulation
    if (in.grid_fade_factor <= 0.0) {
        // Use a dark color that represents the "average" when tiles are too small
        // This gives a sense of the overall simulation state
        return vec4<f32>(0.1, 0.1, 0.15, 1.0);
    }
    
    // Apply grid fade factor to create smooth transition
    let final_color = vec4<f32>(base_color.rgb, base_color.a * in.grid_fade_factor);
    
    // Discard completely transparent pixels for performance
    if (final_color.a <= 0.0) {
        discard;
    }
    
    return final_color;
} 