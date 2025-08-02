pub const VERTEX_SHADER: &str = include_str!("vertex.wgsl");
pub const FRAGMENT_SHADER: &str = include_str!("fragment.wgsl");

// Combined shader with both vertex and fragment stages
pub const COMBINED_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    var uv = (pos[vertex_index] + vec2<f32>(1.0, 1.0)) * 0.5;
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uv;
    return out;
}

@group(0) @binding(0)
var<uniform> time: f32;

@group(1) @binding(0)
var<storage, read> lut_data: array<u32>;

// 3D Simplex noise implementation
fn mod289_3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_4(x: vec4<f32>) -> vec4<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute_3(x: vec3<f32>) -> vec3<f32> {
    return mod289_3(((x * 34.0) + 1.0) * x);
}

fn permute_4(x: vec4<f32>) -> vec4<f32> {
    return mod289_4(((x * 34.0) + 1.0) * x);
}

fn taylor_inv_sqrt(r: vec4<f32>) -> vec4<f32> {
    return 1.79284291400159 - 0.85373472095314 * r;
}

fn simplex_noise_3d(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0 / 6.0, 1.0 / 3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

    // First corner
    let i = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy;
    let x3 = x0 - D.yyy;

    // Permutations
    let i_mod = mod289_3(i);
    let p = permute_4(permute_4(permute_4(
        i_mod.z + vec4<f32>(0.0, i1.z, i2.z, 1.0)
    ) + i_mod.y + vec4<f32>(0.0, i1.y, i2.y, 1.0)) + i_mod.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));

    let n_ = 0.142857142857;
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z * ns.z);

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    let p0 = vec3<f32>(a0.xy, h.x);
    let p1 = vec3<f32>(a0.zw, h.y);
    let p2 = vec3<f32>(a1.xy, h.z);
    let p3 = vec3<f32>(a1.zw, h.w);

    // Normalise gradients
    let norm = taylor_inv_sqrt(vec4<f32>(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    let p0_norm = p0 * norm.x;
    let p1_norm = p1 * norm.y;
    let p2_norm = p2 * norm.z;
    let p3_norm = p3 * norm.w;

    // Mix final noise value
    let m = max(0.6 - vec4<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), vec4<f32>(0.0));
    let m_squared = m * m;
    return 42.0 * dot(m_squared, vec4<f32>(dot(p0_norm, x0), dot(p1_norm, x1), dot(p2_norm, x2), dot(p3_norm, x3)));
}

// Multivariate FBM with more octaves and parameter variation
fn multivariate_fbm(p: vec3<f32>, t: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var sum = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
        let offset = vec3<f32>(
            sin(t * (0.3 + 0.1 * f32(i))) * 2.0 * f32(i),
            cos(t * (0.2 + 0.13 * f32(i))) * 2.0 * f32(i),
            t * (0.1 + 0.07 * f32(i))
        );
        sum += amplitude * simplex_noise_3d(p * frequency + offset);
        frequency *= 1.85 + 0.15 * sin(t + f32(i));
        amplitude *= 0.55 + 0.1 * cos(t * 0.5 + f32(i));
    }
    return sum;
}

// Convert from sRGB (gamma-corrected) to linear RGB
fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

// LUT lookup function
fn lookup_lut(value: f32) -> vec3<f32> {
    let index = u32(value * 255.0);
    let r_srgb = f32(lut_data[index]) / 255.0;
    let g_srgb = f32(lut_data[256 + index]) / 255.0;
    let b_srgb = f32(lut_data[512 + index]) / 255.0;
    
    return vec3<f32>(
        srgb_to_linear(r_srgb),
        srgb_to_linear(g_srgb),
        srgb_to_linear(b_srgb)
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Move the pole to the top of the screen (Y=1.0 is top in screen coordinates)
    let pole = vec2<f32>(0.5, 0.7);
    let delta = uv - pole;
    let angle = atan2(delta.y, delta.x);
    let radius = length(delta);
    let swirl = sin(angle * 3.0 + time + radius * 4.0) * 0.5 + 0.5;

    // Multivariate FBM noise
    let pos_3d = vec3<f32>(uv.x * 4.0, uv.y * 4.0, time * 0.5);
    let noise1 = multivariate_fbm(pos_3d, time);
    let noise2 = multivariate_fbm(pos_3d * 1.7 + vec3<f32>(time * 0.3, time * 0.2, time * 0.4), time * 0.7);
    let noise3 = multivariate_fbm(pos_3d * 0.8 - vec3<f32>(time * 0.2, time * 0.3, time * 0.1), time * 1.3);
    let combined_noise = (noise1 + noise2 + noise3) / 3.0;
    let animated_pattern = combined_noise * swirl;

    // Normalize the pattern to 0-1 range and apply LUT
    let normalized_value = (animated_pattern + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
    let color = lookup_lut(normalized_value);

    // Pulsing
    let pulse = sin(time * 2.0) * 0.1 + 0.9;
    let final_color = color * pulse;

    // Vignette
    let vignette = 1.0 - length(uv - 0.5) * 0.8;
    let final_result = final_color * vignette;

    return vec4<f32>(final_result, 1.0);
}
"#;
