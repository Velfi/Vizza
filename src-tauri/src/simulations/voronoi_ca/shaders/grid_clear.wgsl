// Clear spatial grid counts for Voronoi CA

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

// Atomic per-cell counts used for concurrent population
@group(0) @binding(0) var<storage, read_write> grid_counts: array<atomic<u32>>;
@group(0) @binding(1) var<uniform> params: GridParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let index = gid.x;
  let total_cells = params.grid_width * params.grid_height;
  if (index >= total_cells) { return; }
  atomicStore(&grid_counts[index], 0u);
}


