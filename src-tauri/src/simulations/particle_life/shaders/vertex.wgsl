// Particle Life vertex shader - Instanced Quads for Sizable Points

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
    particle_size: f32, // Add particle size parameter
    _pad1: u32,
    _pad2: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) species: u32,
    @location(1) velocity_magnitude: f32,
    @location(2) world_pos: vec2<f32>,
    @location(3) grid_fade_factor: f32,
    @location(4) uv: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> sim_params: SimParams;

// Instanced quad vertex shader for sizable particles
@vertex
fn main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Create a quad for each particle (6 vertices = 2 triangles)
    let quad_positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    
    let quad_uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    
    let particle = particles[instance_index];
    let quad_pos = quad_positions[vertex_index];
    let quad_uv = quad_uvs[vertex_index];
    
    // Convert particle position from [-1,1] to [0,1] texture space, then to clip space
    let normalized_pos = (particle.position + vec2<f32>(1.0)) * 0.5;
    
    // Use zoom-aware particle size instead of fixed pixel size
    // The particle_size parameter should be in world space units and will scale with zoom
    let particle_size = sim_params.particle_size;
    let quad_offset = quad_pos * particle_size / vec2<f32>(sim_params.width, sim_params.height);
    let final_pos = normalized_pos + quad_offset;
    
    // Convert to clip space [-1,1]
    let clip_pos = final_pos * 2.0 - vec2<f32>(1.0);
    
    var output: VertexOutput;
    output.position = vec4<f32>(clip_pos, 0.0, 1.0);
    output.species = particle.species;
    output.velocity_magnitude = length(particle.velocity);
    output.world_pos = particle.position;
    output.grid_fade_factor = 1.0;
    output.uv = quad_uv;
    
    return output;
}