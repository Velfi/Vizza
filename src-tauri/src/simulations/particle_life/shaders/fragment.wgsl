// Particle Life fragment shader

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) species: u32,
    @location(1) velocity_magnitude: f32,
    @location(2) uv: vec2<f32>,
}

@group(1) @binding(0) var lut_texture: texture_2d<f32>;
@group(1) @binding(1) var lut_sampler: sampler;
@group(1) @binding(2) var<uniform> lut_size: u32;

// Species colors - bright, distinct colors
fn get_species_color(species: u32) -> vec3<f32> {
    switch species {
        case 0u: { return vec3<f32>(1.0, 0.2, 0.2); }  // Red
        case 1u: { return vec3<f32>(0.2, 1.0, 0.2); }  // Green
        case 2u: { return vec3<f32>(0.2, 0.2, 1.0); }  // Blue
        case 3u: { return vec3<f32>(1.0, 1.0, 0.2); }  // Yellow
        case 4u: { return vec3<f32>(1.0, 0.2, 1.0); }  // Magenta
        case 5u: { return vec3<f32>(0.2, 1.0, 1.0); }  // Cyan
        case 6u: { return vec3<f32>(1.0, 0.6, 0.2); }  // Orange
        case 7u: { return vec3<f32>(0.6, 0.2, 1.0); }  // Purple
        default: { return vec3<f32>(1.0, 1.0, 1.0); }  // White fallback
    }
}

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // LUT mode: sample color from LUT texture based on species index
    // LUT is a 2D texture with N+1 colors (first is background, rest are species)
    let species_index = f32(input.species);
    let lut_index = (species_index + 1.0) / f32(lut_size); // +1 to skip background color, normalize to [0,1]
    let lut_color = textureSample(lut_texture, lut_sampler, vec2<f32>(lut_index, 0.5)).rgb;
    let base_color = lut_color;
    
    // Modulate brightness based on velocity for visual feedback
    let velocity_factor = clamp(input.velocity_magnitude / 50.0, 0.3, 1.0);
    let color = base_color * velocity_factor;
    
    // Create circular particles by using distance from center
    let center = vec2<f32>(0.5, 0.5);
    let dist_from_center = distance(input.uv, center);
    
    // Smooth circular falloff
    let alpha = 1.0 - smoothstep(0.35, 0.5, dist_from_center);
    
    return vec4<f32>(color, alpha);
}