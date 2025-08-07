struct SimParams {
    total_pool_size: u32,
    vector_count: u32,
    particle_lifetime: f32,
    particle_speed: f32,
    noise_seed: u32,
    time: f32,
    noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    width: f32,
    height: f32,
    noise_scale: f32,
    noise_x: f32,
    noise_y: f32,
    vector_magnitude: f32,
    trail_decay_rate: f32,
    trail_deposition_rate: f32,
    trail_diffusion_rate: f32,
    trail_wash_out_rate: f32,
    trail_map_width: u32,
    trail_map_height: u32,
    particle_shape: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond
    particle_size: u32, // Particle size in pixels
    screen_width: u32, // Screen width in pixels
    screen_height: u32, // Screen height in pixels
    cursor_x: f32,
    cursor_y: f32,
    cursor_active: u32, // 0=Inactive, 1=Cel, 2=El
    cursor_size: u32,
    cursor_strength: f32,
    particle_autospawn: u32, // 0=disabled, 1=enabled
    particle_spawn_rate: f32, // 0 = no spawn, 1.0 = full spawn rate
    display_mode: u32, // 0=Age, 1=Random, 2=Direction
}

struct FlowVector {
    position: vec2<f32>,
    direction: vec2<f32>,
}

@group(0) @binding(0) var<uniform> sim_params: SimParams;
@group(0) @binding(1) var trail_map: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(2) var<storage, read> flow_vectors: array<FlowVector>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    if (x >= sim_params.trail_map_width || y >= sim_params.trail_map_height) {
        return;
    }
    let map_size = vec2<f32>(f32(sim_params.trail_map_width), f32(sim_params.trail_map_height));
    let world_x = (f32(x) / map_size.x) * 2.0 - 1.0;
    let world_y = (f32(y) / map_size.y) * 2.0 - 1.0;
    let world_pos = vec2<f32>(world_x, world_y);
    var current_trail = textureLoad(trail_map, vec2<i32>(i32(x), i32(y)));
    var current_intensity = current_trail.a;
    var current_color = current_trail.rgb;
    
    // --- Proper advection with inline bilinear sampling ---
    let dt = 1.0; // One time step; can be parameterized
    
    // Inline bilinear sampling for flow field
    let grid_w = 128.0;
    let grid_h = 128.0;
    let tx = (world_pos.x + 1.0) * 0.5 * (grid_w - 1.0);
    let ty = (world_pos.y + 1.0) * 0.5 * (grid_h - 1.0);
    let x0 = u32(floor(tx));
    let x1 = u32(min(ceil(tx), grid_w - 1.0));
    let y0 = u32(floor(ty));
    let y1 = u32(min(ceil(ty), grid_h - 1.0));
    let fx = tx - f32(x0);
    let fy = ty - f32(y0);
    let idx00 = y0 * u32(grid_w) + x0;
    let idx10 = y0 * u32(grid_w) + x1;
    let idx01 = y1 * u32(grid_w) + x0;
    let idx11 = y1 * u32(grid_w) + x1;
    let v00 = flow_vectors[idx00].direction;
    let v10 = flow_vectors[idx10].direction;
    let v01 = flow_vectors[idx01].direction;
    let v11 = flow_vectors[idx11].direction;
    let v0 = mix(v00, v10, fx);
    let v1 = mix(v01, v11, fx);
    let flow = mix(v0, v1, fy);
    
    let back_pos_world = world_pos - flow * sim_params.particle_speed * dt;
    let back_x = (back_pos_world.x + 1.0) * 0.5 * map_size.x;
    let back_y = (back_pos_world.y + 1.0) * 0.5 * map_size.y;
    
    // Inline bilinear sampling for trail map
    let px = clamp(back_x, 0.0, map_size.x - 1.0);
    let py = clamp(back_y, 0.0, map_size.y - 1.0);
    let tx0 = i32(floor(px));
    let tx1 = i32(min(ceil(px), map_size.x - 1.0));
    let ty0 = i32(floor(py));
    let ty1 = i32(min(ceil(py), map_size.y - 1.0));
    let tfx = px - f32(tx0);
    let tfy = py - f32(ty0);
    let c00 = textureLoad(trail_map, vec2<i32>(tx0, ty0));
    let c10 = textureLoad(trail_map, vec2<i32>(tx1, ty0));
    let c01 = textureLoad(trail_map, vec2<i32>(tx0, ty1));
    let c11 = textureLoad(trail_map, vec2<i32>(tx1, ty1));
    let c0 = mix(c00, c10, tfx);
    let c1 = mix(c01, c11, tfx);
    let advected_trail = mix(c0, c1, tfy);
    
    let advection_blend = sim_params.trail_wash_out_rate;
    current_intensity = mix(current_intensity, advected_trail.a, advection_blend);
    current_color = mix(current_color, advected_trail.rgb, advection_blend);
    
    // --- Decay and diffusion as before ---
    var new_intensity = current_intensity * (1.0 - sim_params.trail_decay_rate);
    var new_color = current_color;
    let diffusion_rate = sim_params.trail_diffusion_rate;
    if (diffusion_rate > 0.0) {
        let x_prev = (x + sim_params.trail_map_width - 1u) % sim_params.trail_map_width;
        let x_next = (x + 1u) % sim_params.trail_map_width;
        let y_prev = (y + sim_params.trail_map_height - 1u) % sim_params.trail_map_height;
        let y_next = (y + 1u) % sim_params.trail_map_height;
        let left = textureLoad(trail_map, vec2<i32>(i32(x_prev), i32(y)));
        let right = textureLoad(trail_map, vec2<i32>(i32(x_next), i32(y)));
        let up = textureLoad(trail_map, vec2<i32>(i32(x), i32(y_prev)));
        let down = textureLoad(trail_map, vec2<i32>(i32(x), i32(y_next)));
        let neighbor_intensity_avg = (left.a + right.a + up.a + down.a) * 0.25;
        let neighbor_color_avg = (left.rgb + right.rgb + up.rgb + down.rgb) * 0.25;
        new_intensity = new_intensity * (1.0 - diffusion_rate) + neighbor_intensity_avg * diffusion_rate;
        new_color = new_color * (1.0 - diffusion_rate) + neighbor_color_avg * diffusion_rate;
    }
    textureStore(trail_map, vec2<i32>(i32(x), i32(y)), vec4<f32>(new_color, new_intensity));
} 