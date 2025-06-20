// Force matrix update compute shader
// Updates individual force matrix elements without full CPU-to-GPU transfer

struct ForceUpdateParams {
    species_a: u32,
    species_b: u32,
    new_force: f32,
    species_count: u32,
}

@group(0) @binding(0) var<storage, read_write> force_matrix: array<f32>;
@group(0) @binding(1) var<uniform> update_params: ForceUpdateParams;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // Only one thread needed for a single force matrix update
    if (id.x != 0u || id.y != 0u || id.z != 0u) {
        return;
    }
    
    // Calculate the index in the flattened force matrix
    let index = update_params.species_a * update_params.species_count + update_params.species_b;
    
    // Update the force matrix element
    force_matrix[index] = update_params.new_force;
}