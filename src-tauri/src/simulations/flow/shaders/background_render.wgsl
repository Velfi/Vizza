const PI: f32 = 3.14159265359;

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
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
    background_type: u32, // 0=Black, 1=White, 2=Vector Field
    screen_width: u32, // Screen width in pixels
    screen_height: u32, // Screen height in pixels
    cursor_x: f32,
    cursor_y: f32,
    cursor_active: u32, // 0=inactive, 1=click, 2=hold
    cursor_size: u32,
    cursor_strength: f32,
    particle_autospawn: u32, // 0=disabled, 1=enabled
    particle_spawn_rate: f32, // 0 = no spawn, 1.0 = full spawn rate
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
    
    // Apply camera transformation to the full screen quad
    let camera_pos = camera.transform_matrix * vec4<f32>(pos, 0.0, 1.0);
    
    return VertexOutput(
        camera_pos,
        uv,
    );
}

// Sample flow direction at any position using bilinear interpolation
fn sample_flow_direction(pos: vec2<f32>) -> vec2<f32> {
    // Find the four nearest grid points
    let grid_size = 20.0;
    let grid_spacing = 2.0 / (grid_size - 1.0);
    
    let grid_x = (pos.x + 1.0) / grid_spacing;
    let grid_y = (pos.y + 1.0) / grid_spacing;
    
    let x0 = floor(grid_x);
    let y0 = floor(grid_y);
    let x1 = min(x0 + 1.0, grid_size - 1.0);
    let y1 = min(y0 + 1.0, grid_size - 1.0);
    
    let fx = fract(grid_x);
    let fy = fract(grid_y);
    
    // Get the four corner indices
    let i00 = u32(x0 + y0 * grid_size);
    let i10 = u32(x1 + y0 * grid_size);
    let i01 = u32(x0 + y1 * grid_size);
    let i11 = u32(x1 + y1 * grid_size);
    
    // Clamp indices to valid range
    let max_index = sim_params.vector_count - 1u;
    let i00_clamped = min(i00, max_index);
    let i10_clamped = min(i10, max_index);
    let i01_clamped = min(i01, max_index);
    let i11_clamped = min(i11, max_index);
    
    // Sample the four corners
    let v00 = flow_vectors[i00_clamped].direction;
    let v10 = flow_vectors[i10_clamped].direction;
    let v01 = flow_vectors[i01_clamped].direction;
    let v11 = flow_vectors[i11_clamped].direction;
    
    // Bilinear interpolation
    let v0 = mix(v00, v10, fx);
    let v1 = mix(v01, v11, fx);
    let result = mix(v0, v1, fy);
    
    return normalize(result);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Handle different background types
    if (sim_params.background_type == 0u) {
        // Black background
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if (sim_params.background_type == 1u) {
        // White background
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else {
        // Vector Field background
        // Convert UV to world coordinates
        let world_pos = uv * 2.0 - 1.0;
        
        // Sample flow direction at this position
        let flow_direction = sample_flow_direction(world_pos);
        
        // Calculate direction angle for color
        let angle = atan2(flow_direction.y, flow_direction.x);
        let normalized_angle = (angle + PI) / (2.0 * PI); // 0 to 1
        
        // Sample LUT for color
        let lut_index = u32(normalized_angle * 255.0);
        let r = lut[lut_index];
        let g = lut[lut_index + 256];
        let b = lut[lut_index + 512];
        
        // Create a regular grid of vector lines
        // Each line should be approximately 100px long
        let target_line_length_pixels = 100.0;
        let screen_diagonal = sqrt(f32(sim_params.screen_width) * f32(sim_params.screen_width) + f32(sim_params.screen_height) * f32(sim_params.screen_height));
        let line_length = (target_line_length_pixels / screen_diagonal) * 2.0; // Convert to normalized coordinates
        let line_width = 0.002; // Thin lines
        
        // Create a grid pattern for vector lines
        let grid_spacing = line_length * 1.5; // Space between vector lines
        let grid_x = floor(world_pos.x / grid_spacing);
        let grid_y = floor(world_pos.y / grid_spacing);
        
        // Sample flow direction at the grid point
        let grid_pos = vec2<f32>(
            grid_x * grid_spacing + grid_spacing * 0.5,
            grid_y * grid_spacing + grid_spacing * 0.5
        );
        let grid_direction = sample_flow_direction(grid_pos);
        
        // Calculate line start and end points
        let line_start = grid_pos - normalize(grid_direction) * line_length * 0.5;
        let line_end = grid_pos + normalize(grid_direction) * line_length * 0.5;
        
        // Check if current pixel is on the line
        let to_pixel = world_pos - line_start;
        let line_vector = line_end - line_start;
        let line_length_actual = length(line_vector);
        
        if (line_length_actual > 0.0) {
            let line_direction = line_vector / line_length_actual;
            let along_line = dot(to_pixel, line_direction);
            let perpendicular = vec2<f32>(-line_direction.y, line_direction.x);
            let across_line = dot(to_pixel, perpendicular);
            
            // Check if pixel is within the line bounds
            let in_line = along_line >= 0.0 && along_line <= line_length_actual && abs(across_line) <= line_width;
            
            if (in_line) {
                return vec4<f32>(r / 255.0, g / 255.0, b / 255.0, 0.8);
            }
        }
        
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
} 