// Particle Life fragment shader - Instanced Quads for Sizable Points

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) species: u32,
    @location(1) velocity_magnitude: f32,
    @location(2) world_pos: vec2<f32>,
    @location(3) grid_fade_factor: f32,
    @location(4) uv: vec2<f32>,
}

struct SpeciesColors {
    colors: array<vec4<f32>, 9>, // Allocate space for 9 colors (background + 8 species)
}

@group(1) @binding(0) var<uniform> species_colors: SpeciesColors;

@fragment
fn main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular particles with sharp edges
    // UV coordinates are in [0,1] range for each quad
    let center = vec2<f32>(0.5, 0.5);
    let dist_from_center = distance(input.uv, center);
    
    // Define particle radius - use sharp cutoff for crisp edges
    let particle_radius = 0.45;
    
    // Discard pixels outside the particle radius for circular particles
    if (dist_from_center > particle_radius) {
        discard;
    }
    
    // Get the color for this particle's species directly from the uniform buffer
    let species_index = input.species;
    let base_color = species_colors.colors[species_index].rgb;
    
    // When completely faded (grid_fade_factor = 0), render a solid color
    // representing the average of the simulation
    if (input.grid_fade_factor <= 0.0) {
        // Use a dark color that represents the "average" when tiles are too small
        // This gives a sense of the overall simulation state
        return vec4<f32>(0.1, 0.1, 0.15, 1.0);
    }
    
    // Apply grid fade factor for infinite rendering
    // Use the grid fade factor to adjust the color intensity
    let final_color = vec4<f32>(base_color * input.grid_fade_factor, 1.0);
    
    return final_color;
}