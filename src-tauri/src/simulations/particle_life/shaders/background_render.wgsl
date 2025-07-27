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
    @location(0) world_pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen quad
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    
    let pos = positions[vertex_index];
    
    // Transform to world space
    let world_pos = (camera.transform_matrix * vec4<f32>(pos, 0.0, 1.0)).xy;
    
    return VertexOutput(
        vec4<f32>(pos, 0.0, 1.0),
        world_pos
    );
}

@fragment
fn fs_main(@location(0) world_pos: vec2<f32>) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    
    if (background_params.background_type == 0u) {
        // Black background
        color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if (background_params.background_type == 1u) {
        // White background
        color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if (background_params.background_type == 2u) {
        // Gradient background
        var gradient_value = 0.0;
        
        if (background_params.gradient_enabled == 1u) {
            let center = vec2<f32>(background_params.gradient_center_x, background_params.gradient_center_y);
            let size = background_params.gradient_size;
            let angle = background_params.gradient_angle;
            
            // Rotate coordinates
            let cos_angle = cos(angle);
            let sin_angle = sin(angle);
            let rotated_x = (world_pos.x - center.x) * cos_angle + (world_pos.y - center.y) * sin_angle;
            let rotated_y = -(world_pos.x - center.x) * sin_angle + (world_pos.y - center.y) * cos_angle;
            
            if (background_params.gradient_type == 1u) {
                // Linear gradient
                gradient_value = (rotated_x / size + 0.5) * background_params.gradient_strength;
            } else if (background_params.gradient_type == 2u) {
                // Radial gradient
                let distance = length(world_pos - center);
                gradient_value = (distance / size) * background_params.gradient_strength;
            } else if (background_params.gradient_type == 3u) {
                // Ellipse gradient
                let distance = sqrt((rotated_x * rotated_x) / (size * size) + (rotated_y * rotated_y) / (size * size * 0.25));
                gradient_value = distance * background_params.gradient_strength;
            } else if (background_params.gradient_type == 4u) {
                // Spiral gradient
                let distance = length(world_pos - center);
                let angle_from_center = atan2(world_pos.y - center.y, world_pos.x - center.x);
                let spiral_value = (angle_from_center + 3.14159265359) / (2.0 * 3.14159265359) + distance / background_params.gradient_size;
                gradient_value = fract(spiral_value) * background_params.gradient_strength;
            } else if (background_params.gradient_type == 5u) {
                // Checkerboard gradient
                let scale = size * 0.1;
                let checker_x = floor(rotated_x / scale);
                let checker_y = floor(rotated_y / scale);
                gradient_value = f32((u32(checker_x) + u32(checker_y)) % 2u) * background_params.gradient_strength;
            }
        }
        
        color = vec4<f32>(gradient_value, gradient_value, gradient_value, 1.0);
    }
    
    return color;
} 