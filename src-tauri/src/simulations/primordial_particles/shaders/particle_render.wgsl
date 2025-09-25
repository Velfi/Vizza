// Primordial Particles Render Shader
// Renders particles as colored points

const PI: f32 = 3.14159265359;

struct Particle {
    position: vec2<f32>,
    previous_position: vec2<f32>,
    heading: f32,
    velocity: f32, // Magnitude of velocity for coloring
    density: f32,  // Local density for coloring
    grabbed: u32,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct RenderParams {
    particle_size: f32,
    foreground_color_mode: u32, // 0=Random, 1=Density, 2=Heading, 3=Velocity
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> camera: CameraUniform;

@group(0) @binding(2)
var<uniform> render_params: RenderParams;

@group(0) @binding(3)
var<storage, read> lut: array<u32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) @interpolate(flat) particle_index: u32,
    @location(2) uv: vec2<f32>,
}

// Convert from sRGB (gamma-corrected) to linear RGB
fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

// Get color from LUT based on intensity (0-1)
fn get_lut_color(intensity: f32) -> vec3<f32> {
    let index = u32(clamp(intensity * 255.0, 0.0, 255.0));
    let r_srgb = f32(lut[index]) / 255.0;
    let g_srgb = f32(lut[index + 256u]) / 255.0;
    let b_srgb = f32(lut[index + 512u]) / 255.0;
    return vec3<f32>(
        srgb_to_linear(r_srgb),
        srgb_to_linear(g_srgb),
        srgb_to_linear(b_srgb)
    );
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let particle_index = instance_index; // one instance per particle
    let vertex_in_quad = vertex_index % 6u; // 6 vertices per particle (quad)
    
    let particle = particles[particle_index];
    
    // Use world position directly; the infinite tiling pass applies camera transform
    let world_pos = particle.position;
    let screen_pos = world_pos;
    
    // Create quad vertices
    let size = render_params.particle_size;
    var vertex_pos: vec2<f32>;
    var uv: vec2<f32>;
    
    switch (vertex_in_quad) {
        case 0u: { vertex_pos = vec2<f32>(-size, -size); uv = vec2<f32>(0.0, 0.0); }
        case 1u: { vertex_pos = vec2<f32>(size, -size); uv = vec2<f32>(1.0, 0.0); }
        case 2u: { vertex_pos = vec2<f32>(-size, size); uv = vec2<f32>(0.0, 1.0); }
        case 3u: { vertex_pos = vec2<f32>(size, -size); uv = vec2<f32>(1.0, 0.0); }
        case 4u: { vertex_pos = vec2<f32>(size, size); uv = vec2<f32>(1.0, 1.0); }
        case 5u: { vertex_pos = vec2<f32>(-size, size); uv = vec2<f32>(0.0, 1.0); }
        default: { vertex_pos = vec2<f32>(0.0, 0.0); uv = vec2<f32>(0.0, 0.0); }
    }
    
    // Keep particle size in world units; do not reapply camera zoom here
	// Apply aspect ratio correction like Particle Life so circles don't become ellipses
	let aspect_corrected_vertex = vec2<f32>(vertex_pos.x / camera.aspect_ratio, vertex_pos.y);
	let final_pos = screen_pos + aspect_corrected_vertex;
    
    // Generate color based on color scheme mode
    var color: vec3<f32>;
    
    switch (render_params.foreground_color_mode) {
        // 0 = Random
        case 0u: {
            let random_factor = f32(particle_index) / 10000.0;
            color = get_lut_color(random_factor);
        }
        // 1 = Density
        case 1u: {
            let density_factor = clamp(particle.density / 16.0, 0.0, 1.0);
            color = get_lut_color(density_factor);
        }
        // 2 = Heading
        case 2u: {
            let normalized_heading = (particle.heading + PI) / (2.0 * PI);
            color = get_lut_color(normalized_heading);
        }
        // 3 = Velocity
        case 3u: {
            let velocity_factor = clamp(particle.velocity / 1.0, 0.0, 1.0);
            color = get_lut_color(velocity_factor);
        }
        default: {
            let normalized_heading = (particle.heading + PI) / (2.0 * PI);
            color = get_lut_color(normalized_heading);
        }
    }
    
    return VertexOutput(
        vec4<f32>(final_pos, 0.0, 1.0),
        color,
        particle_index,
        uv
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular particles by discarding pixels outside the circle
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(input.uv - center);
    let particle_radius = 0.45;
    
    if (dist > particle_radius) {
        discard;
    }
    
    return vec4<f32>(input.color, 1.0);
}
