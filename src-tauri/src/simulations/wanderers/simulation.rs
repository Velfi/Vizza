//! # Wanderers Simulation Implementation
//! 
//! The core engine that brings the Wanderers particle physics simulation to life.
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

use super::{settings::Settings, state::State};
use crate::simulations::shared::{camera::Camera, LutManager};

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
    pub mouse_delta: [f32; 2],
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
    pub particle_size: f32, // Calculated particle size for consistent collision and rendering
    pub aspect_ratio: f32,  // Screen aspect ratio for collision correction
    pub long_range_gravity_strength: f32, // Controls orbital motion strength
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

// GPU-based physics implementation - no Rapier needed

pub struct WanderersModel {
    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub physics_params_buffer: wgpu::Buffer,
    pub density_params_buffer: wgpu::Buffer,
    pub render_params_buffer: wgpu::Buffer,
    pub background_params_buffer: wgpu::Buffer,
    pub lut_buffer: wgpu::Buffer,

    // Compute pipelines
    pub physics_compute_pipeline: wgpu::ComputePipeline,
    pub density_compute_pipeline: wgpu::ComputePipeline,

    // Compute bind groups
    pub physics_bind_group: wgpu::BindGroup,
    pub density_bind_group: wgpu::BindGroup,

    // Render pipeline
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_bind_group: wgpu::BindGroup,
    pub background_pipeline: wgpu::RenderPipeline,
    pub background_bind_group: wgpu::BindGroup,

    // Simulation state
    pub particles: Vec<Particle>,
    pub settings: Settings,
    pub state: State,
    pub camera: Camera,
    pub lut_manager: Arc<LutManager>,

    // Surface configuration
    pub surface_config: SurfaceConfiguration,

    // Performance optimization
    pub frame_count: u64,
    pub density_update_frequency: u64,
}

impl WanderersModel {
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
            label: Some("Wanderers Particle Buffer"),
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

        // Create initial state
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
            label: Some("Wanderers LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let render_params = RenderParams {
            particle_size: settings.particle_size,
            screen_width: surface_config.width as f32,
            screen_height: surface_config.height as f32,
            coloring_mode: if settings.coloring_mode == "velocity" {
                1
            } else {
                0
            },
        };

        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wanderers Render Params Buffer"),
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
                label: Some("Wanderers Background Params Buffer"),
                contents: bytemuck::cast_slice(&[background_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create physics params buffer
        // For now, use the original particle size directly to restore collision functionality
        // TODO: Refine this to match exact visual size calculation
        let collision_particle_size = settings.particle_size;

        let physics_params = PhysicsParams {
            mouse_position: [0.0, 0.0],
            mouse_delta: [0.0, 0.0],
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
            particle_size: collision_particle_size,
            aspect_ratio: surface_config.width as f32 / surface_config.height as f32,
            long_range_gravity_strength: 0.0, // Initialize new field
        };

        let physics_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wanderers Physics Params Buffer"),
            contents: bytemuck::cast_slice(&[physics_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create density params buffer
        let density_params = DensityParams {
            particle_count: settings.particle_count,
            density_radius: settings.density_radius,
            coloring_mode: if settings.coloring_mode == "velocity" {
                1
            } else {
                0
            },
            _padding: 0,
        };

        let density_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Wanderers Density Params Buffer"),
            contents: bytemuck::cast_slice(&[density_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create render pipeline
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wanderers Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::PARTICLE_VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wanderers Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::PARTICLE_FRAGMENT_SHADER.into()),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Wanderers Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wanderers Render Pipeline"),
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
            label: Some("Wanderers Physics Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::PHYSICS_COMPUTE_SHADER.into()),
        });

        let density_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wanderers Density Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::DENSITY_COMPUTE_SHADER.into()),
        });

        // Create compute bind group layouts
        let physics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Wanderers Physics Bind Group Layout"),
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

        // Create compute pipelines
        let physics_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Wanderers Physics Compute Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Wanderers Physics Pipeline Layout"),
                        bind_group_layouts: &[&physics_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &physics_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        let density_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Wanderers Density Compute Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Wanderers Density Pipeline Layout"),
                        bind_group_layouts: &[&physics_bind_group_layout], // Reuse same layout
                        push_constant_ranges: &[],
                    }),
                ),
                module: &density_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        // Create compute bind groups
        let physics_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Physics Bind Group"),
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
            ],
        });

        let density_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Density Bind Group"),
            layout: &physics_bind_group_layout,
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
            label: Some("Wanderers Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera.buffer().as_entire_binding(),
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

        // Create background pipeline
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wanderers Background Shader"),
            source: wgpu::ShaderSource::Wgsl(super::shaders::RENDER_SHADER.into()),
        });

        // Create dummy texture for density visualization
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
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

        let background_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Wanderers Background Pipeline Layout"),
                bind_group_layouts: &[&background_bind_group_layout],
                push_constant_ranges: &[],
            });

        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wanderers Background Pipeline"),
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

        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Background Bind Group"),
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

        Ok(WanderersModel {
            particle_buffer,
            physics_params_buffer,
            density_params_buffer,
            render_params_buffer,
            background_params_buffer,
            lut_buffer,
            physics_compute_pipeline,
            density_compute_pipeline,
            physics_bind_group,
            density_bind_group,
            render_pipeline,
            render_bind_group,
            background_pipeline,
            background_bind_group,
            particles,
            settings: settings.clone(),
            state,
            camera,
            lut_manager: Arc::new(lut_manager.clone()),
            surface_config: surface_config.clone(),
            frame_count: 0,
            density_update_frequency: 3, // Update density every 3 frames for performance
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
                // Random position within bounds
                let x = rng.random_range(-0.9..0.9);
                let y = rng.random_range(-0.9..0.9);

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
            label: Some("Wanderers Physics Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Wanderers Physics Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.physics_compute_pipeline);
            compute_pass.set_bind_group(0, &self.physics_bind_group, &[]);

            // Dispatch with optimal workgroup size
            let workgroup_size = 64;
            let num_workgroups =
                self.settings.particle_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        // Update density every few frames for performance
        if self.frame_count % self.density_update_frequency == 0 {
            self.update_density_params(queue);

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Wanderers Density Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.density_compute_pipeline);
            compute_pass.set_bind_group(0, &self.density_bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups =
                self.settings.particle_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn update_physics_params(&self, queue: &Arc<Queue>) {
        // For now, use the original particle size directly
        let collision_particle_size = self.settings.particle_size;

        let physics_params = PhysicsParams {
            mouse_position: self.state.mouse_position,
            mouse_delta: self.state.mouse_delta,
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
            particle_size: collision_particle_size,
            aspect_ratio: self.surface_config.width as f32 / self.surface_config.height as f32,
            long_range_gravity_strength: self.settings.long_range_gravity_strength,
        };

        queue.write_buffer(
            &self.physics_params_buffer,
            0,
            bytemuck::cast_slice(&[physics_params]),
        );
    }

    fn update_density_params(&self, queue: &Arc<Queue>) {
        let density_params = DensityParams {
            particle_count: self.settings.particle_count,
            density_radius: self.settings.density_radius,
            coloring_mode: if self.settings.coloring_mode == "velocity" {
                1
            } else {
                0
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
                label: Some("Wanderers Particle Buffer"),
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
        // Recreate compute bind group layouts
        let physics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Wanderers Physics Bind Group Layout"),
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

        // Recreate physics compute bind group
        self.physics_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Physics Bind Group"),
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
            ],
        });

        // Recreate density compute bind group
        self.density_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Density Bind Group"),
            layout: &physics_bind_group_layout,
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

        // Recreate render bind group
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        self.render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Wanderers Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.camera.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.render_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
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

    fn update_render_params(&self, queue: &Arc<Queue>) {
        let render_params = RenderParams {
            particle_size: self.settings.particle_size,
            screen_width: self.surface_config.width as f32,
            screen_height: self.surface_config.height as f32,
            coloring_mode: if self.settings.coloring_mode == "velocity" {
                1
            } else {
                0
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

impl std::fmt::Debug for WanderersModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WanderersModel")
            .field("particles", &self.particles)
            .field("settings", &self.settings)
            .field("state", &self.state)
            .field("camera", &self.camera)
            .field("surface_config", &self.surface_config)
            .finish()
    }
}

impl crate::simulations::traits::Simulation for WanderersModel {
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

        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Wanderers Render Encoder"),
        });

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Wanderers Render Pass"),
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

            // Render background
            render_pass.set_pipeline(&self.background_pipeline);
            render_pass.set_bind_group(0, &self.background_bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            // Render particles
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..6, 0..self.particles.len() as u32);
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

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Wanderers Static Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Wanderers Static Render Pass"),
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

            render_pass.set_pipeline(&self.background_pipeline);
            render_pass.set_bind_group(0, &self.background_bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..6, 0..self.particles.len() as u32);
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
        self.surface_config = new_config.clone();
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);
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
            "Wanderers::update_setting called with setting_name: '{}', value: {:?}",
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
            "min_particle_mass" => {
                if let Some(mass) = value.as_f64() {
                    self.settings.min_particle_mass = mass as f32;
                }
            }
            "max_particle_mass" => {
                if let Some(mass) = value.as_f64() {
                    self.settings.max_particle_mass = mass as f32;
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
            "clump_distance" => {
                if let Some(distance) = value.as_f64() {
                    self.settings.clump_distance = distance as f32;
                }
            }
            "cohesive_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.settings.cohesive_strength = strength as f32;
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
                })
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
        let clamped_y = world_y.clamp(-1.0, 1.0);

        // Always calculate mouse velocity, even when not pressed
        let previous_position = self.state.mouse_position;
        self.state.mouse_delta = [
            clamped_x - previous_position[0],
            clamped_y - previous_position[1],
        ];

        // Encode mouse button into mode: 0 none, 1 left(attraction)
        let mode = match mouse_button {
            0 => 1u32, // Left click for attraction
            _ => 0u32, // Other buttons do nothing
        };

        self.state.mouse_pressed = true;
        self.state.mouse_mode = mode;
        self.state.mouse_position = [clamped_x, clamped_y];

        // Clear grabbed particles list when starting new interaction
        self.state.grabbed_particles.clear();

        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.state.mouse_pressed = false;
        self.state.mouse_mode = 0;

        // Clear the grabbed particles list
        self.state.grabbed_particles.clear();

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
                label: Some("Wanderers Particle Buffer"),
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
