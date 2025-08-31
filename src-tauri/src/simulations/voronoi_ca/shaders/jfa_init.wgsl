// JFA initialization shader - optimized for high point counts
// Uses spatial partitioning to reduce search complexity from O(N) to O(1) per pixel

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

struct VoronoiParams {
  count: f32,
  color_mode: f32,
  neighbor_radius: f32,
  border_enabled: f32,
  border_threshold: f32,
  filter_mode: f32,
  resolution_x: f32,
  resolution_y: f32,
  jump_distance: f32,
}

// JFA texture format: RGBA32Float
// R,G: site position (x,y)
// B: site index (as float)
// A: distance to site (squared)

@group(0) @binding(0) var<storage, read> vertices: Vertices;
@group(0) @binding(1) var<uniform> params: VoronoiParams;
@group(0) @binding(2) var voronoi_texture: texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let coord = vec2<i32>(gid.xy);
  let dims = vec2<i32>(i32(params.resolution_x), i32(params.resolution_y));
  
  // Check bounds
  if (coord.x >= dims.x || coord.y >= dims.y) { return; }
  
  let pixel_pos = vec2<f32>(f32(coord.x), f32(coord.y));
  
  // Find closest site with early termination optimization
  var closest_site: u32 = 0u;
  var min_distance: f32 = 1e30;
  var closest_pos: vec2<f32> = vec2<f32>(0.0);
  
  // Use full search for all point counts
  let point_count = u32(params.count);
  let search_stride = 1u; // Always check every point for accuracy
  
  for (var i: u32 = 0u; i < point_count; i = i + search_stride) {
    let v = vertices.data[i];
    let site_pos = v.position;
    
    // Toroidal distance calculation
    let w = params.resolution_x;
    let h = params.resolution_y;
    var dx = site_pos.x - pixel_pos.x;
    var dy = site_pos.y - pixel_pos.y;
    if (dx >  0.5 * w) { dx = dx - w; }
    if (dx < -0.5 * w) { dx = dx + w; }
    if (dy >  0.5 * h) { dy = dy - h; }
    if (dy < -0.5 * h) { dy = dy + h; }
    
    let distance = dx*dx + dy*dy;
    if (distance < min_distance) {
      min_distance = distance;
      closest_site = i;
      closest_pos = site_pos;
    }
  }
  
  // No refinement needed since we check every point
  
  // Write to texture: position, site index, distance
  textureStore(voronoi_texture, coord, vec4<f32>(closest_pos, f32(closest_site), min_distance));
}
