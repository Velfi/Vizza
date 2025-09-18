struct SeedVelParams {
    cx: f32,
    cy: f32,
    radius: f32,
    strength: f32,
    grid_w: u32,
    grid_h: u32,
}

@group(0) @binding(0) var vel_dst: texture_storage_2d<rg16float, write>;
@group(0) @binding(1) var<uniform> params: SeedVelParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.grid_w || gid.y >= params.grid_h) { return; }
    let dims = vec2<f32>(f32(params.grid_w), f32(params.grid_h));
    let uv = (vec2<f32>(vec2<u32>(gid.xy)) + vec2<f32>(0.5, 0.5)) / dims;
    let to = uv - vec2<f32>(params.cx, params.cy);
    let d = length(to);
    let fall = smoothstep(params.radius, 0.0, d);
    let tangent = vec2<f32>(-to.y, to.x);
    let v = normalize(tangent) * params.strength * fall;
    let x = i32(gid.x);
    let y = i32(gid.y);
    let prev = textureLoad(vel_dst, vec2<i32>(x,y));
    textureStore(vel_dst, vec2<i32>(x,y), prev + vec4<f32>(v, 0.0, 0.0));
}

