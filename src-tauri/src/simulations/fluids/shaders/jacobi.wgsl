struct JacobiParams {
    alpha: f32,
    beta: f32,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var pressure_src: texture_2d<f32>;
@group(0) @binding(1) var divergence_tex: texture_2d<f32>;
@group(0) @binding(2) var pressure_dst: texture_storage_2d<r16float, write>;
@group(0) @binding(3) var<uniform> params: JacobiParams;
@group(0) @binding(4) var sampler_linear: sampler;

fn sample_r(tex: texture_2d<f32>, uv: vec2<f32>) -> f32 {
    return textureSampleLevel(tex, sampler_linear, uv, 0.0).x;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }

    let dims = vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    let uv = (vec2<f32>(vec2<u32>(gid.xy)) + vec2<f32>(0.5, 0.5)) / dims;

    let left_uv = (vec2<f32>(vec2<u32>(max(i32(gid.x),1u)-1u, gid.y)) + vec2<f32>(0.5)) / dims;
    let right_uv = (vec2<f32>(vec2<u32>(min(gid.x+1u, params.grid_w-1u), gid.y)) + vec2<f32>(0.5)) / dims;
    let bottom_uv = (vec2<f32>(gid.x, max(i32(gid.y),1u)-1u) + vec2<f32>(0.5)) / dims;
    let top_uv = (vec2<f32>(gid.x, min(gid.y+1u, params.grid_h-1u)) + vec2<f32>(0.5)) / dims;

    let pL = sample_r(pressure_src, left_uv);
    let pR = sample_r(pressure_src, right_uv);
    let pB = sample_r(pressure_src, bottom_uv);
    let pT = sample_r(pressure_src, top_uv);
    let div = sample_r(divergence_tex, uv);

    let p = (pL + pR + pB + pT + params.alpha * div) / params.beta;
    textureStore(pressure_dst, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(p, 0.0, 0.0, 0.0));
}

