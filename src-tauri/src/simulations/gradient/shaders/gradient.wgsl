struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    return VertexOutput(
        vec4<f32>(input.position, 0.0, 1.0),
        input.uv,
    );
}

// LUT buffer - contains 256 RGB values in planar format [r0..r255, g0..g255, b0..b255]
@group(0) @binding(0) var<storage, read> lut: array<u32>;

// Parameters uniform buffer
struct GradientParams {
    display_mode: u32, // 0 = smooth, 1 = dithered
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(1) var<uniform> params: GradientParams;

// Color space interpolation functions
fn rgb_to_linear(rgb: f32) -> f32 {
    let normalized = rgb / 255.0;
    return select(
        normalized / 12.92,
        pow((normalized + 0.055) / 1.055, 2.4),
        normalized > 0.04045
    );
}

fn linear_to_rgb(linear: f32) -> f32 {
    let result = select(
        linear * 12.92,
        1.055 * pow(linear, 1.0 / 2.4) - 0.055,
        linear > 0.0031308
    );
    return clamp(result * 255.0, 0.0, 255.0);
}

fn linear_rgb_to_xyz(r: f32, g: f32, b: f32) -> vec3<f32> {
    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.072175;
    let z = r * 0.0193339 + g * 0.119192 + b * 0.9503041;
    return vec3<f32>(x, y, z);
}

fn xyz_to_linear_rgb(x: f32, y: f32, z: f32) -> vec3<f32> {
    let r = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
    let g = x * -0.969266 + y * 1.8760108 + z * 0.041556;
    let b = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;
    return vec3<f32>(r, g, b);
}

fn xyz_to_lab(x: f32, y: f32, z: f32) -> vec3<f32> {
    // D65 white point
    let xn = 0.95047;
    let yn = 1.0;
    let zn = 1.08883;

    let xr = x / xn;
    let yr = y / yn;
    let zr = z / zn;

    let fx = select((7.787 * xr) + (16.0 / 116.0), pow(xr, 1.0/3.0), xr > 0.008856);
    let fy = select((7.787 * yr) + (16.0 / 116.0), pow(yr, 1.0/3.0), yr > 0.008856);
    let fz = select((7.787 * zr) + (16.0 / 116.0), pow(zr, 1.0/3.0), zr > 0.008856);

    let L = (116.0 * fy) - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    return vec3<f32>(L, a, b);
}

fn lab_to_xyz(L: f32, a: f32, b: f32) -> vec3<f32> {
    // D65 white point
    let xn = 0.95047;
    let yn = 1.0;
    let zn = 1.08883;

    let fy = (L + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;

    let xr = select((fx - 16.0 / 116.0) / 7.787, pow(fx, 3.0), fx > 0.206897);
    let yr = select((fy - 16.0 / 116.0) / 7.787, pow(fy, 3.0), fy > 0.206897);
    let zr = select((fz - 16.0 / 116.0) / 7.787, pow(fz, 3.0), fz > 0.206897);

    let x = xr * xn;
    let y = yr * yn;
    let z = zr * zn;

    return vec3<f32>(x, y, z);
}

fn xyz_to_oklab(x: f32, y: f32, z: f32) -> vec3<f32> {
    let l = 0.8189330101 * x + 0.3618667424 * y - 0.1288597137 * z;
    let m = 0.0329845436 * x + 0.9293118715 * y + 0.0361456387 * z;
    let s = 0.0482003018 * x + 0.2643662691 * y + 0.6338517070 * z;

    let l_ = pow(l, 1.0/3.0);
    let m_ = pow(m, 1.0/3.0);
    let s_ = pow(s, 1.0/3.0);

    let L = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    return vec3<f32>(L, a, b);
}

fn oklab_to_xyz(L: f32, a: f32, b: f32) -> vec3<f32> {
    let l_ = L + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = L - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = L - 0.0894841775 * a - 1.291485548 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let x = 1.2268798733 * l - 0.5578149965 * m + 0.2813910456 * s;
    let y = -0.0405801784 * l + 1.1122568696 * m - 0.0716766787 * s;
    let z = -0.0763812845 * l - 0.4214819784 * m + 1.5861632204 * s;

    return vec3<f32>(x, y, z);
}

fn sample_lut(index: f32) -> vec3<f32> {
    // Bilinear interpolation for smooth gradient
    let scaled_index = clamp(index * 255.0, 0.0, 255.0);
    let idx0 = u32(floor(scaled_index));
    let idx1 = min(idx0 + 1u, 255u);
    let t = scaled_index - f32(idx0);
    
    // Sample colors at both indices (keep in sRGB for display)
    let color0 = vec3<f32>(
        f32(lut[idx0]),
        f32(lut[idx0 + 256u]),
        f32(lut[idx0 + 512u])
    );
    let color1 = vec3<f32>(
        f32(lut[idx1]),
        f32(lut[idx1 + 256u]),
        f32(lut[idx1 + 512u])
    );
    
    // Linear interpolation between the two colors
    return mix(color0, color1, t);
}

// 16x16 Bayer matrix for very fine ordered dithering
fn bayer_dither(uv: vec2<f32>) -> f32 {
    let x = u32(fract(uv.x * 16.0) * 16.0);
    let y = u32(fract(uv.y * 16.0) * 16.0);
    
    // 16x16 Bayer matrix for very fine dithering
    let bayer_matrix = array<u32, 256>(
        0u, 128u, 32u, 160u, 8u, 136u, 40u, 168u, 2u, 130u, 34u, 162u, 10u, 138u, 42u, 170u,
        192u, 64u, 224u, 96u, 200u, 72u, 232u, 104u, 194u, 66u, 226u, 98u, 202u, 74u, 234u, 106u,
        48u, 176u, 16u, 144u, 56u, 184u, 24u, 152u, 50u, 178u, 18u, 146u, 58u, 186u, 26u, 154u,
        240u, 112u, 208u, 80u, 248u, 120u, 216u, 88u, 242u, 114u, 210u, 82u, 250u, 122u, 218u, 90u,
        12u, 140u, 44u, 172u, 4u, 132u, 36u, 164u, 14u, 142u, 46u, 174u, 6u, 134u, 38u, 166u,
        204u, 76u, 236u, 108u, 196u, 68u, 228u, 100u, 206u, 78u, 238u, 110u, 198u, 70u, 230u, 102u,
        60u, 188u, 28u, 156u, 52u, 180u, 20u, 148u, 62u, 190u, 30u, 158u, 54u, 182u, 22u, 150u,
        252u, 124u, 220u, 92u, 244u, 116u, 212u, 84u, 254u, 126u, 222u, 94u, 246u, 118u, 214u, 86u,
        3u, 131u, 35u, 163u, 11u, 139u, 43u, 171u, 1u, 129u, 33u, 161u, 9u, 137u, 41u, 169u,
        195u, 67u, 227u, 99u, 203u, 75u, 235u, 107u, 193u, 65u, 225u, 97u, 201u, 73u, 233u, 105u,
        51u, 179u, 19u, 147u, 59u, 187u, 27u, 155u, 49u, 177u, 17u, 145u, 57u, 185u, 25u, 153u,
        243u, 115u, 211u, 83u, 251u, 123u, 219u, 91u, 241u, 113u, 209u, 81u, 249u, 121u, 217u, 89u,
        15u, 143u, 47u, 175u, 7u, 135u, 39u, 167u, 13u, 141u, 45u, 173u, 5u, 133u, 37u, 165u,
        207u, 79u, 239u, 111u, 199u, 71u, 231u, 103u, 205u, 77u, 237u, 109u, 197u, 69u, 229u, 101u,
        63u, 191u, 31u, 159u, 55u, 183u, 23u, 151u, 61u, 189u, 29u, 157u, 53u, 181u, 21u, 149u,
        255u, 127u, 223u, 95u, 247u, 119u, 215u, 87u, 253u, 125u, 221u, 93u, 245u, 117u, 213u, 85u
    );
    
    let index = y * 16u + x;
    return f32(bayer_matrix[index]) / 256.0;
}

// Quantize color to limited palette (Amiga-style)
fn quantize_color(color: vec3<f32>) -> vec3<f32> {
    // Quantize to 16 levels (4 bits per channel) for more pixelated look
    let levels = 16.0;
    let step = 255.0 / (levels - 1.0);
    
    return vec3<f32>(
        round(color.x / step) * step,
        round(color.y / step) * step,
        round(color.z / step) * step
    );
}

// Apply display mode effects to color
fn apply_display_mode(color: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    if (params.display_mode == 0u) {
        // Smooth mode - no modification
        return color;
    } else if (params.display_mode == 1u) {
        // Dithered mode - Amiga-style ordered dithering
        
        // Get dither threshold from Bayer matrix
        let dither_threshold = bayer_dither(uv);
        
        // Quantize the color first
        let quantized = quantize_color(color);
        
        // Apply dithering by comparing each channel to the threshold
        let step_size = 255.0 / 15.0; // Step size for 16 levels
        let dithered = vec3<f32>(
            select(quantized.x, quantized.x + step_size, color.x > quantized.x + (dither_threshold * step_size)),
            select(quantized.y, quantized.y + step_size, color.y > quantized.y + (dither_threshold * step_size)),
            select(quantized.z, quantized.z + step_size, color.z > quantized.z + (dither_threshold * step_size))
        );
        
        // Clamp to valid range
        return clamp(dithered, vec3<f32>(0.0), vec3<f32>(255.0));
    } else {
        // Default to smooth mode for unknown values
        return color;
    }
}

fn interpolate_rgb(color1: vec3<f32>, color2: vec3<f32>, t: f32) -> vec3<f32> {
    return mix(color1, color2, t);
}

fn interpolate_lab(color1: vec3<f32>, color2: vec3<f32>, t: f32) -> vec3<f32> {
    // Convert to linear RGB first
    let linear1 = vec3<f32>(
        rgb_to_linear(color1.x),
        rgb_to_linear(color1.y),
        rgb_to_linear(color1.z)
    );
    let linear2 = vec3<f32>(
        rgb_to_linear(color2.x),
        rgb_to_linear(color2.y),
        rgb_to_linear(color2.z)
    );

    // Convert to XYZ
    let xyz1 = linear_rgb_to_xyz(linear1.x, linear1.y, linear1.z);
    let xyz2 = linear_rgb_to_xyz(linear2.x, linear2.y, linear2.z);

    // Convert to Lab
    let lab1 = xyz_to_lab(xyz1.x, xyz1.y, xyz1.z);
    let lab2 = xyz_to_lab(xyz2.x, xyz2.y, xyz2.z);

    // Interpolate in Lab space
    let lab_interp = mix(lab1, lab2, t);

    // Convert back to XYZ
    let xyz_interp = lab_to_xyz(lab_interp.x, lab_interp.y, lab_interp.z);

    // Convert back to linear RGB
    let linear_interp = xyz_to_linear_rgb(xyz_interp.x, xyz_interp.y, xyz_interp.z);

    // Convert back to sRGB
    return vec3<f32>(
        linear_to_rgb(linear_interp.x),
        linear_to_rgb(linear_interp.y),
        linear_to_rgb(linear_interp.z)
    );
}

fn interpolate_oklab(color1: vec3<f32>, color2: vec3<f32>, t: f32) -> vec3<f32> {
    // Convert to linear RGB first
    let linear1 = vec3<f32>(
        rgb_to_linear(color1.x),
        rgb_to_linear(color1.y),
        rgb_to_linear(color1.z)
    );
    let linear2 = vec3<f32>(
        rgb_to_linear(color2.x),
        rgb_to_linear(color2.y),
        rgb_to_linear(color2.z)
    );

    // Convert to XYZ
    let xyz1 = linear_rgb_to_xyz(linear1.x, linear1.y, linear1.z);
    let xyz2 = linear_rgb_to_xyz(linear2.x, linear2.y, linear2.z);

    // Convert to OkLab
    let oklab1 = xyz_to_oklab(xyz1.x, xyz1.y, xyz1.z);
    let oklab2 = xyz_to_oklab(xyz2.x, xyz2.y, xyz2.z);

    // Interpolate in OkLab space
    let oklab_interp = mix(oklab1, oklab2, t);

    // Convert back to XYZ
    let xyz_interp = oklab_to_xyz(oklab_interp.x, oklab_interp.y, oklab_interp.z);

    // Convert back to linear RGB
    let linear_interp = xyz_to_linear_rgb(xyz_interp.x, xyz_interp.y, xyz_interp.z);

    // Convert back to sRGB
    return vec3<f32>(
        linear_to_rgb(linear_interp.x),
        linear_to_rgb(linear_interp.y),
        linear_to_rgb(linear_interp.z)
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Use X coordinate for gradient position (0.0 to 1.0)
    let gradient_pos = input.uv.x;
    
    // Sample LUT at the gradient position
    let color = sample_lut(gradient_pos);
    
    // Apply display mode effects
    let final_color = apply_display_mode(color, input.uv);
    
    // Convert from 0-255 range to 0-1 range for output
    return vec4<f32>(final_color / 255.0, 1.0);
} 