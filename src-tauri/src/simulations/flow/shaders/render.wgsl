struct Particle {
    position: vec2<f32>,
    age: f32,
    speed: f32,
    color: vec4<f32>,
}

struct Camera {
    position: vec2<f32>,
    zoom: f32,
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var<storage, read> lut_data: array<u32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
    @location(1) particle_index: f32,
    @location(2) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let particle = particles[instance_index];
    
    // Generate quad vertices
    let quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), // Bottom-left
        vec2<f32>( 1.0, -1.0), // Bottom-right
        vec2<f32>(-1.0,  1.0), // Top-left
        vec2<f32>(-1.0,  1.0), // Top-left
        vec2<f32>( 1.0, -1.0), // Bottom-right
        vec2<f32>( 1.0,  1.0)  // Top-right
    );
    
    let vertex_pos = quad_vertices[vertex_index];
    
    // Transform to world space
    let world_pos = particle.position + vertex_pos * 0.01; // Small particle size
    
    // Transform to screen space
    let screen_pos = (world_pos - camera.position) * camera.zoom;
    let clip_pos = vec4<f32>(screen_pos, 0.0, 1.0);
    
    return VertexOutput(
        clip_pos,
        world_pos,
        f32(instance_index),
        particle.color
    );
}

@fragment
fn fs_main(@location(0) world_pos: vec2<f32>, @location(1) particle_index: f32, @location(2) color: vec4<f32>) -> @location(0) vec4<f32> {
    // Simple circular particle
    let center = vec2<f32>(0.0, 0.0);
    let radius = 0.01;
    let dist = length(world_pos - center);
    
    if (dist > radius) {
        discard;
    }
    
    // Fade out towards edges
    let fade = 1.0 - smoothstep(0.0, radius, dist);
    
    return vec4<f32>(color.rgb, color.a * fade);
} 