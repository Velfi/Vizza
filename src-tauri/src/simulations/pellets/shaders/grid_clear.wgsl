// Clear the spatial partitioning grid

struct GridParams {
    particle_count: u32,
    grid_width: u32,
    grid_height: u32,
    cell_size: f32,
    world_width: f32,
    world_height: f32,
    _pad1: u32,
    _pad2: u32,
}

struct GridCell {
    particle_count: u32,
    particle_indices: array<u32, 64>,
}

@group(0) @binding(0) var<storage, read_write> grid: array<GridCell>;
@group(0) @binding(1) var<uniform> params: GridParams;
// Atomic per-cell particle counts used for concurrent population
@group(0) @binding(2) var<storage, read_write> grid_counts: array<atomic<u32>>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let total_cells = params.grid_width * params.grid_height;
    
    if (index >= total_cells) {
        return;
    }
    
    // Clear the grid cell
    grid[index].particle_count = 0u;
    
    // Clear particle indices array
    for (var i = 0u; i < 64u; i++) {
        grid[index].particle_indices[i] = 0u;
    }

    // Reset atomic cell count
    atomicStore(&grid_counts[index], 0u);
} 