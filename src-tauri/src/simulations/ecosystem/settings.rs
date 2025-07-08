use serde::{Deserialize, Serialize};

/// Settings for the Ecosystem simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Agent physical properties
    /// Number of agents in the simulation
    pub agent_count: u32,
    /// Minimum agent speed
    pub agent_speed_min: f32,
    /// Maximum agent speed
    pub agent_speed_max: f32,
    /// Agent turning rate in radians per second
    pub agent_turn_rate: f32,
    /// Agent sensor range for chemical detection
    pub sensor_range: f32,
    /// Number of sensors per agent (typically 2-4)
    pub sensor_count: u32,
    /// Angle between sensors in radians
    pub sensor_angle: f32,

    // Chemical environment
    /// Chemical field resolution (grid size)
    pub chemical_resolution: u32,
    /// Number of different chemical types
    pub chemical_types: u32,
    /// Chemical diffusion rate
    pub chemical_diffusion_rate: f32,
    /// Chemical decay rate
    pub chemical_decay_rate: f32,
    /// Rate at which agents deposit chemicals
    pub chemical_deposition_rate: f32,

    // Agent behavior and learning
    /// Learning rate for neural network updates
    pub learning_rate: f32,
    /// Mutation rate for reproduction
    pub mutation_rate: f32,
    /// Energy consumption rate per frame
    pub energy_consumption_rate: f32,
    /// Energy gained from food sources
    pub energy_gain_from_food: f32,
    /// Energy required for reproduction
    pub reproduction_energy_threshold: f32,
    /// Probability of reproduction when threshold is met
    pub reproduction_probability: f32,

    // Species and interaction
    /// Number of different species
    pub species_count: u32,
    /// Strength of intra-species attraction
    pub intra_species_attraction: f32,
    /// Strength of inter-species repulsion
    pub inter_species_repulsion: f32,

    // Environmental factors
    /// Strength of Brownian motion (random movement)
    pub brownian_motion_strength: f32,
    /// Food spawn rate per frame
    pub food_spawn_rate: f32,
    /// Food decay rate
    pub food_decay_rate: f32,
    /// Maximum food particles in environment
    pub max_food_particles: u32,

    // Visual settings
    /// Whether to show chemical trails
    pub show_chemical_trails: bool,
    /// Chemical trail opacity
    pub trail_opacity: f32,
    /// Whether to show agent sensors
    pub show_sensors: bool,
    /// Whether to show energy levels as agent size
    pub show_energy_as_size: bool,

    // Simulation parameters
    /// Random seed for reproducible results
    pub random_seed: u32,
    /// Simulation time step
    pub time_step: f32,
    /// Whether to wrap edges or use boundaries
    pub wrap_edges: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Agent physical properties (scaled for [-1,1] coordinate system)
            agent_count: 1000,
            agent_speed_min: 0.001, // Much slower for normalized coordinates
            agent_speed_max: 0.005, // Much slower for normalized coordinates
            agent_turn_rate: 2.0,   // Angular rate, unchanged
            sensor_range: 0.1,      // Much smaller for normalized coordinates
            sensor_count: 3,
            sensor_angle: 0.5, // ~30 degrees

            // Chemical environment
            chemical_resolution: 256,
            chemical_types: 4,
            chemical_diffusion_rate: 0.1,
            chemical_decay_rate: 0.01,
            chemical_deposition_rate: 1.0,

            // Agent behavior and learning
            learning_rate: 0.01,
            mutation_rate: 0.05,
            energy_consumption_rate: 0.1,
            energy_gain_from_food: 10.0,
            reproduction_energy_threshold: 50.0,
            reproduction_probability: 0.1,

            // Species and interaction
            species_count: 3,
            intra_species_attraction: 1.0,
            inter_species_repulsion: 0.5,

            // Environmental factors
            brownian_motion_strength: 0.001, // Much smaller for normalized coordinates
            food_spawn_rate: 0.5,
            food_decay_rate: 0.005,
            max_food_particles: 500,

            // Visual settings
            show_chemical_trails: true,
            trail_opacity: 0.3,
            show_sensors: false,
            show_energy_as_size: true,

            // Simulation parameters
            random_seed: 0,
            time_step: 0.016,
            wrap_edges: true,
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        // Randomize agent properties (scaled for [-1,1] coordinate system)
        self.agent_count = rng.random_range(500..2000);
        self.agent_speed_min = rng.random_range(0.0005..0.002); // Much smaller for normalized coordinates
        self.agent_speed_max = self.agent_speed_min + rng.random_range(0.001..0.008); // Much smaller
        self.agent_turn_rate = rng.random_range(0.5..4.0); // Angular rate, unchanged
        self.sensor_range = rng.random_range(0.05..0.3); // Much smaller for normalized coordinates
        self.sensor_angle = rng.random_range(0.2..1.0);

        // Randomize chemical properties
        self.chemical_diffusion_rate = rng.random_range(0.05..0.3);
        self.chemical_decay_rate = rng.random_range(0.001..0.05);
        self.chemical_deposition_rate = rng.random_range(0.5..2.0);

        // Randomize learning parameters
        self.learning_rate = rng.random_range(0.001..0.05);
        self.mutation_rate = rng.random_range(0.01..0.1);
        self.energy_consumption_rate = rng.random_range(0.05..0.2);
        self.energy_gain_from_food = rng.random_range(5.0..20.0);

        // Randomize species interactions
        self.intra_species_attraction = rng.random_range(0.5..2.0);
        self.inter_species_repulsion = rng.random_range(0.1..1.0);

        // Randomize environmental factors (scaled for normalized coordinates)
        self.brownian_motion_strength = rng.random_range(0.0001..0.005); // Much smaller for normalized coordinates
        self.food_spawn_rate = rng.random_range(0.1..1.0);

        self.random_seed = rng.random();
    }
}
