// Fourth Order Runge-Kutta GPU Physics for Pellets
// 
// Pixel-Perfect Collision System with 3 Phases:
// 1. Broad Phase: Spatial filtering to eliminate distant particles
// 2. Narrow Phase: Precise circle-circle collision detection with elastic response
// 3. Overlap Resolution: Separate any overlapping particles after integration
//
// The particle_size parameter is calculated on the backend to exactly match
// the visual particle size from the vertex/fragment shaders for pixel-perfect collision.
//
// Aspect Ratio Correction: The collision system accounts for screen aspect ratio
// to ensure particles pack uniformly in all directions, matching the visual
// circular appearance created by the fragment shader's aspect ratio correction.

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    mass: f32,
    radius: f32,
    clump_id: u32,
    density: f32,
    grabbed: u32,
    _pad0: u32,
    previous_position: vec2<f32>,
}

struct PhysicsParams {
    mouse_position: vec2<f32>,
    mouse_velocity: vec2<f32>, // Mouse velocity in world units per second
    particle_count: u32,
    gravitational_constant: f32,
    energy_damping: f32,
    collision_damping: f32,
    dt: f32,
    gravity_softening: f32,
    interaction_radius: f32,
    mouse_pressed: u32,
    mouse_mode: u32,
    cursor_size: f32,
    cursor_strength: f32,
    particle_size: f32,
    aspect_ratio: f32,
    long_range_gravity_strength: f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: PhysicsParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var particle = particles[index];
    
    // Check if mouse is pressed and in attraction mode
    if (params.mouse_pressed != 0u && params.mouse_mode == 1u) {
        // Check if particle is within cursor range with aspect ratio correction
        let delta = params.mouse_position - particle.position;
        let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
        let distance_sq = dot(aspect_corrected_delta, aspect_corrected_delta);
        let cursor_radius_sq = params.cursor_size * params.cursor_size;
        
        if (distance_sq <= cursor_radius_sq) {
            // Mark particle as grabbed and store initial offset
            if (particle.grabbed == 0u) {
                // First time grabbing this particle, store the offset
                particle.previous_position = particle.position - params.mouse_position;
            }
            particle.grabbed = 1u;
        }
    } else {
        // Mouse not pressed, release particle with mouse velocity
        if (particle.grabbed != 0u) {
            // Apply mouse velocity to the particle when releasing
            // Use cursor_strength as velocity multiplier for throwing
            // Give extra boost for better throwing feel
            particle.velocity = params.mouse_velocity * params.cursor_strength * 2.0; // Doubled the multiplier
        }
        particle.grabbed = 0u;
    }
    
    // Handle grabbed particles
    if (particle.grabbed != 0u) {
        // Maintain the particle's relative position to the mouse center
        particle.position = params.mouse_position + particle.previous_position;
        
        // Keep velocity at zero while grabbed to prevent drift
        particle.velocity = vec2<f32>(0.0, 0.0);
        
        // Skip physics integration for grabbed particles, but still do collision resolution
        resolve_collisions(&particle, index);
        
        // Apply boundary conditions (toroidal wrapping)
        if (particle.position.x > 1.0) {
            particle.position.x -= 2.0;
        } else if (particle.position.x < -1.0) {
            particle.position.x += 2.0;
        }
        
        if (particle.position.y > 1.0) {
            particle.position.y -= 2.0;
        } else if (particle.position.y < -1.0) {
            particle.position.y += 2.0;
        }
        
        particles[index] = particle;
        return;
    }
    
    // Normal physics integration for non-grabbed particles
    // Fourth Order Runge-Kutta Integration
    // k1 = f(t, y)
    let k1_pos = particle.velocity;
    let k1_vel = compute_acceleration(particle, index);
    
    // k2 = f(t + dt/2, y + k1*dt/2)
    var temp_particle = particle;
    temp_particle.position = particle.position + k1_pos * (params.dt * 0.5);
    temp_particle.velocity = particle.velocity + k1_vel * (params.dt * 0.5);
    let k2_pos = temp_particle.velocity;
    let k2_vel = compute_acceleration(temp_particle, index);
    
    // k3 = f(t + dt/2, y + k2*dt/2)
    temp_particle.position = particle.position + k2_pos * (params.dt * 0.5);
    temp_particle.velocity = particle.velocity + k2_vel * (params.dt * 0.5);
    let k3_pos = temp_particle.velocity;
    let k3_vel = compute_acceleration(temp_particle, index);
    
    // k4 = f(t + dt, y + k3*dt)
    temp_particle.position = particle.position + k3_pos * params.dt;
    temp_particle.velocity = particle.velocity + k3_vel * params.dt;
    let k4_pos = temp_particle.velocity;
    let k4_vel = compute_acceleration(temp_particle, index);
    
    // Final RK4 update: y(t+dt) = y(t) + dt/6 * (k1 + 2*k2 + 2*k3 + k4)
    particle.position += (params.dt / 6.0) * (k1_pos + 2.0 * k2_pos + 2.0 * k3_pos + k4_pos);
    particle.velocity += (params.dt / 6.0) * (k1_vel + 2.0 * k2_vel + 2.0 * k3_vel + k4_vel);
    
    // Apply energy damping (reduced for better throwing)
    let damping_factor = 0.999; // Very light damping to preserve throwing momentum
    particle.velocity *= damping_factor;
    
    // Reduced density-based velocity damping for better throwing
    var nearby_count = 0u;
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == index) {
            continue;
        }
        
        let other = particles[i];
        var delta = other.position - particle.position;
        
        // Toroidal wrapping
        if (abs(delta.x) > 1.0) {
            delta.x = delta.x - sign(delta.x) * 2.0;
        }
        if (abs(delta.y) > 1.0) {
            delta.y = delta.y - sign(delta.y) * 2.0;
        }
        
        let distance_sq = dot(delta, delta);
        let particle_radius = params.particle_size;
        let nearby_radius_sq = particle_radius * particle_radius * 4.0; // 2x collision radius
        
        if (distance_sq < nearby_radius_sq) {
            nearby_count += 1u;
        }
    }
    
    // Much lighter density-based velocity damping for better throwing
    let density_factor = min(f32(nearby_count) / 10.0, 1.0); // Reduced from 6.0 to 10.0
    let velocity_damping = 1.0 - density_factor * 0.1; // Reduced from 0.3 to 0.1
    particle.velocity *= velocity_damping;
    
    // Phase 3: Overlap resolution - fix any overlapping particles after integration
    resolve_collisions(&particle, index);
    
    // Boundary conditions (toroidal wrapping)
    if (particle.position.x > 1.0) {
        particle.position.x -= 2.0;
    } else if (particle.position.x < -1.0) {
        particle.position.x += 2.0;
    }
    
    if (particle.position.y > 1.0) {
        particle.position.y -= 2.0;
    } else if (particle.position.y < -1.0) {
        particle.position.y += 2.0;
    }
    
    // Clamp velocities (increased limit for better throwing)
    let max_velocity = 5.0; // Increased from 2.0 to 5.0 for better throwing
    let velocity_magnitude = length(particle.velocity);
    if (velocity_magnitude > max_velocity) {
        particle.velocity = normalize(particle.velocity) * max_velocity;
    }
    
    particles[index] = particle;
}

fn compute_acceleration(particle: Particle, particle_index: u32) -> vec2<f32> {
    var acceleration = vec2<f32>(0.0, 0.0);
    
    // Gravitational forces
    if (params.gravitational_constant > 0.0) {
        acceleration += compute_gravitational_force(particle, particle_index);
    }
    
    // Mouse interaction
    if (params.mouse_pressed != 0u) {
        acceleration += compute_mouse_force(particle);
    }
    
    // Phase 1 & 2: Broad and narrow phase collision detection with impulse response
    acceleration += compute_collision_forces(particle, particle_index);
    
    return acceleration;
}

fn compute_gravitational_force(particle: Particle, particle_index: u32) -> vec2<f32> {
    var total_force = vec2<f32>(0.0, 0.0);
    let interaction_radius_sq = params.interaction_radius * params.interaction_radius;
    let long_range_radius_sq = 4.0; // Much larger radius for orbital motion
    
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == particle_index) {
            continue;
        }
        
        let other = particles[i];
        var delta = other.position - particle.position;
        
        // Toroidal wrapping
        if (abs(delta.x) > 1.0) {
            delta.x = delta.x - sign(delta.x) * 2.0;
        }
        if (abs(delta.y) > 1.0) {
            delta.y = delta.y - sign(delta.y) * 2.0;
        }
        
        let distance_sq = dot(delta, delta);
        
        // Skip if particles are too close (handled by collision system)
        if (distance_sq < 1e-6) {
            continue;
        }
        
        // Skip if particles are too far (beyond long-range radius)
        if (distance_sq > long_range_radius_sq) {
            continue;
        }
        
        let distance = sqrt(distance_sq);
        let softened_distance_sq = distance_sq + params.gravity_softening * params.gravity_softening;
        let softened_distance = sqrt(softened_distance_sq);
        
        // Multi-scale gravitational force
        let force_magnitude = params.gravitational_constant * particle.mass * other.mass / softened_distance_sq;
        var attenuated_force = force_magnitude;
        
        // Apply different force profiles based on distance
        if (distance_sq <= interaction_radius_sq) {
            // Local clumping force (strong, short-range)
            let distance_factor = (params.interaction_radius - distance) / params.interaction_radius;
            attenuated_force = force_magnitude * max(distance_factor, 0.0) * 2.0; // Boost local force
        } else {
            // Long-range orbital force (weaker, but extends much further)
            let long_range_factor = 1.0 - (distance - sqrt(interaction_radius_sq)) / (sqrt(long_range_radius_sq) - sqrt(interaction_radius_sq));
            attenuated_force = force_magnitude * max(long_range_factor, 0.0) * params.long_range_gravity_strength; // Use parameter
        }
        
        let force_direction = delta / softened_distance;
        total_force += force_direction * attenuated_force;
    }
    
    return total_force / particle.mass;
}

fn compute_mouse_force(particle: Particle) -> vec2<f32> {
    let delta = params.mouse_position - particle.position;
    
    // Apply aspect ratio correction to match visual rendering
    let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
    let distance = length(aspect_corrected_delta);
    
    if (distance > params.cursor_size || distance < 1e-6) {
        return vec2<f32>(0.0, 0.0);
    }
    
    let force_strength = params.cursor_strength * (1.0 - distance / params.cursor_size);
    let force_direction = normalize(delta); // Use original delta for force direction
    
    // Only attraction mode (mode 1) is supported
    if (params.mouse_mode == 1u) {
        return force_direction * force_strength;
    }
    
    return vec2<f32>(0.0, 0.0);
}

// Pixel-perfect collision detection with 3-phase system
fn compute_collision_forces(particle: Particle, particle_index: u32) -> vec2<f32> {
    var collision_impulse = vec2<f32>(0.0, 0.0);
    // Use the pre-calculated particle size that matches visual size exactly
    let particle_radius = params.particle_size;
    
    // Count nearby particles for density-based damping
    var nearby_count = 0u;
    var total_impulse = vec2<f32>(0.0, 0.0);
    
    // Phase 1: Broad phase - spatial filtering
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == particle_index) {
            continue;
        }
        
        let other = particles[i];
        var delta = other.position - particle.position;
        
        // Toroidal wrapping
        if (abs(delta.x) > 1.0) {
            delta.x = delta.x - sign(delta.x) * 2.0;
        }
        if (abs(delta.y) > 1.0) {
            delta.y = delta.y - sign(delta.y) * 2.0;
        }
        
        // Apply aspect ratio correction to match visual rendering
        let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
        let distance_sq = dot(aspect_corrected_delta, aspect_corrected_delta);
        let combined_radius = particle_radius + particle_radius; // Both particles use same radius
        let collision_distance_sq = combined_radius * combined_radius;
        
        // Count particles within 2x collision radius for density calculation
        if (distance_sq < collision_distance_sq * 4.0) {
            nearby_count += 1u;
        }
        
        // Broad phase: Skip if particles are definitely not colliding
        if (distance_sq > collision_distance_sq * 1.1) { // Small buffer for broad phase
            continue;
        }
        
        // Phase 2: Narrow phase - precise collision detection
        let distance = sqrt(distance_sq);
        
        // Pixel-perfect collision: check if circles actually overlap
        if (distance < combined_radius && distance > 1e-6) {
            // Collision detected - compute elastic collision response
            let collision_normal = normalize(aspect_corrected_delta);
            
            // Relative velocity
            let relative_velocity = particle.velocity - other.velocity;
            let velocity_along_normal = dot(relative_velocity, collision_normal);
            
            // Do not resolve if velocities are separating
            if (velocity_along_normal > 0.0) {
                continue;
            }
            
            // Calculate standard elastic collision impulse
            var impulse_magnitude = -2.0 * velocity_along_normal;
            impulse_magnitude = impulse_magnitude / (1.0 / particle.mass + 1.0 / other.mass);
            
            // Apply collision damping as energy retention factor (like energy_damping)
            // Higher values = more energy retained, lower values = more energy lost
            // Now works just like energy_damping: multiply by damping factor
            // Reduced damping for better throwing
            impulse_magnitude *= 0.95; // Reduced from params.collision_damping to 0.95
            
            let impulse = collision_normal * impulse_magnitude;
            total_impulse += impulse / particle.mass;
        }
    }
    
    // Apply density-based damping to reduce chaotic motion in clumps
    // Reduced damping for better throwing
    let density_factor = min(f32(nearby_count) / 12.0, 1.0); // Increased from 8.0 to 12.0
    let damping_factor = 1.0 - density_factor * 0.2; // Reduced from 0.5 to 0.2
    
    collision_impulse = total_impulse * damping_factor;
    
    return collision_impulse;
} 

// Phase 3: Overlap resolution - separate overlapping particles with stability improvements
fn resolve_collisions(particle: ptr<function, Particle>, particle_index: u32) {
    let particle_radius = params.particle_size; // Use unified particle size
    
    // Count nearby particles for density-based overlap handling
    var nearby_count = 0u;
    var total_overlap = 0.0;
    var overlap_direction = vec2<f32>(0.0, 0.0);
    
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == particle_index) {
            continue;
        }
        
        let other = particles[i];
        var delta = (*particle).position - other.position;
        
        // Toroidal wrapping
        if (abs(delta.x) > 1.0) {
            delta.x = delta.x - sign(delta.x) * 2.0;
        }
        if (abs(delta.y) > 1.0) {
            delta.y = delta.y - sign(delta.y) * 2.0;
        }
        
        // Apply aspect ratio correction to match visual rendering
        let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
        let distance_sq = dot(aspect_corrected_delta, aspect_corrected_delta);
        let combined_radius = particle_radius + particle_radius; // Both particles use same radius
        let distance = sqrt(distance_sq);
        
        // Count nearby particles for density calculation
        if (distance_sq < combined_radius * combined_radius * 4.0) {
            nearby_count += 1u;
        }
        
        // Check for overlap (pixel-perfect)
        if (distance < combined_radius && distance > 1e-6) {
            let overlap = combined_radius - distance;
            
            if (overlap > 0.0) {
                total_overlap += overlap;
                
                // Calculate separation vector (convert back to world space)
                let separation_direction = normalize(aspect_corrected_delta);
                let world_separation_direction = vec2<f32>(
                    separation_direction.x / params.aspect_ratio, 
                    separation_direction.y
                );
                
                overlap_direction += world_separation_direction * overlap;
            }
        }
    }
    
    // Apply gentler separation based on density
    if (total_overlap > 0.0 && nearby_count > 0u) {
        let density_factor = min(f32(nearby_count) / 12.0, 1.0); // Normalize to 0-1
        let separation_strength = 0.3 * (1.0 - density_factor * 0.7); // Gentler in dense areas
        
        // Normalize and apply separation
        let separation_magnitude = length(overlap_direction);
        if (separation_magnitude > 1e-6) {
            let normalized_direction = overlap_direction / separation_magnitude;
            let separation_distance = total_overlap * separation_strength;
            (*particle).position += normalized_direction * separation_distance;
        }
    }
} 