use super::shaders::{PARTICLE_FRAGMENT_SHADER, PARTICLE_VERTEX_SHADER, RENDER_SHADER};
use super::simulation::{BackgroundParams, Particle, RenderParams};
use std::mem;
use wgpu::util::DeviceExt;

/// Test framework for validating Wanderers shader compilation and buffer binding
struct WanderersValidator {
    device: wgpu::Device,
    _queue: wgpu::Queue,
}

impl WanderersValidator {
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
                    required_features: wgpu::Features::empty(),
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

    /// Validates that the Wanderers vertex shader compiles without errors
    fn validate_vertex_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Wanderers Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_VERTEX_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Wanderers fragment shader compiles without errors
    fn validate_fragment_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Wanderers Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_FRAGMENT_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Wanderers render shader compiles without errors
    fn validate_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Wanderers Render Shader"),
                source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the particle vertex shader can bind to the Rust structs
    /// This will catch buffer size mismatches between Rust and WGSL
    fn validate_particle_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                mass: 1.0,
                radius: 0.1,
                clump_id: 0,
                density: 0.0,
                previous_position: [0.0, 0.0],
            })
            .collect();

        let dummy_render_params = RenderParams {
            particle_size: 0.015,
            screen_width: 1920.0,
            screen_height: 1080.0,
            coloring_mode: 0,
        };

        let dummy_background_params = BackgroundParams {
            background_type: 0,
            density_texture_resolution: 512,
        };

        // Create buffers
        let particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Wanderers Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let render_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Wanderers Render Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_render_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let _background_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Wanderers Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create dummy camera buffer
        let camera_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Wanderers Camera Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32; 16]), // 4x4 matrix
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create dummy LUT buffer
        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Wanderers LUT Buffer"),
                contents: bytemuck::cast_slice(&[0u32; 256]), // 256 color entries
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create shader modules
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Wanderers Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_VERTEX_SHADER.into()),
            });

        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Wanderers Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_FRAGMENT_SHADER.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Wanderers Render Bind Group Layout"),
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
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
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

        // Create pipeline layout
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Wanderers Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create render pipeline (this will validate shader binding)
        let _render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Wanderers Render Pipeline"),
                    layout: Some(&pipeline_layout),
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

        // Create bind group (this will validate buffer binding)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Render Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(())
    }

    /// Validates that the background render shader can bind to the Rust structs
    fn validate_background_shader_binding(&self) -> Result<(), String> {
        let dummy_background_params = BackgroundParams {
            background_type: 0,
            density_texture_resolution: 512,
        };

        let background_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Wanderers Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create shader module
        let background_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Wanderers Background Shader"),
                source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
            });

        // Create dummy texture for density visualization
        let dummy_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Wanderers Dummy Density Texture"),
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

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Wanderers Background Bind Group Layout"),
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

        // Create pipeline layout
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Wanderers Background Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create render pipeline (this will validate shader binding)
        let _background_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Wanderers Background Pipeline"),
                    layout: Some(&pipeline_layout),
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
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: None,
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

        // Create bind group (this will validate buffer binding)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Background Bind Group"),
            layout: &bind_group_layout,
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

        Ok(())
    }

    /// Print struct sizes for debugging buffer alignment issues
    fn print_struct_sizes(&self) {
        println!("Particle struct size: {} bytes", mem::size_of::<Particle>());
        println!(
            "RenderParams struct size: {} bytes",
            mem::size_of::<RenderParams>()
        );
        println!(
            "BackgroundParams struct size: {} bytes",
            mem::size_of::<BackgroundParams>()
        );
    }
}

#[tokio::test]
async fn test_wanderers_shader_compilation() {
    let validator = WanderersValidator::new().await;

    // Test shader compilation
    validator
        .validate_vertex_shader_compilation()
        .expect("Vertex shader compilation failed");
    validator
        .validate_fragment_shader_compilation()
        .expect("Fragment shader compilation failed");
    validator
        .validate_render_shader_compilation()
        .expect("Render shader compilation failed");

    // Print struct sizes for debugging
    validator.print_struct_sizes();
}

#[tokio::test]
async fn test_wanderers_buffer_binding() {
    let validator = WanderersValidator::new().await;

    // Test buffer binding for particle rendering
    validator
        .validate_particle_shader_binding()
        .expect("Particle shader buffer binding failed");

    // Test buffer binding for background rendering
    validator
        .validate_background_shader_binding()
        .expect("Background shader buffer binding failed");
}
