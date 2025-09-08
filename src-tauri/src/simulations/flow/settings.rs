use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VectorFieldType {
    Noise,
    Image,
}

impl Display for VectorFieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Noise => "Noise",
                Self::Image => "Image",
            }
        )
    }
}

impl Default for VectorFieldType {
    fn default() -> Self {
        Self::Noise
    }
}

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
pub enum BackgroundColorMode {
    Black,
    White,
    Gray18,
    ColorScheme,
}

impl Display for BackgroundColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Black => "Black",
                Self::White => "White",
                Self::Gray18 => "Gray18",
                Self::ColorScheme => "ColorScheme",
            }
        )
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

impl Default for BackgroundColorMode {
    fn default() -> Self {
        Self::ColorScheme
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
pub enum ForegroundColorMode {
    Age,
    Random,
    Direction,
}

impl From<&ForegroundColorMode> for u32 {
    fn from(mode: &ForegroundColorMode) -> Self {
        match mode {
            ForegroundColorMode::Age => 0,
            ForegroundColorMode::Random => 1,
            ForegroundColorMode::Direction => 2,
        }
    }
}

impl Display for ForegroundColorMode {
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

impl Default for ForegroundColorMode {
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GradientImageFitMode {
    Stretch,
    Center,
    FitH,
    FitV,
}

impl Display for GradientImageFitMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Stretch => "Stretch",
                Self::Center => "Center",
                Self::FitH => "Fit H",
                Self::FitV => "Fit V",
            }
        )
    }
}

impl Default for GradientImageFitMode {
    fn default() -> Self {
        Self::Stretch
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Flow field parameters
    pub vector_field_type: VectorFieldType,
    pub noise_type: NoiseType,
    pub noise_seed: u32,
    pub noise_scale: f64,
    pub noise_x: f64,
    pub noise_y: f64,
    pub noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    pub vector_magnitude: f32,

    // Image-based vector field parameters
    pub image_fit_mode: GradientImageFitMode,
    pub image_mirror_horizontal: bool,
    pub image_invert_tone: bool,

    // Particle parameters
    pub total_pool_size: u32, // Total number of particles (autospawn + brush)
    pub particle_lifetime: f32,
    pub particle_speed: f32,
    pub particle_size: u32,
    pub particle_shape: ParticleShape,
    pub particle_autospawn: bool,
    pub autospawn_rate: u32,   // Particles per second for autospawn
    pub brush_spawn_rate: u32, // Particles per second when cursor is active

    // Display parameters
    pub foreground_color_mode: ForegroundColorMode,

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
            vector_field_type: VectorFieldType::Noise,
            noise_type: NoiseType::OpenSimplex,
            noise_seed: 0,
            noise_scale: 1.0,
            noise_x: 1.0,
            noise_y: 1.0,
            noise_dt_multiplier: 0.0,
            vector_magnitude: 0.1,

            // Image-based vector field parameters
            image_fit_mode: GradientImageFitMode::Stretch,
            image_mirror_horizontal: false,
            image_invert_tone: false,

            // Particle parameters
            total_pool_size: 100000,
            particle_lifetime: 5.0,
            particle_speed: 1.0,
            particle_size: 4,
            particle_shape: ParticleShape::Circle,
            particle_autospawn: true,
            autospawn_rate: 500,
            brush_spawn_rate: 1000,

            // Display parameters
            foreground_color_mode: ForegroundColorMode::Age,

            // Trail parameters
            trail_decay_rate: 0.0,
            trail_deposition_rate: 1.0,
            trail_diffusion_rate: 0.0,
            trail_wash_out_rate: 0.1,
        }
    }
}
