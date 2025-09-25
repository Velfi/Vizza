//! # Slime Mold State Module
//!
//! Manages the runtime state of the Slime Mold simulation, tracking transient data
//! that changes during execution but is not part of the persistent configuration.
//! This includes mask patterns, user interactions, camera positioning, and UI state.
//!
//! ## State Philosophy
//!
//! The state represents the current condition of the simulation at any moment.
//! Unlike settings, which define how the simulation behaves, state captures
//! what is happening right now. This separation allows for proper preset
//! management and state restoration when simulations restart.
//!
//! ## State Categories
//!
//! The runtime state encompasses user interactions, visual presentation,
//! mask patterns, and simulation execution status, providing the context needed for
//! responsive and intuitive user experience.

use crate::simulations::shared::ImageFitMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskPattern {
    Disabled,
    Checkerboard,
    DiagonalGradient,
    RadialGradient,
    VerticalStripes,
    HorizontalStripes,
    WaveFunction,
    CosineGrid,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskTarget {
    PheromoneDeposition, // Affects pheromone deposition rate
    PheromoneDecay,      // Affects pheromone decay rate
    PheromoneDiffusion,  // Affects pheromone diffusion rate
    AgentSpeed,          // Affects agent speed
    AgentTurnRate,       // Affects agent turn rate
    AgentSensorDistance, // Affects agent sensor distance
    TrailMap,            // Affects trail map values directly
}

// ImageFitMode now shared via simulations::shared::ImageFitMode

impl Default for MaskPattern {
    fn default() -> Self {
        Self::Disabled
    }
}

impl MaskPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaskPattern::Disabled => "Disabled",
            MaskPattern::Checkerboard => "Checkerboard",
            MaskPattern::DiagonalGradient => "Diagonal Gradient",
            MaskPattern::RadialGradient => "Radial Gradient",
            MaskPattern::VerticalStripes => "Vertical Stripes",
            MaskPattern::HorizontalStripes => "Horizontal Stripes",
            MaskPattern::WaveFunction => "Wave Function",
            MaskPattern::CosineGrid => "Cosine Grid",
            MaskPattern::Image => "Image",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Disabled" => Some(MaskPattern::Disabled),
            "Checkerboard" => Some(MaskPattern::Checkerboard),
            "Diagonal Gradient" => Some(MaskPattern::DiagonalGradient),
            "Radial Gradient" => Some(MaskPattern::RadialGradient),
            "Vertical Stripes" => Some(MaskPattern::VerticalStripes),
            "Horizontal Stripes" => Some(MaskPattern::HorizontalStripes),
            "Wave Function" => Some(MaskPattern::WaveFunction),
            "Cosine Grid" => Some(MaskPattern::CosineGrid),
            "Image" => Some(MaskPattern::Image),
            _ => None,
        }
    }

    // Snake-case conversions removed; we standardize on display-case strings
}

impl Default for MaskTarget {
    fn default() -> Self {
        Self::PheromoneDeposition
    }
}

impl MaskTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaskTarget::PheromoneDeposition => "Pheromone Deposition",
            MaskTarget::PheromoneDecay => "Pheromone Decay",
            MaskTarget::PheromoneDiffusion => "Pheromone Diffusion",
            MaskTarget::AgentSpeed => "Agent Speed",
            MaskTarget::AgentTurnRate => "Agent Turn Rate",
            MaskTarget::AgentSensorDistance => "Agent Sensor Distance",
            MaskTarget::TrailMap => "Trail Map",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Pheromone Deposition" => Some(MaskTarget::PheromoneDeposition),
            "Pheromone Decay" => Some(MaskTarget::PheromoneDecay),
            "Pheromone Diffusion" => Some(MaskTarget::PheromoneDiffusion),
            "Agent Speed" => Some(MaskTarget::AgentSpeed),
            "Agent Turn Rate" => Some(MaskTarget::AgentTurnRate),
            "Agent Sensor Distance" => Some(MaskTarget::AgentSensorDistance),
            "Trail Map" => Some(MaskTarget::TrailMap),
            _ => None,
        }
    }

    // Snake-case conversions removed; we standardize on display-case strings
}

impl From<MaskPattern> for u32 {
    fn from(pattern: MaskPattern) -> Self {
        match pattern {
            MaskPattern::Disabled => 0,
            MaskPattern::Checkerboard => 1,
            MaskPattern::DiagonalGradient => 2,
            MaskPattern::RadialGradient => 3,
            MaskPattern::VerticalStripes => 4,
            MaskPattern::HorizontalStripes => 5,
            MaskPattern::WaveFunction => 6,
            MaskPattern::CosineGrid => 7,
            MaskPattern::Image => 8,
        }
    }
}

impl From<MaskTarget> for u32 {
    fn from(target: MaskTarget) -> Self {
        match target {
            MaskTarget::PheromoneDeposition => 0,
            MaskTarget::PheromoneDecay => 1,
            MaskTarget::PheromoneDiffusion => 2,
            MaskTarget::AgentSpeed => 3,
            MaskTarget::AgentTurnRate => 4,
            MaskTarget::AgentSensorDistance => 5,
            MaskTarget::TrailMap => 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Mask system state
    pub mask_pattern: MaskPattern,
    pub mask_target: MaskTarget,
    pub mask_strength: f32, // How strongly the mask affects the target parameter (0.0 = no effect, 1.0 = full effect)
    pub mask_curve: f32,
    pub mask_reversed: bool,
    pub mask_image_fit_mode: ImageFitMode,
    pub mask_mirror_horizontal: bool,
    pub mask_mirror_vertical: bool,
    pub mask_invert_tone: bool,

    /// Current mouse interaction state
    pub mouse_pressed: bool,
    pub mouse_position: [f32; 2],
    pub mouse_screen_position: [f32; 2], // Raw screen coordinates from frontend

    /// Cursor interaction parameters
    pub cursor_size: f32,
    pub cursor_strength: f32,

    /// Current color scheme state (runtime)
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,

    /// UI visibility state
    pub gui_visible: bool,

    /// Mask image runtime state (serializable)
    pub mask_image_base: Option<Vec<f32>>, // before strength mapping
    pub mask_image_raw: Option<Vec<f32>>, // uploaded values
    pub mask_image_needs_upload: bool,

    /// Camera state (position and zoom)
    pub camera_position: [f32; 2],
    pub camera_zoom: f32,

    /// Simulation runtime state
    pub simulation_time: f32,
    pub is_running: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            // Mask system defaults
            mask_pattern: MaskPattern::default(),
            mask_target: MaskTarget::default(),
            mask_strength: 0.5, // Moderate strength by default
            mask_curve: 1.0,
            mask_reversed: false,
            mask_image_fit_mode: ImageFitMode::default(),
            mask_mirror_horizontal: false,
            mask_mirror_vertical: false,
            mask_invert_tone: false,

            // Mouse interaction defaults
            mouse_pressed: false,
            mouse_position: [0.0, 0.0],        // Center of [-1,1] space
            mouse_screen_position: [0.0, 0.0], // Raw screen coordinates

            // Cursor defaults
            cursor_size: 0.20,
            cursor_strength: 1.0,

            // Color scheme defaults
            current_color_scheme: "MATPLOTLIB_prism".to_string(),
            color_scheme_reversed: true,

            // UI defaults
            gui_visible: true,

            // Mask image defaults
            mask_image_base: None,
            mask_image_raw: None,
            mask_image_needs_upload: false,

            // Camera defaults
            camera_position: [0.0, 0.0],
            camera_zoom: 1.0,

            // Simulation defaults
            simulation_time: 0.0,
            is_running: true,
        }
    }
}
