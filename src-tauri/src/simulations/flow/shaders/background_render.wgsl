const PI: f32 = 3.14159265359;

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
}

struct SimParams {
    autospawn_pool_size: u32,
    autospawn_rate: u32,
    brush_pool_size: u32,
    brush_spawn_rate: u32,
    cursor_size: f32,
    cursor_x: f32,
    cursor_y: f32,
    display_mode: u32,
    flow_field_resolution: u32,
    height: f32,
    mouse_button_down: u32,
    noise_dt_multiplier: f32,
    noise_scale: f32,
    noise_seed: u32,
    noise_x: f32,
    noise_y: f32,
    particle_autospawn: u32,
    particle_lifetime: f32,
    particle_shape: u32,
    particle_size: u32,
    particle_speed: f32,
    screen_height: u32,
    screen_width: u32,
    time: f32,
    total_pool_size: u32,
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

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0) var<storage, read> flow_vectors: array<FlowVector>;
@group(0) @binding(1) var<storage, read> lut: array<f32>;
@group(0) @binding(2) var<uniform> sim_params: SimParams;
@group(0) @binding(3) var<uniform> background_color: vec4<f32>;

@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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
    
    // Don't apply camera transformation in offscreen pass - let 3x3 shader handle it
    
    return VertexOutput(
        vec4<f32>(pos, 0.0, 1.0),
        uv,
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Use the pre-calculated background color
    return background_color;
}