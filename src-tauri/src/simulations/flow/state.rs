use super::settings::{BackgroundColorMode, ForegroundColorMode, TrailMapFiltering};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    // Time and simulation state
    pub time: f32,
    pub delta_time: f32,
    pub autospawn_accumulator: f32,
    pub brush_spawn_accumulator: f32,
    pub noise_dt_multiplier: f32,

    // GUI and display state
    pub gui_visible: bool,
    pub trail_map_width: u32,
    pub trail_map_height: u32,
    pub background_color_mode: BackgroundColorMode,
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,
    pub show_particles: bool,
    pub foreground_color_mode: ForegroundColorMode,
    pub trail_map_filtering: TrailMapFiltering,

    // Particle pool management
    pub autospawn_pool_size: u32,
    pub brush_pool_size: u32,
    pub total_pool_size: u32,

    // Mouse interaction state
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,
    pub cursor_size: f32,
    pub mouse_button_down: u32, // 0 = not held, 1 = left click held, 2 = right click held

    // Flow vector generation
    pub flow_field_resolution: u32,

    // Shape drawing
    pub shape_drawing_enabled: bool,

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
            delta_time: 0.0,
            autospawn_accumulator: 0.0,
            brush_spawn_accumulator: 0.0,
            noise_dt_multiplier: 1.0,
            gui_visible: true,
            trail_map_width: 512,
            trail_map_height: 512,
            background_color_mode: BackgroundColorMode::Black,
            current_color_scheme: "MATPLOTLIB_cubehelix".to_string(),
            color_scheme_reversed: true,
            show_particles: true,
            foreground_color_mode: ForegroundColorMode::Age,
            trail_map_filtering: TrailMapFiltering::Nearest,
            autospawn_pool_size: 1000,
            brush_pool_size: 1000,
            total_pool_size: 2000,
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
            cursor_size: 100.0,
            mouse_button_down: 0,
            flow_field_resolution: 128,
            shape_drawing_enabled: false,
            camera_position: [0.0, 0.0],
            camera_zoom: 1.0,
            simulation_time: 0.0,
            is_running: true,
        }
    }
}
