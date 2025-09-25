// Primordial Particles Update Shader
// Implements the PPS motion law with signed turn: Δφ = sgn(R-L) * (α + β * N_t,r)
// Based on "How a life-like system emerges from a simplistic particle motion law"

const PI: f32 = 3.14159265359;

struct Particle {
    position: vec2<f32>,
    previous_position: vec2<f32>,
    heading: f32,
    velocity: f32, // Magnitude of velocity for coloring
    density: f32,  // Local density for coloring
    grabbed: u32,
}

struct SimParams {
    // Mouse interaction parameters (vec2 fields first for alignment)
    mouse_position: vec2<f32>,
    mouse_velocity: vec2<f32>,
    
    alpha: f32,        // Fixed rotation parameter (radians)
    beta: f32,         // Proportional rotation parameter
    velocity: f32,     // Constant velocity
    radius: f32,       // Interaction radius
    
    dt: f32,           // Time step
    width: f32,        // World width
    height: f32,       // World height
    wrap_edges: u32,   // 1 if wrapping edges, 0 otherwise
    
    particle_count: u32,
    mouse_pressed: u32,
    mouse_mode: u32,
    cursor_size: f32,
    cursor_strength: f32,
    aspect_ratio: f32,
    _pad1: f32,
    _pad0: f32,
}

@group(0) @binding(0)
var<storage, read> particles_in: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> particles_out: array<Particle>;

@group(0) @binding(2)
var<uniform> sim_params: SimParams;

// Calculate distance between two points with optional wrapping
fn distance_with_wrapping(p1: vec2<f32>, p2: vec2<f32>) -> f32 {
    var dx = p2.x - p1.x;
    var dy = p2.y - p1.y;
    
    if (sim_params.wrap_edges == 1u) {
        // Wrap around world boundaries
        if (dx > sim_params.width * 0.5) {
            dx -= sim_params.width;
        } else if (dx < -sim_params.width * 0.5) {
            dx += sim_params.width;
        }
        
        if (dy > sim_params.height * 0.5) {
            dy -= sim_params.height;
        } else if (dy < -sim_params.height * 0.5) {
            dy += sim_params.height;
        }
    }
    
    return sqrt(dx * dx + dy * dy);
}

// Count particles in left and right semicircles
fn count_neighbors(particle_index: u32) -> vec2<u32> {
    let current_particle = particles_in[particle_index];
    var left_count: u32 = 0u;
    var right_count: u32 = 0u;
    
    for (var i = 0u; i < sim_params.particle_count; i++) {
        if (i == particle_index) {
            continue;
        }
        
        let other_particle = particles_in[i];
        let dist = distance_with_wrapping(current_particle.position, other_particle.position);
        
        if (dist <= sim_params.radius) {
            // Calculate relative position vector
            var dx = other_particle.position.x - current_particle.position.x;
            var dy = other_particle.position.y - current_particle.position.y;
            
            if (sim_params.wrap_edges == 1u) {
                // Apply same wrapping logic as distance calculation
                if (dx > sim_params.width * 0.5) {
                    dx -= sim_params.width;
                } else if (dx < -sim_params.width * 0.5) {
                    dx += sim_params.width;
                }
                
                if (dy > sim_params.height * 0.5) {
                    dy -= sim_params.height;
                } else if (dy < -sim_params.height * 0.5) {
                    dy += sim_params.height;
                }
            }
            
            // Determine if particle is on left or right side
            // Use cross product to determine side relative to current heading
            let relative_angle = atan2(dy, dx) - current_particle.heading;
            let normalized_angle = ((relative_angle + PI) % (2.0 * PI)) - PI;
            
            if (normalized_angle > 0.0) {
                right_count++;
            } else {
                left_count++;
            }
        }
    }
    
    return vec2<u32>(left_count, right_count);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let particle_index = global_id.x;
    
    if (particle_index >= sim_params.particle_count) {
        return;
    }
    
    var particle = particles_in[particle_index];
    
    // Use per-particle speed; initialize to baseline if unset
    var speed = particle.velocity;
    if (speed <= 0.0) {
        speed = sim_params.velocity;
    }
    
    // Prepare a velocity-space cursor force like Particle Life (affects velocity, not direct steering)
    var cursor_force: vec2<f32> = vec2<f32>(0.0, 0.0);
    var cursor_active: bool = false;
    if (sim_params.mouse_pressed != 0u && sim_params.cursor_size > 0.0) {
        let to_cursor = sim_params.mouse_position - particle.position;
        let aspect_corrected = vec2<f32>(to_cursor.x * sim_params.aspect_ratio, to_cursor.y);
        let distance = length(aspect_corrected);
        if (distance <= sim_params.cursor_size && distance > 1e-6) {
            let distance_factor = 1.0 - (distance / sim_params.cursor_size);
            var dir = normalize(vec2<f32>(to_cursor.x, to_cursor.y));
            if (sim_params.mouse_mode == 2u) { // repel
                dir = -dir;
            }
            // magnitude scaled by proximity and cursor strength (boosted)
            let mag = sim_params.cursor_strength * distance_factor * 4.0;
            cursor_force = dir * mag;
            cursor_active = true;
        }
    }
    
    // Count neighbors in left and right semicircles
    let neighbor_counts = count_neighbors(particle_index);
    let left_count = neighbor_counts.x;
    let right_count = neighbor_counts.y;
    let total_neighbors = left_count + right_count;
    
    // Determine turn direction based on side counts
    var turn_dir: f32 = 1.0; // tie-breaker to the right to preserve alpha effect
    if (right_count < left_count) {
        turn_dir = -1.0;
    } else if (right_count > left_count) {
        turn_dir = 1.0;
    }

    // Apply PPS motion law with sign: Δφ = sgn(R-L) * (α + β * N_t,r)
    let delta_phi_mag = sim_params.alpha + sim_params.beta * f32(total_neighbors);
    particle.heading = (particle.heading + turn_dir * delta_phi_mag * sim_params.dt) % (2.0 * PI);

    // Convert heading/speed to velocity vector and apply cursor force as acceleration
    var v = vec2<f32>(cos(particle.heading) * speed, sin(particle.heading) * speed);
    if (cursor_active) {
        // Increased acceleration scaling for stronger interaction
        let accel_scale = 10.0 * sim_params.dt;
        v += cursor_force * accel_scale;
    }
    // Recompute speed and heading from updated velocity vector
    let new_speed = length(v);
    if (new_speed > 1e-6) {
        speed = new_speed;
        particle.heading = atan2(v.y, v.x);
    }
    
    // Decay speed back toward baseline when not actively influenced
    if (!cursor_active) {
        let t = clamp(0.25 * sim_params.dt, 0.0, 1.0);
        speed = speed + (sim_params.velocity - speed) * t;
    }
    
    // Update position based on heading and per-particle speed
    let dx = cos(particle.heading) * speed * sim_params.dt;
    let dy = sin(particle.heading) * speed * sim_params.dt;
    
    particle.position.x += dx;
    particle.position.y += dy;
    
    // Apply edge wrapping if enabled (using [-1,1] world space)
    if (sim_params.wrap_edges == 1u) {
        if (particle.position.x < -1.0) {
            particle.position.x += 2.0; // wrap from -1 to +1
        } else if (particle.position.x > 1.0) {
            particle.position.x -= 2.0; // wrap from +1 to -1
        }
        
        if (particle.position.y < -1.0) {
            particle.position.y += 2.0; // wrap from -1 to +1
        } else if (particle.position.y > 1.0) {
            particle.position.y -= 2.0; // wrap from +1 to -1
        }
    } else {
        // Clamp to boundaries if not wrapping ([-1,1] world space)
        particle.position.x = clamp(particle.position.x, -1.0, 1.0);
        particle.position.y = clamp(particle.position.y, -1.0, 1.0);
    }
    
    // Store current speed for coloring/next step
    particle.velocity = speed;
    
    // Write final particle state to output buffer
    particles_out[particle_index] = particle;
}


