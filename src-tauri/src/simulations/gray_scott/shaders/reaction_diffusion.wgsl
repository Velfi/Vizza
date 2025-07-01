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
}

struct UVPair {
    u: f32,
    v: f32,
}

@group(0) @binding(0) var<storage, read> uvs_in: array<UVPair>;
@group(0) @binding(1) var<storage, read_write> uvs_out: array<UVPair>;
@group(0) @binding(2) var<uniform> params: SimulationParams;

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
    
    // Cardinal directions (weight 0.2)
    let left = uvs_in[get_index(x - 1, y)];
    let right = uvs_in[get_index(x + 1, y)];
    let up = uvs_in[get_index(x, y - 1)];
    let down = uvs_in[get_index(x, y + 1)];
    laplacian += vec2<f32>(left.u, left.v) * 0.2;
    laplacian += vec2<f32>(right.u, right.v) * 0.2;
    laplacian += vec2<f32>(up.u, up.v) * 0.2;
    laplacian += vec2<f32>(down.u, down.v) * 0.2;
    
    // Diagonal directions (weight 0.05)
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
            result = select(0.5, 1.0, is_checker);
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
            result = select(0.5, 1.0, is_stripe);
        }
        case 5u: { // Horizontal stripes
            let stripe_width = 0.1;
            let is_stripe = (ny / stripe_width) % 2.0 < 1.0;
            result = select(0.5, 1.0, is_stripe);
        }
        case 6u: { // Enhanced Noise with fBm
            let x_u = u32(x);
            let y_u = u32(y);
            
            var fBm = 0.0;
            var amplitude = 0.5;
            var frequency = 1.0;
            
            // Add multiple octaves of noise
            for (var i = 0u; i < 4u; i = i + 1u) {
                let scaled_x = u32(f32(x_u) * frequency);
                let scaled_y = u32(f32(y_u) * frequency);
                fBm += noise2D(scaled_x, scaled_y, i) * amplitude;
                frequency *= 2.0;
                amplitude *= 0.5;
            }
            
            // Add some periodic variation
            let periodic = sin(f32(x_u) * 0.02) * cos(f32(y_u) * 0.02) * 0.2;
            
            // Combine and adjust contrast
            result = 0.5 + pow(fBm + periodic, 2.0) * 0.5;
            result = clamp(result, 0.5, 1.0);
        }
        case 7u: { // Wave function f(x,y) = xe^(-(x² + y²))
            let x_norm = (nx * 4.0) - 2.0;
            let y_norm = (ny * 4.0) - 2.0;
            let squared_dist = x_norm * x_norm + y_norm * y_norm;
            let wave = x_norm * exp(-squared_dist);
            result = 0.5 + ((wave + 0.43) / 0.86) * 0.5;
        }
        case 8u: { // Enhanced cosine grid with phase and frequency variations
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
            
            // Normalize to [0.5, 1.0] with smoother transition
            result = 0.5 + (tanh(raw) * 0.5);
        }
        default: {
            result = 1.0;
        }
    }
    
    // If reversed, invert the pattern (but keep it in the 0.5 to 1.0 range)
    if (params.is_nutrient_pattern_reversed != 0u) {
        result = 1.5 - result;
    }
    
    return result;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
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
    
    // Incorporate nutrient factor into the feed rate
    let effective_feed_rate = params.feed_rate * nutrient_factor;
    
    let delta_u = params.delta_u * laplacian.x - reaction_rate + effective_feed_rate * (1.0 - uv.u);
    let delta_v = params.delta_v * laplacian.y + reaction_rate - (params.kill_rate + effective_feed_rate) * uv.v;
    
    let new_u = clamp(uv.u + delta_u * params.timestep, 0.0, 1.0);
    let new_v = clamp(uv.v + delta_v * params.timestep, 0.0, 1.0);
    
    uvs_out[idx] = UVPair(new_u, new_v);
} 