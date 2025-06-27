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
    
    // Create circular particles with anti-aliased edges
    let center = vec2<f32>(0.5, 0.5);
    let dist_from_center = distance(input.uv, center);
    
    // Define particle radius and anti-aliasing falloff
    let particle_radius = 0.45;
    let falloff_start = 0.35; // Start alpha falloff here
    let falloff_end = particle_radius; // End falloff at particle edge
    
    // Calculate alpha with smooth falloff for anti-aliasing
    var alpha: f32 = 1.0;
    
    if (dist_from_center > falloff_start) {
        // Smooth falloff from falloff_start to falloff_end
        let falloff_range = falloff_end - falloff_start;
        let falloff_progress = (dist_from_center - falloff_start) / falloff_range;
        // Use smoothstep for a nice S-curve falloff
        alpha = 1.0 - smoothstep(0.0, 1.0, falloff_progress);
    }
    
    // Apply grid fade factor for 3x3 grid mode
    alpha = alpha * input.grid_fade_factor;
    
    // Discard completely transparent pixels for performance
    if (alpha <= 0.0) {
        discard;
    }
    
    // Return color with calculated alpha for smooth anti-aliased edges
    return vec4<f32>(base_color, alpha);
}