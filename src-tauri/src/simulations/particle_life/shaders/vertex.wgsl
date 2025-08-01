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
    aspect_ratio: f32,  // Screen aspect ratio for cursor distance calculation
    _pad1: u32,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct ViewportParams {
    world_bounds: vec4<f32>, // [left, bottom, right, top] in world coordinates
    texture_size: vec2<f32>, // [width, height] of offscreen texture
    _pad1: f32,
    _pad2: f32,
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
@group(2) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(1) var<uniform> viewport_params: ViewportParams;

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
    
    // Map particles from world space to the texture's world bounds
    let world_particle_pos = particle.position;
    
    // Get the world bounds that this texture represents
    let world_left = viewport_params.world_bounds.x;
    let world_bottom = viewport_params.world_bounds.y;
    let world_right = viewport_params.world_bounds.z;
    let world_top = viewport_params.world_bounds.w;
    
    // Map world position to texture NDC space [-1, 1]
    let texture_ndc_x = (world_particle_pos.x - world_left) / (world_right - world_left) * 2.0 - 1.0;
    let texture_ndc_y = (world_particle_pos.y - world_bottom) / (world_top - world_bottom) * 2.0 - 1.0;
    let particle_ndc_pos = vec2<f32>(texture_ndc_x, texture_ndc_y);
    
    // Scale particle size from world units to NDC units
    let world_width = world_right - world_left;
    let world_height = world_top - world_bottom;
    let world_scale = min(world_width, world_height) / 2.0; // Scale factor from world to NDC
    let particle_ndc_size = sim_params.particle_size / world_scale * 0.001; // Make particles 0.001 of current size
    
    // Apply aspect ratio correction
    let aspect_corrected_quad = vec2<f32>(quad_pos.x / camera.aspect_ratio, quad_pos.y);
    let quad_offset = aspect_corrected_quad * particle_ndc_size;
    let final_pos = particle_ndc_pos + quad_offset;
    
    var output: VertexOutput;
    output.position = vec4<f32>(final_pos, 0.0, 1.0);  // Output in texture NDC space
    output.species = particle.species;
    output.velocity_magnitude = length(particle.velocity);
    output.world_pos = world_particle_pos;  // Pass world position for post-processing
    output.grid_fade_factor = 1.0;
    output.uv = quad_uv;
    
    return output;
}