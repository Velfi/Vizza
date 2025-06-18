struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Create a quad that covers the full screen area
    // The camera transformation will handle aspect ratio and zoom
    var pos = array<vec2<f32>, 6>(
        // First triangle
        vec2<f32>(-1.0, -1.0),  // Bottom-left
        vec2<f32>(1.0, -1.0),   // Bottom-right  
        vec2<f32>(-1.0, 1.0),   // Top-left
        // Second triangle
        vec2<f32>(1.0, -1.0),   // Bottom-right
        vec2<f32>(1.0, 1.0),    // Top-right
        vec2<f32>(-1.0, 1.0)    // Top-left
    );

    // UV coordinates that map to simulation grid [0,1] x [0,1]
    var uv = array<vec2<f32>, 6>(
        // First triangle
        vec2<f32>(0.0, 0.0),  // Bottom-left
        vec2<f32>(1.0, 0.0),  // Bottom-right
        vec2<f32>(0.0, 1.0),  // Top-left
        // Second triangle
        vec2<f32>(1.0, 0.0),  // Bottom-right
        vec2<f32>(1.0, 1.0),  // Top-right
        vec2<f32>(0.0, 1.0)   // Top-left
    );

    let world_position = pos[vertex_index];
    // Apply camera transformation (handles aspect ratio, zoom, and pan)
    let clip_position = camera.transform_matrix * vec4<f32>(world_position, 0.0, 1.0);

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

// Helper function to safely sample simulation data with bounds checking
fn sample_simulation_data(x: u32, y: u32) -> UVPair {
    if (x >= simulation_params.width || y >= simulation_params.height) {
        var result: UVPair;
        result.u = 0.0;
        result.v = 0.0;
        return result;
    }
    let index = y * simulation_params.width + x;
    return simulation_data[index];
}

// Bilinear interpolation function
fn bilinear_interpolate(uv: vec2<f32>) -> UVPair {
    // Convert UV coordinates to grid coordinates
    let grid_x = uv.x * f32(simulation_params.width);
    let grid_y = uv.y * f32(simulation_params.height);
    
    // Get the four surrounding grid cells
    let x0 = u32(grid_x);
    let y0 = u32(grid_y);
    let x1 = min(x0 + 1u, simulation_params.width - 1u);
    let y1 = min(y0 + 1u, simulation_params.height - 1u);
    
    // Calculate interpolation weights
    let fx = fract(grid_x);
    let fy = fract(grid_y);
    
    // Sample the four corners
    let p00 = sample_simulation_data(x0, y0);
    let p10 = sample_simulation_data(x1, y0);
    let p01 = sample_simulation_data(x0, y1);
    let p11 = sample_simulation_data(x1, y1);
    
    // Bilinear interpolation
    let u_value = mix(
        mix(p00.u, p10.u, fx),
        mix(p01.u, p11.u, fx),
        fy
    );
    
    let v_value = mix(
        mix(p00.v, p10.v, fx),
        mix(p01.v, p11.v, fx),
        fy
    );
    
    var result: UVPair;
    result.u = u_value;
    result.v = v_value;
    return result;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // UV coordinates map to simulation grid [0,1] x [0,1]
    let grid_x = input.uv.x;
    let grid_y = input.uv.y;
    
    // Use bilinear interpolation to get smooth sampling
    let uv = bilinear_interpolate(vec2<f32>(grid_x, grid_y));
    
    // Get color from LUT using the u value (concentration of chemical A)
    var color = get_lut_color(uv.u);
    
    // Apply some brightness adjustment based on v value (concentration of chemical B)
    // This can help visualize the interaction between the two chemicals
    let brightness = 0.5 + 0.5 * uv.v;
    color = color * brightness;
    
    return vec4<f32>(color, 1.0);
} 