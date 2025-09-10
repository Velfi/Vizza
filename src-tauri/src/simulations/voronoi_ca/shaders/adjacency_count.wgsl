// Count alive neighbors using the prebuilt Voronoi adjacency (graph neighbors per site)

struct Vertex {
  position: vec2<f32>,
  state: f32,
  pad0: f32,
  age: f32,
  alive_neighbors: u32,
  dead_neighbors: u32,
  pad1: u32,
}
struct Vertices { data: array<Vertex> }

struct Uniforms {
  resolution: vec2<f32>,
  time: f32,
  drift: f32,
  rule_type: u32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
}

struct NeighborList { data: array<u32> }
struct Degrees { data: array<u32> }

@group(0) @binding(0) var<storage, read_write> vertices: Vertices;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<storage, read> neighbors: NeighborList;
@group(0) @binding(3) var<storage, read> degrees: Degrees;

const MAX_NEIGHBORS: u32 = 16u;

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  let count = arrayLength(&vertices.data);
  if (i >= count) { return; }

  var v = vertices.data[i];

  let deg = degrees.data[i];
  let base = i * MAX_NEIGHBORS;

  var alive_n: u32 = 0u;
  var dead_n: u32 = 0u;
  for (var k = 0u; k < deg && k < MAX_NEIGHBORS; k = k + 1u) {
    let j = neighbors.data[base + k];
    let u = vertices.data[j];
    if (u.state >= 0.5) { alive_n = alive_n + 1u; }
    else { dead_n = dead_n + 1u; }
  }

  v.alive_neighbors = alive_n;
  v.dead_neighbors = dead_n;
  vertices.data[i] = v;
}


