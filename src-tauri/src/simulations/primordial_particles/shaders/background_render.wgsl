// Primordial Particles Background Render Shader
// Renders a simple background

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct BackgroundParams {
    background_color: vec4<f32>, // RGBA color values
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> background_params: BackgroundParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Create fullscreen quad
    var pos: vec2<f32>;
    switch (vertex_index) {
        case 0u: { pos = vec2<f32>(-1.0, -1.0); }
        case 1u: { pos = vec2<f32>(1.0, -1.0); }
        case 2u: { pos = vec2<f32>(-1.0, 1.0); }
        case 3u: { pos = vec2<f32>(1.0, -1.0); }
        case 4u: { pos = vec2<f32>(1.0, 1.0); }
        case 5u: { pos = vec2<f32>(-1.0, 1.0); }
        default: { pos = vec2<f32>(0.0, 0.0); }
    }
    
    // Convert screen coordinates to world coordinates using proper transformation
    let world_pos = (camera.transform_matrix * vec4<f32>(pos, 0.0, 1.0)).xy;
    
    return VertexOutput(
        vec4<f32>(pos, 0.0, 1.0),
        world_pos
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return background_params.background_color;
}


