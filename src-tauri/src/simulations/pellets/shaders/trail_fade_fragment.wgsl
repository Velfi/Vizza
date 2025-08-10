// Fullscreen fade fragment shader for trail persistence

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct FadeUniforms {
    fade_amount: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) var<uniform> fade_uniforms: FadeUniforms;
@group(0) @binding(1) var prev_trail_tex: texture_2d<f32>;
@group(0) @binding(2) var prev_trail_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(prev_trail_tex, prev_trail_sampler, input.uv);
    let faded_alpha = max(tex_color.a - fade_uniforms.fade_amount, 0.0);
    return vec4<f32>(tex_color.rgb, faded_alpha);
}


