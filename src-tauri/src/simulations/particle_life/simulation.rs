use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::{MatrixGenerator, Settings, TypeGenerator};
use super::shaders;
use crate::simulations::shared::{LutManager, PositionGenerator, camera::Camera};
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
pub struct FadeUniforms {
    pub background_color: [f32; 4], // RGBA background color
    pub fade_alpha: f32,            // Alpha for fading effect
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
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
    pub dt: f32,       // Time step for simulation
    pub beta: f32,     // Transition point between repulsion and attraction zones
    pub cursor_x: f32, // Cursor position in world coordinates
    pub cursor_y: f32,
    pub cursor_size: f32,        // Cursor interaction radius
    pub cursor_strength: f32,    // Cursor force strength
    pub cursor_active: u32, // Whether cursor interaction is active (0 = inactive, 1 = attract, 2 = repel)
    pub brownian_motion: f32, // Brownian motion strength (0.0-1.0)
    pub traces_enabled: u32, // Whether particle traces are enabled
    pub trace_fade: f32,    // Trace fade factor (0.0-1.0)
    pub edge_fade_strength: f32, // Edge fade strength
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
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
            brownian_motion: settings.brownian_motion,
            traces_enabled: if state.traces_enabled { 1 } else { 0 },
            trace_fade: state.trace_fade,
            edge_fade_strength: state.edge_fade_strength,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
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
    // LUT management (moved from main struct)
    pub current_lut_name: String,
    pub lut_reversed: bool,
    pub color_mode: ColorMode,
    /// Pre-computed exact RGBA colors for each species, used for both UI display and GPU rendering
    /// In LUT mode: contains species_count + 1 colors (background + species)
    /// In non-LUT mode: contains exactly species_count colors, one for each species
    pub species_colors: Vec<[f32; 4]>, // RGBA colors, always up-to-date
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
            trace_fade: 0.48,
            edge_fade_strength: 1.0,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
            current_lut_name: "MATPLOTLIB_nipy_spectral".to_string(), // Use a proper default
            lut_reversed: false,
            color_mode: ColorMode::Lut, // Use LUT mode as default to match main constructor
            // Placeholder values - will be properly initialized when LUT is loaded in main constructor
            species_colors: vec![[0.0, 0.0, 0.0, 1.0]],
        }
    }

    pub fn reset(&mut self, _species_count: u32, _width: u32, _height: u32, _random_seed: u32) {
        // No longer used - particles are initialized on GPU
    }
}

// Add color mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    Gray18,
    White,
    Black,
    #[serde(rename = "LUT")]
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
    pub color_mode_buffer: wgpu::Buffer,
    pub species_colors_buffer: wgpu::Buffer,

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

    // Fade pipeline for traces
    pub fade_pipeline: wgpu::RenderPipeline,
    pub fade_bind_group_layout: wgpu::BindGroupLayout,
    pub fade_bind_group: wgpu::BindGroup,
    pub fade_uniforms_buffer: wgpu::Buffer,

    // Trail texture for persistent trails
    pub trail_texture: wgpu::Texture,
    pub trail_texture_view: wgpu::TextureView,

    // Blit pipeline to copy trail texture to surface
    pub blit_pipeline: wgpu::RenderPipeline,
    pub blit_bind_group_layout: wgpu::BindGroupLayout,
    pub blit_bind_group: wgpu::BindGroup,

    // Simulation state and settings
    pub settings: Settings,
    pub state: State,
    pub show_gui: bool,

    // LUT management
    pub lut_manager: Arc<LutManager>, // Store reference to LUT manager

    // Dimensions
    pub width: u32,
    pub height: u32,

    // Camera for viewport control
    pub camera: Camera,

    // Frame timing for smooth camera movement
    last_frame_time: std::time::Instant,

    // Cursor interaction state
    pub cursor_active_mode: u32, // 0=inactive, 1=attract, 2=repel
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,
}

impl ParticleLifeModel {
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

    #[allow(clippy::too_many_arguments)]
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

        // Use a proper default LUT name instead of hardcoding
        let default_lut_name = "MATPLOTLIB_nipy_spectral";

        // Get LUT and calculate colors first
        let lut = lut_manager.get(default_lut_name).map_err(|e| {
            SimulationError::InitializationFailed(format!(
                "Failed to load default LUT '{}': {}",
                default_lut_name, e
            ))
        })?;

        // Create LUT buffer
        let (lut_colors, current_lut_name) = if color_mode == ColorMode::Lut {
            // <num_species> + 1 equidistant stops for LUT mode (first for background, rest for species)
            let colors = lut
                .get_colors(settings.species_count as usize + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            // Reorder colors: put background color at the end, species colors at the beginning
            // This way the GPU can use colors[0..species_count] for species and colors[species_count] for background
            let mut reordered_colors = Vec::with_capacity(settings.species_count as usize + 1);

            // First, add all species colors (skip the first color which is background)
            for color in colors.iter().skip(1) {
                reordered_colors.push(*color);
            }

            // Then add the background color at the end
            reordered_colors.push(colors[0]);

            tracing::trace!(
                "Constructor LUT mode: got {} equidistant colors for {} species (reordered)",
                reordered_colors.len(),
                settings.species_count
            );
            (reordered_colors, lut.name.clone())
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

        // Create initial state with proper LUT colors
        let state = State {
            particle_count,
            particles: vec![], // Empty - will be initialized on GPU
            random_seed: 0,
            dt: 0.016,
            cursor_size: 0.5,
            cursor_strength: 1.0,
            traces_enabled: false,
            trace_fade: 0.48,
            edge_fade_strength: 1.0,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
            current_lut_name,
            lut_reversed: false,
            color_mode,
            species_colors: lut_colors.clone(), // Will be properly computed in update_lut
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

        let lut_data_u32 = state
            .species_colors
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

        // LUT bind group layout (now just species colors uniform)
        let lut_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Species Colors Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
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
                    blend: Some(wgpu::BlendState::REPLACE),
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
                width: state.species_colors.len() as u32,
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
        let lut_data: Vec<u8> = state
            .species_colors
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
                bytes_per_row: Some(state.species_colors.len() as u32 * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: state.species_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        // Create LUT size uniform buffer
        let lut_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Size Buffer"),
            contents: bytemuck::cast_slice(&[state.species_colors.len() as u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create color mode uniform buffer
        let color_mode_value = match color_mode {
            ColorMode::Gray18 => 0u32,
            ColorMode::White => 1u32,
            ColorMode::Black => 2u32,
            ColorMode::Lut => 3u32,
        };
        let color_mode_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Color Mode Buffer"),
            contents: bytemuck::cast_slice(&[color_mode_value]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create species colors buffer
        let species_colors_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Species Colors Buffer"),
            size: (9 * std::mem::size_of::<[f32; 4]>()) as u64, // Allocate space for 9 colors (background + 8 species)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create initial species colors data (pad to 9 colors)
        let mut species_colors_data = vec![[0.0f32, 0.0f32, 0.0f32, 1.0f32]; 9];
        for (i, &color) in state.species_colors.iter().enumerate().take(9) {
            if i < settings.species_count as usize {
                species_colors_data[i] = color;
            }
        }

        // Upload initial species colors to GPU buffer
        let species_colors_bytes = bytemuck::cast_slice(&species_colors_data);
        queue.write_buffer(&species_colors_buffer, 0, species_colors_bytes);

        // Create initial species colors bind group
        let lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Species Colors Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: species_colors_buffer.as_entire_binding(),
            }],
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

        // Create fade pipeline for traces
        let fade_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fade Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_VERTEX_SHADER.into()),
        });

        let fade_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fade Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_FRAGMENT_SHADER.into()),
        });

        let fade_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Fade Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let fade_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fade Pipeline Layout"),
            bind_group_layouts: &[&fade_bind_group_layout],
            push_constant_ranges: &[],
        });

        let fade_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fade Pipeline"),
            layout: Some(&fade_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fade_vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fade_fragment_shader,
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

        // Create fade uniforms buffer
        let fade_uniforms = FadeUniforms {
            background_color: [0.0, 0.0, 0.0, 1.0],
            fade_alpha: 0.1,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        let fade_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fade Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[fade_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let fade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fade Bind Group"),
            layout: &fade_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: fade_uniforms_buffer.as_entire_binding(),
            }],
        });

        // Trail texture for persistent trails
        let trail_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Trail Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Trail texture view
        let trail_texture_view = trail_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Trail Texture View"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(surface_config.format),
            ..Default::default()
        });

        // Blit pipeline to copy trail texture to surface
        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Blit Bind Group Layout"),
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
                ],
            });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create blit shaders
        let blit_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_VERTEX_SHADER.into()), // Reuse fade vertex shader
        });

        let blit_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
                @group(0) @binding(0) var trail_texture: texture_2d<f32>;
                @group(0) @binding(1) var trail_sampler: sampler;

                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                }

                @fragment
                fn main(input: VertexOutput) -> @location(0) vec4<f32> {
                    return textureSample(trail_texture, trail_sampler, input.uv);
                }
                "#
                .into(),
            ),
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_fragment_shader,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None, // No blending for blit
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

        // Create sampler for blit
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blit Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Blit bind group
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&trail_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });

        let mut result = Self {
            particle_buffer,
            sim_params_buffer,
            force_matrix_buffer,
            lut_buffer,
            lut_size_buffer,
            color_mode_buffer,
            species_colors_buffer,
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
            fade_pipeline,
            fade_bind_group_layout,
            fade_bind_group,
            fade_uniforms_buffer,
            trail_texture,
            trail_texture_view,
            blit_pipeline,
            blit_bind_group_layout,
            blit_bind_group,
            settings,
            state,
            show_gui: true,
            lut_manager: Arc::new(lut_manager.clone()),
            width,
            height,
            camera,
            last_frame_time: std::time::Instant::now(),
            cursor_active_mode: 0,
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
        };

        // Initialize LUT and species colors properly
        let lut_manager_clone = result.lut_manager.clone();
        result.update_lut(
            device,
            queue,
            &lut_manager_clone,
            color_mode,
            Some("MATPLOTLIB_nipy_spectral"),
            false,
        )?;

        // Initialize particles on GPU
        result.initialize_particles_gpu(device, queue)?;

        // Initialize trail texture with background color
        let background_color = match color_mode {
            ColorMode::Gray18 => wgpu::Color {
                r: 0.18,
                g: 0.18,
                b: 0.18,
                a: 1.0,
            },
            ColorMode::White => wgpu::Color::WHITE,
            ColorMode::Black => wgpu::Color::BLACK,
            ColorMode::Lut => {
                if let Some(&[r, g, b, a]) = result.state.species_colors.last() {
                    wgpu::Color {
                        r: r.into(),
                        g: g.into(),
                        b: b.into(),
                        a: a.into(),
                    }
                } else {
                    wgpu::Color::BLACK
                }
            }
        };
        result.clear_trail_texture(device, queue, background_color);

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
            sim_params.cursor_strength =
                self.state.cursor_strength * self.settings.max_force * 10.0;
        }

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
            let num_workgroups = self.state.particle_count.div_ceil(workgroup_size);
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
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.state.random_seed as u64);
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
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.state.random_seed as u64);
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
            let num_workgroups = self.settings.species_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, num_workgroups, 1);
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
        self.state.color_mode = color_mode;

        // Get LUT name and validate
        let lut_name = lut_name.unwrap_or(&self.state.current_lut_name);
        if lut_name.is_empty() {
            return Err(SimulationError::InvalidSetting {
                setting_name: "lut_name".to_string(),
                message: "LUT name is empty but LUT color mode is enabled".to_string(),
            });
        }

        let mut lut = lut_manager
            .get(lut_name)
            .map_err(|e| SimulationError::InvalidSetting {
                setting_name: "lut_name".to_string(),
                message: format!("Failed to load LUT '{}': {}", lut_name, e),
            })?;

        if lut_reversed {
            lut = lut.reversed();
        }

        // Compute species colors based on color mode
        let species_count = self.settings.species_count as usize;
        let mut species_colors = Vec::with_capacity(species_count);

        if color_mode == ColorMode::Lut {
            // Get species_count + 1 equidistant stops for LUT mode (first for background, rest for species)
            let lut_colors = lut
                .get_colors(species_count + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            // Reorder colors: put background color at the end, species colors at the beginning
            // This way the GPU can use colors[0..species_count] for species and colors[species_count] for background
            let mut reordered_colors = Vec::with_capacity(species_count + 1);

            // First, add all species colors (skip the first color which is background)
            for color in lut_colors.iter().skip(1) {
                reordered_colors.push(*color);
            }

            // Then add the background color at the end
            reordered_colors.push(lut_colors[0]);

            // Store reordered colors (species first, background last)
            species_colors = reordered_colors;

            tracing::debug!(
                "LUT mode: stored {} colors ({} species + background) from LUT",
                species_colors.len(),
                species_count
            );
        } else {
            // Get species_count colors for non-LUT mode
            let lut_colors = lut
                .get_colors(species_count)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            // Direct mapping for non-LUT mode
            for species_index in 0..species_count {
                let color = lut_colors
                    .get(species_index)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                species_colors.push(color);
            }

            tracing::debug!(
                "Non-LUT mode: got {} colors for {} species",
                species_colors.len(),
                species_count
            );
        }

        // Update stored colors and LUT info
        self.state.species_colors = species_colors;
        // Store the original LUT name, not the reversed LUT name
        self.state.current_lut_name = lut_name.to_string();
        self.state.lut_reversed = lut_reversed;

        tracing::debug!(
            "Updated LUT: name={}, reversed={}, species_colors.len={}",
            self.state.current_lut_name,
            self.state.lut_reversed,
            self.state.species_colors.len()
        );

        // Update species colors on GPU
        self.update_species_colors_gpu(device, queue)?;

        Ok(())
    }

    /// Update species colors on GPU
    fn update_species_colors_gpu(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // In LUT mode, we have species_count + 1 colors (background + species)
        // In non-LUT mode, we have species_count colors
        let total_colors = self.state.species_colors.len();

        // Prepare species colors data (pad to 9 colors to accommodate LUT mode)
        let mut species_colors_data = [[0.0f32, 0.0f32, 0.0f32, 1.0f32]; 9];
        for (i, &color) in self.state.species_colors.iter().enumerate().take(9) {
            species_colors_data[i] = color;
        }

        // Upload species colors to GPU buffer
        let species_colors_bytes = bytemuck::cast_slice(&species_colors_data);
        queue.write_buffer(&self.species_colors_buffer, 0, species_colors_bytes);

        // Update species colors bind group
        self.lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Species Colors Bind Group"),
            layout: &self.render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.species_colors_buffer.as_entire_binding(),
            }],
        });

        tracing::debug!(
            "Updated GPU colors: total_colors={}, color_mode={:?}",
            total_colors,
            self.state.color_mode
        );

        Ok(())
    }

    /// Get the current LUT size for shader uniform
    pub fn get_lut_size(&self) -> u32 {
        self.state.species_colors.len() as u32
    }

    /// Update fade uniforms for trace rendering
    fn update_fade_uniforms(
        &self,
        queue: &Arc<Queue>,
        background_color: wgpu::Color,
        fade_alpha: f32,
    ) {
        let fade_uniforms = FadeUniforms {
            background_color: [
                background_color.r as f32,
                background_color.g as f32,
                background_color.b as f32,
                background_color.a as f32,
            ],
            fade_alpha,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        queue.write_buffer(
            &self.fade_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[fade_uniforms]),
        );
    }

    /// Clear the trail texture with the background color
    pub fn clear_trail_texture(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        background_color: wgpu::Color,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Trail Texture Encoder"),
        });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Trail Texture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.trail_texture_view,
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
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

impl Simulation for ParticleLifeModel {
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

        // Update camera with smoothing using actual delta time
        self.camera.update(delta_time);

        // Update camera
        self.camera.upload_to_gpu(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Static Render Encoder"),
        });

        // Skip compute pass - just render current particle positions

        // Determine background color based on color mode
        let background_color = match self.state.color_mode {
            ColorMode::Gray18 => wgpu::Color {
                r: 0.18,
                g: 0.18,
                b: 0.18,
                a: 1.0,
            },
            ColorMode::White => wgpu::Color::WHITE,
            ColorMode::Black => wgpu::Color::BLACK,
            ColorMode::Lut => {
                if let Some(&[r, g, b, a]) = self.state.species_colors.last() {
                    wgpu::Color {
                        r: r.into(),
                        g: g.into(),
                        b: b.into(),
                        a: a.into(),
                    }
                } else {
                    wgpu::Color::BLACK
                }
            }
        };

        // Render pass - draw particles at their current positions
        {
            if self.state.traces_enabled {
                // When trails are enabled, render to trail texture first
                let mut trail_render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Static Trail Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.trail_texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Preserve previous trail content
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                // For static rendering, don't add new trails - just render existing particles
                trail_render_pass.set_pipeline(&self.render_pipeline);
                trail_render_pass.set_bind_group(0, &self.render_bind_group, &[]);
                trail_render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
                trail_render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

                let instance_count = self.state.particle_count as u32 * 9;
                trail_render_pass.draw(0..6, 0..instance_count);
                drop(trail_render_pass);

                // Then blit trail texture to surface
                let mut surface_render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Static Surface Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: surface_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(background_color), // Clear surface
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                // Blit trail texture to surface
                surface_render_pass.set_pipeline(&self.blit_pipeline);
                surface_render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
                surface_render_pass.draw(0..3, 0..1); // Fullscreen triangle
            } else {
                // When trails are disabled, render directly to surface
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Static Direct Render Pass"),
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

                // Render particles directly to surface
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.render_bind_group, &[]);
                render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
                render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

                let instance_count = self.state.particle_count as u32 * 9;
                render_pass.draw(0..6, 0..instance_count);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

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
                let num_workgroups = self.state.particle_count.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
            }
        }

        // Render pass - draw particles
        {
            // Determine background color based on color mode
            let background_color = match self.state.color_mode {
                ColorMode::Gray18 => wgpu::Color {
                    r: 0.18,
                    g: 0.18,
                    b: 0.18,
                    a: 1.0,
                },
                ColorMode::White => wgpu::Color::WHITE,
                ColorMode::Black => wgpu::Color::BLACK,
                ColorMode::Lut => {
                    // Use last LUT color as background
                    if let Some(&[r, g, b, a]) = self.state.species_colors.last() {
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

            if self.state.traces_enabled {
                // When trails are enabled, render to trail texture first
                let mut trail_render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Trail Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.trail_texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load, // Preserve previous trail content
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                // First, apply fade effect if trace_fade < 1.0
                if self.state.trace_fade < 1.0 {
                    // Calculate fade alpha: higher trace_fade = slower fading = lower alpha
                    // Use exponential scaling for more natural fading
                    let fade_factor = 1.0 - self.state.trace_fade;
                    let fade_alpha = (fade_factor * fade_factor * 0.3).clamp(0.002, 0.3);

                    // Update fade uniforms with background color and fade alpha
                    self.update_fade_uniforms(queue, background_color, fade_alpha);

                    // Render fade quad to create fading effect
                    trail_render_pass.set_pipeline(&self.fade_pipeline);
                    trail_render_pass.set_bind_group(0, &self.fade_bind_group, &[]);
                    trail_render_pass.draw(0..3, 0..1); // Fullscreen triangle
                }

                // Then render particles on top
                trail_render_pass.set_pipeline(&self.render_pipeline);
                trail_render_pass.set_bind_group(0, &self.render_bind_group, &[]);
                trail_render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
                trail_render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

                let instance_count = self.state.particle_count as u32 * 9;
                trail_render_pass.draw(0..6, 0..instance_count);
                drop(trail_render_pass);

                // Then blit trail texture to surface
                let mut surface_render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Surface Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: surface_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(background_color), // Clear surface
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                // Blit trail texture to surface
                surface_render_pass.set_pipeline(&self.blit_pipeline);
                surface_render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
                surface_render_pass.draw(0..3, 0..1); // Fullscreen triangle
            } else {
                // When trails are disabled, render directly to surface
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Direct Render Pass"),
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

                // Render particles directly to surface
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.render_bind_group, &[]);
                render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
                render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

                let instance_count = self.state.particle_count as u32 * 9;
                render_pass.draw(0..6, 0..instance_count);
            }
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
                    let current_lut_name = self.state.current_lut_name.clone();
                    let lut_reversed = self.state.lut_reversed;
                    let lut_manager = self.lut_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &lut_manager,
                        self.state.color_mode,
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
            "brownian_motion" => {
                if let Some(brownian) = value.as_f64() {
                    self.settings.brownian_motion = (brownian as f32).clamp(0.0, 1.0);
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
                        "Random" => PositionGenerator::Random,
                        "Center" => PositionGenerator::Center,
                        "UniformCircle" => PositionGenerator::UniformCircle,
                        "CenteredCircle" => PositionGenerator::CenteredCircle,
                        "Ring" => PositionGenerator::Ring,
                        "RainbowRing" => PositionGenerator::RainbowRing,
                        "ColorBattle" => PositionGenerator::ColorBattle,
                        "ColorWheel" => PositionGenerator::ColorWheel,
                        "Line" => PositionGenerator::Line,
                        "Spiral" => PositionGenerator::Spiral,
                        "RainbowSpiral" => PositionGenerator::RainbowSpiral,
                        _ => PositionGenerator::Random,
                    };
                    self.state.position_generator = generator;
                    // Regenerate particles with new position generator
                    self.initialize_particles_gpu(device, queue)?;
                }
            }
            "type_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Radial" => super::settings::TypeGenerator::Radial,
                        "Polar" => super::settings::TypeGenerator::Polar,
                        "StripesH" => super::settings::TypeGenerator::StripesH,
                        "StripesV" => super::settings::TypeGenerator::StripesV,
                        "Random" => super::settings::TypeGenerator::Random,
                        "LineH" => super::settings::TypeGenerator::LineH,
                        "LineV" => super::settings::TypeGenerator::LineV,
                        "Spiral" => super::settings::TypeGenerator::Spiral,
                        "Dithered" => super::settings::TypeGenerator::Dithered,
                        "WavyLineH" => super::settings::TypeGenerator::WavyLineH,
                        "WavyLineV" => super::settings::TypeGenerator::WavyLineV,
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
                        "LUT" => ColorMode::Lut,
                        _ => ColorMode::Lut,
                    };
                    // Update LUT with new color mode
                    let current_lut_name = self.state.current_lut_name.clone();
                    let lut_reversed = self.state.lut_reversed;
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
                    let color_mode = self.state.color_mode;
                    let lut_reversed = self.state.lut_reversed;
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
                    let color_mode = self.state.color_mode;
                    let current_lut_name = self.state.current_lut_name.clone();
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
        serde_json::json!({
            "particle_count": self.state.particle_count,
            "species_count": self.settings.species_count,
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
            "current_lut_name": self.state.current_lut_name,
            "lut_reversed": self.state.lut_reversed,
            "color_mode": self.state.color_mode,
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Determine cursor mode based on mouse_button
        let cursor_mode = if mouse_button == 0 {
            1 // left click = attract
        } else if mouse_button == 2 {
            2 // right click = repel
        } else {
            0 // middle click or other = no interaction
        };

        // Store coordinates directly - conversion is handled in the manager
        let sim_x = world_x;
        let sim_y = world_y;

        // Store cursor values in the model
        self.cursor_active_mode = cursor_mode;
        self.cursor_world_x = sim_x;
        self.cursor_world_y = sim_y;

        tracing::debug!(
            world_x = sim_x,
            world_y = sim_y,
            cursor_mode = cursor_mode,
            cursor_mode_name = match cursor_mode {
                0 => "inactive",
                1 => "attract",
                2 => "repel",
                _ => "unknown",
            },
            cursor_size = self.state.cursor_size,
            cursor_strength = self.state.cursor_strength,
            scaled_strength = self.state.cursor_strength * self.settings.max_force * 10.0,
            sim_width = self.width,
            sim_height = self.height,
            "Mouse interaction updated"
        );

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
            sim_params.cursor_strength =
                self.state.cursor_strength * self.settings.max_force * 10.0;
        }

        // Upload to GPU immediately
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Turn off cursor interaction
        self.cursor_active_mode = 0;
        self.cursor_world_x = 0.0;
        self.cursor_world_y = 0.0;

        tracing::debug!("ParticleLife mouse release: cursor interaction disabled");

        // Update sim params immediately with cursor disabled
        let mut sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );

        // Override with cursor values (disabled)
        sim_params.cursor_x = 0.0;
        sim_params.cursor_y = 0.0;
        sim_params.cursor_active = 0;
        sim_params.cursor_strength = 0.0;

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

    fn apply_settings(
        &mut self,
        settings: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Deserialize the settings and apply them using update_setting for each field
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            // Apply each setting individually to ensure proper handling
            if let Ok(species_count) = serde_json::to_value(new_settings.species_count) {
                self.update_setting("species_count", species_count, device, queue)?;
            }
            if let Ok(force_matrix) = serde_json::to_value(new_settings.force_matrix) {
                self.update_setting("force_matrix", force_matrix, device, queue)?;
            }
            if let Ok(max_force) = serde_json::to_value(new_settings.max_force) {
                self.update_setting("max_force", max_force, device, queue)?;
            }
            if let Ok(min_distance) = serde_json::to_value(new_settings.min_distance) {
                self.update_setting("min_distance", min_distance, device, queue)?;
            }
            if let Ok(max_distance) = serde_json::to_value(new_settings.max_distance) {
                self.update_setting("max_distance", max_distance, device, queue)?;
            }
            if let Ok(friction) = serde_json::to_value(new_settings.friction) {
                self.update_setting("friction", friction, device, queue)?;
            }
            if let Ok(force_beta) = serde_json::to_value(new_settings.force_beta) {
                self.update_setting("force_beta", force_beta, device, queue)?;
            }
            if let Ok(brownian_motion) = serde_json::to_value(new_settings.brownian_motion) {
                self.update_setting("brownian_motion", brownian_motion, device, queue)?;
            }
            if let Ok(wrap_edges) = serde_json::to_value(new_settings.wrap_edges) {
                self.update_setting("wrap_edges", wrap_edges, device, queue)?;
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
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.state.random_seed as u64);
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
