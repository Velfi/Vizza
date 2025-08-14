// Fullscreen Voronoi renderer: shades regions by sampling LUT

struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }

// Matches Rust bind group layout:
// 0: jfa texture (sampled), 1: VoronoiParams (uniform), 2: vertices (storage), 3: LUT (storage)
@group(0) @binding(0) var cellTex : texture_2d<f32>;

struct VoronoiParams {
  jfa_width: f32,
  jfa_height: f32,
  count: f32,
  color_mode: f32,
  neighbor_radius: f32,
  border_enabled: f32,
  border_threshold: f32,
  filter_mode: f32,
};
@group(0) @binding(1) var<uniform> params: VoronoiParams;

struct Vertex {
  position: vec2<f32>,
  state: f32,
  pad0: f32,
  age: f32,
  alive_neighbors: u32,
  dead_neighbors: u32,
  pad1: u32,
};
@group(0) @binding(2) var<storage, read> vertices: array<Vertex>;

// LUT buffer in planar format [r0..r255, g0..r255, b0..b255]
@group(0) @binding(3) var<storage, read> lut_data: array<u32>;

fn srgb_to_linear(srgb: f32) -> f32 {
  if (srgb <= 0.04045) { return srgb / 12.92; }
  return pow((srgb + 0.055) / 1.055, 2.4);
}

fn lut_sample(intensity: f32) -> vec3<f32> {
  let lut_index = clamp(intensity * 255.0, 0.0, 255.0);
  let idx = u32(lut_index);
  let r_srgb = f32(lut_data[idx]) / 255.0;
  let g_srgb = f32(lut_data[idx + 256u]) / 255.0;
  let b_srgb = f32(lut_data[idx + 512u]) / 255.0;
  return vec3<f32>(srgb_to_linear(r_srgb), srgb_to_linear(g_srgb), srgb_to_linear(b_srgb));
}

fn hash_u32_to_unit_float(n: u32) -> f32 {
  // Simple integer hash -> [0,1)
  var x = n;
  x ^= x >> 16u;
  x *= 0x7feb352du;
  x ^= x >> 15u;
  x *= 0x846ca68bu;
  x ^= x >> 16u;
  return f32(x) / 4294967295.0;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
  var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -3.0), vec2<f32>(3.0, 1.0), vec2<f32>(-1.0, 1.0));
  var out: VSOut;
  let pos = p[vi];
  out.pos = vec4<f32>(pos, 0.0, 1.0);
  // Flip Y so UV origin matches textureLoad's top-left origin
  out.uv = vec2<f32>(pos.x * 0.5 + 0.5, -pos.y * 0.5 + 0.5);
  return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
  let uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(0.999999));
  let dims = vec2<u32>(textureDimensions(cellTex));
  let fdim = vec2<f32>(dims);
  let xy = vec2<i32>(clamp(vec2<i32>(vec2<f32>(uv * fdim)), vec2<i32>(0), vec2<i32>(i32(fdim.x) - 1, i32(fdim.y) - 1)));
  
  // Brute-force Voronoi: find closest site directly
  let pixel_pos = vec2<f32>(uv * fdim);
  var closest_site: u32 = 0u;
  var min_distance: f32 = 1e30;
  
  // Check all sites to find the closest one
  for (var i: u32 = 0u; i < u32(params.count); i = i + 1u) {
    let v = vertices[i];
    let site_pos = v.position;
    
    // Toroidal distance calculation
    let w = fdim.x;
    let h = fdim.y;
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
    }
  }
  
  // Get the closest site's data
  let v = vertices[closest_site];

  // Border detection: check if we're near a Voronoi edge
  var is_border = false;
  if (params.border_enabled >= 0.5) {
    // Find the second-closest site
    var second_closest_site: u32 = 0u;
    var second_min_distance: f32 = 1e30;
    
    for (var i: u32 = 0u; i < u32(params.count); i = i + 1u) {
      if (i == closest_site) { continue; } // Skip the closest site
      
      let v2 = vertices[i];
      let site_pos = v2.position;
      
      // Toroidal distance calculation
      let w = fdim.x;
      let h = fdim.y;
      var dx = site_pos.x - pixel_pos.x;
      var dy = site_pos.y - pixel_pos.y;
      if (dx >  0.5 * w) { dx = dx - w; }
      if (dx < -0.5 * w) { dx = dx + w; }
      if (dy >  0.5 * h) { dy = dy - h; }
      if (dy < -0.5 * h) { dy = dy + h; }
      
      let distance = dx*dx + dy*dy;
      if (distance < second_min_distance) {
        second_min_distance = distance;
        second_closest_site = i;
      }
    }
    
    // If the two closest sites are very close in distance, we're near a border
    let distance_ratio = min_distance / second_min_distance;
    // For borders: we want to detect when distances are similar (ratio close to 1.0)
    // Threshold should be 0.0 (no borders) to 1.0 (all borders)
    // Convert threshold to a ratio threshold: 0.0 -> 0.5, 1.0 -> 0.99
    let ratio_threshold = 0.5 + (params.border_threshold * 0.49);
    is_border = distance_ratio > ratio_threshold;
  }

  // Optionally apply filtering mode hint by nudging intensity sampling
  // (Nearest/Linear/Lanczos handled mostly by the post-process or separate pipelines; here just a hook)
  var intensity: f32;
  if (params.color_mode < 0.5) {
    // Random color per cell based on ID
    intensity = hash_u32_to_unit_float(closest_site);
  } else if (params.color_mode < 1.5) {
    // Density coloring using local alive ratio in neighborhood
    let total = max(1u, v.alive_neighbors + v.dead_neighbors);
    intensity = clamp(f32(v.alive_neighbors) / f32(total), 0.0, 1.0);
  } else if (params.color_mode < 2.5) {
    // Age-based coloring (assumes age increases over time, scale loosely)
    intensity = clamp(v.age * 0.25, 0.0, 1.0);
  } else {
    // Binary: dead=0, alive=1
    intensity = select(0.0, 1.0, v.state >= 0.5);
  }

  // If borders enabled, override with mid color at borders
  if (params.border_enabled >= 0.5 && is_border) {
    intensity = 0.5;
  }

  let col = lut_sample(intensity);
  return vec4<f32>(col, 1.0);
}


