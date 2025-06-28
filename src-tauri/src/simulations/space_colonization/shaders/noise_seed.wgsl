// GPU-based noise seeding shader for Space Colonization simulation
// Generates noise patterns to influence attractor placement

@group(0) @binding(0)
var<storage, read_write> attractors_data: array<Attractor>;

@group(0) @binding(1)
var<uniform> params: NoiseParams;

struct Attractor {
    position: vec2<f32>,
    is_active: u32,
    influence_count: u32,
}

struct NoiseParams {
    width: u32,
    height: u32,
    seed: u32,
    noise_strength: f32,
    max_attractors: u32,
    attractor_pattern: u32, // 0=Random, 1=Clustered, 2=Grid, 3=Circular, 4=Boundary, 5=Leaf
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

// Generate random number in range [min, max]
fn random_range(min: f32, max: f32) -> f32 {
    // Use a simple approach with random_float
    return min + random_float(hash(u32(min * 1000.0) + u32(max * 1000.0))) * (max - min);
}

// Generate random number [0.0, 1.0]
fn random() -> f32 {
    return random_float(hash(u32(12345u)));
}

// Safe normalize function that handles zero vectors
fn safe_normalize(v: vec2<f32>) -> vec2<f32> {
    let length = length(v);
    if (length < 0.0001) {
        return vec2<f32>(0.0, 1.0); // Default up direction
    }
    return v / length;
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

// Generate random position within bounds
fn random_position(seed: u32, margin: f32) -> vec2<f32> {
    let x = random_float(seed * 2u) * (2.0 - 2.0 * margin) - (1.0 - margin);
    let y = random_float(seed * 3u) * (2.0 - 2.0 * margin) - (1.0 - margin);
    return vec2<f32>(x, y);
}

// Generate clustered position
fn clustered_position(seed: u32, margin: f32) -> vec2<f32> {
    // Create cluster centers
    let cluster_seed = seed / 10u; // Fewer clusters
    let cluster_x = random_float(cluster_seed * 2u) * (2.0 - 2.0 * margin) - (1.0 - margin);
    let cluster_y = random_float(cluster_seed * 3u) * (2.0 - 2.0 * margin) - (1.0 - margin);
    
    // Add noise around cluster center
    let noise_x = (random_float(seed * 4u) - 0.5) * 0.3;
    let noise_y = (random_float(seed * 5u) - 0.5) * 0.3;
    
    return vec2<f32>(cluster_x + noise_x, cluster_y + noise_y);
}

// Generate grid-based position
fn grid_position(seed: u32, margin: f32) -> vec2<f32> {
    let grid_size = 8u; // 8x8 grid
    let cell_x = seed % grid_size;
    let cell_y = (seed / grid_size) % grid_size;
    
    let cell_width = (2.0 - 2.0 * margin) / f32(grid_size);
    let cell_height = (2.0 - 2.0 * margin) / f32(grid_size);
    
    let base_x = f32(cell_x) * cell_width - (1.0 - margin);
    let base_y = f32(cell_y) * cell_height - (1.0 - margin);
    
    // Add jitter within cell
    let jitter_x = (random_float(seed * 6u) - 0.5) * cell_width * 0.8;
    let jitter_y = (random_float(seed * 7u) - 0.5) * cell_height * 0.8;
    
    return vec2<f32>(base_x + jitter_x, base_y + jitter_y);
}

// Generate circular position
fn circular_position(seed: u32, margin: f32) -> vec2<f32> {
    let angle = random_float(seed * 2u) * 2.0 * 3.14159;
    let radius = sqrt(random_float(seed * 3u)) * (1.0 - margin);
    
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

// Generate boundary position
fn boundary_position(seed: u32, margin: f32) -> vec2<f32> {
    let side = seed % 4u; // 0=top, 1=right, 2=bottom, 3=left
    let t = random_float(seed * 2u); // Position along the side
    
    switch (side) {
        case 0u: { // Top
            return vec2<f32>(-1.0 + margin + t * (2.0 - 2.0 * margin), 1.0 - margin);
        }
        case 1u: { // Right
            return vec2<f32>(1.0 - margin, -1.0 + margin + t * (2.0 - 2.0 * margin));
        }
        case 2u: { // Bottom
            return vec2<f32>(-1.0 + margin + t * (2.0 - 2.0 * margin), -1.0 + margin);
        }
        case 3u: { // Left
            return vec2<f32>(-1.0 + margin, -1.0 + margin + t * (2.0 - 2.0 * margin));
        }
        default: {
            return vec2<f32>(0.0, 0.0);
        }
    }
}

// Generate attractor positions for leaf pattern
fn generate_leaf_attractor_position(index: u32, max_attractors: u32, random_seed: u32) -> vec2<f32> {
    // Create attractor distribution optimized for sustained growth
    // This creates organic patterns that encourage long-term tree-like growth
    var x: f32;
    var y: f32;
    
    // Use different patterns based on index to create organic distribution
    let pattern_seed = index % 10u; // More patterns for better distribution
    
    switch(pattern_seed) {
        case 0u: { // Main trunk attractors (central line)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let curve = sin(t * 3.14159) * 0.05; // Very slight curve
            x = curve + (random_float(random_seed + index * 2u) - 0.5) * 0.08;
            y = -0.9 + t * 1.8; // From base to top
        }
        case 1u: { // Primary branch attractors (left side)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let branch_t = random_float(random_seed + index * 3u) * 0.8 + 0.1;
            let angle = (random_float(random_seed + index * 4u) - 0.5) * 1.2;
            x = -0.5 - branch_t * 0.7 + angle * 0.5;
            y = -0.7 + t * 1.7;
        }
        case 2u: { // Primary branch attractors (right side)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let branch_t = random_float(random_seed + index * 3u) * 0.8 + 0.1;
            let angle = (random_float(random_seed + index * 4u) - 0.5) * 1.2;
            x = 0.5 + branch_t * 0.7 + angle * 0.5;
            y = -0.7 + t * 1.7;
        }
        case 3u: { // Secondary branch attractors (left)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let branch_t = random_float(random_seed + index * 5u) * 0.6 + 0.2;
            x = -0.7 - branch_t * 0.6;
            y = -0.5 + t * 1.5;
        }
        case 4u: { // Secondary branch attractors (right)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let branch_t = random_float(random_seed + index * 5u) * 0.6 + 0.2;
            x = 0.7 + branch_t * 0.6;
            y = -0.5 + t * 1.5;
        }
        case 5u: { // Tertiary branch attractors (left)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let branch_t = random_float(random_seed + index * 6u) * 0.4 + 0.3;
            x = -0.9 - branch_t * 0.5;
            y = -0.3 + t * 1.3;
        }
        case 6u: { // Tertiary branch attractors (right)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let branch_t = random_float(random_seed + index * 6u) * 0.4 + 0.3;
            x = 0.9 + branch_t * 0.5;
            y = -0.3 + t * 1.3;
        }
        case 7u: { // Distant attractors for sustained growth (left)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let angle = (random_float(random_seed + index * 7u) - 0.5) * 1.6;
            let radius = random_float(random_seed + index * 8u) * 0.4 + 0.6;
            x = -0.8 - radius * cos(angle);
            y = -0.2 + t * 1.2;
        }
        case 8u: { // Distant attractors for sustained growth (right)
            let t = f32(index / 10u) / f32(max_attractors / 10u);
            let angle = (random_float(random_seed + index * 7u) - 0.5) * 1.6;
            let radius = random_float(random_seed + index * 8u) * 0.4 + 0.6;
            x = 0.8 + radius * cos(angle);
            y = -0.2 + t * 1.2;
        }
        case 9u: { // Random fill for organic shape
            // Create more organic distribution using polar coordinates
            let angle = random_float(random_seed + index * 9u) * 6.28318530718;
            let radius = random_float(random_seed + index * 10u) * 0.9 + 0.1;
            // Use elliptical shape for tree-like form
            x = cos(angle) * radius * 1.0;
            y = sin(angle) * radius * 1.1 + 0.1;
        }
        default: {
            x = (random_float(random_seed + index * 11u) - 0.5) * 1.8;
            y = (random_float(random_seed + index * 12u) - 0.5) * 1.8;
        }
    }
    
    // Add organic randomness to make it more natural
    x += (random_float(random_seed + index * 13u) - 0.5) * 0.08;
    y += (random_float(random_seed + index * 14u) - 0.5) * 0.08;
    
    // Ensure bounds with some margin
    x = clamp(x, -0.95, 0.95);
    y = clamp(y, -0.95, 0.95);
    
    return vec2<f32>(x, y);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    // Check bounds
    if (index >= params.max_attractors) {
        return;
    }
    
    let margin = 0.1; // 10% margin from edges
    var position: vec2<f32>;
    
    // Generate position based on pattern
    switch (params.attractor_pattern) {
        case 0u: { // Random
            position = random_position(params.seed + index, margin);
        }
        case 1u: { // Clustered
            position = clustered_position(params.seed + index, margin);
        }
        case 2u: { // Grid
            position = grid_position(params.seed + index, margin);
        }
        case 3u: { // Circular
            position = circular_position(params.seed + index, margin);
        }
        case 4u: { // Boundary
            position = boundary_position(params.seed + index, margin);
        }
        case 5u: { // Leaf
            position = generate_leaf_attractor_position(index, params.max_attractors, params.seed + index);
        }
        default: {
            position = random_position(params.seed + index, margin);
        }
    }
    
    // Apply noise-based variation
    let noise_value = fbm_noise(u32(position.x * 100.0), u32(position.y * 100.0), params.seed + index);
    let noise_offset = (noise_value - 0.5) * params.noise_strength * 0.1;
    position += vec2<f32>(noise_offset, noise_offset);
    
    // Clamp to bounds
    position.x = clamp(position.x, -1.0 + margin, 1.0 - margin);
    position.y = clamp(position.y, -1.0 + margin, 1.0 - margin);
    
    // Set attractor data
    attractors_data[index].position = position;
    attractors_data[index].is_active = 1u;
    attractors_data[index].influence_count = 0u;
} 