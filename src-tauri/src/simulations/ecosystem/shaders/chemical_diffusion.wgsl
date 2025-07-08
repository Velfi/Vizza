// Chemical diffusion compute shader for ecosystem simulation
// Handles chemical diffusion and decay in the environment

struct SimParams {
    chemical_resolution: u32,
    chemical_types: u32,
    chemical_diffusion_rate: f32,
    chemical_decay_rate: f32,
    dt: f32,
}

@group(0) @binding(0) var<storage, read_write> chemical_field: array<f32>;
@group(0) @binding(1) var<storage, read_write> chemical_field_temp: array<f32>;
@group(0) @binding(2) var<uniform> params: SimParams;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= params.chemical_resolution || y >= params.chemical_resolution) {
        return;
    }
    
    let resolution = params.chemical_resolution;
    
    // Process each chemical type
    for (var chemical_type = 0u; chemical_type < params.chemical_types; chemical_type++) {
        let center_index = (y * resolution + x) * params.chemical_types + chemical_type;
        var center_value = chemical_field[center_index];
        
        // Apply decay
        center_value *= (1.0 - params.chemical_decay_rate * params.dt);
        
        // Diffusion using 9-point stencil
        var diffused_value = center_value;
        var neighbor_sum = 0.0;
        var neighbor_count = 0.0;
        
        // Sample neighbors
        for (var dy = -1i; dy <= 1i; dy++) {
            for (var dx = -1i; dx <= 1i; dx++) {
                if (dx == 0 && dy == 0) {
                    continue;
                }
                
                let nx = i32(x) + dx;
                let ny = i32(y) + dy;
                
                // Handle boundaries (wrap around)
                let wrapped_x = u32((nx + i32(resolution)) % i32(resolution));
                let wrapped_y = u32((ny + i32(resolution)) % i32(resolution));
                
                let neighbor_index = (wrapped_y * resolution + wrapped_x) * params.chemical_types + chemical_type;
                neighbor_sum += chemical_field[neighbor_index];
                neighbor_count += 1.0;
            }
        }
        
        // Apply diffusion
        let diffusion_amount = (neighbor_sum / neighbor_count - center_value) * params.chemical_diffusion_rate * params.dt;
        diffused_value += diffusion_amount;
        
        // Ensure non-negative
        diffused_value = max(diffused_value, 0.0);
        
        // Write to temporary buffer
        chemical_field_temp[center_index] = diffused_value;
    }
} 