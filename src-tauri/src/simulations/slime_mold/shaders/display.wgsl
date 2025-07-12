// Display shader for converting trail map to displayable texture
// Uses LUT for color mapping

struct SimSizeUniform {
    width: u32,
    height: u32,
    decay_rate: f32,
    agent_jitter: f32,
    agent_speed_min: f32,
    agent_speed_max: f32,
    agent_turn_rate: f32,
    agent_sensor_angle: f32,
    agent_sensor_distance: f32,
    diffusion_rate: f32,
    pheromone_deposition_rate: f32,
    gradient_enabled: u32,
    gradient_type: u32,
    gradient_strength: f32,
    gradient_center_x: f32,
    gradient_center_y: f32,
    gradient_size: f32,
    gradient_angle: f32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0)
var<storage, read> trail_map: array<f32>;

@group(0) @binding(1)
var display_tex: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(2)
var<uniform> sim_size: SimSizeUniform;

@group(0) @binding(3)
var<storage, read> lut_data: array<u32>;

@group(0) @binding(4)
var<storage, read> gradient_map: array<f32>;

// Bilinear interpolation for trail map sampling
fn sample_trail_map_smooth(pos: vec2<f32>) -> f32 {
    let width = i32(sim_size.width);
    let height = i32(sim_size.height);
    
    // Clamp position to valid range
    let clamped_pos = clamp(pos, vec2<f32>(0.0), vec2<f32>(f32(width - 1), f32(height - 1)));
    
    let x0 = i32(floor(clamped_pos.x));
    let y0 = i32(floor(clamped_pos.y));
    let x1 = min(x0 + 1, width - 1);
    let y1 = min(y0 + 1, height - 1);
    
    let dx = clamped_pos.x - f32(x0);
    let dy = clamped_pos.y - f32(y0);
    
    let v00 = trail_map[y0 * width + x0];
    let v10 = trail_map[y0 * width + x1];
    let v01 = trail_map[y1 * width + x0];
    let v11 = trail_map[y1 * width + x1];
    
    let v0 = mix(v00, v10, dx);
    let v1 = mix(v01, v11, dx);
    return mix(v0, v1, dy);
}

// Bilinear interpolation for gradient map sampling
fn sample_gradient_map_smooth(pos: vec2<f32>) -> f32 {
    let width = i32(sim_size.width);
    let height = i32(sim_size.height);
    
    // Clamp position to valid range
    let clamped_pos = clamp(pos, vec2<f32>(0.0), vec2<f32>(f32(width - 1), f32(height - 1)));
    
    let x0 = i32(floor(clamped_pos.x));
    let y0 = i32(floor(clamped_pos.y));
    let x1 = min(x0 + 1, width - 1);
    let y1 = min(y0 + 1, height - 1);
    
    let dx = clamped_pos.x - f32(x0);
    let dy = clamped_pos.y - f32(y0);
    
    let v00 = gradient_map[y0 * width + x0];
    let v10 = gradient_map[y0 * width + x1];
    let v01 = gradient_map[y1 * width + x0];
    let v11 = gradient_map[y1 * width + x1];
    
    let v0 = mix(v00, v10, dx);
    let v1 = mix(v01, v11, dx);
    return mix(v0, v1, dy);
}

// Convert from sRGB (gamma-corrected) to linear RGB
fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

// Get color from LUT
fn get_lut_color(intensity: f32) -> vec3<f32> {
    let idx = clamp(i32(intensity * 255.0), 0, 255);
    let r_srgb = f32(lut_data[idx]) / 255.0;
    let g_srgb = f32(lut_data[256 + idx]) / 255.0;
    let b_srgb = f32(lut_data[512 + idx]) / 255.0;
    
    return vec3<f32>(
        srgb_to_linear(r_srgb),
        srgb_to_linear(g_srgb),
        srgb_to_linear(b_srgb)
    );
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_width = u32(textureDimensions(display_tex).x);
    let tex_height = u32(textureDimensions(display_tex).y);
    if (id.x >= tex_width || id.y >= tex_height) {
        return;
    }

    // Map texture pixel to simulation coordinates (with sub-pixel precision)
    var sim_x = f32(id.x) * f32(sim_size.width) / f32(tex_width);
    var sim_y = f32(id.y) * f32(sim_size.height) / f32(tex_height);

    var color = vec3<f32>(0.0);
    if (sim_x >= 0.0 && sim_x < f32(sim_size.width) && sim_y >= 0.0 && sim_y < f32(sim_size.height)) {
        // Use bilinear interpolation for smooth sampling
        let trail = sample_trail_map_smooth(vec2<f32>(sim_x, sim_y));
        
        // Only add gradient if it's enabled (gradient_type != 0 means enabled)
        var intensity = trail;
        if (sim_size.gradient_type != 0u) {
            let grad = sample_gradient_map_smooth(vec2<f32>(sim_x, sim_y));
            intensity = clamp(trail + grad, 0.0, 1.0);
        }
        
        color = get_lut_color(intensity);
    }
    textureStore(display_tex, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(color, 1.0));
} 