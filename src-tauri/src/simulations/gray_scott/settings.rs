use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub diffusion_rate_u: f32,
    pub diffusion_rate_v: f32,
    pub timestep: f32,

    // New optimization settings
    pub max_timestep: f32,
    pub stability_factor: f32,
    pub enable_adaptive_timestep: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            feed_rate: 0.055,
            kill_rate: 0.062,
            diffusion_rate_u: 0.16,
            diffusion_rate_v: 0.08,
            timestep: 2.5,

            // Optimization defaults - disable adaptive timestep so user timestep slider works
            max_timestep: 4.0,
            stability_factor: 0.9,
            enable_adaptive_timestep: false,
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        self.feed_rate = rng.random_range(0.02..0.08);
        self.kill_rate = rng.random_range(0.04..0.08);
        self.diffusion_rate_u = rng.random_range(0.1..0.3);
        self.diffusion_rate_v = rng.random_range(0.05..0.15);
        self.timestep = rng.random_range(0.5..2.0);
    }
}
