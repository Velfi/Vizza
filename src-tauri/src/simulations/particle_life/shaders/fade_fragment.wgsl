// Fullscreen fade fragment shader for particle traces

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct FadeUniforms {
    background_color: vec3<f32>,  // Background color to fade to
    fade_alpha: f32,              // Alpha for fading effect (0.0 = full fade, 1.0 = no fade)
    _pad1: f32,                   // Padding for 16-byte alignment
}

@group(0) @binding(0) var<uniform> fade_uniforms: FadeUniforms;
@group(0) @binding(1) var display_tex: texture_2d<f32>;
@group(0) @binding(2) var display_sampler: sampler;

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample from the display texture
    let tex_color = textureSample(display_tex, display_sampler, input.uv);
    
    // Apply fade by blending towards background color
    // fade_alpha controls how much of the original color to keep
    // 0.0 = completely fade to background, 1.0 = keep original color
    let faded_color = mix(fade_uniforms.background_color, tex_color.rgb, fade_uniforms.fade_alpha);
    
    // Preserve the original alpha for proper blending
    return vec4<f32>(faded_color, tex_color.a);
} 