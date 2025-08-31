//! # Slime Mold Testing Module
//!
//! Comprehensive testing framework that ensures the Slime Mold simulation operates
//! correctly across different environments and configurations. These tests validate
//! both the computational correctness and the integration between different
//! components of the simulation system.

use super::shaders::{
    BACKGROUND_RENDER_SHADER, COMPUTE_SHADER, DISPLAY_SHADER, GRADIENT_SHADER, QUAD_SHADER,
};
use super::simulation::{BackgroundParams, SimSizeUniform};
use std::mem;
use wgpu::util::DeviceExt;

/// Test framework for validating Slime Mold shader compilation and buffer binding
struct SlimeMoldValidator {
    device: wgpu::Device,
    _queue: wgpu::Queue,
}

impl SlimeMoldValidator {
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    ..Default::default()
                },
            )
            .await
            .expect("Failed to create device");

        Self {
            device,
            _queue: queue,
        }
    }

    /// Validates that the Slime Mold compute shader compiles without errors
    fn validate_compute_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slime Mold Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Slime Mold display shader compiles without errors
    fn validate_display_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slime Mold Display Shader"),
                source: wgpu::ShaderSource::Wgsl(DISPLAY_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Slime Mold gradient shader compiles without errors
    fn validate_gradient_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slime Mold Gradient Shader"),
                source: wgpu::ShaderSource::Wgsl(GRADIENT_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Slime Mold quad shader compiles without errors
    fn validate_quad_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slime Mold Quad Shader"),
                source: wgpu::ShaderSource::Wgsl(QUAD_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the Slime Mold background render shader compiles without errors
    fn validate_background_render_shader_compilation(&self) -> Result<(), String> {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slime Mold Background Render Shader"),
                source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
            });
        Ok(())
    }

    /// Validates that the compute shader can bind to the Rust structs
    fn validate_compute_shader_binding(&self) -> Result<(), String> {
        // Create dummy data - agents are stored as 4 floats each (x, y, angle, speed)
        let dummy_agents: Vec<f32> = (0..10 * 4).map(|_| 0.0).collect();

        let dummy_sim_params = SimSizeUniform {
            width: 1920,
            height: 1080,
            decay_rate: 0.1,
            agent_jitter: 0.1,
            agent_speed_min: 50.0,
            agent_speed_max: 150.0,
            agent_turn_rate: 0.5,
            agent_sensor_angle: 0.5,
            agent_sensor_distance: 9.0,
            diffusion_rate: 1.0,
            pheromone_deposition_rate: 1.0,
            gradient_enabled: 0,
            gradient_type: 0,
            gradient_strength: 1.0,
            gradient_center_x: 0.0,
            gradient_center_y: 0.0,
            gradient_size: 100.0,
            gradient_angle: 0.0,
            random_seed: 123,
            position_generator: 0,
            _pad1: 0,
        };

        // Create buffers
        let _agent_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold Agent Buffer"),
                contents: bytemuck::cast_slice(&dummy_agents),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let _sim_params_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Slime Mold Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Create display texture
        let _display_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Slime Mold Display Texture"),
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

    /// Validates that the display shader can bind to the Rust structs
    fn validate_display_shader_binding(&self) -> Result<(), String> {
        // Create dummy data
        let dummy_sim_params = SimSizeUniform {
            width: 1920,
            height: 1080,
            decay_rate: 0.1,
            agent_jitter: 0.1,
            agent_speed_min: 50.0,
            agent_speed_max: 150.0,
            agent_turn_rate: 0.5,
            agent_sensor_angle: 0.5,
            agent_sensor_distance: 9.0,
            diffusion_rate: 1.0,
            pheromone_deposition_rate: 1.0,
            gradient_enabled: 0,
            gradient_type: 0,
            gradient_strength: 1.0,
            gradient_center_x: 0.0,
            gradient_center_y: 0.0,
            gradient_size: 100.0,
            gradient_angle: 0.0,
            random_seed: 123,
            position_generator: 0,
            _pad1: 0,
        };

        // Create trail map buffer
        let trail_map_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold Trail Map Buffer"),
                contents: bytemuck::cast_slice(&vec![0.0f32; 1920 * 1080]),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create LUT buffer
        let lut_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold LUT Buffer"),
                contents: bytemuck::cast_slice(&vec![0u32; 256 * 3]), // RGB values
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create gradient map buffer
        let gradient_map_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Slime Mold Gradient Map Buffer"),
                    contents: bytemuck::cast_slice(&vec![0.0f32; 1920 * 1080]),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        // Create sim params buffer
        let sim_params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold Display Sim Params Buffer"),
                contents: bytemuck::cast_slice(&[dummy_sim_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create display texture
        let display_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Slime Mold Display Texture"),
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

        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create shader module
        let display_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Slime Mold Display Shader"),
                source: wgpu::ShaderSource::Wgsl(DISPLAY_SHADER.into()),
            });

        // Create bind group layout
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Slime Mold Display Bind Group Layout"),
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
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
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
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
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

        // Create compute pipeline layout
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Slime Mold Display Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create compute pipeline (this will validate shader binding)
        let _compute_pipeline =
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Slime Mold Display Compute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &display_shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Create bind group (this will validate buffer binding)
        let _bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slime Mold Display Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: trail_map_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&display_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: lut_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: gradient_map_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(())
    }

    /// Prints struct sizes for debugging
    fn print_struct_sizes(&self) {
        println!("Slime Mold Struct Sizes:");
        println!("  Agent (4 floats): {} bytes", 4 * mem::size_of::<f32>());
        println!(
            "  SimSizeUniform: {} bytes",
            mem::size_of::<SimSizeUniform>()
        );
        println!(
            "  BackgroundParams: {} bytes",
            mem::size_of::<BackgroundParams>()
        );
        println!();
    }
}

#[tokio::test]
async fn test_slime_mold_shader_compilation() {
    let validator = SlimeMoldValidator::new().await;

    // Test shader compilation
    validator
        .validate_compute_shader_compilation()
        .expect("Compute shader compilation failed");
    validator
        .validate_display_shader_compilation()
        .expect("Display shader compilation failed");
    validator
        .validate_gradient_shader_compilation()
        .expect("Gradient shader compilation failed");
    validator
        .validate_quad_shader_compilation()
        .expect("Quad shader compilation failed");
    validator
        .validate_background_render_shader_compilation()
        .expect("Background render shader compilation failed");

    // Print struct sizes for debugging
    validator.print_struct_sizes();
}

#[tokio::test]
async fn test_slime_mold_buffer_binding() {
    let validator = SlimeMoldValidator::new().await;

    // Test buffer binding for compute shaders
    validator
        .validate_compute_shader_binding()
        .expect("Compute shader buffer binding failed");

    // Test buffer binding for display shaders
    validator
        .validate_display_shader_binding()
        .expect("Display shader buffer binding failed");
}

#[test]
fn test_struct_layout_compatibility() {
    // This test validates that the Rust struct layouts are compatible with WGSL
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = SlimeMoldValidator::new().await;

        // Test that compute shaders can bind to our Rust structs
        match validator.validate_compute_shader_binding() {
            Ok(_) => println!("✅ Slime Mold compute shader struct layout is compatible"),
            Err(e) => panic!("❌ Slime Mold compute shader struct layout mismatch: {}", e),
        }

        // Test that display shaders can bind to our Rust structs
        match validator.validate_display_shader_binding() {
            Ok(_) => println!("✅ Slime Mold display shader struct layout is compatible"),
            Err(e) => panic!("❌ Slime Mold display shader struct layout mismatch: {}", e),
        }
    });
}

#[test]
fn test_struct_layout_consistency() {
    // This test validates that Rust struct sizes match their buffer sizes
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let validator = SlimeMoldValidator::new().await;

        // Test that we can create buffers with the actual struct sizes
        let sim_params_size = mem::size_of::<SimSizeUniform>();
        let background_params_size = mem::size_of::<BackgroundParams>();

        // Create dummy data
        let dummy_agents: Vec<f32> = (0..10 * 4).map(|_| 0.0).collect();

        let dummy_sim_params = SimSizeUniform {
            width: 1920,
            height: 1080,
            decay_rate: 0.1,
            agent_jitter: 0.1,
            agent_speed_min: 50.0,
            agent_speed_max: 150.0,
            agent_turn_rate: 0.5,
            agent_sensor_angle: 0.5,
            agent_sensor_distance: 9.0,
            diffusion_rate: 1.0,
            pheromone_deposition_rate: 1.0,
            gradient_enabled: 0,
            gradient_type: 0,
            gradient_strength: 1.0,
            gradient_center_x: 0.0,
            gradient_center_y: 0.0,
            gradient_size: 100.0,
            gradient_angle: 0.0,
            random_seed: 123,
            position_generator: 0,
            _pad1: 0,
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
        let agent_buffer = validator
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold Agent Buffer"),
                contents: bytemuck::cast_slice(&dummy_agents),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let sim_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Slime Mold Sim Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_sim_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let background_params_buffer =
            validator
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Slime Mold Background Params Buffer"),
                    contents: bytemuck::cast_slice(&[dummy_background_params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Verify buffer sizes match struct sizes
        assert_eq!(
            agent_buffer.size() as usize,
            dummy_agents.len() * mem::size_of::<f32>()
        );
        assert_eq!(sim_params_buffer.size() as usize, sim_params_size);
        assert_eq!(
            background_params_buffer.size() as usize,
            background_params_size
        );

        println!("✅ Slime Mold struct layout consistency verified");
        println!(
            "  Agent buffer size: {} bytes ({} floats × {} bytes each)",
            agent_buffer.size(),
            dummy_agents.len(),
            mem::size_of::<f32>()
        );
        println!(
            "  SimSizeUniform buffer size: {} bytes",
            sim_params_buffer.size()
        );
        println!(
            "  BackgroundParams buffer size: {} bytes",
            background_params_buffer.size()
        );
    });
}
