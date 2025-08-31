// Neighbor counting using spatial grid acceleration (uniform grid)
// Bindings:
//  @binding(0) storage buffer (vertices)
//  @binding(1) uniform (uniforms)
//  @binding(2) storage buffer (grid_indices) flattened [numCells * cellCapacity]
//  @binding(3) storage buffer (grid_counts) per-cell counts (read-only here)
//  @binding(4) uniform (grid params)

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

struct GridParams {
  particle_count: u32,
  grid_width: u32,
  grid_height: u32,
  cell_capacity: u32,
  cell_size: f32,
  _pad1: f32,
  _pad2: f32,
  _pad3: f32,
}

@group(0) @binding(0) var<storage, read_write> vertices: Vertices;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<storage, read> grid_indices: array<u32>;
@group(0) @binding(3) var<storage, read> grid_counts: array<u32>;
@group(0) @binding(4) var<uniform> grid: GridParams;

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  let count = arrayLength(&vertices.data);
  if (i >= count) { return; }

  // Local copy
  var v = vertices.data[i];

  // Note: no age/state changes here; this pass only measures neighbors

  // Neighbor scan: grid-accelerated when grid.cell_size > 0, else naive O(N)
  var alive_n: u32 = 0u;
  var dead_n: u32 = 0u;
  let radius = uniforms.neighbor_radius;
  let r2 = radius * radius;
  if (grid.cell_size > 0.0) {
    let min_cx = i32(max(0u, u32(max(0.0, (v.position.x - radius) / grid.cell_size))));
    let min_cy = i32(max(0u, u32(max(0.0, (v.position.y - radius) / grid.cell_size))));
    let max_cx = i32(min(grid.grid_width - 1u, u32((v.position.x + radius) / grid.cell_size)));
    let max_cy = i32(min(grid.grid_height - 1u, u32((v.position.y + radius) / grid.cell_size)));

    for (var cy = min_cy; cy <= max_cy; cy = cy + 1) {
      for (var cx = min_cx; cx <= max_cx; cx = cx + 1) {
        let cell: u32 = u32(cy) * grid.grid_width + u32(cx);
        let count_in_cell: u32 = grid_counts[cell];
        let base: u32 = cell * grid.cell_capacity;
        var k: u32 = 0u;
        loop {
          if (k >= count_in_cell || k >= grid.cell_capacity) { break; }
          let j: u32 = grid_indices[base + k];
          if (j != i) {
            let u = vertices.data[j];
            // Toroidal distance: wrap across edges
            let w = uniforms.resolution.x;
            let h = uniforms.resolution.y;
            var dx = u.position.x - v.position.x;
            var dy = u.position.y - v.position.y;
            if (dx >  0.5 * w) { dx = dx - w; }
            if (dx < -0.5 * w) { dx = dx + w; }
            if (dy >  0.5 * h) { dy = dy - h; }
            if (dy < -0.5 * h) { dy = dy + h; }
            let d2 = dx*dx + dy*dy;
            if (d2 <= r2) {
              if (u.state >= uniforms.alive_threshold) { alive_n = alive_n + 1u; } else { dead_n = dead_n + 1u; }
            }
          }
          k = k + 1u;
        }
      }
    }
  } else {
    let n = grid.particle_count;
    var j: u32 = 0u;
    loop {
      if (j >= n) { break; }
      if (j != i) {
        let u = vertices.data[j];
        let w = uniforms.resolution.x;
        let h = uniforms.resolution.y;
        var dx = u.position.x - v.position.x;
        var dy = u.position.y - v.position.y;
        if (dx >  0.5 * w) { dx = dx - w; }
        if (dx < -0.5 * w) { dx = dx + w; }
        if (dy >  0.5 * h) { dy = dy - h; }
        if (dy < -0.5 * h) { dy = dy + h; }
        let d2 = dx*dx + dy*dy;
        if (d2 <= r2) {
          if (u.state >= uniforms.alive_threshold) { alive_n = alive_n + 1u; } else { dead_n = dead_n + 1u; }
        }
      }
      j = j + 1u;
    }
  }
  v.alive_neighbors = alive_n;
  v.dead_neighbors = dead_n;

  vertices.data[i] = v;
}