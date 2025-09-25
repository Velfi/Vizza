// Fullscreen fade vertex shader for particle traces

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle vertex shader - no vertex buffer needed
@vertex
fn main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate a fullscreen triangle using vertex index
    // This creates a triangle that covers the entire screen:
    // Vertex 0: (-1, -1) bottom-left
    // Vertex 1: (3, -1) bottom-right (extends beyond screen)  
    // Vertex 2: (-1, 3) top-left (extends beyond screen)
    
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),  // Bottom-left
        vec2<f32>(3.0, -1.0),   // Bottom-right (extends past screen)
        vec2<f32>(-1.0, 3.0)    // Top-left (extends past screen)
    );
    
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),    // Bottom-left UV
        vec2<f32>(2.0, 1.0),    // Bottom-right UV (extends past 1.0)
        vec2<f32>(0.0, -1.0)    // Top-left UV (extends past 0.0)
    );
    
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    
    return output;
}
