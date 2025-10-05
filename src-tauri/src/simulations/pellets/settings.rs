//! # Pellets Settings Module
//!
//! Defines the user-configurable parameters that control the behavior and appearance
//! of the Pellets simulation. These settings determine how particles interact,
//! how the system evolves over time, and how it appears to the user.
//!
//! ## Configuration Philosophy
//!
//! The settings are designed to provide intuitive control over complex behaviors.
//! Small changes to parameters can dramatically alter the simulation's character,
//! from stable orbital systems to chaotic particle storms. All settings are
//! serializable for preset management and reproducible experimentation.
//!
//! ## Parameter Categories
//!
//! Settings are organized into logical groups that control different aspects
//! of the simulation, from basic particle properties to advanced physics
//! behaviors and visual presentation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackgroundColorMode {
    Black,
    White,
    Gray18,
    #[serde(rename = "Color Scheme")]
    ColorScheme,
}

impl BackgroundColorMode {
    pub(crate) fn from_str(bg_type: &str) -> Option<Self> {
        match bg_type {
            "Black" => Some(BackgroundColorMode::Black),
            "White" => Some(BackgroundColorMode::White),
            "Gray18" => Some(BackgroundColorMode::Gray18),
            "Color Scheme" => Some(BackgroundColorMode::ColorScheme),
            _ => {
                tracing::warn!("Invalid background color mode: {}", bg_type);
                None
            }
        }
    }
}

impl From<&BackgroundColorMode> for u32 {
    fn from(mode: &BackgroundColorMode) -> Self {
        match mode {
            BackgroundColorMode::Black => 0,
            BackgroundColorMode::White => 1,
            BackgroundColorMode::Gray18 => 2,
            BackgroundColorMode::ColorScheme => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForegroundColorMode {
    Density,
    Velocity,
    Random,
}

impl ForegroundColorMode {
    pub(crate) fn from_str(mode: &str) -> Option<Self> {
        match mode {
            "Density" => Some(ForegroundColorMode::Density),
            "Velocity" => Some(ForegroundColorMode::Velocity),
            "Random" => Some(ForegroundColorMode::Random),
            _ => {
                tracing::warn!("Invalid foreground color mode: {}", mode);
                None
            }
        }
    }
}

impl From<&ForegroundColorMode> for u32 {
    fn from(mode: &ForegroundColorMode) -> Self {
        match mode {
            ForegroundColorMode::Density => 0,
            ForegroundColorMode::Velocity => 1,
            ForegroundColorMode::Random => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Number of particles in the simulation
    pub particle_count: u32,

    /// Size of each particle for rendering
    pub particle_size: f32,

    /// Energy retention factor during particle collisions (0.0 = no energy retained, 1.0 = all energy retained)
    /// Works like energy_damping but only applies during collision impacts
    pub collision_damping: f32,

    /// Maximum initial velocity for particles
    pub initial_velocity_max: f32,

    /// Minimum initial velocity for particles
    pub initial_velocity_min: f32,

    /// Random seed for reproducible simulations
    pub random_seed: u32,

    /// Background type: "Black", "White", "Gray18", or "Color Scheme"
    pub background_color_mode: BackgroundColorMode,

    // Physics parameters
    /// Gravitational constant for physics calculations
    pub gravitational_constant: f32,

    /// Energy damping factor
    pub energy_damping: f32,

    /// Gravity softening parameter
    pub gravity_softening: f32,

    /// Density visualization radius
    pub density_radius: f32,

    /// Coloring mode: "Density" or "Velocity"
    pub foreground_color_mode: ForegroundColorMode,

    /// Whether to apply density-based velocity damping
    pub density_damping_enabled: bool,

    /// Strength of overlap resolution (0.0 = no separation, 1.0 = maximum separation)
    /// Controls how aggressively overlapping particles are separated
    pub overlap_resolution_strength: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            particle_count: 5000,
            particle_size: 0.015,
            collision_damping: 1.0,
            initial_velocity_max: 0.1,
            initial_velocity_min: 0.1,
            random_seed: 0,
            background_color_mode: BackgroundColorMode::ColorScheme,
            gravitational_constant: 1e-7,
            energy_damping: 1.0,
            gravity_softening: 0.003,
            density_radius: 0.038,
            foreground_color_mode: ForegroundColorMode::Density,
            density_damping_enabled: false,
            overlap_resolution_strength: 0.02,
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        self.particle_size = rng.random_range(0.001..0.005);
        self.collision_damping = rng.random_range(0.5..0.95); // Similar range to energy_damping
        self.initial_velocity_max = rng.random_range(0.1..0.5);
        self.initial_velocity_min = rng.random_range(0.05..self.initial_velocity_max * 0.7);
        self.random_seed = rng.random();

        // Randomize physics fields
        self.gravitational_constant = rng.random_range(0.003..0.012);
        self.energy_damping = rng.random_range(0.9..0.99); // Adjusted to match collision_damping range
        self.gravity_softening = rng.random_range(0.003..0.008);
        self.density_radius = rng.random_range(0.02..0.05);
        self.density_damping_enabled = rng.random_bool(0.5); // 50% chance of being enabled
        self.overlap_resolution_strength = rng.random_range(0.01..0.2); // Conservative range, max 20%
    }
}
