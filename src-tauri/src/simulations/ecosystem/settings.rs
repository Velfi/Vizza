use serde::{Deserialize, Serialize};

/// Settings for the Ecosystem simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // Basic simulation parameters
    /// Number of agents in the simulation
    pub agent_count: u32,
    /// Simulation time step
    pub time_step: f32,
    /// Random seed for reproducible results
    pub random_seed: u32,
    /// Whether to wrap edges or use boundaries
    pub wrap_edges: bool,

    // Agent physical properties
    /// Minimum agent speed
    pub agent_speed_min: f32,
    /// Maximum agent speed
    pub agent_speed_max: f32,
    /// Agent turning rate in radians per second
    pub agent_turn_rate: f32,
    /// Agent sensor range for chemical detection
    pub sensor_range: f32,
    /// Number of sensors per agent (typically 3-4)
    pub sensor_count: u32,
    /// Angle between sensors in radians
    pub sensor_angle: f32,
    /// Strength of Brownian motion (random movement)
    pub brownian_motion_strength: f32,

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

    // Ecological roles and variants
    /// Number of ecological roles (3: Recyclers, Producers, Predators)
    pub ecological_roles: u32,
    /// Number of variants per role (multiple species per role)
    pub variants_per_role: u32,
    /// Recycler efficiency (bacteria, fungi, decomposer protozoans)
    pub recycler_efficiency: f32,
    /// Producer photosynthesis rate (algae, cyanobacteria, photosynthetic protists)
    pub producer_photosynthesis_rate: f32,
    /// Predator hunting efficiency (predatory bacteria, viruses, predatory protozoans, parasitic microbes)
    pub predator_hunting_efficiency: f32,

    // Energy and metabolism
    /// Energy consumption rate per frame
    pub energy_consumption_rate: f32,
    /// Energy gained from food sources
    pub energy_gain_from_food: f32,
    /// Energy required for reproduction
    pub reproduction_energy_threshold: f32,
    /// Base probability of reproduction when threshold is met
    pub reproduction_probability: f32,
    /// Mutation rate for reproduction
    pub mutation_rate: f32,

    // Unified nutrient architecture
    /// Maximum number of visible particles in environment
    pub max_particles: u32,
    /// Rate at which particles decompose into chemical gradients
    pub particle_decomposition_rate: f32,
    /// Rate at which particles naturally decay
    pub particle_decay_rate: f32,
    /// Ratio of particle matter converted to chemical nutrients
    pub matter_to_chemical_ratio: f32,

    // Fluid dynamics system
    /// Enable fluid dynamics simulation
    pub enable_fluid_dynamics: bool,
    /// Fluid viscosity (affects flow speed)
    pub fluid_viscosity: f32,
    /// Fluid density (affects pressure)
    pub fluid_density: f32,
    /// Strength of biological current generation
    pub biological_current_strength: f32,
    /// Strength of chemical current generation
    pub chemical_current_strength: f32,
    /// Flow update frequency (performance optimization)
    pub flow_update_frequency: u32,

    // Light gradient system
    /// Enable light gradient
    pub enable_light_gradient: bool,
    /// Base light intensity
    pub base_light_intensity: f32,
    /// Light gradient strength
    pub light_gradient_strength: f32,
    /// Light gradient rotation speed (day/night cycle)
    pub light_rotation_speed: f32,
    /// Current light direction angle
    pub light_direction_angle: f32,

    // Movement and sensing systems
    /// Chemotaxis sensitivity
    pub chemotaxis_sensitivity: f32,
    /// Run-and-tumble: minimum run duration
    pub run_duration_min: f32,
    /// Run-and-tumble: maximum run duration
    pub run_duration_max: f32,
    /// Tumble angle range (radians)
    pub tumble_angle_range: f32,
    /// Flagella strength for swimming
    pub flagella_strength: f32,
    /// Receptor saturation threshold
    pub receptor_saturation_threshold: f32,

    // Hunting mechanics
    /// Contact range for predation attempts
    pub predation_contact_range: f32,
    /// Pack hunting bonus multiplier
    pub pack_hunting_bonus: f32,
    /// Base predation success rate
    pub predation_success_rate: f32,

    // Spatial organization
    /// Enable biofilm formation
    pub enable_biofilm_formation: bool,
    /// Biofilm growth rate
    pub biofilm_growth_rate: f32,
    /// Biofilm persistence over time
    pub biofilm_persistence: f32,
    /// Threshold for nutrient stream formation
    pub nutrient_stream_threshold: f32,
    /// Range for territory establishment
    pub territory_establishment_range: f32,

    // Population dynamics
    /// Carrying capacity of the environment
    pub carrying_capacity: f32,
    /// Damping factor for population oscillations
    pub population_oscillation_damping: f32,
    /// Strength of resource competition
    pub resource_competition_strength: f32,
    /// Strength of succession patterns
    pub succession_pattern_strength: f32,

    // Environmental factors
    /// Temperature gradient strength
    pub temperature_gradient_strength: f32,
    /// pH gradient strength
    pub ph_gradient_strength: f32,
    /// Toxin accumulation rate
    pub toxin_accumulation_rate: f32,
    /// Threshold for dead zone formation
    pub dead_zone_threshold: f32,

    // Rendering options
    /// Show energy as size
    pub show_energy_as_size: bool,
    /// Show chemical fields
    pub show_chemical_fields: bool,
    /// Show individual chemical types
    pub show_oxygen: bool,
    pub show_co2: bool,
    pub show_nitrogen: bool,
    pub show_pheromones: bool,
    pub show_toxins: bool,
    pub show_attractants: bool,
    /// Show environmental overlays
    pub show_light_gradient: bool,
    pub show_temperature_zones: bool,
    pub show_ph_zones: bool,
    /// Opacity settings
    pub chemical_field_opacity: f32,
    pub environmental_opacity: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Basic simulation parameters
            agent_count: 1500,
            time_step: 0.016,
            random_seed: 42,
            wrap_edges: true,

            // Agent physical properties
            agent_speed_min: 0.005,
            agent_speed_max: 0.03,
            agent_turn_rate: 1.5,
            sensor_range: 0.15,
            sensor_count: 3,
            sensor_angle: 0.4,
            brownian_motion_strength: 0.002,

            // Chemical environment
            chemical_resolution: 256,
            chemical_types: 6,
            chemical_diffusion_rate: 0.08,
            chemical_decay_rate: 0.015,
            chemical_deposition_rate: 0.3,

            // Ecological roles and variants - Balanced for stable coexistence
            ecological_roles: 3,
            variants_per_role: 3,
            recycler_efficiency: 0.65,
            producer_photosynthesis_rate: 0.75,
            predator_hunting_efficiency: 0.55,

            // Energy and metabolism - Tuned for stable oscillations
            energy_consumption_rate: 0.003,
            energy_gain_from_food: 30.0,
            reproduction_energy_threshold: 70.0,
            reproduction_probability: 0.02,
            mutation_rate: 0.02,

            // Unified nutrient architecture
            max_particles: 600,
            particle_decomposition_rate: 0.01,
            particle_decay_rate: 0.005,
            matter_to_chemical_ratio: 0.75,

            // Fluid dynamics system
            enable_fluid_dynamics: true,
            fluid_viscosity: 0.1,
            fluid_density: 1.0,
            biological_current_strength: 0.05,
            chemical_current_strength: 0.03,
            flow_update_frequency: 4,

            // Light gradient system
            enable_light_gradient: true,
            base_light_intensity: 0.3,
            light_gradient_strength: 0.7,
            light_rotation_speed: 0.1,
            light_direction_angle: 0.0,

            // Movement and sensing systems
            chemotaxis_sensitivity: 0.5,
            run_duration_min: 0.5,
            run_duration_max: 2.0,
            tumble_angle_range: 3.14159,
            flagella_strength: 1.0,
            receptor_saturation_threshold: 5.0,

            // Hunting mechanics - Balanced for sustainable predation
            predation_contact_range: 0.045,
            pack_hunting_bonus: 0.8,
            predation_success_rate: 0.15,

            // Spatial organization
            enable_biofilm_formation: true,
            biofilm_growth_rate: 0.02,
            biofilm_persistence: 0.95,
            nutrient_stream_threshold: 2.0,
            territory_establishment_range: 0.3,

            // Population dynamics - Tuned for Lotka-Volterra oscillations
            carrying_capacity: 1800.0,
            population_oscillation_damping: 0.05,
            resource_competition_strength: 0.25,
            succession_pattern_strength: 0.12,

            // Environmental factors
            temperature_gradient_strength: 0.1,
            ph_gradient_strength: 0.05,
            toxin_accumulation_rate: 0.01,
            dead_zone_threshold: 8.0,

            // Rendering options
            show_energy_as_size: false,
            show_chemical_fields: false,
            show_oxygen: false,
            show_co2: false,
            show_nitrogen: false,
            show_pheromones: false,
            show_toxins: false,
            show_attractants: false,
            show_light_gradient: false,
            show_temperature_zones: false,
            show_ph_zones: false,
            chemical_field_opacity: 0.5,
            environmental_opacity: 0.3,
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        // Randomize basic simulation parameters
        self.agent_count = rng.random_range(1000..2000);
        self.time_step = rng.random_range(0.005..0.05);
        self.random_seed = rng.random();
        self.wrap_edges = rng.random();

        // Randomize agent physical properties
        self.agent_speed_min = rng.random_range(0.0005..0.002);
        self.agent_speed_max = self.agent_speed_min + rng.random_range(0.001..0.008);
        self.agent_turn_rate = rng.random_range(0.5..4.0);
        self.sensor_range = rng.random_range(0.05..0.3);
        self.sensor_count = rng.random_range(2..5);
        self.sensor_angle = rng.random_range(0.2..1.0);
        self.brownian_motion_strength = rng.random_range(0.0001..0.005);

        // Randomize chemical properties
        self.chemical_diffusion_rate = rng.random_range(0.05..0.3);
        self.chemical_decay_rate = rng.random_range(0.001..0.05);
        self.chemical_deposition_rate = rng.random_range(0.5..2.0);

        // Randomize ecological roles and variants
        self.ecological_roles = rng.random_range(2..4);
        self.variants_per_role = rng.random_range(2..5);
        self.recycler_efficiency = rng.random_range(0.5..1.0);
        self.producer_photosynthesis_rate = rng.random_range(0.5..1.0);
        self.predator_hunting_efficiency = rng.random_range(0.3..1.0);

        // Randomize energy and metabolism
        self.energy_consumption_rate = rng.random_range(0.001..0.01);
        self.energy_gain_from_food = rng.random_range(5.0..20.0);
        self.reproduction_energy_threshold = rng.random_range(50.0..100.0);
        self.reproduction_probability = rng.random_range(0.005..0.05);
        self.mutation_rate = rng.random_range(0.005..0.1);

        // Randomize unified nutrient architecture
        self.max_particles = rng.random_range(500..1000);
        self.particle_decomposition_rate = rng.random_range(0.001..0.05);
        self.particle_decay_rate = rng.random_range(0.001..0.05);
        self.matter_to_chemical_ratio = rng.random_range(0.5..1.0);

        // Randomize fluid dynamics system
        self.enable_fluid_dynamics = rng.random();
        self.fluid_viscosity = rng.random_range(0.05..0.5);
        self.fluid_density = rng.random_range(0.5..2.0);
        self.biological_current_strength = rng.random_range(0.01..0.1);
        self.chemical_current_strength = rng.random_range(0.01..0.1);
        self.flow_update_frequency = rng.random_range(2..10);

        // Randomize light gradient system
        self.enable_light_gradient = rng.random();
        self.base_light_intensity = rng.random_range(0.1..1.0);
        self.light_gradient_strength = rng.random_range(0.1..1.0);
        self.light_rotation_speed = rng.random_range(0.01..0.5);
        self.light_direction_angle = rng.random_range(0.0..3.14); // 0 to 2*PI

        // Randomize movement and sensing systems
        self.chemotaxis_sensitivity = rng.random_range(0.1..1.0);
        self.run_duration_min = rng.random_range(0.1..1.0);
        self.run_duration_max = self.run_duration_min + rng.random_range(0.1..2.0);
        self.tumble_angle_range = rng.random_range(0.1..3.14); // 0 to 2*PI
        self.flagella_strength = rng.random_range(0.01..1.0);
        self.receptor_saturation_threshold = rng.random_range(1.0..10.0);

        // Randomize hunting mechanics
        self.predation_contact_range = rng.random_range(0.01..0.2);
        self.pack_hunting_bonus = rng.random_range(0.0..1.0);
        self.predation_success_rate = rng.random_range(0.1..1.0);

        // Randomize spatial organization
        self.enable_biofilm_formation = rng.random();
        self.biofilm_growth_rate = rng.random_range(0.005..0.1);
        self.biofilm_persistence = rng.random_range(0.8..1.0);
        self.nutrient_stream_threshold = rng.random_range(1.0..5.0);
        self.territory_establishment_range = rng.random_range(0.1..0.5);

        // Randomize population dynamics
        self.carrying_capacity = rng.random_range(500.0..1500.0);
        self.population_oscillation_damping = rng.random_range(0.05..0.5);
        self.resource_competition_strength = rng.random_range(0.2..0.8);
        self.succession_pattern_strength = rng.random_range(0.05..0.5);

        // Randomize environmental factors
        self.temperature_gradient_strength = rng.random_range(0.1..1.0);
        self.ph_gradient_strength = rng.random_range(0.05..0.5);
        self.toxin_accumulation_rate = rng.random_range(0.001..0.1);
        self.dead_zone_threshold = rng.random_range(3.0..10.0);

        // Randomize rendering options
        self.show_energy_as_size = rng.random();
        self.show_chemical_fields = rng.random();
        self.show_oxygen = rng.random();
        self.show_co2 = rng.random();
        self.show_nitrogen = rng.random();
        self.show_pheromones = rng.random();
        self.show_toxins = rng.random();
        self.show_attractants = rng.random();
        self.show_light_gradient = rng.random();
        self.show_temperature_zones = rng.random();
        self.show_ph_zones = rng.random();
        self.chemical_field_opacity = rng.random_range(0.1..0.5);
        self.environmental_opacity = rng.random_range(0.05..0.3);

        self.random_seed = rng.random();
    }

    /// Set visual mode to one of the preset configurations
    pub fn set_visual_mode(&mut self, mode: &str) {
        match mode {
            "minimal" => {
                self.show_chemical_fields = false;
                self.show_light_gradient = false;
                self.show_temperature_zones = false;
                self.show_ph_zones = false;
                self.environmental_opacity = 0.05;
            }
            "ecological" => {
                self.show_chemical_fields = true;
                self.show_oxygen = true;
                self.show_co2 = false;
                self.show_nitrogen = false;
                self.show_pheromones = true;
                self.show_toxins = true;
                self.show_attractants = true;
                self.chemical_field_opacity = 0.2;
                self.show_light_gradient = true;
                self.show_temperature_zones = false;
                self.show_ph_zones = false;
                self.environmental_opacity = 0.15;
            }
            "chemical" => {
                self.show_chemical_fields = true;
                self.show_oxygen = true;
                self.show_co2 = true;
                self.show_nitrogen = true;
                self.show_pheromones = true;
                self.show_toxins = true;
                self.show_attractants = true;
                self.chemical_field_opacity = 0.4;
                self.show_light_gradient = true;
                self.show_temperature_zones = false;
                self.show_ph_zones = false;
                self.environmental_opacity = 0.2;
            }
            "environmental" => {
                self.show_chemical_fields = true;
                self.show_oxygen = true;
                self.show_co2 = true;
                self.show_nitrogen = true;
                self.show_pheromones = false;
                self.show_toxins = false;
                self.show_attractants = false;
                self.chemical_field_opacity = 0.2;
                self.show_light_gradient = true;
                self.show_temperature_zones = true;
                self.show_ph_zones = true;
                self.environmental_opacity = 0.3;
            }
            "debug" => {
                self.show_chemical_fields = true;
                self.show_oxygen = true;
                self.show_co2 = true;
                self.show_nitrogen = true;
                self.show_pheromones = true;
                self.show_toxins = true;
                self.show_attractants = true;
                self.chemical_field_opacity = 0.5;
                self.show_light_gradient = true;
                self.show_temperature_zones = true;
                self.show_ph_zones = true;
                self.environmental_opacity = 0.4;
            }
            _ => {
                // Default to ecological mode
                self.set_visual_mode("ecological");
            }
        }
    }

    /// Get list of available visual modes
    pub fn get_visual_modes() -> Vec<&'static str> {
        vec!["minimal", "ecological", "chemical", "environmental", "debug"]
    }
}
