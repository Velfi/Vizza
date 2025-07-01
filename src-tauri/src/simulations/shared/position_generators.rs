use serde::{Deserialize, Serialize};

/// Position generator types that can be used by different simulations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum PositionGenerator {
    #[default]
    Random,
    Center,
    UniformCircle,
    CenteredCircle,
    Ring,
    RainbowRing,
    ColorBattle,
    ColorWheel,
    Line,
    Spiral,
    RainbowSpiral,
}

/// Position generator types specifically for slime mold (single agent type)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum SlimeMoldPositionGenerator {
    #[default]
    Random,
    Center,
    UniformCircle,
    CenteredCircle,
    Ring,
    Line,
    Spiral,
}



impl PositionGenerator {
    /// Convert to u32 for GPU shader compatibility
    pub fn as_u32(&self) -> u32 {
        match self {
            PositionGenerator::Random => 0,
            PositionGenerator::Center => 1,
            PositionGenerator::UniformCircle => 2,
            PositionGenerator::CenteredCircle => 3,
            PositionGenerator::Ring => 4,
            PositionGenerator::RainbowRing => 5,
            PositionGenerator::ColorBattle => 6,
            PositionGenerator::ColorWheel => 7,
            PositionGenerator::Line => 8,
            PositionGenerator::Spiral => 9,
            PositionGenerator::RainbowSpiral => 10,
        }
    }

    /// Convert from u32 for GPU shader compatibility
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => PositionGenerator::Random,
            1 => PositionGenerator::Center,
            2 => PositionGenerator::UniformCircle,
            3 => PositionGenerator::CenteredCircle,
            4 => PositionGenerator::Ring,
            5 => PositionGenerator::RainbowRing,
            6 => PositionGenerator::ColorBattle,
            7 => PositionGenerator::ColorWheel,
            8 => PositionGenerator::Line,
            9 => PositionGenerator::Spiral,
            10 => PositionGenerator::RainbowSpiral,
            _ => PositionGenerator::Random,
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            PositionGenerator::Random => "Random",
            PositionGenerator::Center => "Center",
            PositionGenerator::UniformCircle => "Uniform Circle",
            PositionGenerator::CenteredCircle => "Centered Circle",
            PositionGenerator::Ring => "Ring",
            PositionGenerator::RainbowRing => "Rainbow Ring",
            PositionGenerator::ColorBattle => "Color Battle",
            PositionGenerator::ColorWheel => "Color Wheel",
            PositionGenerator::Line => "Line",
            PositionGenerator::Spiral => "Spiral",
            PositionGenerator::RainbowSpiral => "Rainbow Spiral",
        }
    }

    /// Get all available generators for UI dropdowns
    pub fn all_generators() -> Vec<PositionGenerator> {
        vec![
            PositionGenerator::Random,
            PositionGenerator::Center,
            PositionGenerator::UniformCircle,
            PositionGenerator::CenteredCircle,
            PositionGenerator::Ring,
            PositionGenerator::RainbowRing,
            PositionGenerator::ColorBattle,
            PositionGenerator::ColorWheel,
            PositionGenerator::Line,
            PositionGenerator::Spiral,
            PositionGenerator::RainbowSpiral,
        ]
    }
}

impl SlimeMoldPositionGenerator {
    /// Convert to u32 for GPU shader compatibility
    pub fn as_u32(&self) -> u32 {
        match self {
            SlimeMoldPositionGenerator::Random => 0,
            SlimeMoldPositionGenerator::Center => 1,
            SlimeMoldPositionGenerator::UniformCircle => 2,
            SlimeMoldPositionGenerator::CenteredCircle => 3,
            SlimeMoldPositionGenerator::Ring => 4,
            SlimeMoldPositionGenerator::Line => 5,
            SlimeMoldPositionGenerator::Spiral => 6,
        }
    }

    /// Convert from u32 for GPU shader compatibility
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => SlimeMoldPositionGenerator::Random,
            1 => SlimeMoldPositionGenerator::Center,
            2 => SlimeMoldPositionGenerator::UniformCircle,
            3 => SlimeMoldPositionGenerator::CenteredCircle,
            4 => SlimeMoldPositionGenerator::Ring,
            5 => SlimeMoldPositionGenerator::Line,
            6 => SlimeMoldPositionGenerator::Spiral,
            _ => SlimeMoldPositionGenerator::Random,
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            SlimeMoldPositionGenerator::Random => "Random",
            SlimeMoldPositionGenerator::Center => "Center",
            SlimeMoldPositionGenerator::UniformCircle => "Uniform Circle",
            SlimeMoldPositionGenerator::CenteredCircle => "Centered Circle",
            SlimeMoldPositionGenerator::Ring => "Ring",
            SlimeMoldPositionGenerator::Line => "Line",
            SlimeMoldPositionGenerator::Spiral => "Spiral",
        }
    }

    /// Get all available generators for UI dropdowns
    pub fn all_generators() -> Vec<SlimeMoldPositionGenerator> {
        vec![
            SlimeMoldPositionGenerator::Random,
            SlimeMoldPositionGenerator::Center,
            SlimeMoldPositionGenerator::UniformCircle,
            SlimeMoldPositionGenerator::CenteredCircle,
            SlimeMoldPositionGenerator::Ring,
            SlimeMoldPositionGenerator::Line,
            SlimeMoldPositionGenerator::Spiral,
        ]
    }
} 