struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct CameraUniform {
    view_proj_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );

    let world_position = pos[vertex_index];
    let clip_position = camera.view_proj_matrix * vec4<f32>(world_position, 0.0, 1.0);

    var output: VertexOutput;
    output.position = clip_position;
    output.uv = uv[vertex_index];
    output.world_pos = world_position;
    return output;
}

struct UVPair {
    u: f32,
    v: f32,
};

@group(0) @binding(0)
var<storage, read> simulation_data: array<UVPair>;

@group(0) @binding(1)
var<storage, read> lut: array<u32>;

@group(0) @binding(2)
var<uniform> simulation_params: SimulationParams;

struct SimulationParams {
    feed_rate: f32,
    kill_rate: f32,
    delta_u: f32,
    delta_v: f32,
    width: u32,
    height: u32,
    nutrient_pattern: u32,
    is_nutrient_pattern_reversed: u32,
    cursor_x: f32,
    cursor_y: f32,
};

// Get color from LUT (same format as slime mold)
fn get_lut_color(intensity: f32) -> vec3<f32> {
    let idx = clamp(u32(intensity * 255.0), 0u, 255u);
    let r = f32(lut[idx]) / 255.0;
    let g = f32(lut[256u + idx]) / 255.0;
    let b = f32(lut[512u + idx]) / 255.0;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Get simulation data
    let x = u32(input.uv.x * f32(simulation_params.width));
    let y = u32(input.uv.y * f32(simulation_params.height));
    let index = y * simulation_params.width + x;
    let uv = simulation_data[index];
    
    // Get base color from LUT
    var color = get_lut_color(uv.u);
    
    // Draw cursor crosshair
    let cursor_world_pos = vec2<f32>(simulation_params.cursor_x, simulation_params.cursor_y);
    
    // Transform cursor to NDC space
    let cursor_ndc = vec2<f32>(
        (cursor_world_pos.x - camera.position.x) * camera.zoom,
        (cursor_world_pos.y - camera.position.y) * camera.zoom * camera.aspect_ratio
    );
    
    // Get fragment position in NDC space
    let fragment_ndc = vec2<f32>(
        (input.world_pos.x - camera.position.x) * camera.zoom,
        (input.world_pos.y - camera.position.y) * camera.zoom * camera.aspect_ratio
    );
    
    // Draw crosshair
    let crosshair_size = 0.02;  // Size of crosshair in NDC space
    let line_width = 0.002;     // Width of crosshair lines in NDC space
    
    // Calculate distances in NDC space, accounting for aspect ratio
    let dx = fragment_ndc.x - cursor_ndc.x;
    let dy = (fragment_ndc.y - cursor_ndc.y) / camera.aspect_ratio;
    
    // Check if fragment is within crosshair bounds
    let h_line = abs(dy) < line_width && abs(dx) < crosshair_size;
    let v_line = abs(dx) < line_width && abs(dy) < crosshair_size;
    
    if (h_line || v_line) {
        // Golden color
        let cursor_color = vec3<f32>(1.0, 0.84, 0.0);
        color = mix(color, cursor_color, 0.8);  // 0.8 is cursor opacity
    }
    
    return vec4<f32>(color, 1.0);
} 