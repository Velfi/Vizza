struct Particle {
    position: vec2<f32>,
    age: f32,
    color: vec4<f32>,
}

struct SimParams {
    particle_limit: u32,
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
    @location(2) grid_fade_factor: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    // 3x3 grid mode: render each particle 9 times like other simulations
    let particles_per_grid = sim_params.particle_limit;
    let actual_particle_index = instance_index % particles_per_grid;
    let grid_cell_index = instance_index / particles_per_grid;
    
    let particle = particles[actual_particle_index];
    
    // Calculate grid cell position (0-8, arranged as 3x3 grid)
    let grid_x = i32(grid_cell_index % 3u) - 1; // -1, 0, 1
    let grid_y = i32(grid_cell_index / 3u) - 1; // -1, 0, 1
    
    // Calculate fade factor based on distance from center
    let center_distance = abs(grid_x) + abs(grid_y);
    var grid_fade_factor = 1.0;
    if (center_distance == 0) {
        grid_fade_factor = 1.0; // Center cell - full opacity
    } else if (center_distance == 1) {
        grid_fade_factor = 0.4; // Adjacent cells - medium fade
    } else {
        grid_fade_factor = 0.2; // Corner cells - strong fade
    }
    
    // Create a quad centered at particle position
    let quad_size = f32(sim_params.particle_size) * 0.001; // Convert pixels to world units
    
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-quad_size, -quad_size),
        vec2<f32>( quad_size, -quad_size),
        vec2<f32>(-quad_size,  quad_size),
        vec2<f32>(-quad_size,  quad_size),
        vec2<f32>( quad_size, -quad_size),
        vec2<f32>( quad_size,  quad_size),
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
    
    // Offset particle position by grid cell
    let world_pos = particle.position + pos + vec2<f32>(f32(grid_x) * 2.0, f32(grid_y) * 2.0);
    
    // Apply camera transformation
    let camera_pos = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    
    return VertexOutput(
        camera_pos,
        uv,
        actual_particle_index,
        grid_fade_factor,
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>, @location(1) particle_index: u32, @location(2) grid_fade_factor: f32) -> @location(0) vec4<f32> {
    let particle = particles[particle_index];
    
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
    
    // Apply grid fade factor and particle fade
    return vec4<f32>(particle.color.rgb * grid_fade_factor, particle.color.a * fade * grid_fade_factor);
} 