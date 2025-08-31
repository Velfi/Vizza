struct BackgroundParams {
    background_color_mode: u32, // 0 = black, 1 = white, 2 = gray18, 3 = color scheme
}

@group(0) @binding(0)
var<uniform> background_params: BackgroundParams;

@group(0) @binding(1)
var<uniform> background_color: vec4<f32>;


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
    if (background_params.background_color_mode == 0u) {
        // Black background
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if (background_params.background_color_mode == 1u) {
        // White background
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if (background_params.background_color_mode == 2u) {
        // Gray18 background
        return vec4<f32>(0.18, 0.18, 0.18, 1.0);
    } else if (background_params.background_color_mode == 3u) {
        // Color scheme background - use the LUT background color
        return background_color;
    }
    
    // Default to black
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
} 