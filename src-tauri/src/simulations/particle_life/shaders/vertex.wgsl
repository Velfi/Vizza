// Particle Life vertex shader

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
    min_distance: f32,
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
    traces_enabled: u32,
    trace_fade: f32,
    edge_fade_strength: f32,
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

// Vertex shader for instanced particle rendering with 3x3 grid support
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
    
    var grid_fade_factor = 1.0; // Default to full opacity
    
    // Always use 3x3 grid mode: render each particle 9 times
    let particles_per_grid = sim_params.particle_count;
    let actual_particle_index = instance_index % particles_per_grid;
    let grid_cell_index = instance_index / particles_per_grid;
    
    let particle = particles[actual_particle_index];
    
    // Calculate grid cell position (0-8, arranged as 3x3 grid)
    let grid_x = i32(grid_cell_index % 3u) - 1; // -1, 0, 1
    let grid_y = i32(grid_cell_index / 3u) - 1; // -1, 0, 1
    
    // Calculate fade factor based on distance from center
    let center_distance = abs(grid_x) + abs(grid_y);
    if (center_distance == 0) {
        grid_fade_factor = 1.0; // Center cell - full opacity
    } else if (center_distance == 1) {
        grid_fade_factor = 0.4; // Adjacent cells - medium fade
    } else {
        grid_fade_factor = 0.2; // Corner cells - strong fade
    }
    
    // Start with particle world position [-1,1] and offset by grid cell
    // Each grid cell represents a full world tile offset (width/height = 2.0)
    var world_pos = vec2<f32>(
        particle.position.x + f32(grid_x) * 2.0, // Offset by full world width
        particle.position.y + f32(grid_y) * 2.0  // Offset by full world height
    );
    
    // world_pos is already in normalized device coordinates [-1,1]
    // No conversion needed
    
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