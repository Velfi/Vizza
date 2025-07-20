struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct FragmentInput {
    @location(0) color: vec3<f32>,
    @location(1) mass: f32,
    @location(2) density: f32,
    @location(3) uv: vec2<f32>,
    @location(4) coloring_mode: f32,
}

@group(0) @binding(3) var<storage, read> lut: array<u32>;
@group(0) @binding(1) var<uniform> camera: CameraUniform;

fn get_lut_color(index: u32) -> vec3<f32> {
    let r = f32(lut[index]) / 255.0;
    let g = f32(lut[index + 256]) / 255.0;
    let b = f32(lut[index + 512]) / 255.0;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // Create circular particles with aspect ratio correction
    let center = vec2<f32>(0.5, 0.5);
    
    // Correct for aspect ratio to ensure circular particles
    let aspect_corrected_uv = vec2<f32>(
        (in.uv.x - 0.5) * camera.aspect_ratio + 0.5,
        in.uv.y
    );
    
    let dist = distance(aspect_corrected_uv, center);
    
    // Define particle radius with hard cutoff like Particle Life
    let particle_radius = 0.45;
    
    // Discard pixels outside the particle radius for hard edges
    if (dist > particle_radius) {
        discard;
    }
    
    // Density or velocity-based coloring using LUT
    let scale_factor = select(16.0, 4.0, in.coloring_mode > 0.5); // 16.0 for density, 4.0 for velocity
    let color_factor = clamp(in.density / scale_factor, 0.0, 1.0);
    let lut_index = u32(color_factor * 255.0);
    let color = get_lut_color(lut_index);
    
    // Add mass-based glow effect for larger particles
    let mass_factor = clamp(in.mass / 20.0, 0.0, 1.0);
    let glow_intensity = mass_factor * 0.3;
    let glow_color = vec3<f32>(1.0, 0.9, 0.7) * glow_intensity;
    
    // Combine color with glow
    let final_color = color + glow_color;
    
    // Use mass-based alpha for particle opacity
    let alpha = 1.0; // Fully opaque particles
    
    return vec4<f32>(final_color, alpha);
} 