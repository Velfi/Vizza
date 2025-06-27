// quad_3x3.wgsl: 3x3 instanced rendering for slime mold with grid fade factors

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) grid_fade_factor: f32,
};

@group(0) @binding(0)
var display_tex: texture_2d<f32>;
@group(0) @binding(1)
var display_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

// Vertex shader for 3x3 instanced rendering with grid fade factors
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Create a quad that covers the full screen area
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    
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
    var world_pos = vec2<f32>(
        pos[vertex_index].x + f32(grid_x) * 2.0, // Offset by full world width
        pos[vertex_index].y + f32(grid_y) * 2.0  // Offset by full world height
    );
    
    var out: VertexOutput;
    out.position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = uv[vertex_index];
    out.grid_fade_factor = grid_fade_factor;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(display_tex, display_sampler, in.uv);
    
    // Apply grid fade factor
    let final_color = vec4<f32>(base_color.rgb, base_color.a * in.grid_fade_factor);
    
    // Discard completely transparent pixels for performance
    if (final_color.a <= 0.0) {
        discard;
    }
    
    return final_color;
} 