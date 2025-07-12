// Fluid dynamics compute shader for ecosystem simulation
// Implements realistic fluid flow with biological, physical, and chemical current generation

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
    energy: f32,
    age: f32,
    ecological_role: u32,
    variant: u32,
    
    sensor_readings: array<f32, 4>,
    
    heading: f32,
    run_duration: f32,
    run_timer: f32,
    tumble_cooldown: f32,
    
    metabolism_rate: f32,
    reproductive_threshold: f32,
    last_reproduction_time: f32,
    
    behavioral_state: u32,
    state_timer: f32,
    
    chemical_secretion_rates: array<f32, 6>,
    
    food_memory: array<f32, 4>,
    threat_memory: array<f32, 4>,
    
    biofilm_strength: f32,
    biofilm_connections: u32,
    
    hunt_target_id: u32,
    pack_coordination: f32,
    
    territory_center: array<f32, 2>,
    territory_radius: f32,
    
    // Visibility control
    is_visible: u32,  // 0 = hidden, 1 = visible
    
    _pad: array<u32, 1>,
}

struct SimParams {
    agent_count: u32,
    width: f32,
    height: f32,
    dt: f32,
    
    // Fluid dynamics parameters
    enable_fluid_dynamics: u32,
    fluid_viscosity: f32,
    fluid_density: f32,
    biological_current_strength: f32,
    chemical_current_strength: f32,
    flow_update_frequency: u32,
    
    // Chemical parameters
    chemical_resolution: u32,
    chemical_types: u32,
    chemical_diffusion_rate: f32,
    chemical_decay_rate: f32,
    chemical_deposition_rate: f32,
    
    // Light gradient
    enable_light_gradient: u32,
    base_light_intensity: f32,
    light_gradient_strength: f32,
    light_rotation_speed: f32,
    light_direction_angle: f32,
    
    // Environmental factors
    temperature_gradient_strength: f32,
    ph_gradient_strength: f32,
    
    wrap_edges: u32,
    random_seed: u32,
    time: f32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> chemical_field: array<f32>;
@group(0) @binding(2) var<storage, read_write> velocity_field: array<vec2<f32>>;
@group(0) @binding(3) var<storage, read_write> pressure_field: array<f32>;
@group(0) @binding(4) var<uniform> params: SimParams;

// Convert world position to grid coordinates
fn world_to_grid(world_pos: vec2<f32>) -> vec2<u32> {
    let grid_x = u32(clamp((world_pos.x + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u)));
    let grid_y = u32(clamp((world_pos.y + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u)));
    return vec2<u32>(grid_x, grid_y);
}

// Convert grid coordinates to world position
fn grid_to_world(grid_pos: vec2<u32>) -> vec2<f32> {
    let world_x = (f32(grid_pos.x) / f32(params.chemical_resolution)) * 2.0 - 1.0;
    let world_y = (f32(grid_pos.y) / f32(params.chemical_resolution)) * 2.0 - 1.0;
    return vec2<f32>(world_x, world_y);
}

// Sample chemical concentration at grid position
fn sample_chemical(grid_pos: vec2<u32>, chemical_type: u32) -> f32 {
    let index = (grid_pos.y * params.chemical_resolution + grid_pos.x) * params.chemical_types + chemical_type;
    if (index >= arrayLength(&chemical_field)) {
        return 0.0;
    }
    return chemical_field[index];
}

// Calculate biological current generation from agent movement
fn calculate_biological_current(grid_pos: vec2<u32>) -> vec2<f32> {
    let world_pos = grid_to_world(grid_pos);
    var current = vec2<f32>(0.0, 0.0);
    let influence_radius = 0.1; // Influence radius in world coordinates
    
    // Sum contributions from all living agents
    for (var i = 0u; i < params.agent_count; i++) {
        if (agents[i].energy <= 0.0) {
            continue;
        }
        
        let agent_pos = agents[i].position;
        let distance = length(world_pos - agent_pos);
        
        if (distance < influence_radius) {
            // Calculate influence based on distance (fixed influence for all agents)
            let influence = 1.0 - (distance / influence_radius);
            let agent_velocity = agents[i].velocity;
            let fixed_influence_factor = 0.01; // Fixed influence factor for all agents
            
            // Different ecological roles create different flow patterns
            var flow_contribution = agent_velocity * influence * fixed_influence_factor;
            
            if (agents[i].ecological_role == 0u) { // Recyclers
                if (agents[i].variant == 0u) { // Bacteria - create turbulent micro-currents
                    flow_contribution *= 1.5;
                } else if (agents[i].variant == 1u) { // Fungi - create network flows
                    // Create radial flow pattern for network formation
                    let radial_direction = normalize(world_pos - agent_pos);
                    flow_contribution += radial_direction * 0.001 * agents[i].biofilm_strength;
                }
            } else if (agents[i].ecological_role == 1u) { // Producers
                if (agents[i].variant == 0u) { // Algae - create biofilm displacement
                    // Biofilm creates flow obstacles
                    if (agents[i].biofilm_strength > 0.1) {
                        let obstacle_flow = normalize(world_pos - agent_pos) * 0.002 * agents[i].biofilm_strength;
                        flow_contribution += obstacle_flow;
                    }
                }
            }
            
            current += flow_contribution * params.biological_current_strength;
        }
    }
    
    return current;
}

// Calculate physical current generation from density gradients
fn calculate_physical_current(grid_pos: vec2<u32>) -> vec2<f32> {
    let world_pos = grid_to_world(grid_pos);
    var current = vec2<f32>(0.0, 0.0);
    
    // Calculate density from chemical concentrations
    let oxygen = sample_chemical(grid_pos, 0u);
    let co2 = sample_chemical(grid_pos, 1u);
    let nitrogen = sample_chemical(grid_pos, 2u);
    let total_density = oxygen + co2 + nitrogen;
    
    // Calculate density gradient in 4 directions
    let dx = 1u;
    let dy = 1u;
    
    // Handle boundaries
    let left_x = select(grid_pos.x - dx, params.chemical_resolution - 1u, grid_pos.x == 0u);
    let right_x = select(grid_pos.x + dx, 0u, grid_pos.x >= params.chemical_resolution - 1u);
    let up_y = select(grid_pos.y - dy, params.chemical_resolution - 1u, grid_pos.y == 0u);
    let down_y = select(grid_pos.y + dy, 0u, grid_pos.y >= params.chemical_resolution - 1u);
    
    // Sample neighboring densities
    let left_density = sample_chemical(vec2<u32>(left_x, grid_pos.y), 0u) + 
                       sample_chemical(vec2<u32>(left_x, grid_pos.y), 1u) + 
                       sample_chemical(vec2<u32>(left_x, grid_pos.y), 2u);
    let right_density = sample_chemical(vec2<u32>(right_x, grid_pos.y), 0u) + 
                        sample_chemical(vec2<u32>(right_x, grid_pos.y), 1u) + 
                        sample_chemical(vec2<u32>(right_x, grid_pos.y), 2u);
    let up_density = sample_chemical(vec2<u32>(grid_pos.x, up_y), 0u) + 
                     sample_chemical(vec2<u32>(grid_pos.x, up_y), 1u) + 
                     sample_chemical(vec2<u32>(grid_pos.x, up_y), 2u);
    let down_density = sample_chemical(vec2<u32>(grid_pos.x, down_y), 0u) + 
                       sample_chemical(vec2<u32>(grid_pos.x, down_y), 1u) + 
                       sample_chemical(vec2<u32>(grid_pos.x, down_y), 2u);
    
    // Calculate density gradient
    let density_gradient_x = (right_density - left_density) / 2.0;
    let density_gradient_y = (down_density - up_density) / 2.0;
    
    // Flow from high density to low density
    current = vec2<f32>(-density_gradient_x, -density_gradient_y) * 0.01;
    
    // Add temperature-driven convection
    let temperature_factor = sample_temperature_gradient(world_pos);
    let thermal_current = vec2<f32>(0.0, (temperature_factor - 0.5) * 0.005);
    current += thermal_current;
    
    return current;
}

// Calculate chemical current generation from osmotic pressure
fn calculate_chemical_current(grid_pos: vec2<u32>) -> vec2<f32> {
    var current = vec2<f32>(0.0, 0.0);
    
    // Calculate osmotic pressure from chemical gradients
    let dx = 1u;
    let dy = 1u;
    
    // Handle boundaries
    let left_x = select(grid_pos.x - dx, params.chemical_resolution - 1u, grid_pos.x == 0u);
    let right_x = select(grid_pos.x + dx, 0u, grid_pos.x >= params.chemical_resolution - 1u);
    let up_y = select(grid_pos.y - dy, params.chemical_resolution - 1u, grid_pos.y == 0u);
    let down_y = select(grid_pos.y + dy, 0u, grid_pos.y >= params.chemical_resolution - 1u);
    
    // Calculate osmotic pressure from each chemical type
    for (var chem_type = 0u; chem_type < params.chemical_types; chem_type++) {
        let center_conc = sample_chemical(grid_pos, chem_type);
        let left_conc = sample_chemical(vec2<u32>(left_x, grid_pos.y), chem_type);
        let right_conc = sample_chemical(vec2<u32>(right_x, grid_pos.y), chem_type);
        let up_conc = sample_chemical(vec2<u32>(grid_pos.x, up_y), chem_type);
        let down_conc = sample_chemical(vec2<u32>(grid_pos.x, down_y), chem_type);
        
        // Calculate chemical gradient
        let gradient_x = (right_conc - left_conc) / 2.0;
        let gradient_y = (down_conc - up_conc) / 2.0;
        
        // Different chemicals create different osmotic pressures
        var osmotic_strength = 0.001;
        if (chem_type == 0u) { // Oxygen
            osmotic_strength = 0.0015;
        } else if (chem_type == 1u) { // CO2
            osmotic_strength = 0.0012;
        } else if (chem_type == 2u) { // Nitrogen compounds
            osmotic_strength = 0.0008;
        } else if (chem_type == 3u) { // Pheromones
            osmotic_strength = 0.0005;
        } else if (chem_type == 4u) { // Toxins
            osmotic_strength = 0.0007;
        } else if (chem_type == 5u) { // Attractants
            osmotic_strength = 0.0006;
        }
        
        // Flow from high concentration to low concentration
        current += vec2<f32>(-gradient_x, -gradient_y) * osmotic_strength;
    }
    
    return current * params.chemical_current_strength;
}

// Sample light gradient for photosynthesis-driven currents
fn sample_light_gradient(world_pos: vec2<f32>) -> f32 {
    if (params.enable_light_gradient == 0u) {
        return params.base_light_intensity;
    }
    
    let light_direction = vec2<f32>(cos(params.light_direction_angle), sin(params.light_direction_angle));
    let light_intensity = params.base_light_intensity + 
                         dot(world_pos, light_direction) * params.light_gradient_strength;
    return clamp(light_intensity, 0.0, 1.0);
}

// Sample temperature gradient for thermal currents
fn sample_temperature_gradient(world_pos: vec2<f32>) -> f32 {
    let distance_from_center = length(world_pos);
    let base_temperature = 0.5;
    let temperature_variation = sin(world_pos.x * 2.0) * cos(world_pos.y * 1.5) * 0.3;
    let temperature = base_temperature + temperature_variation * params.temperature_gradient_strength;
    return clamp(temperature, 0.0, 1.0);
}

// Apply viscosity and diffusion to velocity field
fn apply_viscosity(grid_pos: vec2<u32>, current_velocity: vec2<f32>) -> vec2<f32> {
    let dx = 1u;
    let dy = 1u;
    
    // Handle boundaries
    let left_x = select(grid_pos.x - dx, params.chemical_resolution - 1u, grid_pos.x == 0u);
    let right_x = select(grid_pos.x + dx, 0u, grid_pos.x >= params.chemical_resolution - 1u);
    let up_y = select(grid_pos.y - dy, params.chemical_resolution - 1u, grid_pos.y == 0u);
    let down_y = select(grid_pos.y + dy, 0u, grid_pos.y >= params.chemical_resolution - 1u);
    
    // Sample neighboring velocities
    let left_vel = velocity_field[(grid_pos.y * params.chemical_resolution + left_x)];
    let right_vel = velocity_field[(grid_pos.y * params.chemical_resolution + right_x)];
    let up_vel = velocity_field[(up_y * params.chemical_resolution + grid_pos.x)];
    let down_vel = velocity_field[(down_y * params.chemical_resolution + grid_pos.x)];
    
    // Calculate velocity laplacian (viscosity diffusion)
    let velocity_laplacian = (left_vel + right_vel + up_vel + down_vel - 4.0 * current_velocity);
    
    // Apply viscosity
    return current_velocity + velocity_laplacian * params.fluid_viscosity * params.dt;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let grid_pos = vec2<u32>(global_id.x, global_id.y);
    
    if (grid_pos.x >= params.chemical_resolution || grid_pos.y >= params.chemical_resolution) {
        return;
    }
    
    let grid_index = grid_pos.y * params.chemical_resolution + grid_pos.x;
    
    // Skip fluid dynamics if disabled
    if (params.enable_fluid_dynamics == 0u) {
        velocity_field[grid_index] = vec2<f32>(0.0, 0.0);
        pressure_field[grid_index] = 0.0;
        return;
    }
    
    // Calculate current velocity
    let current_velocity = velocity_field[grid_index];
    
    // Calculate current generation from different sources
    let biological_current = calculate_biological_current(grid_pos);
    let physical_current = calculate_physical_current(grid_pos);
    let chemical_current = calculate_chemical_current(grid_pos);
    
    // Combine currents
    var new_velocity = current_velocity + 
                      (biological_current + physical_current + chemical_current) * params.dt;
    
    // Apply viscosity and diffusion
    new_velocity = apply_viscosity(grid_pos, new_velocity);
    
    // Apply velocity damping to prevent instability
    let damping_factor = 0.95;
    new_velocity *= damping_factor;
    
    // Clamp velocity to prevent extreme values
    let max_velocity = 0.1;
    new_velocity = clamp(new_velocity, vec2<f32>(-max_velocity, -max_velocity), vec2<f32>(max_velocity, max_velocity));
    
    // Update velocity field
    velocity_field[grid_index] = new_velocity;
    
    // Calculate pressure from velocity divergence (simplified)
    let dx = 1u;
    let dy = 1u;
    
    let left_x = select(grid_pos.x - dx, params.chemical_resolution - 1u, grid_pos.x == 0u);
    let right_x = select(grid_pos.x + dx, 0u, grid_pos.x >= params.chemical_resolution - 1u);
    let up_y = select(grid_pos.y - dy, params.chemical_resolution - 1u, grid_pos.y == 0u);
    let down_y = select(grid_pos.y + dy, 0u, grid_pos.y >= params.chemical_resolution - 1u);
    
    let left_vel = velocity_field[(grid_pos.y * params.chemical_resolution + left_x)];
    let right_vel = velocity_field[(grid_pos.y * params.chemical_resolution + right_x)];
    let up_vel = velocity_field[(up_y * params.chemical_resolution + grid_pos.x)];
    let down_vel = velocity_field[(down_y * params.chemical_resolution + grid_pos.x)];
    
    // Calculate divergence
    let divergence = (right_vel.x - left_vel.x) / 2.0 + (down_vel.y - up_vel.y) / 2.0;
    
    // Update pressure field
    pressure_field[grid_index] = -divergence * params.fluid_density;
} 