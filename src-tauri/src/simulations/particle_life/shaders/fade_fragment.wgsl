// Fullscreen fade fragment shader for particle traces

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct FadeUniforms {
    fade_amount: f32,             // Amount to subtract from alpha each frame (0.0 = no fade, higher = faster fade)
    _pad1: f32,                   // Padding for 16-byte alignment
    _pad2: f32,                   // Padding for 16-byte alignment
    _pad3: f32,                   // Padding for 16-byte alignment
}

@group(0) @binding(0) var<uniform> fade_uniforms: FadeUniforms;
@group(0) @binding(1) var display_tex: texture_2d<f32>;
@group(0) @binding(2) var display_sampler: sampler;

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample from the display texture
    let tex_color = textureSample(display_tex, display_sampler, input.uv);
    
    // Apply alpha fade by subtracting a fixed amount each frame
    // fade_amount controls how much alpha to subtract per frame
    // 0.0 = no fade, higher values = faster fade
    let faded_alpha = max(tex_color.a - fade_uniforms.fade_amount, 0.0);
    
    // Keep the original color but reduce the alpha
    return vec4<f32>(tex_color.rgb, faded_alpha);
} 