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
    cursor_active: u32, // 0=Inactive, 1=Attract, 2=Repel
    cursor_size: u32,
    cursor_strength: f32,
    particle_autospawn: u32, // 0=disabled, 1=enabled
    particle_spawn_rate: f32, // 0.0 = no spawn, 1.0 = full spawn rate
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0) var<uniform> sim_params: SimParams;
@group(0) @binding(1) var trail_map: texture_storage_2d<rgba8unorm, read>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) grid_fade_factor: f32,
    @location(2) world_pos: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Full screen quad
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
    
    // Calculate grid cell position (0-8, arranged as 3x3 grid)
    let grid_x = i32(instance_index % 3u) - 1; // -1, 0, 1
    let grid_y = i32(instance_index / 3u) - 1; // -1, 0, 1
    
    // Calculate fade factor based on distance from center
    let center_distance = abs(grid_x) + abs(grid_y);
    var grid_fade_factor: f32;
    if (center_distance == 0) {
        grid_fade_factor = 1.0; // Center cell - full opacity
    } else if (center_distance == 1) {
        grid_fade_factor = 0.4; // Adjacent cells - medium fade
    } else {
        grid_fade_factor = 0.2; // Corner cells - strong fade
    }
    
    // Start with base world position and offset by grid cell
    // Each grid cell represents a full world tile offset (width/height = 2.0)
    let world_position = vec2<f32>(
        pos.x + f32(grid_x) * 2.0,
        pos.y + f32(grid_y) * 2.0
    );
    
    // Apply camera transformation to the world position
    let camera_pos = camera.transform_matrix * vec4<f32>(world_position, 0.0, 1.0);
    
    return VertexOutput(
        camera_pos,
        uv,
        grid_fade_factor,
        world_position,
    );
}

// Convert world coordinates to trail texture coordinates
fn world_to_trail_coords(world_pos: vec2<f32>) -> vec2<f32> {
    // World coordinates are -1 to 1, convert to 0 to 1 for texture sampling
    return vec2<f32>(
        (world_pos.x + 1.0) * 0.5,
        (world_pos.y + 1.0) * 0.5  // Remove Y flip to match particle_update.wgsl
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>, @location(1) grid_fade_factor: f32, @location(2) world_pos: vec2<f32>) -> @location(0) vec4<f32> {
    let trail_uv = world_to_trail_coords(world_pos);
    let texel = vec2<i32>(
        i32(trail_uv.x * f32(sim_params.trail_map_width)),
        i32(trail_uv.y * f32(sim_params.trail_map_height))
    );
    let trail_data = textureLoad(trail_map, texel);
    let trail_intensity = trail_data.a;
    let trail_color = trail_data.rgb;
    if (trail_intensity <= 0.01) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    let alpha = trail_intensity * grid_fade_factor;
    return vec4<f32>(trail_color, alpha);
} 