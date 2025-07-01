// Particle initialization compute shader

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    species: u32,
    _pad: u32,
}

struct InitParams {
    start_index: u32,
    spawn_count: u32,
    species_count: u32,
    width: f32,
    height: f32,
    random_seed: u32,
    position_generator: u32, // 0=Random, 1=Center, 2=UniformCircle, etc.
    type_generator: u32,     // 0=Random, 1=Randomize10Percent, etc.
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> init_params: InitParams;

// Simple pseudo-random number generator
fn hash(seed: u32) -> u32 {
    var x = seed;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return x;
}

fn random_f32(seed: u32) -> f32 {
    return f32(hash(seed)) / f32(0xffffffffu);
}

// Position generation functions - now using [-1,1] coordinate system
fn generate_random_position(seed: u32) -> vec2<f32> {
    let x = random_f32(seed * 2u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let y = random_f32(seed * 3u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    return vec2<f32>(x, y); // [-1,1] x [-1,1]
}

fn generate_center_position(seed: u32) -> vec2<f32> {
    let x = random_f32(seed * 2u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let y = random_f32(seed * 3u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let scale = 0.3; // Scale around center (0,0)
    return vec2<f32>(x * scale, y * scale);
}

fn generate_uniform_circle_position(seed: u32) -> vec2<f32> {
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = sqrt(random_f32(seed * 3u)) * 0.8; // Scale for [-1,1] space (radius up to 0.8)
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_centered_circle_position(seed: u32) -> vec2<f32> {
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = random_f32(seed * 3u) * 0.8; // Scale for [-1,1] space
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_ring_position(seed: u32) -> vec2<f32> {
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = 0.35 + 0.01 * (random_f32(seed * 3u) - 0.5) * 2.0; // Ring in [-1,1] space
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_rainbow_ring_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let angle = (0.3 * (random_f32(seed * 2u) - 0.5) * 2.0 + f32(type_id)) / f32(n_types) * 2.0 * 3.14159;
    let radius = 0.35 + 0.01 * (random_f32(seed * 3u) - 0.5) * 2.0; // Ring in [-1,1] space
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_color_battle_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let center_angle = f32(type_id) / f32(n_types) * 2.0 * 3.14159;
    let center_radius = 0.25; // Scale for [-1,1] space
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = random_f32(seed * 3u) * 0.05; // Scale for [-1,1] space
    return vec2<f32>(
        center_radius * cos(center_angle) + cos(angle) * radius,
        center_radius * sin(center_angle) + sin(angle) * radius
    );
}

fn generate_color_wheel_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let center_angle = f32(type_id) / f32(n_types) * 2.0 * 3.14159;
    let center_radius = 0.15; // Scale for [-1,1] space
    let individual_radius = 0.1; // Scale for [-1,1] space
    return vec2<f32>(
        center_radius * cos(center_angle) + (random_f32(seed * 2u) - 0.5) * 2.0 * individual_radius,
        center_radius * sin(center_angle) + (random_f32(seed * 3u) - 0.5) * 2.0 * individual_radius
    );
}

fn generate_line_position(seed: u32) -> vec2<f32> {
    let x = random_f32(seed * 2u) * 2.0 - 1.0; // Full width in [-1,1]
    let y = (random_f32(seed * 3u) - 0.5) * 0.3; // Center around 0 with small spread
    return vec2<f32>(x, y);
}

fn generate_spiral_position(seed: u32) -> vec2<f32> {
    let max_rotations = 2.0;
    let f = random_f32(seed * 2u);
    let angle = max_rotations * 2.0 * 3.14159 * f;
    let spread = 0.25 * min(f, 0.2); // Spiral spread
    let radius = 0.45 * f + spread * (random_f32(seed * 3u) - 0.5) * 2.0; // Scale for [-1,1] space
    return vec2<f32>(radius * cos(angle), radius * sin(angle));
}

fn generate_rainbow_spiral_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let max_rotations = 2.0;
    let type_spread = 0.3 / f32(n_types);
    var f = (f32(type_id + 1u) / f32(n_types + 2u) + type_spread * (random_f32(seed * 2u) - 0.5) * 2.0);
    f = clamp(f, 0.0, 1.0);
    let angle = max_rotations * 2.0 * 3.14159 * f;
    let spread = 0.25 * min(f, 0.2); // Spiral spread
    let radius = 0.45 * f + spread * (random_f32(seed * 3u) - 0.5) * 2.0; // Scale for [-1,1] space
    return vec2<f32>(radius * cos(angle), radius * sin(angle));
}

// Type generation functions
fn generate_radial_type(position: vec2<f32>, n_types: u32) -> u32 {
    let distance = length(position); // Distance from center (0,0)
    let normalized_distance = clamp(distance / 1.414, 0.0, 1.0); // Normalize to [0,1], max distance is ~√2
    return u32(normalized_distance * f32(n_types)) % n_types;
}

fn generate_polar_type(position: vec2<f32>, n_types: u32) -> u32 {
    let angle = atan2(position.y, position.x); // Angle from center, range [-π, π]
    let normalized_angle = (angle + 3.14159) / (2.0 * 3.14159); // Convert to [0,1]
    return u32(normalized_angle * f32(n_types)) % n_types;
}

fn generate_stripes_h_type(position: vec2<f32>, n_types: u32) -> u32 {
    let normalized_y = (position.y + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
    return u32(normalized_y * f32(n_types)) % n_types;
}

fn generate_stripes_v_type(position: vec2<f32>, n_types: u32) -> u32 {
    let normalized_x = (position.x + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
    return u32(normalized_x * f32(n_types)) % n_types;
}

fn generate_random_type(seed: u32, n_types: u32) -> u32 {
    return u32(random_f32(seed * 4u) * f32(n_types)) % n_types;
}

fn generate_line_h_type(position: vec2<f32>, n_types: u32) -> u32 {
    // Horizontal line in the middle, distribute other areas across remaining types
    if (abs(position.y) < 0.1) {
        return 0u; // Center line gets type 0
    } else {
        // Distribute upper and lower areas across remaining types
        let normalized_y = (position.y + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
        let region_type = u32(normalized_y * f32(n_types - 1u)) + 1u; // Use types 1 to n_types-1
        return region_type % n_types;
    }
}

fn generate_line_v_type(position: vec2<f32>, n_types: u32) -> u32 {
    // Vertical line in the middle, distribute other areas across remaining types
    if (abs(position.x) < 0.1) {
        return 0u; // Center line gets type 0
    } else {
        // Distribute left and right areas across remaining types
        let normalized_x = (position.x + 1.0) * 0.5; // Convert from [-1,1] to [0,1]
        let region_type = u32(normalized_x * f32(n_types - 1u)) + 1u; // Use types 1 to n_types-1
        return region_type % n_types;
    }
}

fn generate_spiral_type(position: vec2<f32>, n_types: u32) -> u32 {
    let distance = length(position);
    let angle = atan2(position.y, position.x);
    let spiral_value = distance + angle * 0.159; // 0.159 ≈ 1/(2π) for normalization
    let normalized_spiral = fract(spiral_value * 2.0); // Create repeating spiral pattern
    return u32(normalized_spiral * f32(n_types)) % n_types;
}

fn generate_dithered_type(position: vec2<f32>, seed: u32, n_types: u32) -> u32 {
    // Create bold color bands that blend at the edges (classic web JPEG style)
    
    let distance = length(position);
    let angle = atan2(position.y, position.x);
    
    // Create bold radial bands
    let band_value = distance * f32(n_types);
    let base_band = u32(floor(band_value));
    
    // Add blending at edges using noise
    let noise_seed = u32((position.x + 1.0) * 1000.0) + u32((position.y + 1.0) * 1000.0) + seed;
    let noise = random_f32(noise_seed);
    
    // Calculate how close we are to the next band boundary
    let band_fraction = fract(band_value);
    
    // Blend between bands at edges (within 20% of band boundaries)
    if (band_fraction > 0.8 && noise > 0.5) {
        return (base_band + 1u) % n_types;
    } else if (band_fraction < 0.2 && noise < 0.5) {
        return (base_band + n_types - 1u) % n_types;
    }
    
    return base_band % n_types;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= init_params.spawn_count) {
        return;
    }
    
    let particle_index = init_params.start_index + index;
    let seed = init_params.random_seed + index;
    
    // Generate position based on generator type
    var position: vec2<f32>;
    switch (init_params.position_generator) {
        case 0u: { // Random
            position = generate_random_position(seed);
        }
        case 1u: { // Center
            position = generate_center_position(seed);
        }
        case 2u: { // UniformCircle
            position = generate_uniform_circle_position(seed);
        }
        case 3u: { // CenteredCircle
            position = generate_centered_circle_position(seed);
        }
        case 4u: { // Ring
            position = generate_ring_position(seed);
        }
        case 5u: { // RainbowRing
            let type_id = index % init_params.species_count;
            position = generate_rainbow_ring_position(seed, type_id, init_params.species_count);
        }
        case 6u: { // ColorBattle
            let type_id = index % init_params.species_count;
            position = generate_color_battle_position(seed, type_id, init_params.species_count);
        }
        case 7u: { // ColorWheel
            let type_id = index % init_params.species_count;
            position = generate_color_wheel_position(seed, type_id, init_params.species_count);
        }
        case 8u: { // Line
            position = generate_line_position(seed);
        }
        case 9u: { // Spiral
            position = generate_spiral_position(seed);
        }
        case 10u: { // RainbowSpiral
            let type_id = index % init_params.species_count;
            position = generate_rainbow_spiral_position(seed, type_id, init_params.species_count);
        }
        default: {
            position = generate_random_position(seed);
        }
    }
    
    // Generate initial type based on generator
    var initial_type: u32;
    switch (init_params.type_generator) {
        case 0u: { // Radial
            initial_type = generate_radial_type(position, init_params.species_count);
        }
        case 1u: { // Polar
            initial_type = generate_polar_type(position, init_params.species_count);
        }
        case 2u: { // StripesH
            initial_type = generate_stripes_h_type(position, init_params.species_count);
        }
        case 3u: { // StripesV
            initial_type = generate_stripes_v_type(position, init_params.species_count);
        }
        case 4u: { // Random
            initial_type = generate_random_type(seed, init_params.species_count);
        }
        case 5u: { // LineH
            initial_type = generate_line_h_type(position, init_params.species_count);
        }
        case 6u: { // LineV
            initial_type = generate_line_v_type(position, init_params.species_count);
        }
        case 7u: { // Spiral
            initial_type = generate_spiral_type(position, init_params.species_count);
        }
        case 8u: { // Dithered
            initial_type = generate_dithered_type(position, seed, init_params.species_count);
        }
        default: {
            initial_type = generate_random_type(seed, init_params.species_count);
        }
    }
    
    // Ensure type is within bounds
    initial_type = initial_type % init_params.species_count;
    
    // Initialize particle
    particles[particle_index] = Particle(
        position,
        vec2<f32>(0.0, 0.0), // Zero initial velocity
        initial_type,
        0u // _pad field
    );
}