// GPU-based flow vector generation with multiple noise types
// Supports time-varying flow fields

@group(0) @binding(0)
var<storage, read_write> flow_vectors: array<FlowVector>;

@group(0) @binding(1)
var<uniform> params: FlowVectorParams;

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
}

struct FlowVectorParams {
    grid_size: u32,
    noise_type: u32,
    noise_scale: f32,
    noise_x: f32,
    noise_y: f32,
    noise_seed: u32,
    time: f32,
    noise_dt_multiplier: f32,
    vector_magnitude: f32,
}

// Hash function for pseudo-random generation
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

// 3D spatial noise function
fn noise3D(x: u32, y: u32, z: u32, seed: u32) -> f32 {
    return random_float(x * 73856093u + y * 19349663u + z * 83492791u + seed);
}

// Simplex noise implementation (simplified version)
fn simplex_noise_2d(pos: vec2<f32>, seed: u32) -> f32 {
    // Simplified simplex noise using hash-based approach
    let x = floor(pos.x);
    let y = floor(pos.y);
    let fx = fract(pos.x);
    let fy = fract(pos.y);
    
    // Hash the corners
    let n00 = hash(u32(x) * 73856093u + u32(y) * 19349663u + seed);
    let n01 = hash(u32(x) * 73856093u + u32(y + 1.0) * 19349663u + seed);
    let n10 = hash(u32(x + 1.0) * 73856093u + u32(y) * 19349663u + seed);
    let n11 = hash(u32(x + 1.0) * 73856093u + u32(y + 1.0) * 19349663u + seed);
    
    // Smooth interpolation
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v_interp = fy * fy * (3.0 - 2.0 * fy);
    
    // Bilinear interpolation
    let a = mix(f32(n00), f32(n10), u);
    let b = mix(f32(n01), f32(n11), u);
    let result = mix(a, b, v_interp);
    
    return result / f32(0xffffffffu);
}

// 3D Simplex noise implementation
fn simplex_noise_3d(pos: vec3<f32>, seed: u32) -> f32 {
    // Simplified 3D simplex noise using hash-based approach
    let x = floor(pos.x);
    let y = floor(pos.y);
    let z = floor(pos.z);
    let fx = fract(pos.x);
    let fy = fract(pos.y);
    let fz = fract(pos.z);
    
    // Hash the corners
    let n000 = hash(u32(x) * 73856093u + u32(y) * 19349663u + u32(z) * 83492791u + seed);
    let n001 = hash(u32(x) * 73856093u + u32(y) * 19349663u + u32(z + 1.0) * 83492791u + seed);
    let n010 = hash(u32(x) * 73856093u + u32(y + 1.0) * 19349663u + u32(z) * 83492791u + seed);
    let n011 = hash(u32(x) * 73856093u + u32(y + 1.0) * 19349663u + u32(z + 1.0) * 83492791u + seed);
    let n100 = hash(u32(x + 1.0) * 73856093u + u32(y) * 19349663u + u32(z) * 83492791u + seed);
    let n101 = hash(u32(x + 1.0) * 73856093u + u32(y) * 19349663u + u32(z + 1.0) * 83492791u + seed);
    let n110 = hash(u32(x + 1.0) * 73856093u + u32(y + 1.0) * 19349663u + u32(z) * 83492791u + seed);
    let n111 = hash(u32(x + 1.0) * 73856093u + u32(y + 1.0) * 19349663u + u32(z + 1.0) * 83492791u + seed);
    
    // Smooth interpolation
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v_interp = fy * fy * (3.0 - 2.0 * fy);
    let w = fz * fz * (3.0 - 2.0 * fz);
    
    // Trilinear interpolation
    let a = mix(f32(n000), f32(n100), u);
    let b = mix(f32(n010), f32(n110), u);
    let c = mix(f32(n001), f32(n101), u);
    let d = mix(f32(n011), f32(n111), u);
    
    let e = mix(a, b, v_interp);
    let f = mix(c, d, v_interp);
    let result = mix(e, f, w);
    
    return result / f32(0xffffffffu);
}

// 3D Worley/Cellular noise (improved implementation)
fn worley_noise_3d(pos: vec3<f32>, seed: u32) -> f32 {
    let cell = floor(pos);
    let frac = pos - cell;
    
    // Determine which cell is closer for each dimension
    let half = vec3<bool>(frac.x > 0.5, frac.y > 0.5, frac.z > 0.5);
    let near = vec3<i32>(i32(half.x), i32(half.y), i32(half.z)) + vec3<i32>(cell);
    let far = vec3<i32>(i32(!half.x), i32(!half.y), i32(!half.z)) + vec3<i32>(cell);
    
    // Start with the near cell
    var seed_cell = near;
    let seed_index = hash(u32(near.x) * 73856093u + u32(near.y) * 19349663u + u32(near.z) * 83492791u + seed);
    let seed_point = get_worley_point_3d(seed_index, near);
    var distance = length(pos - seed_point);
    
    // Calculate range for optimized neighbor checking
    let range = vec3<f32>(
        pow(0.5 - frac.x, 2.0),
        pow(0.5 - frac.y, 2.0),
        pow(0.5 - frac.z, 2.0)
    );
    
    // Test neighboring cells based on range
    if (range.x < distance) {
        let test_cell = vec3<i32>(far.x, near.y, near.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    if (range.y < distance) {
        let test_cell = vec3<i32>(near.x, far.y, near.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    if (range.z < distance) {
        let test_cell = vec3<i32>(near.x, near.y, far.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    // Test edge cases (2D combinations)
    if (range.x < distance && range.y < distance) {
        let test_cell = vec3<i32>(far.x, far.y, near.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    if (range.x < distance && range.z < distance) {
        let test_cell = vec3<i32>(far.x, near.y, far.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    if (range.y < distance && range.z < distance) {
        let test_cell = vec3<i32>(near.x, far.y, far.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    // Test corner case (all 3 dimensions)
    if (range.x < distance && range.y < distance && range.z < distance) {
        let test_cell = vec3<i32>(far.x, far.y, far.z);
        let test_index = hash(u32(test_cell.x) * 73856093u + u32(test_cell.y) * 19349663u + u32(test_cell.z) * 83492791u + seed);
        let test_point = get_worley_point_3d(test_index, test_cell);
        let test_distance = length(pos - test_point);
        if (test_distance < distance) {
            distance = test_distance;
            seed_cell = test_cell;
        }
    }
    
    // Return distance value (normalized to -1 to 1 range)
    return distance * 2.0 - 1.0;
}

// Helper function to get Worley feature points (simplified version of official implementation)
fn get_worley_point_3d(index: u32, cell: vec3<i32>) -> vec3<f32> {
    let length = f32((index & 0xE0u) >> 5u) * 0.5 / 7.0;
    let diag = length * 0.5773502691896258; // 1/sqrt(3)
    
    let offset = vec3<f32>(
        select(-diag, diag, (index & 1u) == 0u),
        select(-diag, diag, (index & 2u) == 0u),
        select(-diag, diag, (index & 4u) == 0u)
    );
    
    return vec3<f32>(f32(cell.x), f32(cell.y), f32(cell.z)) + offset;
}

// 3D Fractal Brownian Motion (fBm)
fn fbm_noise_3d(pos: vec3<f32>, seed: u32, octaves: u32, lacunarity: f32, persistence: f32) -> f32 {
    var result = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var max_value = 0.0;
    
    for (var i = 0u; i < octaves; i = i + 1u) {
        let sample_pos = pos * frequency;
        let noise_val = simplex_noise_3d(sample_pos, seed + i);
        result += noise_val * amplitude;
        max_value += amplitude;
        frequency *= lacunarity;
        amplitude *= persistence;
    }
    
    return result / max_value;
}

// Fractal Brownian Motion (fBm)
fn fbm_noise(pos: vec2<f32>, seed: u32, octaves: u32, lacunarity: f32, persistence: f32) -> f32 {
    var result = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var max_value = 0.0;
    
    for (var i = 0u; i < octaves; i = i + 1u) {
        let sample_pos = pos * frequency;
        let noise_val = simplex_noise_2d(sample_pos, seed + i);
        result += noise_val * amplitude;
        max_value += amplitude;
        frequency *= lacunarity;
        amplitude *= persistence;
    }
    
    return result / max_value;
}

// Worley/Cellular noise (simplified)
fn worley_noise(pos: vec2<f32>, seed: u32) -> f32 {
    let cell_size = 0.1;
    let cell_pos = floor(pos / cell_size);
    let local_pos = fract(pos / cell_size);
    
    var min_dist = 1000.0;
    
    // Check current cell and 8 neighbors
    for (var dx = -1; dx <= 1; dx = dx + 1) {
        for (var dy = -1; dy <= 1; dy = dy + 1) {
            let check_cell = cell_pos + vec2<f32>(f32(dx), f32(dy));
            let cell_seed = hash(u32(check_cell.x) * 73856093u + u32(check_cell.y) * 19349663u + seed);
            let feature_point = vec2<f32>(
                random_float(cell_seed),
                random_float(cell_seed + 1u)
            );
            let dist = distance(local_pos + vec2<f32>(f32(dx), f32(dy)), feature_point);
            min_dist = min(min_dist, dist);
        }
    }
    
    return min_dist;
}

// Spheres noise implementation (based on official noise crate)
fn spheres_noise_2d(pos: vec2<f32>, seed: u32) -> f32 {
    let frequency = 1.0;
    let point = pos * frequency;
    let dist_from_center = length(point);
    let dist_from_smaller_sphere = dist_from_center - floor(dist_from_center);
    let dist_from_larger_sphere = 1.0 - dist_from_smaller_sphere;
    let nearest_dist = min(dist_from_smaller_sphere, dist_from_larger_sphere);
    return 1.0 - (nearest_dist * 4.0);
}

fn spheres_noise_3d(pos: vec3<f32>, seed: u32) -> f32 {
    let frequency = 1.0;
    let point = pos * frequency;
    let dist_from_center = length(point);
    let dist_from_smaller_sphere = dist_from_center - floor(dist_from_center);
    let dist_from_larger_sphere = 1.0 - dist_from_smaller_sphere;
    let nearest_dist = min(dist_from_smaller_sphere, dist_from_larger_sphere);
    return 1.0 - (nearest_dist * 4.0);
}

// Generate noise value based on type
fn generate_noise(pos: vec2<f32>, noise_type: u32, seed: u32, time: f32) -> f32 {
    let pos_3d = vec3<f32>(pos.x, pos.y, time * 0.1);
    
    switch (noise_type) {
        case 0u: { // OpenSimplex
            return simplex_noise_3d(pos_3d, seed);
        }
        case 1u: { // Worley
            return worley_noise_3d(pos_3d, seed);
        }
        case 2u: { // Value
            let x = u32(pos_3d.x * 100.0);
            let y = u32(pos_3d.y * 100.0);
            let z = u32(pos_3d.z * 100.0);
            return noise3D(x, y, z, seed);
        }
        case 3u: { // FBM
            return fbm_noise_3d(pos_3d, seed, 6u, 2.0, 0.5);
        }
        case 4u: { // FBMBillow
            return fbm_noise_3d(pos_3d, seed, 8u, 2.5, 0.7);
        }
        case 5u: { // FBMClouds
            return fbm_noise_3d(pos_3d, seed, 4u, 1.8, 0.3);
        }
        case 6u: { // FBMRidged
            let fbm_val = fbm_noise_3d(pos_3d, seed, 10u, 3.0, 0.9);
            return 1.0 - abs(fbm_val);
        }
        case 7u: { // Billow
            let fbm_val = fbm_noise_3d(pos_3d, seed, 6u, 2.0, 0.5);
            return abs(fbm_val) * 2.0 - 1.0;
        }
        case 8u: { // RidgedMulti
            let fbm_val = fbm_noise_3d(pos_3d, seed, 6u, 2.0, 0.5);
            return 1.0 - abs(fbm_val);
        }
        case 9u: { // Cylinders
            // Cylinders uses 3D spheres noise
            return spheres_noise_3d(pos_3d, seed);
        }
        case 10u: { // Checkerboard
            // Make grid size responsive to noise scale for better visibility
            let base_grid_size = 2.0; // Base size that gives good visibility
            let grid_size = base_grid_size / max(1.0, params.noise_scale);
            let floor_x = i32(floor(pos_3d.x / grid_size));
            let floor_y = i32(floor(pos_3d.y / grid_size));
            let floor_z = i32(floor(pos_3d.z / grid_size));
            
            // Use XOR operations like the official implementation
            // Create more distinct alternating pattern for better flow visualization
            if ((floor_x & 1) ^ (floor_y & 1) ^ (floor_z & 1)) == 0 {
                return 0.0; // One direction
            } else {
                return 0.5; // Perpendicular direction
            }
        }
        default: {
            return simplex_noise_3d(pos_3d, seed);
        }
    }
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    // Check bounds
    if (x >= params.grid_size || y >= params.grid_size) {
        return;
    }
    
    let index = y * params.grid_size + x;
    
    // Calculate world position
    let world_x = (f32(x) / f32(params.grid_size - 1u)) * 2.0 - 1.0;
    let world_y = (f32(y) / f32(params.grid_size - 1u)) * 2.0 - 1.0;
    
    // Apply noise scale and offset
    let sample_pos = vec2<f32>(
        world_x * params.noise_scale + params.noise_x,
        world_y * params.noise_scale + params.noise_y
    );
    
    // Generate noise value
    let noise_value = generate_noise(sample_pos, params.noise_type, params.noise_seed, params.time * params.noise_dt_multiplier);
    
    // Create flow direction from noise value
    let angle = noise_value * 6.28318530718; // 2 * PI
    let direction = vec2<f32>(cos(angle), sin(angle)) * params.vector_magnitude;
    
    // Store flow vector
    flow_vectors[index] = FlowVector(
        vec2<f32>(world_x, world_y),
        direction
    );
} 