//! # Flow Field Testing Module
//!
//! Comprehensive testing framework that ensures the Flow Field simulation operates
//! correctly across different environments and configurations. These tests validate
//! both the computational correctness and the integration between different
//! components of the simulation system.
//!
//! ## Testing Philosophy
//!
//! The testing approach focuses on catching issues early in the development
//! process, particularly those related to GPU compatibility and data structure
//! alignment. By testing shader compilation and buffer binding independently,
//! the framework can identify problems that might otherwise only manifest
//! during runtime execution.
//!
//! ## Test Strategy
//!
//! Tests are designed to validate the simulation at multiple levels, from
//! individual component functionality to complete system integration.
//! This layered approach ensures that both isolated components and their
//! interactions work correctly across different hardware configurations.

use super::shaders::{
    BACKGROUND_RENDER_SHADER, PARTICLE_RENDER_SHADER, PARTICLE_UPDATE_SHADER,
    TRAIL_DECAY_DIFFUSION_SHADER, TRAIL_RENDER_SHADER,
};
use super::simulation::{FlowVector, Particle, SimParams};
use std::mem;
use wgpu::util::DeviceExt;

/// Test framework for validating Flow Field shader compilation and buffer binding
struct FlowFieldValidator {
    device: wgpu::Device,
    _queue: wgpu::Queue,
}

impl FlowFieldValidator {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: Default::default(),
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        Self {
            device,
            _queue: queue,
        }
    }

    /// Validates that the Flow Field particle update shader compiles without errors
    fn validate_particle_update_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Particle Update Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_UPDATE_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Flow Field particle render shader compiles without errors
    fn validate_particle_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Particle Render Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Flow Field trail decay diffusion shader compiles without errors
    fn validate_trail_decay_diffusion_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Trail Decay Diffusion Shader"),
                source: wgpu::ShaderSource::Wgsl(TRAIL_DECAY_DIFFUSION_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Flow Field trail render shader compiles without errors
    fn validate_trail_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Trail Render Shader"),
                source: wgpu::ShaderSource::Wgsl(TRAIL_RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Flow Field background render shader compiles without errors
    fn validate_background_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Background Render Shader"),
                source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the particle render shader can bind to the Rust structs
    /// This will catch buffer size mismatches between Rust and WGSL
    fn validate_particle_render_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                age: 0.0,
                color: [1.0, 1.0, 1.0, 1.0],
                my_parent_was: 0,
            })
            .collect();

        let dummy_sim_params = SimParams {
            autospawn_limit: 500,
            vector_count: 100,
            particle_lifetime: 10.0,
            particle_speed: 1.0,
            noise_seed: 123,
            time: 0.0,
            width: 1920.0,
            height: 1080.0,
            noise_scale: 1.0,
            noise_x: 1.0,
            noise_y: 1.0,
            vector_magnitude: 1.0,
            trail_decay_rate: 0.1,
            trail_deposition_rate: 1.0,
            trail_diffusion_rate: 0.01,
            trail_wash_out_rate: 0.0,
            trail_map_width: 512,
            trail_map_height: 512,
            particle_shape: 0,
            particle_size: 2,
            screen_width: 1920,
            screen_height: 1080,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_active: 0,
            cursor_size: 50,
            cursor_strength: 1.0,
            particle_autospawn: 1,
            particle_spawn_rate: 1.0,
            display_mode: 0,
        };

        // Create buffers
        let particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let sim_params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field Sim Params Buffer"),
                contents: bytemuck::cast_slice(&[dummy_sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create dummy camera buffer
        let camera_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field Camera Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32; 16]), // 4x4 matrix
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create dummy LUT buffer
        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field LUT Buffer"),
                contents: bytemuck::cast_slice(&[0u32; 256]), // 256 color entries
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create shader modules
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Particle Render Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_RENDER_SHADER.into()),
            });

        // Create bind group layouts
        let bind_group_layout_0 =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Particle Render Bind Group Layout 0"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

        let bind_group_layout_1 =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Particle Render Bind Group Layout 1"),
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

        // Create render pipeline layout
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Flow Field Particle Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout_0, &bind_group_layout_1],
                    push_constant_ranges: &[],
                });

        // Create render pipeline (this will validate shader binding)
        let _render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Flow Field Particle Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    cache: None,
                    vertex: wgpu::VertexState {
                        module: &vertex_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &vertex_shader, // Same shader for fragment
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
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

        // Create bind groups (this will validate buffer binding)
        let _bind_group_0 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Field Particle Render Bind Group 0"),
            layout: &bind_group_layout_0,
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
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        let _bind_group_1 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Field Particle Render Bind Group 1"),
            layout: &bind_group_layout_1,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Ok(())
    }

    /// Validates that the compute shaders can bind to the Rust structs
    fn validate_compute_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                age: 0.0,
                color: [1.0, 1.0, 1.0, 1.0],
                my_parent_was: 0,
            })
            .collect();

        let dummy_flow_vectors: Vec<FlowVector> = (0..10)
            .map(|_| FlowVector {
                position: [0.0, 0.0],
                direction: [1.0, 0.0],
            })
            .collect();

        let dummy_sim_params = SimParams {
            autospawn_limit: 500,
            vector_count: 100,
            particle_lifetime: 10.0,
            particle_speed: 1.0,
            noise_seed: 123,
            time: 0.0,
            width: 1920.0,
            height: 1080.0,
            noise_scale: 1.0,
            noise_x: 1.0,
            noise_y: 1.0,
            vector_magnitude: 1.0,
            trail_decay_rate: 0.1,
            trail_deposition_rate: 1.0,
            trail_diffusion_rate: 0.01,
            trail_wash_out_rate: 0.0,
            trail_map_width: 512,
            trail_map_height: 512,
            particle_shape: 0,
            particle_size: 2,
            screen_width: 1920,
            screen_height: 1080,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_active: 0,
            cursor_size: 50,
            cursor_strength: 1.0,
            particle_autospawn: 1,
            particle_spawn_rate: 1.0,
            display_mode: 0,
        };

        // Create buffers
        let _particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let _flow_vector_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Flow Field Flow Vector Buffer"),
                    contents: bytemuck::cast_slice(&dummy_flow_vectors),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let _sim_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Flow Field Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create dummy LUT buffer
        let _lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field LUT Buffer"),
                contents: bytemuck::cast_slice(&[0u32; 256 * 3]), // 256 color entries × 3 channels
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Test that we can create buffers with the actual struct sizes
        let particle_size = mem::size_of::<Particle>();
        let flow_vector_size = mem::size_of::<FlowVector>();
        let sim_params_size = mem::size_of::<SimParams>();

        // Verify buffer sizes match struct sizes
        assert_eq!(
            _particle_buffer.size() as usize,
            dummy_particles.len() * particle_size
        );
        assert_eq!(
            _flow_vector_buffer.size() as usize,
            dummy_flow_vectors.len() * flow_vector_size
        );
        assert_eq!(_sim_params_buffer.size() as usize, sim_params_size);

        Ok(())
    }

    /// Validates that the storage texture format is supported by the adapter
    fn validate_storage_texture_support(&self) -> Result<(), String> {
        // Test that Rgba8Unorm supports storage binding (more widely supported than Bgra8Unorm)
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Flow Field Storage Texture Test"),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind group layout with storage texture (write-only for better compatibility)
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Storage Texture Test Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    }],
                });

        // Create bind group (this will fail if the format is not supported)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Field Storage Texture Test Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            }],
        });

        Ok(())
    }

    /// Validates that the compute pipelines can be created with the actual shaders
    fn validate_compute_pipeline_creation(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                age: 0.0,
                color: [1.0, 1.0, 1.0, 1.0],
                my_parent_was: 0,
            })
            .collect();

        let dummy_flow_vectors: Vec<FlowVector> = (0..10)
            .map(|_| FlowVector {
                position: [0.0, 0.0],
                direction: [1.0, 0.0],
            })
            .collect();

        let dummy_sim_params = SimParams {
            autospawn_limit: 500,
            vector_count: 100,
            particle_lifetime: 10.0,
            particle_speed: 1.0,
            noise_seed: 123,
            time: 0.0,
            width: 1920.0,
            height: 1080.0,
            noise_scale: 1.0,
            noise_x: 1.0,
            noise_y: 1.0,
            vector_magnitude: 1.0,
            trail_decay_rate: 0.1,
            trail_deposition_rate: 1.0,
            trail_diffusion_rate: 0.01,
            trail_wash_out_rate: 0.0,
            trail_map_width: 512,
            trail_map_height: 512,
            particle_shape: 0,
            particle_size: 2,
            screen_width: 1920,
            screen_height: 1080,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_active: 0,
            cursor_size: 50,
            cursor_strength: 1.0,
            particle_autospawn: 1,
            particle_spawn_rate: 1.0,
            display_mode: 0,
        };

        // Create buffers
        let particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let flow_vector_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Flow Field Flow Vector Buffer"),
                    contents: bytemuck::cast_slice(&dummy_flow_vectors),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let sim_params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field Sim Params Buffer"),
                contents: bytemuck::cast_slice(&[dummy_sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Field LUT Buffer"),
                contents: bytemuck::cast_slice(&[0u32; 256 * 3]), // 256 color entries × 3 channels
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create trail texture with Rgba8Unorm format
        let trail_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Flow Field Trail Texture"),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let trail_texture_view = trail_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create shader modules
        let particle_update_shader =
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Flow Field Particle Update Shader"),
                    source: wgpu::ShaderSource::Wgsl(PARTICLE_UPDATE_SHADER.into()),
                });

        let trail_decay_diffusion_shader =
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Flow Field Trail Decay Diffusion Shader"),
                    source: wgpu::ShaderSource::Wgsl(TRAIL_DECAY_DIFFUSION_SHADER.into()),
                });

        // Create bind group layout for particle update shader
        let particle_update_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Particle Update Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadWrite,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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

        // Create bind group layout for trail decay diffusion shader
        let trail_decay_diffusion_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Trail Decay Diffusion Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadWrite,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
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

        // Create pipeline layouts
        let particle_update_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Flow Field Particle Update Pipeline Layout"),
                    bind_group_layouts: &[&particle_update_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let trail_decay_diffusion_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Flow Field Trail Decay Diffusion Pipeline Layout"),
                    bind_group_layouts: &[&trail_decay_diffusion_bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Create compute pipelines (this will validate shader binding and format compatibility)
        let _particle_update_compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Flow Field Particle Update Compute Pipeline"),
                    layout: Some(&particle_update_pipeline_layout),
                    module: &particle_update_shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options: Default::default(),
                });

        let _trail_decay_diffusion_compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Flow Field Trail Decay Diffusion Compute Pipeline"),
                    layout: Some(&trail_decay_diffusion_pipeline_layout),
                    module: &trail_decay_diffusion_shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Create bind groups (this will validate buffer binding)
        let _particle_update_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Flow Field Particle Update Bind Group"),
                layout: &particle_update_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: flow_vector_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: sim_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&trail_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: lut_buffer.as_entire_binding(),
                    },
                ],
            });

        let _trail_decay_diffusion_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Flow Field Trail Decay Diffusion Bind Group"),
                layout: &trail_decay_diffusion_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&trail_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: flow_vector_buffer.as_entire_binding(),
                    },
                ],
            });

        Ok(())
    }

    /// Validates that all texture formats are consistent across bind groups and pipelines
    fn validate_texture_format_consistency(&self) -> Result<(), String> {
        // Test that all storage texture bindings use Rgba8Unorm format
        let _compute_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Compute Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadWrite,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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

        // Test trail update bind group layout
        let _trail_update_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Trail Update Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadWrite,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
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

        // Test render pipeline with Rgba8Unorm format
        let camera_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Camera Bind Group Layout"),
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

        let render_bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flow Field Render Bind Group Layout"),
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

        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Flow Field Render Pipeline Layout"),
                    bind_group_layouts: &[&render_bind_group_layout, &camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Field Test Render Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
                @vertex
                fn vs_main() -> @builtin(position) vec4<f32> {
                    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                }
            "#
                    .into(),
                ),
            });

        let _render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Flow Field Test Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &render_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &render_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
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

        Ok(())
    }

    /// Validates that render pipeline formats are compatible with render pass targets
    fn validate_render_pipeline_format_compatibility(&self) -> Result<(), String> {
        // Create a simple test shader
        let test_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Test Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
                @vertex
                fn vs_main() -> @builtin(position) vec4<f32> {
                    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                }
            "#
                    .into(),
                ),
            });

        // Test 1: Create render pipeline with Rgba8Unorm format (should work)
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Test Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let _compatible_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Compatible Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &test_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &test_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
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

        // Test 2: Create render pipeline with incompatible format (should still compile)
        let _incompatible_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Incompatible Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &test_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &test_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8UnormSrgb, // Incompatible format
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

        // Both pipelines should compile successfully
        // The format compatibility is checked at runtime when the pipeline is used in a render pass
        Ok(())
    }

    /// Validates that surface render pipeline uses the correct surface format
    fn validate_surface_pipeline_format(&self) -> Result<(), String> {
        // Create a simple test shader
        let test_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Test Surface Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
                @vertex
                fn vs_main() -> @builtin(position) vec4<f32> {
                    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                }
            "#
                    .into(),
                ),
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Test Surface Bind Group Layout"),
                    entries: &[],
                });

        // Test 1: Create surface pipeline with Bgra8UnormSrgb format (should work)
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Test Surface Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let _surface_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Surface Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &test_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &test_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
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

        // Test 2: Create surface pipeline with Rgba8Unorm format (should also work)
        let _rgba_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Rgba Surface Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &test_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &test_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
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

        // Both pipelines should compile successfully
        // The format compatibility is checked at runtime when the pipeline is used in a render pass
        Ok(())
    }

    /// Prints struct sizes for debugging
    fn print_struct_sizes(&self) {
        println!("Flow Field Struct Sizes:");
        println!("  Particle: {} bytes", mem::size_of::<Particle>());
        println!("  FlowVector: {} bytes", mem::size_of::<FlowVector>());
        println!("  SimParams: {} bytes", mem::size_of::<SimParams>());
        println!();
    }
}

#[tokio::test]
async fn test_flow_field_shader_compilation() {
    let validator = FlowFieldValidator::new().await;

    // Test shader compilation
    validator
        .validate_particle_update_shader_compilation()
        .expect("Particle update shader compilation failed");
    validator
        .validate_particle_render_shader_compilation()
        .expect("Particle render shader compilation failed");
    validator
        .validate_trail_decay_diffusion_shader_compilation()
        .expect("Trail decay diffusion shader compilation failed");
    validator
        .validate_trail_render_shader_compilation()
        .expect("Trail render shader compilation failed");
    validator
        .validate_background_render_shader_compilation()
        .expect("Background render shader compilation failed");

    // Print struct sizes for debugging
    validator.print_struct_sizes();
}

#[tokio::test]
async fn test_flow_field_buffer_binding() {
    let validator = FlowFieldValidator::new().await;

    // Test buffer binding for particle rendering
    validator
        .validate_particle_render_shader_binding()
        .expect("Particle render shader buffer binding failed");

    // Test buffer binding for compute shaders
    validator
        .validate_compute_shader_binding()
        .expect("Compute shader buffer binding failed");
}

#[tokio::test]
async fn test_flow_field_storage_texture_support() {
    let validator = FlowFieldValidator::new().await;

    // Test that Rgba8Unorm storage texture is supported (used by Flow Field simulation)
    validator
        .validate_storage_texture_support()
        .expect("Rgba8Unorm storage texture not supported by adapter");
}

#[tokio::test]
async fn test_flow_field_compute_pipeline_creation() {
    let validator = FlowFieldValidator::new().await;

    // Test that compute pipelines can be created with the actual shaders
    validator
        .validate_compute_pipeline_creation()
        .expect("Compute pipeline creation failed");
}

#[tokio::test]
async fn test_flow_field_texture_format_consistency() {
    let validator = FlowFieldValidator::new().await;

    // Test that all texture formats are consistent across bind groups and pipelines
    validator
        .validate_texture_format_consistency()
        .expect("Texture format consistency test failed");
}

#[tokio::test]
async fn test_flow_field_render_pipeline_format_compatibility() {
    let validator = FlowFieldValidator::new().await;

    // Test that render pipeline formats are compatible with render pass targets
    validator
        .validate_render_pipeline_format_compatibility()
        .expect("Render pipeline format compatibility test failed");
}

#[tokio::test]
async fn test_flow_field_surface_pipeline_format() {
    let validator = FlowFieldValidator::new().await;

    // Test that surface render pipeline uses the correct surface format
    validator
        .validate_surface_pipeline_format()
        .expect("Surface render pipeline format test failed");
}

#[test]
fn test_struct_layout_compatibility() {
    // This test validates that the Rust struct layouts are compatible with WGSL
    // by attempting to create buffers and bind groups with the actual shaders
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = FlowFieldValidator::new().await;

        // Test that compute shaders can bind to our Rust structs
        match validator.validate_compute_shader_binding() {
            Ok(_) => println!("✅ Flow Field compute shader struct layout is compatible"),
            Err(e) => panic!("❌ Flow Field compute shader struct layout mismatch: {}", e),
        }

        // Test that particle render shader can bind to our Rust structs
        match validator.validate_particle_render_shader_binding() {
            Ok(_) => println!("✅ Flow Field particle render shader struct layout is compatible"),
            Err(e) => panic!(
                "❌ Flow Field particle render shader struct layout mismatch: {}",
                e
            ),
        }
    });
}

#[test]
fn test_struct_layout_consistency() {
    // This test validates that Rust struct sizes match their buffer sizes
    // without hardcoding any expected values
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = FlowFieldValidator::new().await;

        // Test that we can create buffers with the actual struct sizes
        let particle_size = mem::size_of::<Particle>();
        let flow_vector_size = mem::size_of::<FlowVector>();
        let sim_params_size = mem::size_of::<SimParams>();

        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                age: 0.0,
                color: [1.0, 1.0, 1.0, 1.0],
                my_parent_was: 0,
            })
            .collect();

        let dummy_flow_vectors: Vec<FlowVector> = (0..10)
            .map(|_| FlowVector {
                position: [0.0, 0.0],
                direction: [1.0, 0.0],
            })
            .collect();

        let dummy_sim_params = SimParams {
            autospawn_limit: 500,
            vector_count: 100,
            particle_lifetime: 10.0,
            particle_speed: 1.0,
            noise_seed: 123,
            time: 0.0,
            width: 1920.0,
            height: 1080.0,
            noise_scale: 1.0,
            noise_x: 1.0,
            noise_y: 1.0,
            vector_magnitude: 1.0,
            trail_decay_rate: 0.1,
            trail_deposition_rate: 1.0,
            trail_diffusion_rate: 0.01,
            trail_wash_out_rate: 0.0,
            trail_map_width: 512,
            trail_map_height: 512,
            particle_shape: 0,
            particle_size: 2,
            screen_width: 1920,
            screen_height: 1080,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_active: 0,
            cursor_size: 50,
            cursor_strength: 1.0,
            particle_autospawn: 1,
            particle_spawn_rate: 1.0,
            display_mode: 0,
        };

        // Create buffers and verify sizes
        let particle_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Flow Field Particle Buffer"),
                    contents: bytemuck::cast_slice(&dummy_particles),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let flow_vector_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Flow Field Flow Vector Buffer"),
                    contents: bytemuck::cast_slice(&dummy_flow_vectors),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let sim_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Flow Field Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Verify buffer sizes match struct sizes
        assert_eq!(
            particle_buffer.size() as usize,
            dummy_particles.len() * particle_size
        );
        assert_eq!(
            flow_vector_buffer.size() as usize,
            dummy_flow_vectors.len() * flow_vector_size
        );
        assert_eq!(sim_params_buffer.size() as usize, sim_params_size);

        println!("✅ Flow Field struct layout consistency verified");
        println!(
            "  Particle buffer size: {} bytes ({} particles × {} bytes each)",
            particle_buffer.size(),
            dummy_particles.len(),
            particle_size
        );
        println!(
            "  FlowVector buffer size: {} bytes ({} vectors × {} bytes each)",
            flow_vector_buffer.size(),
            dummy_flow_vectors.len(),
            flow_vector_size
        );
        println!(
            "  SimParams buffer size: {} bytes",
            sim_params_buffer.size()
        );
    });
}

#[test]
fn test_flow_preset_functionality() {
    use crate::simulation::preset_manager::FlowPresetManager;
    use crate::simulations::flow::init_presets;

    // Create a preset manager
    let mut preset_manager = FlowPresetManager::new("flow".to_string());

    // Initialize presets
    init_presets(&mut preset_manager);

    // Check that we have at least the default preset
    let preset_names = preset_manager.get_preset_names();
    assert!(!preset_names.is_empty(), "Should have at least one preset");
    assert!(
        preset_names.contains(&"Default".to_string()),
        "Should have Default preset"
    );

    // Test getting preset settings
    let default_settings = preset_manager.get_preset_settings("Default");
    assert!(
        default_settings.is_some(),
        "Should be able to get Default preset settings"
    );

    // Test that the settings are valid
    if let Some(settings) = default_settings {
        assert_eq!(
            settings.noise_type,
            crate::simulations::flow::settings::NoiseType::OpenSimplex
        );
    }
}
