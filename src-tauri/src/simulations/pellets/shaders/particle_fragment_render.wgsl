struct FragmentInput {
    @location(0) color: vec3<f32>,
    @location(1) mass: f32,
    @location(2) density: f32,
    @location(3) uv: vec2<f32>,
    @location(4) coloring_mode: f32,
}

struct RenderParams {
    particle_size: f32,
    screen_width: f32,
    screen_height: f32,
    coloring_mode: u32, // 0 = density, 1 = velocity, 2 = random
}

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

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
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