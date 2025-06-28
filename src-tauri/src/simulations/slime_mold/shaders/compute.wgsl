// Compute shader for Physarum simulation
// Each agent is represented by a vec4<f32>: x, y, angle, speed

const TAU: f32 = 6.28318530718; // 2π

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
    random_seed: u32,
    _pad1: u32,
};

struct CursorParams {
    is_active: u32, // 0=inactive, 1=attract, 2=repel
    x: f32,
    y: f32,
    strength: f32,
    size: f32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0)
var<storage, read_write> agents: array<vec4<f32>>;

@group(0) @binding(1)
var<storage, read_write> trail_map: array<f32>;

@group(0) @binding(2)
var<uniform> sim_size: SimSizeUniform;

@group(0) @binding(3)
var<storage, read> gradient_map: array<f32>;

@group(0) @binding(4)
var<uniform> cursor: CursorParams;

// Helper function for bilinear interpolation
fn sample_trail_map(pos: vec2<f32>) -> f32 {
    let width = i32(sim_size.width);
    let height = i32(sim_size.height);

    let x0 = ((i32(floor(pos.x)) % width) + width) % width;
    let y0 = ((i32(floor(pos.y)) % height) + height) % height;
    let x1 = (x0 + 1) % width;
    let y1 = (y0 + 1) % height;

    let dx = pos.x - f32(i32(floor(pos.x)));
    let dy = pos.y - f32(i32(floor(pos.y)));

    let v00 = trail_map[y0 * width + x0];
    let v10 = trail_map[y0 * width + x1];
    let v01 = trail_map[y1 * width + x0];
    let v11 = trail_map[y1 * width + x1];

    let v0 = mix(v00, v10, dx);
    let v1 = mix(v01, v11, dx);
    return mix(v0, v1, dy);
}

// Fast sampling using nearest neighbor (much faster)
fn sample_trail_map_fast(pos: vec2<f32>) -> f32 {
    let width = i32(sim_size.width);
    let height = i32(sim_size.height);
    
    let x = ((i32(round(pos.x)) % width) + width) % width;
    let y = ((i32(round(pos.y)) % height) + height) % height;
    
    return trail_map[y * width + x];
}

// Helper function to sample gradient map
fn sample_gradient_map(pos: vec2<f32>) -> f32 {
    let width = i32(sim_size.width);
    let height = i32(sim_size.height);

    let x0 = ((i32(floor(pos.x)) % width) + width) % width;
    let y0 = ((i32(floor(pos.y)) % height) + height) % height;
    let x1 = (x0 + 1) % width;
    let y1 = (y0 + 1) % height;

    let dx = pos.x - f32(i32(floor(pos.x)));
    let dy = pos.y - f32(i32(floor(pos.y)));

    let v00 = gradient_map[y0 * width + x0];
    let v10 = gradient_map[y0 * width + x1];
    let v01 = gradient_map[y1 * width + x0];
    let v11 = gradient_map[y1 * width + x1];

    let v0 = mix(v00, v10, dx);
    let v1 = mix(v01, v11, dx);
    return mix(v0, v1, dy);
}

// Combined function to sample both trail and gradient
fn sample_combined_map(pos: vec2<f32>) -> f32 {
    let trail_value = sample_trail_map(pos);
    var gradient_value: f32;
    if (sim_size.gradient_enabled == 1u) {
        gradient_value = sample_gradient_map(pos);
    } else {
        gradient_value = 0.0;
    }
    return trail_value + gradient_value;
}

// Fast combined sampling for performance-critical paths
fn sample_combined_map_fast(pos: vec2<f32>) -> f32 {
    let trail_value = sample_trail_map_fast(pos);
    var gradient_value: f32;
    if (sim_size.gradient_enabled == 1u) {
        let width = i32(sim_size.width);
        let height = i32(sim_size.height);
        let x = ((i32(round(pos.x)) % width) + width) % width;
        let y = ((i32(round(pos.y)) % height) + height) % height;
        gradient_value = gradient_map[y * width + x];
    } else {
        gradient_value = 0.0;
    }
    return trail_value + gradient_value;
}

// Parameters for the simulation (now mostly from uniform)
const TIME_STEP: f32 = 0.016; // Affects how far agents move per frame based on their speed

// Constants for spatial partitioning
const WORKGROUP_SIZE_X: u32 = 16u;
const WORKGROUP_SIZE_Y: u32 = 16u;
const CELL_SIZE: f32 = 20.0;  // Size of each cell in the spatial grid

// Shared memory for storing local agent positions
var<workgroup> local_agents: array<vec4<f32>, 256>;

@compute @workgroup_size(16, 16, 1)
fn update_agents(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // Calculate linear agent index from 2D global invocation
    let agents_per_row = 65535u * 16u; // Max workgroups per row * threads per workgroup row
    let agent_index = id.x + id.y * agents_per_row;
    
    // For consistent random seeding, create a sequential index (preserves old preset behavior)
    // This ensures random patterns remain the same regardless of dispatch method
    let workgroup_linear_id = (id.x / 16u) + (id.y / 16u) * 65535u; 
    let thread_in_workgroup = (id.x % 16u) + (id.y % 16u) * 16u;
    let random_seed_index = workgroup_linear_id * 256u + thread_in_workgroup;
    
    // Bounds check - exit if this thread doesn't correspond to a valid agent
    if (agent_index >= arrayLength(&agents)) {
        return;
    }

    // Get agent data
    let agent = agents[agent_index];
    var x = agent.x;
    var y = agent.y;
    var angle = agent.z;
    var speed = agent.w;

    // Sample trail map at sensor positions
    let sensor_distance = sim_size.agent_sensor_distance;
    let sensor_angle = sim_size.agent_sensor_angle;
    
    // Calculate sensor positions
    let left_angle = angle - sensor_angle;
    let right_angle = angle + sensor_angle;
    
    let left_pos = vec2<f32>(
        x + cos(left_angle) * sensor_distance,
        y + sin(left_angle) * sensor_distance
    );
    let right_pos = vec2<f32>(
        x + cos(right_angle) * sensor_distance,
        y + sin(right_angle) * sensor_distance
    );
    
    // Sample combined trail + gradient maps at sensor positions
    // Use fast sampling for better performance (sacrifices some accuracy for speed)
    let left_value = sample_combined_map_fast(left_pos);
    let right_value = sample_combined_map_fast(right_pos);
    
    // Update angle based on sensor readings
    if (left_value > right_value) {
        // Calculate shortest path to turn left
        let target_angle = angle - TAU;
        let angle_diff = target_angle - angle;
        angle += min(sim_size.agent_turn_rate, abs(angle_diff)) * sign(angle_diff);
    } else if (right_value > left_value) {
        // Calculate shortest path to turn right
        let target_angle = angle + TAU;
        let angle_diff = target_angle - angle;
        angle += min(sim_size.agent_turn_rate, abs(angle_diff)) * sign(angle_diff);
    } else {
        // If equal, do nothing
    }

    // Update agent position
    let move_dist = speed * TIME_STEP;
    x = x + move_dist * cos(angle);
    y = y + move_dist * sin(angle);

    // --- CURSOR INTERACTION ---
    if (cursor.is_active > 0u) {
        let cursor_pos = vec2<f32>(cursor.x, cursor.y);
        let delta = cursor_pos - vec2<f32>(x, y);
        let dist = length(delta);
        if (dist < cursor.size && dist > 0.01) {
            let dir = normalize(delta);
            let force = cursor.strength * (1.0 - dist / cursor.size);
            if (cursor.is_active == 1u) {
                // Attract
                x += dir.x * force;
                y += dir.y * force;
            } else if (cursor.is_active == 2u) {
                // Repel
                x -= dir.x * force;
                y -= dir.y * force;
            }
        }
    }
    // --- END CURSOR INTERACTION ---

    // Apply jitter with proper random distribution
    let jitter_strength = sim_size.agent_jitter;
    let jitter_x_seed = hash(random_seed_index * 2654435761u + 1013904223u);
    let jitter_y_seed = hash(random_seed_index * 1664525u + 1073741827u);
    let random_x = random_float(jitter_x_seed);
    let random_y = random_float(jitter_y_seed);
    x += (random_x * 2.0 - 1.0) * jitter_strength;
    y += (random_y * 2.0 - 1.0) * jitter_strength;

    // Wrap agent position to stay within bounds (toroidal)
    x = x % f32(sim_size.width);
    if (x < 0.0) { x = x + f32(sim_size.width); }
    y = y % f32(sim_size.height);
    if (y < 0.0) { y = y + f32(sim_size.height); }

    // Deposit trail
    let deposit_x = i32(x);
    let deposit_y = i32(y);
    if (deposit_x >= 0 && deposit_x < i32(sim_size.width) && deposit_y >= 0 && deposit_y < i32(sim_size.height)) {
        let idx = deposit_y * i32(sim_size.width) + deposit_x;
        trail_map[idx] = clamp(trail_map[idx] + sim_size.pheromone_deposition_rate * 0.01, 0.0, 1.0);
    }

    // Update agent in the buffer
    agents[agent_index] = vec4<f32>(x, y, angle, speed);
}

// Add a new compute entry point for trail decay
@compute @workgroup_size(16, 16, 1)
fn decay_trail(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= sim_size.width || y >= sim_size.height) {
        return;
    }
    let idx = y * sim_size.width + x;
    // Apply decay rate
    let decay_rate = sim_size.decay_rate * 0.0001;
    trail_map[idx] = max(0.0, trail_map[idx] - decay_rate);
}

// Add a new compute entry point for diffusion
@compute @workgroup_size(16, 16, 1)
fn diffuse_trail(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= sim_size.width || y >= sim_size.height) {
        return;
    }
    let idx = y * sim_size.width + x;
    // Get neighboring values with toroidal wrapping
    let x_prev = (x + sim_size.width - 1) % sim_size.width;
    let x_next = (x + 1) % sim_size.width;
    let y_prev = (y + sim_size.height - 1) % sim_size.height;
    let y_next = (y + 1) % sim_size.height;
    
    let center = trail_map[y * sim_size.width + x];
    let left = trail_map[y * sim_size.width + x_prev];
    let right = trail_map[y * sim_size.width + x_next];
    let up = trail_map[y_prev * sim_size.width + x];
    let down = trail_map[y_next * sim_size.width + x];
    
    // Simple diffusion: average of neighbors
    let diffusion_rate = sim_size.diffusion_rate * 0.01;
    let new_value = center * (1.0 - diffusion_rate) + 
                   (left + right + up + down) * (diffusion_rate * 0.25);
    
    trail_map[y * sim_size.width + x] = new_value;
}

@compute @workgroup_size(16, 16, 1)
fn update_agent_speeds(@builtin(global_invocation_id) id: vec3<u32>) {
    // Calculate linear agent index from 2D global invocation
    let agents_per_row = 65535u * 16u; // Max workgroups per row * threads per workgroup row
    let agent_index = id.x + id.y * agents_per_row;
    
    // Bounds check - exit if this thread doesn't correspond to a valid agent
    if (agent_index >= arrayLength(&agents)) {
        return;
    }
    
    // Get current agent data
    let agent = agents[agent_index];
    let x = agent.x;
    let y = agent.y;
    let angle = agent.z;
    
    // Generate new random speed within the current range
    let random_speed = fract(sin(f32(agent_index) * 12.9898 + 78.233) * 43758.5453);
    let speed_range = sim_size.agent_speed_max - sim_size.agent_speed_min;
    let new_speed = sim_size.agent_speed_min + random_speed * speed_range;
    
    // Update agent with new speed
    agents[agent_index] = vec4<f32>(x, y, angle, new_speed);
}

// Better random number generation using multiple hash functions
fn hash(seed: u32) -> u32 {
    var x = seed;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return x;
}

fn random_float(seed: u32) -> f32 {
    return f32(hash(seed)) / f32(0xffffffffu);
}

fn random_range(seed: u32, min_val: f32, max_val: f32) -> f32 {
    return min_val + random_float(seed) * (max_val - min_val);
}

@compute @workgroup_size(64, 1, 1)
fn reset_agents(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // With 2D dispatch and workgroup_size(64, 1, 1):
    // global_id.x = linear thread index across all workgroups
    // global_id.y = second dimension for large dispatches  
    let agent_index = global_id.x + global_id.y * 65535u * 64u;
    let total_agents = arrayLength(&agents);
    
    if (agent_index >= total_agents) {
        return;
    }
    
    // Use multiple different seeds for better randomness, incorporating the random seed
    let base_seed = agent_index * 2654435761u + sim_size.random_seed;
    let x_seed = hash(base_seed);
    let y_seed = hash(base_seed + 1013904223u);
    let angle_seed = hash(base_seed + 1664525u);
    
    // Generate random position
    let x = random_range(x_seed, 0.0, f32(sim_size.width));
    let y = random_range(y_seed, 0.0, f32(sim_size.height));
    
    // Generate random angle
    let angle = random_range(angle_seed, 0.0, TAU);
    
    // Set speed to average of min/max
    let speed = (sim_size.agent_speed_min + sim_size.agent_speed_max) * 0.5;
    
    // Update agent
    agents[agent_index] = vec4<f32>(x, y, angle, speed);
} 