use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NutrientPattern {
    Uniform,
    Checkerboard,
    DiagonalGradient,
    RadialGradient,
    VerticalStripes,
    HorizontalStripes,
    EnhancedNoise,
    WaveFunction,
    CosineGrid,
}

impl Serialize for NutrientPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            NutrientPattern::Uniform => "Uniform",
            NutrientPattern::Checkerboard => "Checkerboard",
            NutrientPattern::DiagonalGradient => "Diagonal Gradient",
            NutrientPattern::RadialGradient => "Radial Gradient",
            NutrientPattern::VerticalStripes => "Vertical Stripes",
            NutrientPattern::HorizontalStripes => "Horizontal Stripes",
            NutrientPattern::EnhancedNoise => "Enhanced Noise",
            NutrientPattern::WaveFunction => "Wave Function",
            NutrientPattern::CosineGrid => "Cosine Grid",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for NutrientPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            // Handle frontend display names
            "Uniform" => Ok(NutrientPattern::Uniform),
            "Checkerboard" => Ok(NutrientPattern::Checkerboard),
            "Diagonal Gradient" => Ok(NutrientPattern::DiagonalGradient),
            "Radial Gradient" => Ok(NutrientPattern::RadialGradient),
            "Vertical Stripes" => Ok(NutrientPattern::VerticalStripes),
            "Horizontal Stripes" => Ok(NutrientPattern::HorizontalStripes),
            "Enhanced Noise" => Ok(NutrientPattern::EnhancedNoise),
            "Wave Function" => Ok(NutrientPattern::WaveFunction),
            "Cosine Grid" => Ok(NutrientPattern::CosineGrid),
            // Handle internal names for backward compatibility
            "uniform" => Ok(NutrientPattern::Uniform),
            "checkerboard" => Ok(NutrientPattern::Checkerboard),
            "diagonal_gradient" => Ok(NutrientPattern::DiagonalGradient),
            "radial_gradient" => Ok(NutrientPattern::RadialGradient),
            "vertical_stripes" => Ok(NutrientPattern::VerticalStripes),
            "horizontal_stripes" => Ok(NutrientPattern::HorizontalStripes),
            "enhanced_noise" => Ok(NutrientPattern::EnhancedNoise),
            "wave_function" => Ok(NutrientPattern::WaveFunction),
            "cosine_grid" => Ok(NutrientPattern::CosineGrid),
            // Handle enum variant names
            "DiagonalGradient" => Ok(NutrientPattern::DiagonalGradient),
            "RadialGradient" => Ok(NutrientPattern::RadialGradient),
            "VerticalStripes" => Ok(NutrientPattern::VerticalStripes),
            "HorizontalStripes" => Ok(NutrientPattern::HorizontalStripes),
            "EnhancedNoise" => Ok(NutrientPattern::EnhancedNoise),
            "WaveFunction" => Ok(NutrientPattern::WaveFunction),
            "CosineGrid" => Ok(NutrientPattern::CosineGrid),
            _ => Ok(NutrientPattern::Uniform), // Default fallback
        }
    }
}

impl Default for NutrientPattern {
    fn default() -> Self {
        Self::Uniform
    }
}

impl From<NutrientPattern> for u32 {
    fn from(pattern: NutrientPattern) -> Self {
        match pattern {
            NutrientPattern::Uniform => 0,
            NutrientPattern::Checkerboard => 1,
            NutrientPattern::DiagonalGradient => 2,
            NutrientPattern::RadialGradient => 3,
            NutrientPattern::VerticalStripes => 4,
            NutrientPattern::HorizontalStripes => 5,
            NutrientPattern::EnhancedNoise => 6,
            NutrientPattern::WaveFunction => 7,
            NutrientPattern::CosineGrid => 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub diffusion_rate_u: f32,
    pub diffusion_rate_v: f32,
    pub timestep: f32,
    pub nutrient_pattern: NutrientPattern,
    pub nutrient_pattern_reversed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            feed_rate: 0.055,
            kill_rate: 0.062,
            diffusion_rate_u: 0.2097,
            diffusion_rate_v: 0.105,
            timestep: 1.0,
            nutrient_pattern: NutrientPattern::Uniform,
            nutrient_pattern_reversed: false,
        }
    }
} 