struct BackgroundParams {
    background_type: u32, // 0 = black, 1 = white, 2 = gradient
    gradient_enabled: u32,
    gradient_type: u32,
    gradient_strength: f32,
    gradient_center_x: f32,
    gradient_center_y: f32,
    gradient_size: f32,
    gradient_angle: f32,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0)
var<uniform> background_params: BackgroundParams;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen quad
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    
    var out: VertexOutput;
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (background_params.background_type == 0u) {
        // Black background
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if (background_params.background_type == 1u) {
        // White background
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if (background_params.background_type == 2u && background_params.gradient_enabled != 0u) {
        // Gradient background
        let world_pos = vec2<f32>(
            in.uv.x * 2.0 - 1.0,
            in.uv.y * 2.0 - 1.0
        );
        
        var gradient_value = 0.0;
        
        if (background_params.gradient_type == 1u) {
            // Linear gradient
            let direction = vec2<f32>(
                cos(background_params.gradient_angle),
                sin(background_params.gradient_angle)
            );
            let center = vec2<f32>(background_params.gradient_center_x, background_params.gradient_center_y);
            let offset = world_pos - center;
            gradient_value = dot(offset, direction) * background_params.gradient_strength;
        } else if (background_params.gradient_type == 2u) {
            // Radial gradient
            let center = vec2<f32>(background_params.gradient_center_x, background_params.gradient_center_y);
            let distance = length(world_pos - center);
            gradient_value = (1.0 - distance / background_params.gradient_size) * background_params.gradient_strength;
        } else if (background_params.gradient_type == 3u) {
            // Ellipse gradient
            let center = vec2<f32>(background_params.gradient_center_x, background_params.gradient_center_y);
            let offset = world_pos - center;
            let angle = background_params.gradient_angle;
            let cos_a = cos(angle);
            let sin_a = sin(angle);
            let rotated_x = offset.x * cos_a + offset.y * sin_a;
            let rotated_y = -offset.x * sin_a + offset.y * cos_a;
            let normalized_distance = sqrt((rotated_x * rotated_x) / (background_params.gradient_size * background_params.gradient_size) + 
                                         (rotated_y * rotated_y) / (background_params.gradient_size * background_params.gradient_size * 0.5));
            gradient_value = (1.0 - normalized_distance) * background_params.gradient_strength;
        } else if (background_params.gradient_type == 4u) {
            // Spiral gradient
            let center = vec2<f32>(background_params.gradient_center_x, background_params.gradient_center_y);
            let offset = world_pos - center;
            let angle = atan2(offset.y, offset.x);
            let distance = length(offset);
            let spiral_value = (angle + 3.14159265359) / (2.0 * 3.14159265359) + distance / background_params.gradient_size;
            gradient_value = (fract(spiral_value) * 2.0 - 1.0) * background_params.gradient_strength;
        } else if (background_params.gradient_type == 5u) {
            // Checkerboard gradient
            let center = vec2<f32>(background_params.gradient_center_x, background_params.gradient_center_y);
            let offset = world_pos - center;
            let angle = background_params.gradient_angle;
            let cos_a = cos(angle);
            let sin_a = sin(angle);
            let rotated_x = offset.x * cos_a + offset.y * sin_a;
            let rotated_y = -offset.x * sin_a + offset.y * cos_a;
            let scale = background_params.gradient_size;
            let checker_x = floor(rotated_x / scale);
            let checker_y = floor(rotated_y / scale);
            gradient_value = f32((u32(checker_x) + u32(checker_y)) % 2u) * background_params.gradient_strength;
        }
        
        // Clamp and apply gradient
        gradient_value = clamp(gradient_value, 0.0, 1.0);
        return vec4<f32>(gradient_value, gradient_value, gradient_value, 1.0);
    }
    
    // Default to black
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
} 