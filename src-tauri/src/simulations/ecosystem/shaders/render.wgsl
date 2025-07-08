// Vertex shader for ecosystem simulation rendering

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) energy: f32,
    @location(2) uv: vec2<f32>,
}

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
    energy: f32,
    age: f32,
    species: u32,
    _pad1: u32,
    neural_weights: array<f32, 8>,
    sensor_readings: array<f32, 4>,
    goal: u32,
    _pad2: u32,
    memory: array<f32, 4>,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

struct RenderParams {
    show_energy_as_size: u32,
    show_sensors: u32,
    trail_opacity: f32,
    time: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> render_params: RenderParams;
@group(2) @binding(0) var<storage, read> species_colors: array<vec4<f32>>;
@group(3) @binding(0) var<storage, read> agents: array<Agent>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Get agent data from storage buffer
    let agent = agents[instance_index];
    
    // Create quad vertices based on vertex_index
    var quad_pos = vec2<f32>(0.0, 0.0);
    var uv = vec2<f32>(0.0, 0.0);
    
    if (vertex_index == 0u) { 
        quad_pos = vec2<f32>(-0.5, -0.5); 
        uv = vec2<f32>(0.0, 0.0);
    }
    else if (vertex_index == 1u) { 
        quad_pos = vec2<f32>( 0.5, -0.5); 
        uv = vec2<f32>(1.0, 0.0);
    }
    else if (vertex_index == 2u) { 
        quad_pos = vec2<f32>( 0.5,  0.5); 
        uv = vec2<f32>(1.0, 1.0);
    }
    else if (vertex_index == 3u) { 
        quad_pos = vec2<f32>(-0.5, -0.5); 
        uv = vec2<f32>(0.0, 0.0);
    }
    else if (vertex_index == 4u) { 
        quad_pos = vec2<f32>( 0.5,  0.5); 
        uv = vec2<f32>(1.0, 1.0);
    }
    else if (vertex_index == 5u) { 
        quad_pos = vec2<f32>(-0.5,  0.5); 
        uv = vec2<f32>(0.0, 1.0);
    }
    
    // Calculate agent size based on energy if enabled
    // Use much smaller sizes for [-1, 1] coordinate system
    var size = 0.01;
    if (render_params.show_energy_as_size != 0u) {
        size = mix(0.005, 0.02, clamp(agent.energy / 100.0, 0.0, 1.0));
    }
    
    // Apply size and position
    let world_pos = agent.position + quad_pos * size;
    
    // Transform to clip space
    out.clip_position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    
    // Set color based on species
    let species_index = min(agent.species, u32(arrayLength(&species_colors)) - 1u);
    out.color = species_colors[species_index];
    
    // Modify color based on energy level
    let energy_factor = clamp(agent.energy / 50.0, 0.1, 1.0);
    out.color = vec4<f32>(out.color.rgb * energy_factor, out.color.a);
    
    out.energy = agent.energy;
    out.uv = uv;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular agents using UV coordinates
    let center = vec2<f32>(0.5, 0.5);
    let distance = length(in.uv - center);
    
    if (distance > 0.5) {
        discard;
    }
    
    // Add energy-based glow effect
    let glow = 1.0 - distance * 2.0;
    let energy_glow = clamp(in.energy / 100.0, 0.0, 1.0);
    
    var final_color = in.color;
    let glow_color = glow * energy_glow * 0.3;
    final_color.r += glow_color;
    final_color.g += glow_color;
    final_color.b += glow_color;
    
    return final_color;
} 