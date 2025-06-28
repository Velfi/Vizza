// Vertex shader for rendering space colonization branch networks

struct SimParams {
    width: u32,
    height: u32,
    attraction_distance: f32,
    kill_distance: f32,
    segment_length: f32,
    max_attractors: u32,
    max_nodes: u32,
    open_venation: u32,
    enable_vein_thickening: u32,
    min_thickness: f32,
    max_thickness: f32,
    random_seed: u32,
    growth_speed: f32,
    delta_time: f32,
    frame_count: u32,
    enable_opacity_blending: u32,
    min_opacity: f32,
    max_opacity: f32,
    // Curve rendering parameters
    curve_tension: f32, // Default curve tension (0.0 = straight, 1.0 = tight)
    curve_segments: u32, // Number of segments to subdivide curves into
    _padding: f32,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct Node {
    position: vec2<f32>,
    parent_index: u32,
    child_count: u32,
    thickness: f32,
    is_active: u32,
    generation: u32,
    accumulated_direction: vec2<f32>,
    influence_count: u32,
    path_length: f32,
    // Curve control points for smooth rendering
    control_point_1: vec2<f32>, // First control point for cubic Bézier
    control_point_2: vec2<f32>, // Second control point for cubic Bézier
    curve_tension: f32, // Controls how tight the curve is (0.0 = straight line, 1.0 = tight curve)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) thickness: f32,
    @location(1) generation: f32,
    @location(2) world_position: vec2<f32>,
    @location(3) line_start: vec2<f32>,
    @location(4) line_end: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> nodes: array<Node>;
@group(0) @binding(2) var<uniform> params: SimParams;

// Evaluate cubic Bézier curve at parameter t
fn cubic_bezier(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>, t: f32) -> vec2<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    
    return p0 * mt3 + p1 * (3.0 * mt2 * t) + p2 * (3.0 * mt * t2) + p3 * t3;
}

// Evaluate derivative of cubic Bézier curve at parameter t
fn cubic_bezier_derivative(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>, t: f32) -> vec2<f32> {
    let t2 = t * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    
    return 3.0 * mt2 * (p1 - p0) + 6.0 * mt * t * (p2 - p1) + 3.0 * t2 * (p3 - p2);
}

// 4 vertices per quad: (endpoint, side)
// vertex_index: 0 = (A, +1), 1 = (A, -1), 2 = (B, +1), 3 = (B, -1)

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var output: VertexOutput;
    
    // For debugging, let's first try a simpler approach
    // Just render each node as a single segment to see if the basic rendering works
    let node_index = instance_index;
    
    // Bounds check for node_index
    if (node_index >= arrayLength(&nodes)) {
        // Return a degenerate triangle (all vertices at origin)
        output.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        output.thickness = 0.0;
        output.generation = 0.0;
        output.world_position = vec2<f32>(0.0, 0.0);
        output.line_start = vec2<f32>(0.0, 0.0);
        output.line_end = vec2<f32>(0.0, 0.0);
        return output;
    }
    
    // Find the segment: node with index = node_index
    let node = nodes[node_index];
    let parent_idx = node.parent_index;
    
    // Only render if valid parent and node is active
    if (parent_idx == 0xFFFFFFFFu || parent_idx >= arrayLength(&nodes) || node.is_active == 0u) {
        // Return a degenerate triangle (all vertices at origin)
        output.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        output.thickness = 0.0;
        output.generation = 0.0;
        output.world_position = vec2<f32>(0.0, 0.0);
        output.line_start = vec2<f32>(0.0, 0.0);
        output.line_end = vec2<f32>(0.0, 0.0);
        return output;
    }
    
    let parent = nodes[parent_idx];
    let p0 = parent.position; // Start point
    let p3 = node.position;   // End point
    
    // For now, let's use straight line rendering to ensure basic functionality works
    // We can add curve rendering back once we confirm the basic rendering is working
    
    // Calculate line direction for thickness offset
    let line_direction = normalize(p3 - p0);
    let perp = vec2<f32>(-line_direction.y, line_direction.x);
    
    // Interpolate thickness and generation along the line
    let thickness = mix(parent.thickness, node.thickness, 0.5);
    let generation = mix(f32(parent.generation), f32(node.generation), 0.5);
    
    // Calculate side offset for thick line rendering
    let side = select(1.0, -1.0, vertex_index % 2u == 1u);
    let offset = perp * side * thickness * 0.002; // Scale down for thinner lines
    
    // Calculate world position
    let world_pos = p0 + offset;
    let world_pos_4d = vec4<f32>(world_pos.x, world_pos.y, 0.0, 1.0);
    let transformed_pos = camera.transform_matrix * world_pos_4d;
    
    output.clip_position = transformed_pos;
    output.thickness = thickness;
    output.generation = generation;
    output.world_position = world_pos;
    output.line_start = p0;
    output.line_end = p3;
    return output;
} 