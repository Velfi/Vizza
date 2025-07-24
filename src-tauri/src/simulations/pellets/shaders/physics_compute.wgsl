// Fourth Order Runge-Kutta GPU Physics for Pellets with Spatial Partitioning
// 
// Pixel-Perfect Collision System with 3 Phases:
// 1. Broad Phase: Spatial grid filtering to eliminate distant particles
// 2. Narrow Phase: Precise circle-circle collision detection with elastic response
// 3. Overlap Resolution: Separate any overlapping particles after integration
//
// Spatial Partitioning: Uses a uniform grid to reduce O(n²) complexity to O(n)
// for neighbor lookups, dramatically improving performance with large particle counts.

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
    mouse_velocity: vec2<f32>,
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
    density_damping_enabled: u32,
    overlap_resolution_strength: f32,
}

struct GridParams {
    particle_count: u32,
    grid_width: u32,
    grid_height: u32,
    cell_size: f32,
    world_width: f32,
    world_height: f32,
    _pad1: u32,
    _pad2: u32,
}

struct GridCell {
    particle_count: u32,
    particle_indices: array<u32, 32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: PhysicsParams;
@group(0) @binding(2) var<storage, read> grid: array<GridCell>;
@group(0) @binding(3) var<uniform> grid_params: GridParams;

// Convert world position to grid coordinates
fn world_to_grid(pos: vec2<f32>) -> vec2<u32> {
    let normalized_pos = (pos + vec2<f32>(1.0, 1.0)) * 0.5;
    let grid_x = u32(normalized_pos.x * f32(grid_params.grid_width));
    let grid_y = u32(normalized_pos.y * f32(grid_params.grid_height));
    
    return vec2<u32>(
        min(grid_x, grid_params.grid_width - 1u),
        min(grid_y, grid_params.grid_height - 1u)
    );
}

// Get grid cell index from coordinates
fn grid_coord_to_index(coord: vec2<u32>) -> u32 {
    return coord.y * grid_params.grid_width + coord.x;
}

// Get particles from neighboring grid cells
fn get_neighbor_particles(particle_pos: vec2<f32>, particle_index: u32) -> array<u32, 256> {
    var neighbors: array<u32, 256>;
    var neighbor_count = 0u;
    
    // Get the grid cell this particle is in
    let center_cell = world_to_grid(particle_pos);
    
    // Check 3x3 neighborhood of cells
    for (var dy = -1i; dy <= 1i; dy++) {
        for (var dx = -1i; dx <= 1i; dx++) {
            let cell_x = i32(center_cell.x) + dx;
            let cell_y = i32(center_cell.y) + dy;
            
            // Handle toroidal wrapping for grid cells
            let wrapped_x = (cell_x + i32(grid_params.grid_width)) % i32(grid_params.grid_width);
            let wrapped_y = (cell_y + i32(grid_params.grid_height)) % i32(grid_params.grid_height);
            
            let cell_index = grid_coord_to_index(vec2<u32>(u32(wrapped_x), u32(wrapped_y)));
            let cell = grid[cell_index];
            
            // Add particles from this cell
            for (var i = 0u; i < cell.particle_count; i++) {
                let neighbor_index = cell.particle_indices[i];
                
                // Skip self
                if (neighbor_index == particle_index) {
                    continue;
                }
                
                // Add to neighbors if we have space
                if (neighbor_count < 256u) {
                    neighbors[neighbor_count] = neighbor_index;
                    neighbor_count += 1u;
                }
            }
        }
    }
    
    // Pad remaining slots with invalid index
    for (var i = neighbor_count; i < 256u; i++) {
        neighbors[i] = 0xFFFFFFFFu; // Invalid index
    }
    
    return neighbors;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var particle = particles[index];
    
    // Check if mouse is pressed and in attraction mode
    if (params.mouse_pressed != 0u && params.mouse_mode == 1u) {
        let delta = params.mouse_position - particle.position;
        let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
        let distance_sq = dot(aspect_corrected_delta, aspect_corrected_delta);
        let cursor_radius_sq = params.cursor_size * params.cursor_size;
        
        if (distance_sq <= cursor_radius_sq) {
            if (particle.grabbed == 0u) {
                particle.previous_position = particle.position - params.mouse_position;
            }
            particle.grabbed = 1u;
        }
    } else {
        if (particle.grabbed != 0u) {
            particle.velocity = params.mouse_velocity * params.cursor_strength * 2.0;
        }
        particle.grabbed = 0u;
    }
    
    // Handle grabbed particles
    if (particle.grabbed != 0u) {
        particle.position = params.mouse_position + particle.previous_position;
        particle.velocity = vec2<f32>(0.0, 0.0);
        
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
    let k1_pos = particle.velocity;
    let k1_vel = compute_acceleration(particle, index);
    
    var temp_particle = particle;
    temp_particle.position = particle.position + k1_pos * (params.dt * 0.5);
    temp_particle.velocity = particle.velocity + k1_vel * (params.dt * 0.5);
    let k2_pos = temp_particle.velocity;
    let k2_vel = compute_acceleration(temp_particle, index);
    
    temp_particle.position = particle.position + k2_pos * (params.dt * 0.5);
    temp_particle.velocity = particle.velocity + k2_vel * (params.dt * 0.5);
    let k3_pos = temp_particle.velocity;
    let k3_vel = compute_acceleration(temp_particle, index);
    
    temp_particle.position = particle.position + k3_pos * params.dt;
    temp_particle.velocity = particle.velocity + k3_vel * params.dt;
    let k4_pos = temp_particle.velocity;
    let k4_vel = compute_acceleration(temp_particle, index);
    
    particle.position += (params.dt / 6.0) * (k1_pos + 2.0 * k2_pos + 2.0 * k3_pos + k4_pos);
    particle.velocity += (params.dt / 6.0) * (k1_vel + 2.0 * k2_vel + 2.0 * k3_vel + k4_vel);
    
    // Apply energy damping
    let damping_factor = 0.999;
    particle.velocity *= damping_factor;
    
    // Density-based velocity damping - check all particles for accurate density
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
        let nearby_radius_sq = particle_radius * particle_radius * 4.0;
        
        if (distance_sq < nearby_radius_sq) {
            nearby_count += 1u;
        }
    }
    
    // Apply density-based velocity damping only if enabled
    if (params.density_damping_enabled != 0u) {
        let density_factor = min(f32(nearby_count) / 10.0, 1.0);
        let velocity_damping = 1.0 - density_factor * 0.1;
        particle.velocity *= velocity_damping;
    }
    
    // Overlap resolution
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
    
    // Clamp velocities
    let max_velocity = 5.0;
    let velocity_magnitude = length(particle.velocity);
    if (velocity_magnitude > max_velocity) {
        particle.velocity = normalize(particle.velocity) * max_velocity;
    }
    
    particles[index] = particle;
}

fn compute_acceleration(particle: Particle, particle_index: u32) -> vec2<f32> {
    var acceleration = vec2<f32>(0.0, 0.0);
    
    // Gravitational forces using spatial grid
    if (params.gravitational_constant > 0.0) {
        acceleration += compute_gravitational_force(particle, particle_index);
    }
    
    // Mouse interaction
    if (params.mouse_pressed != 0u) {
        acceleration += compute_mouse_force(particle);
    }
    
    // Collision forces using spatial grid
    acceleration += compute_collision_forces(particle, particle_index);
    
    return acceleration;
}

fn compute_gravitational_force(particle: Particle, particle_index: u32) -> vec2<f32> {
    var total_force = vec2<f32>(0.0, 0.0);
    let interaction_radius_sq = params.interaction_radius * params.interaction_radius;
    let long_range_radius_sq = 4.0;
    
    // Use spatial grid for neighbor lookup
    let neighbors = get_neighbor_particles(particle.position, particle_index);
    
    for (var i = 0u; i < 256u; i++) {
        let neighbor_index = neighbors[i];
        if (neighbor_index == 0xFFFFFFFFu) {
            break;
        }
        
        let other = particles[neighbor_index];
        var delta = other.position - particle.position;
        
        // Toroidal wrapping
        if (abs(delta.x) > 1.0) {
            delta.x = delta.x - sign(delta.x) * 2.0;
        }
        if (abs(delta.y) > 1.0) {
            delta.y = delta.y - sign(delta.y) * 2.0;
        }
        
        let distance_sq = dot(delta, delta);
        
        if (distance_sq < 1e-6) {
            continue;
        }
        
        if (distance_sq > long_range_radius_sq) {
            continue;
        }
        
        let distance = sqrt(distance_sq);
        let softened_distance_sq = distance_sq + params.gravity_softening * params.gravity_softening;
        let softened_distance = sqrt(softened_distance_sq);
        
        let force_magnitude = params.gravitational_constant * particle.mass * other.mass / softened_distance_sq;
        var attenuated_force = force_magnitude;
        
        if (distance_sq <= interaction_radius_sq) {
            let distance_factor = (params.interaction_radius - distance) / params.interaction_radius;
            attenuated_force = force_magnitude * max(distance_factor, 0.0) * 2.0;
        } else {
            let long_range_factor = 1.0 - (distance - sqrt(interaction_radius_sq)) / (sqrt(long_range_radius_sq) - sqrt(interaction_radius_sq));
            attenuated_force = force_magnitude * max(long_range_factor, 0.0) * params.long_range_gravity_strength;
        }
        
        let force_direction = delta / softened_distance;
        total_force += force_direction * attenuated_force;
    }
    
    return total_force / particle.mass;
}

fn compute_mouse_force(particle: Particle) -> vec2<f32> {
    let delta = params.mouse_position - particle.position;
    let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
    let distance = length(aspect_corrected_delta);
    
    if (distance > params.cursor_size || distance < 1e-6) {
        return vec2<f32>(0.0, 0.0);
    }
    
    let force_strength = params.cursor_strength * (1.0 - distance / params.cursor_size);
    let force_direction = normalize(delta);
    
    if (params.mouse_mode == 1u) {
        return force_direction * force_strength;
    }
    
    return vec2<f32>(0.0, 0.0);
}

fn compute_collision_forces(particle: Particle, particle_index: u32) -> vec2<f32> {
    var collision_impulse = vec2<f32>(0.0, 0.0);
    let particle_radius = params.particle_size;
    
    var nearby_count = 0u;
    var total_impulse = vec2<f32>(0.0, 0.0);
    
    // For pixel-perfect collision, check ALL particles
    // The grid is used for gravitational forces, but collision needs to be exact
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
        
        let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
        let distance_sq = dot(aspect_corrected_delta, aspect_corrected_delta);
        let combined_radius = particle_radius + particle_radius;
        let collision_distance_sq = combined_radius * combined_radius;
        
        if (distance_sq < collision_distance_sq * 4.0) {
            nearby_count += 1u;
        }
        
        if (distance_sq > collision_distance_sq * 1.1) {
            continue;
        }
        
        let distance = sqrt(distance_sq);
        
        if (distance < combined_radius && distance > 1e-6) {
            let collision_normal = normalize(aspect_corrected_delta);
            let relative_velocity = particle.velocity - other.velocity;
            let velocity_along_normal = dot(relative_velocity, collision_normal);
            
            if (velocity_along_normal > 0.0) {
                continue;
            }
            
            var impulse_magnitude = -2.0 * velocity_along_normal;
            impulse_magnitude = impulse_magnitude / (1.0 / particle.mass + 1.0 / other.mass);
            // Apply collision damping from settings
            impulse_magnitude *= params.collision_damping;
            
            let impulse = collision_normal * impulse_magnitude;
            total_impulse += impulse / particle.mass;
        }
    }
    
    // Remove density-based damping for collision impulses to maintain bounce
    collision_impulse = total_impulse;
    
    return collision_impulse;
}

fn resolve_collisions(particle: ptr<function, Particle>, particle_index: u32) {
    let particle_radius = params.particle_size;
    
    var nearby_count = 0u;
    var total_overlap = 0.0;
    var overlap_direction = vec2<f32>(0.0, 0.0);
    
    // For pixel-perfect collision resolution, check ALL particles
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
        
        let aspect_corrected_delta = vec2<f32>(delta.x * params.aspect_ratio, delta.y);
        let distance_sq = dot(aspect_corrected_delta, aspect_corrected_delta);
        let combined_radius = particle_radius + particle_radius;
        let distance = sqrt(distance_sq);
        
        if (distance_sq < combined_radius * combined_radius * 4.0) {
            nearby_count += 1u;
        }
        
        if (distance < combined_radius && distance > 1e-6) {
            let overlap = combined_radius - distance;
            
            if (overlap > 0.0) {
                total_overlap += overlap;
                
                let separation_direction = normalize(aspect_corrected_delta);
                let world_separation_direction = vec2<f32>(
                    separation_direction.x / params.aspect_ratio, 
                    separation_direction.y
                );
                
                overlap_direction += world_separation_direction * overlap;
            }
        }
    }
    
    if (total_overlap > 0.0 && nearby_count > 0u) {
        // Use user-controlled overlap resolution strength
        let density_factor = min(f32(nearby_count) / 12.0, 1.0);
        let separation_strength = params.overlap_resolution_strength * (1.0 - density_factor * 0.3);
        
        let separation_magnitude = length(overlap_direction);
        if (separation_magnitude > 1e-6) {
            let normalized_direction = overlap_direction / separation_magnitude;
            let separation_distance = total_overlap * separation_strength;
            (*particle).position += normalized_direction * separation_distance;
        }
    }
} 