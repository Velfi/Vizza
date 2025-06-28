// Fragment shader for space colonization branch rendering

struct FragmentInput {
    @location(0) thickness: f32,
    @location(1) generation: f32,
    @location(2) world_position: vec2<f32>,
    @location(3) line_start: vec2<f32>,
    @location(4) line_end: vec2<f32>,
}

struct SimParams {
    width: u32,
    height: u32,
    attraction_distance: f32,
    kill_distance: f32,
    segment_length: f32,
    max_attractors: u32,
    max_nodes: u32,
    open_venation: u32,
    enable_vein_thickening: u32,
    min_thickness: f32,
    max_thickness: f32,
    random_seed: u32,
    growth_speed: f32,
    delta_time: f32,
    frame_count: u32,
    enable_opacity_blending: u32,
    min_opacity: f32,
    max_opacity: f32,
    // Curve rendering parameters
    curve_tension: f32, // Default curve tension (0.0 = straight, 1.0 = tight)
    curve_segments: u32, // Number of segments to subdivide curves into
    _padding: f32,
}

@group(1) @binding(0) var<uniform> params: SimParams;
@group(1) @binding(1) var<storage, read> lut_data: array<u32>; // LUT data for coloring

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

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // For curve rendering, we need to calculate distance from point to curve
    // Since we're using cubic Bézier curves, we'll approximate by treating the curve
    // as a series of line segments and finding the closest point
    
    // Get the curve control points from the line start/end (which are the curve endpoints)
    let p0 = input.line_start; // Start point
    let p3 = input.line_end;   // End point
    
    // For now, we'll use a simplified approach: calculate distance to the line segment
    // In a more sophisticated implementation, we could evaluate the actual Bézier curve
    let line_vector = p3 - p0;
    let line_length = length(line_vector);
    
    // If line has no length (single point), render as a circle
    if (line_length < 0.001) {
        let distance_from_point = distance(input.world_position, p0);
        let line_width = input.thickness * 0.002; // Even smaller scaling for much thinner lines
        
        if (distance_from_point > line_width) {
            discard;
        }
    } else {
        // Calculate distance from point to line segment
        let t = clamp(dot(input.world_position - p0, line_vector) / (line_length * line_length), 0.0, 1.0);
        let closest_point = p0 + t * line_vector;
        let distance_from_line = distance(input.world_position, closest_point);
        let line_width = input.thickness * 0.002; // Even smaller scaling for much thinner lines
        
        if (distance_from_line > line_width) {
            discard;
        }
    }
    
    // Calculate base color from generation (depth in the tree)
    let max_generation = 20.0; // Reasonable max depth for color mapping
    let normalized_generation = min(input.generation / max_generation, 1.0);
    
    // Sample from LUT based on generation
    let base_color = get_lut_color(normalized_generation);
    
    // Apply thickness-based opacity if enabled
    var alpha = 1.0;
    if (params.enable_opacity_blending == 1u) {
        let thickness_ratio = (input.thickness - params.min_thickness) / 
                             (params.max_thickness - params.min_thickness);
        alpha = mix(params.min_opacity, params.max_opacity, thickness_ratio);
    }
    
    // Add some variation based on position for more organic look
    let noise_x = sin(input.world_position.x * 0.01) * 0.1;
    let noise_y = cos(input.world_position.y * 0.01) * 0.1;
    var final_color = base_color + vec3<f32>(noise_x, noise_y, -noise_x * 0.5);
    final_color = clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0));
    
    // Enhance roots to be more prominent
    if (input.generation < 2.0) {
        final_color = mix(final_color, vec3<f32>(0.8, 0.6, 0.3), 0.3); // Warm brown tint
        alpha = min(alpha * 1.2, 1.0);
    }
    
    return vec4<f32>(final_color, alpha);
} 