use crate::error::SimulationResult;
use crate::simulations::primordial_particles::state::{BackgroundColorMode, ForegroundColorMode};
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::{
    ColorSchemeManager, ComputePipelineBuilder,
    camera::Camera,
    ping_pong_buffers::PingPongBuffers,
    ping_pong_render_textures::PingPongRenderTextures,
    post_processing::{PostProcessingResources, PostProcessingState},
};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::Settings;
use super::shaders;
use super::state::State;
use crate::simulations::traits::Simulation;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimParams {
    // Mouse interaction parameters (vec2 fields first for alignment)
    pub mouse_position: [f32; 2],
    pub mouse_velocity: [f32; 2],

    pub alpha: f32,    // Fixed rotation parameter (radians)
    pub beta: f32,     // Proportional rotation parameter
    pub velocity: f32, // Constant velocity
    pub radius: f32,   // Interaction radius

    pub dt: f32,         // Time step
    pub width: f32,      // World width
    pub height: f32,     // World height
    pub wrap_edges: u32, // 1 if wrapping edges, 0 otherwise

    pub particle_count: u32,
    pub mouse_pressed: u32,
    pub mouse_mode: u32,
    pub cursor_size: f32,
    pub cursor_strength: f32,
    pub aspect_ratio: f32,
    pub _pad1: f32,
    pub _pad0: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct InitParams {
    pub start_index: u32,
    pub spawn_count: u32,
    pub width: f32,
    pub height: f32,

    pub random_seed: u32,
    pub position_generator: u32, // 0=Random, 1=Center, 2=UniformCircle, etc.
    pub _pad1: u32,
    pub _pad2: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct DensityParams {
    pub particle_count: u32,
    pub density_radius: f32,
    pub coloring_mode: u32, // 0=Random, 1=Density, 2=Heading, 3=Velocity
    pub _padding: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RenderParams {
    pub particle_size: f32,
    pub foreground_color_mode: u32, // 0=Random, 1=Density, 2=Heading, 3=Velocity
    pub _pad0: f32,
    pub _pad1: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct BackgroundParams {
    pub background_color: [f32; 4], // RGBA color values
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            // Mouse interaction defaults (vec2 fields first)
            mouse_position: [0.0, 0.0],
            mouse_velocity: [0.0, 0.0],

            alpha: 180.0_f32.to_radians(),
            beta: 0.1,
            velocity: 0.5,
            radius: 0.01,
            dt: 0.016,
            width: 1.0,
            height: 1.0,
            wrap_edges: 1,
            particle_count: 10000,
            mouse_pressed: 0,
            mouse_mode: 0,
            cursor_size: 0.20,
            cursor_strength: 1.0,
            aspect_ratio: 1.0,
            _pad1: 0.0,
            _pad0: 0.0,
        }
    }
}

/// Primordial Particles simulation model
#[derive(Debug)]
pub struct PrimordialParticlesModel {
    // GPU resources
    pub particle_buffers: PingPongBuffers,
    pub sim_params_buffer: wgpu::Buffer,
    pub init_params_buffer: wgpu::Buffer,
    pub render_params_buffer: wgpu::Buffer,
    pub background_params_buffer: wgpu::Buffer,
    pub density_params_buffer: wgpu::Buffer,

    // Compute pipeline
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_bind_group1: wgpu::BindGroup,
    pub compute_bind_group2: wgpu::BindGroup,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,

    // Density calculation pipeline
    pub density_pipeline: wgpu::ComputePipeline,
    pub density_bind_group_a: wgpu::BindGroup,
    pub density_bind_group_b: wgpu::BindGroup,
    pub density_bind_group_layout: wgpu::BindGroupLayout,

    // Initialization pipeline
    pub init_pipeline: wgpu::ComputePipeline,
    pub init_bind_group_a: wgpu::BindGroup,
    pub init_bind_group_b: wgpu::BindGroup,
    pub init_bind_group_layout: wgpu::BindGroupLayout,

    // Render pipeline
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_bind_group_a: wgpu::BindGroup,
    pub render_bind_group_b: wgpu::BindGroup,

    // Background render pipeline
    pub background_render_pipeline: wgpu::RenderPipeline,
    pub background_render_bind_group_layout: wgpu::BindGroupLayout,
    pub background_render_bind_group: wgpu::BindGroup,

    // Camera
    pub camera: Camera,
    pub camera_bind_group: wgpu::BindGroup,

    // Post-processing
    pub post_processing: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,

    // Color scheme management
    pub color_scheme_manager: ColorSchemeManager,
    pub lut_buffer: wgpu::Buffer,
    pub lut_bind_group: wgpu::BindGroup,
    pub lut_bind_group_layout: wgpu::BindGroupLayout,

    // Trail/trace infrastructure
    pub trail_textures: PingPongRenderTextures,

    // Fade pipeline for traces
    pub fade_pipeline: wgpu::RenderPipeline,
    pub fade_bind_group_layout: wgpu::BindGroupLayout,
    pub fade_bind_group: wgpu::BindGroup,
    pub fade_uniforms_buffer: wgpu::Buffer,

    // Blit pipeline to copy trail texture to surface
    pub blit_pipeline: wgpu::RenderPipeline,
    pub blit_bind_group_layout: wgpu::BindGroupLayout,
    pub blit_bind_group: wgpu::BindGroup,
    pub blit_sampler: wgpu::Sampler,

    // Offscreen display for infinite tiling when traces are disabled
    pub display_texture: wgpu::Texture,
    pub display_view: wgpu::TextureView,
    pub display_sampler: wgpu::Sampler,

    // Infinite tiling pipeline (shared shader)
    pub render_infinite_pipeline: wgpu::RenderPipeline,
    pub infinite_render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_infinite_display_bind_group: wgpu::BindGroup,
    pub texture_render_params_buffer: wgpu::Buffer,

    // Settings and state
    pub settings: Settings,
    pub state: State,
}

impl PrimordialParticlesModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: &Settings,
        state: &State,
    ) -> SimulationResult<Self> {
        // Create ping-pong particle buffers and initialize both
        let particle_stride = std::mem::size_of::<super::state::Particle>() as u64;
        let particle_buffer_size = particle_stride * state.particle_count as u64;
        let particle_buffers = PingPongBuffers::new(
            device,
            particle_buffer_size,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            "Primordial Particles Buffer",
        );
        queue.write_buffer(
            particle_buffers.current_buffer(),
            0,
            bytemuck::cast_slice(&state.particles),
        );
        queue.write_buffer(
            particle_buffers.inactive_buffer(),
            0,
            bytemuck::cast_slice(&state.particles),
        );

        // Create simulation parameters buffer
        let sim_params = SimParams {
            // Mouse interaction defaults (vec2 fields first)
            mouse_position: [0.0, 0.0],
            mouse_velocity: [0.0, 0.0],

            alpha: settings.alpha_radians(),
            beta: settings.beta,
            velocity: settings.velocity,
            radius: settings.radius,
            dt: state.dt,
            width: 2.0,  // [-1,1] world space has width of 2
            height: 2.0, // [-1,1] world space has height of 2
            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            particle_count: state.particle_count,
            mouse_pressed: 0,
            mouse_mode: 0,
            cursor_size: 0.20,
            cursor_strength: 1.0,
            aspect_ratio: surface_config.width as f32 / surface_config.height as f32,
            _pad1: 0.0,
            _pad0: 0.0,
        };

        let sim_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Sim Params Buffer",
            &[sim_params],
        );

        // Create initialization parameters buffer
        let init_params = InitParams {
            start_index: 0,
            spawn_count: state.particle_count,
            width: surface_config.width as f32,
            height: surface_config.height as f32,
            random_seed: state.random_seed,
            position_generator: state.position_generator,
            _pad1: 0,
            _pad2: 0,
        };

        let init_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Init Params Buffer",
            &[init_params],
        );

        // Create render parameters buffer
        let render_params = RenderParams {
            particle_size: state.particle_size,
            foreground_color_mode: state.foreground_color_mode as u32,
            _pad0: 0.0,
            _pad1: 0.0,
        };

        let render_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Render Params Buffer",
            &[render_params],
        );

        // Create background parameters buffer
        let background_params = BackgroundParams {
            background_color: [0.0, 0.0, 0.0, 1.0], // Default black color
        };

        let background_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Background Params Buffer",
            &[background_params],
        );

        // Create density parameters buffer
        let density_params = DensityParams {
            particle_count: state.particle_count,
            density_radius: settings.radius,
            coloring_mode: state.foreground_color_mode as u32,
            _padding: 0,
        };

        let density_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Density Params Buffer",
            &[density_params],
        );

        // Create compute bind group layout (particles_in, particles_out, params)
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Compute Bind Group Layout"),
                entries: &[
                    // binding 0: read-only input
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    // binding 1: read-write output
                    resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, false),
                    // binding 2: uniforms
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                ],
            });

        // Create compute bind groups for ping-pong
        let compute_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Compute Bind Group 1"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(2, &sim_params_buffer),
            ],
        });
        let compute_bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Compute Bind Group 2"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(2, &sim_params_buffer),
            ],
        });

        // Create shader modules
        let particle_update_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Update Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::PARTICLE_UPDATE_SHADER.into()),
        });

        let init_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Init Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::INIT_SHADER.into()),
        });

        let particle_render_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Render Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::PARTICLE_RENDER_SHADER.into()),
        });

        let background_render_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Background Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::BACKGROUND_RENDER_SHADER.into()),
        });

        let density_compute_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Density Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::DENSITY_COMPUTE_SHADER.into()),
        });

        // Create compute pipeline
        let compute_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(Arc::new(particle_update_module))
            .with_bind_group_layouts(vec![compute_bind_group_layout.clone()])
            .with_label("Primordial Particles Compute Pipeline".to_string())
            .build();

        // Create density calculation bind group layout
        let density_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Density Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
            });

        // Create density calculation bind groups for ping-pong
        let density_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Density Bind Group A"),
            layout: &density_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, &density_params_buffer),
            ],
        });
        let density_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Density Bind Group B"),
            layout: &density_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, &density_params_buffer),
            ],
        });

        // Create density calculation pipeline
        let density_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(Arc::new(density_compute_module))
            .with_bind_group_layouts(vec![density_bind_group_layout.clone()])
            .with_label("Primordial Particles Density Pipeline".to_string())
            .build();

        // Create initialization bind group layout
        let init_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Init Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
            });

        // Create initialization bind groups for both buffers
        let init_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Init Bind Group A"),
            layout: &init_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, &init_params_buffer),
            ],
        });
        let init_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Init Bind Group B"),
            layout: &init_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, &init_params_buffer),
            ],
        });

        // Initialize camera
        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )?;

        // Upload initial camera data to GPU
        camera.upload_to_gpu(queue);

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Camera Bind Group Layout"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                )],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, camera.buffer())],
        });

        // Create initialization pipeline
        let init_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(Arc::new(init_module))
            .with_bind_group_layouts(vec![init_bind_group_layout.clone()])
            .with_label("Primordial Particles Init Pipeline".to_string())
            .build();

        // Create LUT buffer and bind group for color schemes (needed for render bind groups)
        let color_scheme_manager = ColorSchemeManager::new();
        let default_lut = color_scheme_manager
            .get(&state.current_color_scheme)
            .unwrap_or_else(|_| color_scheme_manager.get_default());
        let lut_data_u32 = default_lut.to_u32_buffer();
        let lut_buffer = resource_helpers::create_storage_buffer_with_data(
            device,
            "Primordial Particles LUT Buffer",
            &lut_data_u32,
        );

        let lut_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles LUT Bind Group Layout"),
                entries: &[resource_helpers::storage_buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    true,
                )],
            });

        let lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles LUT Bind Group"),
            layout: &lut_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, &lut_buffer)],
        });

        // Create render bind group layout
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Render Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::VERTEX, true),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::VERTEX),
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::VERTEX),
                    resource_helpers::storage_buffer_entry(
                        3,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        true,
                    ),
                ],
            });

        // Create render bind groups for A/B
        let render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Render Bind Group A"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, camera.buffer()),
                resource_helpers::buffer_entry(2, &render_params_buffer),
                resource_helpers::buffer_entry(3, &lut_buffer),
            ],
        });
        let render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Render Bind Group B"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, camera.buffer()),
                resource_helpers::buffer_entry(2, &render_params_buffer),
                resource_helpers::buffer_entry(3, &lut_buffer),
            ],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primordial Particles Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Primordial Particles Render Pipeline Layout"),
                    bind_group_layouts: &[&render_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &particle_render_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_render_module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format, // Use surface format
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            cache: None,
        });

        // Create background render bind group layout
        let background_render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Background Render Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::VERTEX),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        // Create background render bind group
        let background_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Background Render Bind Group"),
            layout: &background_render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, camera.buffer()),
                resource_helpers::buffer_entry(1, &background_params_buffer),
            ],
        });

        // Create background render pipeline
        let background_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Primordial Particles Background Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Primordial Particles Background Render Pipeline Layout"),
                        bind_group_layouts: &[&background_render_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &background_render_module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &background_render_module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format, // Use surface format
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
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
                cache: None,
            });

        // Initialize post-processing
        let post_processing_resources = PostProcessingResources::new(device, surface_config)?;
        let post_processing = PostProcessingState::default();

        // Create offscreen display texture for non-trace rendering (Rgba8)
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Primordial Particles Display Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Texture render params buffer (filtering mode etc.) used by shared infinite_render.wgsl
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct TextureRenderParams {
            filtering_mode: u32,
            _pad: [u32; 3],
        }
        let texture_render_params = TextureRenderParams {
            filtering_mode: 1,
            _pad: [0, 0, 0],
        };
        let texture_render_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Texture Render Params",
            &[texture_render_params],
        );

        // Infinite tiling pipeline using shared shader
        let infinite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Infinite Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shared/infinite_render.wgsl").into()),
        });

        // Bind group layout for display texture + sampler + params
        let infinite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Primordial Particles Infinite BGL"),
            entries: &[
                resource_helpers::texture_entry(
                    0,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D2,
                ),
                resource_helpers::sampler_entry(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::SamplerBindingType::Filtering,
                ),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });

        // Create a sampler for infinite rendering
        let display_sampler = resource_helpers::create_linear_sampler(
            device,
            "Primordial Particles Display Sampler",
            wgpu::FilterMode::Linear,
        );

        let render_infinite_display_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Primordial Particles Infinite Display BG"),
                layout: &infinite_bgl,
                entries: &[
                    resource_helpers::texture_view_entry(0, &display_view),
                    resource_helpers::sampler_bind_entry(1, &display_sampler),
                    resource_helpers::buffer_entry(2, &texture_render_params_buffer),
                ],
            });

        // Pipeline layout uses texture bind group + camera
        let infinite_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Primordial Particles Infinite PL"),
            bind_group_layouts: &[&infinite_bgl, &camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Primordial Particles Infinite Pipeline"),
                layout: Some(&infinite_pl),
                vertex: wgpu::VertexState {
                    module: &infinite_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &infinite_shader,
                    entry_point: Some("fs_main_texture"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
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

        // Create trail textures for persistent trails
        let trail_textures = PingPongRenderTextures::new(
            device,
            surface_config.width,
            surface_config.height,
            surface_config.format,
            "Primordial Particles Trail Texture",
        );

        // Create fade shader modules
        let fade_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Fade Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_VERTEX_SHADER.into()),
        });

        let fade_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Fade Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_FRAGMENT_SHADER.into()),
        });

        // Create fade bind group layout
        let fade_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Fade Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::texture_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        2,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        // Create fade uniforms buffer
        let fade_uniforms = [0.01f32, 0.0f32, 0.0f32, 0.0f32]; // fade_amount, _pad1, _pad2, _pad3
        let fade_uniforms_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Primordial Particles Fade Uniforms Buffer",
            &fade_uniforms,
        );

        // Create fade pipeline
        let fade_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Primordial Particles Fade Pipeline Layout"),
            bind_group_layouts: &[&fade_bind_group_layout],
            push_constant_ranges: &[],
        });

        let fade_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primordial Particles Fade Pipeline"),
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
                    format: surface_config.format, // Use surface format
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
            cache: None,
        });

        // Create blit sampler
        let blit_sampler = resource_helpers::create_linear_sampler(
            device,
            "Primordial Particles Blit Sampler",
            wgpu::FilterMode::Linear,
        );

        // Create blit bind group layout
        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Primordial Particles Blit Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        // Create blit vertex shader (fullscreen triangle)
        let blit_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Blit Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                }

                @vertex
                fn main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
                    var positions = array<vec2<f32>, 3>(
                        vec2<f32>(-1.0, -1.0),
                        vec2<f32>(3.0, -1.0),
                        vec2<f32>(-1.0, 3.0)
                    );
                    
                    var uvs = array<vec2<f32>, 3>(
                        vec2<f32>(0.0, 1.0),
                        vec2<f32>(2.0, 1.0),
                        vec2<f32>(0.0, -1.0)
                    );
                    
                    var output: VertexOutput;
                    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
                    output.uv = uvs[vertex_index];
                    return output;
                }
                "#
                .into(),
            ),
        });

        // Create blit fragment shader
        let blit_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primordial Particles Blit Fragment Shader"),
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

        // Create blit pipeline
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Primordial Particles Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primordial Particles Blit Pipeline"),
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
                    format: surface_config.format, // Use surface format
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
            cache: None,
        });

        // Create fade bind group (initially uses current texture)
        let fade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Fade Bind Group"),
            layout: &fade_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &fade_uniforms_buffer),
                resource_helpers::texture_view_entry(1, trail_textures.current_view()),
                resource_helpers::sampler_bind_entry(2, &blit_sampler),
            ],
        });

        // Create blit bind group (initially uses current texture)
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Blit Bind Group"),
            layout: &blit_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, trail_textures.current_view()),
                resource_helpers::sampler_bind_entry(1, &blit_sampler),
            ],
        });

        let model = Self {
            particle_buffers,
            sim_params_buffer,
            init_params_buffer,
            render_params_buffer,
            background_params_buffer,
            density_params_buffer,
            compute_pipeline,
            compute_bind_group1,
            compute_bind_group2,
            compute_bind_group_layout,
            density_pipeline,
            density_bind_group_a,
            density_bind_group_b,
            density_bind_group_layout,
            init_pipeline,
            init_bind_group_a,
            init_bind_group_b,
            init_bind_group_layout,
            render_pipeline,
            render_bind_group_layout,
            render_bind_group_a,
            render_bind_group_b,
            background_render_pipeline,
            background_render_bind_group_layout,
            background_render_bind_group,
            camera,
            camera_bind_group,
            post_processing,
            post_processing_resources,
            color_scheme_manager,
            lut_buffer,
            lut_bind_group,
            lut_bind_group_layout,
            trail_textures,
            fade_pipeline,
            fade_bind_group_layout,
            fade_bind_group,
            fade_uniforms_buffer,
            blit_pipeline,
            blit_bind_group_layout,
            blit_bind_group,
            blit_sampler,
            display_texture,
            display_view,
            display_sampler,
            render_infinite_pipeline,
            infinite_render_bind_group_layout: infinite_bgl,
            render_infinite_display_bind_group,
            texture_render_params_buffer,
            settings: settings.clone(),
            state: (*state).clone(),
        };

        // Initialize with the current color scheme from state
        if let Ok(color_scheme) = model.color_scheme_manager.get(&state.current_color_scheme) {
            // Apply reversed flag if needed
            let color_scheme_data = if state.color_scheme_reversed {
                color_scheme.reversed()
            } else {
                color_scheme
            };
            // Update LUT buffer with the correct color scheme data
            let color_scheme_data_u32 = color_scheme_data.to_u32_buffer();
            queue.write_buffer(
                &model.lut_buffer,
                0,
                bytemuck::cast_slice(&color_scheme_data_u32),
            );

            // If background mode is ColorScheme, update the background color with the first color from the LUT
            if state.background_color_mode == BackgroundColorMode::ColorScheme {
                if let Some(first_color) = color_scheme_data.get_first_color() {
                    let bg = [first_color[0], first_color[1], first_color[2], 1.0];
                    let background_params = BackgroundParams {
                        background_color: bg,
                    };
                    queue.write_buffer(
                        &model.background_params_buffer,
                        0,
                        bytemuck::cast_slice(&[background_params]),
                    );
                }
            }

            // Update background parameters with the correct color
            model.update_background_params(queue)?;
        }

        Ok(model)
    }

    pub fn update(
        &mut self,
        _device: &Device,
        queue: &Queue,
        settings: &Settings,
        state: &State,
    ) -> SimulationResult<()> {
        // Update simulation parameters
        let sim_params = SimParams {
            // Mouse interaction parameters from state (vec2 fields first)
            mouse_position: state.mouse_position,
            mouse_velocity: state.mouse_velocity,

            alpha: settings.alpha_radians(),
            beta: settings.beta,
            velocity: settings.velocity,
            radius: settings.radius,
            dt: state.dt,
            width: 2.0,  // [-1,1] world space has width of 2
            height: 2.0, // [-1,1] world space has height of 2
            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            particle_count: state.particle_count,
            mouse_pressed: if state.mouse_pressed { 1 } else { 0 },
            mouse_mode: state.mouse_mode,
            cursor_size: state.cursor_size,
            cursor_strength: state.cursor_strength,
            aspect_ratio: self.camera.viewport_width / self.camera.viewport_height,
            _pad1: 0.0,
            _pad0: 0.0,
        };

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        // Update render parameters
        let render_params = RenderParams {
            particle_size: state.particle_size,
            foreground_color_mode: state.foreground_color_mode as u32,
            _pad0: 0.0,
            _pad1: 0.0,
        };

        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );

        // Update density parameters
        let density_params = DensityParams {
            particle_count: state.particle_count,
            density_radius: settings.radius,
            coloring_mode: state.foreground_color_mode as u32,
            _padding: 0,
        };

        queue.write_buffer(
            &self.density_params_buffer,
            0,
            bytemuck::cast_slice(&[density_params]),
        );

        // Update background parameters
        self.update_background_params(queue)?;

        Ok(())
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        // Update camera for smooth movement
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        // Ensure the GPU sees the latest mouse/cursor and sim parameters before compute
        self.update_simulation_parameters(queue)?;

        // Dispatch compute shader to update particles (ping-pong) - always first
        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Primordial Particles Compute Encoder"),
        });

        {
            let mut compute_pass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Primordial Particles Compute Pass"),
                    timestamp_writes: None,
                });

            compute_pass.set_pipeline(&self.compute_pipeline);
            let compute_bind_group = self
                .particle_buffers
                .get_bind_group(&self.compute_bind_group1, &self.compute_bind_group2);
            compute_pass.set_bind_group(0, compute_bind_group, &[]);

            let workgroup_count = (self.state.particle_count + 63) / 64; // 64 particles per workgroup
            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }

        // Submit compute pass first
        queue.submit(std::iter::once(compute_encoder.finish()));

        // Run density calculation if needed for coloring
        if self.state.foreground_color_mode == super::state::ForegroundColorMode::Density {
            let mut density_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Primordial Particles Density Encoder"),
                });

            {
                let mut density_pass =
                    density_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Primordial Particles Density Pass"),
                        timestamp_writes: None,
                    });

                density_pass.set_pipeline(&self.density_pipeline);
                let density_bind_group = self
                    .particle_buffers
                    .get_bind_group(&self.density_bind_group_a, &self.density_bind_group_b);
                density_pass.set_bind_group(0, density_bind_group, &[]);

                let workgroup_count = (self.state.particle_count + 63) / 64; // 64 particles per workgroup
                density_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }

            queue.submit(std::iter::once(density_encoder.finish()));
        }

        // Render with or without trails
        if self.state.traces_enabled {
            // Calculate fade amount: convert trace_fade (0-1) to subtraction amount per frame
            let fade_amount = if self.state.trace_fade < 1.0 {
                // Invert trace_fade so 0.0 = fast fade, 1.0 = no fade
                let fade_strength = 1.0 - self.state.trace_fade;
                fade_strength * 0.05 // Scale to reasonable fade rate
            } else {
                0.0 // No fade when trace_fade is 1.0
            };

            self.update_fade_uniforms(queue, fade_amount);

            // Create new encoder for trail rendering
            let mut trail_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Primordial Particles Trail Encoder"),
                });

            // Apply fade effect - reads from previous texture, writes to current
            {
                let mut trail_render_pass =
                    trail_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Primordial Particles Trail Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: self.trail_textures.inactive_view(),
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.resolve_background_clear_color()), // Clear with background color
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                // Apply fade effect - reads from previous texture, writes to current
                trail_render_pass.set_pipeline(&self.fade_pipeline);
                trail_render_pass.set_bind_group(0, &self.fade_bind_group, &[]);
                trail_render_pass.draw(0..3, 0..1);

                // Then render particles on top
                trail_render_pass.set_pipeline(&self.render_pipeline);
                let render_bind_group = self
                    .particle_buffers
                    .get_bind_group(&self.render_bind_group_a, &self.render_bind_group_b);
                trail_render_pass.set_bind_group(0, render_bind_group, &[]);
                trail_render_pass.draw(0..6, 0..self.state.particle_count as u32); // 6 vertices per particle
            }

            // Submit the trail rendering encoder first
            queue.submit(std::iter::once(trail_encoder.finish()));

            // Now create a new encoder for the infinite tiling pass
            let mut surface_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Primordial Particles Surface Encoder"),
                });

            // Create a transient bind group sampling from the just-written trail texture
            // This is now safe because we're in a new encoder
            let trail_read_view = self.trail_textures.inactive_view();
            let infinite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Primordial Particles Infinite Trail BG"),
                layout: &self.infinite_render_bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, trail_read_view),
                    resource_helpers::sampler_bind_entry(1, &self.display_sampler),
                    resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
                ],
            });

            // Now infinite-tile trail texture to surface
            {
                let mut surface_render_pass =
                    surface_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Primordial Particles Infinite Trail Surface Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.resolve_background_clear_color()),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                surface_render_pass.set_pipeline(&self.render_infinite_pipeline);
                surface_render_pass.set_bind_group(0, &infinite_bg, &[]);
                surface_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

                let visible_world_size = 2.0 / self.camera.zoom;
                let tiles_needed = (visible_world_size / 2.0).ceil() as i32 + 6;
                let min_tiles = if self.camera.zoom < 0.1 { 7 } else { 5 };
                let tile_count = tiles_needed.max(min_tiles).min(1024);
                let total_instances = (tile_count * tile_count) as u32;
                surface_render_pass.draw(0..6, 0..total_instances);
            }

            // Submit the surface rendering encoder
            queue.submit(std::iter::once(surface_encoder.finish()));

            // Swap trail textures for next frame first so "current" points to the texture we just wrote
            self.trail_textures.swap();

            // Then update bind groups to reflect the new roles (read from new current)
            self.update_fade_bind_group(device);
            self.update_blit_bind_group(device);
        } else {
            // When trails are disabled, render to offscreen display then infinite-tile to surface
            // Create new encoder for offscreen rendering
            let mut offscreen_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Primordial Particles Offscreen Encoder"),
                });

            // 1) Offscreen pass into display texture
            {
                let mut offscreen_pass =
                    offscreen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Primordial Particles Offscreen Display Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.display_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.resolve_background_clear_color()),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                // Background
                offscreen_pass.set_pipeline(&self.background_render_pipeline);
                offscreen_pass.set_bind_group(0, &self.background_render_bind_group, &[]);
                offscreen_pass.draw(0..6, 0..1);

                // Particles
                offscreen_pass.set_pipeline(&self.render_pipeline);
                let render_bind_group = self
                    .particle_buffers
                    .get_bind_group(&self.render_bind_group_a, &self.render_bind_group_b);
                offscreen_pass.set_bind_group(0, render_bind_group, &[]);
                offscreen_pass.draw(0..6, 0..self.state.particle_count as u32);
            }

            // Submit the offscreen rendering encoder first
            queue.submit(std::iter::once(offscreen_encoder.finish()));

            // 2) Infinite tiling pass to surface with new encoder
            let mut surface_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Primordial Particles Surface Encoder"),
                });

            {
                let mut surface_pass =
                    surface_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Primordial Particles Infinite Surface Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                surface_pass.set_pipeline(&self.render_infinite_pipeline);
                surface_pass.set_bind_group(0, &self.render_infinite_display_bind_group, &[]);
                surface_pass.set_bind_group(1, &self.camera_bind_group, &[]);

                // Compute tile count like other sims
                let visible_world_size = 2.0 / self.camera.zoom;
                let tiles_needed = (visible_world_size / 2.0).ceil() as i32 + 6;
                let min_tiles = if self.camera.zoom < 0.1 { 7 } else { 5 };
                let tile_count = tiles_needed.max(min_tiles).min(1024);
                let total_instances = (tile_count * tile_count) as u32;
                surface_pass.draw(0..6, 0..total_instances);
            }

            // Submit the surface rendering encoder
            queue.submit(std::iter::once(surface_encoder.finish()));
        }

        // Swap for next frame
        self.particle_buffers.swap();

        Ok(())
    }

    /// Recreate bind groups that reference the particle buffers after buffer recreation
    fn recreate_particle_bind_groups(&mut self, device: &Device) {
        // Recreate compute bind groups for ping-pong
        self.compute_bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Compute Bind Group 1"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, self.particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, self.particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(2, &self.sim_params_buffer),
            ],
        });
        self.compute_bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Compute Bind Group 2"),
            layout: &self.compute_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, self.particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, self.particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(2, &self.sim_params_buffer),
            ],
        });

        // Recreate initialization bind groups for both buffers
        self.init_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Init Bind Group A"),
            layout: &self.init_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, self.particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, &self.init_params_buffer),
            ],
        });
        self.init_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Init Bind Group B"),
            layout: &self.init_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, self.particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, &self.init_params_buffer),
            ],
        });

        // Recreate render bind groups for A/B
        self.render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Render Bind Group A"),
            layout: &self.render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, self.particle_buffers.current_buffer()),
                resource_helpers::buffer_entry(1, self.camera.buffer()),
                resource_helpers::buffer_entry(2, &self.render_params_buffer),
                resource_helpers::buffer_entry(3, &self.lut_buffer),
            ],
        });
        self.render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Render Bind Group B"),
            layout: &self.render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, self.particle_buffers.inactive_buffer()),
                resource_helpers::buffer_entry(1, self.camera.buffer()),
                resource_helpers::buffer_entry(2, &self.render_params_buffer),
                resource_helpers::buffer_entry(3, &self.lut_buffer),
            ],
        });
    }

    /// Update simulation parameters buffer on GPU
    fn update_simulation_parameters(&self, queue: &Queue) -> SimulationResult<()> {
        let sim_params = SimParams {
            // Mouse interaction parameters from state (vec2 fields first)
            mouse_position: self.state.mouse_position,
            mouse_velocity: self.state.mouse_velocity,

            alpha: self.settings.alpha_radians(),
            beta: self.settings.beta,
            velocity: self.settings.velocity,
            radius: self.settings.radius,
            dt: self.state.dt,
            width: 2.0,  // [-1,1] world space has width of 2
            height: 2.0, // [-1,1] world space has height of 2
            wrap_edges: if self.settings.wrap_edges { 1 } else { 0 },
            particle_count: self.state.particle_count,
            mouse_pressed: if self.state.mouse_pressed { 1 } else { 0 },
            mouse_mode: self.state.mouse_mode,
            cursor_size: self.state.cursor_size,
            cursor_strength: self.state.cursor_strength,
            aspect_ratio: self.camera.viewport_width / self.camera.viewport_height,
            _pad1: 0.0,
            _pad0: 0.0,
        };

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
        Ok(())
    }

    /// Update render parameters buffer on GPU
    fn update_render_parameters(&self, queue: &Queue) -> SimulationResult<()> {
        let render_params = RenderParams {
            particle_size: self.state.particle_size,
            foreground_color_mode: self.state.foreground_color_mode as u32,
            _pad0: 0.0,
            _pad1: 0.0,
        };

        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );
        Ok(())
    }

    /// Update background parameters buffer on GPU
    fn update_background_params(&self, queue: &Queue) -> SimulationResult<()> {
        let background_color = match self.state.background_color_mode {
            BackgroundColorMode::Black => [0.0, 0.0, 0.0, 1.0],
            BackgroundColorMode::White => [1.0, 1.0, 1.0, 1.0],
            BackgroundColorMode::Gray18 => [0.18, 0.18, 0.18, 1.0],
            BackgroundColorMode::ColorScheme => {
                // Sample the first color from the current LUT
                if let Ok(mut lut) = self
                    .color_scheme_manager
                    .get(&self.state.current_color_scheme)
                {
                    if self.state.color_scheme_reversed {
                        lut = lut.reversed();
                    }
                    if let Some(first_color) = lut.get_first_color() {
                        [first_color[0], first_color[1], first_color[2], 1.0]
                    } else {
                        [0.0, 0.0, 0.0, 1.0]
                    }
                } else {
                    [0.0, 0.0, 0.0, 1.0]
                }
            }
        };

        // Update background parameters
        let background_params = BackgroundParams { background_color };

        queue.write_buffer(
            &self.background_params_buffer,
            0,
            bytemuck::cast_slice(&[background_params]),
        );

        Ok(())
    }

    pub fn reset(&mut self, device: &Device, queue: &Queue) -> SimulationResult<()> {
        // Update initialization parameters
        let init_params = InitParams {
            start_index: 0,
            spawn_count: self.state.particle_count,
            width: self.camera.viewport_width,
            height: self.camera.viewport_height,
            random_seed: self.state.random_seed,
            position_generator: self.state.position_generator,
            _pad1: 0,
            _pad2: 0,
        };

        queue.write_buffer(
            &self.init_params_buffer,
            0,
            bytemuck::cast_slice(&[init_params]),
        );

        // Dispatch initialization compute shader into both buffers
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Primordial Particles Reset Encoder"),
        });

        // Initialize buffer A
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Primordial Particles Reset Compute Pass A"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.init_pipeline);
            compute_pass.set_bind_group(0, &self.init_bind_group_a, &[]);
            let workgroup_count = (self.state.particle_count + 63) / 64;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }
        // Initialize buffer B
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Primordial Particles Reset Compute Pass B"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.init_pipeline);
            compute_pass.set_bind_group(0, &self.init_bind_group_b, &[]);
            let workgroup_count = (self.state.particle_count + 63) / 64;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// Update fade bind group to read from the previous trail texture
    fn update_fade_bind_group(&mut self, device: &Device) {
        // Read from the current trail texture (for fade effect)
        let read_texture_view = self.trail_textures.current_view();

        self.fade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Fade Bind Group"),
            layout: &self.fade_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &self.fade_uniforms_buffer),
                resource_helpers::texture_view_entry(1, read_texture_view),
                resource_helpers::sampler_bind_entry(2, &self.blit_sampler),
            ],
        });
    }

    /// Update blit bind group to read from the current completed trail texture
    fn update_blit_bind_group(&mut self, device: &Device) {
        // For blitting, we want to read from the texture we just finished writing to
        let read_texture_view = self.trail_textures.inactive_view();

        self.blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Primordial Particles Blit Bind Group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, read_texture_view),
                resource_helpers::sampler_bind_entry(1, &self.blit_sampler),
            ],
        });
    }

    /// Update fade uniforms buffer
    fn update_fade_uniforms(&self, queue: &Queue, fade_amount: f32) {
        let fade_uniforms = [fade_amount, 0.0f32, 0.0f32, 0.0f32];
        queue.write_buffer(
            &self.fade_uniforms_buffer,
            0,
            bytemuck::cast_slice(&fade_uniforms),
        );
    }

    /// Resolve background color as wgpu::Color for clears
    fn resolve_background_clear_color(&self) -> wgpu::Color {
        match self.state.background_color_mode {
            BackgroundColorMode::Black => wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            BackgroundColorMode::White => wgpu::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            BackgroundColorMode::Gray18 => wgpu::Color {
                r: 0.18,
                g: 0.18,
                b: 0.18,
                a: 1.0,
            },
            BackgroundColorMode::ColorScheme => {
                if let Ok(mut lut) = self
                    .color_scheme_manager
                    .get(&self.state.current_color_scheme)
                {
                    if self.state.color_scheme_reversed {
                        lut = lut.reversed();
                    }
                    if let Some(first) = lut.get_first_color() {
                        return wgpu::Color {
                            r: first[0] as f64,
                            g: first[1] as f64,
                            b: first[2] as f64,
                            a: 1.0,
                        };
                    }
                }
                wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }
            }
        }
    }

    /// Public helper for external callers (e.g., commands) to get the clear color
    pub fn background_clear_color(&self) -> wgpu::Color {
        self.resolve_background_clear_color()
    }

    /// Clear trail textures with background color
    pub fn clear_trail_texture(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        background_color: wgpu::Color,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Primordial Particles Clear Trail Texture Encoder"),
        });

        // Clear both trail textures
        for (i, view) in self.trail_textures.views().iter().enumerate() {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!(
                    "Clear Trail Texture {} Pass",
                    if i == 0 { "A" } else { "B" }
                )),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

impl Simulation for PrimordialParticlesModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        self.render(device, queue, surface_view, delta_time)
    }

    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Update camera for smooth movement (but don't update simulation)
        self.camera.update(0.016); // Use fixed delta time when paused
        self.camera.upload_to_gpu(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Primordial Particles Paused Render Encoder"),
        });

        // Skip compute shader dispatch - just render current state
        // Render
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Primordial Particles Paused Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render background
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_render_bind_group, &[]);
            render_pass.draw(0..6, 0..1);

            // Render particles from the current buffer (no simulation update)
            render_pass.set_pipeline(&self.render_pipeline);
            let render_bind_group = self
                .particle_buffers
                .get_bind_group(&self.render_bind_group_a, &self.render_bind_group_b);
            render_pass.set_bind_group(0, render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..self.state.particle_count as u32); // 6 vertices per particle
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Don't swap buffers when paused - keep current state
        Ok(())
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.camera.viewport_width = surface_config.width as f32;
        self.camera.viewport_height = surface_config.height as f32;

        // Recreate display texture and view to match new surface size
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Primordial Particles Display Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.display_texture = display_texture;

        // Recreate infinite render display bind group (view changed)
        self.render_infinite_display_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Primordial Particles Infinite Display BG"),
                layout: &self.infinite_render_bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &self.display_view),
                    resource_helpers::sampler_bind_entry(1, &self.display_sampler),
                    resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
                ],
            });
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
            "particle_count" => {
                if let Some(v) = value.as_u64() {
                    let new_count = v as u32;
                    if new_count != self.state.particle_count {
                        self.state.particle_count = new_count;
                        self.state.regenerate_particles(
                            self.camera.viewport_width as u32,
                            self.camera.viewport_height as u32,
                        );

                        // Recreate particle buffers with new size
                        let particle_stride = std::mem::size_of::<super::state::Particle>() as u64;
                        let particle_buffer_size = particle_stride * new_count as u64;
                        self.particle_buffers = PingPongBuffers::new(
                            device,
                            particle_buffer_size,
                            wgpu::BufferUsages::STORAGE
                                | wgpu::BufferUsages::VERTEX
                                | wgpu::BufferUsages::COPY_DST
                                | wgpu::BufferUsages::COPY_SRC,
                            "Primordial Particles Buffer",
                        );

                        // Initialize both buffers with new state particles
                        queue.write_buffer(
                            self.particle_buffers.current_buffer(),
                            0,
                            bytemuck::cast_slice(&self.state.particles),
                        );
                        queue.write_buffer(
                            self.particle_buffers.inactive_buffer(),
                            0,
                            bytemuck::cast_slice(&self.state.particles),
                        );

                        // Recreate bind groups that reference the particle buffers
                        self.recreate_particle_bind_groups(device);

                        // Update simulation parameters with new particle count
                        self.update_simulation_parameters(queue)?;

                        self.reset(device, queue)?;
                    }
                }
            }
            "alpha" => {
                if let Some(v) = value.as_f64() {
                    self.settings.alpha = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "beta" => {
                if let Some(v) = value.as_f64() {
                    self.settings.beta = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "velocity" => {
                if let Some(v) = value.as_f64() {
                    self.settings.velocity = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "radius" => {
                if let Some(v) = value.as_f64() {
                    self.settings.radius = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "wrap_edges" => {
                if let Some(v) = value.as_bool() {
                    self.settings.wrap_edges = v;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "dt" => {
                if let Some(v) = value.as_f64() {
                    self.state.dt = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "random_seed" => {
                if let Some(v) = value.as_u64() {
                    self.state.random_seed = v as u32;
                    self.state.regenerate_particles(
                        self.camera.viewport_width as u32,
                        self.camera.viewport_height as u32,
                    );
                    self.reset(device, queue)?;
                }
            }
            "position_generator" => {
                if let Some(v) = value.as_u64() {
                    self.state.position_generator = v as u32;
                    self.state.regenerate_particles(
                        self.camera.viewport_width as u32,
                        self.camera.viewport_height as u32,
                    );
                    self.reset(device, queue)?;
                }
            }
            _ => {
                tracing::warn!(
                    "Unknown setting parameter for PrimordialParticles: {}",
                    setting_name
                );
            }
        }
        Ok(())
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match state_name {
            "position_generator" => {
                if let Some(v) = value.as_u64() {
                    self.state.position_generator = v as u32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "random_seed" => {
                if let Some(v) = value.as_u64() {
                    self.state.random_seed = v as u32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "particle_count" => {
                if let Some(v) = value.as_u64() {
                    self.state.particle_count = v as u32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "particle_size" => {
                if let Some(v) = value.as_f64() {
                    self.state.particle_size = v as f32;
                    self.update_render_parameters(queue)?;
                }
            }
            "color_scheme" => {
                if let Some(v) = value.as_str() {
                    self.state.current_color_scheme = v.to_string();
                    let color_scheme = self.color_scheme_manager.get(v)?;
                    self.update_color_scheme(&color_scheme, device, queue)?;
                }
            }
            "color_scheme_reversed" => {
                if let Some(v) = value.as_bool() {
                    self.state.color_scheme_reversed = v;
                    let color_scheme = self
                        .color_scheme_manager
                        .get(&self.state.current_color_scheme)?;
                    self.update_color_scheme(&color_scheme, device, queue)?;
                }
            }
            "foreground_color_mode" => {
                if let Some(mode_str) = value.as_str() {
                    // Map frontend color mode strings to shader color scheme values
                    self.state.foreground_color_mode =
                        ForegroundColorMode::from_str(mode_str).expect("mode is valid");
                    self.update_render_parameters(queue)?;
                }
            }
            "background_color_mode" => {
                if let Some(mode_str) = value.as_str() {
                    self.state.background_color_mode =
                        BackgroundColorMode::from_str(mode_str).expect("mode is valid");
                    // When switching to ColorScheme, background uses the first LUT color
                    self.update_background_params(queue)?;
                }
            }
            // Mouse interaction parameters (treated as runtime state)
            "cursor_size" => {
                if let Some(v) = value.as_f64() {
                    self.state.cursor_size = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "cursor_strength" => {
                if let Some(v) = value.as_f64() {
                    self.state.cursor_strength = v as f32;
                    self.update_simulation_parameters(queue)?;
                }
            }
            "traces_enabled" => {
                if let Some(v) = value.as_bool() {
                    self.state.traces_enabled = v;
                    // Note: Trail infrastructure would need to be implemented for this to work
                }
            }
            "trace_fade" => {
                if let Some(v) = value.as_f64() {
                    self.state.trace_fade = v as f32;
                    // Note: Trail infrastructure would need to be implemented for this to work
                }
            }
            _ => {
                tracing::warn!(
                    "Unknown state parameter for PrimordialParticles: {}",
                    state_name
                );
            }
        }
        Ok(())
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
        let clamped_y = world_y.clamp(-1.0, 1.0);

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
            "position": self.camera.position,
            "zoom": self.camera.zoom
        })
    }

    fn save_preset(&self, preset_name: &str) -> SimulationResult<()> {
        // This would typically interact with a preset manager
        // For now, we'll just log that a preset was saved
        tracing::info!("Saving primordial particles preset: {}", preset_name);
        tracing::info!("Settings: {:?}", self.settings);
        Ok(())
    }

    fn load_preset(&mut self, preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // This would typically load from a preset manager
        // For now, we'll just log that a preset was loaded
        tracing::info!("Loading primordial particles preset: {}", preset_name);
        // In a full implementation, this would load the preset settings and apply them
        Ok(())
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Parse settings from JSON
        let new_settings: Settings = serde_json::from_value(settings)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;

        // Update settings
        self.settings = new_settings;

        self.update_simulation_parameters(queue)?;

        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Generate a new random seed for reset (like Particle Life does)
        use rand::Rng;
        let mut rng = rand::rng();
        let new_random_seed = rng.random();

        // Update the random seed in settings
        self.state.random_seed = new_random_seed;

        // Reset particles to new positions using current settings with new random seed
        self.state.regenerate_particles(
            self.camera.viewport_width as u32,
            self.camera.viewport_height as u32,
        );

        self.reset(device, queue)
    }

    fn toggle_gui(&mut self) -> bool {
        // This would need to be implemented if the simulation has GUI state
        false
    }

    fn is_gui_visible(&self) -> bool {
        // This would need to be implemented if the simulation has GUI state
        true
    }

    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Randomize the settings
        self.settings.randomize();

        // Apply the randomized settings (this handles particle regeneration if needed)
        let settings_json = serde_json::to_value(&self.settings)
            .map_err(|e| format!("Failed to serialize randomized settings: {}", e))?;
        self.apply_settings(settings_json, device, queue)?;

        tracing::info!(
            "Randomized Primordial Particles settings: alpha={:.1}°, beta={:.3}, velocity={:.2}, radius={:.3}, particles={}",
            self.settings.alpha,
            self.settings.beta,
            self.settings.velocity,
            self.settings.radius,
            self.state.particle_count
        );

        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Apply reversed flag if needed
        let color_scheme_data = if self.state.color_scheme_reversed {
            color_scheme.reversed()
        } else {
            color_scheme.clone()
        };

        // Update LUT buffer with new color scheme data
        let color_scheme_data_u32 = color_scheme_data.to_u32_buffer();
        queue.write_buffer(
            &self.lut_buffer,
            0,
            bytemuck::cast_slice(&color_scheme_data_u32),
        );

        // Update render parameters to apply the new LUT
        self.update_render_parameters(queue)?;
        // If background depends on the color scheme, update it too
        if self.state.background_color_mode == BackgroundColorMode::ColorScheme {
            self.update_background_params(queue)?;
        }

        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }

    fn get_state(&self) -> Value {
        serde_json::to_value(&self.state).unwrap_or(Value::Null)
    }
}
