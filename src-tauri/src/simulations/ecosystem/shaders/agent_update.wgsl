// Agent update compute shader for ecosystem simulation
// Implements realistic microbial movement, sensing, and ecological behaviors

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
    energy: f32,
    age: f32,
    ecological_role: u32,  // 0: Recycler, 1: Producer, 2: Predator
    variant: u32,          // Variant within ecological role
    
    // Sensor array: 3-4 chemical receptors pointing different directions
    sensor_readings: array<f32, 4>,
    
    // Movement engine parameters for run-and-tumble
    heading: f32,
    run_duration: f32,         // Current run duration in run-and-tumble
    run_timer: f32,            // Timer for current run
    tumble_cooldown: f32,      // Cooldown after tumbling
    
    // Metabolic system
    metabolism_rate: f32,
    reproductive_threshold: f32,
    last_reproduction_time: f32,
    
    // Behavioral state: 0: feeding, 1: hunting, 2: reproducing, 3: escaping
    behavioral_state: u32,
    state_timer: f32,          // Timer for current state
    
    // Chemical secretion rates for 6 chemical types
    chemical_secretion_rates: array<f32, 6>,
    
    // Simple memory: recent food locations and threats
    food_memory: array<f32, 4>,     // x, y positions of recent food
    threat_memory: array<f32, 4>,   // x, y positions of recent threats
    
    // Biofilm formation (for producers)
    biofilm_strength: f32,
    biofilm_connections: u32,
    
    // Hunting mechanics (for predators)
    hunt_target_id: u32,
    pack_coordination: f32,
    
    // Spatial organization
    territory_center: array<f32, 2>,
    territory_radius: f32,
    
    // Visibility control
    is_visible: u32,  // 0 = hidden, 1 = visible
    
    _pad: array<u32, 1>,
}

struct DeadBiomass {
    position: vec2<f32>,
    biomass_amount: f32,
    species_origin: u32,
    decay_time: f32,
    decomposition_progress: f32,
    is_active: u32,
    _pad: u32,
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
    brownian_motion_strength: f32,
    
    // Chemical parameters
    chemical_resolution: u32,
    chemical_types: u32,
    chemical_diffusion_rate: f32,
    chemical_decay_rate: f32,
    chemical_deposition_rate: f32,
    
    // Ecological parameters
    ecological_roles: u32,
    variants_per_role: u32,
    recycler_efficiency: f32,
    producer_photosynthesis_rate: f32,
    predator_hunting_efficiency: f32,

    // Energy and metabolism
    energy_consumption_rate: f32,
    energy_gain_from_food: f32,
    reproduction_energy_threshold: f32,
    reproduction_probability: f32,
    mutation_rate: f32,

    // Unified nutrient architecture
    max_particles: u32,
    particle_decomposition_rate: f32,
    particle_decay_rate: f32,
    matter_to_chemical_ratio: f32,

    // Fluid dynamics
    enable_fluid_dynamics: u32,
    fluid_viscosity: f32,
    fluid_density: f32,
    biological_current_strength: f32,
    chemical_current_strength: f32,
    flow_update_frequency: u32,

    // Light gradient
    enable_light_gradient: u32,
    base_light_intensity: f32,
    light_gradient_strength: f32,
    light_rotation_speed: f32,
    light_direction_angle: f32,

    // Movement and sensing
    chemotaxis_sensitivity: f32,
    run_duration_min: f32,
    run_duration_max: f32,
    tumble_angle_range: f32,
    flagella_strength: f32,
    receptor_saturation_threshold: f32,

    // Hunting mechanics
    predation_contact_range: f32,
    pack_hunting_bonus: f32,
    predation_success_rate: f32,

    // Spatial organization
    enable_biofilm_formation: u32,
    biofilm_growth_rate: f32,
    biofilm_persistence: f32,
    nutrient_stream_threshold: f32,
    territory_establishment_range: f32,

    // Population dynamics
    carrying_capacity: f32,
    population_oscillation_damping: f32,
    resource_competition_strength: f32,
    succession_pattern_strength: f32,

    // Environmental factors
    temperature_gradient_strength: f32,
    ph_gradient_strength: f32,
    toxin_accumulation_rate: f32,
    dead_zone_threshold: f32,

    wrap_edges: u32,
    random_seed: u32,
    time: f32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> biomass_particles: array<DeadBiomass>;
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
    if (index >= arrayLength(&chemical_field)) {
        return 0.0;
    }
    return chemical_field[index];
}

// Deposit chemical at a position
fn deposit_chemical(pos: vec2<f32>, chemical_type: u32, amount: f32) {
    let grid_pos = vec2<u32>(
        u32(clamp((pos.x + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u))),
        u32(clamp((pos.y + 1.0) / 2.0 * f32(params.chemical_resolution), 0.0, f32(params.chemical_resolution - 1u)))
    );
    
    let index = (grid_pos.y * params.chemical_resolution + grid_pos.x) * params.chemical_types + chemical_type;
    if (index < arrayLength(&chemical_field)) {
        chemical_field[index] = clamp(chemical_field[index] + amount, 0.0, 10.0);
    }
}

// Sample light gradient based on constant angular gradient
fn sample_light_gradient(pos: vec2<f32>) -> f32 {
    if (params.enable_light_gradient == 0u) {
        return params.base_light_intensity;
    }
    
    // Calculate light direction vector
    let light_dir = vec2<f32>(cos(params.light_direction_angle), sin(params.light_direction_angle));
    
    // Calculate light intensity based on position along gradient
    let light_factor = dot(pos, light_dir);
    let intensity = params.base_light_intensity + params.light_gradient_strength * light_factor;
    
    return clamp(intensity, 0.0, 1.0);
}

// Update chemical sensors - 3-4 receptors pointing in different directions
fn update_sensors(agent_index: u32, seed: ptr<function, u32>) {
    let agent_pos = agents[agent_index].position;
    let sensor_range = params.sensor_range;
    let sensor_angle = params.sensor_angle;
    
    // Sample chemicals at sensor positions
    for (var i = 0u; i < 4u; i++) {
        let angle = agents[agent_index].heading + f32(i) * sensor_angle - (sensor_angle * 1.5);
        let sensor_pos = agent_pos + vec2<f32>(cos(angle), sin(angle)) * sensor_range;
        
        // Sample multiple chemical types and combine based on ecological role
        var reading = 0.0;
        
        if (agents[agent_index].ecological_role == 0u) { // Recyclers
            // Detect decomposing matter and nitrogen compounds
            reading += sample_chemical(sensor_pos, 2u) * 2.0; // Nitrogen compounds
            reading += sample_chemical(sensor_pos, 5u) * 1.5; // Attractants
        } else if (agents[agent_index].ecological_role == 1u) { // Producers
            // Detect CO2 and nutrients
            reading += sample_chemical(sensor_pos, 1u) * 2.0; // CO2
            reading += sample_chemical(sensor_pos, 2u) * 1.0; // Nitrogen compounds
        } else { // Predators
            // Detect pheromones and other organisms
            reading += sample_chemical(sensor_pos, 3u) * 2.0; // Pheromones
            reading += sample_chemical(sensor_pos, 0u) * 1.0; // Oxygen (indicates life)
        }
        
        // Apply receptor saturation
        reading = clamp(reading, 0.0, params.receptor_saturation_threshold);
        agents[agent_index].sensor_readings[i] = reading;
    }
}

// Implement run-and-tumble movement
fn update_movement(agent_index: u32, seed: ptr<function, u32>) {
    // Update run timer
    agents[agent_index].run_timer += params.dt;
    
    // Check if it's time to tumble
    if (agents[agent_index].run_timer >= agents[agent_index].run_duration) {
        // Tumble: randomly change direction
        let tumble_angle = (random(seed) - 0.5) * params.tumble_angle_range;
        agents[agent_index].heading += tumble_angle;
        
        // Reset run timer and set new run duration
        agents[agent_index].run_timer = 0.0;
        agents[agent_index].run_duration = params.run_duration_min + 
            random(seed) * (params.run_duration_max - params.run_duration_min);
    }
    
    // Apply chemotaxis bias
    let left_reading = agents[agent_index].sensor_readings[0];
    let front_reading = agents[agent_index].sensor_readings[1];
    let right_reading = agents[agent_index].sensor_readings[2];
    
    // Calculate turning bias based on sensor readings
    let turn_bias = (right_reading - left_reading) * params.chemotaxis_sensitivity;
    agents[agent_index].heading += turn_bias * params.dt;
    
    // Calculate speed based on front sensor and ecological role
    var speed = params.agent_speed_min + front_reading * (params.agent_speed_max - params.agent_speed_min);
    
            // Apply ecological role-specific speed modifiers
        if (agents[agent_index].ecological_role == 0u) { // Recyclers
            if (agents[agent_index].variant == 0u) { // Bacteria - fast, swarm behavior
                speed *= 1.2;
            } else if (agents[agent_index].variant == 1u) { // Fungi - slow, network building
                speed *= 0.6;
            } else { // Decomposer Protozoans - moderate speed, selective
                speed *= 0.8;
            }
        } else if (agents[agent_index].ecological_role == 1u) { // Producers
            if (agents[agent_index].variant == 0u) { // Algae - slow, biofilm formation
                speed *= 0.7;
            } else if (agents[agent_index].variant == 1u) { // Cyanobacteria - moderate speed, mobile colonies
                speed *= 0.9;
            } else { // Photosynthetic Protists - complex movement patterns
                speed *= 0.8;
            }
        } else if (agents[agent_index].ecological_role == 2u) { // Predators
            if (agents[agent_index].variant == 0u) { // Predatory bacteria - fast, coordinated groups
                speed *= 1.1;
            } else if (agents[agent_index].variant == 1u) { // Viruses - extremely fast
                speed *= 1.2;
            } else if (agents[agent_index].variant == 2u) { // Predatory Protozoans - slow but powerful
                speed *= 0.8;
            } else { // Parasitic Microbes - moderate speed, persistent
                speed *= 0.9;
            }
        }
    
    // Apply flagella simulation
    let flagella_force = params.flagella_strength * speed;
    
    // Update velocity with heading and flagella force
    agents[agent_index].velocity = vec2<f32>(cos(agents[agent_index].heading), sin(agents[agent_index].heading)) * flagella_force;
    
    // Add Brownian motion
    agents[agent_index].velocity += vec2<f32>(
        (random(seed) - 0.5) * params.brownian_motion_strength,
        (random(seed) - 0.5) * params.brownian_motion_strength
    );
}

// Handle ecological role-specific behaviors
fn update_ecological_behavior(agent_index: u32, seed: ptr<function, u32>) {
    let agent_pos = agents[agent_index].position;
    let ecological_role = agents[agent_index].ecological_role;
    let variant = agents[agent_index].variant;
    
    if (ecological_role == 0u) { // Recyclers
        // Look for dead biomass to decompose
        for (var i = 0u; i < params.max_particles; i++) {
            if (biomass_particles[i].is_active == 1u) {
                let distance = length(biomass_particles[i].position - agent_pos);
                if (distance < 0.05) {
                    // Consume biomass and convert to chemicals
                    let consumption = params.recycler_efficiency * params.dt;
                    let consumed = min(consumption, biomass_particles[i].biomass_amount);
                    
                    biomass_particles[i].biomass_amount -= consumed;
                    biomass_particles[i].decomposition_progress += consumed * 0.1;
                    
                    // Gain energy from decomposition
                    agents[agent_index].energy += consumed * params.energy_gain_from_food;
                    
                    // Produce chemicals
                    deposit_chemical(agent_pos, 2u, consumed * 0.5); // Nitrogen compounds
                    
                    if (biomass_particles[i].biomass_amount <= 0.1) {
                        biomass_particles[i].is_active = 0u;
                    }
                }
            }
        }
    } else if (ecological_role == 1u) { // Producers
        // Photosynthesis based on light
        let light_intensity = sample_light_gradient(agent_pos);
        let photosynthetic_gain = light_intensity * params.producer_photosynthesis_rate * params.dt;
        
        // Consume CO2 and produce oxygen
        let co2_available = sample_chemical(agent_pos, 1u);
        let actual_gain = min(photosynthetic_gain, co2_available * 0.5);
        
        agents[agent_index].energy += actual_gain;
        
        // Deposit oxygen and consume CO2
        deposit_chemical(agent_pos, 0u, actual_gain * 0.8); // Oxygen
        deposit_chemical(agent_pos, 1u, -actual_gain * 0.6); // CO2 consumption
        
        // Biofilm formation for algae
        if (variant == 0u && params.enable_biofilm_formation == 1u) {
            agents[agent_index].biofilm_strength += params.biofilm_growth_rate * params.dt;
            agents[agent_index].biofilm_strength = min(agents[agent_index].biofilm_strength, 1.0);
        }
    } else if (ecological_role == 2u) { // Predators
        // Enhanced hunting mechanics with pack coordination
        var closest_prey_index = 0u;
        var closest_prey_distance = 1000.0;
        var pack_members = 0u;
        
        // First pass: find closest prey and count pack members
        for (var i = 0u; i < params.agent_count; i++) {
            if (i != agent_index && agents[i].energy > 0.0) {
                let distance = length(agents[i].position - agent_pos);
                
                // Count pack members (same species nearby)
                if (agents[i].ecological_role == 2u && agents[i].variant == variant && 
                    distance < params.territory_establishment_range) {
                    pack_members++;
                }
                
                // Find closest prey
                if (agents[i].ecological_role != 2u && distance < closest_prey_distance) {
                    closest_prey_distance = distance;
                    closest_prey_index = i;
                }
            }
        }
        
        // Update pack coordination
        agents[agent_index].pack_coordination = min(f32(pack_members) / 5.0, 1.0);
        
        // Hunt the closest prey if within range
        if (closest_prey_distance < params.predation_contact_range && agents[closest_prey_index].energy > 0.0) {
            let prey_pos = agents[closest_prey_index].position;
            
            // Calculate hunt success factors
            var hunt_success_rate = params.predation_success_rate * params.predator_hunting_efficiency;
            
            // Pack hunting bonus
            hunt_success_rate *= (1.0 + agents[agent_index].pack_coordination * params.pack_hunting_bonus);
            
            // Variant-specific hunting behaviors
            if (variant == 0u) { // Predatory bacteria - coordinated group hunting
                hunt_success_rate *= 1.2;
            } else if (variant == 1u) { // Viruses - inject into hosts
                hunt_success_rate *= 1.5;
            } else if (variant == 2u) { // Predatory Protozoans - engulf smaller organisms
                hunt_success_rate *= 1.4;
            } else { // Parasitic Microbes - attach and drain
                hunt_success_rate *= 0.8;
            }
            
            // Environmental factors
            let toxin_level = sample_chemical(agent_pos, 4u);
            hunt_success_rate *= (1.0 - toxin_level * 0.3); // Toxins reduce efficiency
            
            // Prey escape responses
            let prey_escape_chance = 0.5; // Base escape chance
            hunt_success_rate *= (1.0 - prey_escape_chance);
            
            // Hunt success check - no size restrictions
            let hunt_success = random(seed) < hunt_success_rate;
            
            if (hunt_success) {
                // Consume prey
                let energy_gained = agents[closest_prey_index].energy * 0.4;
                agents[agent_index].energy += energy_gained;
                
                // Create biomass from killed prey
                for (var j = 0u; j < params.max_particles; j++) {
                    if (biomass_particles[j].is_active == 0u) {
                        biomass_particles[j].position = prey_pos;
                        biomass_particles[j].biomass_amount = agents[closest_prey_index].energy * 0.3;
                        biomass_particles[j].species_origin = agents[closest_prey_index].ecological_role;
                        biomass_particles[j].decay_time = 50.0;
                        biomass_particles[j].decomposition_progress = 0.0;
                        biomass_particles[j].is_active = 1u;
                        break;
                    }
                }
                
                // Kill prey
                agents[closest_prey_index].energy = 0.0;
                
                // Update behavioral state
                agents[agent_index].behavioral_state = 1u; // Hunting state
                agents[agent_index].state_timer = 0.0;
                
                // Store hunt target for coordination
                agents[agent_index].hunt_target_id = closest_prey_index;
                
                // Energy cost for successful hunt
                agents[agent_index].energy -= 5.0;
            } else {
                // Failed hunt - trigger prey escape response
                agents[closest_prey_index].behavioral_state = 3u; // Escaping state
                agents[closest_prey_index].state_timer = 10.0; // Escape for 10 time units
                
                // Energy cost for failed hunt
                agents[agent_index].energy -= 2.0;
            }
        }
    }
}

// Deposit chemicals based on secretion rates
fn deposit_chemicals(agent_index: u32) {
    let agent_pos = agents[agent_index].position;
    
    for (var i = 0u; i < 6u; i++) {
        let secretion_rate = agents[agent_index].chemical_secretion_rates[i];
        if (secretion_rate != 0.0) {
            deposit_chemical(agent_pos, i, secretion_rate * params.chemical_deposition_rate * params.dt);
        }
    }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.agent_count) {
        return;
    }
    
    var seed = params.random_seed + index + u32(params.time * 1000.0);
    
    // Skip dead agents
    if (agents[index].energy <= 0.0) {
        return;
    }
    
    // Update sensors
    update_sensors(index, &seed);
    
    // Update movement
    update_movement(index, &seed);
    
    // Update ecological behavior
    update_ecological_behavior(index, &seed);
    
    // Update position
    agents[index].position += agents[index].velocity * params.dt;
    
    // Handle boundaries
    if (params.wrap_edges != 0u) {
        if (agents[index].position.x < -1.0) { agents[index].position.x += 2.0; }
        if (agents[index].position.x > 1.0) { agents[index].position.x -= 2.0; }
        if (agents[index].position.y < -1.0) { agents[index].position.y += 2.0; }
        if (agents[index].position.y > 1.0) { agents[index].position.y -= 2.0; }
    } else {
        agents[index].position.x = clamp(agents[index].position.x, -1.0, 1.0);
        agents[index].position.y = clamp(agents[index].position.y, -1.0, 1.0);
    }
    
    // Deposit chemicals
    deposit_chemicals(index);
    
    // Energy consumption based on metabolism
    let base_consumption = params.energy_consumption_rate * agents[index].metabolism_rate * params.dt;
    agents[index].energy -= base_consumption;
    
    // Age increment
    agents[index].age += params.dt;
    
    // Simple reproduction when energy is high
    if (agents[index].energy > params.reproduction_energy_threshold) {
        if (random(&seed) < params.reproduction_probability * params.dt) {
            // Create offspring nearby
    for (var i = 0u; i < params.agent_count; i++) {
                if (agents[i].energy <= 0.0) {
                    // Reuse dead agent slot
                    agents[i] = agents[index];
                    agents[i].position += vec2<f32>(
                        (random(&seed) - 0.5) * 0.1,
                        (random(&seed) - 0.5) * 0.1
                    );
                    agents[i].energy = agents[index].energy * 0.5;
                    agents[index].energy *= 0.5;
                    agents[i].age = 0.0;
                    break;
                }
            }
        }
    }
    
    // Natural death from old age
    if (agents[index].age > 200.0) {
        // Create biomass from dead agent
        for (var i = 0u; i < params.max_particles; i++) {
            if (biomass_particles[i].is_active == 0u) {
                biomass_particles[i].position = agents[index].position;
                biomass_particles[i].biomass_amount = agents[index].energy + 10.0;
                biomass_particles[i].species_origin = agents[index].ecological_role;
                biomass_particles[i].decay_time = 100.0;
                biomass_particles[i].decomposition_progress = 0.0;
                biomass_particles[i].is_active = 1u;
                break;
            }
        }
        agents[index].energy = 0.0;
    }
} 