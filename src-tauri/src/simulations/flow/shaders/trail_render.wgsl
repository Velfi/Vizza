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

@group(0) @binding(0) var<uniform> sim_params: SimParams;
@group(0) @binding(1) var trail_map: texture_storage_2d<rgba8unorm, read>;
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
    
    // Apply camera transformation to the fullscreen quad
    let camera_pos = camera.transform_matrix * vec4<f32>(pos, 0.0, 1.0);
    
    return VertexOutput(
        camera_pos,
        uv,
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Sample trail texture at screen position
    let x_coord = i32(uv.x * f32(sim_params.trail_map_width));
    let y_coord = i32(uv.y * f32(sim_params.trail_map_height));
    
    // Clamp to valid range
    let x = clamp(x_coord, 0, i32(sim_params.trail_map_width) - 1);
    let y = clamp(y_coord, 0, i32(sim_params.trail_map_height) - 1);
    
    // Choose sampling method based on diffusion rate
    var final_intensity = 0.0;
    var final_color = vec3<f32>(0.0, 0.0, 0.0);
    
    if (sim_params.trail_diffusion_rate <= 0.01) {
        // No diffusion - sample only center pixel for crisp edges
        let trail_data = textureLoad(trail_map, vec2<i32>(x, y));
        final_intensity = trail_data.a;
        final_color = trail_data.rgb;
    } else {
        // With diffusion - sample larger area to match particle size
        let sample_radius = i32(sim_params.particle_size); // Use full particle size for sampling area
        var total_intensity = 0.0;
        var total_color = vec3<f32>(0.0, 0.0, 0.0);
        var sample_count = 0.0;
        
        for (var dx = -sample_radius; dx <= sample_radius; dx++) {
            for (var dy = -sample_radius; dy <= sample_radius; dy++) {
                let sample_x = clamp(x + dx, 0, i32(sim_params.trail_map_width) - 1);
                let sample_y = clamp(y + dy, 0, i32(sim_params.trail_map_height) - 1);
                
                let trail_data = textureLoad(trail_map, vec2<i32>(sample_x, sample_y));
                let trail_intensity = trail_data.a;
                let trail_color = trail_data.rgb;
                
                total_intensity += trail_intensity;
                total_color += trail_color * trail_intensity;
                sample_count += 1.0;
            }
        }
        
        // Average the samples
        final_intensity = total_intensity / sample_count;
        final_color = total_color / max(total_intensity, 0.001); // Avoid division by zero
    }
    
    // Only render if there's trail data
    if (final_intensity <= 0.01) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    
    // Use the stored trail color directly
    let alpha = final_intensity; // Full intensity = full opacity
    
    return vec4<f32>(final_color, alpha);
} 