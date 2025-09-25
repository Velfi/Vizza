struct BackgroundParams {
    background_type: u32, // 0 = black, 1 = white
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0)
var<uniform> background_params: BackgroundParams;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

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
    }
    
    // Default to black
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
} 