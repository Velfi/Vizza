// Spatial partitioning grid population shader
// Creates a uniform grid for efficient neighbor lookups

struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    mass: f32,
    radius: f32,
    clump_id: u32,
    density: f32,
    grabbed: u32,
    _pad0: u32,
    previous_position: vec2<f32>,
}

struct GridParams {
    particle_count: u32,
    grid_width: u32,
    grid_height: u32,
    cell_size: f32,
    world_width: f32,  // 2.0 for [-1,1] space
    world_height: f32, // 2.0 for [-1,1] space
    _pad1: u32,
    _pad2: u32,
}

// Grid cell structure - each cell stores particle indices
struct GridCell {
    particle_count: u32,
    particle_indices: array<u32, 64>, // Max 64 particles per cell
}

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> grid: array<GridCell>;
@group(0) @binding(2) var<uniform> params: GridParams;
// Atomic per-cell particle counts for safe concurrent insertion
@group(0) @binding(3) var<storage, read_write> grid_counts: array<atomic<u32>>;

// Convert world position to grid coordinates
fn world_to_grid(pos: vec2<f32>) -> vec2<u32> {
    // Convert from [-1,1] to [0,1] then to grid coordinates
    let normalized_pos = (pos + vec2<f32>(1.0, 1.0)) * 0.5;
    let grid_x = u32(normalized_pos.x * f32(params.grid_width));
    let grid_y = u32(normalized_pos.y * f32(params.grid_height));
    
    // Clamp to grid bounds
    return vec2<u32>(
        min(grid_x, params.grid_width - 1u),
        min(grid_y, params.grid_height - 1u)
    );
}

// Get grid cell index from coordinates
fn grid_coord_to_index(coord: vec2<u32>) -> u32 {
    return coord.y * params.grid_width + coord.x;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }
    
    let particle = particles[index];
    
    // Skip grabbed particles (they don't need physics)
    if (particle.grabbed != 0u) {
        return;
    }
    
    // Convert particle position to grid coordinates
    let grid_coord = world_to_grid(particle.position);
    let cell_index = grid_coord_to_index(grid_coord);
    
    // Concurrently add particle to grid cell using atomic counter
    let slot = atomicAdd(&grid_counts[cell_index], 1u);
    if (slot < 64u) {
        // Write the index directly to the slot to avoid racy struct writebacks
        grid[cell_index].particle_indices[slot] = index;
    }
} 