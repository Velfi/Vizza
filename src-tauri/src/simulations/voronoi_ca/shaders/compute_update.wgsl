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
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
}

@group(0) @binding(0) var<storage, read_write> vertices: Vertices;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  let count = arrayLength(&vertices.data);
  if (i >= count) { return; }

  var v = vertices.data[i];

  // Life-like cellular automaton rules
  let alive_n = v.alive_neighbors;
  let is_alive = v.state >= 0.5;
  
  var next_state: f32 = 0.0;
  
  // Apply rules based on rule_type
  switch (uniforms.rule_type) {
    case 0u: { // B1357/S1357 - Replicator
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 1u || alive_n == 3u || alive_n == 5u || alive_n == 7u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 1u || alive_n == 3u || alive_n == 5u || alive_n == 7u);
      }
    }
    case 1u: { // B2/S - Seeds
      if (!is_alive && alive_n == 2u) {
        next_state = 1.0;
      } else if (is_alive) {
        next_state = 0.0; // No survival
      }
    }
    case 2u: { // B25/S4 - Small self-replicating pattern
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 4u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 2u || alive_n == 5u);
      }
    }
    case 3u: { // B3/S012345678 - Life without Death
      if (is_alive) {
        next_state = 1.0; // Never die
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u);
      }
    }
    case 4u: { // B3/S23 - Conway's Game of Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 2u || alive_n == 3u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u);
      }
    }
    case 5u: { // B3/S1234 - Maze
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n >= 1u && alive_n <= 4u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u);
      }
    }
    case 6u: { // B3/S12345 - Mazectric
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n >= 1u && alive_n <= 5u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u);
      }
    }
    case 7u: { // B34/S34 - 34 Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 3u || alive_n == 4u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u || alive_n == 4u);
      }
    }
    case 8u: { // B35678/S5678 - Diamoeba
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n >= 5u && alive_n <= 8u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u || alive_n == 5u || alive_n == 6u || alive_n == 7u || alive_n == 8u);
      }
    }
    case 9u: { // B36/S125 - 2x2
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 1u || alive_n == 2u || alive_n == 5u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u || alive_n == 6u);
      }
    }
    case 10u: { // B36/S23 - High Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 2u || alive_n == 3u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u || alive_n == 6u);
      }
    }
    case 11u: { // B368/S245 - Day & Night
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 2u || alive_n == 4u || alive_n == 5u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u || alive_n == 6u || alive_n == 8u);
      }
    }
    case 12u: { // B4678/S35678 - Anneal
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n >= 3u && alive_n <= 8u);
      } else {
        next_state = select(0.0, 1.0, alive_n >= 4u && alive_n <= 8u);
      }
    }
    case 13u: { // B5678/S45678 - Vote
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n >= 4u && alive_n <= 8u);
      } else {
        next_state = select(0.0, 1.0, alive_n >= 5u && alive_n <= 8u);
      }
    }
    case 14u: { // B6/S16 - Coral
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 1u || alive_n == 6u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 6u);
      }
    }
    case 15u: { // B6/S1 - Long Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 1u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 6u);
      }
    }
    case 16u: { // B6/S12 - Stains
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 1u || alive_n == 2u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 6u);
      }
    }
    case 17u: { // B6/S123 - Assimilation
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n >= 1u && alive_n <= 3u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 6u);
      }
    }
    case 18u: { // B6/S15 - Pseudo Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 1u || alive_n == 5u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 6u);
      }
    }
    case 19u: { // B6/S2 - Long Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 2u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 6u);
      }
    }
    case 20u: { // B7/S - Seeds variant
      if (!is_alive && alive_n == 7u) {
        next_state = 1.0;
      } else if (is_alive) {
        next_state = 0.0; // No survival
      }
    }
    case 21u: { // B8/S - Seeds variant
      if (!is_alive && alive_n == 8u) {
        next_state = 1.0;
      } else if (is_alive) {
        next_state = 0.0; // No survival
      }
    }
    case 22u: { // B9/S - Seeds variant
      if (!is_alive && alive_n == 9u) {
        next_state = 1.0;
      } else if (is_alive) {
        next_state = 0.0; // No survival
      }
    }
    default: { // Default to Conway's Game of Life
      if (is_alive) {
        next_state = select(0.0, 1.0, alive_n == 2u || alive_n == 3u);
      } else {
        next_state = select(0.0, 1.0, alive_n == 3u);
      }
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


