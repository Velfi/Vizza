use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    // Time and simulation state
    pub time: f32,
    pub width: u32,
    pub height: u32,

    // Color scheme state
    pub color_scheme_name: String,
    pub color_scheme_reversed: bool,

    // Camera state
    pub camera_position: [f32; 2],
    pub camera_zoom: f32,

    // Simulation runtime state
    pub simulation_time: f32,
    pub is_running: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            time: 0.0,
            width: 512,
            height: 512,
            color_scheme_name: "MATPLOTLIB_cubehelix".to_string(),
            color_scheme_reversed: true,
            camera_position: [0.0, 0.0],
            camera_zoom: 1.0,
            simulation_time: 0.0,
            is_running: true,
        }
    }
}
