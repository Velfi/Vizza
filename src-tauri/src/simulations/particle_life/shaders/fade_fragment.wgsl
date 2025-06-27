// Fullscreen fade fragment shader for particle traces

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct FadeUniforms {
    background_color: vec4<f32>,  // RGBA background color
    fade_alpha: f32,              // Alpha for fading effect
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) var<uniform> fade_uniforms: FadeUniforms;

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Return background color with fade alpha
    // This will be alpha blended over the existing trail content
    // The fade_alpha controls how much the trails fade towards the background
    return vec4<f32>(
        fade_uniforms.background_color.rgb,
        fade_uniforms.fade_alpha
    );
} 