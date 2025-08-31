// Tile-based rendering shader for improved particle life pipeline
// This shader renders individual tiles with camera-aware detail levels

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    species: u32,
    _pad: u32,
}

struct SimParams {
    particle_count: u32,
    species_count: u32,
    max_force: f32,
    max_distance: f32,
    friction: f32,
    wrap_edges: u32,
    width: f32,
    height: f32,
    random_seed: u32,
    dt: f32,
    beta: f32,
    cursor_x: f32,
    cursor_y: f32,
    cursor_size: f32,
    cursor_strength: f32,
    cursor_active: u32,
    brownian_motion: f32,
    particle_size: f32,
    aspect_ratio: f32,
    _pad1: u32,
}

struct TileParams {
    tile_x: i32,
    tile_y: i32,
    camera_zoom: f32,
    _pad0: f32, // Padding for 16-byte alignment
    world_bounds: vec4<f32>, // [left, bottom, right, top] for this tile
    texture_size: vec2<f32>, // [width, height] of tile texture
    _pad1: f32,
    _pad2: f32,
}

struct CameraAwareParams {
    camera_zoom: f32,
    _pad0: f32, // Padding for 16-byte alignment
    camera_position: vec2<f32>,
    viewport_size: vec2<f32>,
    tile_size: f32,
    max_tiles: u32,
    _pad1: f32,
    _pad2: f32,
}

struct SpeciesColors {
    colors: array<vec4<f32>, 9>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) species: u32,
    @location(1) velocity_magnitude: f32,
    @location(2) world_pos: vec2<f32>,
    @location(3) tile_fade_factor: f32,
    @location(4) uv: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> sim_params: SimParams;
@group(1) @binding(0) var<uniform> tile_params: TileParams;
@group(1) @binding(1) var<uniform> camera_aware_params: CameraAwareParams;
@group(2) @binding(0) var<uniform> species_colors: SpeciesColors;

// Calculate LOD factor based on camera zoom
fn calculate_lod_factor(camera_zoom: f32) -> f32 {
    // Base LOD factor - higher zoom = higher detail
    let base_lod = 1.0;
    
    // Adjust LOD based on zoom level
    // At high zoom (close), we want full detail
    // At low zoom (far), we want reduced detail
    let zoom_factor = clamp(camera_zoom / 2.0, 0.5, 2.0);
    
    return base_lod * zoom_factor;
}

// Calculate tile fade factor based on distance from camera
fn calculate_tile_fade_factor(tile_center: vec2<f32>, camera_pos: vec2<f32>, camera_zoom: f32) -> f32 {
    // Calculate distance from camera to tile center
    let distance = distance(tile_center, camera_pos);
    
    // Base fade distance - tiles beyond this start to fade
    let base_fade_distance = 2.0;
    
    // Adjust fade distance based on zoom level
    let fade_distance = base_fade_distance / camera_zoom;
    
    // Calculate fade factor (1.0 = fully visible, 0.0 = fully faded)
    let fade_factor = 1.0 - clamp(distance / fade_distance, 0.0, 1.0);
    
    // Apply smooth falloff
    return smoothstep(0.0, 1.0, fade_factor);
}

// Check if particle is visible within tile bounds
fn is_particle_visible_in_tile(particle_pos: vec2<f32>, world_bounds: vec4<f32>) -> bool {
    let left = world_bounds.x;
    let bottom = world_bounds.y;
    let right = world_bounds.z;
    let top = world_bounds.w;
    
    // Add some margin to ensure particles at edges are rendered
    let margin = 0.1;
    
    return particle_pos.x >= left - margin &&
           particle_pos.x <= right + margin &&
           particle_pos.y >= bottom - margin &&
           particle_pos.y <= top + margin;
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    // Full-screen quad positions for instanced rendering
    let quad_positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    
    let quad_uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    
    let particle = particles[instance_index];
    let quad_pos = quad_positions[vertex_index];
    let quad_uv = quad_uvs[vertex_index];
    
    // Check if particle is visible in this tile
    if (!is_particle_visible_in_tile(particle.position, tile_params.world_bounds)) {
        // Return a position that will be clipped
        var output: VertexOutput;
        output.position = vec4<f32>(0.0, 0.0, -1.0, 1.0);
        output.species = particle.species;
        output.velocity_magnitude = 0.0;
        output.world_pos = particle.position;
        output.tile_fade_factor = 0.0;
        output.uv = quad_uv;
        return output;
    }
    
    // Map world position to tile NDC space [-1, 1]
    let world_left = tile_params.world_bounds.x;
    let world_bottom = tile_params.world_bounds.y;
    let world_right = tile_params.world_bounds.z;
    let world_top = tile_params.world_bounds.w;
    
    let tile_ndc_x = (particle.position.x - world_left) / (world_right - world_left) * 2.0 - 1.0;
    let tile_ndc_y = (particle.position.y - world_bottom) / (world_top - world_bottom) * 2.0 - 1.0;
    let particle_ndc_pos = vec2<f32>(tile_ndc_x, tile_ndc_y);
    
    // Calculate LOD-aware particle size
    let lod_factor = calculate_lod_factor(tile_params.camera_zoom);
    let base_particle_size = sim_params.particle_size;
    let lod_particle_size = base_particle_size * lod_factor;
    
    // Scale particle size from world units to tile NDC units
    let world_width = world_right - world_left;
    let world_height = world_top - world_bottom;
    let world_scale = min(world_width, world_height) / 2.0;
    let particle_ndc_size = lod_particle_size / world_scale;
    
    // Apply aspect ratio correction
    let aspect_ratio = camera_aware_params.viewport_size.x / camera_aware_params.viewport_size.y;
    let aspect_corrected_quad = vec2<f32>(quad_pos.x / aspect_ratio, quad_pos.y);
    let quad_offset = aspect_corrected_quad * particle_ndc_size;
    let final_pos = particle_ndc_pos + quad_offset;
    
    // Calculate tile fade factor
    let tile_center = vec2<f32>(
        (world_left + world_right) * 0.5,
        (world_bottom + world_top) * 0.5
    );
    let tile_fade_factor = calculate_tile_fade_factor(tile_center, camera_aware_params.camera_position, tile_params.camera_zoom);
    
    var output: VertexOutput;
    output.position = vec4<f32>(final_pos, 0.0, 1.0);
    output.species = particle.species;
    output.velocity_magnitude = length(particle.velocity);
    output.world_pos = particle.position;
    output.tile_fade_factor = tile_fade_factor;
    output.uv = quad_uv;
    
    return output;
}

// Convert linear RGB to sRGB for proper display
fn linear_to_srgb(linear: f32) -> f32 {
    if (linear <= 0.0031308) {
        return linear * 12.92;
    } else {
        return 1.055 * pow(linear, 1.0 / 2.4) - 0.055;
    }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular particles with sharp edges
    let center = vec2<f32>(0.5, 0.5);
    let dist_from_center = distance(input.uv, center);
    
    // Define particle radius - use sharp cutoff for crisp edges
    let particle_radius = 0.45;
    
    // Discard pixels outside the particle radius for circular particles
    if (dist_from_center > particle_radius) {
        discard;
    }
    
    // Get the color for this particle's species
    let species_index = input.species;
    let base_color = species_colors.colors[species_index].rgb;
    
    // Apply tile fade factor
    if (input.tile_fade_factor <= 0.0) {
        discard;
    }
    
    // Apply tile fade factor for smooth transitions
    let faded_color = base_color * input.tile_fade_factor;
    let final_color = vec3<f32>(
        linear_to_srgb(faded_color.r),
        linear_to_srgb(faded_color.g),
        linear_to_srgb(faded_color.b)
    );
    
    return vec4<f32>(final_color, input.tile_fade_factor);
} 