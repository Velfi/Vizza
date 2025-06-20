// Classic Particle Life compute shader
// Simple force-based attraction/repulsion model

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
    time_step: f32,
    wrap_edges: u32,
    width: f32,
    height: f32,
    random_seed: u32,
    repulsion_min_distance: f32,
    repulsion_medium_distance: f32,
    repulsion_extreme_strength: f32,
    repulsion_linear_strength: f32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
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

// Calculate classic particle life force
fn calculate_force(r: f32, force_strength: f32) -> f32 {
    // Very short range repulsion to prevent overlap
    if (r < params.repulsion_min_distance) {
        return params.repulsion_extreme_strength;
    }
    
    // Medium range repulsion
    if (r < params.repulsion_medium_distance) {
        let t = (params.repulsion_medium_distance - r) / (params.repulsion_medium_distance - params.repulsion_min_distance);
        return params.repulsion_linear_strength * t;
    }
    
    // Outside minimum distance, apply the force matrix value
    if (r > params.max_distance) {
        return 0.0; // No force beyond max distance
    }
    
    // Linear falloff from max_distance to min_distance
    let distance_factor = 1.0 - (r - params.min_distance) / (params.max_distance - params.min_distance);
    let clamped_factor = clamp(distance_factor, 0.0, 1.0);
    
    return force_strength * clamped_factor;
}

// Wrap position around screen boundaries
fn wrap_position(pos: vec2<f32>) -> vec2<f32> {
    if (params.wrap_edges == 0u) {
        return pos;
    }
    
    var wrapped = pos;
    if (wrapped.x < 0.0) {
        wrapped.x += params.width;
    } else if (wrapped.x >= params.width) {
        wrapped.x -= params.width;
    }
    
    if (wrapped.y < 0.0) {
        wrapped.y += params.height;
    } else if (wrapped.y >= params.height) {
        wrapped.y -= params.height;
    }
    
    return wrapped;
}

// Calculate shortest distance considering wrapping
fn wrapped_distance(pos_a: vec2<f32>, pos_b: vec2<f32>) -> vec2<f32> {
    var delta = pos_b - pos_a;
    
    if (params.wrap_edges == 1u) {
        let half_width = params.width * 0.5;
        let half_height = params.height * 0.5;
        
        if (abs(delta.x) > half_width) {
            if (delta.x > 0.0) {
                delta.x -= params.width;
            } else {
                delta.x += params.width;
            }
        }
        
        if (abs(delta.y) > half_height) {
            if (delta.y > 0.0) {
                delta.y -= params.height;
            } else {
                delta.y += params.height;
            }
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
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == index) {
            continue;
        }
        
        let other = particles[i];
        let delta = wrapped_distance(particle.position, other.position);
        let distance = length(delta);
        
        // Skip if too far
        if (distance > params.max_distance) {
            continue;
        }
        
        // Get force strength from force matrix
        let force_strength = get_force(particle.species, other.species);
        
        // Calculate force magnitude using classic particle life model
        let force_magnitude = calculate_force(distance, force_strength);
        
        // Limit maximum force to prevent instability
        let clamped_force = clamp(force_magnitude, -params.max_force, params.max_force);
        
        // Apply force in direction between particles
        if (distance > 0.001) {
            let direction = normalize(delta);
            force += direction * clamped_force;
        }
    }
    
    // Update velocity with force and friction
    particle.velocity += force * params.time_step;
    particle.velocity *= params.friction;
    
    // Clamp velocity to prevent extreme values
    let max_velocity = 1000.0;
    let velocity_magnitude = length(particle.velocity);
    if (velocity_magnitude > max_velocity) {
        particle.velocity = normalize(particle.velocity) * max_velocity;
    }
    
    // Update position
    particle.position += particle.velocity * params.time_step;
    
    // Handle boundary conditions
    if (params.wrap_edges == 1u) {
        particle.position = wrap_position(particle.position);
    } else {
        // Bounce off walls
        if (particle.position.x < 0.0 || particle.position.x >= params.width) {
            particle.velocity.x *= -0.8;
            particle.position.x = clamp(particle.position.x, 0.0, params.width - 1.0);
        }
        if (particle.position.y < 0.0 || particle.position.y >= params.height) {
            particle.velocity.y *= -0.8;
            particle.position.y = clamp(particle.position.y, 0.0, params.height - 1.0);
        }
    }
    
    particles[index] = particle;
}