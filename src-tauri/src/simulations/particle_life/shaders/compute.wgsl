// Optimized Particle Life compute shader
// Based on the standalone particle-life implementation for better performance

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    species: u32,
    _pad: u32,
}

struct SimParams {
    particle_count: u32,
    species_count: u32,
    max_force: f32,
    min_distance: f32,
    max_distance: f32,
    friction: f32,
    wrap_edges: u32,
    width: f32,
    height: f32,
    random_seed: u32,
    dt: f32,  // Time step for simulation
    beta: f32,  // Transition point between repulsion and attraction zones
    cursor_x: f32,  // Cursor position in world coordinates
    cursor_y: f32,
    cursor_size: f32,  // Cursor interaction radius
    cursor_strength: f32,  // Cursor force strength
    cursor_active: u32,  // Whether cursor interaction is active (0 = inactive, 1 = attract, 2 = repel)
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
    _pad5: u32,
    _pad6: u32,
    _pad7: u32,  // Added to make struct 88 bytes (22 * 4)
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<storage, read> force_matrix: array<f32>;

// Simple random number generator
var<private> rng_state: u32;

fn init_rng(index: u32) {
    rng_state = params.random_seed + index * 1664525u + 1013904223u;
}

fn rand_u32() -> u32 {
    rng_state = rng_state * 1664525u + 1013904223u;
    return rng_state;
}

fn rand_f32() -> f32 {
    return f32(rand_u32()) / 4294967295.0;
}

fn rand_range(min_val: f32, max_val: f32) -> f32 {
    return min_val + rand_f32() * (max_val - min_val);
}

// Get force value from force matrix
fn get_force(species_a: u32, species_b: u32) -> f32 {
    let index = species_a * params.species_count + species_b;
    if (index >= arrayLength(&force_matrix)) {
        return 0.0;
    }
    return force_matrix[index];
}

// Calculate force using the exact same model as standalone version
fn calculate_force(distance: f32, attraction: f32) -> f32 {
    let rmax = params.max_distance;
    let force_multiplier = params.max_force;
    
    if (distance < params.beta * rmax) {
        // Close range: linear repulsion
        return (distance / (params.beta * rmax) - 1.0) * force_multiplier;
    } else if (distance <= rmax) {
        // Far range: species-specific attraction/repulsion
        return attraction * (1.0 - (1.0 + params.beta - 2.0 * distance / rmax) / (1.0 - params.beta)) * force_multiplier;
    }
    
    return 0.0; // No force beyond max distance
}

// Wrap position around world boundaries (-2.0 to 2.0)
// Using the same logic as standalone version
fn wrap_position(pos: vec2<f32>) -> vec2<f32> {
    if (params.wrap_edges == 0u) {
        return pos;
    }
    
    let world_size = 4.0; // -2.0 to 2.0 = 4.0 width
    let world_min = -2.0;
    
    // Proper modulo wrapping that handles negative numbers correctly
    let wrapped_x = world_min + ((pos.x - world_min) % world_size);
    let wrapped_y = world_min + ((pos.y - world_min) % world_size);
    
    return vec2<f32>(wrapped_x, wrapped_y);
}

// Calculate shortest distance considering wrapping in world coordinates
// Using the same logic as standalone version
fn wrapped_distance(pos_a: vec2<f32>, pos_b: vec2<f32>) -> vec2<f32> {
    var delta = pos_b - pos_a;
    
    if (params.wrap_edges == 1u) {
        let world_size = 4.0; // -2.0 to 2.0 = 4.0 width
        
        // Find shortest distance across world boundaries
        if (delta.x > world_size / 2.0) {
            delta.x -= world_size;
        } else if (delta.x < -world_size / 2.0) {
            delta.x += world_size;
        }
        
        if (delta.y > world_size / 2.0) {
            delta.y -= world_size;
        } else if (delta.y < -world_size / 2.0) {
            delta.y += world_size;
        }
    }
    
    return delta;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }
    
    init_rng(index);
    
    var particle = particles[index];
    var force = vec2<f32>(0.0, 0.0);
    
    // Calculate forces from all other particles
    // This is the O(n²) part - in a real implementation you'd use spatial partitioning
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == index) {
            continue;
        }
        
        let other = particles[i];
        let delta = wrapped_distance(particle.position, other.position);
        let distance_sq = dot(delta, delta);
        
        // Skip if too far (using squared distance for efficiency)
        if (distance_sq > params.max_distance * params.max_distance) {
            continue;
        }
        
        let distance = sqrt(distance_sq);
        
        // Skip if too close to avoid singularities
        if (distance < 0.001) {
            continue;
        }
        
        // Get force strength from force matrix
        let attraction = get_force(particle.species, other.species);
        
        // Calculate force magnitude using the same model as standalone
        let force_magnitude = calculate_force(distance, attraction);
        
        // Apply force in direction between particles
        let direction = delta / distance;
        force += direction * force_magnitude;
    }
    
    // Calculate cursor interaction force
    if (params.cursor_active > 0u) {
        let cursor_pos = vec2<f32>(params.cursor_x, params.cursor_y);
        let delta_to_cursor = wrapped_distance(particle.position, cursor_pos);
        let distance_to_cursor_sq = dot(delta_to_cursor, delta_to_cursor);
        let distance_to_cursor = sqrt(distance_to_cursor_sq);
        
        // Only apply cursor force if within cursor radius
        if (distance_to_cursor < params.cursor_size && distance_to_cursor > 0.001) {
            let direction_to_cursor = delta_to_cursor / distance_to_cursor;
            
            // Calculate cursor force strength based on distance (stronger when closer)
            let distance_factor = 1.0 - (distance_to_cursor / params.cursor_size);
            let cursor_force_strength = params.cursor_strength * distance_factor;
            
            // Apply force based on cursor mode (attract or repel)
            if (params.cursor_active == 1u) {
                // Attract particles to cursor
                force += direction_to_cursor * cursor_force_strength;
            } else if (params.cursor_active == 2u) {
                // Repel particles from cursor
                force -= direction_to_cursor * cursor_force_strength;
            }
        }
    }
    
    // Update velocity with force and friction
    // Using the same time stepping as standalone version
    let dt = params.dt;
    particle.velocity += force * dt;
    
    // Apply friction with proper time scaling like standalone
    particle.velocity *= pow(params.friction, dt * 60.0);
    
    // Update position with time stepping
    particle.position += particle.velocity * dt;
    
    // Handle boundary conditions
    if (params.wrap_edges == 1u) {
        particle.position = wrap_position(particle.position);
    } else {
        // Bounce off world boundaries (-2.0 to 2.0)
        let world_min = -2.0;
        let world_max = 2.0;
        
        // Handle X boundary
        if (particle.position.x < world_min) {
            particle.position.x = world_min;
            particle.velocity.x = -particle.velocity.x * 0.8;
        } else if (particle.position.x >= world_max) {
            particle.position.x = world_max - 0.001;
            particle.velocity.x = -particle.velocity.x * 0.8;
        }
        
        // Handle Y boundary
        if (particle.position.y < world_min) {
            particle.position.y = world_min;
            particle.velocity.y = -particle.velocity.y * 0.8;
        } else if (particle.position.y >= world_max) {
            particle.position.y = world_max - 0.001;
            particle.velocity.y = -particle.velocity.y * 0.8;
        }
    }
    
    particles[index] = particle;
}