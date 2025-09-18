struct GridParams {
    half_cell: f32,
    _pad0: vec3<f32>,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var velocity_tex: texture_2d<f32>;
@group(0) @binding(1) var divergence_tex: texture_storage_2d<r16float, write>;
@group(0) @binding(2) var<uniform> params: GridParams;
@group(0) @binding(3) var sampler_linear: sampler;

fn vel(ix: i32, iy: i32) -> vec2<f32> {
    let nx = clamp(ix, 0, i32(params.grid_w) - 1);
    let ny = clamp(iy, 0, i32(params.grid_h) - 1);
    let uv = (vec2<f32>(f32(nx) + 0.5, f32(ny) + 0.5)) / vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    return textureSampleLevel(velocity_tex, sampler_linear, uv, 0.0).xy;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }
    let x = i32(gid.x);
    let y = i32(gid.y);
    let vx_r = vel(x + 1, y).x;
    let vx_l = vel(x - 1, y).x;
    let vy_t = vel(x, y + 1).y;
    let vy_b = vel(x, y - 1).y;
    let div = 0.5 * (vx_r - vx_l + vy_t - vy_b);
    textureStore(divergence_tex, vec2<i32>(x, y), vec4<f32>(div, 0.0, 0.0, 0.0));
}

