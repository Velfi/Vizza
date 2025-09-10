// Build per-site Voronoi adjacency by casting rays from each site into the JFA texture.
// Each invocation handles one site; deduplicates neighbors locally and writes a capped list.

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

// Adjacency buffers
struct NeighborList { data: array<u32> }            // flattened [numSites * MAX_NEIGHBORS]
struct Degrees { data: array<u32> }                 // per-site degree

@group(0) @binding(0) var<storage, read> vertices: Vertices;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<storage, read_write> neighbors: NeighborList;
@group(0) @binding(3) var<storage, read_write> degrees: Degrees;
@group(0) @binding(4) var jfa_texture: texture_2d<f32>;

const MAX_NEIGHBORS: u32 = 16u;

fn write_neighbor(site_index: u32, slot: u32, neighbor_id: u32) {
  let base = site_index * MAX_NEIGHBORS;
  neighbors.data[base + slot] = neighbor_id;
}

fn get_site_index_at_pixel(px: vec2<i32>) -> u32 {
  let texel = textureLoad(jfa_texture, px, 0);
  return u32(texel.b);
}

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  let count = arrayLength(&vertices.data);
  if (i >= count) { return; }

  let res = vec2<i32>(i32(uniforms.resolution.x), i32(uniforms.resolution.y));
  let pos = vertices.data[i].position;

  // Convert to pixel coords and clamp inside texture
  var pixel = vec2<i32>(clamp(vec2<i32>(vec2<f32>(pos)), vec2<i32>(0), res - vec2<i32>(1)));
  // Ensure starting site matches i; if not, locate nearest pixel for this site by probing a tiny area
  var center_site = get_site_index_at_pixel(pixel);
  if (center_site != i) {
    // Probe small 3x3 neighborhood to find a pixel owned by this site
    var found = false;
    for (var dy = -1; dy <= 1 && !found; dy = dy + 1) {
      for (var dx = -1; dx <= 1 && !found; dx = dx + 1) {
        let q = pixel + vec2<i32>(dx, dy);
        let wrapped = vec2<i32>((q.x + res.x) % res.x, (q.y + res.y) % res.y);
        let s = get_site_index_at_pixel(wrapped);
        if (s == i) { pixel = wrapped; center_site = s; found = true; }
      }
    }
  }

  // Local dedup buffer
  var local_n: array<u32, 32u>;
  var local_len: u32 = 0u;

  let sample_count: i32 = 24;
  let max_steps: i32 = 1024;
  let step_size: f32 = 2.0; // pixels per step
  let two_pi = 6.2831853;

  for (var k = 0; k < sample_count; k = k + 1) {
    let angle = f32(k) * (two_pi / f32(sample_count));
    let dir = vec2<f32>(cos(angle), sin(angle));

    var t: f32 = step_size;
    var neighbor_id: u32 = center_site;
    var hit = false;
    var step = 0;
    loop {
      if (step >= max_steps) { break; }
      step = step + 1;
      let p = vec2<f32>(pos + dir * t);
      let q = vec2<i32>(
        (i32(floor(p.x)) % res.x + res.x) % res.x,
        (i32(floor(p.y)) % res.y + res.y) % res.y
      );
      let s = get_site_index_at_pixel(q);
      if (s != center_site) { neighbor_id = s; hit = true; break; }
      t = t + step_size;
    }

    if (hit) {
      // Dedup locally
      var exists = false;
      for (var m = 0u; m < local_len; m = m + 1u) {
        if (local_n[m] == neighbor_id) { exists = true; break; }
      }
      if (!exists && local_len < MAX_NEIGHBORS) {
        local_n[local_len] = neighbor_id;
        local_len = local_len + 1u;
      }
    }
  }

  // Write out degree and neighbor list (pad unspecified entries)
  degrees.data[i] = local_len;
  for (var m = 0u; m < local_len; m = m + 1u) {
    write_neighbor(i, m, local_n[m]);
  }
}


