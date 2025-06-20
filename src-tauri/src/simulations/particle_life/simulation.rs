use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};
use serde_json::Value;
use crate::error::{SimulationError, SimulationResult};

use super::settings::Settings;
use super::shaders;
use crate::simulations::shared::{LutManager, camera::Camera};
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
pub struct LJParams {
    pub epsilon: f32,  // Potential well depth (attraction strength)
    pub sigma: f32,    // Distance where potential is zero
    pub _pad1: f32,
    pub _pad2: f32,
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
    pub time_step: f32,
    pub wrap_edges: u32,
    pub width: f32,
    pub height: f32,
    pub random_seed: u32,
    pub repulsion_min_distance: f32,
    pub repulsion_medium_distance: f32,
    pub repulsion_extreme_strength: f32,
    pub repulsion_linear_strength: f32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

impl SimParams {
    pub fn new(width: u32, height: u32, particle_count: u32, settings: &Settings) -> Self {
        Self {
            particle_count,
            species_count: settings.species_count,
            max_force: settings.max_force,
            min_distance: settings.min_distance,
            max_distance: settings.max_distance,
            friction: settings.friction,
            time_step: settings.time_step,
            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            width: width as f32,
            height: height as f32,
            random_seed: settings.random_seed,
            repulsion_min_distance: settings.repulsion_min_distance,
            repulsion_medium_distance: settings.repulsion_medium_distance,
            repulsion_extreme_strength: settings.repulsion_extreme_strength,
            repulsion_linear_strength: settings.repulsion_linear_strength,
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
}

impl State {
    pub fn new(particle_count: usize, species_count: u32, width: u32, height: u32, random_seed: u32) -> Self {
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
        }
    }
    
    pub fn reset(&mut self, _species_count: u32, _width: u32, _height: u32, _random_seed: u32) {
        // No longer used - particles are initialized on GPU
    }
}

/// Particle Life simulation model
#[derive(Debug)]
pub struct ParticleLifeModel {
    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub lj_params_buffer: wgpu::Buffer,
    pub lut_buffer: Arc<wgpu::Buffer>,
    
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
    
    // Dimensions
    pub width: u32,
    pub height: u32,
    
    // Camera for viewport control
    pub camera: Camera,
}

impl ParticleLifeModel {
    /// Convert force matrix values to Lennard-Jones parameters
    fn force_matrix_to_lj_params(force_matrix: &[Vec<f32>]) -> Vec<LJParams> {
        let mut lj_params = Vec::new();
        
        for i in 0..force_matrix.len() {
            for j in 0..force_matrix[i].len() {
                let force = force_matrix[i][j];
                
                // Convert force value [-1, 1] to Lennard-Jones parameters
                // Original scaling for stronger dynamics
                let epsilon = if force > 0.0 {
                    force.abs() * 2.0  // Scale attraction strength
                } else {
                    0.1  // Minimal attraction for repulsive interactions
                };
                
                let sigma = if force < 0.0 {
                    10.0 + force.abs() * 20.0  // Larger sigma for repulsive forces
                } else {
                    5.0 + (1.0 - force) * 5.0  // Smaller sigma for attractive forces
                };
                
                lj_params.push(LJParams {
                    epsilon,
                    sigma,
                    _pad1: 0.0,
                    _pad2: 0.0,
                });
            }
        }
        
        lj_params
    }
    
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        _adapter_info: &wgpu::AdapterInfo,
        particle_count: usize,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> SimulationResult<Self> {
        let width = surface_config.width;
        let height = surface_config.height;
        
        // Create initial state (but don't initialize particles on CPU)
        let state = State {
            particle_count,
            particles: vec![], // Empty - will be initialized on GPU
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
        let sim_params = SimParams::new(width, height, particle_count as u32, &settings);
        let sim_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sim Params Buffer"),
            contents: bytemuck::cast_slice(&[sim_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create Lennard-Jones parameters buffer
        let lj_params_data = Self::force_matrix_to_lj_params(&settings.force_matrix);
        let lj_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LJ Params Buffer"),
            contents: bytemuck::cast_slice(&lj_params_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        
        // Create LUT buffer
        let default_lut = lut_manager.get("viridis").unwrap_or_else(|_| {
            // Fallback to default LUT
            lut_manager.get_default()
        });
        let lut_data_u32 = default_lut.to_u32_buffer();
        let lut_buffer = Arc::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        }));
        
        // Create compute shader and pipeline
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::COMPUTE_SHADER.into()),
        });
        
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        
        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
                    resource: lj_params_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create initialization compute shader and pipeline
        let init_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Init Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::INIT_SHADER.into()),
        });
        
        let init_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            random_seed: settings.random_seed,
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
        let force_update_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Update Params Buffer"),
            contents: bytemuck::cast_slice(&[force_update_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let force_update_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        
        let force_update_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Force Update Pipeline Layout"),
            bind_group_layouts: &[&force_update_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let force_update_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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
                    resource: lj_params_buffer.as_entire_binding(),
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
            random_seed: settings.random_seed,
            min_force: -1.0,
            max_force: 1.0,
        };
        let force_randomize_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Randomize Params Buffer"),
            contents: bytemuck::cast_slice(&[force_randomize_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let force_randomize_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        
        let force_randomize_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Force Randomize Pipeline Layout"),
            bind_group_layouts: &[&force_randomize_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let force_randomize_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
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
                    resource: lj_params_buffer.as_entire_binding(),
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
        let render_bind_group_layout_particles = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let lut_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            ],
        });
        
        let render_bind_group_layout = lut_bind_group_layout.clone();
        
        // Camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[
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
            ],
        });
        
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Life Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout_particles, &render_bind_group_layout, &camera_bind_group_layout],
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
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
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
            ],
        });
        
        // Create camera
        let camera = Camera::new(device, width as f32, height as f32)?;
        
        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer().as_entire_binding(),
                },
            ],
        });
        
        let result = Self {
            particle_buffer,
            sim_params_buffer,
            lj_params_buffer,
            lut_buffer,
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
            current_lut_name: "viridis".to_string(),
            lut_reversed: false,
            width,
            height,
            camera,
        };
        
        // Initialize particles on GPU
        result.initialize_particles_gpu(device, queue)?;
        
        Ok(result)
    }
    
    fn update_sim_params(&self, queue: &Arc<Queue>) {
        let sim_params = SimParams::new(self.width, self.height, self.settings.particle_count, &self.settings);
        queue.write_buffer(&self.sim_params_buffer, 0, bytemuck::cast_slice(&[sim_params]));
    }
    
    
    fn initialize_particles_gpu(&self, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
        // Update init params with current settings
        let init_params = InitParams {
            start_index: 0,
            spawn_count: self.settings.particle_count,
            species_count: self.settings.species_count,
            width: self.width as f32,
            height: self.height as f32,
            random_seed: self.settings.random_seed,
            _pad1: 0,
            _pad2: 0,
        };
        
        queue.write_buffer(&self.init_params_buffer, 0, bytemuck::cast_slice(&[init_params]));
        
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
            let num_workgroups = (self.settings.particle_count + workgroup_size - 1) / workgroup_size;
            init_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
    
    pub fn reset_particles_gpu(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
        // Update random seed for reset
        use rand::Rng;
        let mut rng = rand::rng();
        self.settings.random_seed = rng.random();
        
        // Update sim params with new random seed
        self.update_sim_params(queue);
        
        // Re-initialize particles on GPU
        self.initialize_particles_gpu(device, queue)
    }
    
    pub fn update_force_element_gpu(&self, device: &Arc<Device>, queue: &Arc<Queue>, species_a: u32, species_b: u32, new_force: f32) -> SimulationResult<()> {
        let update_params = ForceUpdateParams {
            species_a,
            species_b,
            new_force,
            species_count: self.settings.species_count,
        };
        
        // Update the uniform buffer
        queue.write_buffer(&self.force_update_params_buffer, 0, bytemuck::cast_slice(&[update_params]));
        
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
    
    pub fn randomize_force_matrix_gpu(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
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
        queue.write_buffer(&self.force_randomize_params_buffer, 0, bytemuck::cast_slice(&[randomize_params]));
        
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
            let num_workgroups = (self.settings.species_count + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, num_workgroups, 1);
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        
        // Update the CPU-side force matrix by regenerating it with the same seed
        // This keeps the CPU and GPU versions in sync for UI display
        self.sync_force_matrix_from_gpu(new_seed);
        
        Ok(())
    }
    
    fn sync_force_matrix_from_gpu(&mut self, seed: u32) {
        // Regenerate the same random values on CPU for UI synchronization
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        
        for i in 0..self.settings.species_count {
            for j in 0..self.settings.species_count {
                let _index = i * self.settings.species_count + j;
                let random_val: f32 = rng.random();
                let force_range = 1.0 - (-1.0);
                let new_force = -1.0 + random_val * force_range;
                self.settings.force_matrix[i as usize][j as usize] = new_force;
            }
        }
    }
    
    fn recreate_bind_groups_with_lj_params(&mut self, device: &Arc<Device>) {
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
                    resource: self.lj_params_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Recreate force update bind group
        self.force_update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Force Update Bind Group"),
            layout: &self.force_update_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lj_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.force_update_params_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Recreate force randomize bind group
        self.force_randomize_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Force Randomize Bind Group"),
            layout: &self.force_randomize_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lj_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.force_randomize_params_buffer.as_entire_binding(),
                },
            ],
        });
    }
    
}

impl Simulation for ParticleLifeModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Update GPU buffers with current state
        self.update_sim_params(queue);
        
        // Update camera
        self.camera.upload_to_gpu(queue);
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Render Encoder"),
        });
        
        // Compute pass - update particle positions
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Particle Life Compute Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            
            let workgroup_size = 64;
            let num_workgroups = (self.state.particle_count as u32 + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        
        // Render pass - draw particles
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Particle Life Render Pass"),
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
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            render_pass.set_bind_group(2, &self.camera_bind_group, &[]);
            
            // Draw instanced particles (6 vertices per particle for quad)
            render_pass.draw(0..6, 0..self.state.particle_count as u32);
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
    
    fn resize(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.width = new_config.width;
        self.height = new_config.height;
        
        // Update camera viewport
        self.camera.resize(new_config.width as f32, new_config.height as f32);
        
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
                    self.settings.set_species_count(count as u32);
                    
                    // Recreate LJ params buffer with new size
                    let lj_params_data = Self::force_matrix_to_lj_params(&self.settings.force_matrix);
                    self.lj_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("LJ Params Buffer"),
                        contents: bytemuck::cast_slice(&lj_params_data),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    });
                    
                    // Recreate bind groups that use this buffer
                    self.recreate_bind_groups_with_lj_params(device);
                    
                    // Respawn all particles to ensure proper species distribution
                    self.initialize_particles_gpu(device, queue)?;
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
                                    if i < self.settings.force_matrix.len() && j < self.settings.force_matrix[i].len() {
                                        self.settings.force_matrix[i][j] = force_val as f32;
                                    }
                                }
                            }
                        }
                    }
                    // Update entire LJ params buffer since we changed the force matrix
                    let lj_params_data = Self::force_matrix_to_lj_params(&self.settings.force_matrix);
                    queue.write_buffer(&self.lj_params_buffer, 0, bytemuck::cast_slice(&lj_params_data));
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
            "time_step" => {
                if let Some(dt) = value.as_f64() {
                    self.settings.time_step = dt as f32;
                }
            }
            "wrap_edges" => {
                if let Some(wrap) = value.as_bool() {
                    self.settings.wrap_edges = wrap;
                }
            }
            "repulsion_min_distance" => {
                if let Some(dist) = value.as_f64() {
                    self.settings.repulsion_min_distance = dist as f32;
                }
            }
            "repulsion_medium_distance" => {
                if let Some(dist) = value.as_f64() {
                    self.settings.repulsion_medium_distance = dist as f32;
                }
            }
            "repulsion_extreme_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.settings.repulsion_extreme_strength = strength as f32;
                }
            }
            "repulsion_linear_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.settings.repulsion_linear_strength = strength as f32;
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
        })
    }
    
    fn handle_mouse_interaction(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _is_seeding: bool,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // For now, just add some attraction at the mouse position
        // This could be extended to add/remove particles or create forces
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
    
    fn apply_settings(&mut self, settings: Value, queue: &Arc<Queue>) -> SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            self.settings = new_settings;
            // Upload the entire LJ params when applying new settings
            let lj_params_data = Self::force_matrix_to_lj_params(&self.settings.force_matrix);
            queue.write_buffer(&self.lj_params_buffer, 0, bytemuck::cast_slice(&lj_params_data));
        }
        Ok(())
    }
    
    fn reset_runtime_state(&mut self, queue: &Arc<Queue>) -> SimulationResult<()> {
        // Update random seed for reset
        use rand::Rng;
        let mut rng = rand::rng();
        self.settings.random_seed = rng.random();
        
        // Update sim params with new random seed
        self.update_sim_params(queue);
        
        // TODO: Re-initialize particles on GPU - need device reference in trait method
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
        // Randomize force matrix on GPU for better performance
        self.randomize_force_matrix_gpu(device, queue)?;
        
        // Randomize other settings on CPU (these are small values)
        // Note: particle_count and species_count are preserved
        use rand::Rng;
        let mut rng = rand::rng();
        self.settings.max_force = rng.random_range(50.0..200.0);
        self.settings.min_distance = rng.random_range(2.0..10.0);
        self.settings.max_distance = rng.random_range(50.0..150.0);
        self.settings.friction = rng.random_range(0.9..0.999);
        self.settings.time_step = rng.random_range(0.01..0.03);
        self.settings.wrap_edges = rng.random_bool(0.7);
        self.settings.random_seed = rng.random();
        
        // Explicitly preserve particle_count and species_count - they should not be randomized
        
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
        let old_count = self.settings.particle_count;
        
        if new_count == old_count {
            return Ok(());
        }
        
        // Update settings
        self.settings.particle_count = new_count;
        
        // Check buffer size limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let new_particle_buffer_size = (new_count as usize * std::mem::size_of::<Particle>()) as u64;
        
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
        
        // Recreate bind groups with new buffer
        self.recreate_bind_groups(device)?;
        
        // Respawn all particles with new count
        self.initialize_particles_gpu(device, queue)?;
        
        // Update simulation parameters with new particle count
        self.update_sim_params(queue);
        
        tracing::info!("Updated particle count from {} to {} (respawned all)", old_count, new_count);
        Ok(())
    }


    /// Recreate bind groups after particle buffer changes
    fn recreate_bind_groups(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
        // Recreate compute bind group
        self.compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Compute Bind Group"),
            layout: &self.compute_bind_group_layout,
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
                    resource: self.lj_params_buffer.as_entire_binding(),
                },
            ],
        });
        
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
        
        Ok(())
    }
}