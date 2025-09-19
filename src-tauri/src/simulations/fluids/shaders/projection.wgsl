struct GridParams {
    half_cell: f32,
    _pad0: vec3<f32>,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var pressure_tex: texture_2d<f32>;
@group(0) @binding(1) var velocity_src: texture_2d<f32>;
@group(0) @binding(2) var velocity_dst: texture_storage_2d<rg16float, write>;
@group(0) @binding(3) var<uniform> params: GridParams;
@group(0) @binding(4) var sampler_linear: sampler;

fn sample_r(tex: texture_2d<f32>, uv: vec2<f32>) -> f32 {
    return textureSampleLevel(tex, sampler_linear, uv, 0.0).x;
}

fn p(ix: i32, iy: i32) -> f32 {
    let nx = clamp(ix, 0, i32(params.grid_w) - 1);
    let ny = clamp(iy, 0, i32(params.grid_h) - 1);
    let uv = (vec2<f32>(f32(nx) + 0.5, f32(ny) + 0.5)) / vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    return sample_r(pressure_tex, uv);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }

    let dims = vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    let uv = (vec2<f32>(vec2<u32>(gid.xy)) + vec2<f32>(0.5, 0.5)) / dims;

    let grad_p = vec2<f32>(
        p(i32(gid.x + 1u), i32(gid.y)) - p(i32(gid.x - 1u), i32(gid.y)),
        p(i32(gid.x), i32(gid.y + 1u)) - p(i32(gid.x), i32(gid.y - 1u))
    ) * 0.5;

    let v_src = textureSampleLevel(velocity_src, sampler_linear, uv, 0.0).xy;
    let v = v_src - grad_p;
    textureStore(velocity_dst, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(v, 0.0, 0.0));
}

