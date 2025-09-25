use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    // Simulation state
    pub num_points: u32,
    pub time_accum: f32,
    pub time_scale: f32,
    pub last_ca_update_time: f32,
    pub drift: f32,
    pub resolution: [f32; 2],
    pub gui_visible: bool,

    // Brownian motion parameters
    pub brownian_speed: f32, // pixels per second

    // Cursor config
    pub cursor_size: f32,
    pub cursor_strength: f32,

    // LUT + coloring
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,
    pub color_mode: u32, // 0=Random, 1=Density, 2=Age
    pub borders_enabled: bool,
    pub border_width: f32, // Border width in pixels

    // VCA settings
    pub rulestring: String,

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
            num_points: 1000,
            time_accum: 0.0,
            time_scale: 1.0,
            last_ca_update_time: 0.0,
            drift: 0.0,
            resolution: [512.0, 512.0],
            gui_visible: true,
            brownian_speed: 10.0,
            cursor_size: 0.1,
            cursor_strength: 1.0,
            current_color_scheme: "MATPLOTLIB_cubehelix".to_string(),
            color_scheme_reversed: true,
            color_mode: 0, // Random
            borders_enabled: false,
            border_width: 1.0,
            rulestring: "B3/S23".to_string(),
            camera_position: [0.0, 0.0],
            camera_zoom: 1.0,
            simulation_time: 0.0,
            is_running: true,
        }
    }
}
