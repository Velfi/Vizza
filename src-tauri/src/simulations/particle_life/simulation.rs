use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::{MatrixGenerator, PositionGenerator, Settings, TypeGenerator};
use super::shaders;
use crate::simulations::shared::{camera::Camera, LutManager};
use crate::simulations::traits::Simulation;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Particle {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub species: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ForceUpdateParams {
    pub species_a: u32,
    pub species_b: u32,
    pub new_force: f32,
    pub species_count: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct InitParams {
    pub start_index: u32,
    pub spawn_count: u32,
    pub species_count: u32,
    pub width: f32,
    pub height: f32,
    pub random_seed: u32,
    pub position_generator: u32, // 0=Random, 1=Center, 2=UniformCircle, etc.
    pub type_generator: u32,     // 0=Random, 1=Randomize10Percent, etc.
    pub _pad1: u32,
    pub _pad2: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ForceRandomizeParams {
    pub species_count: u32,
    pub random_seed: u32,
    pub min_force: f32,
    pub max_force: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimParams {
    pub particle_count: u32,
    pub species_count: u32,
    pub max_force: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub friction: f32,
    pub wrap_edges: u32,
    pub width: f32,
    pub height: f32,
    pub random_seed: u32,
    pub dt: f32, // Time step for simulation
    pub beta: f32, // Transition point between repulsion and attraction zones
    pub cursor_x: f32, // Cursor position in world coordinates
    pub cursor_y: f32,
    pub cursor_size: f32, // Cursor interaction radius
    pub cursor_strength: f32, // Cursor force strength
    pub cursor_active: u32, // Whether cursor interaction is active (0 = inactive, 1 = attract, 2 = repel)
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
    _pad5: u32,
    _pad6: u32,
    _pad7: u32, // Added to make struct 88 bytes (22 * 4)
}

impl SimParams {
    pub fn new(
        width: u32,
        height: u32,
        particle_count: u32,
        settings: &Settings,
        state: &State,
    ) -> Self {
        Self {
            particle_count,
            species_count: settings.species_count,
            max_force: settings.max_force,
            min_distance: settings.min_distance,
            max_distance: settings.max_distance,
            friction: settings.friction,
            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            width: width as f32,
            height: height as f32,
            random_seed: state.random_seed,
            dt: state.dt,
            beta: settings.force_beta,
            cursor_x: 0.0, // Initialize cursor position to center
            cursor_y: 0.0,
            cursor_size: state.cursor_size,
            cursor_strength: state.cursor_strength,
            cursor_active: 0, // Start with cursor interaction inactive
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
            _pad5: 0,
            _pad6: 0,
            _pad7: 0,
        }
    }
}

/// Particle Life simulation state (runtime data, not saved in presets)
#[derive(Debug)]
pub struct State {
    pub particle_count: usize,
    pub particles: Vec<Particle>,
    pub random_seed: u32,
    pub dt: f32,
    pub cursor_size: f32,
    pub cursor_strength: f32,
    pub traces_enabled: bool,
    pub trace_fade: f32,
    pub edge_fade_strength: f32,
    pub position_generator: PositionGenerator,
    pub type_generator: TypeGenerator,
    pub matrix_generator: MatrixGenerator,
}

impl State {
    pub fn new(
        particle_count: usize,
        species_count: u32,
        width: u32,
        height: u32,
        random_seed: u32,
    ) -> Self {
        let mut particles = Vec::with_capacity(particle_count);

        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(random_seed as u64);

        // Distribute particles evenly among species
        for i in 0..particle_count {
            let species = (i as u32) % species_count;

            particles.push(Particle {
                position: [
                    rng.random_range(0.0..width as f32),
                    rng.random_range(0.0..height as f32),
                ],
                velocity: [0.0, 0.0], // Start with no velocity
                species,
                _pad: 0,
            });
        }

        Self {
            particle_count,
            particles,
            random_seed,
            dt: 0.016,
            cursor_size: 0.5,
            cursor_strength: 1.0,
            traces_enabled: false,
            trace_fade: 0.95,
            edge_fade_strength: 1.0,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
        }
    }

    pub fn reset(&mut self, _species_count: u32, _width: u32, _height: u32, _random_seed: u32) {
        // No longer used - particles are initialized on GPU
    }
}

// Add color mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Gray18,
    White,
    Black,
    Lut,
}

/// Particle Life simulation model
#[derive(Debug)]
pub struct ParticleLifeModel {
    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub force_matrix_buffer: wgpu::Buffer,
    pub lut_buffer: Arc<wgpu::Buffer>,
    pub lut_size_buffer: wgpu::Buffer,

    // Compute pipeline
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,

    // Initialization pipeline
    pub init_pipeline: wgpu::ComputePipeline,
    pub init_bind_group: wgpu::BindGroup,
    pub init_bind_group_layout: wgpu::BindGroupLayout,
    pub init_params_buffer: wgpu::Buffer,

    // Force matrix update pipelines
    pub force_update_pipeline: wgpu::ComputePipeline,
    pub force_update_params_buffer: wgpu::Buffer,
    pub force_update_bind_group: wgpu::BindGroup,

    // Force matrix randomization pipeline
    pub force_randomize_pipeline: wgpu::ComputePipeline,
    pub force_randomize_params_buffer: wgpu::Buffer,
    pub force_randomize_bind_group: wgpu::BindGroup,

    // Render pipeline
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_particles_bind_group_layout: wgpu::BindGroupLayout,
    pub render_bind_group: wgpu::BindGroup,
    pub lut_bind_group: wgpu::BindGroup,
    pub camera_bind_group: wgpu::BindGroup,

    // Simulation state and settings
    pub settings: Settings,
    pub state: State,
    pub show_gui: bool,

    // LUT management
    pub current_lut_name: String,
    pub lut_reversed: bool,
    pub lut_manager: Arc<LutManager>, // Store reference to LUT manager

    // Dimensions
    pub width: u32,
    pub height: u32,

    // Camera for viewport control
    pub camera: Camera,

    // Frame timing for smooth camera movement
    last_frame_time: std::time::Instant,

    // Add color mode field
    pub color_mode: ColorMode,
    pub lut_colors: Vec<[f32; 4]>, // Store LUT colors for current mode
    
    // Cursor interaction state
    pub cursor_active_mode: u32, // 0=inactive, 1=attract, 2=repel
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,
}

impl ParticleLifeModel {
    /// Calculate type distribution based on particle count and species count
    fn calculate_type_distribution(&self) -> Vec<u32> {
        let total_particles = self.state.particle_count as u32;
        let species_count = self.settings.species_count;

        // Particles are distributed evenly among species
        let base_count = total_particles / species_count;
        let remainder = total_particles % species_count;

        let mut type_counts = vec![base_count; species_count as usize];

        // Distribute remainder particles among the first few species
        for i in 0..remainder as usize {
            type_counts[i] += 1;
        }

        type_counts
    }

    /// Flatten 2D force matrix to 1D array for GPU
    pub fn flatten_force_matrix(force_matrix: &[Vec<f32>]) -> Vec<f32> {
        let mut flattened = Vec::new();

        for row in force_matrix {
            for &force in row {
                flattened.push(force);
            }
        }

        flattened
    }

    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        _adapter_info: &wgpu::AdapterInfo,
        particle_count: usize,
        settings: Settings,
        lut_manager: &LutManager,
        color_mode: ColorMode, // Add color mode param
    ) -> SimulationResult<Self> {
        let width = surface_config.width;
        let height = surface_config.height;

        // Create initial state (but don't initialize particles on CPU)
        let state = State {
            particle_count,
            particles: vec![], // Empty - will be initialized on GPU
            random_seed: 0,
            dt: 0.016,
            cursor_size: 0.5,
            cursor_strength: 1.0,
            traces_enabled: false,
            trace_fade: 0.95,
            edge_fade_strength: 1.0,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
        };

        // Check buffer size limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let particle_buffer_size = (particle_count * std::mem::size_of::<Particle>()) as u64;

        if particle_buffer_size > max_storage_buffer_size {
            return Err(SimulationError::BufferTooLarge {
                requested: particle_buffer_size,
                max_available: max_storage_buffer_size,
            });
        }

        // Create empty particle buffer (will be initialized on GPU)
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: particle_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create simulation parameters buffer
        let sim_params = SimParams::new(width, height, particle_count as u32, &settings, &state);
        let sim_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sim Params Buffer"),
            contents: bytemuck::cast_slice(&[sim_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create force matrix buffer (flatten 2D matrix to 1D array)
        let force_matrix_data = Self::flatten_force_matrix(&settings.force_matrix);
        let force_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let lut = lut_manager.get("MATPLOTLIB_nipy_spectral")?;

        // Create LUT buffer
        let (lut_colors, current_lut_name) = if color_mode == ColorMode::Lut {
            // <num_species> + 1 points for LUT mode
            let colors = lut
                .get_colors(settings.species_count as usize + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();
            tracing::trace!(
                "Constructor LUT mode: got {} colors for {} species",
                colors.len(),
                settings.species_count
            );
            (colors, lut.name.clone())
        } else {
            let colors = lut
                .get_colors(settings.species_count as usize)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();
            tracing::trace!(
                "Constructor non-LUT mode: got {} colors for {} species",
                colors.len(),
                settings.species_count
            );
            (colors, lut.name.clone())
        };

        let lut_data_u32 = lut_colors
            .iter()
            .flat_map(|&[r, g, b, a]| [r, g, b, a])
            .collect::<Vec<_>>();
        let lut_buffer = Arc::new(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LUT Buffer"),
                contents: bytemuck::cast_slice(&lut_data_u32),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
        );

        // Create compute shader and pipeline
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::COMPUTE_SHADER.into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Particle Life Compute Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Life Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Life Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: force_matrix_buffer.as_entire_binding(),
                },
            ],
        });

        // Create initialization compute shader and pipeline
        let init_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Init Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::INIT_SHADER.into()),
        });

        let init_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Particle Life Init Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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

        let init_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Life Init Pipeline Layout"),
            bind_group_layouts: &[&init_bind_group_layout],
            push_constant_ranges: &[],
        });

        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Life Init Pipeline"),
            layout: Some(&init_pipeline_layout),
            module: &init_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create init params buffer
        let init_params = InitParams {
            start_index: 0,
            spawn_count: particle_count as u32,
            species_count: settings.species_count,
            width: width as f32,
            height: height as f32,
            random_seed: state.random_seed,
            position_generator: state.position_generator as u32,
            type_generator: state.type_generator as u32,
            _pad1: 0,
            _pad2: 0,
        };

        let init_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Init Params Buffer"),
            contents: bytemuck::cast_slice(&[init_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let init_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Init Bind Group"),
            layout: &init_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: init_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create force update compute shader and pipeline
        let force_update_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Force Update Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FORCE_UPDATE_SHADER.into()),
        });

        let force_update_params = ForceUpdateParams {
            species_a: 0,
            species_b: 0,
            new_force: 0.0,
            species_count: settings.species_count,
        };
        let force_update_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Force Update Params Buffer"),
                contents: bytemuck::cast_slice(&[force_update_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let force_update_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Force Update Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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

        let force_update_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Force Update Pipeline Layout"),
                bind_group_layouts: &[&force_update_bind_group_layout],
                push_constant_ranges: &[],
            });

        let force_update_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Force Update Pipeline"),
                layout: Some(&force_update_pipeline_layout),
                module: &force_update_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        let force_update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Force Update Bind Group"),
            layout: &force_update_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: force_matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: force_update_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create force randomization compute shader and pipeline
        let force_randomize_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Force Randomize Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FORCE_RANDOMIZE_SHADER.into()),
        });

        let force_randomize_params = ForceRandomizeParams {
            species_count: settings.species_count,
            random_seed: state.random_seed,
            min_force: -1.0,
            max_force: 1.0,
        };
        let force_randomize_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Force Randomize Params Buffer"),
                contents: bytemuck::cast_slice(&[force_randomize_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let force_randomize_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Force Randomize Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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

        let force_randomize_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Force Randomize Pipeline Layout"),
                bind_group_layouts: &[&force_randomize_bind_group_layout],
                push_constant_ranges: &[],
            });

        let force_randomize_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Force Randomize Pipeline"),
                layout: Some(&force_randomize_pipeline_layout),
                module: &force_randomize_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        let force_randomize_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Force Randomize Bind Group"),
            layout: &force_randomize_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: force_matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: force_randomize_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render shaders and pipeline
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FRAGMENT_SHADER.into()),
        });

        // Render bind group layout (particles + sim params)
        let render_bind_group_layout_particles =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout (Particles)"),
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
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // LUT bind group layout
        let lut_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("LUT Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let render_bind_group_layout = lut_bind_group_layout.clone();

        // Camera bind group layout
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Life Render Pipeline Layout"),
                bind_group_layouts: &[
                    &render_bind_group_layout_particles,
                    &render_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Life Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: Some("main"),
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create bind groups
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout_particles,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create a simple LUT texture for now (we'll implement proper LUT support later)
        let lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("LUT Texture"),
            size: wgpu::Extent3d {
                width: lut_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload LUT colors to texture
        let lut_data: Vec<u8> = lut_colors
            .iter()
            .flat_map(|&[r, g, b, a]| {
                [
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (a * 255.0) as u8,
                ]
            })
            .collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &lut_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &lut_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(lut_colors.len() as u32 * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: lut_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let lut_view = lut_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let lut_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("LUT Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create LUT size uniform buffer
        let lut_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Size Buffer"),
            contents: bytemuck::cast_slice(&[lut_colors.len() as u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LUT Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&lut_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lut_size_buffer.as_entire_binding(),
                },
            ],
        });

        // Create camera
        let camera = Camera::new(device, width as f32, height as f32)?;

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
        });

        let result = Self {
            particle_buffer,
            sim_params_buffer,
            force_matrix_buffer,
            lut_buffer,
            lut_size_buffer,
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            init_pipeline,
            init_bind_group,
            init_bind_group_layout,
            init_params_buffer,
            force_update_pipeline,
            force_update_params_buffer,
            force_update_bind_group,
            force_randomize_pipeline,
            force_randomize_params_buffer,
            force_randomize_bind_group,
            render_pipeline,
            render_bind_group_layout,
            render_particles_bind_group_layout: render_bind_group_layout_particles,
            render_bind_group,
            lut_bind_group,
            camera_bind_group,
            settings,
            state,
            show_gui: false,
            current_lut_name,
            lut_reversed: false,
            lut_manager: Arc::new(lut_manager.clone()),
            width,
            height,
            camera,
            last_frame_time: std::time::Instant::now(),
            color_mode,
            lut_colors,
            cursor_active_mode: 0,
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
        };

        // Initialize particles on GPU
        result.initialize_particles_gpu(device, queue)?;

        Ok(result)
    }

    fn update_sim_params(&mut self, _device: &Arc<Device>, queue: &Arc<Queue>) {
        let mut sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );
        
        // Override with stored cursor values if cursor is active
        sim_params.cursor_x = self.cursor_world_x;
        sim_params.cursor_y = self.cursor_world_y;
        sim_params.cursor_active = self.cursor_active_mode;
        if self.cursor_active_mode > 0 {
            sim_params.cursor_strength = self.state.cursor_strength * self.settings.max_force * 10.0;
        }
        
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
    }

    fn update_sim_params_preserve_cursor(&mut self, _device: &Arc<Device>, queue: &Arc<Queue>, preserve_cursor: bool) {
        if preserve_cursor {
            // Don't update if we want to preserve cursor settings
            return;
        }
        
        let sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
    }

    fn initialize_particles_gpu(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update init params with current particle count
        let init_params = InitParams {
            start_index: 0,
            spawn_count: self.state.particle_count as u32,
            species_count: self.settings.species_count,
            width: self.width as f32,
            height: self.height as f32,
            random_seed: self.state.random_seed,
            position_generator: self.state.position_generator as u32,
            type_generator: self.state.type_generator as u32,
            _pad1: 0,
            _pad2: 0,
        };

        queue.write_buffer(
            &self.init_params_buffer,
            0,
            bytemuck::cast_slice(&[init_params]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Init Encoder"),
        });

        {
            let mut init_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Particle Life Init Pass"),
                timestamp_writes: None,
            });

            init_pass.set_pipeline(&self.init_pipeline);
            init_pass.set_bind_group(0, &self.init_bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups = (self.state.particle_count + workgroup_size - 1) / workgroup_size;
            init_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn reset_particles_gpu(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        tracing::info!(
            "Resetting particles with count: {}",
            self.state.particle_count
        );

        // Update random seed for reset
        use rand::Rng;
        let mut rng = rand::rng();
        self.state.random_seed = rng.random();

        // Update sim params with new random seed and current particle count
        self.update_sim_params(device, queue);

        tracing::info!(
            "Reinitializing {} particles on GPU",
            self.state.particle_count
        );
        // Re-initialize particles on GPU
        self.initialize_particles_gpu(device, queue)?;

        // Ensure GPU operations complete
        device.poll(wgpu::Maintain::Wait);

        tracing::info!("Particle reset complete");
        Ok(())
    }

    pub fn update_force_element_gpu(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        species_a: u32,
        species_b: u32,
        new_force: f32,
    ) -> SimulationResult<()> {
        let update_params = ForceUpdateParams {
            species_a,
            species_b,
            new_force,
            species_count: self.settings.species_count,
        };

        // Update the uniform buffer
        queue.write_buffer(
            &self.force_update_params_buffer,
            0,
            bytemuck::cast_slice(&[update_params]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Force Update Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Update Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.force_update_pipeline);
            compute_pass.set_bind_group(0, &self.force_update_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn randomize_force_matrix_gpu(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update random seed
        use rand::Rng;
        let mut rng = rand::rng();
        let new_seed = rng.random();

        let randomize_params = ForceRandomizeParams {
            species_count: self.settings.species_count,
            random_seed: new_seed,
            min_force: -1.0,
            max_force: 1.0,
        };

        // Update the uniform buffer
        queue.write_buffer(
            &self.force_randomize_params_buffer,
            0,
            bytemuck::cast_slice(&[randomize_params]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Force Randomize Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Randomize Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.force_randomize_pipeline);
            compute_pass.set_bind_group(0, &self.force_randomize_bind_group, &[]);

            // Dispatch with enough workgroups to cover the species matrix
            let workgroup_size = 8;
            let num_workgroups =
                (self.settings.species_count + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups as u32, num_workgroups as u32, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn recreate_bind_groups_with_force_matrix(&mut self, device: &Arc<Device>) {
        // Recreate compute bind group with new force matrix
        self.compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Compute Bind Group"),
            layout: &self.compute_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.force_matrix_buffer.as_entire_binding(),
                },
            ],
        });
    }

    // Note: generate_force_matrix method removed - now using settings.randomize_force_matrix()

    /// Update the LUT with new settings
    pub fn update_lut(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        lut_manager: &LutManager,
        color_mode: ColorMode,
        lut_name: Option<&str>,
        lut_reversed: bool,
    ) -> SimulationResult<()> {
        // Update color mode
        self.color_mode = color_mode;

        // Get new LUT colors
        let (new_lut_colors, new_lut_name, new_lut_reversed) = if color_mode == ColorMode::Lut {
            let lut_name = lut_name.unwrap_or(&self.current_lut_name);
            let mut lut = lut_manager
                .get(lut_name)
                .unwrap_or_else(|_| lut_manager.get_default());
            if lut_reversed {
                lut = lut.reversed();
            }
            // <num_species> + 1 points for LUT mode
            let colors = lut
                .get_colors(self.settings.species_count as usize + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();
            tracing::debug!(
                "LUT mode: got {} colors for {} species",
                colors.len(),
                self.settings.species_count
            );
            (colors, lut.name.clone(), lut_reversed)
        } else {
            // <num_species> points for non-LUT mode
            let lut_name = lut_name.unwrap_or(&self.current_lut_name);
            let lut = lut_manager
                .get(lut_name)
                .unwrap_or_else(|_| lut_manager.get_default());
            let lut = if lut_reversed { lut.reversed() } else { lut };
            let colors = lut
                .get_colors(self.settings.species_count as usize)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();
            tracing::debug!(
                "Non-LUT mode: got {} colors for {} species",
                colors.len(),
                self.settings.species_count
            );
            (colors, lut.name.clone(), lut_reversed)
        };

        // Update stored colors and LUT info
        self.lut_colors = new_lut_colors;
        self.current_lut_name = new_lut_name;
        self.lut_reversed = new_lut_reversed;

        tracing::debug!(
            "Updated LUT: name={}, reversed={}, colors.len={}",
            self.current_lut_name,
            self.lut_reversed,
            self.lut_colors.len()
        );
        if let Some(&[r, g, b, a]) = self.lut_colors.first() {
            tracing::debug!("First LUT color (background): [{}, {}, {}, {}]", r, g, b, a);
        }

        // Recreate LUT texture with new colors
        self.recreate_lut_texture(device, queue)?;

        Ok(())
    }

    /// Recreate LUT texture with current colors
    fn recreate_lut_texture(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Create new LUT texture
        let lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("LUT Texture"),
            size: wgpu::Extent3d {
                width: self.lut_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload LUT colors to texture
        let lut_data: Vec<u8> = self
            .lut_colors
            .iter()
            .flat_map(|&[r, g, b, a]| {
                [
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (a * 255.0) as u8,
                ]
            })
            .collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &lut_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &lut_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.lut_colors.len() as u32 * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: self.lut_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        // Create new texture view and sampler
        let lut_view = lut_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let lut_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("LUT Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Recreate LUT bind group
        self.lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LUT Bind Group"),
            layout: &self.render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&lut_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.lut_size_buffer.as_entire_binding(),
                },
            ],
        });

        // Update LUT size buffer
        queue.write_buffer(
            &self.lut_size_buffer,
            0,
            bytemuck::cast_slice(&[self.lut_colors.len() as u32]),
        );

        Ok(())
    }

    /// Get the current LUT size for shader uniform
    pub fn get_lut_size(&self) -> u32 {
        self.lut_colors.len() as u32
    }
}

impl Simulation for ParticleLifeModel {
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

        // Update GPU buffers with current state
        self.update_sim_params(device, queue);

        // Update camera with smoothing using actual delta time
        self.camera.update(delta_time);

        // Update camera
        self.camera.upload_to_gpu(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Render Encoder"),
        });

        // Run multiple physics steps per frame for maximum speed
        let physics_steps_per_frame = 3; // Reduced from 10 since we removed time step

        for _ in 0..physics_steps_per_frame {
            // Compute pass - update particle positions
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Particle Life Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

                let workgroup_size = 64;
                let num_workgroups =
                    (self.state.particle_count + workgroup_size - 1) / workgroup_size;
                tracing::debug!(
                    "Compute dispatch: particle_count={}, num_workgroups={}, buffer_size={}",
                    self.state.particle_count,
                    num_workgroups,
                    self.particle_buffer.size()
                );
                compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
        }

        // Render pass - draw particles
        {
            // Determine background color based on color mode
            let background_color = match self.color_mode {
                ColorMode::Gray18 => wgpu::Color {
                    r: 0.18,
                    g: 0.18,
                    b: 0.18,
                    a: 1.0,
                },
                ColorMode::White => wgpu::Color::WHITE,
                ColorMode::Black => wgpu::Color::BLACK,
                ColorMode::Lut => {
                    // Use first LUT color as background
                    if let Some(&[r, g, b, a]) = self.lut_colors.first() {
                        tracing::debug!(
                            "LUT background color: [{}, {}, {}, {}], lut_colors.len: {}",
                            r,
                            g,
                            b,
                            a,
                            self.lut_colors.len()
                        );
                        wgpu::Color {
                            r: r.into(),
                            g: g.into(),
                            b: b.into(),
                            a: a.into(),
                        }
                    } else {
                        tracing::warn!("No LUT colors available, using black background");
                        wgpu::Color::BLACK // Fallback
                    }
                }
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Particle Life Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

            // Draw instanced particles (6 vertices per particle for quad)
            tracing::debug!(
                "Render draw: drawing {} particles",
                self.state.particle_count
            );
            render_pass.draw(0..6, 0..self.state.particle_count as u32);
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
        self.width = new_config.width;
        self.height = new_config.height;

        // Update camera viewport
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "species_count" => {
                if let Some(count) = value.as_u64() {
                    let old_count = self.settings.species_count;
                    self.settings.set_species_count(count as u32);

                    // Recreate force matrix buffer with new size
                    let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
                    self.force_matrix_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Force Matrix Buffer"),
                            contents: bytemuck::cast_slice(&force_matrix_data),
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        });

                    // Recreate bind groups that use this buffer
                    self.recreate_bind_groups_with_force_matrix(device);

                    // Update LUT colors for new species count
                    let current_lut_name = self.current_lut_name.clone();
                    let lut_reversed = self.lut_reversed;
                    let lut_manager = self.lut_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &lut_manager,
                        self.color_mode,
                        Some(&current_lut_name),
                        lut_reversed,
                    )?;

                    // Respawn all particles to ensure proper species distribution
                    self.initialize_particles_gpu(device, queue)?;

                    tracing::info!(
                        "Updated species count from {} to {} (respawned all particles)",
                        old_count,
                        count
                    );
                }
            }
            "particle_count" => {
                if let Some(count) = value.as_u64() {
                    self.update_particle_count(count as u32, device, queue)?;
                }
            }
            "force_matrix" => {
                if let Some(matrix_array) = value.as_array() {
                    // Update CPU side for UI display
                    for (i, row) in matrix_array.iter().enumerate() {
                        if let Some(row_array) = row.as_array() {
                            for (j, val) in row_array.iter().enumerate() {
                                if let Some(force_val) = val.as_f64() {
                                    if i < self.settings.force_matrix.len()
                                        && j < self.settings.force_matrix[i].len()
                                    {
                                        self.settings.force_matrix[i][j] = force_val as f32;
                                    }
                                }
                            }
                        }
                    }
                    // Update entire LJ params buffer since we changed the force matrix
                    let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
                    queue.write_buffer(
                        &self.force_matrix_buffer,
                        0,
                        bytemuck::cast_slice(&force_matrix_data),
                    );
                }
            }
            "max_force" => {
                if let Some(force) = value.as_f64() {
                    self.settings.max_force = force as f32;
                }
            }
            "min_distance" => {
                if let Some(dist) = value.as_f64() {
                    self.settings.min_distance = dist as f32;
                }
            }
            "max_distance" => {
                if let Some(dist) = value.as_f64() {
                    self.settings.max_distance = dist as f32;
                }
            }
            "friction" => {
                if let Some(friction) = value.as_f64() {
                    self.settings.friction = friction as f32;
                }
            }
            "force_beta" => {
                if let Some(beta) = value.as_f64() {
                    self.settings.force_beta = beta as f32;
                }
            }
            "wrap_edges" => {
                if let Some(wrap) = value.as_bool() {
                    self.settings.wrap_edges = wrap;
                }
            }
            "dt" => {
                if let Some(dt) = value.as_f64() {
                    self.state.dt = dt as f32;
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.state.cursor_size = size as f32;
                }
            }
            "cursor_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.cursor_strength = (strength as f32).clamp(0.0, 10.0);
                }
            }
            "traces_enabled" => {
                if let Some(enabled) = value.as_bool() {
                    self.state.traces_enabled = enabled;
                }
            }
            "trace_fade" => {
                if let Some(fade) = value.as_f64() {
                    self.state.trace_fade = fade as f32;
                }
            }
            "edge_fade_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.edge_fade_strength = strength as f32;
                }
            }
            "random_seed" => {
                if let Some(seed) = value.as_u64() {
                    self.state.random_seed = seed as u32;
                }
            }
            "position_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Random" => super::settings::PositionGenerator::Random,
                        "Center" => super::settings::PositionGenerator::Center,
                        "UniformCircle" => super::settings::PositionGenerator::UniformCircle,
                        "CenteredCircle" => super::settings::PositionGenerator::CenteredCircle,
                        "Ring" => super::settings::PositionGenerator::Ring,
                        "RainbowRing" => super::settings::PositionGenerator::RainbowRing,
                        "ColorBattle" => super::settings::PositionGenerator::ColorBattle,
                        "ColorWheel" => super::settings::PositionGenerator::ColorWheel,
                        "Line" => super::settings::PositionGenerator::Line,
                        "Spiral" => super::settings::PositionGenerator::Spiral,
                        "RainbowSpiral" => super::settings::PositionGenerator::RainbowSpiral,
                        _ => super::settings::PositionGenerator::Random,
                    };
                    self.state.position_generator = generator;
                    // Regenerate particles with new position generator
                    self.initialize_particles_gpu(device, queue)?;
                }
            }
            "type_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Random" => super::settings::TypeGenerator::Random,
                        "Randomize10Percent" => super::settings::TypeGenerator::Randomize10Percent,
                        "Slices" => super::settings::TypeGenerator::Slices,
                        "Onion" => super::settings::TypeGenerator::Onion,
                        "Rotate" => super::settings::TypeGenerator::Rotate,
                        "Flip" => super::settings::TypeGenerator::Flip,
                        "MoreOfFirst" => super::settings::TypeGenerator::MoreOfFirst,
                        "KillStill" => super::settings::TypeGenerator::KillStill,
                        _ => super::settings::TypeGenerator::Random,
                    };
                    self.state.type_generator = generator;
                    // Regenerate particles with new type generator
                    self.initialize_particles_gpu(device, queue)?;
                }
            }
            "matrix_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Random" => super::settings::MatrixGenerator::Random,
                        "Symmetry" => super::settings::MatrixGenerator::Symmetry,
                        "Chains" => super::settings::MatrixGenerator::Chains,
                        "Chains2" => super::settings::MatrixGenerator::Chains2,
                        "Chains3" => super::settings::MatrixGenerator::Chains3,
                        "Snakes" => super::settings::MatrixGenerator::Snakes,
                        "Zero" => super::settings::MatrixGenerator::Zero,
                        "PredatorPrey" => super::settings::MatrixGenerator::PredatorPrey,
                        "Symbiosis" => super::settings::MatrixGenerator::Symbiosis,
                        "Territorial" => super::settings::MatrixGenerator::Territorial,
                        "Magnetic" => super::settings::MatrixGenerator::Magnetic,
                        "Crystal" => super::settings::MatrixGenerator::Crystal,
                        "Wave" => super::settings::MatrixGenerator::Wave,
                        "Hierarchy" => super::settings::MatrixGenerator::Hierarchy,
                        "Clique" => super::settings::MatrixGenerator::Clique,
                        "AntiClique" => super::settings::MatrixGenerator::AntiClique,
                        "Fibonacci" => super::settings::MatrixGenerator::Fibonacci,
                        "Prime" => super::settings::MatrixGenerator::Prime,
                        "Fractal" => super::settings::MatrixGenerator::Fractal,
                        "RockPaperScissors" => super::settings::MatrixGenerator::RockPaperScissors,
                        "Cooperation" => super::settings::MatrixGenerator::Cooperation,
                        "Competition" => super::settings::MatrixGenerator::Competition,
                        _ => super::settings::MatrixGenerator::Random,
                    };
                    // Generate new force matrix before moving the generator
                    self.settings.randomize_force_matrix(&generator);
                    self.state.matrix_generator = generator;
                    self.recreate_bind_groups_with_force_matrix(device);
                    self.update_sim_params(device, queue);
                }
            }
            "color_mode" => {
                if let Some(mode_str) = value.as_str() {
                    let color_mode = match mode_str {
                        "Gray18" => ColorMode::Gray18,
                        "White" => ColorMode::White,
                        "Black" => ColorMode::Black,
                        "Lut" => ColorMode::Lut,
                        _ => ColorMode::Lut,
                    };
                    // Update LUT with new color mode
                    let current_lut_name = self.current_lut_name.clone();
                    let lut_reversed = self.lut_reversed;
                    let lut_manager = self.lut_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &lut_manager,
                        color_mode,
                        Some(&current_lut_name),
                        lut_reversed,
                    )?;
                }
            }
            "lut_name" => {
                if let Some(lut_name) = value.as_str() {
                    let color_mode = self.color_mode;
                    let lut_reversed = self.lut_reversed;
                    let lut_manager = self.lut_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &lut_manager,
                        color_mode,
                        Some(lut_name),
                        lut_reversed,
                    )?;
                }
            }
            "lut_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    let color_mode = self.color_mode;
                    let current_lut_name = self.current_lut_name.clone();
                    let lut_manager = self.lut_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &lut_manager,
                        color_mode,
                        Some(&current_lut_name),
                        reversed,
                    )?;
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
        // Calculate type distribution
        let type_counts = self.calculate_type_distribution();

        serde_json::json!({
            "particle_count": self.state.particle_count,
            "species_count": self.settings.species_count,
            "type_counts": type_counts,
            "random_seed": self.state.random_seed,
            "dt": self.state.dt,
            "cursor_size": self.state.cursor_size,
            "cursor_strength": self.state.cursor_strength,
            "traces_enabled": self.state.traces_enabled,
            "trace_fade": self.state.trace_fade,
            "edge_fade_strength": self.state.edge_fade_strength,
            "position_generator": self.state.position_generator,
            "type_generator": self.state.type_generator,
            "matrix_generator": self.state.matrix_generator,
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        is_attract: bool,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Determine cursor mode based on is_attract and handle mouse release
        let cursor_mode = if world_x == -9999.0 && world_y == -9999.0 {
            0 // mouse release - turn off cursor interaction
        } else if is_attract { 
            1 // attract
        } else {
            2 // repel
        };
        
        // Store coordinates directly - conversion is handled in the manager
        let (sim_x, sim_y) = if cursor_mode == 0 {
            (0.0, 0.0) // Don't matter when cursor is inactive
        } else {
            (world_x, world_y)
        };
        
        // Store cursor values in the model
        self.cursor_active_mode = cursor_mode;
        self.cursor_world_x = sim_x;
        self.cursor_world_y = sim_y;
        
        println!("🎯 Mouse interaction: world=({}, {}), mode={} ({}), size={}, strength={} (scaled: {})", 
                 sim_x, sim_y, cursor_mode, 
                 match cursor_mode { 0 => "inactive", 1 => "attract", 2 => "repel", _ => "unknown" },
                 self.state.cursor_size, self.state.cursor_strength, 
                 self.state.cursor_strength * self.settings.max_force * 10.0);
        println!("📏 Simulation bounds: width={}, height={}, particles live in [-2,2] space", 
                 self.width, self.height);
        
        // Update sim params immediately with new cursor values
        let mut sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );
        
        // Override with cursor values
        sim_params.cursor_x = sim_x;
        sim_params.cursor_y = sim_y;
        sim_params.cursor_active = cursor_mode;
        if cursor_mode > 0 {
            sim_params.cursor_strength = self.state.cursor_strength * self.settings.max_force * 10.0;
        }
        
        // Upload to GPU immediately
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
        
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
        self.camera.get_state()
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // TODO: Implement preset saving
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // TODO: Implement preset loading
        Ok(())
    }

    fn apply_settings(&mut self, settings: Value, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            let old_species_count = self.settings.species_count;
            self.settings = new_settings;
            
            // Upload the entire force matrix when applying new settings
            let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
            queue.write_buffer(
                &self.force_matrix_buffer,
                0,
                bytemuck::cast_slice(&force_matrix_data),
            );
            
            // Update LUT if species count changed
            if self.settings.species_count != old_species_count {
                let current_lut_name = self.current_lut_name.clone();
                let lut_reversed = self.lut_reversed;
                let lut_manager = self.lut_manager.clone();
                self.update_lut(
                    device,
                    queue,
                    &lut_manager,
                    self.color_mode,
                    Some(&current_lut_name),
                    lut_reversed,
                )?;
            }
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update random seed for reset
        use rand::Rng;
        let mut rng = rand::rng();
        self.state.random_seed = rng.random();

        // Update sim params with new random seed
        self.update_sim_params(device, queue);

        // Re-initialize particles on GPU with new random seed
        self.initialize_particles_gpu(device, queue)?;

        // Ensure GPU operations complete
        device.poll(wgpu::Maintain::Wait);

        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.show_gui = !self.show_gui;
        self.show_gui
    }

    fn is_gui_visible(&self) -> bool {
        self.show_gui
    }

    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Store the current matrix generator to avoid borrowing issues
        let matrix_generator = self.state.matrix_generator;

        // Generate new force matrix using the current matrix generator
        self.settings.randomize_force_matrix(&matrix_generator);

        // Update the force matrix buffer on GPU
        let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
        queue.write_buffer(
            &self.force_matrix_buffer,
            0,
            bytemuck::cast_slice(&force_matrix_data),
        );

        // Update random seed for consistency
        use rand::Rng;
        let mut rng = rand::rng();
        self.state.random_seed = rng.random();

        // Update sim params with new random seed
        self.update_sim_params(device, queue);

        // Note: Physics settings (max_force, distances, friction, wrap_edges)
        // are intentionally NOT randomized to preserve user's simulation setup
        // Note: particle_count and species_count are preserved

        Ok(())
    }
}

impl ParticleLifeModel {
    /// Update particle count by recreating buffer and respawning all particles
    pub fn update_particle_count(
        &mut self,
        new_count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let new_count = new_count.clamp(1000, 100000);
        let old_count = self.state.particle_count as u32;

        if new_count == old_count {
            tracing::info!("Particle count unchanged at {}, skipping update", new_count);
            return Ok(());
        }

        tracing::info!(
            "Starting particle count update: {} -> {}",
            old_count,
            new_count
        );

        // Update state
        self.state.particle_count = new_count as usize;

        tracing::info!(
            "Updated state: particle_count={}",
            self.state.particle_count
        );

        // Check buffer size limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let new_particle_buffer_size =
            (new_count as usize * std::mem::size_of::<Particle>()) as u64;

        if new_particle_buffer_size > max_storage_buffer_size {
            return Err(SimulationError::BufferTooLarge {
                requested: new_particle_buffer_size,
                max_available: max_storage_buffer_size,
            });
        }

        // Create new particle buffer with new size
        let new_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: new_particle_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Replace the buffer
        self.particle_buffer = new_particle_buffer;

        tracing::info!("Recreating bind groups after buffer recreation");
        // Recreate bind groups with new buffer
        self.recreate_bind_groups(device)?;

        tracing::info!("Updating sim params buffer BEFORE particle initialization");
        // Update simulation parameters with new particle count BEFORE initializing particles
        self.update_sim_params(device, queue);

        tracing::info!("Respawning particles on GPU with count: {}", new_count);
        // Respawn all particles with new count
        self.initialize_particles_gpu(device, queue)?;

        tracing::info!("Waiting for GPU commands to complete");
        // Force GPU to finish all commands to ensure buffer updates are complete
        device.poll(wgpu::Maintain::Wait);

        tracing::info!(
            "Particle count update complete: {} -> {} (buffer_size={})",
            old_count,
            new_count,
            self.particle_buffer.size()
        );
        Ok(())
    }

    /// Recreate bind groups after particle buffer changes
    fn recreate_bind_groups(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
        tracing::info!("Recreating compute bind group");
        // Recreate compute bind group
        self.compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Compute Bind Group"),
            layout: &self.compute_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.force_matrix_buffer.as_entire_binding(),
                },
            ],
        });

        tracing::info!("Recreating render bind group");
        // Recreate render bind group
        self.render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Render Bind Group"),
            layout: &self.render_particles_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
            ],
        });

        tracing::info!("Recreating init bind group");
        // Recreate init bind group (critical for particle initialization)
        self.init_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Init Bind Group"),
            layout: &self.init_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.init_params_buffer.as_entire_binding(),
                },
            ],
        });

        tracing::info!("All bind groups recreated successfully");
        Ok(())
    }
}
