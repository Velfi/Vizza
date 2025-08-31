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

struct ColorMode {
    mode: u32, // 0=Gray18, 1=White, 2=Black, 3=ColorScheme
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(1) @binding(0) var<uniform> species_colors: SpeciesColors;
@group(1) @binding(1) var<uniform> color_mode: ColorMode;

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
    
    // Species colors are stored first in the buffer for all modes.
    // When using a LUT, the background color is appended at the end by the backend.
    let color_index = species_index;
    
    let base_color = species_colors.colors[color_index].rgb;
    
    // When completely faded (grid_fade_factor = 0), render a color based on the species
    // This gives a better representation of the simulation state than a fixed dark color
    if (input.grid_fade_factor <= 0.0) {
        // Use a dimmed version of the species color to represent the average
        // This creates a more dynamic fade that reflects the simulation content
        let dimmed_color = base_color * 0.15;
        let srgb_color = vec3<f32>(
            dimmed_color.r,
            dimmed_color.g,
            dimmed_color.b
        );
        return vec4<f32>(srgb_color, 1.0);
    }
    
    // Apply grid fade factor for infinite rendering
    // Use the grid fade factor to adjust the color intensity
    let faded_color = base_color * input.grid_fade_factor;
    let final_color = vec3<f32>(
        faded_color.r,
        faded_color.g,
        faded_color.b
    );
    
    return vec4<f32>(final_color, 1.0);
}