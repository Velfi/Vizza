//! # Moiré Settings Module
//!
//! Defines the user-configurable parameters that control the behavior and appearance
//! of the Moiré simulation. These settings combine moiré pattern
//! generation and fluid advection to create
//! complex visual effects.
//!
//! ## Configuration Philosophy
//!
//! The settings provide intuitive control over multiple interacting systems:
//! moiré pattern generation, flow field advection,
//! and color processing. The interaction between these systems creates
//! emergent visual complexity from relatively simple parameters.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MoireGeneratorType {
    Linear,
    Radial,
}

impl Default for MoireGeneratorType {
    fn default() -> Self {
        Self::Linear
    }
}

impl FromStr for MoireGeneratorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(MoireGeneratorType::Linear),
            "radial" => Ok(MoireGeneratorType::Radial),
            _ => Err(format!("Invalid MoireGeneratorType: '{}'. Expected 'linear' or 'radial'", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GradientImageFitMode {
    Stretch,
    Center,
    FitH,
    FitV,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageInterferenceMode {
    Replace,    // Current behavior - replace moiré with image
    Add,        // Add image to moiré pattern
    Multiply,   // Multiply image with moiré pattern
    Overlay,    // Overlay blending mode
    Mask,       // Use image as mask for moiré pattern
    Modulate,   // Use image to modulate moiré intensity
}

impl Default for GradientImageFitMode {
    fn default() -> Self {
        Self::Stretch
    }
}

impl FromStr for GradientImageFitMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stretch" => Ok(GradientImageFitMode::Stretch),
            "center" => Ok(GradientImageFitMode::Center),
            "fit h" | "fith" => Ok(GradientImageFitMode::FitH),
            "fit v" | "fitv" => Ok(GradientImageFitMode::FitV),
            _ => Err(format!("Invalid GradientImageFitMode: '{}'. Expected 'stretch', 'center', 'fit h', or 'fit v'", s)),
        }
    }
}

impl Default for ImageInterferenceMode {
    fn default() -> Self {
        Self::Modulate
    }
}

impl FromStr for ImageInterferenceMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "replace" => Ok(ImageInterferenceMode::Replace),
            "add" => Ok(ImageInterferenceMode::Add),
            "multiply" => Ok(ImageInterferenceMode::Multiply),
            "overlay" => Ok(ImageInterferenceMode::Overlay),
            "mask" => Ok(ImageInterferenceMode::Mask),
            "modulate" => Ok(ImageInterferenceMode::Modulate),
            _ => Err(format!("Invalid ImageInterferenceMode: '{}'. Expected 'replace', 'add', 'multiply', 'overlay', 'mask', or 'modulate'", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Animation
    pub speed: f32,

    // Moiré Pattern Generation
    pub generator_type: MoireGeneratorType,

    // Moiré Pattern Parameters
    pub base_freq: f32,
    pub moire_amount: f32,
    pub moire_rotation: f32,
    pub moire_scale: f32,
    pub moire_interference: f32,

    // Third Grid Pattern Parameters
    pub moire_rotation3: f32,
    pub moire_scale3: f32,
    pub moire_weight3: f32,

    // Radial Pattern Parameters
    pub radial_swirl_strength: f32,
    pub radial_starburst_count: f32,
    pub radial_center_brightness: f32,

    // Advection Flow Parameters
    pub advect_strength: f32,
    pub advect_speed: f32,
    pub curl: f32,
    pub decay: f32,

    // Image controls (for optional image inputs)
    pub image_mode_enabled: bool,
    pub image_fit_mode: GradientImageFitMode,
    pub image_mirror_horizontal: bool,
    pub image_invert_tone: bool,
    pub image_interference_mode: ImageInterferenceMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            speed: 0.1,
            generator_type: MoireGeneratorType::Linear,
            base_freq: 20.0,
            moire_amount: 0.5,
            moire_rotation: 0.2,
            moire_scale: 1.05,
            moire_interference: 0.5,
            moire_rotation3: -0.1,
            moire_scale3: 1.1,
            moire_weight3: 0.3,
            radial_swirl_strength: 0.5,
            radial_starburst_count: 16.0,
            radial_center_brightness: 1.0,
            advect_strength: 0.6,
            advect_speed: 1.5,
            curl: 0.8,
            decay: 0.98,
            image_mode_enabled: false,
            image_fit_mode: GradientImageFitMode::FitV,
            image_mirror_horizontal: false,
            image_invert_tone: true,
            image_interference_mode: ImageInterferenceMode::Modulate,
        }
    }
}
