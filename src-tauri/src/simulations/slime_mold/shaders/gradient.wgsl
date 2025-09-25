// Mask compute shader for generating mask patterns
// Provides spatial masks that affect simulation parameters

const PI: f32 = 3.14159265359;

@group(0) @binding(3)
var<storage, read_write> mask_map: array<f32>;

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
    mask_pattern: u32,
    mask_target: u32,
    mask_strength: f32,
    mask_curve: f32,
    mask_mirror_horizontal: u32,
    mask_mirror_vertical: u32,
    mask_invert_tone: u32,
    _pad1: u32,
};

@group(0) @binding(2)
var<uniform> sim_size: SimSizeUniform;

// Mask pattern types
const MASK_DISABLED: u32 = 0u;
const MASK_CHECKERBOARD: u32 = 1u;
const MASK_DIAGONAL_GRADIENT: u32 = 2u;
const MASK_RADIAL_GRADIENT: u32 = 3u;
const MASK_VERTICAL_STRIPES: u32 = 4u;
const MASK_HORIZONTAL_STRIPES: u32 = 5u;
const MASK_WAVE_FUNCTION: u32 = 6u;
const MASK_COSINE_GRID: u32 = 7u;
const MASK_IMAGE: u32 = 8u;

@compute @workgroup_size(256)
fn generate_mask(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    let total_size = sim_size.width * sim_size.height;
    if (idx >= total_size) {
        return;
    }

    // Check if mask is disabled or image-driven
    if (sim_size.mask_pattern == MASK_DISABLED || sim_size.mask_pattern == MASK_IMAGE) {
        mask_map[idx] = 0.0;
        return;
    }

    let x = idx % sim_size.width;
    let y = idx / sim_size.width;
    
    // Normalize coordinates to [0, 1]
    let original_nx = f32(x) / f32(sim_size.width);
    let original_ny = f32(y) / f32(sim_size.height);
    
    var result = 0.0;
    
    switch (sim_size.mask_pattern) {
        case 0u: { // Disabled
            result = 1.0;
        }
        case 1u: { // Checkerboard
            let block_size = 200u;
            let bx = u32(x) / block_size;
            let by = u32(y) / block_size;
            let is_checker = ((bx + by) % 2u) == 0u;
            result = select(0.0, 1.0, is_checker);
        }
        case 2u: { // Diagonal gradient
            let nx = original_nx;
            let ny = original_ny;
            result = (nx + ny) / 2.0;
        }
        case 3u: { // Radial gradient
            let nx = original_nx;
            let ny = original_ny;
            let center_x = 0.5;
            let center_y = 0.5;
            let dx = nx - center_x;
            let dy = ny - center_y;
            let distance = sqrt(dx * dx + dy * dy);
            result = 1.0 - distance * 2.0; // Scale to [0,1]
        }
        case 4u: { // Vertical stripes
            let nx = original_nx;
            result = sin(nx * 3.14159 * 4.0) * 0.5 + 0.5;
        }
        case 5u: { // Horizontal stripes
            let ny = original_ny;
            result = sin(ny * 3.14159 * 4.0) * 0.5 + 0.5;
        }
        case 6u: { // Wave function
            let nx = original_nx;
            let ny = original_ny;
            let wave = sin(nx * 3.14159 * 2.0) * cos(ny * 3.14159 * 2.0);
            result = wave * 0.5 + 0.5;
        }
        case 7u: { // Cosine grid
            let nx = original_nx;
            let ny = original_ny;
            let grid_x = cos(nx * 3.14159 * 4.0);
            let grid_y = cos(ny * 3.14159 * 4.0);
            result = (grid_x + grid_y) * 0.5;
        }
        default: {
            result = 1.0;
        }
    }

    // Clamp to valid range
    mask_map[idx] = clamp(result, 0.0, 1.0);
} 