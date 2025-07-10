// Background shader for chemical field and environmental visualization
// Renders chemical gradients, light zones, and environmental overlays

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec2<f32>,
    @location(2) grid_fade_factor: f32,
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
@group(0) @binding(2) var<storage, read> chemical_field: array<f32>;

// Chemical type constants
const OXYGEN = 0u;
const CARBON_DIOXIDE = 1u;
const NITROGEN_COMPOUNDS = 2u;
const PHEROMONES = 3u;
const TOXINS = 4u;
const ATTRACTANTS = 5u;

// Sample chemical concentration at world position
fn sample_chemical_at_pos(world_pos: vec2<f32>, chemical_type: u32) -> f32 {
    // Convert from world space [-1, 1] to chemical grid coordinates
    let grid_x = clamp((world_pos.x + 1.0) / 2.0, 0.0, 1.0) * render_params.chemical_resolution;
    let grid_y = clamp((world_pos.y + 1.0) / 2.0, 0.0, 1.0) * render_params.chemical_resolution;
    
    let x = u32(grid_x) % u32(render_params.chemical_resolution);
    let y = u32(grid_y) % u32(render_params.chemical_resolution);
    
    let index = (y * u32(render_params.chemical_resolution) + x) * 6u + chemical_type;
    
    if (index >= arrayLength(&chemical_field)) {
        return 0.0;
    }
    
    return chemical_field[index];
}

// Generate environmental gradients
fn sample_light_gradient(world_pos: vec2<f32>) -> f32 {
    let distance_from_center = length(world_pos);
    return max(0.0, 1.0 - distance_from_center * 0.4);
}

fn sample_temperature_gradient(world_pos: vec2<f32>) -> f32 {
    let distance_from_center = length(world_pos);
    return 0.5 + (1.0 - distance_from_center) * 0.5;
}

fn sample_ph_gradient(world_pos: vec2<f32>) -> f32 {
    let ph = 7.0 + sin(world_pos.x * 3.0) * cos(world_pos.y * 2.0) * 2.0;
    return (ph - 5.0) / 6.0; // Normalize to [0, 1]
}

// Color palettes for different chemical types
fn get_oxygen_color(concentration: f32) -> vec3<f32> {
    // Blue-green for oxygen
    let intensity = pow(concentration, 0.7);
    return vec3<f32>(0.0, 0.6 * intensity, 0.8 * intensity);
}

fn get_co2_color(concentration: f32) -> vec3<f32> {
    // Orange-red for CO2
    let intensity = pow(concentration, 0.8);
    return vec3<f32>(0.9 * intensity, 0.4 * intensity, 0.1 * intensity);
}

fn get_nitrogen_color(concentration: f32) -> vec3<f32> {
    // Yellow-brown for nitrogen compounds
    let intensity = pow(concentration, 0.6);
    return vec3<f32>(0.8 * intensity, 0.7 * intensity, 0.2 * intensity);
}

fn get_pheromone_color(concentration: f32) -> vec3<f32> {
    // Purple for pheromones
    let intensity = pow(concentration, 0.5);
    return vec3<f32>(0.6 * intensity, 0.2 * intensity, 0.8 * intensity);
}

fn get_toxin_color(concentration: f32) -> vec3<f32> {
    // Dark red for toxins
    let intensity = pow(concentration, 0.4);
    return vec3<f32>(0.8 * intensity, 0.1 * intensity, 0.1 * intensity);
}

fn get_attractant_color(concentration: f32) -> vec3<f32> {
    // Bright green for attractants
    let intensity = pow(concentration, 0.6);
    return vec3<f32>(0.3 * intensity, 0.9 * intensity, 0.4 * intensity);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Create fullscreen quad vertices (6 vertices for 2 triangles)
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),  // Bottom-left
        vec2<f32>( 1.0, -1.0),  // Bottom-right
        vec2<f32>(-1.0,  1.0),  // Top-left
        vec2<f32>( 1.0, -1.0),  // Bottom-right
        vec2<f32>( 1.0,  1.0),  // Top-right
        vec2<f32>(-1.0,  1.0)   // Top-left
    );
    
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),  // Bottom-left
        vec2<f32>(1.0, 0.0),  // Bottom-right
        vec2<f32>(0.0, 1.0),  // Top-left
        vec2<f32>(1.0, 0.0),  // Bottom-right
        vec2<f32>(1.0, 1.0),  // Top-right
        vec2<f32>(0.0, 1.0)   // Top-left
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
    var world_position = vec2<f32>(
        pos[vertex_index].x + f32(grid_x) * 2.0, // Offset by full world width
        pos[vertex_index].y + f32(grid_y) * 2.0  // Offset by full world height
    );
    
    // Apply camera transformation to get clip position
    out.clip_position = camera.transform_matrix * vec4<f32>(world_position, 0.0, 1.0);
    out.uv = uv[vertex_index];
    out.world_pos = world_position;
    out.grid_fade_factor = grid_fade_factor;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var final_color = vec3<f32>(0.0, 0.0, 0.0);
    
    // Sample chemical fields if enabled
    if (render_params.show_chemical_fields != 0u) {
        var chemical_color = vec3<f32>(0.0, 0.0, 0.0);
        
        // Only show chemical types that are enabled in the current visual mode
        if (render_params.show_oxygen != 0u) {
            let oxygen_conc = sample_chemical_at_pos(in.world_pos, OXYGEN);
            chemical_color += get_oxygen_color(oxygen_conc);
        }
        
        if (render_params.show_co2 != 0u) {
            let co2_conc = sample_chemical_at_pos(in.world_pos, CARBON_DIOXIDE);
            chemical_color += get_co2_color(co2_conc);
        }
        
        if (render_params.show_nitrogen != 0u) {
            let nitrogen_conc = sample_chemical_at_pos(in.world_pos, NITROGEN_COMPOUNDS);
            chemical_color += get_nitrogen_color(nitrogen_conc);
        }
        
        if (render_params.show_pheromones != 0u) {
            let pheromone_conc = sample_chemical_at_pos(in.world_pos, PHEROMONES);
            chemical_color += get_pheromone_color(pheromone_conc);
        }
        
        if (render_params.show_toxins != 0u) {
            let toxin_conc = sample_chemical_at_pos(in.world_pos, TOXINS);
            chemical_color += get_toxin_color(toxin_conc);
        }
        
        if (render_params.show_attractants != 0u) {
            let attractant_conc = sample_chemical_at_pos(in.world_pos, ATTRACTANTS);
            chemical_color += get_attractant_color(attractant_conc);
        }
        
        // Apply chemical field opacity
        final_color += chemical_color * render_params.chemical_field_opacity;
    }
    
    // Environmental overlays
    if (render_params.show_light_gradient != 0u) {
        let light_intensity = sample_light_gradient(in.world_pos);
        let light_color = vec3<f32>(1.0, 1.0, 0.8) * light_intensity;
        final_color += light_color * render_params.environmental_opacity * 0.3;
    }
    
    if (render_params.show_temperature_zones != 0u) {
        let temperature = sample_temperature_gradient(in.world_pos);
        let temp_color = vec3<f32>(temperature, 0.0, 1.0 - temperature);
        final_color += temp_color * render_params.environmental_opacity * 0.2;
    }
    
    if (render_params.show_ph_zones != 0u) {
        let ph = sample_ph_gradient(in.world_pos);
        let ph_color = vec3<f32>(ph, ph * 0.5, 0.0);
        final_color += ph_color * render_params.environmental_opacity * 0.15;
    }
    
    // Add subtle animation to make it feel alive
    let time_factor = sin(render_params.time * 0.5) * 0.1 + 0.9;
    final_color *= time_factor;
    
    // Ensure we don't oversaturate
    final_color = min(final_color, vec3<f32>(1.0, 1.0, 1.0));
    
    // Apply grid fade factor for 3x3 grid mode
    final_color = final_color * in.grid_fade_factor;
    
    // Discard completely transparent pixels for performance
    if (length(final_color) <= 0.0) {
        discard;
    }
    
    return vec4<f32>(final_color, 1.0);
} 