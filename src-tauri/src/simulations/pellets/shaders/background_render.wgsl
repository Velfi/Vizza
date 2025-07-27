struct BackgroundParams {
    background_type: u32, // 0 = black, 1 = white, 2 = density
    density_texture_resolution: u32,
}

@group(0) @binding(0)
var<uniform> background_params: BackgroundParams;

@group(0) @binding(1)
var density_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen quad
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    
    var out: VertexOutput;
    out.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (background_params.background_type == 0u) {
        // Black background
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if (background_params.background_type == 1u) {
        // White background
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if (background_params.background_type == 2u) {
        // Potential field visualization
        let tex_coord = vec2<i32>(
            i32(in.uv.x * f32(background_params.density_texture_resolution)),
            i32(in.uv.y * f32(background_params.density_texture_resolution))
        );
        
        // Sample the potential field
        let potential = textureLoad(density_texture, tex_coord, 0).r;
        
        // For inverse square field, use power law scaling to better show the falloff
        let enhanced_potential = pow(potential, 0.5);
        
        // Scale for better visibility of the inverse square falloff
        let scaled_potential = enhanced_potential * 4.0;
        let normalized_potential = min(scaled_potential, 1.0);
        
        // Create a heat map that shows the inverse square falloff clearly
        let t = normalized_potential;
        
        // Color mapping optimized for inverse square field visualization
        let color = vec3<f32>(
            // Red: increases with potential (strong field)
            t,
            // Green: peaks in middle range
            select(2.0 * t, 2.0 * (1.0 - t), t > 0.5),
            // Blue: decreases with potential (weak field)
            1.0 - t
        );
        
        // Add brightness scaling to make the field strength more visible
        let brightness = 0.1 + normalized_potential * 0.9;
        let final_color = color * brightness;
        
        return vec4<f32>(final_color, 1.0);
    }
    
    // Default to black
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
} 