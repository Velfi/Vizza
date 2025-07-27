use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;
use tokio::sync::oneshot;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::Settings;
use super::shaders::{
    AGENT_UPDATE_SHADER, CHEMICAL_DIFFUSION_SHADER, FLUID_DYNAMICS_SHADER, RENDER_SHADER,
};
use crate::simulations::shared::{LutManager, camera::Camera};
use crate::simulations::traits::Simulation;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Agent {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: f32,
    pub age: f32,
    pub ecological_role: u32, // 0: Recycler, 1: Producer, 2: Predator
    pub variant: u32,         // Variant within ecological role

    // Sensor array: 3-4 chemical receptors pointing different directions
    pub sensor_readings: [f32; 4],

    // Movement engine parameters for run-and-tumble
    pub heading: f32,
    pub run_duration: f32,    // Current run duration in run-and-tumble
    pub run_timer: f32,       // Timer for current run
    pub tumble_cooldown: f32, // Cooldown after tumbling

    // Metabolic system
    pub metabolism_rate: f32,
    pub reproductive_threshold: f32,
    pub last_reproduction_time: f32,

    // Behavioral state: 0: feeding, 1: hunting, 2: reproducing, 3: escaping
    pub behavioral_state: u32,
    pub state_timer: f32, // Timer for current state

    // Chemical secretion rates for 6 chemical types
    pub chemical_secretion_rates: [f32; 6],

    // Simple memory: recent food locations and threats
    pub food_memory: [f32; 4],   // x, y positions of recent food
    pub threat_memory: [f32; 4], // x, y positions of recent threats

    // Biofilm formation (for producers)
    pub biofilm_strength: f32,
    pub biofilm_connections: u32,

    // Hunting mechanics (for predators)
    pub hunt_target_id: u32,
    pub pack_coordination: f32,

    // Spatial organization
    pub territory_center: [f32; 2],
    pub territory_radius: f32,

    // Visibility control
    pub is_visible: u32, // 0 = hidden, 1 = visible

    pub _pad: [u32; 1],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct DeadBiomass {
    pub position: [f32; 2],
    pub biomass_amount: f32,
    pub species_origin: u32,         // What species died
    pub decay_time: f32,             // Time until natural decay
    pub decomposition_progress: f32, // How much has been decomposed (0.0 to 1.0)
    pub is_active: u32,
    pub _pad: [u32; 1], // Padding for 32-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimParams {
    pub agent_count: u32,
    pub width: f32,
    pub height: f32,
    pub dt: f32,

    // Agent parameters
    pub agent_speed_min: f32,
    pub agent_speed_max: f32,
    pub agent_turn_rate: f32,
    pub sensor_range: f32,
    pub sensor_count: u32,
    pub sensor_angle: f32,
    pub brownian_motion_strength: f32,

    // Chemical parameters
    pub chemical_resolution: u32,
    pub chemical_types: u32,
    pub chemical_diffusion_rate: f32,
    pub chemical_decay_rate: f32,
    pub chemical_deposition_rate: f32,

    // Ecological parameters (fixed at 3 roles, 3 variants per role = 9 total species)
    pub recycler_efficiency: f32,
    pub producer_photosynthesis_rate: f32,
    pub predator_hunting_efficiency: f32,

    // Energy and metabolism
    pub energy_consumption_rate: f32,
    pub energy_gain_from_food: f32,
    pub reproduction_energy_threshold: f32,
    pub reproduction_probability: f32,
    pub mutation_rate: f32,

    // Unified nutrient architecture
    pub max_particles: u32,
    pub particle_decomposition_rate: f32,
    pub particle_decay_rate: f32,
    pub matter_to_chemical_ratio: f32,

    // Fluid dynamics
    pub enable_fluid_dynamics: u32,
    pub fluid_viscosity: f32,
    pub fluid_density: f32,
    pub biological_current_strength: f32,
    pub chemical_current_strength: f32,
    pub flow_update_frequency: u32,

    // Light gradient
    pub enable_light_gradient: u32,
    pub base_light_intensity: f32,
    pub light_gradient_strength: f32,
    pub light_rotation_speed: f32,
    pub light_direction_angle: f32,

    // Movement and sensing
    pub chemotaxis_sensitivity: f32,
    pub run_duration_min: f32,
    pub run_duration_max: f32,
    pub tumble_angle_range: f32,
    pub flagella_strength: f32,
    pub receptor_saturation_threshold: f32,

    // Hunting mechanics
    pub predation_contact_range: f32,
    pub pack_hunting_bonus: f32,
    pub predation_success_rate: f32,

    // Spatial organization
    pub enable_biofilm_formation: u32,
    pub biofilm_growth_rate: f32,
    pub biofilm_persistence: f32,
    pub nutrient_stream_threshold: f32,
    pub territory_establishment_range: f32,

    // Population dynamics
    pub carrying_capacity: f32,
    pub population_oscillation_damping: f32,
    pub resource_competition_strength: f32,
    pub succession_pattern_strength: f32,

    // Environmental factors
    pub temperature_gradient_strength: f32,
    pub ph_gradient_strength: f32,
    pub toxin_accumulation_rate: f32,
    pub dead_zone_threshold: f32,

    pub wrap_edges: u32,
    pub random_seed: u32,
    pub time: f32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ChemicalDiffusionParams {
    pub chemical_resolution: u32,
    pub chemical_types: u32,
    pub chemical_diffusion_rate: f32,
    pub chemical_decay_rate: f32,
    pub dt: f32,
    pub enable_fluid_dynamics: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderParams {
    pub show_energy_as_size: u32,
    pub time: f32,

    pub show_chemical_fields: u32,
    pub chemical_field_opacity: f32,
    pub show_light_gradient: u32,
    pub environmental_opacity: f32,

    pub chemical_resolution: f32,

    // Individual chemical type flags
    pub show_oxygen: u32,
    pub show_co2: u32,
    pub show_nitrogen: u32,
    pub show_pheromones: u32,
    pub show_toxins: u32,
    pub show_attractants: u32,

    // Environmental overlay flags
    pub show_temperature_zones: u32,
    pub show_ph_zones: u32,

    pub _pad: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

#[derive(Debug, Clone)]
pub struct PopulationData {
    pub time: f32,
    pub species_counts: Vec<u32>,
    pub total_population: u32,
}

#[derive(Debug, Clone)]
pub struct PopulationHistory {
    pub data: VecDeque<PopulationData>,
    pub max_history_size: usize,
}

impl PopulationHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::new(),
            max_history_size: max_size,
        }
    }

    pub fn add_sample(&mut self, sample: PopulationData) {
        if self.data.len() >= self.max_history_size {
            self.data.pop_front();
        }
        self.data.push_back(sample);
    }

    pub fn get_latest(&self) -> Option<&PopulationData> {
        self.data.back()
    }

    pub fn get_all(&self) -> Vec<&PopulationData> {
        self.data.iter().collect()
    }
}

/// Ecosystem simulation model
#[derive(Debug)]
pub struct EcosystemModel {
    // GPU resources
    pub agent_buffer: wgpu::Buffer,
    pub biomass_buffer: wgpu::Buffer,
    pub chemical_field_buffer: wgpu::Buffer,
    pub chemical_field_temp_buffer: wgpu::Buffer,
    pub velocity_field_buffer: wgpu::Buffer,
    pub pressure_field_buffer: wgpu::Buffer,
    pub biofilm_field_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub chemical_diffusion_params_buffer: wgpu::Buffer,
    pub render_params_buffer: wgpu::Buffer,
    pub species_colors_buffer: wgpu::Buffer,
    pub visibility_buffer: wgpu::Buffer,

    // Compute pipelines
    pub agent_update_pipeline: wgpu::ComputePipeline,
    pub agent_update_bind_group: wgpu::BindGroup,
    pub chemical_diffusion_pipeline: wgpu::ComputePipeline,
    pub chemical_diffusion_bind_group: wgpu::BindGroup,
    pub fluid_dynamics_pipeline: wgpu::ComputePipeline,
    pub fluid_dynamics_bind_group: wgpu::BindGroup,

    // Render pipelines
    pub background_render_pipeline: wgpu::RenderPipeline,
    pub background_bind_group: wgpu::BindGroup,

    pub biomass_render_pipeline: wgpu::RenderPipeline,
    pub biomass_bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
    pub camera_bind_group: wgpu::BindGroup,
    pub render_params_bind_group: wgpu::BindGroup,
    pub species_colors_bind_group: wgpu::BindGroup,
    pub agent_buffer_bind_group: wgpu::BindGroup,

    // Camera and state
    pub camera: Camera,
    pub settings: Settings,
    pub gui_visible: bool,
    pub time: f32,
    pub species_colors: Vec<[f32; 4]>,
    pub last_frame_time: std::time::Instant,
    pub population_history: PopulationHistory,

    // LUT management
    pub lut_manager: Arc<LutManager>,

    // Visibility state management
    pub visibility_flags: Vec<u32>,
}

// Implementation methods are included inline

impl SimParams {
    pub fn from_settings(settings: &Settings, _width: f32, _height: f32, agent_count: u32) -> Self {
        Self {
            agent_count,
            width: 2.0,
            height: 2.0,
            dt: settings.time_step,

            // Agent parameters
            agent_speed_min: settings.agent_speed_min,
            agent_speed_max: settings.agent_speed_max,
            agent_turn_rate: settings.agent_turn_rate,
            sensor_range: settings.sensor_range,
            sensor_count: settings.sensor_count,
            sensor_angle: settings.sensor_angle,
            brownian_motion_strength: settings.brownian_motion_strength,

            // Chemical parameters
            chemical_resolution: settings.chemical_resolution,
            chemical_types: settings.chemical_types,
            chemical_diffusion_rate: settings.chemical_diffusion_rate,
            chemical_decay_rate: settings.chemical_decay_rate,
            chemical_deposition_rate: settings.chemical_deposition_rate,

            // Ecological parameters (fixed at 3 roles, 3 variants per role = 9 total species)
            recycler_efficiency: settings.recycler_efficiency,
            producer_photosynthesis_rate: settings.producer_photosynthesis_rate,
            predator_hunting_efficiency: settings.predator_hunting_efficiency,

            // Energy and metabolism
            energy_consumption_rate: settings.energy_consumption_rate,
            energy_gain_from_food: settings.energy_gain_from_food,
            reproduction_energy_threshold: settings.reproduction_energy_threshold,
            reproduction_probability: settings.reproduction_probability,
            mutation_rate: settings.mutation_rate,

            // Unified nutrient architecture
            max_particles: settings.max_particles,
            particle_decomposition_rate: settings.particle_decomposition_rate,
            particle_decay_rate: settings.particle_decay_rate,
            matter_to_chemical_ratio: settings.matter_to_chemical_ratio,

            // Fluid dynamics
            enable_fluid_dynamics: if settings.enable_fluid_dynamics { 1 } else { 0 },
            fluid_viscosity: settings.fluid_viscosity,
            fluid_density: settings.fluid_density,
            biological_current_strength: settings.biological_current_strength,
            chemical_current_strength: settings.chemical_current_strength,
            flow_update_frequency: settings.flow_update_frequency,

            // Light gradient
            enable_light_gradient: if settings.enable_light_gradient { 1 } else { 0 },
            base_light_intensity: settings.base_light_intensity,
            light_gradient_strength: settings.light_gradient_strength,
            light_rotation_speed: settings.light_rotation_speed,
            light_direction_angle: settings.light_direction_angle,

            // Movement and sensing
            chemotaxis_sensitivity: settings.chemotaxis_sensitivity,
            run_duration_min: settings.run_duration_min,
            run_duration_max: settings.run_duration_max,
            tumble_angle_range: settings.tumble_angle_range,
            flagella_strength: settings.flagella_strength,
            receptor_saturation_threshold: settings.receptor_saturation_threshold,

            // Hunting mechanics
            predation_contact_range: settings.predation_contact_range,
            pack_hunting_bonus: settings.pack_hunting_bonus,
            predation_success_rate: settings.predation_success_rate,

            // Spatial organization
            enable_biofilm_formation: if settings.enable_biofilm_formation {
                1
            } else {
                0
            },
            biofilm_growth_rate: settings.biofilm_growth_rate,
            biofilm_persistence: settings.biofilm_persistence,
            nutrient_stream_threshold: settings.nutrient_stream_threshold,
            territory_establishment_range: settings.territory_establishment_range,

            // Population dynamics
            carrying_capacity: settings.carrying_capacity,
            population_oscillation_damping: settings.population_oscillation_damping,
            resource_competition_strength: settings.resource_competition_strength,
            succession_pattern_strength: settings.succession_pattern_strength,

            // Environmental factors
            temperature_gradient_strength: settings.temperature_gradient_strength,
            ph_gradient_strength: settings.ph_gradient_strength,
            toxin_accumulation_rate: settings.toxin_accumulation_rate,
            dead_zone_threshold: settings.dead_zone_threshold,

            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            random_seed: settings.random_seed,
            time: 0.0,
            _pad: 0,
        }
    }
}

impl ChemicalDiffusionParams {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            chemical_resolution: settings.chemical_resolution,
            chemical_types: settings.chemical_types,
            chemical_diffusion_rate: settings.chemical_diffusion_rate,
            chemical_decay_rate: settings.chemical_decay_rate,
            dt: settings.time_step,
            enable_fluid_dynamics: 1, // Always enabled for the new ecosystem
        }
    }
}

impl RenderParams {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            show_energy_as_size: if settings.show_energy_as_size { 1 } else { 0 },
            time: 0.0,
            show_chemical_fields: if settings.show_chemical_fields { 1 } else { 0 },
            chemical_field_opacity: settings.chemical_field_opacity,
            show_light_gradient: if settings.show_light_gradient { 1 } else { 0 },
            environmental_opacity: settings.environmental_opacity,
            chemical_resolution: settings.chemical_resolution as f32,
            show_oxygen: if settings.show_oxygen { 1 } else { 0 },
            show_co2: if settings.show_co2 { 1 } else { 0 },
            show_nitrogen: if settings.show_nitrogen { 1 } else { 0 },
            show_pheromones: if settings.show_pheromones { 1 } else { 0 },
            show_toxins: if settings.show_toxins { 1 } else { 0 },
            show_attractants: if settings.show_attractants { 1 } else { 0 },
            show_temperature_zones: if settings.show_temperature_zones {
                1
            } else {
                0
            },
            show_ph_zones: if settings.show_ph_zones { 1 } else { 0 },
            _pad: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }
}

impl EcosystemModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        agent_count: u32,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> Result<Self, SimulationError> {
        let width = surface_config.width as f32;
        let height = surface_config.height as f32;

        // Initialize camera
        let camera = Camera::new(device, width, height)?;

        // Generate species colors (fixed at 3 roles * 3 variants = 9 species)
        let species_colors = Self::generate_species_colors(9);

        // Create buffers - simplified for now
        // Initialize agents with random positions and properties
        // Use normalized coordinate system [-1, 1] to match other simulations
        let mut agents = Vec::with_capacity(agent_count as usize);
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(settings.random_seed as u64);

        for i in 0..agent_count {
            let ecological_role = i % 3; // 0: Recycler, 1: Producer, 2: Predator
            let variant = (i / 3) % 3; // Fixed at 3 variants per role

            // Ecological role-specific initialization following design document
            let (metabolism, chemical_secretion_rates) = match ecological_role {
                0 => {
                    // Recyclers - break down dead matter and waste into usable nutrients
                    let mut rates = [0.0; 6];
                    match variant {
                        0 => {
                            // Bacteria - small, fast-moving, swarm behavior around food sources
                            rates[0] = -0.4; // Consume oxygen for decomposition
                            rates[1] = 0.6; // Produce CO2 from organic matter breakdown
                            rates[2] = 0.9; // Produce nitrogen compounds from protein breakdown
                            rates[5] = 0.2; // Produce attractants to signal food sources
                            (1.0, rates)
                        }
                        1 => {
                            // Fungi - slower movement, create visible thread networks
                            rates[0] = -0.2; // Consume oxygen for complex matter breakdown
                            rates[2] = 1.2; // Excellent at producing nitrogen compounds
                            rates[5] = 0.8; // Strong attractant production for network formation
                            (0.5, rates)
                        }
                        _ => {
                            // Decomposer Protozoans - larger, engulf debris particles whole
                            rates[0] = -0.6; // High oxygen consumption for large-scale decomposition
                            rates[1] = 0.4; // Moderate CO2 production
                            rates[2] = 0.8; // Good nitrogen production from engulfed matter
                            rates[5] = 0.1; // Low attractant production (more selective)
                            (0.7, rates)
                        }
                    }
                }
                1 => {
                    // Producers - convert raw nutrients and light energy into biomass
                    let mut rates = [0.0; 6];
                    match variant {
                        0 => {
                            // Algae - form biofilm mats, highly light-dependent, create persistent structures
                            rates[0] = 1.0; // High oxygen production from photosynthesis
                            rates[1] = -0.6; // High CO2 consumption for biomass building
                            rates[5] = 0.4; // Moderate attractant production for mat formation
                            (0.4, rates)
                        }
                        1 => {
                            // Cyanobacteria - mobile colonies, moderate light needs, can fix nitrogen
                            rates[0] = 0.7; // Good oxygen production
                            rates[1] = -0.4; // Moderate CO2 consumption
                            rates[2] = 0.3; // Nitrogen fixation from environment
                            rates[3] = 0.2; // Pheromone production for colony coordination
                            (0.6, rates)
                        }
                        _ => {
                            // Photosynthetic Protists - larger individual organisms, complex movement, efficient nutrient use
                            rates[0] = 0.8; // Efficient oxygen production
                            rates[1] = -0.7; // High CO2 consumption for efficient photosynthesis
                            rates[2] = 0.1; // Efficient nitrogen use (low production)
                            (0.3, rates)
                        }
                    }
                }
                _ => {
                    // Predators - control population dynamics through consumption
                    let mut rates = [0.0; 6];
                    match variant {
                        0 => {
                            // Predatory Bacteria - small, fast, hunt in coordinated groups
                            rates[0] = -0.3; // Consume oxygen for rapid movement
                            rates[1] = 0.4; // Produce CO2 from respiration
                            rates[3] = 0.6; // High pheromone production for pack coordination
                            rates[4] = 0.1; // Low toxin production for prey stunning
                            (1.1, rates)
                        }
                        1 => {
                            // Viruses - inject into hosts, replicate internally, burst out after delay
                            rates[4] = 0.3; // Produce toxins for host injection
                            rates[3] = 0.2; // Pheromone production for target recognition
                            (1.5, rates)
                        }
                        _ => {
                            // Predatory Protozoans - large, engulf smaller organisms, slow but powerful
                            rates[0] = -0.5; // High oxygen consumption for large body processes
                            rates[1] = 0.5; // CO2 production from consumed organisms
                            rates[4] = 0.2; // Toxin production for prey immobilization
                            (0.5, rates)
                        }
                    }
                }
            };

            agents.push(Agent {
                position: [rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)],
                velocity: [rng.random_range(-0.01..0.01), rng.random_range(-0.01..0.01)],
                energy: 50.0,
                age: 0.0,
                ecological_role,
                variant,
                sensor_readings: [0.0; 4],
                heading: rng.random_range(0.0..std::f32::consts::TAU),
                run_duration: rng
                    .random_range(settings.run_duration_min..settings.run_duration_max),
                run_timer: 0.0,
                tumble_cooldown: 0.0,
                metabolism_rate: metabolism,
                reproductive_threshold: 70.0,
                last_reproduction_time: 0.0,
                behavioral_state: 0, // Start in feeding state
                state_timer: 0.0,
                chemical_secretion_rates,
                food_memory: [0.0; 4],
                threat_memory: [0.0; 4],
                biofilm_strength: 0.0,
                biofilm_connections: 0,
                hunt_target_id: 0,
                pack_coordination: 0.0,
                territory_center: [0.0, 0.0],
                territory_radius: 0.0,
                is_visible: 1, // Start visible
                _pad: [0],
            });
        }

        let agent_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Agent Buffer"),
            contents: bytemuck::cast_slice(&agents),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        tracing::info!("Agent struct size: {} bytes", mem::size_of::<Agent>());
        tracing::info!(
            "Agent buffer size: {} bytes ({} agents)",
            agent_buffer.size(),
            agents.len()
        );

        let biomass_buffer_size =
            (settings.max_particles as u64) * std::mem::size_of::<DeadBiomass>() as u64;
        let biomass_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Biomass Buffer"),
            size: biomass_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        tracing::info!(
            "Created biomass buffer: size={} bytes, max_particles={}, struct_size={} bytes",
            biomass_buffer_size,
            settings.max_particles,
            std::mem::size_of::<DeadBiomass>()
        );

        // Clear biomass buffer to prevent garbage data
        let clear_biomass_data = vec![
            DeadBiomass {
                position: [0.0, 0.0],
                biomass_amount: 0.0,
                species_origin: 0,
                decay_time: 0.0,
                decomposition_progress: 0.0,
                is_active: 0,
                _pad: [0],
            };
            settings.max_particles as usize
        ];
        queue.write_buffer(
            &biomass_buffer,
            0,
            bytemuck::cast_slice(&clear_biomass_data),
        );

        let chemical_buffer_size = (settings.chemical_resolution
            * settings.chemical_resolution
            * settings.chemical_types) as u64
            * 4; // 4 bytes per f32
        let chemical_field_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chemical Field Buffer"),
            size: chemical_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let chemical_field_temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chemical Field Temp Buffer"),
            size: chemical_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let sim_params = SimParams::from_settings(&settings, width, height, agent_count);
        let sim_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ecosystem Sim Params Buffer"),
            contents: bytemuck::cast_slice(&[sim_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let chemical_diffusion_params = ChemicalDiffusionParams::from_settings(&settings);
        let chemical_diffusion_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chemical Diffusion Params Buffer"),
                contents: bytemuck::cast_slice(&[chemical_diffusion_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let render_params = RenderParams::from_settings(&settings);
        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Params Buffer"),
            contents: bytemuck::cast_slice(&[render_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let species_colors_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Species Colors Buffer"),
            contents: bytemuck::cast_slice(&species_colors),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create visibility buffer - one flag per species variant
        let total_species = 9; // Fixed at 3 roles * 3 variants = 9 species
        let visibility_flags = vec![1u32; total_species as usize]; // Start all visible
        let visibility_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Visibility Buffer"),
            contents: bytemuck::cast_slice(&visibility_flags),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create fluid dynamics buffers
        let velocity_field_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Velocity Field Buffer"),
            size: (settings.chemical_resolution * settings.chemical_resolution) as u64 * 8, // 8 bytes per vec2<f32>
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pressure_field_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pressure Field Buffer"),
            size: (settings.chemical_resolution * settings.chemical_resolution) as u64 * 4, // 4 bytes per f32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let biofilm_field_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Biofilm Field Buffer"),
            size: (settings.chemical_resolution * settings.chemical_resolution) as u64 * 4, // 4 bytes per f32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create fluid dynamics pipeline
        let fluid_dynamics_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fluid Dynamics Shader"),
            source: wgpu::ShaderSource::Wgsl(FLUID_DYNAMICS_SHADER.into()),
        });

        let fluid_dynamics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Fluid Dynamics Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let fluid_dynamics_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Fluid Dynamics Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Fluid Dynamics Pipeline Layout"),
                        bind_group_layouts: &[&fluid_dynamics_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &fluid_dynamics_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        let fluid_dynamics_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fluid Dynamics Bind Group"),
            layout: &fluid_dynamics_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: agent_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chemical_field_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: velocity_field_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: pressure_field_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: sim_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create proper compute pipelines
        let (agent_update_pipeline, agent_update_bind_group) = Self::create_agent_update_pipeline(
            device,
            &agent_buffer,
            &biomass_buffer,
            &chemical_field_buffer,
            &sim_params_buffer,
        )?;
        let (chemical_diffusion_pipeline, chemical_diffusion_bind_group) =
            Self::create_chemical_diffusion_pipeline(
                device,
                &chemical_field_buffer,
                &chemical_field_temp_buffer,
                &chemical_diffusion_params_buffer,
                &velocity_field_buffer,
            )?;

        // Create render pipelines
        let (background_render_pipeline, background_bind_group) =
            Self::create_background_render_pipeline(
                device,
                surface_config,
                &camera,
                &render_params_buffer,
                &chemical_field_buffer,
            )?;

        let (biomass_render_pipeline, biomass_bind_group) = Self::create_biomass_render_pipeline(
            device,
            surface_config,
            &camera,
            &render_params_buffer,
            &biomass_buffer,
        )?;

        // Create bind group layouts for rendering
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Params Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let species_colors_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Species Colors Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let agent_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Agent Buffer Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create minimal render pipeline
        let render_pipeline = Self::create_minimal_render_pipeline(device, surface_config)?;

        // Create proper bind groups for rendering
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
        });

        let render_params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Params Bind Group"),
            layout: &render_params_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: render_params_buffer.as_entire_binding(),
            }],
        });

        let species_colors_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Species Colors Bind Group"),
            layout: &species_colors_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: species_colors_buffer.as_entire_binding(),
            }],
        });

        let agent_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Agent Buffer Bind Group"),
            layout: &agent_buffer_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: agent_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: visibility_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(Self {
            agent_buffer,

            biomass_buffer,
            chemical_field_buffer,
            chemical_field_temp_buffer,
            velocity_field_buffer,
            pressure_field_buffer,
            biofilm_field_buffer,
            sim_params_buffer,
            chemical_diffusion_params_buffer,
            render_params_buffer,
            species_colors_buffer,
            visibility_buffer,
            agent_update_pipeline,
            agent_update_bind_group,
            chemical_diffusion_pipeline,
            chemical_diffusion_bind_group,
            fluid_dynamics_pipeline,
            fluid_dynamics_bind_group,
            background_render_pipeline,
            background_bind_group,

            biomass_render_pipeline,
            biomass_bind_group,
            render_pipeline,
            camera_bind_group,
            render_params_bind_group,
            species_colors_bind_group,
            agent_buffer_bind_group,
            camera,
            settings,
            gui_visible: true,
            time: 0.0,
            species_colors,
            last_frame_time: std::time::Instant::now(),
            population_history: PopulationHistory::new(100), // Store last 100 frames
            lut_manager: Arc::new(lut_manager.clone()),
            visibility_flags,
        })
    }

    /// Fallback population counting (only used if GPU readback fails)
    fn count_populations_fallback(&self) -> Vec<u32> {
        let mut counts = vec![0u32; 3]; // Fixed at 3 ecological roles
        let total_agents = self.settings.agent_count;

        tracing::debug!(
            "Fallback population count: total_agents={}, ecological_roles=3",
            total_agents
        );

        // Only count if we have agents
        if total_agents > 0 {
            // Distribute agents evenly among ecological roles (this is how they're initialized)
            for i in 0..total_agents {
                let ecological_role = (i % 3) as usize; // Fixed at 3 roles
                if ecological_role < counts.len() {
                    counts[ecological_role] += 1;
                } else {
                    tracing::error!(
                        "Invalid ecological role index {} for fixed 3 roles",
                        ecological_role
                    );
                }
            }
        } else {
            tracing::warn!("No agents to count: agents={}", total_agents);
        }

        tracing::debug!("Fallback population counts: {:?}", counts);

        counts
    }

    /// Count living agents by reading back from GPU buffer
    pub async fn count_living_populations(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<u32> {
        let mut counts = vec![0u32; 3]; // Fixed at 3 ecological roles

        tracing::debug!(
            "Starting GPU population count: agent_count={}, ecological_roles=3",
            self.settings.agent_count
        );

        // Create a staging buffer to read data from GPU
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Population Count Staging Buffer"),
            size: self.agent_buffer.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy agent data from GPU to staging buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Population Count Encoder"),
        });

        encoder.copy_buffer_to_buffer(
            &self.agent_buffer,
            0,
            &staging_buffer,
            0,
            self.agent_buffer.size(),
        );

        queue.submit(Some(encoder.finish()));

        // Map the staging buffer and read the data
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // Wait for the buffer to be mapped
        device.poll(wgpu::Maintain::Wait);

        match receiver.await {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let agents: &[Agent] = bytemuck::cast_slice(&data);

                tracing::debug!("Successfully read {} agents from GPU buffer", agents.len());

                // Count living agents by species
                let mut total_agents = 0;
                let mut living_agents = 0;
                let mut energy_sum = 0.0;
                let mut min_energy = f32::INFINITY;
                let mut max_energy = f32::NEG_INFINITY;

                for (i, agent) in agents.iter().enumerate() {
                    total_agents += 1;
                    energy_sum += agent.energy;
                    min_energy = min_energy.min(agent.energy);
                    max_energy = max_energy.max(agent.energy);

                    // Count all agents with positive energy as living
                    if agent.energy > 0.0 {
                        living_agents += 1;
                        let species_idx = agent.ecological_role as usize;
                        if species_idx < counts.len() {
                            counts[species_idx] += 1;
                        } else {
                            tracing::warn!(
                                "Agent {} has invalid species index: {} (max: {})",
                                i,
                                species_idx,
                                counts.len() - 1
                            );
                        }
                    }

                    // Debug first few agents and a few random ones
                    if i < 3 || (i % 500 == 0 && i < 1500) {
                        tracing::debug!(
                            "Agent {}: species={}, energy={:.2}, age={:.2}, position=({:.2}, {:.2})",
                            i,
                            agent.ecological_role,
                            agent.energy,
                            agent.age,
                            agent.position[0],
                            agent.position[1]
                        );
                    }
                }

                let avg_energy = if total_agents > 0 {
                    energy_sum / total_agents as f32
                } else {
                    0.0
                };

                tracing::debug!(
                    "GPU population analysis: total={}, living={}, avg_energy={:.2}, min_energy={:.2}, max_energy={:.2}",
                    total_agents,
                    living_agents,
                    avg_energy,
                    min_energy,
                    max_energy
                );
                tracing::debug!("GPU species counts: {:?}", counts);

                return counts;
            }
            Ok(Err(e)) => {
                tracing::error!("GPU buffer mapping failed: {:?}", e);
            }
            Err(e) => {
                tracing::error!("Failed to receive mapping result: {:?}", e);
            }
        }

        drop(staging_buffer);

        // Only fall back to static counting if there was an error reading from GPU
        tracing::warn!("GPU readback failed, falling back to static counting");
        self.count_populations_fallback()
    }

    /// Update population history with current population data using GPU readback
    pub async fn update_population_history(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let species_counts = self.count_living_populations(device, queue).await;
        let total_population = species_counts.iter().sum();

        let population_data = PopulationData {
            time: self.time,
            species_counts,
            total_population,
        };

        self.population_history.add_sample(population_data);
    }

    /// Get current population data by reading actual agent states from GPU
    pub async fn get_current_population(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> PopulationData {
        let current_counts = self.count_living_populations(device, queue).await;
        PopulationData {
            time: self.time,
            species_counts: current_counts.clone(),
            total_population: current_counts.iter().sum(),
        }
    }

    /// Get population history
    pub fn get_population_history(&self) -> Vec<PopulationData> {
        self.population_history.data.iter().cloned().collect()
    }

    /// Update population history (called from manager)
    pub async fn update_population_history_async(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) {
        self.update_population_history(device, queue).await;
        tracing::debug!("Population history updated successfully");
    }

    /// Create stable default preset following design document principles
    pub fn create_stable_default_preset(
        &self,
    ) -> crate::simulations::ecosystem::settings::Settings {
        // This preset is designed to produce stable Lotka-Volterra oscillations
        // with 3-4 year cycles as specified in the design document
        crate::simulations::ecosystem::settings::Settings::default()
    }

    fn generate_species_colors(total_species: u32) -> Vec<[f32; 4]> {
        let mut colors = Vec::new();

        // Colors with consistent schemes for each ecological role
        let species_colors = [
            // Recyclers (Role 0) - Shades of Green
            [0.298, 0.686, 0.314, 1.0], // Bacteria - Green (#4CAF50)
            [0.129, 0.588, 0.212, 1.0], // Fungi - Dark Green (#219622)
            [0.612, 0.847, 0.314, 1.0], // Decomposer Protozoans - Light Green (#9CD850)
            // Producers (Role 1) - Shades of Blue
            [0.129, 0.588, 0.952, 1.0], // Algae - Blue (#2196F3)
            [0.0, 0.588, 0.847, 1.0],   // Cyanobacteria - Dark Blue (#0096D8)
            [0.612, 0.847, 0.952, 1.0], // Photosynthetic Protists - Light Blue (#9CD8F3)
            // Predators (Role 2) - Shades of Red
            [0.957, 0.263, 0.212, 1.0], // Predatory Bacteria - Red (#F44336)
            [0.545, 0.0, 0.0, 1.0],     // Viruses - Dark Red (#8B0000)
            [1.0, 0.596, 0.596, 1.0],   // Predatory Protozoans - Light Red (#FF9898)
        ];

        for i in 0..total_species {
            let color_index = usize::min(i as usize, species_colors.len() - 1);
            colors.push(species_colors[color_index]);
        }

        colors
    }

    fn create_agent_update_pipeline(
        device: &Arc<Device>,
        agent_buffer: &wgpu::Buffer,
        biomass_buffer: &wgpu::Buffer,
        chemical_field_buffer: &wgpu::Buffer,
        sim_params_buffer: &wgpu::Buffer,
    ) -> Result<(wgpu::ComputePipeline, wgpu::BindGroup), SimulationError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Agent Update Shader"),
            source: wgpu::ShaderSource::Wgsl(AGENT_UPDATE_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Agent Update Bind Group Layout"),
            entries: &[
                // Agents buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Biomass buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Chemical field buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Simulation parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Agent Update Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Agent Update Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Agent Update Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: agent_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: biomass_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: chemical_field_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sim_params_buffer.as_entire_binding(),
                },
            ],
        });

        Ok((pipeline, bind_group))
    }

    fn create_chemical_diffusion_pipeline(
        device: &Arc<Device>,
        chemical_field_buffer: &wgpu::Buffer,
        chemical_field_temp_buffer: &wgpu::Buffer,
        chemical_diffusion_params_buffer: &wgpu::Buffer,
        velocity_field_buffer: &wgpu::Buffer,
    ) -> Result<(wgpu::ComputePipeline, wgpu::BindGroup), SimulationError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chemical Diffusion Shader"),
            source: wgpu::ShaderSource::Wgsl(CHEMICAL_DIFFUSION_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Chemical Diffusion Bind Group Layout"),
            entries: &[
                // Chemical field buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Chemical field temp buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Chemical diffusion parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Velocity field buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Chemical Diffusion Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Chemical Diffusion Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chemical Diffusion Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chemical_field_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chemical_field_temp_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: chemical_diffusion_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: velocity_field_buffer.as_entire_binding(),
                },
            ],
        });

        Ok((pipeline, bind_group))
    }

    fn create_background_render_pipeline(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
        camera: &Camera,
        render_params_buffer: &wgpu::Buffer,
        chemical_field_buffer: &wgpu::Buffer,
    ) -> Result<(wgpu::RenderPipeline, wgpu::BindGroup), SimulationError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::BACKGROUND_SHADER.into()),
        });

        // Create single bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Background Bind Group Layout"),
            entries: &[
                // Camera uniform - @group(0) @binding(0)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Render params uniform - @group(0) @binding(1)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Chemical field storage - @group(0) @binding(2)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Background Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Background Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: chemical_field_buffer.as_entire_binding(),
                },
            ],
        });

        Ok((pipeline, bind_group))
    }

    fn create_biomass_render_pipeline(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
        camera: &Camera,
        render_params_buffer: &wgpu::Buffer,
        biomass_buffer: &wgpu::Buffer,
    ) -> Result<(wgpu::RenderPipeline, wgpu::BindGroup), SimulationError> {
        // Create shader modules
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Biomass Particle Shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulations::ecosystem::shaders::RENDER_SHADER.into(),
            ),
        });

        // Create single bind group layout for all resources in group 0
        let biomass_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Biomass Bind Group Layout"),
                entries: &[
                    // Camera uniform (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Render params uniform (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Biomass buffer storage (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Biomass Render Pipeline Layout"),
            bind_group_layouts: &[&biomass_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Biomass Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Biomass Bind Group"),
            layout: &biomass_bind_group_layout,
            entries: &[
                // Camera uniform (binding 0)
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer().as_entire_binding(),
                },
                // Render params uniform (binding 1)
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_params_buffer.as_entire_binding(),
                },
                // Biomass buffer storage (binding 2)
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: biomass_buffer.as_entire_binding(),
                },
            ],
        });

        Ok((pipeline, bind_group))
    }

    fn create_minimal_render_pipeline(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<wgpu::RenderPipeline, SimulationError> {
        // No vertex buffer needed - we generate quad vertices in the shader

        // Create shader modules
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ecosystem Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
        });

        // Create bind group layouts
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Params Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let species_colors_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Species Colors Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let agent_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Agent Buffer Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ecosystem Render Pipeline Layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &render_params_bind_group_layout,
                &species_colors_bind_group_layout,
                &agent_buffer_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Ecosystem Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &vertex_shader, // Use same shader for fragment
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }),
        )
    }
}

// Simulation trait implementation
impl Simulation for EcosystemModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Calculate delta time for smooth camera movement
        let current_time = std::time::Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        // Clamp delta time to prevent large jumps (e.g., when tab is inactive)
        let delta_time = delta_time.min(1.0 / 30.0); // Max 30 FPS equivalent

        // Update time
        self.time += self.settings.time_step;

        // Get surface dimensions from camera
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

        // Update camera for smooth movement
        self.camera.update(delta_time);

        // Update simulation parameters with current time
        let mut sim_params =
            SimParams::from_settings(&self.settings, width, height, self.settings.agent_count);
        sim_params.time = self.time;
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        // Update render parameters
        let render_params = RenderParams {
            show_energy_as_size: if self.settings.show_energy_as_size {
                1
            } else {
                0
            },
            time: self.time,
            show_chemical_fields: if self.settings.show_chemical_fields {
                1
            } else {
                0
            },
            chemical_field_opacity: self.settings.chemical_field_opacity,
            show_light_gradient: if self.settings.show_light_gradient {
                1
            } else {
                0
            },
            environmental_opacity: self.settings.environmental_opacity,

            chemical_resolution: self.settings.chemical_resolution as f32,
            show_oxygen: if self.settings.show_oxygen { 1 } else { 0 },
            show_co2: if self.settings.show_co2 { 1 } else { 0 },
            show_nitrogen: if self.settings.show_nitrogen { 1 } else { 0 },
            show_pheromones: if self.settings.show_pheromones { 1 } else { 0 },
            show_toxins: if self.settings.show_toxins { 1 } else { 0 },
            show_attractants: if self.settings.show_attractants { 1 } else { 0 },
            show_temperature_zones: if self.settings.show_temperature_zones {
                1
            } else {
                0
            },
            show_ph_zones: if self.settings.show_ph_zones { 1 } else { 0 },
            _pad: 0,
            _pad2: 0,
            _pad3: 0,
        };
        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );

        // Update camera
        self.camera.upload_to_gpu(queue);

        // Update species colors if needed
        queue.write_buffer(
            &self.species_colors_buffer,
            0,
            bytemuck::cast_slice(&self.species_colors),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Ecosystem Render Encoder"),
        });

        // Run simulation steps
        let simulation_steps = 3;
        for step in 0..simulation_steps {
            // Update agents
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(&format!("Agent Update Pass {}", step)),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.agent_update_pipeline);
                compute_pass.set_bind_group(0, &self.agent_update_bind_group, &[]);

                let workgroup_size = 64;
                let num_workgroups = self.settings.agent_count.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);

                tracing::debug!(
                    "Dispatched agent update: workgroups={}, agent_count={}",
                    num_workgroups,
                    self.settings.agent_count
                );
            }

            // Update fluid dynamics to generate velocity field
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Fluid Dynamics Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.fluid_dynamics_pipeline);
                compute_pass.set_bind_group(0, &self.fluid_dynamics_bind_group, &[]);

                let workgroup_size = 8;
                let num_workgroups_x = self.settings.chemical_resolution.div_ceil(workgroup_size);
                let num_workgroups_y = self.settings.chemical_resolution.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            // Update chemical diffusion with advection
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Chemical Diffusion Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.chemical_diffusion_pipeline);
                compute_pass.set_bind_group(0, &self.chemical_diffusion_bind_group, &[]);

                let workgroup_size = 8;
                let num_workgroups_x = self.settings.chemical_resolution.div_ceil(workgroup_size);
                let num_workgroups_y = self.settings.chemical_resolution.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups_x, num_workgroups_y, 1);
            }

            // Copy temp buffer back to main buffer for next iteration
            encoder.copy_buffer_to_buffer(
                &self.chemical_field_temp_buffer,
                0,
                &self.chemical_field_buffer,
                0,
                self.chemical_field_temp_buffer.size(),
            );
        }

        // Render in multiple passes for layered visualization
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ecosystem Multi-Layer Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Pass 1: Render background (chemical fields and environmental overlays)
            if self.settings.show_chemical_fields
                || self.settings.show_light_gradient
                || self.settings.show_temperature_zones
                || self.settings.show_ph_zones
            {
                render_pass.set_pipeline(&self.background_render_pipeline);
                render_pass.set_bind_group(0, &self.background_bind_group, &[]);
                render_pass.draw(0..6, 0..9); // 3x3 grid = 9 instances
            }

            // Pass 2: Render biomass particles (decomposing organic matter)
            render_pass.set_pipeline(&self.biomass_render_pipeline);
            render_pass.set_bind_group(0, &self.biomass_bind_group, &[]);
            // Draw biomass particles as instanced quads
            let biomass_instance_count = self.settings.max_particles;
            render_pass.draw(0..6, 0..biomass_instance_count);

            // Pass 3: Render agents (microbes)
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_params_bind_group, &[]);
            render_pass.set_bind_group(2, &self.species_colors_bind_group, &[]);
            render_pass.set_bind_group(3, &self.agent_buffer_bind_group, &[]);
            // Draw agents as instanced quads with 3x3 grid
            let agent_instance_count = self.settings.agent_count * 9;
            render_pass.draw(0..6, 0..agent_instance_count);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Calculate delta time for smooth camera movement
        let current_time = std::time::Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        // Clamp delta time to prevent large jumps (e.g., when tab is inactive)
        let delta_time = delta_time.min(1.0 / 30.0); // Max 30 FPS equivalent

        // Update camera for smooth movement
        self.camera.update(delta_time);

        // Render without updating simulation state
        // Update camera
        self.camera.upload_to_gpu(queue);

        // Update species colors if needed
        queue.write_buffer(
            &self.species_colors_buffer,
            0,
            bytemuck::cast_slice(&self.species_colors),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Ecosystem Static Render Encoder"),
        });

        // Render in multiple passes for layered visualization (static)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ecosystem Static Multi-Layer Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Pass 1: Render background (chemical fields and environmental overlays)
            if self.settings.show_chemical_fields
                || self.settings.show_light_gradient
                || self.settings.show_temperature_zones
                || self.settings.show_ph_zones
            {
                render_pass.set_pipeline(&self.background_render_pipeline);
                render_pass.set_bind_group(0, &self.background_bind_group, &[]);
                render_pass.draw(0..6, 0..9); // 3x3 grid = 9 instances
            }

            // Pass 2: Render agents (microbes)
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_params_bind_group, &[]);
            render_pass.set_bind_group(2, &self.species_colors_bind_group, &[]);
            render_pass.set_bind_group(3, &self.agent_buffer_bind_group, &[]);
            // Draw agents as instanced quads with 3x3 grid
            let agent_instance_count = self.settings.agent_count * 9;
            render_pass.draw(0..6, 0..agent_instance_count);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn resize(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);
        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "agent_count" => {
                if let Some(val) = value.as_u64() {
                    self.settings.agent_count = val as u32;
                }
            }
            "agent_speed_min" => {
                if let Some(val) = value.as_f64() {
                    self.settings.agent_speed_min = val as f32;
                }
            }
            "agent_speed_max" => {
                if let Some(val) = value.as_f64() {
                    self.settings.agent_speed_max = val as f32;
                }
            }
            "agent_turn_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.agent_turn_rate = val as f32;
                }
            }
            "sensor_range" => {
                if let Some(val) = value.as_f64() {
                    self.settings.sensor_range = val as f32;
                }
            }
            "mutation_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.mutation_rate = val as f32;
                }
            }
            "energy_consumption_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.energy_consumption_rate = val as f32;
                }
            }
            "energy_gain_from_food" => {
                if let Some(val) = value.as_f64() {
                    self.settings.energy_gain_from_food = val as f32;
                }
            }
            // ecological_roles and variants_per_role are now fixed at 3 and 3 respectively
            // No longer configurable through settings
            "chemical_diffusion_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.chemical_diffusion_rate = val as f32;
                }
            }
            "chemical_decay_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.chemical_decay_rate = val as f32;
                }
            }
            "chemical_deposition_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.chemical_deposition_rate = val as f32;
                }
            }

            "brownian_motion_strength" => {
                if let Some(val) = value.as_f64() {
                    self.settings.brownian_motion_strength = val as f32;
                }
            }
            "wrap_edges" => {
                if let Some(val) = value.as_bool() {
                    self.settings.wrap_edges = val;
                }
            }
            "show_chemical_fields" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_chemical_fields = val;
                }
            }
            "show_energy_as_size" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_energy_as_size = val;
                }
            }

            "visual_mode" => {
                if let Some(val) = value.as_str() {
                    self.settings.set_visual_mode(val);
                }
            }
            // Visualization settings
            "show_oxygen" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_oxygen = val;
                }
            }
            "show_co2" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_co2 = val;
                }
            }
            "show_nitrogen" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_nitrogen = val;
                }
            }
            "show_pheromones" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_pheromones = val;
                }
            }
            "show_toxins" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_toxins = val;
                }
            }
            "show_attractants" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_attractants = val;
                }
            }
            "chemical_field_opacity" => {
                if let Some(val) = value.as_f64() {
                    self.settings.chemical_field_opacity = val as f32;
                }
            }
            "show_light_gradient" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_light_gradient = val;
                }
            }
            "show_temperature_zones" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_temperature_zones = val;
                }
            }
            "show_ph_zones" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_ph_zones = val;
                }
            }
            "environmental_opacity" => {
                if let Some(val) = value.as_f64() {
                    self.settings.environmental_opacity = val as f32;
                }
            }
            "predator_hunting_efficiency" => {
                if let Some(val) = value.as_f64() {
                    self.settings.predator_hunting_efficiency = val as f32;
                }
            }
            "max_particles" => {
                if let Some(val) = value.as_u64() {
                    let new_max_particles = val as u32;
                    let buffer_capacity =
                        self.biomass_buffer.size() as usize / std::mem::size_of::<DeadBiomass>();

                    if new_max_particles as usize > buffer_capacity {
                        tracing::info!(
                            "Recreating biomass buffer: old_capacity={}, new_max_particles={}",
                            buffer_capacity,
                            new_max_particles
                        );

                        // Recreate biomass buffer with new size
                        let new_buffer_size =
                            (new_max_particles as u64) * std::mem::size_of::<DeadBiomass>() as u64;
                        self.biomass_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("Biomass Buffer"),
                            size: new_buffer_size,
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });

                        // Recreate agent update pipeline with new biomass buffer
                        let (new_agent_update_pipeline, new_agent_update_bind_group) =
                            Self::create_agent_update_pipeline(
                                device,
                                &self.agent_buffer,
                                &self.biomass_buffer,
                                &self.chemical_field_buffer,
                                &self.sim_params_buffer,
                            )?;

                        self.agent_update_pipeline = new_agent_update_pipeline;
                        self.agent_update_bind_group = new_agent_update_bind_group;

                        tracing::info!(
                            "Biomass buffer and agent update pipeline recreated: new_size={} bytes, new_capacity={}",
                            new_buffer_size,
                            new_max_particles
                        );
                    }

                    self.settings.max_particles = new_max_particles;
                }
            }
            "particle_decomposition_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.particle_decomposition_rate = val as f32;
                }
            }
            "particle_decay_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.particle_decay_rate = val as f32;
                }
            }
            "matter_to_chemical_ratio" => {
                if let Some(val) = value.as_f64() {
                    self.settings.matter_to_chemical_ratio = val as f32;
                }
            }
            // These settings were removed in the new design
            _ => {}
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }

    fn get_state(&self) -> Value {
        // Calculate total energy across all agents
        let total_energy = self.settings.agent_count as f32 * 50.0; // Rough estimate since we can't read from GPU buffer

        // Estimate alive agents (all agents start alive after reset)
        let alive_agents = self.settings.agent_count;

        serde_json::json!({
            "time": self.time,
            "gui_visible": self.gui_visible,
            "camera": {
                "position": [self.camera.position[0], self.camera.position[1]],
                "zoom": self.camera.zoom
            },
            "agent_count": self.settings.agent_count,
            // ecological_roles and variants_per_role are fixed at 3 and 3 respectively
            "total_energy": total_energy,
            "alive_agents": alive_agents
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _mouse_button: u32,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Could implement food spawning or agent attraction here
        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Ecosystem doesn't currently support mouse interaction
        Ok(())
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
    }

    fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    fn reset_camera(&mut self) {
        self.camera.reset();
    }

    fn get_camera_state(&self) -> Value {
        serde_json::json!({
            "position": [self.camera.position[0], self.camera.position[1]],
            "zoom": self.camera.zoom
        })
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // TODO: Implement preset saving
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // TODO: Implement preset loading
        Ok(())
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            self.settings = new_settings;
            self.species_colors = Self::generate_species_colors(9); // Fixed at 3 roles * 3 variants = 9 species
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let mut rng = rand::rng();
        self.settings.random_seed = rng.random();

        // Reset time
        self.time = 0.0;

        // --- Ensure biomass buffer is correct size BEFORE any operations ---
        let required_biomass_size =
            (self.settings.max_particles as u64) * std::mem::size_of::<DeadBiomass>() as u64;
        if self.biomass_buffer.size() != required_biomass_size {
            tracing::info!(
                "Resizing biomass buffer: old_size={} new_size={} (max_particles={})",
                self.biomass_buffer.size(),
                required_biomass_size,
                self.settings.max_particles
            );
            self.biomass_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Biomass Buffer"),
                size: required_biomass_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            // Recreate agent update pipeline with new biomass buffer
            let (new_agent_update_pipeline, new_agent_update_bind_group) =
                Self::create_agent_update_pipeline(
                    device,
                    &self.agent_buffer,
                    &self.biomass_buffer,
                    &self.chemical_field_buffer,
                    &self.sim_params_buffer,
                )?;
            self.agent_update_pipeline = new_agent_update_pipeline;
            self.agent_update_bind_group = new_agent_update_bind_group;
            // Recreate biomass render pipeline and bind group
            let surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb, // Default format
                width: 1,
                height: 1,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 1,
            };
            let (new_biomass_render_pipeline, new_biomass_bind_group) =
                Self::create_biomass_render_pipeline(
                    device,
                    &surface_config,
                    &self.camera,
                    &self.render_params_buffer,
                    &self.biomass_buffer,
                )?;
            self.biomass_render_pipeline = new_biomass_render_pipeline;
            self.biomass_bind_group = new_biomass_bind_group;
        }
        // --- End biomass buffer resize logic ---

        // Regenerate agents with current settings
        let mut agents = Vec::with_capacity(self.settings.agent_count as usize);
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.settings.random_seed as u64);

        for i in 0..self.settings.agent_count {
            let ecological_role = i % 3; // 0: Recycler, 1: Producer, 2: Predator
            let variant = (i / 3) % 3; // Fixed at 3 variants per role

            // Ecological role-specific initialization following design document
            let (metabolism, chemical_secretion_rates) = match ecological_role {
                0 => {
                    // Recyclers - break down dead matter and waste into usable nutrients
                    let mut rates = [0.0; 6];
                    match variant {
                        0 => {
                            // Bacteria - small, fast-moving, swarm behavior around food sources
                            rates[0] = -0.4; // Consume oxygen for decomposition
                            rates[1] = 0.6; // Produce CO2 from organic matter breakdown
                            rates[2] = 0.9; // Produce nitrogen compounds from protein breakdown
                            rates[5] = 0.2; // Produce attractants to signal food sources
                            (1.0, rates) // Metabolism for bacteria
                        }
                        1 => {
                            // Fungi - slower movement, create visible thread networks
                            rates[0] = -0.2; // Consume oxygen for complex matter breakdown
                            rates[2] = 1.2; // Excellent at producing nitrogen compounds
                            rates[5] = 0.8; // Strong attractant production for network formation
                            (0.5, rates) // Metabolism for fungi
                        }
                        _ => {
                            // Decomposer Protozoans - larger, engulf debris particles whole
                            rates[0] = -0.6; // High oxygen consumption for large-scale decomposition
                            rates[1] = 0.4; // Moderate CO2 production
                            rates[2] = 0.8; // Good nitrogen production from engulfed matter
                            rates[5] = 0.1; // Low attractant production (more selective)
                            (0.7, rates) // Metabolism for decomposer protozoans
                        }
                    }
                }
                1 => {
                    // Producers - convert raw nutrients and light energy into biomass
                    let mut rates = [0.0; 6];
                    match variant {
                        0 => {
                            // Algae - form biofilm mats, highly light-dependent, create persistent structures
                            rates[0] = 1.0; // High oxygen production from photosynthesis
                            rates[1] = -0.6; // High CO2 consumption for biomass building
                            rates[5] = 0.4; // Moderate attractant production for mat formation
                            (0.4, rates) // Metabolism for algae
                        }
                        1 => {
                            // Cyanobacteria - mobile colonies, moderate light needs, can fix nitrogen
                            rates[0] = 0.7; // Good oxygen production
                            rates[1] = -0.4; // Moderate CO2 consumption
                            rates[2] = 0.3; // Nitrogen fixation from environment
                            rates[3] = 0.2; // Pheromone production for colony coordination
                            (0.6, rates) // Metabolism for cyanobacteria
                        }
                        _ => {
                            // Photosynthetic Protists - larger individual organisms, complex movement, efficient nutrient use
                            rates[0] = 0.8; // Efficient oxygen production
                            rates[1] = -0.7; // High CO2 consumption for efficient photosynthesis
                            rates[2] = 0.1; // Efficient nitrogen use (low production)
                            (0.3, rates) // Metabolism for photosynthetic protists
                        }
                    }
                }
                _ => {
                    // Predators - control population dynamics through consumption
                    let mut rates = [0.0; 6];
                    match variant {
                        0 => {
                            // Predatory Bacteria - small, fast, hunt in coordinated groups
                            rates[0] = -0.3; // Consume oxygen for rapid movement
                            rates[1] = 0.4; // Produce CO2 from respiration
                            rates[3] = 0.6; // High pheromone production for pack coordination
                            rates[4] = 0.1; // Low toxin production for prey stunning
                            (1.1, rates) // Metabolism for predatory bacteria
                        }
                        1 => {
                            // Viruses - inject into hosts, replicate internally, burst out after delay
                            rates[4] = 0.3; // Produce toxins for host injection
                            rates[3] = 0.2; // Pheromone production for target recognition
                            (1.5, rates) // Metabolism for viruses
                        }
                        2 => {
                            // Predatory Protozoans - large, engulf smaller organisms, slow but powerful
                            rates[0] = -0.5; // High oxygen consumption for large body processes
                            rates[1] = 0.5; // CO2 production from consumed organisms
                            rates[4] = 0.2; // Toxin production for prey immobilization
                            (0.5, rates) // Metabolism for predatory protozoans
                        }
                        _ => {
                            // Parasitic Microbes - attach to hosts, drain resources over time
                            rates[0] = -0.1; // Low oxygen consumption (parasitic lifestyle)
                            rates[4] = 0.4; // Toxin production for host weakening
                            rates[3] = 0.3; // Pheromone production for host tracking
                            (0.8, rates) // Metabolism for parasitic microbes
                        }
                    }
                }
            };

            agents.push(Agent {
                position: [rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)],
                velocity: [rng.random_range(-0.01..0.01), rng.random_range(-0.01..0.01)],
                energy: 50.0,
                age: 0.0,
                ecological_role,
                variant,
                sensor_readings: [0.0; 4],
                heading: rng.random_range(0.0..std::f32::consts::TAU),
                run_duration: rng
                    .random_range(self.settings.run_duration_min..self.settings.run_duration_max),
                run_timer: 0.0,
                tumble_cooldown: 0.0,
                metabolism_rate: metabolism,
                reproductive_threshold: 70.0,
                last_reproduction_time: 0.0,
                behavioral_state: 0, // Start in feeding state
                state_timer: 0.0,
                chemical_secretion_rates,
                food_memory: [0.0; 4],
                threat_memory: [0.0; 4],
                biofilm_strength: 0.0,
                biofilm_connections: 0,
                hunt_target_id: 0,
                pack_coordination: 0.0,
                territory_center: [0.0, 0.0],
                territory_radius: 0.0,
                is_visible: 1, // Start visible
                _pad: [0],
            });
        }

        // Update agent buffer on GPU
        queue.write_buffer(&self.agent_buffer, 0, bytemuck::cast_slice(&agents));

        // Update simulation parameters with new random seed
        let sim_params =
            SimParams::from_settings(&self.settings, 2.0, 2.0, self.settings.agent_count);
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        // Clear chemical field buffer
        let chemical_buffer_size = (self.settings.chemical_resolution
            * self.settings.chemical_resolution
            * self.settings.chemical_types) as usize;
        let clear_data = vec![0.0f32; chemical_buffer_size];
        let clear_bytes = bytemuck::cast_slice(&clear_data);
        queue.write_buffer(&self.chemical_field_buffer, 0, clear_bytes);
        queue.write_buffer(&self.chemical_field_temp_buffer, 0, clear_bytes);

        // Clear biomass buffer - buffer is already the correct size
        let clear_biomass_data = vec![
            DeadBiomass {
                position: [0.0, 0.0],
                biomass_amount: 0.0,
                species_origin: 0,
                decay_time: 0.0,
                decomposition_progress: 0.0,
                is_active: 0,
                _pad: [0],
            };
            self.settings.max_particles as usize
        ];
        queue.write_buffer(
            &self.biomass_buffer,
            0,
            bytemuck::cast_slice(&clear_biomass_data),
        );

        // Reset visibility flags to all visible
        let total_species = 9; // 3 roles * 3 variants = 9 species
        self.visibility_flags = vec![1u32; total_species as usize];
        queue.write_buffer(
            &self.visibility_buffer,
            0,
            bytemuck::cast_slice(&self.visibility_flags),
        );

        // Ensure GPU operations complete
        device.poll(wgpu::Maintain::Wait);

        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.gui_visible
    }

    fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.settings.randomize();
        self.species_colors = Self::generate_species_colors(9); // Fixed at 3 roles * 3 variants = 9 species
        Ok(())
    }
}

impl EcosystemModel {
    /// Toggle visibility for a specific species variant
    pub fn toggle_species_visibility(
        &mut self,
        ecological_role: u32,
        variant: u32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let species_index = (ecological_role * 3 + variant) as usize; // Fixed at 3 variants per role
        let total_species = 9; // Fixed at 3 roles * 3 variants = 9 species

        if species_index >= total_species {
            return Err(SimulationError::InvalidParameter(format!(
                "Species index {} out of range (max: {})",
                species_index,
                total_species - 1
            )));
        }

        // Toggle the visibility flag in our local state
        self.visibility_flags[species_index] = if self.visibility_flags[species_index] == 1 {
            0
        } else {
            1
        };

        // Update GPU buffer with the current state
        queue.write_buffer(
            &self.visibility_buffer,
            0,
            bytemuck::cast_slice(&self.visibility_flags),
        );

        tracing::info!(
            "Toggled visibility for species role={}, variant={} to {}",
            ecological_role,
            variant,
            self.visibility_flags[species_index]
        );

        Ok(())
    }
}
