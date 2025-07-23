struct Particle {
    position: vec2<f32>,
    age: f32,
    color: vec4<f32>,
    my_parent_was: u32, // 0 = autospawned, 1 = spawned by brush
}

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
}

struct SimParams {
    particle_limit: u32, // Kept for backward compatibility, no longer used for limiting
    autospawn_limit: u32, // New setting for limiting autospawned particles
    vector_count: u32,
    particle_lifetime: f32,
    particle_speed: f32,
    noise_seed: u32,
    time: f32,
    width: f32,
    height: f32,
    noise_scale: f32,
    vector_magnitude: f32,
    trail_decay_rate: f32,
    trail_deposition_rate: f32,
    trail_diffusion_rate: f32,
    trail_wash_out_rate: f32,
    trail_map_width: u32,
    trail_map_height: u32,
    particle_shape: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond
    particle_size: u32, // Particle size in pixels
    background_type: u32, // 0=Black, 1=White, 2=Vector Field
    screen_width: u32, // Screen width in pixels
    screen_height: u32, // Screen height in pixels
    cursor_x: f32,
    cursor_y: f32,
    cursor_active: u32, // 0=inactive, 1=attract, 2=repel
    cursor_size: u32,
    cursor_strength: f32,
    particle_autospawn: u32, // 0=disabled, 1=enabled
    particle_spawn_rate: f32, // 0 = no spawn, 1.0 = full spawn rate
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

// Get color from LUT
fn get_lut_color(intensity: f32) -> vec3<f32> {
    let lut_index = clamp(intensity * 255.0, 0.0, 255.0);
    let index = u32(lut_index);
    
    // LUT data format: [r0, r1, ..., r255, g0, g1, ..., g255, b0, b1, ..., b255]
    let r = f32(lut_data[index]) / 255.0;
    let g = f32(lut_data[index + 256u]) / 255.0;
    let b = f32(lut_data[index + 512u]) / 255.0;
    
    return vec3<f32>(r, g, b);
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
    
    // No longer limit by particle_limit - process all particles
    // The actual particle count is managed by the buffer size
    
    var particle = particles[particle_index];
    
    // Update age
    particle.age += 0.016; // ~60 FPS
    
    // Check if particle should be reset or if we should spawn new particles at cursor
    var should_reset = particle.age >= sim_params.particle_lifetime;
    var spawn_x: f32;
    var spawn_y: f32;
    
    // Determine spawn position based on cursor state
    if (sim_params.cursor_active == 1u) {
        // Only allow forced reset for autospawned particles
        if (particle.my_parent_was == 0u) {
            let spawn_chance = fract(f32(particle_index) * 0.1234 + sim_params.time * 2.0);
            let scaled_strength = sim_params.cursor_strength * 0.2 * sim_params.particle_spawn_rate;
            if (spawn_chance < scaled_strength) {
                let radius = f32(sim_params.cursor_size) * 0.01;
                let seed1 = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                let seed2 = f32(particle_index) * 0.5678 + sim_params.time * 0.05;
                let angle = fract(sin(seed1) * 43758.5453) * 2.0 * 3.14159;
                let distance = fract(cos(seed2) * 43758.5453);
                let offset_x = cos(angle) * radius * distance;
                let offset_y = sin(angle) * radius * distance;
                spawn_x = sim_params.cursor_x + offset_x;
                spawn_y = sim_params.cursor_y + offset_y;
                should_reset = true;
                particle.my_parent_was = 1u;
                            } else if (should_reset) {
                    // If dying naturally, allow respawn at cursor for both types
                    let natural_spawn_chance = fract(f32(particle_index) * 0.5678 + sim_params.time * 0.3);
                    if (natural_spawn_chance < scaled_strength * 0.5 * sim_params.particle_spawn_rate) {
                    let radius = f32(sim_params.cursor_size) * 0.01;
                    let seed1 = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                    let seed2 = f32(particle_index) * 0.5678 + sim_params.time * 0.05;
                    let angle = fract(sin(seed1) * 43758.5453) * 2.0 * 3.14159;
                    let distance = fract(cos(seed2) * 43758.5453);
                    let offset_x = cos(angle) * radius * distance;
                    let offset_y = sin(angle) * radius * distance;
                    spawn_x = sim_params.cursor_x + offset_x;
                    spawn_y = sim_params.cursor_y + offset_y;
                    particle.my_parent_was = 1u;
                } else {
                    let random_x = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                    let random_y = f32(particle_index) * 0.5678 + sim_params.time * 0.1;
                    spawn_x = fract(sin(random_x) * 43758.5453) * 2.0 - 1.0;
                    spawn_y = fract(cos(random_y) * 43758.5453) * 2.0 - 1.0;
                    particle.my_parent_was = 0u;
                }
            } else {
                spawn_x = particle.position.x;
                spawn_y = particle.position.y;
            }
        } else {
            // For brush-spawned particles, only reset if they naturally expire
            if (should_reset) {
                let spawn_chance = fract(f32(particle_index) * 0.9012 + sim_params.time * 0.2);
                if (spawn_chance < sim_params.particle_spawn_rate) {
                    let radius = f32(sim_params.cursor_size) * 0.01;
                    let seed1 = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                    let seed2 = f32(particle_index) * 0.5678 + sim_params.time * 0.05;
                    let angle = fract(sin(seed1) * 43758.5453) * 2.0 * 3.14159;
                    let distance = fract(cos(seed2) * 43758.5453);
                    let offset_x = cos(angle) * radius * distance;
                    let offset_y = sin(angle) * radius * distance;
                    spawn_x = sim_params.cursor_x + offset_x;
                    spawn_y = sim_params.cursor_y + offset_y;
                    particle.my_parent_was = 1u;
                } else {
                    // Spawn at random position instead
                    let random_x = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                    let random_y = f32(particle_index) * 0.5678 + sim_params.time * 0.1;
                    spawn_x = fract(sin(random_x) * 43758.5453) * 2.0 - 1.0;
                    spawn_y = fract(cos(random_y) * 43758.5453) * 2.0 - 1.0;
                    particle.my_parent_was = 0u;
                }
            } else {
                spawn_x = particle.position.x;
                spawn_y = particle.position.y;
            }
        }
    } else if (sim_params.cursor_active == 2u) {
        // Destroy mode - don't spawn new particles, let existing ones die naturally
        if (should_reset) {
            // Spawn at random position instead of at cursor
            let random_x = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
            let random_y = f32(particle_index) * 0.5678 + sim_params.time * 0.1;
            spawn_x = fract(sin(random_x) * 43758.5453) * 2.0 - 1.0;
            spawn_y = fract(cos(random_y) * 43758.5453) * 2.0 - 1.0;
        } else {
            // Particle continues normally
            spawn_x = particle.position.x;
            spawn_y = particle.position.y;
        }
    } else if (should_reset) {
        // Check if autospawn is enabled and within autospawn limit
        if (sim_params.particle_autospawn == 1u && particle_index < sim_params.autospawn_limit) {
            // Apply spawn rate to autospawn
            let spawn_chance = fract(f32(particle_index) * 0.3456 + sim_params.time * 0.1);
            if (spawn_chance < sim_params.particle_spawn_rate) {
                // Normal mode - spawn at random position when dying
                let random_x = f32(particle_index) * 0.1234 + sim_params.time * 0.1;
                let random_y = f32(particle_index) * 0.5678 + sim_params.time * 0.1;
                spawn_x = fract(sin(random_x) * 43758.5453) * 2.0 - 1.0;
                spawn_y = fract(cos(random_y) * 43758.5453) * 2.0 - 1.0;
                particle.my_parent_was = 0u;
            } else {
                // Don't spawn - keep particle dead
                spawn_x = particle.position.x;
                spawn_y = particle.position.y;
                should_reset = false;
                particle.age = sim_params.particle_lifetime;
                particles[particle_index] = particle;
                return;
            }
        } else {
            // Autospawn disabled or beyond autospawn limit - particle stays dead (age remains at lifetime)
            spawn_x = particle.position.x;
            spawn_y = particle.position.y;
            should_reset = false; // Don't reset, keep particle dead
            // Don't update position or age - particle remains at lifetime
            particle.age = sim_params.particle_lifetime;
            particles[particle_index] = particle;
            return; // Exit early, don't process this particle further
        }
    } else {
        // Particle continues normally
        spawn_x = particle.position.x;
        spawn_y = particle.position.y;
    }
    
    // Reset particle if needed
    if (should_reset) {
        particle.position = vec2<f32>(spawn_x, spawn_y);
        particle.age = 0.0;
        
        // Generate new random color from LUT using multiple random seeds for better variation
        let seed1 = f32(particle_index) * 0.1234;
        let seed2 = f32(particle_index) * 0.5678;
        let seed3 = f32(particle_index) * 0.9012;
        
        let random1 = fract(sin(seed1) * 43758.5453);
        let random2 = fract(sin(seed2) * 43758.5453);
        let random3 = fract(sin(seed3) * 43758.5453);
        
        // Combine multiple random values for better distribution
        let color_intensity = fract((random1 + random2 + random3) / 3.0);
        let new_color = get_lut_color(color_intensity);
        particle.color = vec4<f32>(new_color, 0.9); // Keep alpha at 0.9
    }
    
    // Get flow direction from nearest flow vector
    var direction = find_nearest_flow_vector(particle.position);
    
    // Apply cursor interaction if active
    if (sim_params.cursor_active == 2u) {
        // Destroy mode - destroy particles near cursor
        let cursor_pos = vec2<f32>(sim_params.cursor_x, sim_params.cursor_y);
        let to_cursor = cursor_pos - particle.position;
        let cursor_dist = length(to_cursor);
        let cursor_radius = f32(sim_params.cursor_size) * 0.01; // Scale cursor size
        
        if (cursor_dist < cursor_radius) {
            // Add a small delay to prevent instant death cycle
            particle.age = min(particle.age + 0.1, sim_params.particle_lifetime);
        }
    }
    
    // Move particle along flow direction
    particle.position += direction * sim_params.particle_speed;
    
    // Wrap around edges
    particle.position.x = fract(particle.position.x * 0.5 + 0.5) * 2.0 - 1.0;
    particle.position.y = fract(particle.position.y * 0.5 + 0.5) * 2.0 - 1.0;
    
    // Deposit trail at particle position with particle color
    deposit_trail(particle.position, particle.color);
    
    // Keep original color from LUT, only adjust alpha based on age
    let age_ratio = particle.age / sim_params.particle_lifetime;
    particle.color.a = 1.0 - age_ratio * 0.5;
    
    particles[particle_index] = particle;
} 