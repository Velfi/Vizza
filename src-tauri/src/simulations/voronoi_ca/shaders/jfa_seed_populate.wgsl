// Writes per-point seeds into the JFA texture
struct Vertex {
  position: vec2<f32>,
  state: f32,
  pad0: f32,
  age: f32,
  alive_neighbors: u32,
  dead_neighbors: u32,
  pad1: u32,
};
struct Vertices { data: array<Vertex> };

struct JfaSeedParams {
  width: u32,
  height: u32,
  count: u32,
  _pad: u32,
};

@group(0) @binding(0) var<storage, read> vertices: Vertices;
@group(0) @binding(1) var dstTex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: JfaSeedParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= params.count) { return; }
  let v = vertices.data[i];
  let x = clamp(i32(round(v.position.x)), 0, i32(params.width) - 1);
  let y = clamp(i32(round(v.position.y)), 0, i32(params.height) - 1);
  
  // Write actual world position and id; last-write wins if collisions occur
  textureStore(dstTex, vec2<i32>(x, y), vec4<f32>(v.position.x, v.position.y, f32(i), 0.0));
}
