// Brownian motion compute shader
// Bindings:
//  @binding(0) storage buffer (vertices) - read/write
//  @binding(1) uniform (uniforms)
//  @binding(2) uniform (brownian_params)

struct Vertex {
  position: vec2<f32>,
  state: f32,
  pad0: f32,
  age: f32,
  alive_neighbors: u32,
  dead_neighbors: u32,
  random_state: u32,
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

struct BrownianParams {
  speed: f32,
  delta_time: f32,
}

@group(0) @binding(0) var<storage, read_write> vertices: Vertices;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<uniform> params: BrownianParams;

// Improved pseudo-random number generator (Linear Congruential Generator)
fn lcg_random(state: ptr<function, u32>) -> f32 {
  var x = *state;
  x = x * 1664525u + 1013904223u; // LCG constants
  *state = x;
  return f32(x) / 4294967296.0; // Convert to [0,1)
}

// Generate multiple random values using LCG
fn random_vec2(state: ptr<function, u32>) -> vec2<f32> {
  return vec2<f32>(lcg_random(state), lcg_random(state));
}

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  let count = arrayLength(&vertices.data);
  if (i >= count) { return; }

  let dt = params.delta_time;
  let speed = params.speed * uniforms.drift;
  let min_x = 0.0;
  let min_y = 0.0;
  let size_x = uniforms.resolution.x;
  let size_y = uniforms.resolution.y;

  // Use per-point random state for independent brownian motion
  var random_state = vertices.data[i].random_state;
  
  // Mix in time and position for additional variation
  random_state = random_state ^ u32(uniforms.time * 1000.0);
  random_state = random_state ^ u32(vertices.data[i].position.x * 1000.0);
  random_state = random_state ^ u32(vertices.data[i].position.y * 1000.0);

  // Generate random displacement for brownian motion
  let random_displacement = random_vec2(&random_state);
  
  // Update the random state for next frame
  vertices.data[i].random_state = random_state;
  let dx = (random_displacement.x - 0.5) * 2.0 * speed * dt; // [-1, 1] * speed * dt
  let dy = (random_displacement.y - 0.5) * 2.0 * speed * dt; // [-1, 1] * speed * dt
  
  var new_x = vertices.data[i].position.x + dx;
  var new_y = vertices.data[i].position.y + dy;
  
  // Toroidal wrapping
  new_x = (new_x - min_x) % size_x + min_x;
  new_y = (new_y - min_y) % size_y + min_y;
  
  vertices.data[i].position = vec2<f32>(new_x, new_y);
}
