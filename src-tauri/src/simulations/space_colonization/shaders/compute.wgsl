// Space Colonization Compute Shader
// Implements the iterative growth algorithm for organic branching structures

struct SimParams {
    width: u32,
    height: u32,
    attraction_distance: f32,
    kill_distance: f32,
    segment_length: f32,
    max_attractors: u32,
    max_nodes: u32,
    open_venation: u32, // 0 = closed, 1 = open
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

struct Attractor {
    position: vec2<f32>,
    is_active: u32, // 0 = inactive, 1 = active
    influence_count: u32, // Number of nodes currently influenced by this attractor
}

struct Node {
    position: vec2<f32>,
    parent_index: u32, // Index of parent node, u32::MAX if root
    child_count: u32, // Number of children
    thickness: f32,
    is_active: u32, // 0 = inactive, 1 = active, 2 = growing tip
    generation: u32, // Distance from root (for thickness calculation)
    accumulated_direction: vec2<f32>, // Sum of directions from all influencing attractors
    influence_count: u32, // Number of attractors influencing this node
    path_length: f32, // Total path length from root to this node
    // Curve control points for smooth rendering
    control_point_1: vec2<f32>, // First control point for cubic Bézier
    control_point_2: vec2<f32>, // Second control point for cubic Bézier
    curve_tension: f32, // Controls how tight the curve is (0.0 = straight line, 1.0 = tight curve)
}

struct MouseParams {
    is_active: u32, // 0=inactive, 1=attract, 2=repel
    x: f32,
    y: f32,
    size: f32,
    density: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read_write> attractors: array<Attractor>;
@group(0) @binding(2) var<storage, read_write> nodes: array<Node>;
@group(0) @binding(3) var<storage, read_write> counters: array<atomic<u32>, 4>; // [active_attractors, active_nodes, new_nodes_this_frame, unused]
@group(0) @binding(4) var<uniform> mouse_params: MouseParams;

// PRNG state
var<workgroup> rng_state: u32;

// Simple hash function for PRNG
fn hash(x: u32) -> u32 {
    var h = x;
    h ^= h >> 16u;
    h *= 0x85ebca6bu;
    h ^= h >> 13u;
    h *= 0xc2b2ae35u;
    h ^= h >> 16u;
    return h;
}

// Generate random float [0, 1)
fn random() -> f32 {
    rng_state = hash(rng_state);
    return f32(rng_state) / 4294967296.0;
}

// Generate random float in range [min, max)
fn random_range(min_val: f32, max_val: f32) -> f32 {
    return min_val + random() * (max_val - min_val);
}

// Generate random point in circle
fn random_in_circle(center: vec2<f32>, radius: f32) -> vec2<f32> {
    let angle = random() * 6.28318530718; // 2π
    let r = sqrt(random()) * radius;
    return center + vec2<f32>(cos(angle) * r, sin(angle) * r);
}

// Distance between two points
fn distance(a: vec2<f32>, b: vec2<f32>) -> f32 {
    let delta = a - b;
    return sqrt(delta.x * delta.x + delta.y * delta.y);
}

// Normalize vector
fn safe_normalize(v: vec2<f32>) -> vec2<f32> {
    let len = sqrt(v.x * v.x + v.y * v.y);
    if (len < 0.0001) {
        return vec2<f32>(0.0, 0.0);
    }
    return v / len;
}

// Check if point is within canvas bounds (normalized coordinates)
fn in_bounds(pos: vec2<f32>) -> bool {
    return pos.x >= -1.0 && pos.x <= 1.0 && 
           pos.y >= -1.0 && pos.y <= 1.0;
}

// Convert normalized coordinates to pixel coordinates
fn normalized_to_pixel(normalized: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (normalized.x + 1.0) * 0.5 * f32(params.width),
        (1.0 - normalized.y) * 0.5 * f32(params.height)
    );
}

// Convert pixel coordinates to normalized coordinates
fn pixel_to_normalized(pixel: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (pixel.x / f32(params.width)) * 2.0 - 1.0,
        1.0 - (pixel.y / f32(params.height)) * 2.0
    );
}

// Initialize attractors for open venation (tree-like growth)
@compute @workgroup_size(64, 1, 1)
fn init_attractors(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.max_attractors) {
        return;
    }
    
    // Initialize RNG for this thread
    rng_state = hash(params.random_seed + index);
    
    // Create attractor distribution optimized for sustained growth
    // This creates organic patterns that encourage long-term tree-like growth
    var x: f32;
    var y: f32;
    
    // Use different patterns based on index to create organic distribution
    let pattern_seed = index % 10u; // More patterns for better distribution
    
    switch(pattern_seed) {
        case 0u: { // Main trunk attractors (central line)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let curve = sin(t * 3.14159) * 0.05; // Very slight curve
            x = curve + random_range(-0.04, 0.04);
            y = -0.9 + t * 1.8; // From base to top
        }
        case 1u: { // Primary branch attractors (left side)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let branch_t = random_range(0.1, 0.9);
            let angle = random_range(-0.6, 0.6); // More angle variation
            x = -0.5 - branch_t * 0.7 + angle * 0.5;
            y = -0.7 + t * 1.7;
        }
        case 2u: { // Primary branch attractors (right side)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let branch_t = random_range(0.1, 0.9);
            let angle = random_range(-0.6, 0.6);
            x = 0.5 + branch_t * 0.7 + angle * 0.5;
            y = -0.7 + t * 1.7;
        }
        case 3u: { // Secondary branch attractors (left)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let branch_t = random_range(0.2, 0.8);
            x = -0.7 - branch_t * 0.6;
            y = -0.5 + t * 1.5;
        }
        case 4u: { // Secondary branch attractors (right)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let branch_t = random_range(0.2, 0.8);
            x = 0.7 + branch_t * 0.6;
            y = -0.5 + t * 1.5;
        }
        case 5u: { // Tertiary branch attractors (left)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let branch_t = random_range(0.3, 0.7);
            x = -0.9 - branch_t * 0.5;
            y = -0.3 + t * 1.3;
        }
        case 6u: { // Tertiary branch attractors (right)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let branch_t = random_range(0.3, 0.7);
            x = 0.9 + branch_t * 0.5;
            y = -0.3 + t * 1.3;
        }
        case 7u: { // Distant attractors for sustained growth (left)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let angle = random_range(-0.8, 0.8);
            let radius = random_range(0.6, 1.0);
            x = -0.8 - radius * cos(angle);
            y = -0.2 + t * 1.2;
        }
        case 8u: { // Distant attractors for sustained growth (right)
            let t = f32(index / 10u) / f32(params.max_attractors / 10u);
            let angle = random_range(-0.8, 0.8);
            let radius = random_range(0.6, 1.0);
            x = 0.8 + radius * cos(angle);
            y = -0.2 + t * 1.2;
        }
        case 9u: { // Random fill for organic shape
            // Create more organic distribution using polar coordinates
            let angle = random() * 6.28318530718;
            let radius = random_range(0.1, 1.0);
            // Use elliptical shape for tree-like form
            x = cos(angle) * radius * 1.0;
            y = sin(angle) * radius * 1.1 + 0.1;
        }
        default: {
            x = random_range(-0.9, 0.9);
            y = random_range(-0.9, 0.9);
        }
    }
    
    // Add organic randomness to make it more natural
    x += random_range(-0.04, 0.04);
    y += random_range(-0.04, 0.04);
    
    // Ensure bounds with some margin
    x = clamp(x, -0.95, 0.95);
    y = clamp(y, -0.95, 0.95);
    
    attractors[index].position = vec2<f32>(x, y);
    attractors[index].is_active = 1u;
    attractors[index].influence_count = 0u;
    
    if (index == 0u) {
        atomicStore(&counters[0], params.max_attractors);
    }
}

// Initialize root nodes (run once at startup)
@compute @workgroup_size(64, 1, 1)
fn init_nodes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.max_nodes) {
        return;
    }
    
    // Initialize RNG for this thread
    rng_state = hash(params.random_seed + index + 1000u);
    
    if (index < 1u) {
        // Create a single root node at the bottom center (trunk base)
        nodes[index].position = vec2<f32>(0.0, -0.9);
        nodes[index].parent_index = 0xFFFFFFFFu; // No parent (root)
        nodes[index].child_count = 0u;
        nodes[index].thickness = params.max_thickness; // Thickest at base
        nodes[index].is_active = 2u; // Growing tip
        nodes[index].generation = 0u;
        nodes[index].accumulated_direction = vec2<f32>(0.0, 0.0);
        nodes[index].influence_count = 0u;
        nodes[index].path_length = 0.0;
        // Initialize curve control points
        nodes[index].control_point_1 = vec2<f32>(0.0, -0.9);
        nodes[index].control_point_2 = vec2<f32>(0.0, -0.9);
        nodes[index].curve_tension = params.curve_tension;
        
        atomicStore(&counters[1], 1u); // 1 initial root node
    } else {
        // Initialize unused nodes
        nodes[index].is_active = 0u;
        nodes[index].position = vec2<f32>(0.0, 0.0);
        nodes[index].parent_index = 0xFFFFFFFFu;
        nodes[index].child_count = 0u;
        nodes[index].thickness = 0.0;
        nodes[index].generation = 0u;
        nodes[index].accumulated_direction = vec2<f32>(0.0, 0.0);
        nodes[index].influence_count = 0u;
        nodes[index].path_length = 0.0;
        // Initialize curve control points
        nodes[index].control_point_1 = vec2<f32>(0.0, 0.0);
        nodes[index].control_point_2 = vec2<f32>(0.0, 0.0);
        nodes[index].curve_tension = 0.0;
    }
}

// Reset node influence accumulation
@compute @workgroup_size(64, 1, 1)
fn reset_influences(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.max_nodes) {
        return;
    }
    
    if (nodes[index].is_active > 0u) {
        nodes[index].accumulated_direction = vec2<f32>(0.0, 0.0);
        nodes[index].influence_count = 0u;
    }
}

// Calculate influences between attractors and nodes (open venation)
@compute @workgroup_size(64, 1, 1)
fn calculate_influences(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let attractor_index = global_id.x;
    if (attractor_index >= params.max_attractors || attractors[attractor_index].is_active == 0u) {
        return;
    }
    
    let attractor_pos = attractors[attractor_index].position;
    var closest_node_index = 0xFFFFFFFFu;
    var closest_distance = params.attraction_distance + 1.0;
    var influence_count = 0u;
    
    // Find the closest growing tip within attraction distance
    for (var node_index = 0u; node_index < params.max_nodes; node_index++) {
        if (nodes[node_index].is_active != 2u) { // Only growing tips can be influenced
            continue;
        }
        
        let node_pos = nodes[node_index].position;
        let dist = distance(attractor_pos, node_pos);
        
        if (dist <= params.attraction_distance) {
            influence_count++;
            
            // For open venation: always associate with the closest node only
            if (dist < closest_distance) {
                closest_distance = dist;
                closest_node_index = node_index;
            }
        }
    }
    
    // If we found an influencing node, add direction (open venation)
    if (closest_node_index != 0xFFFFFFFFu && closest_distance <= params.attraction_distance) {
        let direction = safe_normalize(attractor_pos - nodes[closest_node_index].position);
        
        // Add to accumulated direction
        nodes[closest_node_index].accumulated_direction += direction;
        nodes[closest_node_index].influence_count++;
    }
    
    attractors[attractor_index].influence_count = influence_count;
}

// Grow new nodes based on accumulated influences with open venation branching
@compute @workgroup_size(64, 1, 1)
fn grow_nodes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_index = global_id.x;
    if (node_index >= params.max_nodes || nodes[node_index].is_active != 2u) {
        return;
    }
    
    // Skip if this node has no influences
    if (nodes[node_index].influence_count == 0u) {
        return;
    }
    
    // Average the accumulated direction
    let avg_direction = nodes[node_index].accumulated_direction / f32(nodes[node_index].influence_count);
    let normalized_direction = safe_normalize(avg_direction);
    
    // Calculate new node position
    let new_position = nodes[node_index].position + normalized_direction * params.segment_length;
    
    // Check bounds
    if (!in_bounds(new_position)) {
        return;
    }
    
    // Determine branching strategy for open venation
    let generation = nodes[node_index].generation;
    let influence_count = nodes[node_index].influence_count;
    
    // More aggressive branching based on influence count
    var max_branches = 1u;
    
    // Base branching on influence count (more attractors = more branches)
    if (influence_count >= 5u) {
        max_branches = 3u; // High influence = many branches
    } else if (influence_count >= 3u) {
        max_branches = 2u; // Medium influence = some branches
    } else if (influence_count >= 1u) {
        max_branches = 1u; // Low influence = single branch
    }
    
    // Adjust based on generation (allow more branching in early generations)
    if (generation < 3u) {
        max_branches = min(max_branches + 1u, 4u); // Extra branch for early generations
    }
    
    // Create branches
    for (var branch = 0u; branch < max_branches; branch++) {
        var branch_direction = normalized_direction;
        
        // Add variation for branching (more variation for multiple branches)
        if (branch > 0u) {
            let angle_offset = random_range(-0.5, 0.5); // ±30 degrees for more dramatic branching
            let cos_angle = cos(angle_offset);
            let sin_angle = sin(angle_offset);
            branch_direction = vec2<f32>(
                cos_angle * normalized_direction.x - sin_angle * normalized_direction.y,
                sin_angle * normalized_direction.x + cos_angle * normalized_direction.y
            );
        }
        
        let branch_position = nodes[node_index].position + branch_direction * params.segment_length;
        
        // Check bounds for branch
        if (!in_bounds(branch_position)) {
            continue;
        }
        
        // Find a free node slot for the new node
        for (var new_node_index = 0u; new_node_index < params.max_nodes; new_node_index++) {
            if (nodes[new_node_index].is_active == 0u) {
                // Create new node
                nodes[new_node_index].position = branch_position;
                nodes[new_node_index].parent_index = node_index;
                nodes[new_node_index].child_count = 0u;
                nodes[new_node_index].thickness = params.min_thickness;
                nodes[new_node_index].is_active = 2u; // Growing tip
                nodes[new_node_index].generation = generation + 1u;
                nodes[new_node_index].accumulated_direction = vec2<f32>(0.0, 0.0);
                nodes[new_node_index].influence_count = 0u;
                nodes[new_node_index].path_length = nodes[node_index].path_length + params.segment_length;
                
                // Calculate curve control points for smooth rendering
                let parent_pos = nodes[node_index].position;
                let grandparent_idx = nodes[node_index].parent_index;
                var grandparent_pos = vec2<f32>(0.0, 0.0);
                
                // Get grandparent position if it exists
                if (grandparent_idx != 0xFFFFFFFFu && grandparent_idx < params.max_nodes) {
                    grandparent_pos = nodes[grandparent_idx].position;
                } else {
                    // For root node, use parent position as grandparent
                    grandparent_pos = parent_pos;
                }
                
                // Calculate control points for the curve from parent to new node
                let cp1 = calculate_curve_control_points(grandparent_pos, parent_pos, branch_position, params.curve_tension);
                let cp2 = calculate_curve_control_points(parent_pos, branch_position, branch_position + branch_direction * params.segment_length, params.curve_tension);
                
                nodes[new_node_index].control_point_1 = cp1;
                nodes[new_node_index].control_point_2 = cp2;
                nodes[new_node_index].curve_tension = params.curve_tension;
                
                // Update parent node
                nodes[node_index].child_count++;
                
                // Update counters
                atomicAdd(&counters[1], 1u); // active_nodes
                atomicAdd(&counters[2], 1u); // new_nodes_this_frame
                
                break;
            }
        }
    }
    
    // Only mark parent as no longer a growing tip if it has multiple children
    // This allows single branches to continue growing
    if (nodes[node_index].child_count > 1u) {
        nodes[node_index].is_active = 1u; // No longer a growing tip
    }
    // If it has only one child, keep it as a growing tip to allow continued growth
}

// Remove attractors that are within kill distance of nodes (more conservative)
@compute @workgroup_size(64, 1, 1)
fn prune_attractors(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let attractor_index = global_id.x;
    if (attractor_index >= params.max_attractors || attractors[attractor_index].is_active == 0u) {
        return;
    }
    
    let attractor_pos = attractors[attractor_index].position;
    var should_kill = false;
    
    // Check distance to all active nodes
    for (var node_index = 0u; node_index < params.max_nodes; node_index++) {
        if (nodes[node_index].is_active == 0u) {
            continue;
        }
        
        let node_pos = nodes[node_index].position;
        let dist = distance(attractor_pos, node_pos);
        
        // Only kill if very close to a node (more conservative)
        if (dist <= params.kill_distance * 0.5) { // Use half the kill distance for more conservative pruning
            should_kill = true;
            break;
        }
    }
    
    if (should_kill) {
        attractors[attractor_index].is_active = 0u;
        atomicSub(&counters[0], 1u); // active_attractors
    }
}

// Update node thickness based on descendants (Murray's law)
@compute @workgroup_size(64, 1, 1)
fn update_thickness(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_index = global_id.x;
    if (node_index >= params.max_nodes || nodes[node_index].is_active == 0u || params.enable_vein_thickening == 0u) {
        return;
    }
    
    // For growing tips (leaf nodes), thickness is just the minimum
    if (nodes[node_index].child_count == 0u) {
        nodes[node_index].thickness = params.min_thickness;
        return;
    }
    
    // For branch nodes, thickness is based on number of descendants and generation
    // This creates more realistic branching where thicker branches support more leaves
    let base_thickness = params.min_thickness;
    let child_factor = f32(nodes[node_index].child_count) * 0.4;
    let generation_factor = f32(nodes[node_index].generation) * 0.15;
    
    let thickness_increment = base_thickness * (child_factor + generation_factor);
    nodes[node_index].thickness = min(base_thickness + thickness_increment, params.max_thickness);
}

// Calculate curve control points for smooth Bézier interpolation
fn calculate_curve_control_points(
    prev_pos: vec2<f32>,
    current_pos: vec2<f32>,
    next_pos: vec2<f32>,
    tension: f32
) -> vec2<f32> {
    // If we don't have a previous or next point, use straight line
    // Compare vector components individually since direct vector comparison is not allowed
    let prev_eq_current = all(prev_pos == current_pos);
    let next_eq_current = all(next_pos == current_pos);
    
    if (prev_eq_current || next_eq_current) {
        return current_pos;
    }
    
    // Calculate the direction vectors
    let dir_to_current = normalize(current_pos - prev_pos);
    let dir_from_current = normalize(next_pos - current_pos);
    
    // Calculate the average direction (tangent)
    let tangent = normalize(dir_to_current + dir_from_current);
    
    // Calculate the distance for control points
    let dist_to_prev = distance(current_pos, prev_pos);
    let dist_to_next = distance(current_pos, next_pos);
    let avg_distance = (dist_to_prev + dist_to_next) * 0.5;
    
    // Control point distance based on tension
    let control_distance = avg_distance * tension * 0.3;
    
    // Return the control point in the tangent direction
    return current_pos + tangent * control_distance;
} 