// JFA iteration shader - performs one step of the jump flood algorithm

struct VoronoiParams {
  count: f32,
  color_mode: f32,
  border_enabled: f32,
  border_threshold: f32,
  filter_mode: f32,
  resolution_x: f32,
  resolution_y: f32,
  jump_distance: f32,
}

@group(0) @binding(0) var<uniform> params: VoronoiParams;
@group(0) @binding(1) var voronoi_texture_read: texture_2d<f32>;
@group(0) @binding(2) var voronoi_texture_write: texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let coord = vec2<i32>(gid.xy);
  let dims = vec2<i32>(i32(params.resolution_x), i32(params.resolution_y));
  
  if (coord.x >= dims.x || coord.y >= dims.y) { return; }
  
  let pixel_pos = vec2<f32>(f32(coord.x), f32(coord.y));
  
  // Current best
  var best_site: u32 = 0u;
  var best_distance: f32 = 1e30;
  var best_pos: vec2<f32> = vec2<f32>(0.0);
  
  // Load from current pixel
  let current = textureLoad(voronoi_texture_read, coord, 0);
  if (current.a < best_distance) {
    best_distance = current.a;
    best_site = u32(current.b);
    best_pos = current.rg;
  }
  
  // Sample from jump offset positions
  // JFA uses powers of 2 for jump distances
  let jump_distance = params.jump_distance;
  
  for (var dy = -1; dy <= 1; dy = dy + 1) {
    for (var dx = -1; dx <= 1; dx = dx + 1) {
      if (dx == 0 && dy == 0) { continue; }
      
      let offset = vec2<i32>(dx, dy) * i32(jump_distance);
      let sample_coord = coord + offset;
      
      // Toroidal wrapping
      let wrapped_coord = vec2<i32>(
        (sample_coord.x + dims.x) % dims.x,
        (sample_coord.y + dims.y) % dims.y
      );
      let sample = textureLoad(voronoi_texture_read, wrapped_coord, 0);
      
      // Calculate distance from this pixel to the sampled site
      let sampled_pos = sample.rg;
      let w = params.resolution_x;
      let h = params.resolution_y;
      var dist_x = sampled_pos.x - pixel_pos.x;
      var dist_y = sampled_pos.y - pixel_pos.y;
      if (dist_x >  0.5 * w) { dist_x = dist_x - w; }
      if (dist_x < -0.5 * w) { dist_x = dist_x + w; }
      if (dist_y >  0.5 * h) { dist_y = dist_y - h; }
      if (dist_y < -0.5 * h) { dist_y = dist_y + h; }
      
      let distance = dist_x*dist_x + dist_y*dist_y;
      
      if (distance < best_distance) {
        best_distance = distance;
        best_site = u32(sample.b);
        best_pos = sampled_pos;
      }
    }
  }
  
  // Write result
  textureStore(voronoi_texture_write, coord, vec4<f32>(best_pos, f32(best_site), best_distance));
}
