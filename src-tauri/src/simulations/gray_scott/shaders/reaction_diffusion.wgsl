struct SimulationParams {
    feed_rate: f32,
    kill_rate: f32,
    delta_u: f32,
    delta_v: f32,

    timestep: f32,
    width: u32,
    height: u32,
    nutrient_pattern: u32,
    
    is_nutrient_pattern_reversed: u32,
    // Adaptive timestep parameters
    max_timestep: f32,
    stability_factor: f32,
    enable_adaptive_timestep: u32,
}


struct UVPair {
    u: f32,
    v: f32,
}


@group(0) @binding(0) var<storage, read> uvs_in: array<UVPair>;
@group(0) @binding(1) var<storage, read_write> uvs_out: array<UVPair>;
@group(0) @binding(2) var<uniform> params: SimulationParams;
// Optional image-driven nutrient pattern (bound only when used)
@group(0) @binding(3) var<storage, read> gradient_map: array<f32>;

fn get_index(x: i32, y: i32) -> u32 {
    let width = i32(params.width);
    let height = i32(params.height);
    let wrapped_x = (x + width) % width;
    let wrapped_y = (y + height) % height;
    return u32(wrapped_y * width + wrapped_x);
}


fn get_laplacian(x: i32, y: i32) -> vec2<f32> {
    let idx = get_index(x, y);
    let current = uvs_in[idx];
    
    var laplacian = vec2<f32>(0.0);
    
    // Center weight
    laplacian -= vec2<f32>(current.u, current.v) * 1.0;
    
    // Cardinal directions (weight 0.2) - batch the lookups for better performance
    let left_idx = get_index(x - 1, y);
    let right_idx = get_index(x + 1, y);
    let up_idx = get_index(x, y - 1);
    let down_idx = get_index(x, y + 1);
    
    let left = uvs_in[left_idx];
    let right = uvs_in[right_idx];
    let up = uvs_in[up_idx];
    let down = uvs_in[down_idx];
    
    laplacian += vec2<f32>(left.u, left.v) * 0.2;
    laplacian += vec2<f32>(right.u, right.v) * 0.2;
    laplacian += vec2<f32>(up.u, up.v) * 0.2;
    laplacian += vec2<f32>(down.u, down.v) * 0.2;
    
    // Restore all diagonal directions for symmetric diffusion
    let up_left = uvs_in[get_index(x - 1, y - 1)];
    let up_right = uvs_in[get_index(x + 1, y - 1)];
    let down_left = uvs_in[get_index(x - 1, y + 1)];
    let down_right = uvs_in[get_index(x + 1, y + 1)];
    laplacian += vec2<f32>(up_left.u, up_left.v) * 0.05;
    laplacian += vec2<f32>(up_right.u, up_right.v) * 0.05;
    laplacian += vec2<f32>(down_left.u, down_left.v) * 0.05;
    laplacian += vec2<f32>(down_right.u, down_right.v) * 0.05;
    
    return laplacian;
}

fn hash(n: u32) -> f32 {
    return fract(sin(f32(n)) * 43758.5453);
}

fn noise2D(x: u32, y: u32, seed: u32) -> f32 {
    return hash(x * 73856093u + y * 19349663u + seed);
}

fn get_nutrient_factor(x: i32, y: i32) -> f32 {
    // Calculate normalized coordinates
    let nx = f32(x) / f32(params.width);
    let ny = f32(y) / f32(params.height);
    
    var result = 0.0;
    
    switch (params.nutrient_pattern) {
        case 0u: { // Uniform
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
            result = (nx + ny) / 2.0;
        }
        case 3u: { // Radial gradient
            let center_x = 0.5;
            let center_y = 0.5;
            let dx = nx - center_x;
            let dy = ny - center_y;
            let distance = sqrt(dx * dx + dy * dy);
            result = 1.0 - distance;
        }
        case 4u: { // Vertical stripes
            let stripe_width = 0.1;
            let is_stripe = (nx / stripe_width) % 2.0 < 1.0;
            result = select(0.0, 1.0, is_stripe);
        }
        case 5u: { // Horizontal stripes
            let stripe_width = 0.1;
            let is_stripe = (ny / stripe_width) % 2.0 < 1.0;
            result = select(0.0, 1.0, is_stripe);
        }
        case 6u: { // Wave function f(x,y) = xe^(-(x² + y²))
            let x_norm = (nx * 4.0) - 2.0;
            let y_norm = (ny * 4.0) - 2.0;
            let squared_dist = x_norm * x_norm + y_norm * y_norm;
            let wave = x_norm * exp(-squared_dist);
            result = (wave + 0.43) / 0.86;
        }
        case 7u: { // Enhanced cosine grid with phase and frequency variations
            // Scale coordinates with different frequencies
            let x_scaled = nx * 18.85; // 6π
            let y_scaled = ny * 12.566; // 4π
            
            // Add phase variation and cross-modulation
            let pattern1 = cos(x_scaled + cos(y_scaled * 0.5));
            let pattern2 = cos(y_scaled + sin(x_scaled * 0.3));
            
            // Create interesting interference pattern
            let interference = pattern1 * pattern2;
            
            // Add non-linear transformation
            let raw = -(interference * interference) * cos(x_scaled * 0.5);
            
            // Normalize to [0, 1] with smoother transition
            result = (tanh(raw) + 1.0) * 0.5;
        }
        case 8u: { // Image-driven gradient map
            let idx = get_index(x, y);
            // Map [0,1] image to [0,1] nutrient factor
            let img = clamp(gradient_map[idx], 0.0, 1.0);
            result = img;
        }
        default: {
            result = 1.0;
        }
    }
    
    // If reversed, invert the pattern (keep it in the 0 to 1 range)
    if (params.is_nutrient_pattern_reversed != 0u) {
        result = 1.0 - result;
    }
    
    return result;
}

fn calculate_adaptive_timestep(delta_u: f32, delta_v: f32, feed_rate: f32, kill_rate: f32, stability_factor: f32) -> f32 {
    // Von Neumann stability condition for 2D diffusion
    // For 2D, the condition is dt <= 0.25 / (Du + Dv) where Du, Dv are diffusion coefficients
    let diffusion_limit = 0.25 / (delta_u + delta_v);
    
    // Reaction rate stability - consider the maximum reaction rate
    // The reaction term uv² can be at most 1.0 when u=1, v=1
    // A more conservative estimate: dt <= 1.0 / (max_reaction_rate + feed_rate + kill_rate)
    let max_reaction_rate = 1.0; // uv² <= 1.0
    let reaction_limit = 1.0 / (max_reaction_rate + feed_rate + kill_rate);
    
    // Take the minimum for stability
    let stable_timestep = min(diffusion_limit, reaction_limit) * stability_factor;
    
    return stable_timestep;
}



// Balanced workgroup size for better GPU utilization and stability
// 16x16 provides good balance between performance and numerical stability
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    
    if (x >= i32(params.width) || y >= i32(params.height)) {
        return;
    }
    
    let idx = get_index(x, y);
    
    let uv = uvs_in[idx];
    let reaction_rate = uv.u * uv.v * uv.v;
    
    let laplacian = get_laplacian(x, y);
    let nutrient_factor = get_nutrient_factor(x, y);
    
    // Always use adaptive timestep for better stability
    let effective_timestep = calculate_adaptive_timestep(
        params.delta_u,
        params.delta_v,
        params.feed_rate,
        params.kill_rate,
        params.stability_factor
    );
    
    // Incorporate nutrient factor into the feed rate
    // Map nutrient factor from [0,1] to [0.5,1.0] for feed rate scaling
    let effective_feed_rate = params.feed_rate * (0.5 + nutrient_factor * 0.5);
    
    let delta_u = params.delta_u * laplacian.x - reaction_rate + effective_feed_rate * (1.0 - uv.u);
    let delta_v = params.delta_v * laplacian.y + reaction_rate - (params.kill_rate + effective_feed_rate) * uv.v;
    
    let new_u = clamp(uv.u + delta_u * effective_timestep, 0.0, 1.0);
    let new_v = clamp(uv.v + delta_v * effective_timestep, 0.0, 1.0);
    
    let new_uv = UVPair(new_u, new_v);
    uvs_out[idx] = new_uv;
} 