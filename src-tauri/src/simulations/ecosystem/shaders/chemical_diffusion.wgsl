// Enhanced chemical diffusion compute shader for ecosystem simulation
// Handles chemical diffusion, decay, and reactions for 6 chemical types

struct SimParams {
    chemical_resolution: u32,
    chemical_types: u32, // Now 6 types: O2, CO2, N, pheromones, toxins, attractants
    chemical_diffusion_rate: f32,
    chemical_decay_rate: f32,
    dt: f32,
    enable_fluid_dynamics: u32,
}

@group(0) @binding(0) var<storage, read_write> chemical_field: array<f32>;
@group(0) @binding(1) var<storage, read_write> chemical_field_temp: array<f32>;
@group(0) @binding(2) var<uniform> params: SimParams;
@group(0) @binding(3) var<storage, read> velocity_field: array<vec2<f32>>;

// Chemical type constants
const OXYGEN = 0u;
const CARBON_DIOXIDE = 1u;
const NITROGEN_COMPOUNDS = 2u;
const PHEROMONES = 3u;
const TOXINS = 4u;
const ATTRACTANTS = 5u;

// Get diffusion rate for specific chemical type
fn get_diffusion_rate(chemical_type: u32) -> f32 {
    switch (chemical_type) {
        case 0u: { return params.chemical_diffusion_rate * 1.2; } // Oxygen diffuses faster
        case 1u: { return params.chemical_diffusion_rate * 1.1; } // CO2 diffuses moderately
        case 2u: { return params.chemical_diffusion_rate * 0.8; } // N compounds diffuse slower
        case 3u: { return params.chemical_diffusion_rate * 0.6; } // Pheromones diffuse slowly
        case 4u: { return params.chemical_diffusion_rate * 0.7; } // Toxins diffuse moderately
        case 5u: { return params.chemical_diffusion_rate * 1.0; } // Attractants diffuse normally
        default: { return params.chemical_diffusion_rate; }
    }
}

// Get decay rate for specific chemical type
fn get_decay_rate(chemical_type: u32) -> f32 {
    switch (chemical_type) {
        case 0u: { return params.chemical_decay_rate * 0.5; } // Oxygen is stable
        case 1u: { return params.chemical_decay_rate * 0.7; } // CO2 is moderately stable
        case 2u: { return params.chemical_decay_rate * 0.8; } // N compounds are stable
        case 3u: { return params.chemical_decay_rate * 2.0; } // Pheromones decay quickly
        case 4u: { return params.chemical_decay_rate * 1.5; } // Toxins decay moderately
        case 5u: { return params.chemical_decay_rate * 1.8; } // Attractants decay quickly
        default: { return params.chemical_decay_rate; }
    }
}

// Realistic chemical reactions based on natural ecosystem processes
fn apply_chemical_reactions(chemical_values: array<f32, 6>) -> array<f32, 6> {
    var result = chemical_values;
    
    // Define realistic concentration limits and equilibrium constants
    let max_safe_concentration = 8.0;  // Above this becomes toxic
    let equilibrium_constant = 0.5;   // Natural equilibrium point
    
    // 1. OXYGEN-CO2 EQUILIBRIUM (Photosynthesis/Respiration)
    // In nature: CO2 + H2O + light ⇌ O2 + glucose
    let o2_co2_equilibrium = equilibrium_constant;
    let current_ratio = result[OXYGEN] / (result[CARBON_DIOXIDE] + 0.1);
    
    if (current_ratio < o2_co2_equilibrium) {
        // Favor oxygen production (photosynthesis-like)
        let conversion = min(result[CARBON_DIOXIDE] * 0.02 * params.dt, 0.1);
        result[CARBON_DIOXIDE] -= conversion;
        result[OXYGEN] += conversion * 0.8; // Not 1:1 due to other products
    } else {
        // Favor CO2 production (respiration-like)
        let conversion = min(result[OXYGEN] * 0.01 * params.dt, 0.1);
        result[OXYGEN] -= conversion;
        result[CARBON_DIOXIDE] += conversion * 0.6;
    }
    
    // 2. NITROGEN CYCLE (Michaelis-Menten kinetics)
    // Nitrification: NH3 + O2 → NO2 + NO3 (enzyme-limited)
    let max_nitrification_rate = 0.05;
    let km_oxygen = 1.0; // Michaelis constant for oxygen
    let km_nitrogen = 0.8; // Michaelis constant for nitrogen
    
    let nitrification_rate = (max_nitrification_rate * result[OXYGEN] * result[NITROGEN_COMPOUNDS]) /
                            ((km_oxygen + result[OXYGEN]) * (km_nitrogen + result[NITROGEN_COMPOUNDS]));
    
    let nitrification = nitrification_rate * params.dt;
    result[NITROGEN_COMPOUNDS] -= nitrification;
    result[OXYGEN] -= nitrification * 2.0; // Stoichiometric ratio
    
    // 3. PHEROMONE DEGRADATION (Enzymatic breakdown)
    // Pheromones naturally degrade faster at high concentrations due to enzyme saturation
    let pheromone_degradation = result[PHEROMONES] * result[PHEROMONES] * 0.03 * params.dt;
    result[PHEROMONES] -= min(pheromone_degradation, result[PHEROMONES] * 0.5);
    
    // 4. TOXIN NEUTRALIZATION (Competitive inhibition)
    // High toxin concentrations inhibit their own production and promote neutralization
    let toxin_self_inhibition = result[TOXINS] * result[TOXINS] * 0.08 * params.dt;
    result[TOXINS] -= toxin_self_inhibition;
    
    // Oxygen helps neutralize toxins (oxidative breakdown)
    let detox_rate = min(result[TOXINS], result[OXYGEN]) * 0.04 * params.dt;
    result[TOXINS] -= detox_rate;
    result[OXYGEN] -= detox_rate * 0.3; // Oxygen partially consumed
    
    // 5. ATTRACTANT SATURATION
    // Attractants become less effective at high concentrations (receptor saturation)
    if (result[ATTRACTANTS] > 2.0) {
        let saturation_decay = (result[ATTRACTANTS] - 2.0) * 0.1 * params.dt;
        result[ATTRACTANTS] -= saturation_decay;
    }
    
    // 6. CROSS-INHIBITION (Natural competition for resources)
    // High concentrations of one chemical can inhibit production of others
    let total_chemical_load = result[OXYGEN] + result[CARBON_DIOXIDE] + result[NITROGEN_COMPOUNDS] + 
                              result[PHEROMONES] + result[TOXINS] + result[ATTRACTANTS];
    
    if (total_chemical_load > 10.0) {
        let inhibition_factor = 1.0 - min((total_chemical_load - 10.0) * 0.02, 0.3);
        result[OXYGEN] *= inhibition_factor;
        result[CARBON_DIOXIDE] *= inhibition_factor;
        result[NITROGEN_COMPOUNDS] *= inhibition_factor;
        result[PHEROMONES] *= inhibition_factor;
        result[TOXINS] *= inhibition_factor;
        result[ATTRACTANTS] *= inhibition_factor;
    }
    
    // 7. ENVIRONMENTAL BUFFERING
    // Gradual drift toward natural baseline concentrations
    let natural_baselines = array<f32, 6>(2.0, 1.5, 0.8, 0.1, 0.05, 0.2);
    let buffering_strength = 0.005 * params.dt;
    
    let drift_oxygen = (natural_baselines[0] - result[OXYGEN]) * buffering_strength;
    let drift_co2 = (natural_baselines[1] - result[CARBON_DIOXIDE]) * buffering_strength;
    let drift_nitrogen = (natural_baselines[2] - result[NITROGEN_COMPOUNDS]) * buffering_strength;
    let drift_pheromones = (natural_baselines[3] - result[PHEROMONES]) * buffering_strength;
    let drift_toxins = (natural_baselines[4] - result[TOXINS]) * buffering_strength;
    let drift_attractants = (natural_baselines[5] - result[ATTRACTANTS]) * buffering_strength;
    
    result[OXYGEN] += drift_oxygen;
    result[CARBON_DIOXIDE] += drift_co2;
    result[NITROGEN_COMPOUNDS] += drift_nitrogen;
    result[PHEROMONES] += drift_pheromones;
    result[TOXINS] += drift_toxins;
    result[ATTRACTANTS] += drift_attractants;
    
    // 8. SAFETY LIMITS (Prevent ecological collapse)
    // Hard caps to prevent toxic accumulation or complete depletion
    result[OXYGEN] = clamp(result[OXYGEN], 0.1, max_safe_concentration);
    result[CARBON_DIOXIDE] = clamp(result[CARBON_DIOXIDE], 0.1, max_safe_concentration);
    result[NITROGEN_COMPOUNDS] = clamp(result[NITROGEN_COMPOUNDS], 0.0, max_safe_concentration * 0.6);
    result[PHEROMONES] = clamp(result[PHEROMONES], 0.0, 3.0); // Lower limit for signaling chemicals
    result[TOXINS] = clamp(result[TOXINS], 0.0, 2.0); // Strict limit on toxins
    result[ATTRACTANTS] = clamp(result[ATTRACTANTS], 0.0, 4.0);
    
    return result;
}

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
        
        // Apply type-specific decay
        center_value *= (1.0 - get_decay_rate(chemical_type) * params.dt);
        
        // Diffusion using 9-point stencil with type-specific rates
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
        
        // Apply type-specific diffusion
        let diffusion_amount = (neighbor_sum / neighbor_count - center_value) * get_diffusion_rate(chemical_type) * params.dt;
        diffused_value += diffusion_amount;
        
        // Apply fluid advection if enabled
        if (params.enable_fluid_dynamics != 0u) {
            let velocity = velocity_field[y * resolution + x];
            
            // Calculate advection using upwind scheme
            let advection_strength = 0.5; // Adjustable parameter
            var advected_value = diffused_value;
            
            // X-direction advection
            if (velocity.x > 0.0) {
                let left_x = select(x - 1u, resolution - 1u, x == 0u);
                let left_index = (y * resolution + left_x) * params.chemical_types + chemical_type;
                advected_value -= velocity.x * advection_strength * params.dt * (diffused_value - chemical_field[left_index]);
            } else if (velocity.x < 0.0) {
                let right_x = select(x + 1u, 0u, x >= resolution - 1u);
                let right_index = (y * resolution + right_x) * params.chemical_types + chemical_type;
                advected_value += velocity.x * advection_strength * params.dt * (chemical_field[right_index] - diffused_value);
            }
            
            // Y-direction advection
            if (velocity.y > 0.0) {
                let up_y = select(y - 1u, resolution - 1u, y == 0u);
                let up_index = (up_y * resolution + x) * params.chemical_types + chemical_type;
                advected_value -= velocity.y * advection_strength * params.dt * (diffused_value - chemical_field[up_index]);
            } else if (velocity.y < 0.0) {
                let down_y = select(y + 1u, 0u, y >= resolution - 1u);
                let down_index = (down_y * resolution + x) * params.chemical_types + chemical_type;
                advected_value += velocity.y * advection_strength * params.dt * (chemical_field[down_index] - diffused_value);
            }
            
            diffused_value = advected_value;
        }
        
        // Ensure non-negative and apply upper bound to prevent accumulation
        diffused_value = clamp(diffused_value, 0.0, 15.0);
        
        // Write to temporary buffer
        chemical_field_temp[center_index] = diffused_value;
    }
    
    // Apply chemical reactions at this grid point
    var chemical_values = array<f32, 6>(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (var i = 0u; i < params.chemical_types; i++) {
        let index = (y * resolution + x) * params.chemical_types + i;
        chemical_values[i] = chemical_field_temp[index];
    }
    
    let reacted_values = apply_chemical_reactions(chemical_values);
    
    // Write back the reacted values with final bounds checking
    for (var i = 0u; i < params.chemical_types; i++) {
        let index = (y * resolution + x) * params.chemical_types + i;
        chemical_field_temp[index] = clamp(reacted_values[i], 0.0, 15.0);
    }
} 