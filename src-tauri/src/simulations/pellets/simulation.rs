//! # Pellets Simulation Implementation
//!
//! The core engine that brings the Pellets particle physics simulation to life.
//! This module orchestrates the interaction between user input, GPU computation,
//! and visual rendering to create a responsive and engaging simulation experience.
//!
//! ## Simulation Philosophy
//!
//! The simulation balances computational performance with user interactivity.
//! By leveraging GPU parallelization for physics calculations while keeping
//! user interface responsive on the CPU, it creates a seamless experience
//! where users can explore and experiment with complex particle behaviors.
//!
//! ## System Architecture
//!
//! The simulation uses a hybrid architecture that separates concerns between
//! configuration management, real-time computation, and user interaction.
//! This design enables both high-performance physics simulation and
//! intuitive user control over the system's behavior.

use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::shaders::{
    BACKGROUND_RENDER_SHADER, PARTICLE_FRAGMENT_RENDER_SHADER, PARTICLE_RENDER_SHADER,
    RENDER_INFINITE_SHADER,
};
use super::{settings::Settings, state::State};
use crate::simulations::shared::{LutManager, camera::Camera, AverageColorResources};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct Particle {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub mass: f32,
    pub radius: f32,
    pub clump_id: u32,
    pub density: f32,
    pub grabbed: u32,
    pub _pad0: u32,
    pub previous_position: [f32; 2], // Kept for compatibility
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PhysicsParams {
    pub mouse_position: [f32; 2],
    pub mouse_velocity: [f32; 2], // Mouse velocity in world units per second
    pub particle_count: u32,
    pub gravitational_constant: f32,
    pub energy_damping: f32,
    pub collision_damping: f32,
    pub dt: f32,
    pub gravity_softening: f32,
    pub interaction_radius: f32,
    pub mouse_pressed: u32,
    pub mouse_mode: u32,
    pub cursor_size: f32,
    pub cursor_strength: f32,
    pub particle_size: f32, // Pre-calculated particle size for consistent collision and rendering
    pub aspect_ratio: f32,  // Screen aspect ratio for collision correction
    pub long_range_gravity_strength: f32, // Controls orbital motion strength
    pub density_damping_enabled: u32, // Whether to apply density-based velocity damping
    pub overlap_resolution_strength: f32, // Controls how aggressively overlapping particles are separated
    pub _padding: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DensityParams {
    pub particle_count: u32,
    pub density_radius: f32,
    pub coloring_mode: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RenderParams {
    pub particle_size: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub coloring_mode: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BackgroundParams {
    pub background_type: u32,
    pub density_texture_resolution: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PostEffectParams {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub gamma: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GridParams {
    pub particle_count: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub cell_size: f32,
    pub world_width: f32,  // 2.0 for [-1,1] space
    pub world_height: f32, // 2.0 for [-1,1] space
    pub _pad1: u32,
    pub _pad2: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GridCell {
    pub particle_count: u32,
    pub particle_indices: [u32; 32], // Max 32 particles per cell
}

// GPU-based physics implementation - no Rapier needed

pub struct PelletsModel {
    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub physics_params_buffer: wgpu::Buffer,
    pub density_params_buffer: wgpu::Buffer,
    pub render_params_buffer: wgpu::Buffer,
    pub background_params_buffer: wgpu::Buffer,
    pub post_effect_params_buffer: wgpu::Buffer,
    pub lut_buffer: wgpu::Buffer,
    pub background_color_buffer: wgpu::Buffer,

    // Spatial partitioning resources
    pub grid_buffer: wgpu::Buffer,
    pub grid_params_buffer: wgpu::Buffer,

    // Compute pipelines
    pub physics_compute_pipeline: wgpu::ComputePipeline,
    pub density_compute_pipeline: wgpu::ComputePipeline,
    pub grid_clear_pipeline: wgpu::ComputePipeline,
    pub grid_populate_pipeline: wgpu::ComputePipeline,

    // Compute bind groups
    pub physics_bind_group: wgpu::BindGroup,
    pub density_bind_group: wgpu::BindGroup,
    pub grid_clear_bind_group: wgpu::BindGroup,
    pub grid_populate_bind_group: wgpu::BindGroup,

    // Legacy render pipeline (kept for compatibility)
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_bind_group: wgpu::BindGroup,
    pub background_pipeline: wgpu::RenderPipeline,
    pub background_bind_group: wgpu::BindGroup,

    // Offscreen rendering resources
    pub display_texture: wgpu::Texture,
    pub display_view: wgpu::TextureView,
    pub display_sampler: wgpu::Sampler,
    pub post_effect_texture: wgpu::Texture,
    pub post_effect_view: wgpu::TextureView,
    pub density_texture: wgpu::Texture,
    pub density_view: wgpu::TextureView,

    // Offscreen render pipelines
    pub background_render_pipeline: wgpu::RenderPipeline,
    pub background_render_bind_group: wgpu::BindGroup,
    pub particle_render_pipeline: wgpu::RenderPipeline,
    pub particle_render_bind_group: wgpu::BindGroup,
    pub post_effect_pipeline: wgpu::RenderPipeline,
    pub post_effect_bind_group: wgpu::BindGroup,
    pub render_infinite_pipeline: wgpu::RenderPipeline,
    pub render_infinite_bind_group: wgpu::BindGroup,

    // Average color calculation for infinite rendering
    pub average_color_resources: AverageColorResources,

    // Camera bind group
    pub camera_bind_group: wgpu::BindGroup,

    // Particle data (simulation runtime, not UI state)
    pub particles: Vec<Particle>,

    // Simulation state and settings
    pub settings: Settings,
    pub state: State,
    pub camera: Camera,
    pub lut_manager: Arc<LutManager>,

    // Surface configuration
    pub surface_config: SurfaceConfiguration,

    // Performance optimization
    pub frame_count: u64,
    pub density_update_frequency: u64,

    // Grid parameters
    pub grid_width: u32,
    pub grid_height: u32,
    pub cell_size: f32,
}

impl PelletsModel {
    pub fn new(
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize particles
        let particles = Self::initialize_particles(settings.particle_count, &settings);

        // Create buffers
        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets Particle Buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )
        .map_err(|e| format!("Failed to create camera: {}", e))?;

        // Camera is already set up for [-1,1] x [-1,1] world space, no adjustment needed
        let camera_position = camera.position;
        let camera_zoom = camera.zoom;

        // Create initial state without particles (particles are stored separately)
        let state = State {
            camera_position,
            camera_zoom,
            ..Default::default()
        };

        // Initialize LUT
        let mut lut = lut_manager
            .get(&state.current_lut_name)
            .map_err(|e| format!("Failed to load LUT '{}': {}", state.current_lut_name, e))?;
        if state.lut_reversed {
            lut = lut.reversed();
        }
        let lut_data_u32 = lut.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create background color buffer (black by default)
        let background_color_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Background Color Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // Black background
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let render_params = RenderParams {
            particle_size: settings.particle_size,
            screen_width: (surface_config.width * 2) as f32,
            screen_height: (surface_config.height * 2) as f32,
            coloring_mode: match settings.coloring_mode.as_str() {
                "velocity" => 1,
                "random" => 2,
                _ => 0, // Default to density
            },
        };

        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets Render Params Buffer"),
            contents: bytemuck::cast_slice(&[render_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let background_params = BackgroundParams {
            background_type: if settings.background_type == "white" {
                1
            } else {
                0
            },
            density_texture_resolution: 512, // Default texture resolution
        };

        let background_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Background Params Buffer"),
                contents: bytemuck::cast_slice(&[background_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create post-effect parameters buffer
        let post_effect_params = PostEffectParams {
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
        };

        let post_effect_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Post Effect Params Buffer"),
                contents: bytemuck::cast_slice(&[post_effect_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create physics params buffer
        let physics_params = PhysicsParams {
            mouse_position: [0.0, 0.0],
            mouse_velocity: [0.0, 0.0],
            particle_count: settings.particle_count,
            gravitational_constant: settings.gravitational_constant,
            energy_damping: settings.energy_damping,
            collision_damping: settings.collision_damping,
            dt: 1.0 / 60.0, // 60 FPS target
            gravity_softening: settings.gravity_softening,
            interaction_radius: 0.5, // Limit interaction range for performance
            mouse_pressed: 0,
            mouse_mode: 0,
            cursor_size: state.cursor_size,
            cursor_strength: state.cursor_strength,
            particle_size: settings.particle_size,
            aspect_ratio: surface_config.width as f32 / surface_config.height as f32,
            long_range_gravity_strength: 0.0, // Initialize new field
            density_damping_enabled: settings.density_damping_enabled as u32,
            overlap_resolution_strength: settings.overlap_resolution_strength,
            _padding: 0,
        };

        let physics_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets Physics Params Buffer"),
            contents: bytemuck::cast_slice(&[physics_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create density params buffer
        let density_params = DensityParams {
            particle_count: settings.particle_count,
            density_radius: settings.density_radius,
            coloring_mode: match settings.coloring_mode.as_str() {
                "velocity" => 1,
                "random" => 2,
                _ => 0, // Default to density
            },
            _padding: 0,
        };

        let density_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets Density Params Buffer"),
            contents: bytemuck::cast_slice(&[density_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create render pipeline
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::PARTICLE_RENDER_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(
                super::shaders::PARTICLE_FRAGMENT_RENDER_SHADER.into(),
            ),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Render Bind Group Layout"),
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
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pellets Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pellets Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
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
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create compute shaders
        let physics_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Physics Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::PHYSICS_COMPUTE_SHADER.into()),
        });

        let density_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Density Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::DENSITY_COMPUTE_SHADER.into()),
        });

        // Create compute bind group layouts
        let physics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Physics Bind Group Layout"),
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

        // Create compute pipelines
        let physics_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pellets Physics Compute Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Physics Pipeline Layout"),
                        bind_group_layouts: &[&physics_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &physics_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        // Create density bind group layout (separate from physics layout)
        let density_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Density Bind Group Layout"),
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

        let density_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pellets Density Compute Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Density Pipeline Layout"),
                        bind_group_layouts: &[&density_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &density_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        let density_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Density Bind Group"),
            layout: &density_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: density_params_buffer.as_entire_binding(),
                },
            ],
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        // Create background pipeline
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Background Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::BACKGROUND_RENDER_SHADER.into()),
        });

        // Create dummy texture for density visualization
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Dummy Density Texture"),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let background_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Background Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pellets Background Pipeline Layout"),
                bind_group_layouts: &[&background_bind_group_layout],
                push_constant_ranges: &[],
            });

        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pellets Background Pipeline"),
            layout: Some(&background_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &background_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &background_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Background Bind Group"),
            layout: &background_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: background_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });

        // Initialize spatial partitioning grid
        let cell_size = 0.1; // Each cell is 0.1 world units
        let grid_width = (2.0 / cell_size) as u32; // 20 cells across [-1,1]
        let grid_height = (2.0 / cell_size) as u32; // 20 cells across [-1,1]
        let total_cells = grid_width * grid_height;

        // Initialize grid with empty cells
        let grid_cells = vec![
            GridCell {
                particle_count: 0,
                particle_indices: [0; 32],
            };
            total_cells as usize
        ];

        // Create grid buffer
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets Grid Buffer"),
            contents: bytemuck::cast_slice(&grid_cells),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create grid parameters
        let grid_params = GridParams {
            particle_count: settings.particle_count,
            grid_width,
            grid_height,
            cell_size,
            world_width: 2.0,
            world_height: 2.0,
            _pad1: 0,
            _pad2: 0,
        };

        let grid_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pellets Grid Params Buffer"),
            contents: bytemuck::cast_slice(&[grid_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create grid compute shaders
        let grid_clear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Grid Clear Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::GRID_CLEAR_SHADER.into()),
        });

        let grid_populate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pellets Grid Populate Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::GRID_POPULATE_SHADER.into()),
        });

        // Create grid bind group layouts
        let grid_clear_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Grid Clear Bind Group Layout"),
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

        let grid_populate_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Grid Populate Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
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

        // Create grid bind groups
        let grid_clear_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Grid Clear Bind Group"),
            layout: &grid_clear_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grid_params_buffer.as_entire_binding(),
                },
            ],
        });

        let grid_populate_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Grid Populate Bind Group"),
            layout: &grid_populate_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create grid compute pipelines
        let grid_clear_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pellets Grid Clear Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Grid Clear Pipeline Layout"),
                        bind_group_layouts: &[&grid_clear_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &grid_clear_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        let grid_populate_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pellets Grid Populate Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Grid Populate Pipeline Layout"),
                        bind_group_layouts: &[&grid_populate_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &grid_populate_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        // Create physics bind group (after grid buffers are created)
        let physics_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Physics Bind Group"),
            layout: &physics_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: physics_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: grid_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create offscreen rendering resources at 2x resolution for better particle quality
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Display Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width * 2,
                height: surface_config.height * 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Pellets Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create density texture for background visualization
        let density_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Density Texture"),
            size: wgpu::Extent3d {
                width: 512, // density_texture_resolution
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let density_view = density_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create post-effect texture
        let post_effect_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Post Effect Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let post_effect_view =
            post_effect_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Camera Bind Group Layout"),
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
        });

        // Create offscreen render pipelines
        let background_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Pellets Background Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Background Render Pipeline Layout"),
                        bind_group_layouts: &[&device.create_bind_group_layout(
                            &wgpu::BindGroupLayoutDescriptor {
                                label: Some("Pellets Background Render Bind Group Layout"),
                                entries: &[
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 0,
                                        visibility: wgpu::ShaderStages::FRAGMENT,
                                        ty: wgpu::BindingType::Buffer {
                                            ty: wgpu::BufferBindingType::Uniform,
                                            has_dynamic_offset: false,
                                            min_binding_size: None,
                                        },
                                        count: None,
                                    },
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 1,
                                        visibility: wgpu::ShaderStages::FRAGMENT,
                                        ty: wgpu::BindingType::Texture {
                                            sample_type: wgpu::TextureSampleType::Float {
                                                filterable: false,
                                            },
                                            view_dimension: wgpu::TextureViewDimension::D2,
                                            multisampled: false,
                                        },
                                        count: None,
                                    },
                                ],
                            },
                        )],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Pellets Background Render Vertex Shader"),
                        source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
                    }),
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Pellets Background Render Fragment Shader"),
                        source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
                    }),
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let background_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Background Render Bind Group"),
            layout: &background_render_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: background_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&density_view),
                },
            ],
        });

        let particle_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Pellets Particle Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Particle Render Pipeline Layout"),
                        bind_group_layouts: &[&device.create_bind_group_layout(
                            &wgpu::BindGroupLayoutDescriptor {
                                label: Some("Pellets Particle Render Bind Group Layout"),
                                entries: &[
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 0,
                                        visibility: wgpu::ShaderStages::VERTEX,
                                        ty: wgpu::BindingType::Buffer {
                                            ty: wgpu::BufferBindingType::Storage {
                                                read_only: true,
                                            },
                                            has_dynamic_offset: false,
                                            min_binding_size: None,
                                        },
                                        count: None,
                                    },
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 1,
                                        visibility: wgpu::ShaderStages::VERTEX
                                            | wgpu::ShaderStages::FRAGMENT,
                                        ty: wgpu::BindingType::Buffer {
                                            ty: wgpu::BufferBindingType::Uniform,
                                            has_dynamic_offset: false,
                                            min_binding_size: None,
                                        },
                                        count: None,
                                    },
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 2,
                                        visibility: wgpu::ShaderStages::VERTEX
                                            | wgpu::ShaderStages::FRAGMENT,
                                        ty: wgpu::BindingType::Buffer {
                                            ty: wgpu::BufferBindingType::Storage {
                                                read_only: true,
                                            },
                                            has_dynamic_offset: false,
                                            min_binding_size: None,
                                        },
                                        count: None,
                                    },
                                ],
                            },
                        )],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Pellets Particle Render Vertex Shader"),
                        source: wgpu::ShaderSource::Wgsl(PARTICLE_RENDER_SHADER.into()),
                    }),
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Pellets Particle Render Fragment Shader"),
                        source: wgpu::ShaderSource::Wgsl(PARTICLE_FRAGMENT_RENDER_SHADER.into()),
                    }),
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Particle Render Bind Group"),
            layout: &particle_render_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        let post_effect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pellets Post Effect Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pellets Post Effect Pipeline Layout"),
                    bind_group_layouts: &[&device.create_bind_group_layout(
                        &wgpu::BindGroupLayoutDescriptor {
                            label: Some("Pellets Post Effect Bind Group Layout"),
                            entries: &[
                                wgpu::BindGroupLayoutEntry {
                                    binding: 0,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Buffer {
                                        ty: wgpu::BufferBindingType::Uniform,
                                        has_dynamic_offset: false,
                                        min_binding_size: None,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 1,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Texture {
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: true,
                                        },
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                        multisampled: false,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 2,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Sampler(
                                        wgpu::SamplerBindingType::Filtering,
                                    ),
                                    count: None,
                                },
                            ],
                        },
                    )],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Pellets Post Effect Vertex Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        super::shaders::POST_EFFECT_VERTEX_SHADER.into(),
                    ),
                }),
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Pellets Post Effect Fragment Shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        super::shaders::POST_EFFECT_FRAGMENT_SHADER.into(),
                    ),
                }),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let post_effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Post Effect Bind Group"),
            layout: &post_effect_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: post_effect_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&display_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&display_sampler),
                },
            ],
        });

        let render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Pellets Render Infinite Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Pellets Render Infinite Pipeline Layout"),
                        bind_group_layouts: &[
                            &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                                label: Some("Pellets Render Infinite Bind Group Layout"),
                                entries: &[
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 0,
                                        visibility: wgpu::ShaderStages::FRAGMENT,
                                        ty: wgpu::BindingType::Texture {
                                            sample_type: wgpu::TextureSampleType::Float {
                                                filterable: true,
                                            },
                                            view_dimension: wgpu::TextureViewDimension::D2,
                                            multisampled: false,
                                        },
                                        count: None,
                                    },
                                    wgpu::BindGroupLayoutEntry {
                                        binding: 1,
                                        visibility: wgpu::ShaderStages::FRAGMENT,
                                        ty: wgpu::BindingType::Sampler(
                                            wgpu::SamplerBindingType::Filtering,
                                        ),
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
                            }),
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Pellets Render Infinite Vertex Shader"),
                        source: wgpu::ShaderSource::Wgsl(RENDER_INFINITE_SHADER.into()),
                    }),
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Pellets Render Infinite Fragment Shader"),
                        source: wgpu::ShaderSource::Wgsl(RENDER_INFINITE_SHADER.into()),
                    }),
                    entry_point: Some("fs_main_texture"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Render Infinite Bind Group"),
            layout: &render_infinite_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&post_effect_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&display_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &background_color_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        // Create average color calculation resources
        let average_color_resources = AverageColorResources::new(
            device,
            &post_effect_texture,
            &post_effect_view,
            "Pellets",
        );

        Ok(PelletsModel {
            particle_buffer,
            physics_params_buffer,
            density_params_buffer,
            render_params_buffer,
            background_params_buffer,
            post_effect_params_buffer,
            lut_buffer,
            background_color_buffer,
            grid_buffer,
            grid_params_buffer,
            physics_compute_pipeline,
            density_compute_pipeline,
            grid_clear_pipeline,
            grid_populate_pipeline,
            physics_bind_group,
            density_bind_group,
            grid_clear_bind_group,
            grid_populate_bind_group,
            render_pipeline,
            render_bind_group,
            background_pipeline,
            background_bind_group,

            // Offscreen rendering resources
            display_texture,
            display_view,
            display_sampler,
            post_effect_texture,
            post_effect_view,
            density_texture,
            density_view,
            background_render_pipeline,
            background_render_bind_group,
            particle_render_pipeline,
            particle_render_bind_group,
            post_effect_pipeline,
            post_effect_bind_group,
            render_infinite_pipeline,
            render_infinite_bind_group,
            average_color_resources,
            camera_bind_group,

            particles,
            settings: settings.clone(),
            state,
            camera,
            lut_manager: Arc::new(lut_manager.clone()),
            surface_config: surface_config.clone(),
            frame_count: 0,
            density_update_frequency: 3, // Update density every 3 frames for performance
            grid_width,
            grid_height,
            cell_size,
        })
    }

    fn initialize_particles(count: u32, settings: &Settings) -> Vec<Particle> {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut particles = Vec::with_capacity(count as usize);

        if count == 1 {
            let mass = 1.0;
            let radius_particle = settings.particle_size;
            // Give single particle some initial motion
            let velocity = [0.1, 0.1];
            let dt = 0.016;
            let prev_x = 0.0 - velocity[0] * dt;
            let prev_y = 0.0 - velocity[1] * dt;

            particles.push(Particle {
                position: [0.0, 0.0],
                velocity,
                mass,
                radius: radius_particle,
                clump_id: 0,
                density: 0.0,
                grabbed: 0,
                _pad0: 0,
                previous_position: [prev_x, prev_y],
            });
        } else if count == 2 {
            // Place two particles side-by-side near the centre so both are visible
            let mass = 1.0;
            let radius_particle = settings.particle_size;
            let offset = 0.02; // small horizontal separation in world units

            // Give particles some initial motion
            let velocity1 = [0.1, 0.05];
            let velocity2 = [-0.1, -0.05];
            let dt = 0.016;
            let prev_x1 = -offset - velocity1[0] * dt;
            let prev_y1 = 0.0 - velocity1[1] * dt;
            let prev_x2 = offset - velocity2[0] * dt;
            let prev_y2 = 0.0 - velocity2[1] * dt;

            particles.push(Particle {
                position: [-offset, 0.0],
                velocity: velocity1,
                mass,
                radius: radius_particle,
                clump_id: 0,
                density: 0.0,
                grabbed: 0,
                _pad0: 0,
                previous_position: [prev_x1, prev_y1],
            });

            particles.push(Particle {
                position: [offset, 0.0],
                velocity: velocity2,
                mass,
                radius: radius_particle,
                clump_id: 0,
                density: 0.0,
                grabbed: 0,
                _pad0: 0,
                previous_position: [prev_x2, prev_y2],
            });
        } else {
            // Simple random placement for all particles
            for _ in 0..count {
                // Random position within bounds - use full world space for infinity rendering
                let x = rng.random_range(-1.0..1.0);
                let y = rng.random_range(-1.0..1.0);

                // Uniform mass and radius for basic collision behaviour
                let mass = 1.0;
                let radius_particle = settings.particle_size;

                // Random velocities
                let angle = rng.random_range(0.0..2.0 * std::f32::consts::PI);
                let speed =
                    rng.random_range(settings.initial_velocity_min..settings.initial_velocity_max);
                let velocity = [angle.cos() * speed, angle.sin() * speed];

                // For Verlet integration: previous_position = current_position - velocity * dt
                let dt = 0.016; // Match the dt used in simulation
                let prev_x = x - velocity[0] * dt;
                let prev_y = y - velocity[1] * dt;

                particles.push(Particle {
                    position: [x, y],
                    velocity,
                    mass,
                    radius: radius_particle,
                    clump_id: 0, // All initial particles belong to clump 0
                    density: 0.0,
                    grabbed: 0,
                    _pad0: 0,
                    previous_position: [prev_x, prev_y],
                });
            }
        }

        particles
    }

    pub fn step_physics(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.frame_count += 1;

        // Update physics parameters
        self.update_physics_params(queue);

        // Dispatch physics compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pellets Physics Compute Encoder"),
        });

        // Step 1: Clear the spatial grid
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pellets Grid Clear Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.grid_clear_pipeline);
            compute_pass.set_bind_group(0, &self.grid_clear_bind_group, &[]);

            let total_cells = self.grid_width * self.grid_height;
            let workgroup_size = 64;
            let num_workgroups = total_cells.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Step 2: Populate the spatial grid with particle positions
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pellets Grid Populate Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.grid_populate_pipeline);
            compute_pass.set_bind_group(0, &self.grid_populate_bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups = self.settings.particle_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Step 3: Run physics simulation using spatial grid
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pellets Physics Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.physics_compute_pipeline);
            compute_pass.set_bind_group(0, &self.physics_bind_group, &[]);

            // Dispatch with optimal workgroup size
            let workgroup_size = 64;
            let num_workgroups = self.settings.particle_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Update density every few frames for performance
        if self.frame_count % self.density_update_frequency == 0 {
            self.update_density_params(queue);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Pellets Density Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.density_compute_pipeline);
            compute_pass.set_bind_group(0, &self.density_bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups = self.settings.particle_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn update_physics_params(&mut self, queue: &Arc<Queue>) {
        // Apply velocity decay when mouse is not pressed (after throwing)
        if !self.state.mouse_pressed {
            // Decay velocity over time to prevent persistent throwing
            let decay_factor = 0.95; // Adjust this for faster/slower decay
            self.state.mouse_velocity = [
                self.state.mouse_velocity[0] * decay_factor,
                self.state.mouse_velocity[1] * decay_factor,
            ];
        }

        let physics_params = PhysicsParams {
            mouse_position: self.state.mouse_position,
            mouse_velocity: self.state.mouse_velocity,
            particle_count: self.settings.particle_count,
            gravitational_constant: self.settings.gravitational_constant,
            energy_damping: self.settings.energy_damping,
            collision_damping: self.settings.collision_damping,
            dt: 1.0 / 60.0,
            gravity_softening: self.settings.gravity_softening,
            interaction_radius: 0.5,
            mouse_pressed: if self.state.mouse_pressed { 1 } else { 0 },
            mouse_mode: self.state.mouse_mode,
            cursor_size: self.state.cursor_size,
            cursor_strength: self.state.cursor_strength,
            particle_size: self.settings.particle_size,
            aspect_ratio: self.surface_config.width as f32 / self.surface_config.height as f32,
            long_range_gravity_strength: self.settings.long_range_gravity_strength,
            density_damping_enabled: if self.settings.density_damping_enabled {
                1
            } else {
                0
            },
            overlap_resolution_strength: self.settings.overlap_resolution_strength,
            _padding: 0,
        };

        queue.write_buffer(
            &self.physics_params_buffer,
            0,
            bytemuck::cast_slice(&[physics_params]),
        );

        // Update grid parameters
        let grid_params = GridParams {
            particle_count: self.settings.particle_count,
            grid_width: self.grid_width,
            grid_height: self.grid_height,
            cell_size: self.cell_size,
            world_width: 2.0,
            world_height: 2.0,
            _pad1: 0,
            _pad2: 0,
        };

        queue.write_buffer(
            &self.grid_params_buffer,
            0,
            bytemuck::cast_slice(&[grid_params]),
        );
    }

    fn update_density_params(&self, queue: &Arc<Queue>) {
        let density_params = DensityParams {
            particle_count: self.settings.particle_count,
            density_radius: self.settings.density_radius,
            coloring_mode: match self.settings.coloring_mode.as_str() {
                "velocity" => 1,
                "random" => 2,
                _ => 0, // Default to density
            },
            _padding: 0,
        };

        queue.write_buffer(
            &self.density_params_buffer,
            0,
            bytemuck::cast_slice(&[density_params]),
        );
    }

    fn update_particle_count(
        &mut self,
        new_count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let current_count = self.particles.len() as u32;
        tracing::debug!(
            "Updating particle count: {} -> {}",
            current_count,
            new_count
        );

        if new_count > current_count {
            // Add particles
            let particles_to_add = new_count - current_count;
            tracing::debug!("Adding {} particles", particles_to_add);
            let new_particles = Self::initialize_particles(particles_to_add, &self.settings);
            self.particles.extend(new_particles);
        } else if new_count < current_count {
            // Remove particles
            let particles_to_remove = current_count - new_count;
            tracing::debug!("Removing {} particles", particles_to_remove);
            self.particles.truncate(new_count as usize);
        }

        // Update settings
        self.settings.particle_count = new_count;
        tracing::debug!("Updated settings.particle_count to {}", new_count);

        // Check if we need to recreate the buffer (if it's too small)
        let required_buffer_size = self.particles.len() * std::mem::size_of::<Particle>();
        if self.particle_buffer.size() < required_buffer_size as u64 {
            tracing::debug!(
                "Recreating particle buffer: current_size={}, required_size={}",
                self.particle_buffer.size(),
                required_buffer_size
            );
            // Recreate the particle buffer with the new size
            self.particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Particle Buffer"),
                contents: bytemuck::cast_slice(&self.particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            // Recreate the bind groups since the buffer changed
            tracing::debug!("Recreating bind groups after buffer change");
            self.recreate_bind_groups(device)?;
        } else {
            // Update existing GPU buffer
            tracing::debug!(
                "Updating existing GPU buffer with {} particles",
                self.particles.len()
            );
            queue.write_buffer(
                &self.particle_buffer,
                0,
                bytemuck::cast_slice(&self.particles),
            );
        }

        tracing::debug!(
            "Particle count update complete: {} particles",
            self.particles.len()
        );
        Ok(())
    }

    fn recreate_bind_groups(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
        // Recreate physics bind group layout (with grid buffers)
        let physics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Physics Bind Group Layout"),
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

        // Create density bind group layout (separate from physics layout)
        let density_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Density Bind Group Layout"),
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

        // Create grid populate bind group layout
        let grid_populate_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Grid Populate Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
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

        // Recreate physics compute bind group
        self.physics_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Physics Bind Group"),
            layout: &physics_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.physics_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.grid_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Recreate density compute bind group
        self.density_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Density Bind Group"),
            layout: &density_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.density_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Recreate grid populate bind group
        self.grid_populate_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Grid Populate Bind Group"),
            layout: &grid_populate_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.grid_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Recreate render bind group
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Pellets Render Bind Group Layout"),
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
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        self.render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.lut_buffer.as_entire_binding(),
                },
            ],
        });

        // Recreate particle render bind group (for offscreen rendering)
        self.particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Particle Render Bind Group"),
            layout: &self.particle_render_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.lut_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(())
    }

    fn update_camera_uniform(&self, queue: &Arc<Queue>) {
        // Camera updates its own buffer
        self.camera.upload_to_gpu(queue);
    }

    fn calculate_tile_count(&self) -> u32 {
        // At zoom 1.0, we need at least 5x5 tiles
        // As zoom decreases (zooming out), we need more tiles
        // Each tile covers 2.0 world units, so we need enough tiles to cover the visible area
        let visible_world_size = 2.0 / self.camera.zoom; // World size visible on screen
        let tiles_needed = (visible_world_size / 2.0).ceil() as u32 + 6; // +6 for extra padding at extreme zoom levels
        let min_tiles = if self.camera.zoom < 0.1 { 7 } else { 5 }; // More tiles needed at extreme zoom out
        // Allow more tiles for proper infinite tiling, but cap at reasonable limit
        tiles_needed.max(min_tiles).min(1024) // Cap at 200x200 for performance (matches Flow simulation)
    }

    fn update_render_params(&self, queue: &Arc<Queue>) {
        let render_params = RenderParams {
            particle_size: self.settings.particle_size,
            screen_width: (self.surface_config.width * 2) as f32,
            screen_height: (self.surface_config.height * 2) as f32,
            coloring_mode: match self.settings.coloring_mode.as_str() {
                "velocity" => 1,
                "random" => 2,
                _ => 0, // Default to density
            },
        };

        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );
    }

    fn update_background_params(&self, queue: &Arc<Queue>) {
        let background_params = BackgroundParams {
            background_type: if self.settings.background_type == "white" {
                1
            } else {
                0
            },
            density_texture_resolution: 512, // Default texture resolution
        };

        queue.write_buffer(
            &self.background_params_buffer,
            0,
            bytemuck::cast_slice(&[background_params]),
        );
    }

    fn update_post_effect_params(&self, queue: &Arc<Queue>) {
        let post_effect_params = PostEffectParams {
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
        };

        queue.write_buffer(
            &self.post_effect_params_buffer,
            0,
            bytemuck::cast_slice(&[post_effect_params]),
        );
    }

    fn update_background_color(&self, queue: &Arc<Queue>) {
        // Get the background color from the LUT (first color in the LUT)
        let lut = self
            .lut_manager
            .get(&self.state.current_lut_name)
            .unwrap_or_else(|_| self.lut_manager.get_default());

        let colors = lut.get_colors(1); // Get just the first color
        if let Some(color) = colors.first() {
            let background_color = [
                color[0] as f32,
                color[1] as f32,
                color[2] as f32,
                color[3] as f32,
            ];
            queue.write_buffer(
                &self.background_color_buffer,
                0,
                bytemuck::cast_slice(&[background_color]),
            );
        }
    }

    fn calculate_average_color(&self, device: &Arc<Device>, queue: &Arc<Queue>) {
        self.average_color_resources.calculate_average_color(device, queue, &self.post_effect_texture);
        
        // Wait for the GPU work to complete
        device.poll(wgpu::Maintain::Wait);
        
        // Read the result and update the background color buffer
        if let Some(average_color) = self.average_color_resources.get_average_color() {
            queue.write_buffer(
                &self.background_color_buffer,
                0,
                bytemuck::cast_slice(&[average_color]),
            );
        }
        // Unmap the staging buffer after reading
        self.average_color_resources.unmap_staging_buffer();
    }

    fn update_particle_radii(&mut self, queue: &Arc<Queue>) {
        // Update all existing particles' radii to match the new particle size setting
        for particle in &mut self.particles {
            particle.radius = self.settings.particle_size;
        }

        // Update the GPU buffer with the new particle data
        queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&self.particles),
        );
    }

    pub fn update_lut(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
        lut_name: &str,
        lut_reversed: bool,
    ) -> SimulationResult<()> {
        let mut lut =
            self.lut_manager
                .get(lut_name)
                .map_err(|e| SimulationError::InvalidSetting {
                    setting_name: "current_lut".to_string(),
                    message: format!("Failed to load LUT '{}': {}", lut_name, e),
                })?;

        if lut_reversed {
            lut = lut.reversed();
        }

        let lut_data_u32 = lut.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));

        self.state.current_lut_name = lut_name.to_string();
        self.state.lut_reversed = lut_reversed;

        Ok(())
    }

    // GPU compute shaders handle all physics interactions
}

impl std::fmt::Debug for PelletsModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PelletsModel")
            .field("particles", &self.particles.len()) // Show count instead of full data
            .field("settings", &self.settings)
            .field("state", &self.state)
            .field("frame_count", &self.frame_count)
            .finish()
    }
}

impl crate::simulations::traits::Simulation for PelletsModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Step GPU physics simulation
        self.step_physics(device, queue)?;

        // Update camera with smoothing
        self.camera.update(0.016); // Assume 60 FPS for now

        // Update uniforms
        self.update_camera_uniform(queue);
        self.update_render_params(queue);
        self.update_background_params(queue);
        self.update_post_effect_params(queue);
        self.update_background_color(queue);

        // 1. Render background to display texture (offscreen)
        let mut offscreen_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pellets Offscreen Encoder"),
            });
        {
            let mut render_pass =
                offscreen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Pellets Background Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            // Render background
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_render_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(std::iter::once(offscreen_encoder.finish()));

        // 2. Render particles to display texture (offscreen)
        let mut particle_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pellets Particle Encoder"),
        });
        {
            let mut render_pass = particle_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pellets Particle Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render particles (9 instances per particle for wrapping)
            render_pass.set_pipeline(&self.particle_render_pipeline);
            render_pass.set_bind_group(0, &self.particle_render_bind_group, &[]);
            render_pass.draw(0..6, 0..(self.particles.len() * 9) as u32);
        }
        queue.submit(std::iter::once(particle_encoder.finish()));

        // 3. Render post effects to post-effect texture (offscreen)
        let mut post_effect_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pellets Post Effect Encoder"),
            });
        {
            let mut render_pass =
                post_effect_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Pellets Post Effect Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.post_effect_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            // Render post effects
            render_pass.set_pipeline(&self.post_effect_pipeline);
            render_pass.set_bind_group(0, &self.post_effect_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(std::iter::once(post_effect_encoder.finish()));

        // 3. Calculate average color from the post-effect texture
        self.calculate_average_color(device, queue);

        // 4. Render post-effect texture to surface with infinite tiling
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pellets Infinite Surface Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pellets Infinite Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &self.render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
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
        // For static rendering, just render without updating physics
        // Update camera with smoothing
        self.camera.update(0.016); // Assume 60 FPS for now

        self.update_camera_uniform(queue);
        self.update_render_params(queue);
        self.update_background_params(queue);
        self.update_post_effect_params(queue);
        self.update_background_color(queue);

        // 1. Render background to display texture (offscreen)
        let mut offscreen_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pellets Static Offscreen Encoder"),
            });
        {
            let mut render_pass =
                offscreen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Pellets Static Background Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            // Render background
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_render_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(std::iter::once(offscreen_encoder.finish()));

        // 2. Render particles to display texture (offscreen)
        let mut particle_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pellets Static Particle Encoder"),
        });
        {
            let mut render_pass = particle_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pellets Static Particle Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render particles (9 instances per particle for wrapping)
            render_pass.set_pipeline(&self.particle_render_pipeline);
            render_pass.set_bind_group(0, &self.particle_render_bind_group, &[]);
            render_pass.draw(0..6, 0..(self.particles.len() * 9) as u32);
        }
        queue.submit(std::iter::once(particle_encoder.finish()));

        // 3. Render post effects to post-effect texture (offscreen)
        let mut post_effect_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pellets Static Post Effect Encoder"),
            });
        {
            let mut render_pass =
                post_effect_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Pellets Static Post Effect Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.post_effect_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            // Render post effects
            render_pass.set_pipeline(&self.post_effect_pipeline);
            render_pass.set_bind_group(0, &self.post_effect_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(std::iter::once(post_effect_encoder.finish()));

        // 4. Render post-effect texture to surface with infinite tiling
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pellets Static Infinite Surface Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pellets Static Infinite Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &self.render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }
        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.surface_config = new_config.clone();
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        // Recreate offscreen rendering resources for new dimensions at 2x resolution
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Display Texture"),
            size: wgpu::Extent3d {
                width: new_config.width * 2,
                height: new_config.height * 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Pellets Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Update the display texture and view
        self.display_texture = display_texture;
        self.display_view = display_view;
        self.display_sampler = display_sampler;

        // Recreate post-effect texture for new dimensions
        let post_effect_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Post Effect Texture"),
            size: wgpu::Extent3d {
                width: new_config.width,
                height: new_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let post_effect_view =
            post_effect_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.post_effect_texture = post_effect_texture;
        self.post_effect_view = post_effect_view;

        // Recreate density texture for new dimensions
        let density_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pellets Density Texture"),
            size: wgpu::Extent3d {
                width: 512, // density_texture_resolution
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let density_view = density_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.density_texture = density_texture;
        self.density_view = density_view;

        // Update render params to reflect new screen dimensions
        self.update_render_params(queue);
        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        tracing::debug!(
            "Pellets::update_setting called with setting_name: '{}', value: {:?}",
            setting_name,
            value
        );

        match setting_name {
            "particle_count" => {
                tracing::debug!("Processing particle_count setting");
                if let Some(count) = value.as_u64() {
                    let new_count = count as u32;
                    tracing::debug!(
                        "Particle count change: {} -> {}",
                        self.particles.len(),
                        new_count
                    );
                    if new_count != self.particles.len() as u32 {
                        tracing::debug!("Calling update_particle_count");
                        self.update_particle_count(new_count, device, queue)?;
                        tracing::debug!("update_particle_count completed successfully");
                    } else {
                        tracing::debug!("Particle count unchanged, skipping update");
                    }
                } else {
                    tracing::warn!("Invalid particle_count value: {:?}", value);
                }
            }
            "gravitational_constant" => {
                if let Some(constant) = value.as_f64() {
                    tracing::debug!(
                        "Updating gravitational_constant from {} to {}",
                        self.settings.gravitational_constant,
                        constant
                    );
                    self.settings.gravitational_constant = constant as f32;
                    // GPU compute shaders will use the updated value
                }
            }

            "particle_size" => {
                if let Some(size) = value.as_f64() {
                    self.settings.particle_size = size as f32;
                    // Update all particle radii and GPU buffers immediately
                    self.update_particle_radii(queue);
                    self.update_render_params(queue);
                }
            }

            "energy_damping" => {
                if let Some(damping) = value.as_f64() {
                    self.settings.energy_damping = damping as f32;
                    // GPU compute shaders will use the updated value
                }
            }
            "gravity_softening" => {
                if let Some(softening) = value.as_f64() {
                    self.settings.gravity_softening = softening as f32;
                }
            }
            "density_radius" => {
                if let Some(radius) = value.as_f64() {
                    self.settings.density_radius = radius as f32;
                }
            }
            "coloring_mode" => {
                if let Some(mode) = value.as_str() {
                    self.settings.coloring_mode = mode.to_string();
                }
            }
            "initial_velocity_max" => {
                if let Some(velocity) = value.as_f64() {
                    self.settings.initial_velocity_max = velocity as f32;
                }
            }
            "initial_velocity_min" => {
                if let Some(velocity) = value.as_f64() {
                    self.settings.initial_velocity_min = velocity as f32;
                }
            }
            "collision_damping" => {
                if let Some(damping) = value.as_f64() {
                    self.settings.collision_damping = damping as f32;
                    // GPU compute shaders will use the updated value
                }
            }
            "long_range_gravity_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.settings.long_range_gravity_strength = strength as f32;
                }
            }
            "density_damping_enabled" => {
                if let Some(enabled) = value.as_bool() {
                    self.settings.density_damping_enabled = enabled;
                }
            }
            "overlap_resolution_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.settings.overlap_resolution_strength = (strength as f32).clamp(0.0, 1.0);
                }
            }
            "random_seed" => {
                if let Some(seed) = value.as_u64() {
                    self.settings.random_seed = seed as u32;
                }
            }
            "background_type" => {
                if let Some(bg_type) = value.as_str() {
                    self.settings.background_type = bg_type.to_string();
                    // Update GPU buffer immediately
                    self.update_background_params(queue);
                }
            }
            "currentLut" => {
                if let Some(lut_name) = value.as_str() {
                    self.update_lut(device, queue, lut_name, self.state.lut_reversed)?;
                }
            }
            "lut_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    let lut_name = self.state.current_lut_name.clone();
                    self.update_lut(device, queue, &lut_name, reversed)?;
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.state.cursor_size = (size as f32).clamp(0.05, 1.0);
                }
            }
            "cursor_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.cursor_strength = (strength as f32).clamp(0.0, 1.0);
                }
            }
            _ => {
                return Err(SimulationError::InvalidSetting {
                    setting_name: setting_name.to_string(),
                    message: "Unknown setting".to_string(),
                });
            }
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }

    fn get_state(&self) -> Value {
        serde_json::to_value(&self.state).unwrap_or(Value::Null)
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Clamp world coordinates to valid bounds
        let clamped_x = world_x.clamp(-1.0, 1.0);
        let clamped_y = (-world_y).clamp(-1.0, 1.0); // Fix Y-axis inversion

        // Calculate mouse velocity based on time difference
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let time_delta = current_time - self.state.last_mouse_time;

        // Only update velocity if we have a meaningful time difference (avoid division by very small numbers)
        if time_delta > 0.001 && self.state.last_mouse_time > 0.0 {
            // Calculate velocity in world units per second
            let previous_position = self.state.mouse_position;
            let position_delta = [
                clamped_x - previous_position[0],
                clamped_y - previous_position[1],
            ];

            let new_velocity = [
                position_delta[0] / time_delta as f32,
                position_delta[1] / time_delta as f32,
            ];

            // Apply velocity smoothing (exponential moving average)
            let smoothing_factor = 0.7; // Adjust this for more/less smoothing
            self.state.mouse_velocity = [
                self.state.mouse_velocity[0] * (1.0 - smoothing_factor)
                    + new_velocity[0] * smoothing_factor,
                self.state.mouse_velocity[1] * (1.0 - smoothing_factor)
                    + new_velocity[1] * smoothing_factor,
            ];
        }

        // Encode mouse button into mode: 0 none, 1 left(attraction)
        let mode = match mouse_button {
            0 => 1u32, // Left click for attraction
            _ => 0u32, // Other buttons do nothing
        };

        self.state.mouse_pressed = true;
        self.state.mouse_mode = mode;
        self.state.mouse_position = [clamped_x, clamped_y];
        self.state.last_mouse_time = current_time;

        // Clear grabbed particles list when starting new interaction
        self.state.grabbed_particles.clear();

        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Keep the current velocity for throwing, don't clear it immediately
        // The shader will use this velocity when releasing particles
        self.state.mouse_pressed = false;
        self.state.mouse_mode = 0;

        // Clear the grabbed particles list
        self.state.grabbed_particles.clear();

        // Start velocity decay after a short delay
        // This will be handled in the physics step

        Ok(())
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
        self.state.camera_position = self.camera.position;
    }

    fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
        self.state.camera_zoom = self.camera.zoom;
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
        self.state.camera_position = self.camera.position;
        self.state.camera_zoom = self.camera.zoom;
    }

    fn reset_camera(&mut self) {
        self.camera.reset();
        self.state.reset_camera();
    }

    fn get_camera_state(&self) -> Value {
        serde_json::json!({
            "position": self.state.camera_position,
            "zoom": self.state.camera_zoom,
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
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Reinitialize particles
        self.particles = Self::initialize_particles(self.settings.particle_count, &self.settings);

        // Check if we need to recreate the buffer (if it's too small)
        let required_buffer_size = self.particles.len() * std::mem::size_of::<Particle>();
        if self.particle_buffer.size() < required_buffer_size as u64 {
            // Recreate the particle buffer with the new size
            self.particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Particle Buffer"),
                contents: bytemuck::cast_slice(&self.particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            // Recreate the bind groups since the buffer changed
            self.recreate_bind_groups(device)?;
        } else {
            // Update existing GPU buffer
            queue.write_buffer(
                &self.particle_buffer,
                0,
                bytemuck::cast_slice(&self.particles),
            );
        }

        // Reset camera
        self.camera.reset();

        // Reset state
        self.state.reset();

        // Reset frame counter
        self.frame_count = 0;

        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.state.gui_visible = !self.state.gui_visible;
        self.state.gui_visible
    }

    fn is_gui_visible(&self) -> bool {
        self.state.gui_visible
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.settings.randomize();
        Ok(())
    }
}
