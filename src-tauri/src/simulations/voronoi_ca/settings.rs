use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub rulestring: String, // e.g. "B3/S23" or extended rule
    pub timestep: f32,      // sim dt multiplier per frame
    pub steps_per_frame: u32, // number of CA steps per rendered frame
    pub random_seed: u32,
    pub brush_radius: f32,
    pub brush_strength: f32,
    pub auto_reseed_enabled: bool,
    pub auto_reseed_interval_secs: f32,
    pub lut_name: String,
    pub lut_reversed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rulestring: "B3/S23".to_string(),
            timestep: 1.0,
            steps_per_frame: 1,
            random_seed: 0,
            brush_radius: 10.0,
            brush_strength: 1.0,
            auto_reseed_enabled: false,
            auto_reseed_interval_secs: 10.0,
            lut_name: "MATPLOTLIB_Blues".to_string(),
            lut_reversed: false,
        }
    }
}