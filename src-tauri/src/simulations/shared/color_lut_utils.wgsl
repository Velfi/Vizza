fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

fn get_lut_color(index: u32) -> vec3<f32> {
    // LUT data format: [r0..r255, g0..g255, b0..b255]
    let r_srgb = f32(lut_data[index]) / 255.0;
    let g_srgb = f32(lut_data[index + 256u]) / 255.0;
    let b_srgb = f32(lut_data[index + 512u]) / 255.0;
    return vec3<f32>(
        srgb_to_linear(r_srgb),
        srgb_to_linear(g_srgb),
        srgb_to_linear(b_srgb)
    );
}

fn get_lut_color_from_intensity(intensity: f32) -> vec3<f32> {
    let lut_index = clamp(intensity * 255.0, 0.0, 255.0);
    return get_lut_color(u32(lut_index));
}