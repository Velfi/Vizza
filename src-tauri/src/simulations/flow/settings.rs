use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NoiseType {
    Perlin,
    Simplex,
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
                Self::Perlin => "Perlin",
                Self::Simplex => "Simplex",
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
        Self::Perlin
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Black,
    White,
    Vectors,
}

impl Display for Background {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Black => "Black",
                Self::White => "White",
                Self::Vectors => "Vector Field",
            }
        )
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::Black
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Flow field parameters
    pub noise_type: NoiseType,
    pub noise_seed: u32,
    pub noise_scale: f64,
    pub vector_spacing: f32,
    pub vector_magnitude: f32,

    // Particle parameters
    pub particle_limit: u32, // Kept for backward compatibility, no longer used for limiting
    pub autospawn_limit: u32, // New setting for limiting autospawned particles
    pub particle_lifetime: f32,
    pub particle_speed: f32,
    pub particle_size: u32,
    pub particle_shape: ParticleShape,
    pub particle_autospawn: bool,
    pub particle_spawn_rate: f32, // 0.0 = no spawn, 1.0 = full spawn rate

    // Trail parameters
    pub trail_decay_rate: f32,
    pub trail_deposition_rate: f32,
    pub trail_diffusion_rate: f32,
    pub trail_wash_out_rate: f32,

    // Visual parameters
    pub background: Background,
    pub current_lut: String,
    pub lut_reversed: bool,
    pub show_particles: bool,
    pub display_mode: DisplayMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Flow field parameters
            noise_type: NoiseType::Perlin,
            noise_seed: 42,
            noise_scale: 1.0,
            vector_spacing: 0.1,
            vector_magnitude: 0.1,

            // Particle parameters
            particle_limit: 50000,  // Kept for backward compatibility
            autospawn_limit: 50000, // New setting for autospawn limit
            particle_lifetime: 3.0, // 3 seconds
            particle_speed: 0.02,   // Consistent speed for all particles
            particle_size: 4,
            particle_shape: ParticleShape::Circle,
            particle_autospawn: true,
            particle_spawn_rate: 0.1, // 10% spawn rate by default

            // Trail parameters
            trail_decay_rate: 0.0,      // No trail decay by default
            trail_deposition_rate: 1.0, // Maximum trail deposition strength
            trail_diffusion_rate: 0.0,  // No trail diffusion by default
            trail_wash_out_rate: 0.0,

            // Visual parameters
            background: Background::Black,
            current_lut: "MATPLOTLIB_viridis".to_string(),
            lut_reversed: false,
            show_particles: true,
            display_mode: DisplayMode::Age,
        }
    }
}
