//! # Mouse Interaction Module
//!
//! Provides shared mouse interaction utilities to eliminate duplication
//! of mouse handling logic across simulations.

use crate::error::SimulationResult;
use crate::simulations::shared::CursorState;
use std::sync::Arc;
use tracing;
use wgpu::Queue;

/// Mouse button constants for consistent handling
pub const MOUSE_LEFT: u32 = 0;
pub const MOUSE_RIGHT: u32 = 2;

/// Cursor interaction modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorMode {
    Inactive = 0,
    Attract = 1,
    Repel = 2,
}

impl From<u32> for CursorMode {
    fn from(value: u32) -> Self {
        match value {
            1 => CursorMode::Attract,
            2 => CursorMode::Repel,
            _ => CursorMode::Inactive,
        }
    }
}

impl From<CursorMode> for u32 {
    fn from(mode: CursorMode) -> Self {
        mode as u32
    }
}

/// Mouse interaction handler for simulations
#[derive(Debug)]
pub struct MouseInteractionHandler {
    /// Current cursor state
    pub cursor: CursorState,
}

impl MouseInteractionHandler {
    /// Create new mouse interaction handler
    pub fn new() -> Self {
        Self {
            cursor: CursorState::default(),
        }
    }

    /// Handle mouse interaction with standard logic
    pub fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        queue: &Arc<Queue>,
        update_callback: impl FnOnce(&CursorState, &Arc<Queue>) -> SimulationResult<()>,
    ) -> SimulationResult<()> {
        // Determine cursor mode based on mouse_button
        let cursor_mode = Self::determine_cursor_mode(mouse_button);

        // Update cursor state
        self.cursor.update(world_x, world_y, cursor_mode.into());

        tracing::debug!(
            world_x = world_x,
            world_y = world_y,
            cursor_mode = ?cursor_mode,
            cursor_size = self.cursor.size,
            cursor_strength = self.cursor.strength,
            "Mouse interaction updated"
        );

        // Call the simulation-specific update callback
        update_callback(&self.cursor, queue)
    }

    /// Handle mouse release with standard logic
    pub fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        queue: &Arc<Queue>,
        update_callback: impl FnOnce(&CursorState, &Arc<Queue>) -> SimulationResult<()>,
    ) -> SimulationResult<()> {
        // Turn off cursor interaction
        self.cursor.reset();

        tracing::debug!("Mouse release: cursor interaction disabled");

        // Call the simulation-specific update callback
        update_callback(&self.cursor, queue)
    }

    /// Determine cursor mode from mouse button
    pub fn determine_cursor_mode(mouse_button: u32) -> CursorMode {
        match mouse_button {
            MOUSE_LEFT => CursorMode::Attract,
            MOUSE_RIGHT => CursorMode::Repel,
            _ => CursorMode::Inactive,
        }
    }

    /// Convert world coordinates to simulation coordinates
    pub fn world_to_sim_coords(
        world_x: f32,
        world_y: f32,
        sim_width: u32,
        sim_height: u32,
    ) -> (f32, f32) {
        // Convert world coordinates [-1, 1] to simulation pixel coordinates [0, width] x [0, height]
        // World space is [-1, 1] where (-1, -1) is bottom-left and (1, 1) is top-right
        // Simulation space is [0, width] x [0, height] where (0, 0) is top-left
        let sim_x = ((world_x + 1.0) * 0.5) * sim_width as f32;
        let sim_y = ((1.0 - world_y) * 0.5) * sim_height as f32; // Flip Y axis
        (sim_x, sim_y)
    }

    /// Convert simulation coordinates to world coordinates
    pub fn sim_to_world_coords(
        sim_x: f32,
        sim_y: f32,
        sim_width: u32,
        sim_height: u32,
    ) -> (f32, f32) {
        // Convert simulation pixel coordinates [0, width] x [0, height] to world coordinates [-1, 1]
        let world_x = (sim_x / sim_width as f32) * 2.0 - 1.0;
        let world_y = 1.0 - (sim_y / sim_height as f32) * 2.0; // Flip Y axis
        (world_x, world_y)
    }

    /// Get current cursor state
    pub fn get_cursor(&self) -> &CursorState {
        &self.cursor
    }

    /// Get mutable cursor state
    pub fn get_cursor_mut(&mut self) -> &mut CursorState {
        &mut self.cursor
    }
}

impl Default for MouseInteractionHandler {
    fn default() -> Self {
        Self::new()
    }
}
