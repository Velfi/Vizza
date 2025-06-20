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

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let local_index = id.x;
    if (local_index >= init_params.spawn_count) {
        return;
    }
    
    // Calculate actual particle index
    let index = init_params.start_index + local_index;
    
    // Generate unique seed for this particle
    let seed = init_params.random_seed + index;
    
    // Generate random position
    let pos_x = random_f32(seed * 2u + 1u) * init_params.width;
    let pos_y = random_f32(seed * 2u + 2u) * init_params.height;
    
    // Assign species evenly
    let species = index % init_params.species_count;
    
    // Initialize particle
    particles[index].position = vec2<f32>(pos_x, pos_y);
    particles[index].velocity = vec2<f32>(0.0, 0.0);
    particles[index].species = species;
    particles[index]._pad = 0u;
}