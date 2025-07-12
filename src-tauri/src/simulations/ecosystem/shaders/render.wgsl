// Vertex shader for ecosystem simulation rendering

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) energy: f32,
    @location(2) uv: vec2<f32>,
    @location(3) ecological_role: u32,
    @location(4) variant: u32,
    @location(5) grid_fade_factor: f32,
}

struct Agent {
    position: vec2<f32>,
    velocity: vec2<f32>,
    energy: f32,
    age: f32,
    ecological_role: u32,  // 0: Recycler, 1: Producer, 2: Predator
    variant: u32,          // Variant within ecological role
    
    // Sensor array: 3-4 chemical receptors pointing different directions
    sensor_readings: array<f32, 4>,
    
    // Movement engine parameters for run-and-tumble
    heading: f32,
    run_duration: f32,         // Current run duration in run-and-tumble
    run_timer: f32,            // Timer for current run
    tumble_cooldown: f32,      // Cooldown after tumbling
    
    // Metabolic system
    metabolism_rate: f32,
    reproductive_threshold: f32,
    last_reproduction_time: f32,
    
    // Behavioral state: 0: feeding, 1: hunting, 2: reproducing, 3: escaping
    behavioral_state: u32,
    state_timer: f32,          // Timer for current state
    
    // Chemical secretion rates for 6 chemical types
    chemical_secretion_rates: array<f32, 6>,
    
    // Simple memory: recent food locations and threats
    food_memory: array<f32, 4>,     // x, y positions of recent food
    threat_memory: array<f32, 4>,   // x, y positions of recent threats
    
    // Biofilm formation (for producers)
    biofilm_strength: f32,
    biofilm_connections: u32,
    
    // Hunting mechanics (for predators)
    hunt_target_id: u32,
    pack_coordination: f32,
    
    // Spatial organization
    territory_center: array<f32, 2>,
    territory_radius: f32,
    
    // Visibility control
    is_visible: u32,  // 0 = hidden, 1 = visible
    
    _pad: array<u32, 1>,
}

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
@group(1) @binding(0) var<uniform> render_params: RenderParams;
@group(2) @binding(0) var<storage, read> species_colors: array<vec4<f32>>;
@group(3) @binding(0) var<storage, read> agents: array<Agent>;
@group(3) @binding(1) var<storage, read> visibility_flags: array<u32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // 3x3 grid mode: render each agent 9 times like Particle Life
    let agents_per_grid = arrayLength(&agents);
    let actual_agent_index = instance_index % agents_per_grid;
    let grid_cell_index = instance_index / agents_per_grid;
    
    // Get agent data from storage buffer
    let agent = agents[actual_agent_index];
    
    // Check if this agent's species variant is visible
    let species_index = agent.ecological_role * 3u + agent.variant;
    let is_species_visible = visibility_flags[species_index];
    
    // Calculate grid cell position (0-8, arranged as 3x3 grid)
    let grid_x = i32(grid_cell_index % 3u) - 1; // -1, 0, 1
    let grid_y = i32(grid_cell_index / 3u) - 1; // -1, 0, 1
    
    // Calculate fade factor based on distance from center
    let center_distance = abs(grid_x) + abs(grid_y);
    var grid_fade_factor = 1.0;
    if (center_distance == 0) {
        grid_fade_factor = 1.0; // Center cell - full opacity
    } else if (center_distance == 1) {
        grid_fade_factor = 0.4; // Adjacent cells - medium fade
    } else {
        grid_fade_factor = 0.2; // Corner cells - strong fade
    }
    
    // If species is hidden, move vertex far away to cull it
    if (is_species_visible == 0u) {
        out.clip_position = vec4<f32>(1000.0, 1000.0, 0.0, 1.0); // Move far away
        out.color = vec4<f32>(0.0, 0.0, 0.0, 0.0); // Transparent
        out.energy = 0.0;
        out.uv = vec2<f32>(0.0, 0.0);
        out.ecological_role = 0u;
        out.variant = 0u;
        out.grid_fade_factor = 0.0;
        return out;
    }
    
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
    
    // Use fixed size for all agents like Particle Life
    let fixed_size = 0.008; // Fixed size in world coordinates
    
    // Start with agent world position [-1,1] and offset by grid cell
    // Each grid cell represents a full world tile offset (width/height = 2.0)
    var agent_world_pos = vec2<f32>(
        agent.position.x + f32(grid_x) * 2.0, // Offset by full world width
        agent.position.y + f32(grid_y) * 2.0  // Offset by full world height
    );
    
    // Apply size and position
    let world_pos = agent_world_pos + quad_pos * fixed_size;
    
    // Transform to clip space
    out.clip_position = camera.transform_matrix * vec4<f32>(world_pos, 0.0, 1.0);
    
    // Calculate color index based on ecological role and variant
    let color_index = agent.ecological_role * 3u + agent.variant;
    let color_species_index = min(color_index, u32(arrayLength(&species_colors)) - 1u);
    out.color = species_colors[color_species_index];
    
    // Modify color based on energy level
    let energy_factor = clamp(agent.energy / 50.0, 0.1, 1.0);
    out.color = vec4<f32>(out.color.rgb * energy_factor, out.color.a);
    
    // Add behavioral state indicators
    if (agent.behavioral_state == 1u) { // Hunting
        out.color.r = min(out.color.r + 0.3, 1.0);
    } else if (agent.behavioral_state == 2u) { // Reproducing
        out.color.g = min(out.color.g + 0.3, 1.0);
    } else if (agent.behavioral_state == 3u) { // Escaping
        out.color.b = min(out.color.b + 0.3, 1.0);
    }
    
    // Add biofilm indication for producers
    if (agent.ecological_role == 1u && agent.biofilm_strength > 0.1) {
        // Make biofilm-forming agents slightly brighter
        let biofilm_factor = 1.0 + agent.biofilm_strength * 0.2;
        out.color = vec4<f32>(out.color.rgb * biofilm_factor, out.color.a);
    }
    
    out.energy = agent.energy;
    out.uv = uv;
    out.ecological_role = agent.ecological_role;
    out.variant = agent.variant;
    out.grid_fade_factor = grid_fade_factor;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create ecological role-specific shapes matching the frontend legend
    let center = vec2<f32>(0.5, 0.5);
    let distance = length(in.uv - center);
    
    var is_dead = in.energy <= 0.0;
    var shape_distance = 0.0;
    var outline_distance = 0.0;
    
    // Different shapes for different ecological roles (consistent within each role)
    if (in.ecological_role == 0u) { // Recyclers - all circles
        shape_distance = distance;
        outline_distance = 0.4 - 0.05; // outline thickness
        if (distance > 0.4) { discard; }
    } else if (in.ecological_role == 1u) { // Producers - all diamonds
        shape_distance = abs(in.uv.x - 0.5) + abs(in.uv.y - 0.5);
        outline_distance = 0.4 - 0.05; // outline thickness
        if (shape_distance > 0.4) { discard; }
    } else if (in.ecological_role == 2u) { // Predators - all triangles
        let triangle_uv = vec2<f32>(in.uv.x - 0.5, in.uv.y - 0.5);
        shape_distance = abs(triangle_uv.x) + abs(triangle_uv.y * 1.732);
        outline_distance = 0.4 - 0.05; // outline thickness
        if (shape_distance > 0.4) { discard; }
    } else {
        // Default circular shape
        shape_distance = distance;
        outline_distance = 0.4 - 0.05; // outline thickness
        if (distance > 0.4) { discard; }
    }
    
    // Render dead agents as dim outlines
    if (is_dead) {
        let in_outline = shape_distance > outline_distance;
        
        if (in_outline) {
            return vec4<f32>(0.3, 0.3, 0.3, 0.6);
        } else {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }
    
    // Add energy-based glow effect for living agents
    let glow = 1.0 - shape_distance * 2.0;
    let energy_glow = clamp(in.energy / 100.0, 0.0, 1.0);
    
    var final_color = in.color;
    let glow_color = glow * energy_glow * 0.2;
    
    // Apply grid fade factor for 3x3 grid mode
    final_color = vec4<f32>(final_color.rgb * in.grid_fade_factor, final_color.a);
    
    // Add subtle time-based pulsing for living agents
    let pulse = sin(render_params.time * 2.0) * 0.1 + 0.9;
    final_color = vec4<f32>(final_color.rgb * pulse, final_color.a);
    
    let glow_addition = vec3<f32>(glow_color, glow_color, glow_color * 0.5);
    final_color = vec4<f32>(final_color.rgb + glow_addition, final_color.a);
    
    return final_color;
} 