// Populate spatial grid for Voronoi CA site positions

struct Vertex {
  position: vec2<f32>,
  state: f32,
  pad0: f32,
  age: f32,
  alive_neighbors: u32,
  dead_neighbors: u32,
  pad1: u32,
}

struct GridParams {
  particle_count: u32,
  grid_width: u32,
  grid_height: u32,
  cell_capacity: u32,
  cell_size: f32,
  jfa_width: f32,
  jfa_height: f32,
  _pad: f32,
}

@group(0) @binding(0) var<storage, read> vertices: array<Vertex>;
// A flattened [grid_width*grid_height * cell_capacity] array of indices
@group(0) @binding(1) var<storage, read_write> grid_indices: array<u32>;
// Per-cell atomic counters
@group(0) @binding(2) var<storage, read_write> grid_counts: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params: GridParams;

fn grid_flat_index(cell_x: u32, cell_y: u32) -> u32 {
  return cell_y * params.grid_width + cell_x;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= params.particle_count) { return; }
  let p = vertices[i].position;
  // Convert JFA pixel coords to grid cell
  let gx = clamp(u32(p.x / params.cell_size), 0u, max(1u, params.grid_width) - 1u);
  let gy = clamp(u32(p.y / params.cell_size), 0u, max(1u, params.grid_height) - 1u);
  let cell = grid_flat_index(gx, gy);
  // Reserve a slot
  let slot = atomicAdd(&grid_counts[cell], 1u);
  if (slot < params.cell_capacity) {
    let base = cell * params.cell_capacity;
    grid_indices[base + slot] = i;
  }
}


