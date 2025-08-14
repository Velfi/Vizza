struct JfaSeedParams {
  width: u32,
  height: u32,
  count: u32,
  _pad: u32,
};

@group(0) @binding(0) var dstTex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> params: JfaSeedParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.width || gid.y >= params.height) { return; }
  let p = vec2<i32>(i32(gid.x), i32(gid.y));
  // Clear to invalid seed id (-1), zero seed position
  textureStore(dstTex, p, vec4<f32>(0.0, 0.0, -1.0, 0.0));
}



