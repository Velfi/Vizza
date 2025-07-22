//! # Cursor State Module
//!
//! Manages shared cursor interaction state across all simulations.
//! This eliminates duplication of cursor fields and provides consistent
//! cursor interaction behavior.

use serde::{Deserialize, Serialize};

/// Cursor interaction state shared across all simulations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    /// Cursor interaction mode: 0=inactive, 1=attract, 2=repel
    pub active_mode: u32,
    /// Cursor position in world coordinates
    pub world_x: f32,
    /// Cursor position in world coordinates
    pub world_y: f32,
    /// Cursor interaction radius
    pub size: f32,
    /// Cursor force strength
    pub strength: f32,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            active_mode: 0,
            world_x: 0.0,
            world_y: 0.0,
            size: 0.1,
            strength: 1.0,
        }
    }
}

impl CursorState {
    /// Reset cursor to inactive state
    pub fn reset(&mut self) {
        self.active_mode = 0;
        self.world_x = 0.0;
        self.world_y = 0.0;
    }

    /// Update cursor position and mode
    pub fn update(&mut self, world_x: f32, world_y: f32, mode: u32) {
        self.world_x = world_x;
        self.world_y = world_y;
        self.active_mode = mode;
    }

    /// Check if cursor interaction is active
    pub fn is_active(&self) -> bool {
        self.active_mode > 0
    }

    /// Get cursor mode as string for debugging
    pub fn mode_name(&self) -> &'static str {
        match self.active_mode {
            0 => "inactive",
            1 => "attract",
            2 => "repel",
            _ => "unknown",
        }
    }

    /// Set cursor size
    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    /// Set cursor strength
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength;
    }
}
