// Display shader for converting trail map to displayable texture
// Uses LUT for color mapping

struct SimSizeUniform {
    width: u32,
    height: u32,
    decay_rate: f32,
    agent_jitter: f32,
    agent_speed_min: f32,
    agent_speed_max: f32,
    agent_turn_rate: f32,
    agent_sensor_angle: f32,
    agent_sensor_distance: f32,
    diffusion_rate: f32,
    pheromone_deposition_rate: f32,
    gradient_enabled: u32,
    gradient_type: u32,
    gradient_strength: f32,
    gradient_center_x: f32,
    gradient_center_y: f32,
    gradient_size: f32,
    gradient_angle: f32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0)
var<storage, read> trail_map: array<f32>;

@group(0) @binding(1)
var display_tex: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(2)
var<uniform> sim_size: SimSizeUniform;

@group(0) @binding(3)
var<storage, read> lut_data: array<u32>;

@group(0) @binding(4)
var<storage, read> gradient_map: array<f32>;

fn get_lut_color(intensity: f32) -> vec4<f32> {
    // Clamp intensity to [0, 1]
    let clamped_intensity = clamp(intensity, 0.0, 1.0);
    
    // Convert to index in LUT (0-255)
    let index = u32(clamped_intensity * 255.0);
    
    // Get RGB values from LUT (each component is 256 bytes long)
    let r = f32(lut_data[index]) / 255.0;
    let g = f32(lut_data[index + 256u]) / 255.0;
    let b = f32(lut_data[index + 512u]) / 255.0;
    
    return vec4<f32>(r, g, b, 1.0);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let texture_width = textureDimensions(display_tex).x;
    let texture_height = textureDimensions(display_tex).y;
    let sim_width = f32(sim_size.width);
    let sim_height = f32(sim_size.height);
    let tex_width = f32(texture_width);
    let tex_height = f32(texture_height);

    // Compute aspect ratios
    let sim_aspect = sim_width / sim_height;
    let tex_aspect = tex_width / tex_height;

    // Compute scale and offsets to center the simulation
    var scale: f32;
    var offset_x: f32 = 0.0;
    var offset_y: f32 = 0.0;
    if (tex_aspect > sim_aspect) {
        // Texture is wider than simulation: fit height
        scale = tex_height / sim_height;
        offset_x = (tex_width - sim_width * scale) * 0.5;
    } else {
        // Texture is taller than simulation: fit width
        scale = tex_width / sim_width;
        offset_y = (tex_height - sim_height * scale) * 0.5;
    }

    // Map texture pixel to simulation coordinates
    let fx = (f32(id.x) - offset_x) / scale;
    let fy = (f32(id.y) - offset_y) / scale;

    // Only draw if inside simulation bounds
    if (fx >= 0.0 && fx < sim_width && fy >= 0.0 && fy < sim_height) {
        let x = i32(fx);
        let y = i32(fy);
        let idx = y * i32(sim_size.width) + x;
        
        // Get trail intensity and scale it for better visibility
        let trail_intensity = clamp(trail_map[idx] * 10.0, 0.0, 1.0);
        
        // Get gradient intensity if enabled
        var gradient_intensity: f32;
        if (sim_size.gradient_enabled == 1u) {
            gradient_intensity = clamp(gradient_map[idx], 0.0, 1.0) * 0.3;  // Reduce gradient visibility so trails still show
        } else {
            gradient_intensity = 0.0;
        }
        
        // Combine trail and gradient intensities
        let combined_intensity = clamp(trail_intensity + gradient_intensity, 0.0, 1.0);
        
        let color = get_lut_color(combined_intensity);
        textureStore(display_tex, vec2<i32>(i32(id.x), i32(id.y)), color);
    } else {
        // Fill bars with black
        textureStore(display_tex, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    }
} 