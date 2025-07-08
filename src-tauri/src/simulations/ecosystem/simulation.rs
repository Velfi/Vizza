use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::Settings;
use crate::simulations::shared::{camera::Camera, LutManager};
use crate::simulations::traits::Simulation;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Agent {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: f32,
    pub age: f32,
    pub species: u32,
    pub _pad1: u32,
    pub neural_weights: [f32; 8],
    pub sensor_readings: [f32; 4],
    pub goal: u32,
    pub _pad2: u32,
    pub memory: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Food {
    pub position: [f32; 2],
    pub energy: f32,
    pub is_active: u32,
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

    // Chemical parameters
    pub chemical_resolution: u32,
    pub chemical_types: u32,
    pub chemical_diffusion_rate: f32,
    pub chemical_decay_rate: f32,
    pub chemical_deposition_rate: f32,

    // Learning parameters
    pub learning_rate: f32,
    pub mutation_rate: f32,
    pub energy_consumption_rate: f32,
    pub energy_gain_from_food: f32,
    pub reproduction_energy_threshold: f32,
    pub reproduction_probability: f32,

    // Species parameters
    pub species_count: u32,
    pub intra_species_attraction: f32,
    pub inter_species_repulsion: f32,

    // Environmental parameters
    pub brownian_motion_strength: f32,
    pub food_spawn_rate: f32,
    pub max_food_particles: u32,
    pub wrap_edges: u32,

    pub random_seed: u32,
    pub time: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ChemicalDiffusionParams {
    pub chemical_resolution: u32,
    pub chemical_types: u32,
    pub chemical_diffusion_rate: f32,
    pub chemical_decay_rate: f32,
    pub dt: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RenderParams {
    pub show_energy_as_size: u32,
    pub show_sensors: u32,
    pub trail_opacity: f32,
    pub time: f32,
}

/// Ecosystem simulation model
#[derive(Debug)]
pub struct EcosystemModel {
    // GPU resources
    pub agent_buffer: wgpu::Buffer,
    pub food_buffer: wgpu::Buffer,
    pub chemical_field_buffer: wgpu::Buffer,
    pub chemical_field_temp_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub chemical_diffusion_params_buffer: wgpu::Buffer,
    pub render_params_buffer: wgpu::Buffer,
    pub species_colors_buffer: wgpu::Buffer,

    // Compute pipelines
    pub agent_update_pipeline: wgpu::ComputePipeline,
    pub agent_update_bind_group: wgpu::BindGroup,
    pub chemical_diffusion_pipeline: wgpu::ComputePipeline,
    pub chemical_diffusion_bind_group: wgpu::BindGroup,

    // Render pipeline
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

    // LUT management
    pub lut_manager: Arc<LutManager>,
}

// Implementation methods are included inline

impl SimParams {
    pub fn from_settings(settings: &Settings, _width: f32, _height: f32, agent_count: u32) -> Self {
        Self {
            agent_count,
            width: 2.0,  // Normalized coordinate system width [-1, 1]
            height: 2.0, // Normalized coordinate system height [-1, 1]
            dt: settings.time_step,
            agent_speed_min: settings.agent_speed_min,
            agent_speed_max: settings.agent_speed_max,
            agent_turn_rate: settings.agent_turn_rate,
            sensor_range: settings.sensor_range,
            sensor_count: settings.sensor_count,
            sensor_angle: settings.sensor_angle,
            chemical_resolution: settings.chemical_resolution,
            chemical_types: settings.chemical_types,
            chemical_diffusion_rate: settings.chemical_diffusion_rate,
            chemical_decay_rate: settings.chemical_decay_rate,
            chemical_deposition_rate: settings.chemical_deposition_rate,
            learning_rate: settings.learning_rate,
            mutation_rate: settings.mutation_rate,
            energy_consumption_rate: settings.energy_consumption_rate,
            energy_gain_from_food: settings.energy_gain_from_food,
            reproduction_energy_threshold: settings.reproduction_energy_threshold,
            reproduction_probability: settings.reproduction_probability,
            species_count: settings.species_count,
            intra_species_attraction: settings.intra_species_attraction,
            inter_species_repulsion: settings.inter_species_repulsion,
            brownian_motion_strength: settings.brownian_motion_strength,
            food_spawn_rate: settings.food_spawn_rate,
            max_food_particles: settings.max_food_particles,
            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            random_seed: settings.random_seed,
            time: 0.0,
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
        }
    }
}

impl RenderParams {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            show_energy_as_size: if settings.show_energy_as_size { 1 } else { 0 },
            show_sensors: if settings.show_sensors { 1 } else { 0 },
            trail_opacity: settings.trail_opacity,
            time: 0.0,
        }
    }
}

impl EcosystemModel {
    pub fn new(
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        agent_count: u32,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> Result<Self, SimulationError> {
        let width = surface_config.width as f32;
        let height = surface_config.height as f32;

        // Initialize camera
        let camera = Camera::new(device, width, height)?;

        // Generate species colors
        let species_colors = Self::generate_species_colors(settings.species_count);

        // Create buffers - simplified for now
        // Initialize agents with random positions and properties
        // Use normalized coordinate system [-1, 1] to match other simulations
        let mut agents = Vec::with_capacity(agent_count as usize);
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(settings.random_seed as u64);

        for i in 0..agent_count {
            let species = (i as u32) % settings.species_count;
            agents.push(Agent {
                position: [rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)],
                velocity: [rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)],
                energy: 50.0,
                age: 0.0,
                species,
                _pad1: 0,
                neural_weights: [0.0; 8],
                sensor_readings: [0.0; 4],
                goal: 0,
                _pad2: 0,
                memory: [0.0; 4],
            });
        }

        let agent_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Agent Buffer"),
            contents: bytemuck::cast_slice(&agents),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
        });

        let food_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Food Buffer"),
            size: (settings.max_food_particles as u64) * std::mem::size_of::<Food>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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

        // Create proper compute pipelines
        let (agent_update_pipeline, agent_update_bind_group) = Self::create_agent_update_pipeline(
            device,
            &agent_buffer,
            &food_buffer,
            &chemical_field_buffer,
            &sim_params_buffer,
        )?;
        let (chemical_diffusion_pipeline, chemical_diffusion_bind_group) =
            Self::create_chemical_diffusion_pipeline(
                device,
                &chemical_field_buffer,
                &chemical_field_temp_buffer,
                &chemical_diffusion_params_buffer,
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
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: agent_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            agent_buffer,
            food_buffer,
            chemical_field_buffer,
            chemical_field_temp_buffer,
            sim_params_buffer,
            chemical_diffusion_params_buffer,
            render_params_buffer,
            species_colors_buffer,
            agent_update_pipeline,
            agent_update_bind_group,
            chemical_diffusion_pipeline,
            chemical_diffusion_bind_group,
            render_pipeline,
            camera_bind_group,
            render_params_bind_group,
            species_colors_bind_group,
            agent_buffer_bind_group,
            camera: camera,
            settings,
            gui_visible: true,
            time: 0.0,
            species_colors,
            lut_manager: Arc::new(lut_manager.clone()),
        })
    }

    fn generate_species_colors(species_count: u32) -> Vec<[f32; 4]> {
        let mut colors = Vec::new();
        for i in 0..species_count {
            let hue = (i as f32 / species_count as f32) * 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);
            colors.push([r, g, b, 1.0]);
        }
        colors
    }

    fn recreate_species_colors_buffer(&mut self, device: &Arc<Device>) {
        // Create new buffer with updated species colors
        self.species_colors_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Species Colors Buffer"),
            contents: bytemuck::cast_slice(&self.species_colors),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate the bind group with the new buffer
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

        self.species_colors_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Species Colors Bind Group"),
            layout: &species_colors_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.species_colors_buffer.as_entire_binding(),
            }],
        });
    }

    fn create_agent_update_pipeline(
        device: &Arc<Device>,
        agent_buffer: &wgpu::Buffer,
        food_buffer: &wgpu::Buffer,
        chemical_field_buffer: &wgpu::Buffer,
        sim_params_buffer: &wgpu::Buffer,
    ) -> Result<(wgpu::ComputePipeline, wgpu::BindGroup), SimulationError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Agent Update Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/agent_update.wgsl").into()),
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
                // Food buffer
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
                    resource: food_buffer.as_entire_binding(),
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
    ) -> Result<(wgpu::ComputePipeline, wgpu::BindGroup), SimulationError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chemical Diffusion Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/chemical_diffusion.wgsl").into(),
            ),
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
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
        // Update time
        self.time += self.settings.time_step;

        // Get surface dimensions from camera
        let width = self.camera.viewport_width;
        let height = self.camera.viewport_height;

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
            show_sensors: if self.settings.show_sensors { 1 } else { 0 },
            trail_opacity: self.settings.trail_opacity,
            time: self.time,
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
        for _ in 0..simulation_steps {
            // Update agents
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Agent Update Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.agent_update_pipeline);
                compute_pass.set_bind_group(0, &self.agent_update_bind_group, &[]);

                let workgroup_size = 64;
                let num_workgroups = self.settings.agent_count.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
            }

            // Update chemical diffusion
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

        // Render agents
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ecosystem Render Pass"),
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_params_bind_group, &[]);
            render_pass.set_bind_group(2, &self.species_colors_bind_group, &[]);
            render_pass.set_bind_group(3, &self.agent_buffer_bind_group, &[]);

            // Draw agents as instanced quads
            let instance_count = self.settings.agent_count;
            render_pass.draw(0..6, 0..instance_count);
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

        // Render agents
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ecosystem Static Render Pass"),
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_params_bind_group, &[]);
            render_pass.set_bind_group(2, &self.species_colors_bind_group, &[]);
            render_pass.set_bind_group(3, &self.agent_buffer_bind_group, &[]);

            // Draw agents as instanced quads
            let instance_count = self.settings.agent_count;
            render_pass.draw(0..6, 0..instance_count);
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
            "learning_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.learning_rate = val as f32;
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
            "species_count" => {
                if let Some(val) = value.as_u64() {
                    self.settings.species_count = val as u32;
                    self.species_colors =
                        Self::generate_species_colors(self.settings.species_count);
                    // Recreate the species colors buffer with the new size
                    self.recreate_species_colors_buffer(device);
                }
            }
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
            "food_spawn_rate" => {
                if let Some(val) = value.as_f64() {
                    self.settings.food_spawn_rate = val as f32;
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
            "show_chemical_trails" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_chemical_trails = val;
                }
            }
            "show_energy_as_size" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_energy_as_size = val;
                }
            }
            "show_sensors" => {
                if let Some(val) = value.as_bool() {
                    self.settings.show_sensors = val;
                }
            }
            "trail_opacity" => {
                if let Some(val) = value.as_f64() {
                    self.settings.trail_opacity = val as f32;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }

    fn get_state(&self) -> Value {
        serde_json::json!({
            "time": self.time,
            "gui_visible": self.gui_visible,
            "camera": {
                "position": [self.camera.position[0], self.camera.position[1]],
                "zoom": self.camera.zoom
            }
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Could implement food spawning or agent attraction here
        println!("Mouse interaction at ({}, {})", world_x, world_y);
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
            self.species_colors = Self::generate_species_colors(self.settings.species_count);
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.time = 0.0;
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
        self.species_colors = Self::generate_species_colors(self.settings.species_count);
        Ok(())
    }
}

// Helper function to convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r_prime + m, g_prime + m, b_prime + m)
}
