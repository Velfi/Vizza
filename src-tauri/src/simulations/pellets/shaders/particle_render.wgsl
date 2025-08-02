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
    coloring_mode: u32, // 0 = density, 1 = velocity, 2 = random
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

// Convert from sRGB (gamma-corrected) to linear RGB
fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

fn get_lut_color(index: u32) -> vec3<f32> {
    let r_srgb = f32(lut[index]) / 255.0;
    let g_srgb = f32(lut[index + 256]) / 255.0;
    let b_srgb = f32(lut[index + 512]) / 255.0;
    return vec3<f32>(
        srgb_to_linear(r_srgb),
        srgb_to_linear(g_srgb),
        srgb_to_linear(b_srgb)
    );
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

// Calculate wrapped position based on instance index
fn get_wrapped_position(base_position: vec2<f32>, wrap_instance: u32) -> vec2<f32> {
    // Each particle gets rendered 9 times: center + 8 wrapped positions
    // Instance 0 = center, 1-8 = wrapped positions
    if (wrap_instance == 0u) {
        return base_position;
    }
    
    // Calculate wrap offsets for the 8 surrounding positions
    let wrap_offsets = array<vec2<f32>, 8>(
        vec2<f32>(-2.0, -2.0), // top-left
        vec2<f32>( 0.0, -2.0), // top
        vec2<f32>( 2.0, -2.0), // top-right
        vec2<f32>(-2.0,  0.0), // left
        vec2<f32>( 2.0,  0.0), // right
        vec2<f32>(-2.0,  2.0), // bottom-left
        vec2<f32>( 0.0,  2.0), // bottom
        vec2<f32>( 2.0,  2.0)  // bottom-right
    );
    
    let offset = wrap_offsets[wrap_instance - 1u];
    return base_position + offset;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Each particle gets rendered 9 times (center + 8 wrapped positions)
    // So we need to calculate which particle and which wrap instance
    let particle_index = instance_index / 9u;
    let wrap_instance = instance_index % 9u;
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
    
    // Use the pre-calculated particle size that matches collision detection exactly
    let size = params.particle_size; // Already calculated on backend to match collision size
    
    // Get the wrapped position for this instance
    let wrapped_position = get_wrapped_position(particle.position, wrap_instance);
    let world_pos = wrapped_position + pos * size;
    
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular particles with aspect ratio correction
    let center = vec2<f32>(0.5, 0.5);
    let aspect_ratio = params.screen_width / params.screen_height;
    
    // Correct UV coordinates for aspect ratio to ensure circular particles
    var corrected_uv = in.uv - center;
    corrected_uv.x *= aspect_ratio;
    let dist = length(corrected_uv);
    
    // Standard particle rendering with hard edges
    let particle_radius = 0.45;
    
    if (dist > particle_radius) {
        discard;
    }
    
    var final_color: vec3<f32>;
    
    if (in.coloring_mode > 1.5) {
        // Random mode (coloring_mode == 2)
        let color_factor = clamp(in.density / 255.0, 0.0, 1.0);
        let lut_index = u32(color_factor * 255.0);
        let color = get_lut_color(lut_index);
        
        // Add mass-based glow effect for larger particles
        let mass_factor = clamp(in.mass / 20.0, 0.0, 1.0);
        let glow_intensity = mass_factor * 0.3;
        let glow_color = vec3<f32>(1.0, 0.9, 0.7) * glow_intensity;
        
        final_color = color + glow_color;
    } else if (in.coloring_mode > 0.5) {
        // Velocity mode (coloring_mode == 1) 
        let color_factor = clamp(in.density / 4.0, 0.0, 1.0);
        let lut_index = u32(color_factor * 255.0);
        let color = get_lut_color(lut_index);
        
        // Add mass-based glow effect for larger particles
        let mass_factor = clamp(in.mass / 20.0, 0.0, 1.0);
        let glow_intensity = mass_factor * 0.3;
        let glow_color = vec3<f32>(1.0, 0.9, 0.7) * glow_intensity;
        
        final_color = color + glow_color;
    } else {
        // Density mode (coloring_mode == 0)
        let color_factor = clamp(in.density / 16.0, 0.0, 1.0);
        let lut_index = u32(color_factor * 255.0);
        let color = get_lut_color(lut_index);
        
        // Add mass-based glow effect for larger particles
        let mass_factor = clamp(in.mass / 20.0, 0.0, 1.0);
        let glow_intensity = mass_factor * 0.3;
        let glow_color = vec3<f32>(1.0, 0.9, 0.7) * glow_intensity;
        
        final_color = color + glow_color;
    }
    
    return vec4<f32>(final_color, 1.0);
} 