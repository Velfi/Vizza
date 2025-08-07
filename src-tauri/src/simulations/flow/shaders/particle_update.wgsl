struct Particle {
    position: vec2<f32>,
    age: f32,
    color: vec4<f32>,
    is_alive: u32, // 0=dead, 1=alive
    spawn_type: u32, // 0=autospawn, 1=brush
    last_spawn_time: f32, // Track when this particle last spawned
}

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
}

struct SimParams {
    total_pool_size: u32, // Total number of particles (autospawn + brush)
    vector_count: u32,
    particle_lifetime: f32,
    particle_speed: f32,
    noise_seed: u32,
    time: f32,
    noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    width: f32,
    height: f32,
    noise_scale: f32,
    noise_x: f32,
    noise_y: f32,
    vector_magnitude: f32,
    trail_decay_rate: f32,
    trail_deposition_rate: f32,
    trail_diffusion_rate: f32,
    trail_wash_out_rate: f32,
    trail_map_width: u32,
    trail_map_height: u32,
    particle_shape: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond
    particle_size: u32, // Particle size in pixels
    screen_width: u32, // Screen width in pixels
    screen_height: u32, // Screen height in pixels
    cursor_x: f32,
    cursor_y: f32,
    cursor_size: u32,
    cursor_strength: f32,
    mouse_button_down: u32, // 0=not held, 1=left click held, 2=right click held
    particle_autospawn: u32, // 0=disabled, 1=enabled
    autospawn_rate: u32,     // Particles per second for autospawn
    brush_spawn_rate: u32,   // Particles per second when cursor is active
    display_mode: u32, // 0=Age, 1=Random, 2=Direction
    autospawn_pool_size: u32, // Size of autospawn pool
    brush_pool_size: u32,     // Size of brush pool
    total_pool_size: u32,     // Total pool size (autospawn + brush)

}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> flow_vectors: array<FlowVector>;
@group(0) @binding(2) var<uniform> sim_params: SimParams;
@group(0) @binding(3) var trail_map: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(4) var<storage, read> lut_data: array<u32>;

// Find nearest flow vector and return its direction
fn find_nearest_flow_vector(pos: vec2<f32>) -> vec2<f32> {
    var nearest_dist = 999999.0;
    var nearest_direction = vec2<f32>(0.0, 0.0);
    
    for (var i = 0u; i < sim_params.vector_count; i++) {
        let vector = flow_vectors[i];
        let dist = distance(pos, vector.position);
        if (dist < nearest_dist) {
            nearest_dist = dist;
            nearest_direction = vector.direction;
        }
    }
    
    return nearest_direction;
}

// Convert world position to trail map coordinates
fn world_to_trail_coords(world_pos: vec2<f32>) -> vec2<f32> {
    // Convert from [-1, 1] world space to [0, 1] texture space
    let x = (world_pos.x + 1.0) * 0.5;
    let y = (world_pos.y + 1.0) * 0.5;
    return vec2<f32>(x, y);
}

// Convert from sRGB (gamma-corrected) to linear RGB
fn srgb_to_linear(srgb: f32) -> f32 {
    if (srgb <= 0.04045) {
        return srgb / 12.92;
    } else {
        return pow((srgb + 0.055) / 1.055, 2.4);
    }
}

// Get color from LUT
fn get_lut_color(intensity: f32) -> vec3<f32> {
    let lut_index = clamp(intensity * 255.0, 0.0, 255.0);
    let index = u32(lut_index);
    
    // LUT data format: [r0, r1, ..., r255, g0, g1, ..., g255, b0, b1, ..., b255]
    let r_srgb = f32(lut_data[index]) / 255.0;
    let g_srgb = f32(lut_data[index + 256u]) / 255.0;
    let b_srgb = f32(lut_data[index + 512u]) / 255.0;
    
    return vec3<f32>(
        srgb_to_linear(r_srgb),
        srgb_to_linear(g_srgb),
        srgb_to_linear(b_srgb)
    );
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

// Deposit trail at particle position with particle color
fn deposit_trail(pos: vec2<f32>, particle_color: vec4<f32>) {
    let trail_pos = world_to_trail_coords(pos);
    let x_coord = i32(trail_pos.x * f32(sim_params.trail_map_width));
    let y_coord = i32(trail_pos.y * f32(sim_params.trail_map_height));
    
    // Particle size in texture coordinates - use the actual particle size setting
    let particle_radius = i32(sim_params.particle_size); // Use full particle size for deposition area
    
    // Deposit trail in a small area around the particle position
    for (var dx = -particle_radius; dx <= particle_radius; dx++) {
        for (var dy = -particle_radius; dy <= particle_radius; dy++) {
            let x = clamp(x_coord + dx, 0, i32(sim_params.trail_map_width) - 1);
            let y = clamp(y_coord + dy, 0, i32(sim_params.trail_map_height) - 1);
            
            // Check if this pixel is inside the particle shape
            if (!is_inside_particle_shape(f32(dx), f32(dy), f32(particle_radius), sim_params.particle_shape)) {
                continue;
            }
            
            // Calculate distance from center for falloff (still use circular falloff for smoothness)
            let dist = sqrt(f32(dx * dx + dy * dy));
            let max_dist = f32(particle_radius);
            
            // Get current trail value and color
            let current_trail = textureLoad(trail_map, vec2<i32>(x, y));
            let current_intensity = current_trail.a;
            let current_color = current_trail.rgb;
            
            // Calculate deposition strength based on distance falloff
            let falloff = 1.0 - dist / max_dist;
            let deposition_strength = sim_params.trail_deposition_rate * falloff;
            let new_intensity = clamp(current_intensity + deposition_strength, 0.0, 1.0);
            
            // Choose color based on deposition rate
            var final_color = current_color;
            if (sim_params.trail_deposition_rate >= 0.99) {
                // At maximum deposition rate, completely replace the color
                final_color = particle_color.rgb;
            } else {
                // Blend colors: new color weighted by deposition strength, existing color by current intensity
                let total_weight = current_intensity + deposition_strength;
                final_color = (current_color * current_intensity + particle_color.rgb * deposition_strength) / total_weight;
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
    
    // Update age for all particles
    particle.age += 0.016; // ~60 FPS
    
    // Check if particle should be reset due to age
    var should_reset = particle.age >= sim_params.particle_lifetime;
    var spawn_x: f32;
    var spawn_y: f32;
    
    // Unified spawning logic based on spawn_type
    if (particle.spawn_type == 0u) {
        // Autospawn particles
        if (sim_params.particle_autospawn == 1u && particle.is_alive == 0u) {
            // Check if it's time to spawn this autospawn particle
            let spawn_interval = 1.0 / f32(sim_params.autospawn_rate);
            let time_since_last_spawn = sim_params.time - particle.last_spawn_time;
            
            if (time_since_last_spawn >= spawn_interval) {
                // Spawn at random position in world space
                let random_x = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                let random_y = f32(particle_index) * 0.5678 + sim_params.time * 0.1;
                spawn_x = fract(sin(random_x) * 43758.5453) * 2.0 - 1.0;
                spawn_y = fract(cos(random_y) * 43758.5453) * 2.0 - 1.0;
                should_reset = true; // Force spawn
                particle.last_spawn_time = sim_params.time + spawn_interval; // Update spawn time to next spawn
            } else {
                // Don't spawn yet - keep particle dead
                particle.position = vec2<f32>(0.0, 0.0);
                particle.age = 0.0;
                particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
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
                particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
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
            particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            particle.is_alive = 0u;
            particles[particle_index] = particle;
            return;
        }
    } else {
        // Brush particles (spawn_type == 1u)
        if (sim_params.mouse_button_down == 1u && particle.is_alive == 0u) {
            // Left click is held - spawn brush particles like a spray can
            let spawn_interval = 1.0 / f32(sim_params.brush_spawn_rate);
            let time_since_last_spawn = sim_params.time - particle.last_spawn_time;
            
            if (time_since_last_spawn >= spawn_interval) {
                // Spawn at cursor with random offset (spray can effect)
                let radius = f32(sim_params.cursor_size) * 0.01;
                let seed1 = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                let seed2 = f32(particle_index) * 0.5678 + sim_params.time * 0.05;
                let angle = fract(sin(seed1) * 43758.5453) * 2.0 * 3.14159;
                let distance = fract(cos(seed2) * 43758.5453);
                let offset_x = cos(angle) * radius * distance;
                let offset_y = sin(angle) * radius * distance;
                spawn_x = sim_params.cursor_x + offset_x;
                spawn_y = sim_params.cursor_y + offset_y;
                should_reset = true; // Force spawn
                particle.last_spawn_time = sim_params.time + spawn_interval; // Update spawn time to next spawn
            } else {
                // Don't spawn yet - keep particle dead
                particle.position = vec2<f32>(0.0, 0.0);
                particle.age = 0.0;
                particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
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
                particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
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
            particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
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
    
    // Set particle color based on display mode (every frame to handle mode changes)
    if (sim_params.display_mode == 0u) { // Age mode
        // Color will be set in render shader based on age
        particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.9);
    } else if (sim_params.display_mode == 1u) { // Random mode
        // Generate random color based only on particle index (not time, so it stays constant)
        let seed = f32(particle_index) * 0.1234;
        let random_value = fract(sin(seed) * 43758.5453);
        particle.color = vec4<f32>(random_value, 0.0, 0.0, 0.9);
    } else if (sim_params.display_mode == 2u) { // Direction mode
        // Color will be set based on velocity direction
        particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.9);
    }
    
    // Get flow direction from nearest flow vector
    var direction = find_nearest_flow_vector(particle.position);
    
    // Update particle color based on display mode
    if (sim_params.display_mode == 2u) { // Direction mode
        // Set color based on velocity direction
        let direction_angle = atan2(direction.y, direction.x);
        let normalized_angle = (direction_angle + 3.14159) / (2.0 * 3.14159); // Normalize to [0, 1]
        particle.color.r = normalized_angle;
    }
    
    // Apply cursor interaction if active (right click destroys particles)
    if (sim_params.mouse_button_down == 2u) {
        // Right click destroy mode - destroy particles near cursor
        let cursor_pos = vec2<f32>(sim_params.cursor_x, sim_params.cursor_y);
        let to_cursor = cursor_pos - particle.position;
        let cursor_dist = length(to_cursor);
        let cursor_radius = f32(sim_params.cursor_size) * 0.01; // Scale cursor size
        
        if (cursor_dist < cursor_radius) {
            // Zero out particle when destroyed
            particle.position = vec2<f32>(0.0, 0.0);
            particle.age = 0.0;
            particle.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            particle.is_alive = 0u;
            particles[particle_index] = particle;
            return;
        }
    }
    
    // Move particle along flow direction
    particle.position += direction * sim_params.particle_speed;
    
    // Wrap around edges
    particle.position.x = fract(particle.position.x * 0.5 + 0.5) * 2.0 - 1.0;
    particle.position.y = fract(particle.position.y * 0.5 + 0.5) * 2.0 - 1.0;
    
    // Deposit trail at particle position with LUT color based on display mode
    var trail_color_intensity = 0.0;
    
    if (sim_params.display_mode == 0u) { // Age mode
        let age_ratio = particle.age / sim_params.particle_lifetime;
        trail_color_intensity = 1.0 - age_ratio; // Younger particles = higher intensity
    } else if (sim_params.display_mode == 1u) { // Random mode
        trail_color_intensity = particle.color.r; // Use the random color value
    } else if (sim_params.display_mode == 2u) { // Direction mode
        trail_color_intensity = particle.color.r; // Use the direction-based color value
    } else {
        // Fallback to age mode
        let age_ratio = particle.age / sim_params.particle_lifetime;
        trail_color_intensity = 1.0 - age_ratio;
    }
    
    let trail_color = get_lut_color(trail_color_intensity);
    deposit_trail(particle.position, vec4<f32>(trail_color, particle.color.a));
    
    // Adjust alpha based on age
    let age_ratio = particle.age / sim_params.particle_lifetime;
    particle.color.a = 1.0 - age_ratio * 0.5;
    
    particles[particle_index] = particle;
} 