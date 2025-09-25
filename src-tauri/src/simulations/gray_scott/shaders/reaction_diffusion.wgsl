struct SimulationParams {
    feed_rate: f32,
    kill_rate: f32,
    delta_u: f32,
    delta_v: f32,

    timestep: f32,
    width: u32,
    height: u32,
    
    // Mask system
    mask_pattern: u32,
    mask_target: u32,
    mask_strength: f32,
    mask_mirror_horizontal: u32,
    mask_mirror_vertical: u32,
    mask_invert_tone: u32,
    
    // Adaptive timestep parameters
    max_timestep: f32,
    stability_factor: f32,
    enable_adaptive_timestep: u32,
}


struct UVPair {
    u: f32,
    v: f32,
}


@group(0) @binding(0) var uvs_in: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var uvs_out: texture_storage_2d<rgba16float, write>;
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
    let width = i32(params.width);
    let height = i32(params.height);
    let wrapped_x = (x + width) % width;
    let wrapped_y = (y + height) % height;
    
    let current_sample = textureLoad(uvs_in, vec2<i32>(wrapped_x, wrapped_y));
    let current = current_sample.xy; // Extract only the first two components (RG -> UV)
    
    var laplacian = vec2<f32>(0.0);
    
    // 5-point stencil: center + 4 cardinal neighbors
    // Center weight: -4 (for 5-point stencil)
    laplacian -= current * 4.0;
    
    // Cardinal directions (weight 1.0 each) - batch the lookups for better performance
    let left_x = (x - 1 + width) % width;
    let right_x = (x + 1 + width) % width;
    let up_y = (y - 1 + height) % height;
    let down_y = (y + 1 + height) % height;
    
    let left_sample = textureLoad(uvs_in, vec2<i32>(left_x, wrapped_y));
    let right_sample = textureLoad(uvs_in, vec2<i32>(right_x, wrapped_y));
    let up_sample = textureLoad(uvs_in, vec2<i32>(wrapped_x, up_y));
    let down_sample = textureLoad(uvs_in, vec2<i32>(wrapped_x, down_y));
    
    let left = left_sample.xy;
    let right = right_sample.xy;
    let up = up_sample.xy;
    let down = down_sample.xy;
    
    laplacian += left;
    laplacian += right;
    laplacian += up;
    laplacian += down;
    
    return laplacian;
}

fn hash(n: u32) -> f32 {
    return fract(sin(f32(n)) * 43758.5453);
}

fn noise2D(x: u32, y: u32, seed: u32) -> f32 {
    return hash(x * 73856093u + y * 19349663u + seed);
}

fn get_mask_factor(x: i32, y: i32) -> f32 {
    // Calculate normalized coordinates
    let original_nx = f32(x) / f32(params.width);
    let original_ny = f32(y) / f32(params.height);
    
    // For image-based masks, we need to handle mirroring differently
    let pattern = params.mask_pattern;
    
    var result = 0.0;
    
    switch (pattern) {
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
            var nx = original_nx;
            var ny = original_ny;
            // Apply mirror transformations
            if (params.mask_mirror_horizontal != 0u) {
                nx = 1.0 - nx;
            }
            if (params.mask_mirror_vertical != 0u) {
                ny = 1.0 - ny;
            }
            result = (nx + ny) / 2.0;
        }
        case 3u: { // Radial gradient
            var nx = original_nx;
            var ny = original_ny;
            // Apply mirror transformations
            if (params.mask_mirror_horizontal != 0u) {
                nx = 1.0 - nx;
            }
            if (params.mask_mirror_vertical != 0u) {
                ny = 1.0 - ny;
            }
            let center_x = 0.5;
            let center_y = 0.5;
            let dx = nx - center_x;
            let dy = ny - center_y;
            let distance = sqrt(dx * dx + dy * dy);
            result = 1.0 - distance;
        }
        case 4u: { // Vertical stripes
            var nx = original_nx;
            // Apply mirror transformations
            if (params.mask_mirror_horizontal != 0u) {
                nx = 1.0 - nx;
            }
            let stripe_width = 0.1;
            let is_stripe = (nx / stripe_width) % 2.0 < 1.0;
            result = select(0.0, 1.0, is_stripe);
        }
        case 5u: { // Horizontal stripes
            var ny = original_ny;
            // Apply mirror transformations
            if (params.mask_mirror_vertical != 0u) {
                ny = 1.0 - ny;
            }
            let stripe_width = 0.1;
            let is_stripe = (ny / stripe_width) % 2.0 < 1.0;
            result = select(0.0, 1.0, is_stripe);
        }
        case 6u: { // Wave function f(x,y) = xe^(-(x² + y²))
            var nx = original_nx;
            var ny = original_ny;
            // Apply mirror transformations
            if (params.mask_mirror_horizontal != 0u) {
                nx = 1.0 - nx;
            }
            if (params.mask_mirror_vertical != 0u) {
                ny = 1.0 - ny;
            }
            let x_norm = (nx * 4.0) - 2.0;
            let y_norm = (ny * 4.0) - 2.0;
            let squared_dist = x_norm * x_norm + y_norm * y_norm;
            let wave = x_norm * exp(-squared_dist);
            result = (wave + 0.43) / 0.86;
        }
        case 7u: { // Enhanced cosine grid with phase and frequency variations
            var nx = original_nx;
            var ny = original_ny;
            // Apply mirror transformations
            if (params.mask_mirror_horizontal != 0u) {
                nx = 1.0 - nx;
            }
            if (params.mask_mirror_vertical != 0u) {
                ny = 1.0 - ny;
            }
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
            // For image masks, we need to apply mirroring to the sampling coordinates
            var sample_x = original_nx;
            var sample_y = original_ny;
            
            // Apply mirror transformations to sampling coordinates
            if (params.mask_mirror_horizontal != 0u) {
                sample_x = 1.0 - sample_x;
            }
            if (params.mask_mirror_vertical != 0u) {
                sample_y = 1.0 - sample_y;
            }
            
            // Use mirrored sampling coordinates for image indexing
            let mirrored_x = i32(sample_x * f32(params.width));
            let mirrored_y = i32(sample_y * f32(params.height));
            let idx = get_index(mirrored_x, mirrored_y);
            // Map [0,1] image to [0,1] nutrient factor
            let img = clamp(gradient_map[idx], 0.0, 1.0);
            result = img;
        }
        default: {
            result = 1.0;
        }
    }
    
    // If tone inverted, invert the pattern (keep it in the 0 to 1 range)
    if (params.mask_invert_tone != 0u) {
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



@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    
    if (x >= i32(params.width) || y >= i32(params.height)) {
        return;
    }
    
    let width = i32(params.width);
    let height = i32(params.height);
    let wrapped_x = (x + width) % width;
    let wrapped_y = (y + height) % height;
    
    let uv_sample = textureLoad(uvs_in, vec2<i32>(wrapped_x, wrapped_y));
    let uv = uv_sample.xy; // Extract only the first two components (RG -> UV)
    let reaction_rate = uv.x * uv.y * uv.y;
    
    let laplacian = get_laplacian(x, y);
    let mask_factor = get_mask_factor(x, y);
    
    // Use adaptive timestep only if enabled, otherwise use the user-specified timestep
    var effective_timestep: f32;
    if (params.enable_adaptive_timestep != 0u) {
        effective_timestep = calculate_adaptive_timestep(
            params.delta_u,
            params.delta_v,
            params.feed_rate,
            params.kill_rate,
            params.stability_factor
        );
    } else {
        effective_timestep = params.timestep;
    }
    
    // Apply mask to the appropriate parameters based on mask_target
    var effective_feed_rate = params.feed_rate;
    var effective_kill_rate = params.kill_rate;
    var effective_delta_u = params.delta_u;
    var effective_delta_v = params.delta_v;
    
    // Calculate mask influence: 0.0 = no effect, 1.0 = full effect
    let mask_influence = mask_factor * params.mask_strength;
    
    switch (params.mask_target) {
        case 1u: { // FeedRate
            // Map mask factor from [0,1] to [0.5,1.0] for feed rate scaling
            effective_feed_rate = params.feed_rate * (0.5 + mask_influence * 0.5);
        }
        case 2u: { // KillRate
            // Map mask factor from [0,1] to [0.5,1.0] for kill rate scaling
            effective_kill_rate = params.kill_rate * (0.5 + mask_influence * 0.5);
        }
        case 3u: { // DiffusionU
            // Map mask factor from [0,1] to [0.5,1.5] for diffusion scaling
            effective_delta_u = params.delta_u * (0.5 + mask_influence * 1.0);
        }
        case 4u: { // DiffusionV
            // Map mask factor from [0,1] to [0.5,1.5] for diffusion scaling
            effective_delta_v = params.delta_v * (0.5 + mask_influence * 1.0);
        }
        case 5u: { // UVConcentration
            // This would affect initial concentrations, but we'll implement it as a feed rate effect for now
            effective_feed_rate = params.feed_rate * (0.5 + mask_influence * 0.5);
        }
        default: { // None or any other value
            // No masking applied - use original values
        }
    }
    
    let delta_u = effective_delta_u * laplacian.x - reaction_rate + effective_feed_rate * (1.0 - uv.x);
    let delta_v = effective_delta_v * laplacian.y + reaction_rate - (effective_kill_rate + effective_feed_rate) * uv.y;
    
    let new_u = clamp(uv.x + delta_u * effective_timestep, 0.0, 1.0);
    let new_v = clamp(uv.y + delta_v * effective_timestep, 0.0, 1.0);
    
    textureStore(uvs_out, vec2<i32>(wrapped_x, wrapped_y), vec4<f32>(new_u, new_v, 0.0, 0.0));
} 