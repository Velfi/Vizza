// Applies CA update using neighbor counts computed previously
// Reads and writes the same storage buffer, but only this pass mutates state/age
// Layout matches compute.wgsl: binding(0) = storage Vertices, binding(1) = Uniforms

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
  neighbor_radius: f32,
  alive_threshold: f32,
  _pad0: u32,
}

@group(0) @binding(0) var<storage, read_write> vertices: Vertices;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  let count = arrayLength(&vertices.data);
  if (i >= count) { return; }

  var v = vertices.data[i];

  // Outer-totalistic rule on Voronoi sites: compare local alive ratio to threshold
  let total_n = max(1u, v.alive_neighbors + v.dead_neighbors);
  let alive_ratio = f32(v.alive_neighbors) / f32(total_n);
  let target_alive = select(0.0, 1.0, alive_ratio > uniforms.alive_threshold);

  // Optional: support a couple of rule flavors via rule_type (0: threshold, 1: majority with inertia)
  var next_state = target_alive;
  if (uniforms.rule_type == 1u) {
    // Simple inertia: blend toward target to avoid flicker
    next_state = mix(v.state, target_alive, 0.5);
    next_state = select(0.0, 1.0, next_state > 0.5);
  }

  // Age update: grow age when alive, decay slightly when dead; clamp to a small range
  if (next_state >= 0.5) {
    v.age = fract(v.age + 0.016 * 0.6);
  } else {
    v.age = v.age * 0.98;
  }

  v.state = next_state;
  vertices.data[i] = v;
}


