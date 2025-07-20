use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Number of particles in the simulation
    pub particle_count: u32,

    /// Size of each particle for rendering
    pub particle_size: f32,

    /// Damping factor for particle collisions (0.0 = no damping, 1.0 = fully damped)
    pub collision_damping: f32,

    /// Maximum initial velocity for particles
    pub initial_velocity_max: f32,

    /// Minimum initial velocity for particles
    pub initial_velocity_min: f32,

    /// Random seed for reproducible simulations
    pub random_seed: u32,

    /// Background type: "black" or "white"
    pub background_type: String,

    // Physics parameters
    /// Gravitational constant for physics calculations
    pub gravitational_constant: f32,

    /// Minimum particle mass
    pub min_particle_mass: f32,

    /// Maximum particle mass
    pub max_particle_mass: f32,

    /// Distance at which particles form clumps
    pub clump_distance: f32,

    /// How strongly particles stick together in clumps
    pub cohesive_strength: f32,

    /// Energy damping factor
    pub energy_damping: f32,

    /// Gravity softening parameter
    pub gravity_softening: f32,

    /// Density visualization radius
    pub density_radius: f32,

    /// Coloring mode: "density" or "velocity"
    pub coloring_mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Default tuned for pool ball behavior with thousands of particles
            particle_count: 1000,
            particle_size: 0.008,
            collision_damping: 0.1, // Small damping to prevent particles from getting stuck
            // Give particles initial motion for natural interaction
            initial_velocity_max: 0.3,
            initial_velocity_min: 0.1,
            random_seed: 0,
            background_type: "black".to_string(),
            // Physics tuned for pool ball vs clumping modes
            gravitational_constant: 0.005, // Moderate gravity to keep particles moving
            // Uniform mass for all particles
            min_particle_mass: 1.0,
            max_particle_mass: 1.0,
            clump_distance: 0.02,
            // No cohesion in basic mode
            cohesive_strength: 0.0,
            energy_damping: 0.01, // Small energy loss to prevent infinite motion
            gravity_softening: 0.003,
            density_radius: 0.04,
            coloring_mode: "density".to_string(),
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        self.particle_size = rng.random_range(0.001..0.005);
        self.collision_damping = rng.random_range(0.9..0.99);
        self.initial_velocity_max = rng.random_range(0.1..0.5);
        self.initial_velocity_min = rng.random_range(0.05..self.initial_velocity_max * 0.7);
        self.random_seed = rng.random();

        // Randomize physics fields
        self.gravitational_constant = rng.random_range(0.003..0.012);
        self.min_particle_mass = rng.random_range(0.05..0.2);
        self.max_particle_mass = rng.random_range(0.2..0.5);
        self.clump_distance = rng.random_range(0.015..0.03);
        self.cohesive_strength = rng.random_range(30.0..80.0);
        self.energy_damping = rng.random_range(0.995..0.999);
        self.gravity_softening = rng.random_range(0.003..0.008);
        self.density_radius = rng.random_range(0.02..0.05);
    }
}
