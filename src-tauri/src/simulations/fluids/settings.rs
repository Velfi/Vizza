use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Simulation grid resolution scale relative to surface
    pub simulation_resolution_scale: f32,

    // Fluid parameters
    pub time_step: f32,
    pub viscosity: f32,
    pub diffusion: f32,
    pub pressure_iterations: u32,

    // Dye injection parameters
    pub dye_decay: f32,
    pub dye_diffusion: f32,

    // Force interaction
    pub force_strength: f32,
    pub force_radius: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            simulation_resolution_scale: 0.5,
            time_step: 0.016,
            viscosity: 0.0001,
            diffusion: 0.0,
            pressure_iterations: 30,
            dye_decay: 0.01,
            dye_diffusion: 0.0,
            force_strength: 5.0,
            force_radius: 0.05,
        }
    }
}

