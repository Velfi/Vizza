// Gradient compute shader for generating constant pheromone fields
// Provides background gradients that don't decay like regular pheromones

const PI: f32 = 3.14159265359;

@group(0) @binding(3)
var<storage, read_write> gradient_map: array<f32>;

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

@group(0) @binding(2)
var<uniform> sim_size: SimSizeUniform;

// Gradient types
const GRADIENT_DISABLED: u32 = 0u;
const GRADIENT_LINEAR: u32 = 1u;
const GRADIENT_RADIAL: u32 = 2u;
const GRADIENT_ELLIPSE: u32 = 3u;
const GRADIENT_SPIRAL: u32 = 4u;
const GRADIENT_CHECKERBOARD: u32 = 5u;

@compute @workgroup_size(256)
fn generate_gradient(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    let total_size = sim_size.width * sim_size.height;
    if (idx >= total_size) {
        return;
    }

    // Check if gradient is disabled
    if (sim_size.gradient_type == GRADIENT_DISABLED) {
        gradient_map[idx] = 0.0;
        return;
    }

    let x = idx % sim_size.width;
    let y = idx / sim_size.width;
    
    // Normalize coordinates to [0, 1]
    let norm_x = f32(x) / f32(sim_size.width);
    let norm_y = f32(y) / f32(sim_size.height);
    
    // Apply rotation to all coordinate systems based on angle
    let angle_rad = sim_size.gradient_angle * PI / 180.0;
    let cos_angle = cos(angle_rad);
    let sin_angle = sin(angle_rad);
    
    // Rotate coordinates around center point for all gradient types
    let centered_x = norm_x - 0.5;
    let centered_y = norm_y - 0.5;
    let rotated_x = centered_x * cos_angle - centered_y * sin_angle + 0.5;
    let rotated_y = centered_x * sin_angle + centered_y * cos_angle + 0.5;
    
    var gradient_value: f32 = 0.0;
    let size = max(sim_size.gradient_size, 0.001); // Prevent division by zero
    
    if (sim_size.gradient_type == GRADIENT_LINEAR) {
        // Linear gradient using rotated coordinates
        // Scale the gradient based on size (smaller size = tighter gradient)
        let scaled_x = (rotated_x - 0.5) / size + 0.5;
        gradient_value = clamp(scaled_x, 0.0, 1.0);
        
    } else if (sim_size.gradient_type == GRADIENT_RADIAL) {
        // Radial gradient from center, using rotated coordinates
        let dx = rotated_x - sim_size.gradient_center_x;
        let dy = rotated_y - sim_size.gradient_center_y;
        let distance = sqrt(dx * dx + dy * dy);
        
        // Use size as the falloff distance
        gradient_value = 1.0 - clamp(distance / size, 0.0, 1.0);
        
    } else if (sim_size.gradient_type == GRADIENT_ELLIPSE) {
        // Elliptical gradient from center, using rotated coordinates
        let dx = rotated_x - sim_size.gradient_center_x;
        let dy = rotated_y - sim_size.gradient_center_y;
        
        // Create elliptical distance (2:1 aspect ratio)
        let ellipse_x = dx / size;
        let ellipse_y = dy / (size * 0.5); // Make it twice as tall as wide
        let elliptical_distance = sqrt(ellipse_x * ellipse_x + ellipse_y * ellipse_y);
        
        gradient_value = 1.0 - clamp(elliptical_distance, 0.0, 1.0);
        
    } else if (sim_size.gradient_type == GRADIENT_SPIRAL) {
        // Spiral gradient, using rotated coordinates
        let dx = rotated_x - sim_size.gradient_center_x;
        let dy = rotated_y - sim_size.gradient_center_y;
        let distance = sqrt(dx * dx + dy * dy);
        let spiral_angle = atan2(dy, dx);
        
        // Create spiral pattern with size affecting both frequency and falloff
        let spiral_freq = 4.0 / size; // Smaller size = tighter spiral
        let spiral_value = sin(spiral_angle * spiral_freq + distance * 20.0 / size) * 0.5 + 0.5;
        
        // Size controls the radial falloff
        let radial_falloff = 1.0 - clamp(distance / size, 0.0, 1.0);
        gradient_value = spiral_value * radial_falloff;
        
    } else if (sim_size.gradient_type == GRADIENT_CHECKERBOARD) {
        // Checkerboard pattern, using rotated coordinates
        let scale = 8.0 / size; // Smaller size = smaller checkers
        let check_x = floor(rotated_x * scale);
        let check_y = floor(rotated_y * scale);
        let checker = (i32(check_x) + i32(check_y)) % 2;
        gradient_value = f32(checker);
    }
    
    // Apply strength multiplier
    gradient_value *= sim_size.gradient_strength;
    
    // Clamp to valid range
    gradient_map[idx] = clamp(gradient_value, 0.0, 1.0);
} 