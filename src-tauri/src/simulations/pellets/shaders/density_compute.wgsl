// GPU-based density calculation for particle coloring

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

struct DensityParams {
    particle_count: u32,
    density_radius: f32,
    coloring_mode: u32, // 0 = density, 1 = velocity, 2 = random
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: DensityParams;
// Optional: if grid is available, we could switch to neighbor-based density later

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var particle = particles[index];
    
    if (params.coloring_mode == 0u) {
        // Density mode
        particle.density = compute_density(particle, index);
    } else if (params.coloring_mode == 1u) {
        // Velocity mode
        particle.density = length(particle.velocity) * 4.0; // Velocity-based coloring
    } else {
        // Random mode - use particle index to generate consistent random value
        let seed = f32(index) * 0.1234;
        particle.density = fract(sin(seed) * 43758.5453) * 255.0; // Random value 0-255
    }
    
    particles[index] = particle;
}

fn compute_density(particle: Particle, particle_index: u32) -> f32 {
    var density = 0.0;
    let density_radius_sq = params.density_radius * params.density_radius;
    
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == particle_index) {
            continue;
        }
        
        let other = particles[i];
        var delta = particle.position - other.position;
        
        // Toroidal wrapping
        if (abs(delta.x) > 1.0) {
            delta.x = delta.x - sign(delta.x) * 2.0;
        }
        if (abs(delta.y) > 1.0) {
            delta.y = delta.y - sign(delta.y) * 2.0;
        }
        
        let distance_sq = dot(delta, delta);
        
        if (distance_sq < density_radius_sq) {
            density += 1.0 / (1.0 + distance_sq);
        }
    }
    
    return density;
} 