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


@group(0) @binding(0) var uvs_in: texture_storage_2d<rgba32float, read>;
@group(0) @binding(1) var uvs_out: texture_storage_2d<rgba32float, write>;
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
    // Isotropic 9-point Laplacian approximation:
    // (1 4 1; 4 -20 4; 1 4 1) / 6, applied per-channel (U and V)
    let left_x = (x - 1 + width) % width;
    let right_x = (x + 1 + width) % width;
    let up_y = (y - 1 + height) % height;
    let down_y = (y + 1 + height) % height;

    let left = textureLoad(uvs_in, vec2<i32>(left_x, wrapped_y)).xy;
    let right = textureLoad(uvs_in, vec2<i32>(right_x, wrapped_y)).xy;
    let up = textureLoad(uvs_in, vec2<i32>(wrapped_x, up_y)).xy;
    let down = textureLoad(uvs_in, vec2<i32>(wrapped_x, down_y)).xy;

    let up_left = textureLoad(uvs_in, vec2<i32>((x - 1 + width) % width, (y - 1 + height) % height)).xy;
    let up_right = textureLoad(uvs_in, vec2<i32>((x + 1 + width) % width, (y - 1 + height) % height)).xy;
    let down_left = textureLoad(uvs_in, vec2<i32>((x - 1 + width) % width, (y + 1 + height) % height)).xy;
    let down_right = textureLoad(uvs_in, vec2<i32>((x + 1 + width) % width, (y + 1 + height) % height)).xy;

    let neighbors4 = left + right + up + down;
    let neighbors_diag = up_left + up_right + down_left + down_right;

    let isotropic = (4.0 * neighbors4 + neighbors_diag - 20.0 * current) / 6.0;

    // Conservative scaling to preserve similar overall diffusion strength
    return isotropic * 0.30;
}

// --- Gaussian diffusion helpers ---
// 5x5 Gaussian blur per-channel with wrap addressing.
// Uses σ derived from diffusion: σ² ≈ 2 * D * dt (in pixel units).
fn gaussian_blur_component(x: i32, y: i32, sigma: f32, channel: u32) -> f32 {
    let width = i32(params.width);
    let height = i32(params.height);

    // Avoid division by zero / extremely sharp kernel
    let sigma_safe = max(sigma, 0.001);
    let inv_sigma2 = 1.0 / (sigma_safe * sigma_safe);

    // 1D weights for offsets 0,1,2
    var w: array<f32, 3>;
    w[0] = 1.0;
    w[1] = exp(-0.5 * (1.0 * 1.0) * inv_sigma2);
    w[2] = exp(-0.5 * (2.0 * 2.0) * inv_sigma2);
    let norm = w[0] + 2.0 * (w[1] + w[2]);
    w[0] = w[0] / norm;
    w[1] = w[1] / norm;
    w[2] = w[2] / norm;

    var acc = 0.0;
    for (var oy = -2; oy <= 2; oy = oy + 1) {
        let wy = w[u32(abs(oy))];
        let yy = (y + oy + height) % height;
        for (var ox = -2; ox <= 2; ox = ox + 1) {
            let wx = w[u32(abs(ox))];
            let xx = (x + ox + width) % width;
            let s = textureLoad(uvs_in, vec2<i32>(xx, yy));
            let v = select(s.y, s.x, channel == 0u);
            acc = acc + (wx * wy) * v;
        }
    }
    return acc;
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



// Use 16x16 workgroup to avoid visible grid artifacts
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
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
    let nutrient_factor = get_nutrient_factor(x, y);
    
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
    
    // Incorporate nutrient factor into the feed rate
    // Map nutrient factor from [0,1] to [0.5,1.0] for feed rate scaling
    let effective_feed_rate = params.feed_rate * (0.5 + nutrient_factor * 0.5);

    // Gaussian diffusion step (operator splitting):
    // Perform exact diffusion over dt via Gaussian blur with σ^2 = 2 D dt
    let sigma_u = sqrt(max(2.0 * params.delta_u * effective_timestep, 0.0));
    let sigma_v = sqrt(max(2.0 * params.delta_v * effective_timestep, 0.0));
    let u_diff = gaussian_blur_component(wrapped_x, wrapped_y, sigma_u, 0u);
    let v_diff = gaussian_blur_component(wrapped_x, wrapped_y, sigma_v, 1u);
    let uv_diff = vec2<f32>(u_diff, v_diff);

    // Reaction step using diffused concentrations
    let reaction_rate = uv_diff.x * uv_diff.y * uv_diff.y;
    let du_react = -reaction_rate + effective_feed_rate * (1.0 - uv_diff.x);
    let dv_react =  reaction_rate - (params.kill_rate + effective_feed_rate) * uv_diff.y;

    let new_u = clamp(uv_diff.x + du_react * effective_timestep, 0.0, 1.0);
    let new_v = clamp(uv_diff.y + dv_react * effective_timestep, 0.0, 1.0);
    
    textureStore(uvs_out, vec2<i32>(wrapped_x, wrapped_y), vec4<f32>(new_u, new_v, 0.0, 0.0));
} 