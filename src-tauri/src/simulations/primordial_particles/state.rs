use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum BackgroundColorMode {
    Black,
    White,
    Gray18,
    ColorScheme,
}

impl fmt::Display for BackgroundColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Black => "Black",
                Self::White => "White",
                Self::Gray18 => "Gray18",
                Self::ColorScheme => "Color Scheme",
            }
        )
    }
}

impl BackgroundColorMode {
    pub(crate) fn from_str(mode_str: &str) -> Option<Self> {
        match mode_str {
            "Black" => Some(BackgroundColorMode::Black),
            "White" => Some(BackgroundColorMode::White),
            "Gray18" => Some(BackgroundColorMode::Gray18),
            "Color Scheme" => Some(BackgroundColorMode::ColorScheme),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq)]
pub enum ForegroundColorMode {
    Random,
    Density,
    Heading,
    Velocity,
}

impl ForegroundColorMode {
    pub(crate) fn from_str(mode_str: &str) -> Option<Self> {
        match mode_str {
            "Random" => Some(ForegroundColorMode::Random),
            "Density" => Some(ForegroundColorMode::Density),
            "Heading" => Some(ForegroundColorMode::Heading),
            "Velocity" => Some(ForegroundColorMode::Velocity),
            _ => None,
        }
    }
}

impl fmt::Display for ForegroundColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Random => "Random",
                Self::Density => "Density",
                Self::Heading => "Heading",
                Self::Velocity => "Velocity",
            }
        )
    }
}

/// Runtime state for the Primordial Particles simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub particle_count: u32,
    pub particles: Vec<Particle>,
    pub random_seed: u32,
    pub dt: f32,
    pub particle_size: f32,

    /// Position generator type for particle initialization
    /// 0=Random, 1=Center, 2=UniformCircle, 3=CenteredCircle, 4=Ring, 5=Line, 6=Spiral
    pub position_generator: u32,

    /// Color scheme UI state
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,
    pub background_color_mode: BackgroundColorMode,
    pub foreground_color_mode: ForegroundColorMode,

    /// Mouse interaction state
    pub mouse_pressed: bool,
    pub mouse_mode: u32,
    pub mouse_position: [f32; 2],
    pub mouse_velocity: [f32; 2],
    pub mouse_screen_position: [f32; 2],
    pub last_mouse_time: f64,

    /// Cursor interaction parameters
    pub cursor_size: f32,
    pub cursor_strength: f32,

    /// Grabbed particles for drag interaction
    pub grabbed_particles: Vec<usize>,

    /// Trail/trace settings
    pub traces_enabled: bool,
    pub trace_fade: f32,
}

/// Particle data structure for the simulation
#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Pod, Zeroable)]
pub struct Particle {
    pub position: [f32; 2],
    pub previous_position: [f32; 2], // For mouse interaction
    pub heading: f32,
    pub velocity: f32, // Magnitude of velocity for coloring
    pub density: f32,  // Local density for coloring
    pub grabbed: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            particle_count: 10000,
            particles: Vec::new(),
            random_seed: 42,
            dt: 0.016,
            particle_size: 0.01,
            position_generator: 0, // Default to Random

            // Color scheme UI defaults
            current_color_scheme: "MATPLOTLIB_turbo".to_string(),
            color_scheme_reversed: false,
            background_color_mode: BackgroundColorMode::ColorScheme,
            foreground_color_mode: ForegroundColorMode::Heading,

            // Mouse interaction defaults
            mouse_pressed: false,
            mouse_mode: 0,
            mouse_position: [0.0, 0.0],
            mouse_velocity: [0.0, 0.0],
            mouse_screen_position: [0.0, 0.0],
            last_mouse_time: 0.0,
            cursor_size: 0.20,
            cursor_strength: 1.0,
            grabbed_particles: Vec::new(),

            // Trail/trace defaults
            traces_enabled: false,
            trace_fade: 0.48,
        }
    }
}

impl State {
    pub fn new(width: u32, height: u32) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut state = Self::default();

        state.random_seed = rng.random_range(0..u32::MAX);
        state.particles =
            initialize_particles(state.particle_count, width, height, state.random_seed);

        state
    }

    pub(crate) fn regenerate_particles(&mut self, width: u32, height: u32) {
        use rand::Rng;
        let mut rng = rand::rng();
        self.random_seed = rng.random_range(0..u32::MAX);

        self.particles = initialize_particles(self.particle_count, width, height, self.random_seed);
    }
}

fn initialize_particles(count: u32, _width: u32, _height: u32, seed: u32) -> Vec<Particle> {
    use std::f32::consts::PI;

    let mut particles = Vec::with_capacity(count as usize);
    let mut rng_state = seed;

    for _i in 0..count {
        // Simple Xorshift PRNG
        rng_state = rng_state ^ (rng_state << 13);
        rng_state = rng_state ^ (rng_state >> 17);
        rng_state = rng_state ^ (rng_state << 5);
        let rand_x = (rng_state as f32) / (u32::MAX as f32);

        rng_state = rng_state ^ (rng_state << 13);
        rng_state = rng_state ^ (rng_state >> 17);
        rng_state = rng_state ^ (rng_state << 5);
        let rand_y = (rng_state as f32) / (u32::MAX as f32);

        rng_state = rng_state ^ (rng_state << 13);
        rng_state = rng_state ^ (rng_state >> 17);
        rng_state = rng_state ^ (rng_state << 5);
        let rand_heading = (rng_state as f32) / (u32::MAX as f32);

        let particle = Particle {
            position: [rand_x * 2.0 - 1.0, rand_y * 2.0 - 1.0], // Convert [0,1] -> [-1,1]
            previous_position: [0.0, 0.0],
            heading: rand_heading * 2.0 * PI,
            velocity: 0.0, // Will be calculated during simulation
            density: 0.0,  // Will be calculated during simulation
            grabbed: 0,
        };

        particles.push(particle);
    }

    particles
}
