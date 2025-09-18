struct AdvectParams {
    dt: f32,
    dissipation: f32,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var velocity_tex: texture_2d<f32>;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var dst_tex: texture_storage_2d<rg16float, write>;
@group(0) @binding(3) var<uniform> params: AdvectParams;
@group(0) @binding(4) var sampler_linear: sampler;

fn sample_velocity(uv: vec2<f32>) -> vec2<f32> {
    let v = textureSampleLevel(velocity_tex, sampler_linear, uv, 0.0).xy;
    return v;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }

    let dims = vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    let uv = (vec2<f32>(vec2<u32>(gid.xy)) + vec2<f32>(0.5, 0.5)) / dims;

    // Backtrace
    let vel = sample_velocity(uv);
    let prev_uv = uv - vel * params.dt / vec2<f32>(1.0, 1.0);
    let prev_sample = textureSampleLevel(src_tex, sampler_linear, clamp(prev_uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);

    let out_color = prev_sample * params.dissipation;
    textureStore(dst_tex, vec2<i32>(i32(gid.x), i32(gid.y)), out_color);
}

