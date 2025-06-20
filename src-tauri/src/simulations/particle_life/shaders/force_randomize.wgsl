// Force matrix randomization compute shader
// Generates random force values entirely on GPU

struct RandomizeParams {
    species_count: u32,
    random_seed: u32,
    min_force: f32,
    max_force: f32,
}

@group(0) @binding(0) var<storage, read_write> force_matrix: array<f32>;
@group(0) @binding(1) var<uniform> randomize_params: RandomizeParams;

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

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let species_a = id.x;
    let species_b = id.y;
    
    // Only process valid matrix elements
    if (species_a >= randomize_params.species_count || species_b >= randomize_params.species_count) {
        return;
    }
    
    // Calculate the index in the flattened force matrix
    let index = species_a * randomize_params.species_count + species_b;
    
    // Generate unique seed for this matrix element
    let seed = randomize_params.random_seed + index;
    
    // Generate random force value in the specified range
    let random_val = random_f32(seed);
    let force_range = randomize_params.max_force - randomize_params.min_force;
    let new_force = randomize_params.min_force + random_val * force_range;
    
    // Update the force matrix element
    force_matrix[index] = new_force;
}