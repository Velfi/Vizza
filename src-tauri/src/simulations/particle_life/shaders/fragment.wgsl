// Particle Life fragment shader

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) species: u32,
    @location(1) velocity_magnitude: f32,
    @location(2) uv: vec2<f32>,
    @location(3) grid_fade_factor: f32,
}

struct SpeciesColors {
    colors: array<vec4<f32>, 9>, // Allocate space for 9 colors (background + 8 species)
}

@group(1) @binding(0) var<uniform> species_colors: SpeciesColors;

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Get the color for this particle's species directly from the uniform buffer
    let species_index = input.species;
    let base_color = species_colors.colors[species_index].rgb;
    
    // Create circular particles with smooth borders
    let center = vec2<f32>(0.5, 0.5);
    let dist_from_center = distance(input.uv, center);
    
    // Define particle radius - use hard cutoff for opaque particles
    let particle_radius = 0.45;
    
    // Discard pixels outside the particle radius for hard edges
    if (dist_from_center > particle_radius) {
        discard;
    }
    
    // Apply grid fade factor for 3x3 grid mode
    // Since we're using REPLACE blend, we need to handle grid fading differently
    // We'll use the grid fade factor to adjust the color intensity
    let grid_fade_factor = input.grid_fade_factor;
    
    // Return opaque color with grid fade applied to RGB components
    return vec4<f32>(base_color * grid_fade_factor, 1.0);
}