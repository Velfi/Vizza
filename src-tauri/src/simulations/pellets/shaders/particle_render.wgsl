struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    mass: f32,
    radius: f32,
    clump_id: u32,
    density: f32,
    grabbed: u32,
    _pad0: u32,
    previous_position: vec2<f32>,
}

struct RenderParams {
    particle_size: f32,
    screen_width: f32,
    screen_height: f32,
    coloring_mode: u32, // 0 = density, 1 = velocity
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) mass: f32,
    @location(2) density: f32,
    @location(3) uv: vec2<f32>,
    @location(4) coloring_mode: f32,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: RenderParams;
@group(0) @binding(2) var<storage, read> lut: array<u32>;

fn get_lut_color(index: u32) -> vec3<f32> {
    let r = f32(lut[index]) / 255.0;
    let g = f32(lut[index + 256]) / 255.0;
    let b = f32(lut[index + 512]) / 255.0;
    return vec3<f32>(r, g, b);
}

fn get_particle_color(particle: Particle) -> vec3<f32> {
    // Color based on mass and clump id
    // Scale mass (expected ~0.1-0.3) into 0-1 range for LUT selection
    let mass_factor = clamp(particle.mass * 3.33, 0.0, 1.0);
    let clump_factor = clamp(f32(particle.clump_id) / 5.0, 0.0, 1.0);
    
    // Use LUT based on mass (smaller particles = blue, larger = red)
    let mass_index = u32(mass_factor * 255.0);
    let base_color = get_lut_color(mass_index);
    
    // Add brightness for clumped particles
    let clumped_brightness = vec3<f32>(1.0, 1.0, 1.0) * clump_factor * 0.3;
    return base_color + clumped_brightness;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Each instance represents one particle. Use instance_index to select the particle.
    let particle_index = instance_index;
    let vertex_id = vertex_index; // 0-5 within the quad
    
    let particle = particles[particle_index];
    
    // Skip rendering if particle has no mass
    if (particle.mass <= 0.0) {
        return VertexOutput(
            vec4<f32>(0.0),
            vec3<f32>(0.0),
            0.0,
            0.0,
            vec2<f32>(0.0),
            0.0
        );
    }
    
    // Create a quad for each particle
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)
    );
    
    let uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );
    
    let pos = positions[vertex_id];
    let uv = uvs[vertex_id];
    
    // Use particle size from settings (uniform size for all particles)
    let particle_size_pixels = params.particle_size * 1000.0; // Convert from world units to pixels
    let world_size = particle_size_pixels / min(params.screen_width, params.screen_height);
    let size = world_size * 2.0; // Uniform size for all particles, no mass scaling
    let world_pos = particle.position + pos * size;
    
    // Convert to clip coordinates using camera transformation
    let clip_pos = vec4<f32>(world_pos.x, world_pos.y, 0.0, 1.0);
    let transformed_pos = clip_pos;
    
    // Color based on mass and merged count
    let color = get_particle_color(particle);
    
    return VertexOutput(
        transformed_pos,
        color,
        particle.mass,
        particle.density,
        uv,
        f32(params.coloring_mode)
    );
} 