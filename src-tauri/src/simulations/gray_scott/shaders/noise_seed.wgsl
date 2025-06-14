// GPU-based noise seeding shader for Gray-Scott simulation
// Based on the standalone implementation optimizations

@group(0) @binding(0)
var<storage, read_write> uvs_data: array<vec2<f32>>;

@group(0) @binding(1)
var<uniform> params: SimulationParams;

struct SimulationParams {
    width: u32,
    height: u32,
    seed: u32,
    noise_strength: f32,
}

// High-quality hash function for pseudo-random generation
fn hash(seed: u32) -> u32 {
    var x = seed;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return x;
}

// Convert hash to float [0.0, 1.0]
fn random_float(seed: u32) -> f32 {
    return f32(hash(seed)) / f32(0xffffffffu);
}

// 2D spatial noise function
fn noise2D(x: u32, y: u32, seed: u32) -> f32 {
    return random_float(x * 73856093u + y * 19349663u + seed);
}

// Fractal Brownian Motion (fBm) for richer noise patterns
fn fbm_noise(x: u32, y: u32, seed: u32) -> f32 {
    var result = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    
    // Multiple octaves for fractal detail
    for (var i = 0u; i < 4u; i = i + 1u) {
        let scaled_x = u32(f32(x) * frequency);
        let scaled_y = u32(f32(y) * frequency);
        result += noise2D(scaled_x, scaled_y, seed + i) * amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    return result;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    // Check bounds
    if (x >= params.width || y >= params.height) {
        return;
    }
    
    let index = y * params.width + x;
    
    // Generate noise using spatial coordinates
    let noise_value = fbm_noise(x, y, params.seed);
    
    // Apply noise with configurable strength
    if (noise_value < (0.05 * params.noise_strength)) {
        // Seed reaction areas with high V concentration
        let u_val = 0.2 + noise_value * 0.3;
        let v_val = 0.8 + noise_value * 0.2;
        uvs_data[index] = vec2<f32>(u_val, v_val);
    } else {
        // Default empty state
        uvs_data[index] = vec2<f32>(1.0, 0.0);
    }
}