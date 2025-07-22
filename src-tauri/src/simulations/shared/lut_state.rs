//! # LUT State Module
//!
//! Manages shared LUT (Look-Up Table) state across all simulations.
//! This eliminates duplication of LUT fields and provides consistent
//! LUT management behavior.

use crate::simulations::shared::lut::LutManager;
use serde::{Deserialize, Serialize};

/// LUT state shared across all simulations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LutState {
    /// Current LUT name
    pub current_lut_name: String,
    /// Whether the LUT is reversed
    pub reversed: bool,
}

impl Default for LutState {
    fn default() -> Self {
        Self {
            current_lut_name: "MATPLOTLIB_viridis".to_string(),
            reversed: false,
        }
    }
}

impl LutState {
    /// Create new LUT state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create LUT state with specific values
    pub fn with_values(lut_name: String, reversed: bool) -> Self {
        Self {
            current_lut_name: lut_name,
            reversed,
        }
    }

    /// Reset to default LUT state
    pub fn reset(&mut self) {
        self.current_lut_name = "MATPLOTLIB_viridis".to_string();
        self.reversed = false;
    }

    /// Update LUT name
    pub fn set_lut_name(&mut self, name: String) {
        self.current_lut_name = name;
    }

    /// Toggle LUT reversal
    pub fn toggle_reversed(&mut self) {
        self.reversed = !self.reversed;
    }

    /// Set LUT reversal state
    pub fn set_reversed(&mut self, reversed: bool) {
        self.reversed = reversed;
    }

    /// Get LUT data from manager with current settings
    pub fn get_lut_data(&self, lut_manager: &LutManager) -> Result<Vec<f32>, String> {
        let lut_data = lut_manager
            .get(&self.current_lut_name)
            .map_err(|e| format!("Failed to load LUT '{}': {}", self.current_lut_name, e))?;

        let colors = lut_data.get_colors(256);
        let mut flattened = colors.into_iter().flatten().collect::<Vec<f32>>();
        if self.reversed {
            flattened.reverse();
        }

        Ok(flattened)
    }
}
