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

  // Conway's Game of Life rules (B3/S23)
  // B3: Birth if exactly 3 alive neighbors
  // S23: Survival if 2 or 3 alive neighbors
  let alive_n = v.alive_neighbors;
  let is_alive = v.state >= 0.5;
  
  var next_state: f32;
  if (is_alive) {
    // Survival: live cell survives with 2 or 3 neighbors
    next_state = select(0.0, 1.0, alive_n == 2u || alive_n == 3u);
  } else {
    // Birth: dead cell becomes alive with exactly 3 neighbors
    next_state = select(0.0, 1.0, alive_n == 3u);
  }

  // Optional: support rule variations via rule_type
  if (uniforms.rule_type == 1u) {
    // High Life variant (B36/S23) - also birth with 6 neighbors
    if (!is_alive && alive_n == 6u) {
      next_state = 1.0;
    }
  } else if (uniforms.rule_type == 2u) {
    // Seeds variant (B2/S) - birth with 2 neighbors, no survival
    if (!is_alive && alive_n == 2u) {
      next_state = 1.0;
    } else if (is_alive) {
      next_state = 0.0; // No survival
    }
  }

  // Age update: grow age when alive, decay when dead
  if (next_state >= 0.5) {
    v.age = fract(v.age + 0.016 * 0.6);
  } else {
    v.age = v.age * 0.98;
  }

  v.state = next_state;
  vertices.data[i] = v;
}


