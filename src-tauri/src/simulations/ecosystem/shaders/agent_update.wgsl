// Agent update compute shader for ecosystem simulation
// Handles agent movement, chemical sensing, decision making, and reproduction

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
    energy: f32,
    age: f32,
    species: u32,
    _pad1: u32,
    neural_weights: array<f32, 8>, // Simple neural network weights
    sensor_readings: array<f32, 4>, // Chemical sensor readings
    goal: u32, // Current behavioral goal
    _pad2: u32,
    memory: array<f32, 4>, // Short-term memory
}

struct Food {
    position: vec2<f32>,
    energy: f32,
    is_active: u32,
}

struct SimParams {
    agent_count: u32,
    width: f32,
    height: f32,
    dt: f32,
    
    // Agent parameters
    agent_speed_min: f32,
    agent_speed_max: f32,
    agent_turn_rate: f32,
    sensor_range: f32,
    sensor_count: u32,
    sensor_angle: f32,
    
    // Chemical parameters
    chemical_resolution: u32,
    chemical_types: u32,
    chemical_diffusion_rate: f32,
    chemical_decay_rate: f32,
    chemical_deposition_rate: f32,
    
    // Learning parameters
    learning_rate: f32,
    mutation_rate: f32,
    energy_consumption_rate: f32,
    energy_gain_from_food: f32,
    reproduction_energy_threshold: f32,
    reproduction_probability: f32,
    
    // Species parameters
    species_count: u32,
    intra_species_attraction: f32,
    inter_species_repulsion: f32,
    
    // Environmental parameters
    brownian_motion_strength: f32,
    food_spawn_rate: f32,
    max_food_particles: u32,
    wrap_edges: u32,
    
    random_seed: u32,
    time: f32,
}

@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> food_particles: array<Food>;
@group(0) @binding(2) var<storage, read_write> chemical_field: array<f32>;
@group(0) @binding(3) var<uniform> params: SimParams;

// Simple random number generator
fn random(seed: ptr<function, u32>) -> f32 {
    *seed = (*seed * 1664525u + 1013904223u);
    return f32(*seed) / 4294967296.0;
}

// Sample chemical concentration at a position
fn sample_chemical(pos: vec2<f32>, chemical_type: u32) -> f32 {
    // Convert from [-1, 1] to [0, chemical_resolution]
    let grid_pos = vec2<u32>(
        u32(clamp((pos.x + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u))),
        u32(clamp((pos.y + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u)))
    );
    
    let index = (grid_pos.y * params.chemical_resolution + grid_pos.x) * params.chemical_types + chemical_type;
    return chemical_field[index];
}

// Deposit chemical at a position
fn deposit_chemical(pos: vec2<f32>, chemical_type: u32, amount: f32) {
    // Convert from [-1, 1] to [0, chemical_resolution]
    let grid_pos = vec2<u32>(
        u32(clamp((pos.x + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u))),
        u32(clamp((pos.y + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u)))
    );
    
    let index = (grid_pos.y * params.chemical_resolution + grid_pos.x) * params.chemical_types + chemical_type;
    chemical_field[index] += amount;
}

// Simple neural network forward pass
fn neural_decision(agent: ptr<function, Agent>, inputs: array<f32, 4>) -> vec2<f32> {
    var output = vec2<f32>(0.0, 0.0);
    
    // Simple feedforward network
    for (var i = 0u; i < 4u; i++) {
        output.x += inputs[i] * (*agent).neural_weights[i];
        output.y += inputs[i] * (*agent).neural_weights[i + 4u];
    }
    
    // Apply activation function (tanh)
    output.x = tanh(output.x);
    output.y = tanh(output.y);
    
    return output;
}

// Update agent sensors
fn update_sensors(agent: ptr<function, Agent>) {
    let agent_pos = (*agent).position;
    let agent_heading = atan2((*agent).velocity.y, (*agent).velocity.x);
    
    // Sample chemicals at sensor positions
    for (var i = 0u; i < params.sensor_count && i < 4u; i++) {
        let sensor_angle = agent_heading + (f32(i) - f32(params.sensor_count - 1u) * 0.5) * params.sensor_angle;
        let sensor_pos = agent_pos + vec2<f32>(cos(sensor_angle), sin(sensor_angle)) * params.sensor_range;
        
        // Sample different chemical types and combine
        var total_chemical = 0.0;
        for (var j = 0u; j < params.chemical_types; j++) {
            total_chemical += sample_chemical(sensor_pos, j);
        }
        
        (*agent).sensor_readings[i] = total_chemical;
    }
}

// Find nearest food particle
fn find_nearest_food(agent_pos: vec2<f32>) -> vec2<f32> {
    var nearest_pos = vec2<f32>(0.0, 0.0);
    var min_distance = 1000000.0;
    
    for (var i = 0u; i < params.max_food_particles; i++) {
        if (food_particles[i].is_active != 0u) {
            let distance = length(food_particles[i].position - agent_pos);
            if (distance < min_distance) {
                min_distance = distance;
                nearest_pos = food_particles[i].position;
            }
        }
    }
    
    return nearest_pos;
}

// Check for food consumption
fn consume_food(agent: ptr<function, Agent>) {
    let agent_pos = (*agent).position;
    
    for (var i = 0u; i < params.max_food_particles; i++) {
        if (food_particles[i].is_active != 0u) {
            let distance = length(food_particles[i].position - agent_pos);
            if (distance < 0.05) { // Consumption radius (small in [-1, 1] space)
                (*agent).energy += food_particles[i].energy;
                food_particles[i].is_active = 0u;
                food_particles[i].energy = 0.0;
                break;
            }
        }
    }
}

// Handle agent reproduction
fn try_reproduce(agent: ptr<function, Agent>, agent_index: u32, seed: ptr<function, u32>) {
    if ((*agent).energy > params.reproduction_energy_threshold && 
        random(seed) < params.reproduction_probability) {
        
        // Find empty slot for offspring
        for (var i = 0u; i < params.agent_count; i++) {
            if (agents[i].energy <= 0.0) {
                // Create offspring with mutations
                agents[i] = *agent;
                agents[i].energy = (*agent).energy * 0.5;
                (*agent).energy *= 0.5;
                agents[i].age = 0.0;
                
                // Mutate neural weights
                for (var j = 0u; j < 8u; j++) {
                    if (random(seed) < params.mutation_rate) {
                        agents[i].neural_weights[j] += (random(seed) - 0.5) * 0.1;
                        agents[i].neural_weights[j] = clamp(agents[i].neural_weights[j], -1.0, 1.0);
                    }
                }
                
                // Slightly offset position (small offset in [-1, 1] space)
                agents[i].position += vec2<f32>(
                    (random(seed) - 0.5) * 0.1,
                    (random(seed) - 0.5) * 0.1
                );
                
                break;
            }
        }
    }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.agent_count) {
        return;
    }
    
    var agent = agents[index];
    var seed = params.random_seed + index + u32(params.time * 1000.0);
    
    // Skip dead agents
    if (agent.energy <= 0.0) {
        return;
    }
    
    // Update sensors
    update_sensors(&agent);
    
    // Make decision using neural network
    let decision = neural_decision(&agent, agent.sensor_readings);
    
    // Apply decision to movement
    let current_heading = atan2(agent.velocity.y, agent.velocity.x);
    let new_heading = current_heading + decision.x * params.agent_turn_rate * params.dt;
    let speed = mix(params.agent_speed_min, params.agent_speed_max, (decision.y + 1.0) * 0.5);
    
    // Update velocity
    agent.velocity = vec2<f32>(cos(new_heading), sin(new_heading)) * speed;
    
    // Add Brownian motion
    agent.velocity += vec2<f32>(
        (random(&seed) - 0.5) * params.brownian_motion_strength,
        (random(&seed) - 0.5) * params.brownian_motion_strength
    );
    
    // Update position
    agent.position += agent.velocity * params.dt;
    
    // Handle boundaries ([-1, 1] coordinate system)
    if (params.wrap_edges != 0u) {
        if (agent.position.x < -1.0) { agent.position.x += 2.0; }
        if (agent.position.x > 1.0) { agent.position.x -= 2.0; }
        if (agent.position.y < -1.0) { agent.position.y += 2.0; }
        if (agent.position.y > 1.0) { agent.position.y -= 2.0; }
    } else {
        agent.position.x = clamp(agent.position.x, -1.0, 1.0);
        agent.position.y = clamp(agent.position.y, -1.0, 1.0);
    }
    
    // Consume energy
    agent.energy -= params.energy_consumption_rate * params.dt;
    agent.age += params.dt;
    
    // Check for food consumption
    consume_food(&agent);
    
    // Deposit pheromones based on species
    deposit_chemical(agent.position, agent.species, params.chemical_deposition_rate * params.dt);
    
    // Try to reproduce
    try_reproduce(&agent, index, &seed);
    
    // Update agent in buffer
    agents[index] = agent;
} 