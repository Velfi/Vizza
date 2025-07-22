struct BackgroundParams {
    background_type: u32, // 0 = black, 1 = white, 2 = density
    density_texture_resolution: u32, // Add texture resolution for proper sampling
}

@group(0) @binding(0)
var<uniform> background_params: BackgroundParams;

@group(0) @binding(1)
var density_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) grid_fade_factor: f32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Full-screen quad
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    
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
        (x * 2.0 - 1.0) + f32(grid_x) * 2.0, // Offset by full world width
        (y * 2.0 - 1.0) + f32(grid_y) * 2.0  // Offset by full world height
    );
    
    var out: VertexOutput;
    out.position = vec4<f32>(world_position, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    out.grid_fade_factor = grid_fade_factor;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (background_params.background_type == 0u) {
        // Black background
        return vec4<f32>(0.0, 0.0, 0.0, in.grid_fade_factor);
    } else if (background_params.background_type == 1u) {
        // White background
        return vec4<f32>(1.0, 1.0, 1.0, in.grid_fade_factor);
    } else if (background_params.background_type == 2u) {
        // Potential field visualization
        let tex_coord = vec2<i32>(
            i32(in.uv.x * f32(background_params.density_texture_resolution)),
            i32(in.uv.y * f32(background_params.density_texture_resolution))
        );
        
        // Sample the potential field
        let potential = textureLoad(density_texture, tex_coord, 0).r;
        
        // For inverse square field, use power law scaling to better show the falloff
        // The field falls off as 1/r, so we need to enhance the contrast
        let enhanced_potential = pow(potential, 0.5); // Square root to compress high values
        
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
        
        return vec4<f32>(final_color, in.grid_fade_factor);
    }
    
    // Default to black
    return vec4<f32>(0.0, 0.0, 0.0, in.grid_fade_factor);
} 