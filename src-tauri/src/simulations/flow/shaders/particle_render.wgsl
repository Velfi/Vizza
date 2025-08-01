struct Particle {
    position: vec2<f32>,
    age: f32,
    color: vec4<f32>,
    my_parent_was: u32, // 0= autospawned, 1 = spawned by brush
}

struct SimParams {
    particle_limit: u32,
    autospawn_limit: u32,
    vector_count: u32,
    particle_lifetime: f32,
    particle_speed: f32,
    noise_seed: u32,
    time: f32,
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
    background_type: u32, // 0=Black, 1=Texture, 2=Field
    screen_width: u32, // Screen width in pixels
    screen_height: u32, // Screen height in pixels
    cursor_x: f32,
    cursor_y: f32,
    cursor_active: u32, // 0=inactive, 1=attract, 2=repel
    cursor_size: u32,
    cursor_strength: f32,
    particle_autospawn: u32, // 0=disabled, 1=enabled
    particle_spawn_rate: f32, // 0.0 = no spawn, 1.0 = full spawn rate
    display_mode: u32, // 0=Age, 1=Random, 2=Direction
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> sim_params: SimParams;
@group(0) @binding(2) var<storage, read> lut_data: array<u32>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) particle_index: u32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let particle = particles[instance_index];
    
    // Create a quad centered at particle position
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    
    let uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    
    let pos = positions[vertex_index];
    let uv = uvs[vertex_index];
    
    // Calculate quad offset for this vertex
    let quad_offset = pos * f32(sim_params.particle_size) / vec2<f32>(f32(sim_params.screen_width), f32(sim_params.screen_height));
    
    // Add quad offset to particle position
    let world_pos = particle.position + quad_offset;
    
    // Don't apply camera transformation in offscreen pass - let 3x3 shader handle it
    
    return VertexOutput(
        vec4<f32>(world_pos, 0.0, 1.0),
        uv,
        instance_index,
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>, @location(1) particle_index: u32) -> @location(0) vec4<f32> {
    let particle = particles[particle_index];
    
    // Check if particle is dead (age >= lifetime) - if so, discard it completely
    if (particle.age >= sim_params.particle_lifetime) {
        discard;
    }
    
    // Get particle shape from uniform
    let shape = sim_params.particle_shape;
    
    // Check if pixel is inside the particle shape
    var is_inside = false;
    let center = vec2<f32>(0.5, 0.5);
    let offset = uv - center;
    
    if (shape == 0u) { // Circle
        let dist = distance(uv, center);
        is_inside = dist <= 0.5;
    } else if (shape == 1u) { // Square
        is_inside = abs(offset.x) <= 0.5 && abs(offset.y) <= 0.5;
    } else if (shape == 2u) { // Triangle
        let triangle_uv = vec2<f32>(offset.x, offset.y * 1.732); // Scale Y for equilateral triangle
        is_inside = abs(triangle_uv.x) + abs(triangle_uv.y) <= 0.5;
    } else if (shape == 3u) { // Star
        let angle = atan2(offset.y, offset.x);
        let radius = length(offset);
        let star_radius = 0.5;
        let inner_radius = star_radius * 0.4;
        let points = 5.0;
        let angle_per_point = 6.28318 / points;
        let point_angle = (angle + 6.28318) % angle_per_point;
        let point_radius = mix(inner_radius, star_radius, smoothstep(0.0, angle_per_point * 0.3, point_angle));
        is_inside = radius <= point_radius;
    } else if (shape == 4u) { // Diamond
        is_inside = abs(offset.x) + abs(offset.y) <= 0.5;
    }
    
    if (!is_inside) {
        discard;
    }
    
    // Create smooth fade for particle edges
    let fade = 1.0 - smoothstep(0.0, 0.5, length(offset));
    
    // Calculate color based on display mode
    var color_intensity = 0.0;
    
    if (sim_params.display_mode == 0u) { // Age mode
        // Use particle age as the color intensity to create a gradient effect
        let age_ratio = particle.age / sim_params.particle_lifetime;
        color_intensity = 1.0 - age_ratio; // Younger particles = higher intensity
    } else if (sim_params.display_mode == 1u) { // Random mode
        // Use the stored color from the particle (set during creation)
        color_intensity = particle.color.r; // Use red channel as intensity
    } else if (sim_params.display_mode == 2u) { // Direction mode
        // Use the stored color from the particle (set during update based on velocity)
        color_intensity = particle.color.r; // Use red channel as intensity
    } else {
        // Fallback to age mode if display mode is invalid
        let age_ratio = particle.age / sim_params.particle_lifetime;
        color_intensity = 1.0 - age_ratio;
    }
    
    let particle_color = get_lut_color(color_intensity);
    
    // Apply particle fade
    return vec4<f32>(particle_color, particle.color.a * fade);
} 