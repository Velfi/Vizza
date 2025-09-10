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
    brownian_motion: f32,  // Brownian motion strength (0.0-1.0)
    aspect_ratio: f32,  // Screen aspect ratio for cursor distance calculation
    _pad1: u32,
    _pad2: u32,
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

// Calculate force using linear repulsion for close range
fn calculate_force(distance: f32, attraction: f32) -> f32 {
    let rmax = params.max_distance;
    let force_multiplier = params.max_force;
    let beta = params.beta;
    let min_dist = 0.001;
    let effective_distance = max(distance, min_dist);
    let beta_rmax = beta * rmax;
    if (distance < beta_rmax) {
        // Close range: linear repulsion
        return (effective_distance / beta_rmax - 1.0) * force_multiplier;
    } else if (distance <= rmax) {
        // Far range: species-specific attraction/repulsion
        return attraction * (1.0 - (1.0 + beta - 2.0 * distance / rmax) / (1.0 - beta)) * force_multiplier;
    }
    return 0.0;
}

// Wrap position around world boundaries [-1,1]
fn wrap_position(pos: vec2<f32>) -> vec2<f32> {
    if (params.wrap_edges == 0u) {
        return pos;
    }
    
    // Proper modulo wrapping for [-1,1] space
    var wrapped_x = pos.x;
    var wrapped_y = pos.y;
    
    // Map to [0,2] space, wrap, then map back to [-1,1]
    wrapped_x = wrapped_x + 1.0; // [-1,1] -> [0,2]
    wrapped_x = wrapped_x - floor(wrapped_x / 2.0) * 2.0; // wrap in [0,2]
    wrapped_x = wrapped_x - 1.0; // [0,2] -> [-1,1]
    
    wrapped_y = wrapped_y + 1.0; // [-1,1] -> [0,2]
    wrapped_y = wrapped_y - floor(wrapped_y / 2.0) * 2.0; // wrap in [0,2]
    wrapped_y = wrapped_y - 1.0; // [0,2] -> [-1,1]
    
    return vec2<f32>(wrapped_x, wrapped_y);
}

// Calculate shortest distance considering wrapping in world coordinates [-1,1]
fn wrapped_distance(pos_a: vec2<f32>, pos_b: vec2<f32>) -> vec2<f32> {
    var delta = pos_b - pos_a;
    
    if (params.wrap_edges == 1u) {
        // Find shortest distance across world boundaries in [-1,1] space
        // The world has width/height of 2.0, so half is 1.0
        if (delta.x > 1.0) {
            delta.x -= 2.0;
        } else if (delta.x < -1.0) {
            delta.x += 2.0;
        }
        
        if (delta.y > 1.0) {
            delta.y -= 2.0;
        } else if (delta.y < -1.0) {
            delta.y += 2.0;
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
        
        // Use circular cursor area (same size for X and Y)
        let cursor_size = params.cursor_size;
        
        // Check if particle is within the circular cursor area
        let distance_to_cursor_sq = dot(delta_to_cursor, delta_to_cursor);
        let normalized_distance_sq = distance_to_cursor_sq / (cursor_size * cursor_size);
        
        if (normalized_distance_sq <= 1.0) {
            let distance_to_cursor = sqrt(distance_to_cursor_sq);
            let direction_to_cursor = delta_to_cursor / distance_to_cursor;
            
            // Calculate cursor force strength based on distance (stronger when closer)
            let distance_factor = 1.0 - (distance_to_cursor / params.cursor_size);
            let cursor_force_strength = params.cursor_strength * distance_factor;
            
            // Apply force based on cursor mode (attract or repel)
            if (params.cursor_active == 1u) {
                // Attract particles to cursor with swirling effect
                force += direction_to_cursor * cursor_force_strength;
                
                // Add swirling force (tangential component) for black hole effect
                // Create perpendicular vector for tangential force
                let tangential_direction = vec2<f32>(-direction_to_cursor.y, direction_to_cursor.x);
                
                // Swirling strength increases as particles get closer to center
                let swirl_strength = cursor_force_strength * 0.8; // 80% of radial force
                
                // Apply tangential force to create circular motion
                force += tangential_direction * swirl_strength;
                
            } else if (params.cursor_active == 2u) {
                // Repel particles from cursor with swirling effect
                force -= direction_to_cursor * cursor_force_strength;
                
                // Add swirling force (tangential component) for centrifugal effect
                // Create perpendicular vector for tangential force (opposite direction from attract)
                let tangential_direction = vec2<f32>(direction_to_cursor.y, -direction_to_cursor.x);
                
                // Swirling strength increases as particles get closer to center
                let swirl_strength = cursor_force_strength * 0.8; // 80% of radial force
                
                // Apply tangential force to create circular motion (opposite direction)
                force += tangential_direction * swirl_strength;
            }
        }
    }
    
    // Apply Brownian motion (random thermal motion)
    if (params.brownian_motion > 0.0) {
        // Generate random force in random direction
        let angle = rand_f32() * 2.0 * 3.14159; // Random angle 0 to 2π
        let magnitude = rand_f32() * params.brownian_motion * params.max_force; // Random magnitude scaled by brownian_motion
        
        let brownian_force = vec2<f32>(
            cos(angle) * magnitude,
            sin(angle) * magnitude
        );
        
        force += brownian_force;
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
        // Bounce off world boundaries [-1,1]
        let world_min = -1.0;
        let world_max = 1.0;
        
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