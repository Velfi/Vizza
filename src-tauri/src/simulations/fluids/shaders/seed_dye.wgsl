struct SeedParams {
    cx: f32,
    cy: f32,
    radius: f32,
    intensity: f32,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var dye_src: texture_2d<f32>;
@group(0) @binding(1) var dye_dst: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> params: SeedParams;
@group(0) @binding(3) var s: sampler;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }
    let dims = vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    let uv = (vec2<f32>(vec2<u32>(gid.xy)) + vec2<f32>(0.5, 0.5)) / dims;
    let d = distance(uv, vec2<f32>(params.cx, params.cy));
    let a = smoothstep(params.radius, 0.0, d);
    let col = vec4<f32>(a*params.intensity, a*params.intensity*0.5, a*params.intensity*0.2, a);
    let x = i32(gid.x);
    let y = i32(gid.y);
    let prev = textureSampleLevel(dye_src, s, uv, 0.0);
    textureStore(dye_dst, vec2<i32>(x,y), prev + col);
}

