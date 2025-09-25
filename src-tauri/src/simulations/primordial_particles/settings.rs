use serde::{Deserialize, Serialize};

/// Settings for the Primordial Particles simulation that can be saved in presets
/// Based on the Nature article "How a life-like system emerges from a simplistic particle motion law"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Fixed rotation parameter α (in degrees)
    /// Controls the base turning behavior of particles
    /// α = 180° makes isolated particles hold position within 2 time steps
    pub alpha: f32,

    /// Proportional rotation parameter β
    /// Models rotation proportional to local neighborhood size
    /// Higher values create stronger responses to particle density
    pub beta: f32,

    /// Constant velocity of particles
    /// All particles move at the same speed
    pub velocity: f32,

    /// Interaction radius for particle detection
    /// Particles react to others within this radius
    pub radius: f32,

    /// Wrap particles around screen edges if true
    pub wrap_edges: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            alpha: 180.0, // Default from paper - makes isolated particles hold position
            beta: 0.1,    // Moderate response to neighborhood density
            velocity: 0.2,
            radius: 0.1,
            wrap_edges: true,
        }
    }
}

impl Settings {
    /// Get alpha in radians for GPU calculations
    pub fn alpha_radians(&self) -> f32 {
        self.alpha.to_radians()
    }

    /// Validate settings to ensure they're within reasonable bounds
    pub fn validate(&self) -> Result<(), String> {
        if self.velocity <= 0.0 {
            return Err("Velocity must be positive".to_string());
        }
        if self.radius <= 0.0 {
            return Err("Radius must be positive".to_string());
        }
        Ok(())
    }

    /// Randomize all settings within reasonable bounds based on PPS research
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        self.alpha = rng.random_range(-180.0..180.0);
        self.beta = rng.random_range(-60.0..60.0);
        self.velocity = rng.random_range(0.1..0.8);
        self.radius = rng.random_range(0.015..0.15);
        // I don't think this should be randomized but I do think it should be a setting
        self.wrap_edges = true;
    }
}
