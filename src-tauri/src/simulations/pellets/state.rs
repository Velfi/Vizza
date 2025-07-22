//! # Pellets State Module
//! 
//! Manages the runtime state of the Pellets simulation, tracking transient data
//! that changes during execution but is not part of the persistent configuration.
//! This includes user interactions, camera positioning, and UI state.
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
//! and simulation execution status, providing the context needed for
//! responsive and intuitive user experience.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Current mouse interaction state
    pub mouse_pressed: bool,
    /// 0 = no mouse, 1 = attract (left click)
    pub mouse_mode: u32,
    pub mouse_position: [f32; 2],
    pub mouse_velocity: [f32; 2], // Mouse velocity in world units per second
    pub mouse_screen_position: [f32; 2], // Raw screen coordinates from frontend
    pub last_mouse_time: f64, // Timestamp of last mouse interaction for velocity calculation

    /// Cursor interaction parameters
    pub cursor_size: f32,
    pub cursor_strength: f32,

    /// Grabbed particles for drag interaction
    pub grabbed_particles: Vec<usize>, // Indices of particles being dragged

    /// Current LUT state (runtime)
    pub current_lut_name: String,
    pub lut_reversed: bool,

    /// UI visibility state
    pub gui_visible: bool,

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
            mouse_pressed: false,
            mouse_mode: 0,
            mouse_position: [0.0, 0.0], // Center of [-1,1] space
            mouse_velocity: [0.0, 0.0],
            mouse_screen_position: [0.0, 0.0], // Raw screen coordinates
            last_mouse_time: 0.0,
            cursor_size: 0.20,
            cursor_strength: 1.0, // Increased for better throwing visibility
            grabbed_particles: Vec::new(),
            current_lut_name: "MATPLOTLIB_bone".to_string(),
            lut_reversed: true,
            gui_visible: true,
            camera_position: [0.0, 0.0],
            camera_zoom: 1.0,
            simulation_time: 0.0,
            is_running: true,
        }
    }
}

impl State {
    /// Reset all state to default values
    pub fn reset(&mut self) {
        self.mouse_pressed = false;
        self.mouse_mode = 0;
        self.mouse_position = [0.0, 0.0]; // Center of [-1,1] space
        self.mouse_velocity = [0.0, 0.0];
        self.mouse_screen_position = [0.0, 0.0];
        self.last_mouse_time = 0.0;
        self.grabbed_particles.clear();
        self.gui_visible = true;
        self.camera_position = [0.0, 0.0];
        self.camera_zoom = 1.0;
        self.simulation_time = 0.0;
        self.is_running = true;
    }

    /// Reset only the camera state
    pub fn reset_camera(&mut self) {
        self.camera_position = [0.0, 0.0];
        self.camera_zoom = 1.0;
    }

    /// Reset only the mouse interaction state
    pub fn reset_mouse(&mut self) {
        self.mouse_pressed = false;
        self.mouse_mode = 0;
        self.mouse_position = [0.0, 0.0]; // Center of [-1,1] space
        self.mouse_velocity = [0.0, 0.0];
        self.mouse_screen_position = [0.0, 0.0];
        self.last_mouse_time = 0.0;
        self.grabbed_particles.clear();
        self.cursor_size = 0.1;
        self.cursor_strength = 1.0; // Increased for better throwing
    }
}
