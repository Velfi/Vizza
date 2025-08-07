// Antialiased shape drawing compute shader for Flow Field
// Draws smooth geometric shapes directly onto the trail map

struct ShapeParams {
    center_x: f32,
    center_y: f32,
    size: f32,
    shape_type: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond, 5=Line
    color: vec4<f32>,
    intensity: f32,
    antialiasing_width: f32, // Width of antialiasing edge in pixels
    rotation: f32, // Rotation angle in radians
    aspect_ratio: f32, // For ellipses and rectangles
    trail_map_width: u32,
    trail_map_height: u32,
    _padding: u32,
}

@group(0) @binding(0) var trail_map: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var<uniform> shape_params: ShapeParams;

// Signed distance functions for various shapes
fn sdf_circle(p: vec2<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

fn sdf_box(p: vec2<f32>, size: vec2<f32>) -> f32 {
    let d = abs(p) - size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn sdf_triangle(p: vec2<f32>, size: f32) -> f32 {
    let k = sqrt(3.0);
    let px = abs(p.x) - size;
    let py = p.y + size / k;
    
    if (px + k * py > 0.0) {
        let temp = vec2<f32>(px - k * py, -k * px - py) / 2.0;
        return -length(temp);
    }
    
    let px_clamped = max(px, 0.0);
    return -sqrt(px_clamped * px_clamped + py * py);
}

fn sdf_star(p: vec2<f32>, r: f32, n: f32, m: f32) -> f32 {
    // Star with n points, inner radius m*r, outer radius r
    let an = 3.141593 / n;
    let en = 3.141593 / m;
    let acs = vec2<f32>(cos(an), sin(an));
    let ecs = vec2<f32>(cos(en), sin(en));
    
    let bn = (atan2(p.x, p.y) % (2.0 * an)) - an;
    let p_rot = length(p) * vec2<f32>(cos(bn), abs(sin(bn)));
    
    let p_final = p_rot - r * acs;
    let p_clamped = vec2<f32>(clamp(p_final.x, -ecs.y * r, ecs.y * r), p_final.y);
    
    return length(p_final - p_clamped) * sign(p_final.y - ecs.x * r);
}

fn sdf_diamond(p: vec2<f32>, size: f32) -> f32 {
    return (abs(p.x) + abs(p.y)) - size;
}

// Rotation matrix
fn rotate2d(angle: f32) -> mat2x2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat2x2<f32>(
        vec2<f32>(c, -s),
        vec2<f32>(s, c)
    );
}

// Main compute shader
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coords = vec2<i32>(i32(global_id.x), i32(global_id.y));
    
    // Check bounds
    if (global_id.x >= shape_params.trail_map_width || global_id.y >= shape_params.trail_map_height) {
        return;
    }
    
    // Convert pixel coordinates to world space (-1 to 1)
    let uv = vec2<f32>(
        (f32(global_id.x) / f32(shape_params.trail_map_width)) * 2.0 - 1.0,
        (f32(global_id.y) / f32(shape_params.trail_map_height)) * 2.0 - 1.0
    );
    
    // Transform to shape local space
    let center = vec2<f32>(shape_params.center_x, shape_params.center_y);
    let p = uv - center;
    
    // Apply rotation
    let p_rotated = rotate2d(shape_params.rotation) * p;
    
    // Apply aspect ratio scaling
    let p_scaled = vec2<f32>(p_rotated.x, p_rotated.y / shape_params.aspect_ratio);
    
    // Calculate signed distance based on shape type
    var distance: f32;
    
    switch (shape_params.shape_type) {
        case 0u: { // Circle
            distance = sdf_circle(p_scaled, shape_params.size);
        }
        case 1u: { // Square/Rectangle
            let size = vec2<f32>(shape_params.size, shape_params.size);
            distance = sdf_box(p_scaled, size);
        }
        case 2u: { // Triangle
            distance = sdf_triangle(p_scaled, shape_params.size);
        }
        case 3u: { // Star
            distance = sdf_star(p_scaled, shape_params.size, 5.0, 2.5);
        }
        case 4u: { // Diamond
            distance = sdf_diamond(p_scaled, shape_params.size);
        }
        default: { // Default to circle
            distance = sdf_circle(p_scaled, shape_params.size);
        }
    }
    
    // Calculate antialiased alpha based on distance
    let edge_width = shape_params.antialiasing_width * 0.01; // Convert to world space
    let alpha = 1.0 - smoothstep(-edge_width, edge_width, distance);
    
    // Only draw if alpha is significant
    if (alpha > 0.001) {
        // Read existing trail color
        let existing_color = textureLoad(trail_map, pixel_coords);
        
        // Blend new shape with existing trail
        let shape_color = vec4<f32>(
            shape_params.color.rgb,
            alpha * shape_params.intensity
        );
        
        // Alpha blending: new_color = src * src_alpha + dst * (1 - src_alpha)
        let final_color = vec4<f32>(
            shape_color.rgb * shape_color.a + existing_color.rgb * (1.0 - shape_color.a),
            min(shape_color.a + existing_color.a, 1.0)
        );
        
        textureStore(trail_map, pixel_coords, final_color);
    }
}
