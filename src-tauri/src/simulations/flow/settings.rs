use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NoiseType {
    OpenSimplex,
    Worley,
    Value,
    Fbm,
    FBMBillow,
    FBMClouds,
    FBMRidged,
    Billow,
    RidgedMulti,
    Cylinders,
    Checkerboard,
}

impl Display for NoiseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OpenSimplex => "OpenSimplex",
                Self::Worley => "Worley",
                Self::Value => "Value",
                Self::Fbm => "FBM",
                Self::FBMBillow => "FBM Billow",
                Self::FBMClouds => "FBM Clouds",
                Self::FBMRidged => "FBM Ridged",
                Self::Billow => "Billow",
                Self::RidgedMulti => "Ridged Multi",
                Self::Cylinders => "Cylinders",
                Self::Checkerboard => "Checkerboard",
            }
        )
    }
}

impl Default for NoiseType {
    fn default() -> Self {
        Self::OpenSimplex
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Black,
    White,
    Lut,
}

impl Display for Background {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Black => "Black",
                Self::White => "White",
                Self::Lut => "LUT",
            }
        )
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::Lut
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ParticleShape {
    Circle,
    Square,
    Triangle,
    Star,
    Diamond,
}

impl Display for ParticleShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Circle => "Circle",
                Self::Square => "Square",
                Self::Triangle => "Triangle",
                Self::Star => "Flower",
                Self::Diamond => "Diamond",
            }
        )
    }
}

impl Default for ParticleShape {
    fn default() -> Self {
        Self::Circle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DisplayMode {
    Age,
    Random,
    Direction,
}

impl Display for DisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Age => "Age",
                Self::Random => "Random",
                Self::Direction => "Direction",
            }
        )
    }
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self::Age
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrailMapFiltering {
    Nearest,
    Linear,
}

impl Display for TrailMapFiltering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Nearest => "Nearest",
                Self::Linear => "Linear",
            }
        )
    }
}

impl Default for TrailMapFiltering {
    fn default() -> Self {
        Self::Nearest
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Flow field parameters
    pub noise_type: NoiseType,
    pub noise_seed: u32,
    pub noise_scale: f64,
    pub noise_x: f64,
    pub noise_y: f64,
    pub noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    pub vector_magnitude: f32,

    // Particle parameters
    pub total_pool_size: u32, // Total number of particles (autospawn + brush)
    pub particle_lifetime: f32,
    pub particle_speed: f32,
    pub particle_size: u32,
    pub particle_shape: ParticleShape,
    pub particle_autospawn: bool,
    pub autospawn_rate: u32,   // Particles per second for autospawn
    pub brush_spawn_rate: u32, // Particles per second when cursor is active

    // Trail parameters
    pub trail_decay_rate: f32,
    pub trail_deposition_rate: f32,
    pub trail_diffusion_rate: f32,
    pub trail_wash_out_rate: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Flow field parameters
            noise_type: NoiseType::OpenSimplex,
            noise_seed: 0,
            noise_scale: 1.0,
            noise_x: 1.0,
            noise_y: 1.0,
            noise_dt_multiplier: 0.0, // Default multiplier
            vector_magnitude: 0.1,

            // Particle parameters
            total_pool_size: 10000, // Total particle pool size (will be divided evenly between autospawn and brush)
            particle_lifetime: 3.0, // 3 seconds
            particle_speed: 0.02,   // Consistent speed for all particles
            particle_size: 4,
            particle_shape: ParticleShape::Circle,
            particle_autospawn: true,
            autospawn_rate: 100,  // Default autospawn rate
            brush_spawn_rate: 10, // Default brush spawn rate

            // Trail parameters
            trail_decay_rate: 0.0,      // No trail decay by default
            trail_deposition_rate: 1.0, // Maximum trail deposition strength
            trail_diffusion_rate: 0.0,  // No trail diffusion by default
            trail_wash_out_rate: 0.0,
        }
    }
}
