// Fullscreen fade fragment shader for particle traces

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct FadeUniforms {
    fade_alpha: f32,              // Alpha for fading effect
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) var<uniform> fade_uniforms: FadeUniforms;
@group(0) @binding(1) var display_tex: texture_2d<f32>;
@group(0) @binding(2) var display_sampler: sampler;

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample from the display texture and apply fade alpha
    let tex_color = textureSample(display_tex, display_sampler, input.uv);
    return vec4<f32>(
        tex_color.rgb,
        fade_uniforms.fade_alpha
    );
} 