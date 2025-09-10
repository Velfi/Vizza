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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Animation
    pub speed: f32,

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

    // Advection Flow Parameters
    pub advect_strength: f32,
    pub advect_speed: f32,
    pub curl: f32,
    pub decay: f32,

    // Color Scheme
    pub color_scheme_name: String,
    pub color_scheme_reversed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            speed: 0.1,
            base_freq: 20.0,
            moire_amount: 0.5,
            moire_rotation: 0.2,
            moire_scale: 1.05,
            moire_interference: 0.5,
            moire_rotation3: -0.1,
            moire_scale3: 1.1,
            moire_weight3: 0.3,
            advect_strength: 0.6,
            advect_speed: 1.5,
            curl: 0.8,
            decay: 0.98,
            color_scheme_name: "MATPLOTLIB_tab20b".to_string(),
            color_scheme_reversed: false,
        }
    }
}
