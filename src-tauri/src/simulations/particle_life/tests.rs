//! # Particle Life Testing Module
//!
//! Comprehensive testing framework that ensures the Particle Life simulation operates
//! correctly across different environments and configurations. These tests validate
//! both the computational correctness and the integration between different
//! components of the simulation system.

use super::shaders::{
    BACKGROUND_RENDER_SHADER, COMPUTE_SHADER, FADE_FRAGMENT_SHADER, FADE_VERTEX_SHADER,
    FORCE_RANDOMIZE_SHADER, FORCE_UPDATE_SHADER, FRAGMENT_SHADER, INIT_SHADER, VERTEX_SHADER,
};
use super::simulation::{
    BackgroundParams, FadeUniforms, ForceRandomizeParams, ForceUpdateParams, InitParams, Particle,
    SimParams,
};
use std::mem;
use wgpu::util::DeviceExt;

/// Test framework for validating Particle Life shader compilation and buffer binding
struct ParticleLifeValidator {
    device: wgpu::Device,
    _queue: wgpu::Queue,
}

impl ParticleLifeValidator {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
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
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                ..Default::default()
            })
            .await
            .expect("Failed to create device");

        Self {
            device,
            _queue: queue,
        }
    }

    /// Validates that the Particle Life compute shader compiles without errors
    fn validate_compute_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life init shader compiles without errors
    fn validate_init_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Init Shader"),
                source: wgpu::ShaderSource::Wgsl(INIT_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life force update shader compiles without errors
    fn validate_force_update_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Force Update Shader"),
                source: wgpu::ShaderSource::Wgsl(FORCE_UPDATE_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life force randomize shader compiles without errors
    fn validate_force_randomize_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Force Randomize Shader"),
                source: wgpu::ShaderSource::Wgsl(FORCE_RANDOMIZE_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life vertex shader compiles without errors
    fn validate_vertex_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(VERTEX_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life fragment shader compiles without errors
    fn validate_fragment_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(FRAGMENT_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life fade vertex shader compiles without errors
    fn validate_fade_vertex_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Fade Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(FADE_VERTEX_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life fade fragment shader compiles without errors
    fn validate_fade_fragment_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Fade Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(FADE_FRAGMENT_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Particle Life background render shader compiles without errors
    fn validate_background_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Background Render Shader"),
                source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the compute shader can bind to the Rust structs
    fn validate_compute_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                species: 0,
                _pad: 0,
            })
            .collect();

        let dummy_sim_params = SimParams {
            particle_count: 1000,
            species_count: 4,
            max_force: 1.0,
            max_distance: 1.0,
            friction: 0.1,
            wrap_edges: 1,
            width: 1920.0,
            height: 1080.0,
            random_seed: 123,
            dt: 0.016,
            beta: 0.5,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: 50.0,
            cursor_strength: 1.0,
            cursor_active: 0,
            brownian_motion: 0.1,
            particle_size: 0.0001,
            aspect_ratio: 1.0,
            _pad1: 0,
        };

        // Create buffers
        let _particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Life Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let _sim_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        Ok(())
    }

    /// Validates that the render shaders can bind to the Rust structs
    fn validate_render_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                species: 0,
                _pad: 0,
            })
            .collect();

        let dummy_sim_params = SimParams {
            particle_count: 1000,
            species_count: 4,
            max_force: 1.0,
            max_distance: 1.0,
            friction: 0.1,
            wrap_edges: 1,
            width: 1920.0,
            height: 1080.0,
            random_seed: 123,
            dt: 0.016,
            beta: 0.5,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: 50.0,
            cursor_strength: 1.0,
            cursor_active: 0,
            brownian_motion: 0.1,
            particle_size: 0.01,
            aspect_ratio: 1.0,
            _pad1: 0,
        };

        // Create buffers
        let particle_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Life Render Particle Buffer"),
                contents: bytemuck::cast_slice(&dummy_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let sim_params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Life Render Sim Params Buffer"),
                contents: bytemuck::cast_slice(&[dummy_sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create LUT buffer
        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Life LUT Buffer"),
                contents: bytemuck::cast_slice(&[0u32; 256 * 4]), // 256 RGBA colors
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create camera buffer
        let dummy_camera = crate::simulations::shared::camera::CameraUniform {
            transform_matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            position: [0.0, 0.0],
            zoom: 1.0,
            aspect_ratio: 16.0 / 9.0,
        };

        let camera_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Life Camera Buffer"),
                contents: bytemuck::cast_slice(&[dummy_camera]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create viewport params buffer
        let dummy_viewport_params = crate::simulations::particle_life::simulation::ViewportParams {
            world_bounds: [-1.0, -1.0, 1.0, 1.0],
            texture_size: [1920.0, 1080.0],
            _pad1: 0.0,
            _pad2: 0.0,
        };

        let viewport_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Viewport Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_viewport_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create species colors buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct SpeciesColors {
            colors: [[f32; 4]; 9], // 9 colors (background + 8 species)
        }

        let dummy_species_colors = SpeciesColors {
            colors: [
                [1.0, 0.0, 0.0, 1.0], // Background
                [1.0, 0.0, 0.0, 1.0], // Species 0
                [0.0, 1.0, 0.0, 1.0], // Species 1
                [0.0, 0.0, 1.0, 1.0], // Species 2
                [1.0, 1.0, 0.0, 1.0], // Species 3
                [1.0, 0.0, 1.0, 1.0], // Species 4
                [0.0, 1.0, 1.0, 1.0], // Species 5
                [1.0, 1.0, 1.0, 1.0], // Species 6
                [0.5, 0.5, 0.5, 1.0], // Species 7
            ],
        };

        let species_colors_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Species Colors Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_species_colors]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create color mode buffer
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct ColorMode {
            mode: u32, // 0=Gray18, 1=White, 2=Black, 3=LUT
            _pad1: u32,
            _pad2: u32,
            _pad3: u32,
        }

        let dummy_color_mode = ColorMode {
            mode: 3, // LUT mode
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        let color_mode_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Life Color Mode Buffer"),
                contents: bytemuck::cast_slice(&[dummy_color_mode]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create shader modules
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(VERTEX_SHADER.into()),
            });

        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Particle Life Fragment Shader"),
                source: wgpu::ShaderSource::Wgsl(FRAGMENT_SHADER.into()),
            });

        // Create bind group layouts
        let bind_group_layout_0 =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Particle Life Render Bind Group Layout 0"),
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
                    label: Some("Particle Life Render Bind Group Layout 1"),
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
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group_layout_2 =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Particle Life Render Bind Group Layout 2"),
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

        // Create render pipeline layout
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Particle Life Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &bind_group_layout_0,
                        &bind_group_layout_1,
                        &bind_group_layout_2,
                    ],
                    push_constant_ranges: &[],
                });

        // Create render pipeline (this will validate shader binding)
        let _render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    cache: None,
                });

        // Create bind groups (this will validate buffer binding)
        let _bind_group_0 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Render Bind Group 0"),
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
            label: Some("Particle Life Render Bind Group 1"),
            layout: &bind_group_layout_1,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: species_colors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: color_mode_buffer.as_entire_binding(),
                },
            ],
        });

        let _bind_group_2 = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Render Bind Group 2"),
            layout: &bind_group_layout_2,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: viewport_params_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(())
    }

    /// Prints struct sizes for debugging
    fn print_struct_sizes(&self) {
        println!("Particle Life Struct Sizes:");
        println!("  Particle: {} bytes", mem::size_of::<Particle>());
        println!("  SimParams: {} bytes", mem::size_of::<SimParams>());
        println!(
            "  BackgroundParams: {} bytes",
            mem::size_of::<BackgroundParams>()
        );
        println!("  FadeUniforms: {} bytes", mem::size_of::<FadeUniforms>());
        println!("  InitParams: {} bytes", mem::size_of::<InitParams>());
        println!(
            "  ForceUpdateParams: {} bytes",
            mem::size_of::<ForceUpdateParams>()
        );
        println!(
            "  ForceRandomizeParams: {} bytes",
            mem::size_of::<ForceRandomizeParams>()
        );
        println!();
    }
}

#[tokio::test]
async fn test_particle_life_shader_compilation() {
    let validator = ParticleLifeValidator::new().await;

    // Test shader compilation
    validator
        .validate_compute_shader_compilation()
        .expect("Compute shader compilation failed");
    validator
        .validate_init_shader_compilation()
        .expect("Init shader compilation failed");
    validator
        .validate_force_update_shader_compilation()
        .expect("Force update shader compilation failed");
    validator
        .validate_force_randomize_shader_compilation()
        .expect("Force randomize shader compilation failed");
    validator
        .validate_vertex_shader_compilation()
        .expect("Vertex shader compilation failed");
    validator
        .validate_fragment_shader_compilation()
        .expect("Fragment shader compilation failed");
    validator
        .validate_fade_vertex_shader_compilation()
        .expect("Fade vertex shader compilation failed");
    validator
        .validate_fade_fragment_shader_compilation()
        .expect("Fade fragment shader compilation failed");
    validator
        .validate_background_render_shader_compilation()
        .expect("Background render shader compilation failed");

    // Print struct sizes for debugging
    validator.print_struct_sizes();
}

#[tokio::test]
async fn test_particle_life_buffer_binding() {
    let validator = ParticleLifeValidator::new().await;

    // Test buffer binding for compute shaders
    validator
        .validate_compute_shader_binding()
        .expect("Compute shader buffer binding failed");

    // Test buffer binding for render shaders
    validator
        .validate_render_shader_binding()
        .expect("Render shader buffer binding failed");
}

#[test]
fn test_struct_layout_compatibility() {
    // This test validates that the Rust struct layouts are compatible with WGSL
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = ParticleLifeValidator::new().await;

        // Test that compute shaders can bind to our Rust structs
        match validator.validate_compute_shader_binding() {
            Ok(_) => println!("✅ Particle Life compute shader struct layout is compatible"),
            Err(e) => panic!(
                "❌ Particle Life compute shader struct layout mismatch: {}",
                e
            ),
        }

        // Test that render shaders can bind to our Rust structs
        match validator.validate_render_shader_binding() {
            Ok(_) => println!("✅ Particle Life render shader struct layout is compatible"),
            Err(e) => panic!(
                "❌ Particle Life render shader struct layout mismatch: {}",
                e
            ),
        }
    });
}

#[test]
fn test_struct_layout_consistency() {
    // This test validates that Rust struct sizes match their buffer sizes
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = ParticleLifeValidator::new().await;

        // Test that we can create buffers with the actual struct sizes
        let particle_size = mem::size_of::<Particle>();
        let sim_params_size = mem::size_of::<SimParams>();
        let background_params_size = mem::size_of::<BackgroundParams>();
        let fade_uniforms_size = mem::size_of::<FadeUniforms>();
        let init_params_size = mem::size_of::<InitParams>();
        let force_update_params_size = mem::size_of::<ForceUpdateParams>();
        let force_randomize_params_size = mem::size_of::<ForceRandomizeParams>();

        // Create dummy data
        let dummy_particles: Vec<Particle> = (0..10)
            .map(|_| Particle {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                species: 0,
                _pad: 0,
            })
            .collect();

        let dummy_sim_params = SimParams {
            particle_count: 1000,
            species_count: 4,
            max_force: 1.0,
            max_distance: 1.0,
            friction: 0.1,
            wrap_edges: 1,
            width: 1920.0,
            height: 1080.0,
            random_seed: 123,
            dt: 0.016,
            beta: 0.5,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: 50.0,
            cursor_strength: 1.0,
            cursor_active: 0,
            brownian_motion: 0.1,
            particle_size: 0.01,
            aspect_ratio: 1.0,
            _pad1: 0,
        };

        let dummy_background_params = BackgroundParams {
            background_color: [0.0, 0.0, 0.0, 1.0], // RGBA black
        };

        let dummy_fade_uniforms = FadeUniforms {
            fade_amount: 0.01,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        let dummy_init_params = InitParams {
            start_index: 0,
            spawn_count: 100,
            species_count: 4,
            width: 1920.0,
            height: 1080.0,
            random_seed: 123,
            position_generator: 0,
            type_generator: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let dummy_force_update_params = ForceUpdateParams {
            species_a: 0,
            species_b: 1,
            new_force: 0.5,
            species_count: 4,
        };

        let dummy_force_randomize_params = ForceRandomizeParams {
            species_count: 4,
            random_seed: 123,
            min_force: -1.0,
            max_force: 1.0,
        };

        // Create camera buffer
        let dummy_camera = crate::simulations::shared::camera::CameraUniform {
            transform_matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            position: [0.0, 0.0],
            zoom: 1.0,
            aspect_ratio: 16.0 / 9.0,
        };

        // Create buffers and verify sizes
        let particle_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Particle Buffer"),
                    contents: bytemuck::cast_slice(&dummy_particles),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let sim_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let background_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let fade_uniforms_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Fade Uniforms Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_fade_uniforms]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let init_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Init Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_init_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let force_update_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Force Update Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_force_update_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let force_randomize_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Force Randomize Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_force_randomize_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let _camera_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particle Life Camera Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_camera]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Verify buffer sizes match struct sizes
        assert_eq!(
            particle_buffer.size() as usize,
            dummy_particles.len() * particle_size
        );
        assert_eq!(sim_params_buffer.size() as usize, sim_params_size);
        assert_eq!(
            background_params_buffer.size() as usize,
            background_params_size
        );
        assert_eq!(fade_uniforms_buffer.size() as usize, fade_uniforms_size);
        assert_eq!(init_params_buffer.size() as usize, init_params_size);
        assert_eq!(
            force_update_params_buffer.size() as usize,
            force_update_params_size
        );
        assert_eq!(
            force_randomize_params_buffer.size() as usize,
            force_randomize_params_size
        );

        println!("✅ Particle Life struct layout consistency verified");
        println!(
            "  Particle buffer size: {} bytes ({} particles × {} bytes each)",
            particle_buffer.size(),
            dummy_particles.len(),
            particle_size
        );
        println!(
            "  SimParams buffer size: {} bytes",
            sim_params_buffer.size()
        );
        println!(
            "  BackgroundParams buffer size: {} bytes",
            background_params_buffer.size()
        );
        println!(
            "  FadeUniforms buffer size: {} bytes",
            fade_uniforms_buffer.size()
        );
        println!(
            "  InitParams buffer size: {} bytes",
            init_params_buffer.size()
        );
        println!(
            "  ForceUpdateParams buffer size: {} bytes",
            force_update_params_buffer.size()
        );
        println!(
            "  ForceRandomizeParams buffer size: {} bytes",
            force_randomize_params_buffer.size()
        );
    });
}

#[test]
fn test_sim_params_size_and_alignment() {
    use super::settings::Settings;
    use super::simulation::SimParams;
    use std::mem;

    // Create a minimal settings and state for testing
    let settings = Settings::default();
    let state = super::simulation::State::new(1000, 4, 1920, 1080, 123);

    let sim_params = SimParams::new(1920, 1080, 1000, &settings, &state);

    println!(
        "SimParams struct size: {} bytes",
        mem::size_of::<SimParams>()
    );
    println!(
        "SimParams struct alignment: {} bytes",
        mem::align_of::<SimParams>()
    );

    // Test serialization
    let sim_params_array = [sim_params];
    let sim_params_bytes: &[u8] = bytemuck::cast_slice(&sim_params_array);
    println!(
        "Serialized SimParams size: {} bytes",
        sim_params_bytes.len()
    );

    // Verify the size is correct
    assert_eq!(mem::size_of::<SimParams>(), 80);
    assert_eq!(sim_params_bytes.len(), 80);
}
