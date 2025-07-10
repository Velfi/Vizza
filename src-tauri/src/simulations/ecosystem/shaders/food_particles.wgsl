// Food particle shader for ecosystem simulation
// Renders food particles as organic-looking blobs with pulsing animation

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) energy: f32,
    @location(2) world_pos: vec2<f32>,
}

struct Food {
    position: vec2<f32>,
    energy: f32,
    is_active: u32,
}

struct RenderParams {
    show_energy_as_size: u32,
    time: f32,
    
    show_chemical_fields: u32,
    chemical_field_opacity: f32,
    show_light_gradient: u32,
    environmental_opacity: f32,
    
    chemical_resolution: f32,
    
    // Individual chemical type flags
    show_oxygen: u32,
    show_co2: u32,
    show_nitrogen: u32,
    show_pheromones: u32,
    show_toxins: u32,
    show_attractants: u32,
    
    // Environmental overlay flags
    show_temperature_zones: u32,
    show_ph_zones: u32,

    _pad: u32,
    _pad2: u32,
    _pad3: u32,
}

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> render_params: RenderParams;
@group(0) @binding(2) var<storage, read> food_particles: array<Food>;

// Simple noise function for organic variation
fn noise(p: vec2<f32>) -> f32 {
    let K1 = vec2<f32>(23.14069263277926, 2.665144142690225);
    return fract(cos(dot(p, K1)) * 12345.6789);
}

// Generate organic blob shape
fn organic_blob(uv: vec2<f32>, time: f32, seed: f32) -> f32 {
    let center = vec2<f32>(0.5, 0.5);
    let offset = uv - center;
    
    // Create irregular blob shape using sine waves
    let angle = atan2(offset.y, offset.x);
    let radius = length(offset);
    
    // Generate organic distortion
    let distortion1 = sin(angle * 3.0 + time * 2.0 + seed) * 0.1;
    let distortion2 = sin(angle * 7.0 + time * 1.5 + seed * 2.0) * 0.05;
    let distortion3 = sin(angle * 11.0 + time * 0.8 + seed * 3.0) * 0.03;
    
    let organic_radius = 0.4 + distortion1 + distortion2 + distortion3;
    
    // Create soft edge
    let edge_softness = 0.1;
    let distance_from_edge = organic_radius - radius;
    
    return smoothstep(-edge_softness, edge_softness, distance_from_edge);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Skip inactive food particles
    let food = food_particles[instance_index];
    if (food.is_active == 0u) {
        // Move off-screen
        out.clip_position = vec4<f32>(-10.0, -10.0, 0.0, 1.0);
        return out;
    }
    
    // Create quad vertices
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
    
    // Calculate particle size based on energy
    let base_size = 0.004; // Small nutrient particles
    let energy_factor = clamp(food.energy / 15.0, 0.5, 1.5);
    let size = base_size * energy_factor;
    
    // Add subtle pulsing animation
    let pulse = sin(render_params.time * 3.0 + f32(instance_index) * 0.1) * 0.1 + 1.0;
    let final_size = size * pulse;
    
    // Apply size and position
    let world_pos = food.position + quad_pos * final_size;
    
    // Transform to clip space
    out.clip_position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = uv;
    out.energy = food.energy;
    out.world_pos = world_pos;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create organic blob shape
    let seed = noise(in.world_pos * 100.0) * 6.28318; // Random seed based on position
    let blob_alpha = organic_blob(in.uv, render_params.time, seed);
    
    if (blob_alpha < 0.1) {
        discard;
    }
    
    // Base color - warm brownish-yellow for organic matter
    let base_color = vec3<f32>(0.8, 0.6, 0.3);
    
    // Add energy-based variation
    let energy_factor = clamp(in.energy / 15.0, 0.3, 1.0);
    let energy_color = mix(
        vec3<f32>(0.6, 0.4, 0.2), // Low energy - darker
        vec3<f32>(0.9, 0.7, 0.4), // High energy - brighter
        energy_factor
    );
    
    // Add center glow effect
    let center_distance = length(in.uv - vec2<f32>(0.5, 0.5));
    let glow = 1.0 - center_distance * 1.5;
    let glow_color = energy_color * max(glow, 0.3);
    
    // Add subtle time-based color variation
    let time_variation = sin(render_params.time * 2.0 + seed) * 0.1 + 0.9;
    let final_color = glow_color * time_variation;
    
    // Apply opacity
    let final_alpha = blob_alpha * 0.8;
    
    // Add slight attractant chemical glow (green tint)
    let attractant_glow = vec3<f32>(0.2, 0.4, 0.2) * 0.3;
    let enhanced_color = final_color + attractant_glow;
    
    return vec4<f32>(enhanced_color, final_alpha);
} 