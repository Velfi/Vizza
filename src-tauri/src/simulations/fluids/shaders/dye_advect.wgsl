struct DyeParams {
    dt: f32,
    decay: f32,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var velocity_tex: texture_2d<f32>;
@group(0) @binding(1) var dye_src: texture_2d<f32>;
@group(0) @binding(2) var dye_dst: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<uniform> params: DyeParams;
@group(0) @binding(4) var sampler_linear: sampler;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }
    let dims = vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    let uv = (vec2<f32>(vec2<u32>(gid.xy)) + vec2<f32>(0.5, 0.5)) / dims;
    let v = textureSampleLevel(velocity_tex, sampler_linear, uv, 0.0).xy;
    let prev_uv = uv - v * params.dt;
    let c = textureSampleLevel(dye_src, sampler_linear, clamp(prev_uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    let out_color = c * (1.0 - params.decay);
    textureStore(dye_dst, vec2<i32>(i32(gid.x), i32(gid.y)), out_color);
}

