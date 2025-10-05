use super::settings::{MatrixGenerator, TrailMapFiltering, TypeGenerator};
use crate::simulations::shared::{BackgroundColorMode, PositionGenerator};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct Particle {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub species: u32,
    pub _pad: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub particle_count: usize,
    pub particles: Vec<Particle>,
    pub random_seed: u32,
    pub dt: f32,
    pub cursor_size: f32,
    pub cursor_strength: f32,
    pub traces_enabled: bool,
    pub trace_fade: f32,
    pub edge_fade_strength: f32,
    pub position_generator: PositionGenerator,
    pub type_generator: TypeGenerator,
    pub matrix_generator: MatrixGenerator,
    // Color scheme management (moved from main struct)
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,
    pub background_color_mode: BackgroundColorMode,
    /// Pre-computed exact RGBA colors for each species, used for both UI display and GPU rendering
    /// In LUT mode: contains species_count + 1 colors (background + species)
    /// In non-LUT mode: contains exactly species_count colors, one for each species
    pub species_colors: Vec<[f32; 4]>, // RGBA colors, always up-to-date

    /// Particle size in world space units
    /// Controls the visual size of particles in the simulation
    pub particle_size: f32,
    /// Trail map filtering mode.
    /// Controls how trail textures are sampled during rendering
    pub trail_map_filtering: TrailMapFiltering,
}

impl State {
    pub fn new(
        particle_count: usize,
        species_count: u32,
        width: u32,
        height: u32,
        random_seed: u32,
    ) -> Self {
        let mut particles = Vec::with_capacity(particle_count);
        let mut rng = rand::rngs::StdRng::seed_from_u64(random_seed as u64);

        // Distribute particles evenly among species
        for i in 0..particle_count {
            let species = (i as u32) % species_count;

            particles.push(Particle {
                position: [
                    rng.random_range(0.0..width as f32),
                    rng.random_range(0.0..height as f32),
                ],
                velocity: [0.0, 0.0], // Start with no velocity
                species,
                _pad: 0,
            });
        }

        Self {
            particle_count,
            particles,
            random_seed,
            dt: 0.016,
            cursor_size: 0.5,
            cursor_strength: 5.0,
            traces_enabled: false,
            trace_fade: 0.48,
            edge_fade_strength: 1.0,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
            current_color_scheme: "MATPLOTLIB_ocean".to_string(), // Use a proper default
            color_scheme_reversed: true,
            background_color_mode: BackgroundColorMode::ColorScheme, // Use color scheme mode as default to match main constructor
            // Placeholder values - will be properly initialized when LUT is loaded in main constructor
            species_colors: vec![[0.0, 0.0, 0.0, 1.0]],
            particle_size: 0.1,
            trail_map_filtering: TrailMapFiltering::Nearest,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            particle_count: 1000,
            particles: Vec::new(),
            random_seed: 42,
            dt: 0.016,
            cursor_size: 0.1,
            cursor_strength: 1.0,
            traces_enabled: true,
            trace_fade: 0.95,
            edge_fade_strength: 0.1,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
            current_color_scheme: "MATPLOTLIB_cubehelix".to_string(),
            color_scheme_reversed: true,
            background_color_mode: BackgroundColorMode::ColorScheme,
            species_colors: Vec::new(),
            particle_size: 0.01,
            trail_map_filtering: TrailMapFiltering::Nearest,
        }
    }
}
