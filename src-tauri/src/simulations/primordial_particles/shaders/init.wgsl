// Primordial Particles Initialization Shader
// Randomly distributes particles across the world with random headings

const PI: f32 = 3.14159265359;

struct Particle {
    position: vec2<f32>,
    previous_position: vec2<f32>,
    heading: f32,
    grabbed: u32,
    _pad0: u32,
    _pad1: u32,
}

struct InitParams {
    start_index: u32,
    spawn_count: u32,
    width: f32,
    height: f32,
    
    random_seed: u32,
    position_generator: u32, // 0=Random, 1=Center, 2=UniformCircle, etc.
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> init_params: InitParams;

// Simple pseudo-random number generator
fn hash(seed: u32) -> u32 {
    var x = seed;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return x;
}

fn random(seed: u32) -> f32 {
    return f32(hash(seed)) / f32(0xffffffffu);
}

// Generate random position within world bounds [-1,1] coordinate space
fn random_position(index: u32) -> vec2<f32> {
    let seed1 = init_params.random_seed + index * 2654435761u;
    let seed2 = init_params.random_seed + index * 1013904223u;
    
    let x = random(seed1) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let y = random(seed2) * 2.0 - 1.0; // [0,1] -> [-1,1]
    
    return vec2<f32>(x, y);
}

// Generate random heading
fn random_heading(index: u32) -> f32 {
    let seed = init_params.random_seed + index * 1664525u;
    return random(seed) * 2.0 * PI;
}

// Position generation functions - using [-1,1] coordinate system
fn generate_random_position(seed: u32) -> vec2<f32> {
    let x = random(seed * 2u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let y = random(seed * 3u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    return vec2<f32>(x, y); // [-1,1] x [-1,1]
}

fn generate_center_position(seed: u32) -> vec2<f32> {
    let x = random(seed * 2u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let y = random(seed * 3u) * 2.0 - 1.0; // [0,1] -> [-1,1]
    let scale = 0.3; // Scale around center (0,0)
    return vec2<f32>(x * scale, y * scale);
}

fn generate_uniform_circle_position(seed: u32) -> vec2<f32> {
    let angle = random(seed * 2u) * 2.0 * PI;
    let radius = sqrt(random(seed * 3u)) * 0.8; // Scale for [-1,1] space (radius up to 0.8)
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_centered_circle_position(seed: u32) -> vec2<f32> {
    let angle = random(seed * 2u) * 2.0 * PI;
    let radius = random(seed * 3u) * 0.8; // Scale for [-1,1] space
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_ring_position(seed: u32) -> vec2<f32> {
    let angle = random(seed * 2u) * 2.0 * PI;
    let radius = 0.35 + 0.01 * (random(seed * 3u) - 0.5) * 2.0; // Ring in [-1,1] space
    return vec2<f32>(cos(angle) * radius, sin(angle) * radius);
}

fn generate_line_position(seed: u32) -> vec2<f32> {
    let x = random(seed * 2u) * 2.0 - 1.0; // Full width in [-1,1]
    let y = (random(seed * 3u) - 0.5) * 0.3; // Center around 0 with small spread
    return vec2<f32>(x, y);
}

fn generate_spiral_position(seed: u32) -> vec2<f32> {
    let max_rotations = 2.0;
    let f = random(seed * 2u);
    let angle = max_rotations * 2.0 * PI * f;
    let spread = 0.25 * min(f, 0.2); // Spiral spread
    let radius = 0.45 * f + spread * (random(seed * 3u) - 0.5) * 2.0; // Scale for [-1,1] space
    return vec2<f32>(radius * cos(angle), radius * sin(angle));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let local_index = global_id.x;
    let global_index = init_params.start_index + local_index;
    
    if (local_index >= init_params.spawn_count) {
        return;
    }
    
    let seed = init_params.random_seed + local_index;
    
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
        case 5u: { // Line
            position = generate_line_position(seed);
        }
        case 6u: { // Spiral
            position = generate_spiral_position(seed);
        }
        default: {
            position = generate_random_position(seed);
        }
    }
    
    // Initialize particle with generated position and random heading
    particles[global_index].position = position;
    particles[global_index].previous_position = vec2<f32>(0.0, 0.0);
    particles[global_index].heading = random_heading(global_index);
    particles[global_index].grabbed = 0u;
    particles[global_index]._pad0 = 0u;
    particles[global_index]._pad1 = 0u;
}


