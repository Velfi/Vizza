// Fullscreen Voronoi renderer using JFA texture: shades regions by sampling LUT
// This version uses the pre-computed JFA texture instead of brute-force search

struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }

// Matches Rust bind group layout:
// 0: VoronoiParams (uniform), 1: vertices (storage), 2: LUT (storage), 3: JFA texture

struct VoronoiParams {
  count: f32,
  color_mode: f32,
  neighbor_radius: f32,
  border_enabled: f32,
  border_width: f32,
  filter_mode: f32,
  resolution_x: f32,
  resolution_y: f32,
  jump_distance: f32,
};
@group(0) @binding(0) var<uniform> params: VoronoiParams;

struct Vertex {
  position: vec2<f32>,
  state: f32,
  pad0: f32,
  age: f32,
  alive_neighbors: u32,
  dead_neighbors: u32,
  pad1: u32,
};
@group(0) @binding(1) var<storage, read> vertices: array<Vertex>;

// LUT buffer in planar format [r0..r255, g0..r255, b0..b255]
@group(0) @binding(2) var<storage, read> lut_data: array<u32>;

// JFA texture: RGBA32Float
// R,G: site position (x,y)
// B: site index (as float)
// A: distance to site (squared)
@group(0) @binding(3) var jfa_texture: texture_2d<f32>;

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
  
  // Load JFA texture to get closest site information
  let fdim = vec2<f32>(params.resolution_x, params.resolution_y);
  let pixel_coord = vec2<i32>(clamp(vec2<i32>(vec2<f32>(uv * fdim)), vec2<i32>(0), vec2<i32>(i32(fdim.x) - 1, i32(fdim.y) - 1)));
  let jfa_data = textureLoad(jfa_texture, pixel_coord, 0);
  let closest_site_index = u32(jfa_data.b);
  let closest_distance = jfa_data.a;
  
  // Get the closest site's data
  let v = vertices[closest_site_index];

  var is_border = false;
  if (params.border_enabled >= 0.5 && params.border_width > 0.0) {
    var found_different_site = false;
    let border_width = params.border_width;
    
    if (border_width <= 4.0) {
      let check_radius = i32(ceil(border_width));
      for (var dy = -check_radius; dy <= check_radius && !found_different_site; dy = dy + 1) {
        for (var dx = -check_radius; dx <= check_radius && !found_different_site; dx = dx + 1) {
          if (dx == 0 && dy == 0) { continue; }
          
          let distance = sqrt(f32(dx * dx + dy * dy));
          if (distance > border_width) { continue; }
          
          let check_coord = pixel_coord + vec2<i32>(dx, dy);
          let wrapped_coord = vec2<i32>(
            (check_coord.x + i32(fdim.x)) % i32(fdim.x),
            (check_coord.y + i32(fdim.y)) % i32(fdim.y)
          );
          let check_jfa_data = textureLoad(jfa_texture, wrapped_coord, 0);
          let check_site_index = u32(check_jfa_data.b);
          
          if (check_site_index != closest_site_index) {
            found_different_site = true;
          }
        }
      }
    } else {
      let sample_count = min(i32(border_width), 16);
      let angle_step = 6.28318 / f32(sample_count);
      
      for (var i = 0; i < sample_count && !found_different_site; i = i + 1) {
        let angle = f32(i) * angle_step;
        let dx = i32(round(cos(angle) * border_width));
        let dy = i32(round(sin(angle) * border_width));
        
        let check_coord = pixel_coord + vec2<i32>(dx, dy);
        let wrapped_coord = vec2<i32>(
          (check_coord.x + i32(fdim.x)) % i32(fdim.x),
          (check_coord.y + i32(fdim.y)) % i32(fdim.y)
        );
        let check_jfa_data = textureLoad(jfa_texture, wrapped_coord, 0);
        let check_site_index = u32(check_jfa_data.b);
        
        if (check_site_index != closest_site_index) {
          found_different_site = true;
        }
      }
    }
    
    is_border = found_different_site;
  }

  // Optionally apply filtering mode hint by nudging intensity sampling
  var intensity: f32;
  if (params.color_mode < 0.5) {
    // Random color per cell based on ID
    intensity = hash_u32_to_unit_float(closest_site_index);
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
