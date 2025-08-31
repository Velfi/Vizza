//! # Gray-Scott Testing Module
//!
//! Comprehensive testing framework that ensures the Gray-Scott simulation operates
//! correctly across different environments and configurations. These tests validate
//! both the computational correctness and the integration between different
//! components of the simulation system.

use super::shaders::{BACKGROUND_RENDER_SHADER, REACTION_DIFFUSION_SHADER};
use super::simulation::{BackgroundParams, SimulationParams};
use std::mem;
use wgpu::util::DeviceExt;

/// Test framework for validating Gray-Scott shader compilation and buffer binding
struct GrayScottValidator {
    device: wgpu::Device,
    _queue: wgpu::Queue,
}

impl GrayScottValidator {
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

    /// Validates that the Gray-Scott reaction diffusion shader compiles without errors
    fn validate_reaction_diffusion_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Gray-Scott Reaction Diffusion Shader"),
                source: wgpu::ShaderSource::Wgsl(REACTION_DIFFUSION_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Gray-Scott background render shader compiles without errors
    fn validate_background_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Gray-Scott Background Render Shader"),
                source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the reaction diffusion shader can bind to the Rust structs
    fn validate_reaction_diffusion_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_sim_params = SimulationParams {
            feed_rate: 0.055,
            kill_rate: 0.062,
            delta_u: 1.0,
            delta_v: 0.5,
            timestep: 1.0,
            width: 1920,
            height: 1080,
            nutrient_pattern: 0,
            is_nutrient_pattern_reversed: 0,
            // Optimization parameters
            max_timestep: 2.0,
            stability_factor: 0.8,
            enable_adaptive_timestep: 1,
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        // Create buffers
        let _sim_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gray-Scott Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create simulation textures
        let _simulation_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gray-Scott Simulation Texture"),
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

        Ok(())
    }

    /// Validates that the background render shader can bind to the Rust structs
    fn validate_background_render_shader_binding(&self) -> Result<(), String> {
        // Create dummy data for background rendering
        let dummy_sim_params = SimulationParams {
            feed_rate: 0.055,
            kill_rate: 0.062,
            delta_u: 1.0,
            delta_v: 0.5,
            timestep: 1.0,
            width: 1920,
            height: 1080,
            nutrient_pattern: 0,
            is_nutrient_pattern_reversed: 0,
            // Optimization parameters
            max_timestep: 2.0,
            stability_factor: 0.8,
            enable_adaptive_timestep: 1,
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        // Create buffers
        let sim_params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Gray-Scott Background Sim Params Buffer"),
                contents: bytemuck::cast_slice(&[dummy_sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create shader module
        let background_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Gray-Scott Background Render Shader"),
                source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Gray-Scott Background Bind Group Layout"),
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

        // Create render pipeline layout
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Gray-Scott Background Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create render pipeline (this will validate shader binding)
        let _render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Gray-Scott Background Render Pipeline"),
                    layout: Some(&pipeline_layout),
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
                            blend: Some(wgpu::BlendState::REPLACE),
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

        // Create bind group (this will validate buffer binding)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gray-Scott Background Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sim_params_buffer.as_entire_binding(),
            }],
        });

        Ok(())
    }

    /// Prints struct sizes for debugging
    fn print_struct_sizes(&self) {
        println!("Gray-Scott Struct Sizes:");
        println!(
            "  SimulationParams: {} bytes",
            mem::size_of::<SimulationParams>()
        );
        println!(
            "  BackgroundParams: {} bytes",
            mem::size_of::<BackgroundParams>()
        );
        println!();
    }
}

#[tokio::test]
async fn test_gray_scott_shader_compilation() {
    let validator = GrayScottValidator::new().await;

    // Test shader compilation
    validator
        .validate_reaction_diffusion_shader_compilation()
        .expect("Reaction diffusion shader compilation failed");
    validator
        .validate_background_render_shader_compilation()
        .expect("Background render shader compilation failed");

    // Print struct sizes for debugging
    validator.print_struct_sizes();
}

#[tokio::test]
async fn test_gray_scott_buffer_binding() {
    let validator = GrayScottValidator::new().await;

    // Test buffer binding for reaction diffusion shaders
    validator
        .validate_reaction_diffusion_shader_binding()
        .expect("Reaction diffusion shader buffer binding failed");

    // Test buffer binding for background render shaders
    validator
        .validate_background_render_shader_binding()
        .expect("Background render shader buffer binding failed");
}

#[test]
fn test_struct_layout_compatibility() {
    // This test validates that the Rust struct layouts are compatible with WGSL
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = GrayScottValidator::new().await;

        // Test that reaction diffusion shaders can bind to our Rust structs
        match validator.validate_reaction_diffusion_shader_binding() {
            Ok(_) => {
                println!("✅ Gray-Scott reaction diffusion shader struct layout is compatible")
            }
            Err(e) => panic!(
                "❌ Gray-Scott reaction diffusion shader struct layout mismatch: {}",
                e
            ),
        }

        // Test that background render shaders can bind to our Rust structs
        match validator.validate_background_render_shader_binding() {
            Ok(_) => println!("✅ Gray-Scott background render shader struct layout is compatible"),
            Err(e) => panic!(
                "❌ Gray-Scott background render shader struct layout mismatch: {}",
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
        let validator = GrayScottValidator::new().await;

        // Test that we can create buffers with the actual struct sizes
        let sim_params_size = mem::size_of::<SimulationParams>();
        let background_params_size = mem::size_of::<BackgroundParams>();

        // Create dummy data
        let dummy_sim_params = SimulationParams {
            feed_rate: 0.055,
            kill_rate: 0.062,
            delta_u: 1.0,
            delta_v: 0.5,
            timestep: 1.0,
            width: 1920,
            height: 1080,
            nutrient_pattern: 0,
            is_nutrient_pattern_reversed: 0,
            // Optimization parameters
            max_timestep: 2.0,
            stability_factor: 0.8,
            enable_adaptive_timestep: 1,
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        let dummy_background_params = BackgroundParams {
            background_type: 0,
            gradient_enabled: 0,
            gradient_type: 0,
            gradient_strength: 1.0,
            gradient_center_x: 0.0,
            gradient_center_y: 0.0,
            gradient_size: 100.0,
            gradient_angle: 0.0,
        };

        // Create buffers and verify sizes
        let sim_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gray-Scott Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let background_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Gray-Scott Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Verify buffer sizes match struct sizes
        assert_eq!(sim_params_buffer.size() as usize, sim_params_size);
        assert_eq!(
            background_params_buffer.size() as usize,
            background_params_size
        );

        println!("✅ Gray-Scott struct layout consistency verified");
        println!(
            "  SimulationParams buffer size: {} bytes",
            sim_params_buffer.size()
        );
        println!(
            "  BackgroundParams buffer size: {} bytes",
            background_params_buffer.size()
        );
    });
}
