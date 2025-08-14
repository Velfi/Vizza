struct JfaParamsStep {
  width: u32,
  height: u32,
  step: u32,
  _pad: u32,
};

@group(0) @binding(0) var srcTex: texture_2d<f32>;
@group(0) @binding(1) var dstTex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var<uniform> params: JfaParamsStep;

fn wrap(v:i32, m:i32) -> i32 {
  return ((v % m) + m) % m;
}

fn toroidal_d2(ax:f32, ay:f32, sx:f32, sy:f32, w:f32, h:f32) -> f32 {
  var dx = sx - ax;
  var dy = sy - ay;
  if (dx >  0.5 * w) { dx = dx - w; }
  if (dx < -0.5 * w) { dx = dx + w; }
  if (dy >  0.5 * h) { dy = dy - h; }
  if (dy < -0.5 * h) { dy = dy + h; }
  return dx*dx + dy*dy;
}

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= params.width || gid.y >= params.height) { return; }
  let W = i32(params.width);
  let H = i32(params.height);
  let s = i32(params.step);
  let p = vec2<i32>(i32(gid.x), i32(gid.y));
  
  // Get current pixel's best site
  var best = textureLoad(srcTex, p, 0);
  var best_d2: f32 = 1e30;
  
  // If we have a valid site, calculate its distance
  if (best.z >= 0.0) {
    best_d2 = toroidal_d2(f32(p.x), f32(p.y), best.x, best.y, f32(W), f32(H));
  }

  // Sample 8 neighbors at step distance
  let offsets = array<vec2<i32>,8>(
    vec2<i32>( s, 0), vec2<i32>(-s, 0), vec2<i32>(0, s), vec2<i32>(0,-s),
    vec2<i32>( s, s), vec2<i32>(-s, s), vec2<i32>( s,-s), vec2<i32>(-s,-s)
  );

  // Find the closest site among neighbors
  for (var k:i32=0; k<8; k=k+1) {
    let q = vec2<i32>(wrap(p.x + offsets[u32(k)].x, W), wrap(p.y + offsets[u32(k)].y, H));
    let cand = textureLoad(srcTex, q, 0);
    
    if (cand.z >= 0.0) {
      let d2 = toroidal_d2(f32(p.x), f32(p.y), cand.x, cand.y, f32(W), f32(H));
      if (d2 < best_d2) { 
        best = cand; 
        best_d2 = d2; 
      }
    }
  }
  
  // Write the best site found
  textureStore(dstTex, p, best);
}


