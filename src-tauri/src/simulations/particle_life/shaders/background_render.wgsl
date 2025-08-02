struct BackgroundParams {
    background_color: vec4<f32>, // RGBA color values
}

@group(0) @binding(0)
var<uniform> background_params: BackgroundParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full-screen quad
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    
    let pos = positions[vertex_index];
    
    return VertexOutput(
        vec4<f32>(pos, 0.0, 1.0),
        pos
    );
}

@fragment
fn fs_main(@location(0) pos: vec2<f32>) -> @location(0) vec4<f32> {
    return background_params.background_color;
} 