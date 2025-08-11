struct Particle {
    position: vec2<f32>,
    age: f32,
    lut_index: u32, // 0-255 LUT stop index
    is_alive: u32, // 0=dead, 1=alive
    spawn_type: u32, // 0=autospawn, 1=brush
    pad0: u32,
    pad1: u32,
}

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
}

struct SimParams {
    autospawn_pool_size: u32, // Size of autospawn pool
    autospawn_rate: u32, // Particles per second for autospawn
    brush_pool_size: u32, // Size of brush pool
    brush_spawn_rate: u32, // Particles per second when cursor is active
    cursor_size: f32,
    cursor_x: f32,
    cursor_y: f32,
    display_mode: u32, // 0=Age, 1=Random, 2=Direction
    flow_field_resolution: u32,
    height: f32,
    mouse_button_down: u32, // 0=not held, 1=left click held, 2=right click held
    noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    noise_scale: f32,
    noise_seed: u32,
    noise_x: f32,
    noise_y: f32,
    particle_autospawn: u32, // 0=disabled, 1=enabled
    particle_lifetime: f32,
    particle_shape: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond
    particle_size: u32, // Particle size in pixels
    particle_speed: f32,
    screen_height: u32, // Screen height in pixels
    screen_width: u32, // Screen width in pixels
    time: f32,
    total_pool_size: u32, // Total pool size (autospawn + brush)
    trail_decay_rate: f32,
    trail_deposition_rate: f32,
    trail_diffusion_rate: f32,
    trail_map_height: u32,
    trail_map_width: u32,
    trail_wash_out_rate: f32,
    vector_magnitude: f32,
    width: f32,
    delta_time: f32,
    _padding_1: u32,
    _padding_2: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> flow_vectors: array<FlowVector>;
@group(0) @binding(2) var<uniform> sim_params: SimParams;
@group(0) @binding(3) var trail_map: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(4) var<storage, read> lut_data: array<u32>;
// Per-frame quotas for autospawn and brush, controlled by CPU
struct SpawnControl {
    autospawn_allowed: u32,
    brush_allowed: u32,
    autospawn_count: atomic<u32>,
    brush_count: atomic<u32>,
}
@group(0) @binding(5) var<storage, read_write> spawn_control: SpawnControl;

// O(1) bilinear sample of flow direction from a uniform grid (assumed sqrt(res) x sqrt(res))
fn sample_flow_vector(pos: vec2<f32>) -> vec2<f32> {
    let grid = f32(sqrt(f32(sim_params.flow_field_resolution)));
    let tx = clamp((pos.x + 1.0) * 0.5 * (grid - 1.0), 0.0, grid - 1.0);
    let ty = clamp((pos.y + 1.0) * 0.5 * (grid - 1.0), 0.0, grid - 1.0);
    let x0 = u32(floor(tx));
    let y0 = u32(floor(ty));
    let x1 = min(x0 + 1u, u32(grid - 1.0));
    let y1 = min(y0 + 1u, u32(grid - 1.0));
    let fx = tx - f32(x0);
    let fy = ty - f32(y0);
    let idx00 = y0 * u32(grid) + x0;
    let idx10 = y0 * u32(grid) + x1;
    let idx01 = y1 * u32(grid) + x0;
    let idx11 = y1 * u32(grid) + x1;
    let v00 = flow_vectors[idx00].direction;
    let v10 = flow_vectors[idx10].direction;
    let v01 = flow_vectors[idx01].direction;
    let v11 = flow_vectors[idx11].direction;
    let v0 = mix(v00, v10, fx);
    let v1 = mix(v01, v11, fx);
    return mix(v0, v1, fy);
}

// Convert world position to trail map coordinates
fn world_to_trail_coords(world_pos: vec2<f32>) -> vec2<f32> {
    // Convert from [-1, 1] world space to [0, 1] texture space
    let x = (world_pos.x + 1.0) * 0.5;
    let y = (world_pos.y + 1.0) * 0.5;
    return vec2<f32>(x, y);
}


// Check if a point is inside the particle shape
fn is_inside_particle_shape(offset_x: f32, offset_y: f32, radius: f32, shape: u32) -> bool {
    let normalized_x = offset_x / radius;
    let normalized_y = offset_y / radius;
    
    switch (shape) {
        case 0u: { // Circle
            let dist = sqrt(normalized_x * normalized_x + normalized_y * normalized_y);
            return dist <= 1.0;
        }
        case 1u: { // Square
            return abs(normalized_x) <= 1.0 && abs(normalized_y) <= 1.0;
        }
        case 2u: { // Triangle
            let triangle_y = normalized_y * 1.732; // Scale Y for equilateral triangle
            return abs(triangle_y) <= 1.0 - abs(normalized_x);
        }
        case 3u: { // Star (Flower)
            let angle = atan2(normalized_y, normalized_x);
            let radius_dist = sqrt(normalized_x * normalized_x + normalized_y * normalized_y);
            let star_radius = 1.0;
            let inner_radius = star_radius * 0.4;
            let points = 5.0;
            let angle_per_point = 6.28318 / points;
            let point_angle = (angle + 6.28318) % angle_per_point;
            let point_radius = mix(inner_radius, star_radius, smoothstep(0.0, angle_per_point * 0.3, point_angle));
            return radius_dist <= point_radius;
        }
        case 4u: { // Diamond
            return abs(normalized_x) + abs(normalized_y) <= 1.0;
        }
        default: {
            // Fallback to circle
            let dist = sqrt(normalized_x * normalized_x + normalized_y * normalized_y);
            return dist <= 1.0;
        }
    }
}

// Deposit trail covering the same footprint as the particle size and shape
fn deposit_trail(pos: vec2<f32>, particle_color: vec4<f32>) {
    let trail_pos = world_to_trail_coords(pos);
    let x_coord = i32(trail_pos.x * f32(sim_params.trail_map_width));
    let y_coord = i32(trail_pos.y * f32(sim_params.trail_map_height));

    // Use the on-screen particle size (pixels) as the trail footprint radius
    let particle_radius = i32(sim_params.particle_size);

    // Scan the footprint and write with a smooth radial falloff
    for (var dx = -particle_radius; dx <= particle_radius; dx++) {
        for (var dy = -particle_radius; dy <= particle_radius; dy++) {
            let x = clamp(x_coord + dx, 0, i32(sim_params.trail_map_width) - 1);
            let y = clamp(y_coord + dy, 0, i32(sim_params.trail_map_height) - 1);

            // Respect particle shape when laying trails
            if (!is_inside_particle_shape(f32(dx), f32(dy), f32(particle_radius), sim_params.particle_shape)) {
                continue;
            }

            // Distance-based falloff
            let dist = sqrt(f32(dx * dx + dy * dy));
            let max_dist = max(1.0, f32(particle_radius));
            let falloff = 1.0 - clamp(dist / max_dist, 0.0, 1.0);

            let current = textureLoad(trail_map, vec2<i32>(x, y));
            let deposition_strength = sim_params.trail_deposition_rate * falloff;
            let new_intensity = clamp(current.a + deposition_strength, 0.0, 1.0);

            var final_color = current.rgb;
            if (sim_params.trail_deposition_rate >= 0.99) {
                final_color = particle_color.rgb;
            } else {
                let total_weight = current.a + deposition_strength;
                final_color = (current.rgb * current.a + particle_color.rgb * deposition_strength) / max(total_weight, 1e-6);
            }

            textureStore(trail_map, vec2<i32>(x, y), vec4<f32>(final_color, new_intensity));
        }
    }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let particle_index = global_id.x;
    
    // Only process particles within the total pool size
    if (particle_index >= sim_params.total_pool_size) {
        return;
    }
    
    var particle = particles[particle_index];
    
    // Update age for all particles using actual delta time
    particle.age += sim_params.delta_time;
    
    // Check if particle should be reset due to age
    var should_reset = particle.age >= sim_params.particle_lifetime;
    var spawn_x: f32;
    var spawn_y: f32;
    
    // Unified spawning logic based on spawn_type
    if (particle.spawn_type == 0u) {
        // Autospawn particles
        if (sim_params.particle_autospawn == 1u && particle.is_alive == 0u) {
            // Probabilistic autospawn: expected spawns this frame = rate * dt across the pool
            let dt = sim_params.delta_time;
            let expected_spawns = f32(sim_params.autospawn_rate) * dt;
            let pool = max(1u, sim_params.autospawn_pool_size);
            // Adjust probability by estimating number of dead particles to maintain target rate as pool fills
            let expected_alive = min(f32(sim_params.autospawn_rate) * sim_params.particle_lifetime, f32(pool));
            let estimated_dead = max(1.0, f32(pool) - expected_alive);
            let p = clamp(expected_spawns / estimated_dead, 0.0, 1.0);
            // Deterministic per-frame random using a frame index seed
            let frame_idx = floor(sim_params.time / max(dt, 1e-6));
            let seed = f32(particle_index) * 2.71828 + frame_idx;
            let randv = fract(sin(seed) * 43758.5453);

            if (randv < p) {
                // Claim an autospawn ticket to cap spawns this frame
                let ticket = atomicAdd(&spawn_control.autospawn_count, 1u);
                if (ticket >= spawn_control.autospawn_allowed) {
                    // Quota exhausted: do not spawn
                    particle.position = vec2<f32>(0.0, 0.0);
                    particle.age = 0.0;
                    particle.lut_index = 0u;
                    particle.is_alive = 0u;
                    particles[particle_index] = particle;
                    return;
                }
                // Spawn at random position in world space
                let random_x = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                let random_y = f32(particle_index) * 0.5678 + sim_params.time * 0.1;
                spawn_x = fract(sin(random_x) * 43758.5453) * 2.0 - 1.0;
                spawn_y = fract(cos(random_y) * 43758.5453) * 2.0 - 1.0;
                should_reset = true; // Force spawn
            } else {
                // Don't spawn yet - keep particle dead
                particle.position = vec2<f32>(0.0, 0.0);
                particle.age = 0.0;
                particle.lut_index = 0u;
                particle.is_alive = 0u;
                particles[particle_index] = particle;
                return;
            }
        } else if (particle.is_alive == 1u) {
            // Particle is alive - continue normally
            if (should_reset) {
                // Particle is dying - zero out particle
                particle.position = vec2<f32>(0.0, 0.0);
                particle.age = 0.0;
                particle.lut_index = 0u;
                particle.is_alive = 0u;
                particles[particle_index] = particle;
                return;
            } else {
                // Particle continues normally
                spawn_x = particle.position.x;
                spawn_y = particle.position.y;
            }
        } else {
            // Autospawn disabled or particle is dead - keep dead
            particle.position = vec2<f32>(0.0, 0.0);
            particle.age = 0.0;
            particle.lut_index = 0u;
            particle.is_alive = 0u;
            particles[particle_index] = particle;
            return;
        }
    } else {
        // Brush particles (spawn_type == 1u)
        if (sim_params.mouse_button_down == 1u && particle.is_alive == 0u) {
            // Left click is held - probabilistic spawn so total matches brush_spawn_rate
            // Expected spawns this frame = rate * dt
            let dt = sim_params.delta_time;
            let expected_spawns = f32(sim_params.brush_spawn_rate) * dt;
            let pool = max(1u, sim_params.brush_pool_size);
            // Adjust probability by estimating number of dead particles to maintain target rate as pool fills
            let expected_alive = min(f32(sim_params.brush_spawn_rate) * sim_params.particle_lifetime, f32(pool));
            let estimated_dead = max(1.0, f32(pool) - expected_alive);
            let p = clamp(expected_spawns / estimated_dead, 0.0, 1.0);
            // Deterministic per-frame random for brush spawning
            let frame_idx = floor(sim_params.time / max(dt, 1e-6));
            let seed = f32(particle_index) * 3.14159 + frame_idx;
            let randv = fract(sin(seed) * 43758.5453);

            if (randv < p) {
                // Claim a brush ticket to cap spawns this frame
                let ticket = atomicAdd(&spawn_control.brush_count, 1u);
                if (ticket >= spawn_control.brush_allowed) {
                    // Quota exhausted: do not spawn
                    particle.position = vec2<f32>(0.0, 0.0);
                    particle.age = 0.0;
                    particle.lut_index = 0u;
                    particle.is_alive = 0u;
                    particles[particle_index] = particle;
                    return;
                }
                // Spawn at cursor with random offset (spray can effect)
                let radius = sim_params.cursor_size;
                let seed1 = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                let seed2 = f32(particle_index) * 0.5678 + sim_params.time * 0.05;
                let angle = fract(sin(seed1) * 43758.5453) * 2.0 * 3.14159;
                let distance = fract(cos(seed2) * 43758.5453);
                let offset_x = cos(angle) * radius * distance;
                let offset_y = sin(angle) * radius * distance;
                spawn_x = sim_params.cursor_x + offset_x;
                spawn_y = sim_params.cursor_y + offset_y;
                should_reset = true; // Force spawn
            } else {
                // Don't spawn yet - keep particle dead
                particle.position = vec2<f32>(0.0, 0.0);
                particle.age = 0.0;
                particle.lut_index = 0u;
                particle.is_alive = 0u;
                particles[particle_index] = particle;
                return;
            }
        } else if (particle.is_alive == 1u) {
            // Particle is alive - continue normally
            if (should_reset) {
                // Particle is dying - zero out particle
                particle.position = vec2<f32>(0.0, 0.0);
                particle.age = 0.0;
                particle.lut_index = 0u;
                particle.is_alive = 0u;
                particles[particle_index] = particle;
                return;
            } else {
                // Particle continues normally
                spawn_x = particle.position.x;
                spawn_y = particle.position.y;
            }
        } else {
            // Particle is dead and left click is not held - keep dead
            particle.position = vec2<f32>(0.0, 0.0);
            particle.age = 0.0;
            particle.lut_index = 0u;
            particle.is_alive = 0u;
            particles[particle_index] = particle;
            return;
        }
    }
    
    // Reset particle if needed
    if (should_reset) {
        particle.position = vec2<f32>(spawn_x, spawn_y);
        particle.age = 0.0;
        particle.is_alive = 1u; // Mark as active when spawning
    }
    
    // Set particle LUT index based on display mode (every frame to handle mode changes)
    if (sim_params.display_mode == 0u) {
        // Age mode: no per-particle index needed; compute from age when rendering
        particle.lut_index = 0u;
    } else if (sim_params.display_mode == 1u) {
        // Random mode: stable random per particle based on index
        let seed = f32(particle_index) * 0.1234;
        let random_value = fract(sin(seed) * 43758.5453);
        particle.lut_index = u32(clamp(random_value * 255.0, 0.0, 255.0));
    } else if (sim_params.display_mode == 2u) {
        // Direction mode: will set after computing direction below
        // Initialize to 0 to avoid undefined data
        particle.lut_index = 0u;
    }
    
    // Get flow direction with O(1) bilinear sample
    var direction = sample_flow_vector(particle.position);
    
    // Update particle LUT index based on display mode
    if (sim_params.display_mode == 2u) { // Direction mode
        // Set LUT index based on velocity direction
        let direction_angle = atan2(direction.y, direction.x);
        let normalized_angle = (direction_angle + 3.14159) / (2.0 * 3.14159); // Normalize to [0, 1]
        particle.lut_index = u32(clamp(normalized_angle * 255.0, 0.0, 255.0));
    }
    
    // Apply cursor interaction if active (right click destroys particles)
    if (sim_params.mouse_button_down == 2u) {
        // Right click destroy mode - destroy particles near cursor
        let cursor_pos = vec2<f32>(sim_params.cursor_x, sim_params.cursor_y);
        let to_cursor = cursor_pos - particle.position;
        let cursor_dist = length(to_cursor);
        let cursor_radius = sim_params.cursor_size; // World units
        
        if (cursor_dist < cursor_radius) {
            // Zero out particle when destroyed
            particle.position = vec2<f32>(0.0, 0.0);
            particle.age = 0.0;
            particle.lut_index = 0u;
            particle.is_alive = 0u;
            particles[particle_index] = particle;
            return;
        }
    }
    
    // Move particle along flow direction using delta time
    particle.position += direction * sim_params.particle_speed * sim_params.delta_time;
    
    // Wrap around edges
    particle.position.x = fract(particle.position.x * 0.5 + 0.5) * 2.0 - 1.0;
    particle.position.y = fract(particle.position.y * 0.5 + 0.5) * 2.0 - 1.0;
    
    // Deposit trail at particle position with LUT color based on display mode
    var trail_color_intensity = 0.0;
    
    if (sim_params.display_mode == 0u) { // Age mode
        let age_ratio = particle.age / sim_params.particle_lifetime;
        trail_color_intensity = 1.0 - age_ratio; // Younger particles = higher intensity
    } else if (sim_params.display_mode == 1u) { // Random mode
        trail_color_intensity = f32(particle.lut_index) / 255.0;
    } else if (sim_params.display_mode == 2u) { // Direction mode
        trail_color_intensity = f32(particle.lut_index) / 255.0;
    } else {
        // Fallback to age mode
        let age_ratio = particle.age / sim_params.particle_lifetime;
        trail_color_intensity = 1.0 - age_ratio;
    }
    
    let trail_color = get_lut_color(trail_color_intensity);
    // Alpha is not used by deposit_trail (intensity is computed internally), pass placeholder
    deposit_trail(particle.position, vec4<f32>(trail_color, 1.0));
    
    // No stored alpha; visual alpha is computed in render from age
    
    particles[particle_index] = particle;
} 