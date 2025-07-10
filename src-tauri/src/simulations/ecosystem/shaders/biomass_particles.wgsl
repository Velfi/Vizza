// Biomass particles render shader for ecosystem simulation
// Renders decomposable organic matter from dead agents

struct DeadBiomass {
    position: vec2<f32>,
    biomass_amount: f32,
    species_origin: u32,
    decay_time: f32,
    decomposition_progress: f32,
    is_active: u32,
    _pad: u32,
};

struct CameraUniform {
    transform_matrix: mat4x4<f32>,
    position: vec2<f32>,
    zoom: f32,
    aspect_ratio: f32,
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

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> render_params: RenderParams;
@group(0) @binding(2) var<storage, read> biomass_particles: array<DeadBiomass>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) biomass_amount: f32,
    @location(2) decomposition_progress: f32,
    @location(3) world_pos: vec2<f32>,
}

// Simple noise function for organic variation
fn noise(pos: vec2<f32>) -> f32 {
    return fract(sin(dot(pos, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Create organic blob shape
fn organic_blob(uv: vec2<f32>, time: f32, seed: f32, decomposition_progress: f32) -> f32 {
    let center = vec2<f32>(0.5, 0.5);
    let distance = length(uv - center);
    
    // Base circular shape
    var alpha = 1.0 - smoothstep(0.4, 0.5, distance);
    
    // Add organic variation
    let variation = sin(uv.x * 8.0 + seed) * cos(uv.y * 6.0 + seed) * 0.1;
    alpha += variation;
    
    // Add decomposition-based decay
    alpha *= (1.0 - decomposition_progress * 0.3);
    
    return max(alpha, 0.0);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let biomass = biomass_particles[instance_index];
    
    // Skip inactive biomass
    if (biomass.is_active == 0u) {
        return VertexOutput(
            vec4<f32>(0.0, 0.0, 0.0, 0.0),
            vec2<f32>(0.0, 0.0),
            0.0,
            0.0,
            vec2<f32>(0.0, 0.0)
        );
    }
    
    // Create quad vertices (6 vertices for 2 triangles)
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
    
    // Calculate particle size based on biomass amount - should be smaller than agents
    let base_size = 0.0015; // Smaller base size, comparable to small bacteria
    let biomass_factor = clamp(biomass.biomass_amount / 50.0, 0.5, 2.0);
    let size = base_size * biomass_factor;
    
    // Add decomposition-based size reduction
    let decomposition_size_factor = 1.0 - biomass.decomposition_progress * 0.4;
    let final_size = size * decomposition_size_factor;
    
    // Add subtle pulsing animation
    let pulse = sin(render_params.time * 2.0 + f32(instance_index) * 0.1) * 0.05 + 1.0;
    let final_size_with_pulse = final_size * pulse;
    
    // Apply size and position
    let world_pos = biomass.position + quad_pos * final_size_with_pulse;
    
    // Transform to clip space
    let out = VertexOutput(
        camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0),
        uv,
        biomass.biomass_amount,
        biomass.decomposition_progress,
        world_pos
    );
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create organic blob shape
    let seed = noise(in.world_pos * 50.0) * 6.28318;
    let blob_alpha = organic_blob(in.uv, render_params.time, seed, in.decomposition_progress);
    
    if (blob_alpha < 0.1) {
        discard;
    }
    
    // Base color - dark brownish for decomposing organic matter
    let base_color = vec3<f32>(0.4, 0.25, 0.15);
    
    // Add decomposition-based color variation
    let decomposition_color = mix(
        vec3<f32>(0.4, 0.25, 0.15), // Fresh biomass - dark brown
        vec3<f32>(0.2, 0.15, 0.1),  // Decomposed biomass - darker
        in.decomposition_progress
    );
    
    // Add biomass amount variation
    let biomass_factor = clamp(in.biomass_amount / 50.0, 0.3, 1.0);
    let biomass_color = mix(
        vec3<f32>(0.3, 0.2, 0.1), // Low biomass - darker
        vec3<f32>(0.5, 0.3, 0.2), // High biomass - lighter
        biomass_factor
    );
    
    // Add center glow effect
    let center_distance = length(in.uv - vec2<f32>(0.5, 0.5));
    let glow = 1.0 - center_distance * 1.2;
    let glow_color = biomass_color * max(glow, 0.4);
    
    // Add decomposition progress indicator (green tint for active decomposition)
    let decomposition_indicator = vec3<f32>(0.1, 0.3, 0.1) * in.decomposition_progress * 0.5;
    let final_color = glow_color + decomposition_indicator;
    
    // Add subtle time-based color variation
    let time_variation = sin(render_params.time * 1.5 + seed) * 0.05 + 0.95;
    let final_color_with_variation = final_color * time_variation;
    
    // Apply opacity based on decomposition progress
    let base_opacity = 0.9;
    let decomposition_opacity = 1.0 - in.decomposition_progress * 0.3;
    let final_alpha = blob_alpha * base_opacity * decomposition_opacity;
    
    // Add slight green glow for decomposition activity
    let decomposition_glow = vec3<f32>(0.1, 0.4, 0.1) * in.decomposition_progress * 0.3;
    
    return vec4<f32>(final_color_with_variation + decomposition_glow, final_alpha);
} 