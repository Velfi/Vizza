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

// Position generation functions
fn generate_random_position(seed: u32) -> vec2<f32> {
    let x = random_f32(seed * 2u);
    let y = random_f32(seed * 3u);
    return vec2<f32>((x - 0.5) * 4.0, (y - 0.5) * 4.0); // -2.0 to 2.0
}

fn generate_center_position(seed: u32) -> vec2<f32> {
    let x = random_f32(seed * 2u);
    let y = random_f32(seed * 3u);
    let scale = 1.2;
    return vec2<f32>((x - 0.5) * 2.0 * scale, (y - 0.5) * 2.0 * scale);
}

fn generate_uniform_circle_position(seed: u32) -> vec2<f32> {
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = sqrt(random_f32(seed * 3u)) * 2.0;
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_centered_circle_position(seed: u32) -> vec2<f32> {
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = random_f32(seed * 3u) * 2.0;
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_ring_position(seed: u32) -> vec2<f32> {
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = 0.7 + 0.02 * (random_f32(seed * 3u) - 0.5) * 2.0;
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_rainbow_ring_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let angle = (0.3 * (random_f32(seed * 2u) - 0.5) * 2.0 + f32(type_id)) / f32(n_types) * 2.0 * 3.14159;
    let radius = 0.7 + 0.02 * (random_f32(seed * 3u) - 0.5) * 2.0;
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_color_battle_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let center_angle = f32(type_id) / f32(n_types) * 2.0 * 3.14159;
    let center_radius = 0.5;
    let angle = random_f32(seed * 2u) * 2.0 * 3.14159;
    let radius = random_f32(seed * 3u) * 0.1;
    return vec2<f32>(
        center_radius * cos(center_angle) + cos(angle) * radius,
        center_radius * sin(center_angle) + sin(angle) * radius
    );
}

fn generate_color_wheel_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let center_angle = f32(type_id) / f32(n_types) * 2.0 * 3.14159;
    let center_radius = 0.3;
    let individual_radius = 0.2;
    return vec2<f32>(
        center_radius * cos(center_angle) + (random_f32(seed * 2u) - 0.5) * 2.0 * individual_radius,
        center_radius * sin(center_angle) + (random_f32(seed * 3u) - 0.5) * 2.0 * individual_radius
    );
}

fn generate_line_position(seed: u32) -> vec2<f32> {
    let x = (random_f32(seed * 2u) - 0.5) * 4.0;
    let y = (random_f32(seed * 3u) - 0.5) * 1.2;
    return vec2<f32>(x, y);
}

fn generate_spiral_position(seed: u32) -> vec2<f32> {
    let max_rotations = 2.0;
    let f = random_f32(seed * 2u);
    let angle = max_rotations * 2.0 * 3.14159 * f;
    let spread = 0.5 * min(f, 0.2);
    let radius = 0.9 * f + spread * (random_f32(seed * 3u) - 0.5) * 2.0 * spread;
    return vec2<f32>(radius * cos(angle), radius * sin(angle));
}

fn generate_rainbow_spiral_position(seed: u32, type_id: u32, n_types: u32) -> vec2<f32> {
    let max_rotations = 2.0;
    let type_spread = 0.3 / f32(n_types);
    var f = (f32(type_id + 1u) / f32(n_types + 2u) + type_spread * (random_f32(seed * 2u) - 0.5) * 2.0);
    f = clamp(f, 0.0, 1.0);
    let angle = max_rotations * 2.0 * 3.14159 * f;
    let spread = 0.5 * min(f, 0.2);
    let radius = 0.9 * f + spread * (random_f32(seed * 3u) - 0.5) * 2.0 * spread;
    return vec2<f32>(radius * cos(angle), radius * sin(angle));
}

// Type generation functions
fn generate_random_type(seed: u32, n_types: u32) -> u32 {
    return u32(random_f32(seed * 4u) * f32(n_types));
}

fn generate_randomize_10_percent_type(seed: u32, current_type: u32, n_types: u32) -> u32 {
    if (random_f32(seed * 4u) < 0.1) {
        return generate_random_type(seed * 5u, n_types);
    }
    return current_type;
}

fn generate_slices_type(position: vec2<f32>, n_types: u32) -> u32 {
    let normalized_x = (position.x + 2.0) / 4.0; // Convert -2..2 to 0..1
    return u32(normalized_x * f32(n_types));
}

fn generate_onion_type(position: vec2<f32>, n_types: u32) -> u32 {
    let distance = length(position) * 2.0; // Scale to 0..1 range
    return u32(distance * f32(n_types));
}

fn generate_rotate_type(current_type: u32, n_types: u32) -> u32 {
    return (current_type + 1u) % n_types;
}

fn generate_flip_type(current_type: u32, n_types: u32) -> u32 {
    return n_types - 1u - current_type;
}

fn generate_more_of_first_type(seed: u32, n_types: u32) -> u32 {
    let value = random_f32(seed * 4u) * random_f32(seed * 5u);
    return u32(value * f32(n_types));
}

fn generate_kill_still_type(velocity: vec2<f32>, current_type: u32, n_types: u32) -> u32 {
    if (length(velocity) < 0.01) {
        return n_types - 1u;
    }
    return current_type;
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
        case 0u: { // Random
            initial_type = generate_random_type(seed, init_params.species_count);
        }
        case 1u: { // Randomize10Percent
            let base_type = index % init_params.species_count;
            initial_type = generate_randomize_10_percent_type(seed, base_type, init_params.species_count);
        }
        case 2u: { // Slices
            initial_type = generate_slices_type(position, init_params.species_count);
        }
        case 3u: { // Onion
            initial_type = generate_onion_type(position, init_params.species_count);
        }
        case 4u: { // Rotate
            let base_type = index % init_params.species_count;
            initial_type = generate_rotate_type(base_type, init_params.species_count);
        }
        case 5u: { // Flip
            let base_type = index % init_params.species_count;
            initial_type = generate_flip_type(base_type, init_params.species_count);
        }
        case 6u: { // MoreOfFirst
            initial_type = generate_more_of_first_type(seed, init_params.species_count);
        }
        case 7u: { // KillStill
            let base_type = index % init_params.species_count;
            initial_type = generate_kill_still_type(vec2<f32>(0.0, 0.0), base_type, init_params.species_count);
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