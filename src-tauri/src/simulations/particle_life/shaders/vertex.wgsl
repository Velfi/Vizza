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
    repulsion_min_distance: f32,
    repulsion_medium_distance: f32,
    repulsion_extreme_strength: f32,
    repulsion_linear_strength: f32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,  // Added to make struct 72 bytes (18 * 4)
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
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> sim_params: SimParams;
@group(2) @binding(0) var<uniform> camera: CameraUniform;

// Vertex shader for instanced particle rendering
@vertex
fn main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let particle = particles[instance_index];
    
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
    
    // Convert particle world position (-2.0 to 2.0) to normalized device coordinates [-1,1]
    // World coordinates are -2.0 to 2.0, so we need to map them to -1 to 1
    let world_pos = vec2<f32>(
        particle.position.x / 2.0, // -2.0 to 2.0 -> -1.0 to 1.0
        particle.position.y / 2.0  // -2.0 to 2.0 -> -1.0 to 1.0
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
    
    return output;
}