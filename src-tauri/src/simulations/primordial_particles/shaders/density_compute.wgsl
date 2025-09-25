// GPU-based density calculation for PPS particle coloring

struct Particle {
    position: vec2<f32>,
    previous_position: vec2<f32>,
    heading: f32,
    velocity: f32,
    density: f32,
    grabbed: u32,
}

struct DensityParams {
    particle_count: u32,
    density_radius: f32,
    coloring_mode: u32, // 0 = random, 1 = density, 2 = heading, 3 = velocity
    _padding: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: DensityParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.particle_count) {
        return;
    }

    var particle = particles[index];
    
    // Always compute density here. Rendering decides how to use it.
    // This avoids stale or incorrect values when UI state changes.
    particle.density = compute_density(particle, index);
    
    particles[index] = particle;
}

fn compute_density(particle: Particle, particle_index: u32) -> f32 {
    var density: f32 = 0.0;
    let density_radius_sq = params.density_radius * params.density_radius;
    
    for (var i = 0u; i < params.particle_count; i++) {
        if (i == particle_index) {
            continue;
        }
        
        let other = particles[i];
        var delta = particle.position - other.position;
        
        // Toroidal wrapping for PPS world space [-1,1]
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

