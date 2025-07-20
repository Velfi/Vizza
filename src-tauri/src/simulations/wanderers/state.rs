use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Current particle positions and properties (runtime data)
    pub particles: Vec<[f32; 6]>, // position[2], velocity[2], mass, temperature

    /// Current mouse interaction state
    pub mouse_pressed: bool,
    /// 0 = no mouse, 1 = grab (left), 2 = repel (right)
    pub mouse_mode: u32,
    pub mouse_position: [f32; 2],
    pub mouse_previous_position: [f32; 2],

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
            particles: Vec::new(),
            mouse_pressed: false,
            mouse_mode: 0,
            mouse_position: [0.0, 0.0], // Center of [-1,1] space
            mouse_previous_position: [0.0, 0.0],
            cursor_size: 0.1,
            cursor_strength: 0.1,
            grabbed_particles: Vec::new(),
            current_lut_name: "MATPLOTLIB_viridis".to_string(),
            lut_reversed: false,
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
        self.mouse_previous_position = [0.0, 0.0];
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
        self.mouse_previous_position = [0.0, 0.0];
        self.cursor_size = 0.1;
        self.cursor_strength = 0.1;
        self.grabbed_particles.clear();
    }
}
