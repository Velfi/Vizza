const PI: f32 = 3.14159265359;

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
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
    allow_mouse_interaction: u32, // 0=False, 1=True
    trail_render_mode: u32, // 0=Full, 1=Compact
    background_strength: f32, // Strength of the background vector field
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
    // Convert UV to world position for flow field sampling
    let world_pos = vec2<f32>(
        uv.x * 2.0 - 1.0,
        uv.y * 2.0 - 1.0
    );
    
    var final_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    
    if (sim_params.background_type == 0u) {
        // Black background
        final_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if (sim_params.background_type == 1u) {
        // White background
        final_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if (sim_params.background_type == 2u) {
        // Vector Field background
        // Sample flow field at world position
        let grid_w = 128.0;
        let grid_h = 128.0;
        let tx = (world_pos.x + 1.0) * 0.5 * (grid_w - 1.0);
        let ty = (world_pos.y + 1.0) * 0.5 * (grid_h - 1.0);
        let x0 = u32(floor(tx));
        let x1 = u32(min(ceil(tx), grid_w - 1.0));
        let y0 = u32(floor(ty));
        let y1 = u32(min(ceil(ty), grid_h - 1.0));
        let fx = tx - f32(x0);
        let fy = ty - f32(y0);
        let idx00 = y0 * u32(grid_w) + x0;
        let idx10 = y0 * u32(grid_w) + x1;
        let idx01 = y1 * u32(grid_w) + x0;
        let idx11 = y1 * u32(grid_w) + x1;
        let v00 = flow_vectors[idx00].direction;
        let v10 = flow_vectors[idx10].direction;
        let v01 = flow_vectors[idx01].direction;
        let v11 = flow_vectors[idx11].direction;
        let v0 = mix(v00, v10, fx);
        let v1 = mix(v01, v11, fx);
        let flow = mix(v0, v1, fy);
        
        let angle = atan2(flow.y, flow.x);
        let normalized_angle = (angle + PI) / (2.0 * PI);
        
        let lut_index = u32(normalized_angle * 255.0);
        let r = lut[lut_index] / 255.0;
        let g = lut[lut_index + 256u] / 255.0;
        let b = lut[lut_index + 512u] / 255.0;
        
        final_color = vec4<f32>(r, g, b, 1.0) * sim_params.background_strength;
    }
    
    return final_color;
}