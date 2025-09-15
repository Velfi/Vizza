@group(0) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<storage, read> lut_data: array<u32>;
@group(0) @binding(3) var prev_texture: texture_2d<f32>;
@group(0) @binding(4) var tex_sampler: sampler;
@group(0) @binding(5) var image_texture: texture_2d<f32>;

struct Params {
    time: f32,
    width: f32,
    height: f32,
    generator_type: f32, // 0 = Linear, 1 = Radial
    base_freq: f32,
    moire_amount: f32,
    moire_rotation: f32,
    moire_scale: f32,
    moire_interference: f32,
    moire_rotation3: f32,
    moire_scale3: f32,
    moire_weight3: f32,
    radial_swirl_strength: f32,
    radial_starburst_count: f32,
    radial_center_brightness: f32,
    color_scheme_reversed: f32,
    advect_strength: f32,
    advect_speed: f32,
    curl: f32,
    decay: f32,
    // Flags
    image_loaded: f32,
    image_mode_enabled: f32,
    image_interference_mode: f32, // 0=Replace, 1=Add, 2=Multiply, 3=Overlay, 4=Mask, 5=Modulate
}


fn rotate2d(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        v.x * c - v.y * s,
        v.x * s + v.y * c
    );
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

fn compute_linear_moire(pos: vec2<f32>) -> f32 {
    let t = params.time * 0.5;
    
    // First grid pattern with time variation
    let grid1 = sin(pos.x * params.base_freq + t) * sin(pos.y * params.base_freq + t * 0.7);
    
    // Second grid pattern - rotated and scaled with time variation
    let rotated_pos = rotate2d(pos, params.moire_rotation + t * 0.1);
    let scaled_pos = rotated_pos * (params.moire_scale + sin(t) * 0.1);
    let grid2 = sin(scaled_pos.x * params.base_freq + t * 1.3) * sin(scaled_pos.y * params.base_freq + t * 0.9);
    
    // Third grid for more complex patterns with time variation
    let rotated_pos2 = rotate2d(pos, params.moire_rotation3 + t * 0.2);
    let grid3 = sin(rotated_pos2.x * params.base_freq * params.moire_scale3 + t * 0.5) * sin(rotated_pos2.y * params.base_freq * params.moire_scale3 + t * 1.1);
    
    // Combine grids with interference
    let interference = mix(grid1 * grid2, (grid1 + grid2) * 0.5, params.moire_interference);
    let complex_pattern = mix(interference, interference * grid3, params.moire_weight3);
    
    return complex_pattern * params.moire_amount;
}

fn compute_radial_moire(pos: vec2<f32>) -> f32 {
    let t = params.time * 0.5;
    
    // Calculate distance from center for radial patterns
    let dist = length(pos);
    let angle = atan2(pos.y, pos.x);
    
    // Create bright center glow
    let center_glow = exp(-dist * 8.0) * params.radial_center_brightness;
    
    // Starburst pattern - straight radiating lines
    let starburst_angle = angle * params.radial_starburst_count;
    let starburst = sin(starburst_angle + t * 0.3) * 0.8;
    
    // Swirling radial pattern - curved lines with vortex effect
    let swirl_angle = angle + params.radial_swirl_strength * dist * 3.0 + t * 0.2;
    let swirl_radial = sin(dist * params.base_freq * 2.0 + swirl_angle * 2.0 + t * 0.5);
    
    // Concentric circles for layered effect
    let concentric = sin(dist * params.base_freq + t * 0.8) * 0.6;
    
    // Secondary interference pattern
    let interference_angle = angle + params.moire_rotation + t * 0.1;
    let interference_dist = dist * (params.moire_scale + sin(t) * 0.1);
    let interference = sin(interference_dist * params.base_freq * 1.3 + interference_angle * 3.0 + t * 1.2);
    
    // Combine patterns with different weights
    let primary_pattern = mix(starburst, swirl_radial, params.radial_swirl_strength);
    let secondary_pattern = mix(concentric, interference, params.moire_interference);
    
    // Final combination with center glow
    let final_pattern = primary_pattern * 0.7 + secondary_pattern * 0.3;
    let result = final_pattern * params.moire_amount + center_glow;
    
    return result;
}

fn compute_moire(pos: vec2<f32>) -> f32 {
    // Switch between linear and radial generators based on generator_type
    if (params.generator_type < 0.5) {
        return compute_linear_moire(pos);
    } else {
        return compute_radial_moire(pos);
    }
}

fn sample_image_intensity(uv: vec2<f32>) -> f32 {
    // Sample external image texture assumed preprocessed to grayscale in R8
    let color = textureSampleLevel(image_texture, tex_sampler, uv, 0.0);
    // Use red channel as grayscale
    return clamp(color.r, 0.0, 1.0);
}

fn compute_velocity(pos: vec2<f32>, uv: vec2<f32>) -> vec2<f32> {
    let t = params.time * params.advect_speed;
    
    // Create multiple overlapping flow patterns for more complex dynamics
    var vel = vec2<f32>(0.0, 0.0);
    
    // Primary flow pattern - large scale circulation
    let primary_scale = 2.0;
    vel += vec2<f32>(
        sin(pos.y * primary_scale + t * 0.8) * 0.6,
        cos(pos.x * primary_scale + t * 1.2) * 0.6
    );
    
    // Secondary flow pattern - medium scale waves
    let secondary_scale = 4.0;
    vel += vec2<f32>(
        sin(pos.x * secondary_scale + t * 1.5) * cos(pos.y * secondary_scale * 0.7 + t * 0.9) * 0.4,
        cos(pos.y * secondary_scale + t * 1.1) * sin(pos.x * secondary_scale * 0.8 + t * 1.3) * 0.4
    );
    
    // Tertiary flow pattern - fine scale turbulence
    let tertiary_scale = 8.0;
    vel += vec2<f32>(
        sin(pos.x * tertiary_scale + t * 2.1) * 0.2,
        cos(pos.y * tertiary_scale + t * 1.8) * 0.2
    );
    
    // Add time-varying amplitude modulation for pulsing flow
    let pulse = 1.0 + sin(t * 0.3) * 0.3;
    vel *= pulse;
    
    // Enhanced curl effect with multiple vortices
    if (params.curl > 0.0) {
        // Main central vortex
        let center = vec2<f32>(0.0, 0.0);
        let offset = pos - center;
        let dist = length(offset);
        let vortex_strength = exp(-dist * 2.0); // Decay with distance
        vel += vec2<f32>(-offset.y, offset.x) * params.curl * vortex_strength * 0.8;
        
        // Secondary vortices at different positions
        let vortex1_pos = vec2<f32>(0.5, 0.3);
        let offset1 = pos - vortex1_pos;
        let dist1 = length(offset1);
        let vortex1_strength = exp(-dist1 * 3.0);
        vel += vec2<f32>(-offset1.y, offset1.x) * params.curl * vortex1_strength * 0.4;
        
        let vortex2_pos = vec2<f32>(-0.4, -0.2);
        let offset2 = pos - vortex2_pos;
        let dist2 = length(offset2);
        let vortex2_strength = exp(-dist2 * 2.5);
        vel += vec2<f32>(-offset2.y, offset2.x) * params.curl * vortex2_strength * 0.3;
    }
    
    // Add some noise-like variation for more organic flow
    let noise_scale = 12.0;
    vel += vec2<f32>(
        sin(pos.x * noise_scale + t * 3.0) * cos(pos.y * noise_scale * 1.3 + t * 2.7) * 0.1,
        cos(pos.y * noise_scale + t * 2.5) * sin(pos.x * noise_scale * 0.9 + t * 3.2) * 0.1
    );
    
    return vel * params.advect_strength * 0.15; // Increased base strength
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = vec2<f32>(params.width, params.height);
    let coords = vec2<f32>(f32(id.x), f32(id.y));
    
    if (coords.x >= dims.x || coords.y >= dims.y) {
        return;
    }
    
    let x = (coords.x / dims.x) * 2.0 - 1.0;
    let y = (coords.y / dims.y) * 2.0 - 1.0;
    
    // Compute moiré pattern
    let moire = compute_moire(vec2<f32>(x, y));
    
    // Add time-varying component to moiré
    let animated_moire = moire * (1.0 + sin(params.time * 3.0) * 0.5);
    
    // Generate intensity directly from moiré patterns
    var base_intensity = (animated_moire + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
    
    // Add some spatial variation and time-based animation
    let spatial_variation = sin(x * 2.0) * cos(y * 2.0) * 0.2;
    let time_variation = sin(params.time * 0.5) * 0.1;
    
    // Enhanced advection with multiple sampling points for better flow
    let current_uv = coords / dims;
    
    // If image mode is enabled and an image is loaded, apply interference
    if (params.image_mode_enabled > 0.5 && params.image_loaded > 0.5) {
        let image_intensity = sample_image_intensity(current_uv);
        
        // Apply different interference modes
        if (params.image_interference_mode < 0.5) {
            // Replace mode - current behavior
            base_intensity = image_intensity;
        } else if (params.image_interference_mode < 1.5) {
            // Add mode - add image to moiré pattern
            base_intensity = clamp(base_intensity + image_intensity, 0.0, 1.0);
        } else if (params.image_interference_mode < 2.5) {
            // Multiply mode - multiply image with moiré pattern
            base_intensity = base_intensity * image_intensity;
        } else if (params.image_interference_mode < 3.5) {
            // Overlay mode - complex blending that preserves highlights and shadows
            if (base_intensity < 0.5) {
                base_intensity = 2.0 * base_intensity * image_intensity;
            } else {
                base_intensity = 1.0 - 2.0 * (1.0 - base_intensity) * (1.0 - image_intensity);
            }
        } else if (params.image_interference_mode < 4.5) {
            // Mask mode - use image as mask for moiré pattern
            base_intensity = base_intensity * image_intensity;
        } else {
            // Modulate mode - use image to modulate moiré intensity
            base_intensity = base_intensity * (0.5 + image_intensity * 0.5);
        }
    }

    let final_intensity = clamp(base_intensity + spatial_variation + time_variation, 0.0, 1.0);
    
    // Apply color scheme reversal if enabled
    var lut_intensity = final_intensity;
    if (params.color_scheme_reversed > 0.5) {
        lut_intensity = 1.0 - final_intensity;
    }
    
    // Get color from LUT
    var nn_color = get_lut_color(lut_intensity);
    let vel = compute_velocity(vec2<f32>(x, y), current_uv);
    
    // Sample multiple points along the velocity vector for smoother advection
    let advected_uv1 = current_uv - vel * 0.5;
    let advected_uv2 = current_uv - vel;
    let advected_uv3 = current_uv - vel * 1.5;
    
    let prev_color1 = textureSampleLevel(prev_texture, tex_sampler, advected_uv1, 0.0).rgb;
    let prev_color2 = textureSampleLevel(prev_texture, tex_sampler, advected_uv2, 0.0).rgb;
    let prev_color3 = textureSampleLevel(prev_texture, tex_sampler, advected_uv3, 0.0).rgb;
    
    // Blend the multiple samples for smoother advection
    let prev_color = (prev_color1 + prev_color2 * 2.0 + prev_color3) * 0.25;
    
    // Enhanced blending with better flow visibility
    let advection_mix = params.advect_strength * 1.2; // Increased mixing strength
    let new_pattern_weight = 1.0 - advection_mix;
    let advected_weight = advection_mix * params.decay;
    
    // Add some temporal variation to the blending for more dynamic flow
    let dynamic_mix = clamp(advection_mix + time_variation, 0.0, 1.0);
    
    let final_color = nn_color * new_pattern_weight + prev_color * advected_weight;
    
    textureStore(output_texture, vec2<i32>(id.xy), vec4<f32>(clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0));
}
