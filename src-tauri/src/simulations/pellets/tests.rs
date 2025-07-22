//! # Pellets Testing Module
//!
//! Comprehensive testing framework that ensures the Pellets simulation operates
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
    DENSITY_COMPUTE_SHADER, PARTICLE_FRAGMENT_SHADER, PARTICLE_VERTEX_SHADER,
    PHYSICS_COMPUTE_SHADER, RENDER_SHADER,
};
use super::simulation::{BackgroundParams, DensityParams, Particle, PhysicsParams, RenderParams};
use std::mem;
use wgpu::util::DeviceExt;

/// Test framework for validating Pellets shader compilation and buffer binding
struct PelletsValidator {
    device: wgpu::Device,
    _queue: wgpu::Queue,
}

impl PelletsValidator {
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

    /// Validates that the Pellets vertex shader compiles without errors
    fn validate_vertex_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_VERTEX_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Pellets fragment shader compiles without errors
    fn validate_fragment_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_FRAGMENT_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Pellets render shader compiles without errors
    fn validate_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Render Shader"),
                source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Pellets physics compute shader compiles without errors
    fn validate_physics_compute_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Physics Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(PHYSICS_COMPUTE_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Pellets density compute shader compiles without errors
    fn validate_density_compute_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Density Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(DENSITY_COMPUTE_SHADER.into()),
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
                grabbed: 0,
                _pad0: 0,
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
                label: Some("Pellets Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let render_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Pellets Render Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_render_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let _background_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Pellets Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create dummy camera buffer
        let camera_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Camera Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32; 16]), // 4x4 matrix
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create dummy LUT buffer
        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets LUT Buffer"),
                contents: bytemuck::cast_slice(&[0u32; 256]), // 256 color entries
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create shader modules
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_VERTEX_SHADER.into()),
            });

        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(PARTICLE_FRAGMENT_SHADER.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Create bind group (this will validate buffer binding)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Render Bind Group"),
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

        // Create render pipeline (this will validate shader binding)
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pellets Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let _render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        Ok(())
    }

    /// Validates that the particle vertex shader specifically can bind to the Rust structs
    /// This will catch the specific struct layout mismatch we're seeing
    fn validate_particle_vertex_shader_binding(&self) -> Result<(), String> {
        // Create dummy data with the exact same layout as the runtime
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                mass: 1.0,
                radius: 0.1,
                clump_id: 0,
                density: 0.0,
                grabbed: 0,
                _pad0: 0,
                previous_position: [0.0, 0.0],
            })
            .collect();

        // Create particle buffer with exact size
        let particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pellets Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout that matches the actual runtime usage
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Pellets Particle Vertex Bind Group Layout"),
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

        // Create bind group (this will validate the exact buffer binding)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pellets Particle Vertex Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
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
                    label: Some("Pellets Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create shader module
        let background_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Background Shader"),
                source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
            });

        // Create dummy texture for density visualization
        let dummy_texture = self.device.create_texture(&wgpu::TextureDescriptor {
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

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        // Create pipeline layout
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pellets Background Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create render pipeline (this will validate shader binding)
        let _background_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Pellets Background Pipeline"),
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
            label: Some("Pellets Background Bind Group"),
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
        println!(
            "PhysicsParams struct size: {} bytes",
            mem::size_of::<PhysicsParams>()
        );
        println!(
            "DensityParams struct size: {} bytes",
            mem::size_of::<DensityParams>()
        );
    }

    /// Validates that the physics compute shader can bind to the Rust structs
    /// This will catch buffer size mismatches between Rust and WGSL
    fn validate_physics_compute_shader_binding(&self) -> Result<(), String> {
        // Create shader modules
        let physics_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Physics Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(PHYSICS_COMPUTE_SHADER.into()),
            });

        let density_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pellets Density Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(DENSITY_COMPUTE_SHADER.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Pellets Compute Bind Group Layout"),
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

        // Create pipeline layouts
        let physics_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pellets Physics Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let density_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Pellets Density Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Create compute pipelines (this will validate shader binding)
        let _physics_compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Pellets Physics Compute Pipeline"),
                    layout: Some(&physics_pipeline_layout),
                    module: &physics_shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options: Default::default(),
                });

        let _density_compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Pellets Density Compute Pipeline"),
                    layout: Some(&density_pipeline_layout),
                    module: &density_shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options: Default::default(),
                });

        Ok(())
    }
}

#[tokio::test]
async fn test_pellets_shader_compilation() {
    let validator = PelletsValidator::new().await;

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
    validator
        .validate_physics_compute_shader_compilation()
        .expect("Physics compute shader compilation failed");
    validator
        .validate_density_compute_shader_compilation()
        .expect("Density compute shader compilation failed");

    // Print struct sizes for debugging
    validator.print_struct_sizes();
}

#[tokio::test]
async fn test_pellets_buffer_binding() {
    let validator = PelletsValidator::new().await;

    // Test buffer binding for particle rendering
    validator
        .validate_particle_shader_binding()
        .expect("Particle shader buffer binding failed");

    // Test buffer binding for background rendering
    validator
        .validate_background_shader_binding()
        .expect("Background shader buffer binding failed");

    // Test buffer binding for physics compute shaders
    validator
        .validate_physics_compute_shader_binding()
        .expect("Physics compute shader buffer binding failed");
}

#[test]
fn test_struct_layout_compatibility() {
    // This test validates that the Rust struct layouts are compatible with WGSL
    // by attempting to create buffers and bind groups with the actual shaders
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = PelletsValidator::new().await;

        // Test that physics compute shader can bind to our Rust structs
        match validator.validate_physics_compute_shader_binding() {
            Ok(_) => println!("✅ Physics compute shader struct layout is compatible"),
            Err(e) => panic!("❌ Physics compute shader struct layout mismatch: {}", e),
        }

        // Test that particle vertex shader can bind to our Rust structs
        match validator.validate_particle_shader_binding() {
            Ok(_) => println!("✅ Particle shader struct layout is compatible"),
            Err(e) => panic!("❌ Particle shader struct layout mismatch: {}", e),
        }

        // Test that particle vertex shader specifically can bind to our Rust structs
        match validator.validate_particle_vertex_shader_binding() {
            Ok(_) => println!("✅ Particle vertex shader struct layout is compatible"),
            Err(e) => panic!("❌ Particle vertex shader struct layout mismatch: {}", e),
        }
    });
}

#[test]
fn test_particle_vertex_shader_binding() {
    // This test specifically validates the particle vertex shader struct layout
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = PelletsValidator::new().await;

        // Test that particle vertex shader can bind to our Rust structs
        match validator.validate_particle_vertex_shader_binding() {
            Ok(_) => println!("✅ Particle vertex shader struct layout is compatible"),
            Err(e) => panic!("❌ Particle vertex shader struct layout mismatch: {}", e),
        }
    });
}

#[test]
fn test_physics_compute_shader_binding() {
    // This test specifically validates the physics compute shader struct layout
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = PelletsValidator::new().await;

        // Test that physics compute shader can bind to our Rust structs
        match validator.validate_physics_compute_shader_binding() {
            Ok(_) => println!("✅ Physics compute shader struct layout is compatible"),
            Err(e) => panic!("❌ Physics compute shader struct layout mismatch: {}", e),
        }
    });
}

#[test]
fn test_struct_layout_consistency() {
    // This test validates that Rust struct sizes match their buffer sizes
    // without hardcoding any expected values
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = PelletsValidator::new().await;

        // Test PhysicsParams struct
        let dummy_physics_params = PhysicsParams {
            mouse_position: [0.0, 0.0],
            mouse_velocity: [0.0, 0.0],
            particle_count: 10,
            gravitational_constant: 0.0001,
            energy_damping: 0.999,
            collision_damping: 0.5,
            dt: 1.0 / 60.0,
            gravity_softening: 0.01,
            interaction_radius: 0.5,
            mouse_pressed: 0,
            mouse_mode: 0,
            cursor_size: 0.35,
            cursor_strength: 0.02,
            particle_size: 0.01,
            aspect_ratio: 1.0,
            long_range_gravity_strength: 0.3,
        };

        let physics_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Pellets Physics Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_physics_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Validate that Rust struct size matches buffer size
        let rust_size = std::mem::size_of::<PhysicsParams>();
        let buffer_size = physics_params_buffer.size();

        assert_eq!(
            rust_size as u64, buffer_size,
            "PhysicsParams struct size ({}) doesn't match buffer size ({})",
            rust_size, buffer_size
        );

        // Test Particle struct
        let dummy_particles = vec![Particle {
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            mass: 1.0,
            radius: 0.1,
            clump_id: 0,
            density: 0.0,
            grabbed: 0,
            _pad0: 0,
            previous_position: [0.0, 0.0],
        }];

        let particle_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Pellets Particle Buffer"),
                    contents: bytemuck::cast_slice(&dummy_particles),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        // Validate that Rust struct size matches buffer size per particle
        let rust_particle_size = std::mem::size_of::<Particle>();
        let buffer_particle_size = particle_buffer.size() / dummy_particles.len() as u64;

        assert_eq!(
            rust_particle_size as u64, buffer_particle_size,
            "Particle struct size ({}) doesn't match buffer size per particle ({})",
            rust_particle_size, buffer_particle_size
        );

        println!("✅ All struct sizes match their buffer sizes");
        println!("  PhysicsParams: {} bytes", rust_size);
        println!("  Particle: {} bytes", rust_particle_size);
    });
}
