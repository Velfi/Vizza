// Particle Life vertex shader - Infinite Rendering

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    species: u32,
    _pad: u32,
}

struct SimParams {
    particle_count: u32,
    species_count: u32,
    max_force: f32,
    max_distance: f32,
    friction: f32,
    wrap_edges: u32,
    width: f32,
    height: f32,
    random_seed: u32,
    dt: f32,
    beta: f32,
    cursor_x: f32,
    cursor_y: f32,
    cursor_size: f32,
    cursor_strength: f32,
    cursor_active: u32,
    brownian_motion: f32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) species: u32,
    @location(1) velocity_magnitude: f32,
    @location(2) uv: vec2<f32>,
    @location(3) grid_fade_factor: f32,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> sim_params: SimParams;
@group(2) @binding(0) var<uniform> camera: CameraUniform;

// Calculate how many tiles we need based on zoom level
fn calculate_tile_count(zoom: f32) -> i32 {
    // At zoom 1.0, we need at least 5x5 tiles
    // As zoom decreases (zooming out), we need more tiles
    // Each tile covers 2.0 world units, so we need enough tiles to cover the visible area
    let visible_world_size = 2.0 / zoom; // World size visible on screen
    let tiles_needed = i32(ceil(visible_world_size / 2.0)) + 6; // +6 for extra padding at extreme zoom levels
    let min_tiles = select(5, 7, zoom < 0.1); // More tiles needed at extreme zoom out
    // Allow more tiles for proper infinite tiling, but cap at reasonable limit
    return min(max(tiles_needed, min_tiles), 1024); // Cap at 200x200 for performance
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

// Vertex shader for infinite instanced particle rendering
@vertex
fn main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Create a small quad for each particle (2 triangles = 6 vertices)
    let particle_size = 4.0; // Size in pixels
    
    var quad_positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    
    // UV coordinates for each quad vertex
    var quad_uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),  // Bottom-left
        vec2<f32>(1.0, 1.0),  // Bottom-right
        vec2<f32>(0.0, 0.0),  // Top-left
        vec2<f32>(1.0, 1.0),  // Bottom-right
        vec2<f32>(1.0, 0.0),  // Top-right
        vec2<f32>(0.0, 0.0),  // Top-left
    );
    
    // Calculate dynamic grid size based on zoom
    let tile_count = calculate_tile_count(camera.zoom);
    let tile_start = calculate_tile_start(camera.position, camera.zoom);
    
    // Calculate which particle and which tile this instance represents
    let particles_per_tile = sim_params.particle_count;
    let actual_particle_index = instance_index % particles_per_tile;
    let tile_index = instance_index / particles_per_tile;
    
    let particle = particles[actual_particle_index];
    
    // Calculate grid position for this tile
    let grid_x = i32(tile_index % u32(tile_count)) + tile_start.x;
    let grid_y = i32(tile_index / u32(tile_count)) + tile_start.y;
    
    // Calculate fade factor based on zoom level
    let grid_fade_factor = calculate_fade_factor(camera.zoom);
    
    // Calculate world position for this particle in this tile
    // Each tile is 2.0 world units, particle positions are in [-1,1]
    var world_pos = vec2<f32>(
        particle.position.x + f32(grid_x) * 2.0, // Offset by tile position
        particle.position.y + f32(grid_y) * 2.0
    );
    
    // Add particle quad offset in normalized space
    let quad_offset = quad_positions[vertex_index] * particle_size / vec2<f32>(sim_params.width, sim_params.height);
    let final_world_pos = world_pos + quad_offset;
    
    // Apply camera transformation
    let camera_pos = camera.transform_matrix * vec4<f32>(final_world_pos, 0.0, 1.0);
    
    var output: VertexOutput;
    output.position = camera_pos;
    output.species = particle.species;
    output.velocity_magnitude = length(particle.velocity);
    output.uv = quad_uvs[vertex_index];
    output.grid_fade_factor = grid_fade_factor;
    
    return output;
}