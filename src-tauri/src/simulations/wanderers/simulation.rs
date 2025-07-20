use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use rapier2d::dynamics::ImpulseJointSet;
use rapier2d::dynamics::IslandManager;
use rapier2d::dynamics::MultibodyJointSet;
use rapier2d::pipeline::PhysicsPipeline;
use rapier2d::pipeline::QueryPipeline;
use rapier2d::prelude::*;
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
    pub clump_id: u32,               // ID of the clump this particle belongs to
    pub density: f32,                // Number of nearby particles
    pub previous_position: [f32; 2], // For Verlet integration
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RenderParams {
    pub particle_size: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub coloring_mode: u32, // 0 = density, 1 = velocity
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BackgroundParams {
    pub background_type: u32,            // 0 = black, 1 = white, 2 = density
    pub density_texture_resolution: u32, // Add texture resolution for proper sampling
}

pub struct RapierPhysics {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub gravity: Vector<f32>,
    pub integration_parameters: IntegrationParameters,
    pub body_handles: Vec<RigidBodyHandle>,
}

impl RapierPhysics {
    pub fn new(gravitational_constant: f32) -> Self {
        let integration_parameters = IntegrationParameters {
            dt: 1.0 / 120.0, // Higher frequency for better collision detection
            ..Default::default()
        };

        Self {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            query_pipeline: QueryPipeline::new(),
            gravity: vector![0.0, -gravitational_constant],
            integration_parameters,
            body_handles: Vec::new(),
        }
    }

    pub fn create_boundary_walls(&mut self) {
        let wall_thickness = 0.05; // Thinner walls
        let wall_restitution = 0.98; // Very bouncy walls
        let wall_friction = 0.01; // Very low friction to prevent sticking

        // Bottom wall
        let bottom_wall = RigidBodyBuilder::fixed()
            .translation(vector![0.0, -1.0 - wall_thickness / 2.0])
            .build();
        let bottom_wall_handle = self.bodies.insert(bottom_wall);
        let bottom_collider = ColliderBuilder::cuboid(1.0, wall_thickness / 2.0)
            .restitution(wall_restitution)
            .friction(wall_friction)
            .build();
        self.colliders
            .insert_with_parent(bottom_collider, bottom_wall_handle, &mut self.bodies);

        // Top wall
        let top_wall = RigidBodyBuilder::fixed()
            .translation(vector![0.0, 1.0 + wall_thickness / 2.0])
            .build();
        let top_wall_handle = self.bodies.insert(top_wall);
        let top_collider = ColliderBuilder::cuboid(1.0, wall_thickness / 2.0)
            .restitution(wall_restitution)
            .friction(wall_friction)
            .build();
        self.colliders
            .insert_with_parent(top_collider, top_wall_handle, &mut self.bodies);

        // Left wall
        let left_wall = RigidBodyBuilder::fixed()
            .translation(vector![-1.0 - wall_thickness / 2.0, 0.0])
            .build();
        let left_wall_handle = self.bodies.insert(left_wall);
        let left_collider = ColliderBuilder::cuboid(wall_thickness / 2.0, 1.0)
            .restitution(wall_restitution)
            .friction(wall_friction)
            .build();
        self.colliders
            .insert_with_parent(left_collider, left_wall_handle, &mut self.bodies);

        // Right wall
        let right_wall = RigidBodyBuilder::fixed()
            .translation(vector![1.0 + wall_thickness / 2.0, 0.0])
            .build();
        let right_wall_handle = self.bodies.insert(right_wall);
        let right_collider = ColliderBuilder::cuboid(wall_thickness / 2.0, 1.0)
            .restitution(wall_restitution)
            .friction(wall_friction)
            .build();
        self.colliders
            .insert_with_parent(right_collider, right_wall_handle, &mut self.bodies);
    }

    pub fn create_particle_bodies(
        &mut self,
        particles: &[Particle],
        particle_size: f32,
        energy_damping: f32,
        collision_damping: f32,
    ) {
        self.body_handles.clear();
        self.body_handles.reserve(particles.len());

        for p in particles {
            let rb = RigidBodyBuilder::dynamic()
                .translation(vector![p.position[0], p.position[1]])
                .linvel(vector![p.velocity[0], p.velocity[1]])
                .lock_rotations() // Keep particles as circles, no rotation
                .linear_damping(1.0 - energy_damping)
                .build();
            let rb_handle = self.bodies.insert(rb);

            // Create circular collider with proper collision properties
            let collider = ColliderBuilder::ball(particle_size)
                .restitution(1.0 - collision_damping)
                .friction(0.05) // Lower friction for smoother particle movement
                .density(1.0) // Uniform density for all particles
                .sensor(false) // Make sure it's not a sensor
                .build();
            self.colliders
                .insert_with_parent(collider, rb_handle, &mut self.bodies);
            self.body_handles.push(rb_handle);
        }
    }

    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    pub fn update_gravity(&mut self, gravitational_constant: f32) {
        self.gravity = vector![0.0, -gravitational_constant];
    }

    pub fn update_energy_damping(&mut self, energy_damping: f32) {
        for (_, rb) in self.bodies.iter_mut() {
            rb.set_linear_damping(1.0 - energy_damping);
        }
    }

    pub fn update_collision_damping(&mut self, collision_damping: f32) {
        for (_, collider) in self.colliders.iter_mut() {
            collider.set_restitution(1.0 - collision_damping);
        }
    }

    pub fn reset(&mut self, gravitational_constant: f32) {
        self.bodies = RigidBodySet::new();
        self.colliders = ColliderSet::new();
        self.body_handles.clear();

        // Reset physics pipeline components to clear stale collision data
        self.island_manager = IslandManager::new();
        self.broad_phase = BroadPhaseMultiSap::new();
        self.narrow_phase = NarrowPhase::new();
        self.ccd_solver = CCDSolver::new();
        self.impulse_joints = ImpulseJointSet::new();
        self.multibody_joints = MultibodyJointSet::new();
        self.query_pipeline = QueryPipeline::new();

        self.gravity = vector![0.0, -gravitational_constant];
        self.integration_parameters = IntegrationParameters {
            dt: 1.0 / 120.0,
            ..Default::default()
        };

        // Recreate boundary walls
        self.create_boundary_walls();
    }
}

pub struct WanderersModel {
    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub render_params_buffer: wgpu::Buffer,
    pub background_params_buffer: wgpu::Buffer,
    pub lut_buffer: wgpu::Buffer,

    // Render pipeline
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_bind_group: wgpu::BindGroup,
    pub background_pipeline: wgpu::RenderPipeline,
    pub background_bind_group: wgpu::BindGroup,

    // Physics engine
    pub rapier: RapierPhysics,

    // Simulation state
    pub particles: Vec<Particle>,
    pub settings: Settings,
    pub state: State,
    pub camera: Camera,
    pub lut_manager: Arc<LutManager>,

    // Surface configuration
    pub surface_config: SurfaceConfiguration,
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

        // Initialize Rapier physics world
        let mut rapier = RapierPhysics::new(settings.gravitational_constant);
        rapier.create_boundary_walls();
        rapier.create_particle_bodies(
            &particles,
            settings.particle_size,
            settings.energy_damping,
            settings.collision_damping,
        );

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
            background_bind_group,
            background_params_buffer,
            background_pipeline,
            camera,
            lut_buffer,
            lut_manager: Arc::new(lut_manager.clone()),
            particle_buffer,
            particles,
            rapier,
            render_bind_group,
            render_params_buffer,
            render_pipeline,
            settings: settings.clone(),
            state,
            surface_config: surface_config.clone(),
        })
    }

    fn initialize_particles(count: u32, settings: &Settings) -> Vec<Particle> {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut particles = Vec::with_capacity(count as usize);

        if count == 1 {
            let mass = 1.0;
            particles.push(Particle {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                mass,
                radius: 0.008,
                clump_id: 0,
                density: 0.0,
                previous_position: [0.0, 0.0], // Stationary particle
            });
        } else if count == 2 {
            // Place two particles side-by-side near the centre so both are visible
            let mass = 1.0;
            let offset = 0.02; // small horizontal separation in world units

            particles.push(Particle {
                position: [-offset, 0.0],
                velocity: [0.0, 0.0],
                mass,
                radius: 0.008,
                clump_id: 0,
                density: 0.0,
                previous_position: [-offset, 0.0], // Stationary particle
            });

            particles.push(Particle {
                position: [offset, 0.0],
                velocity: [0.0, 0.0],
                mass,
                radius: 0.008,
                clump_id: 0,
                density: 0.0,
                previous_position: [offset, 0.0], // Stationary particle
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
                    previous_position: [prev_x, prev_y],
                });
            }
        }

        particles
    }

    fn update_particle_count(
        &mut self,
        new_count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let current_count = self.particles.len() as u32;

        if new_count > current_count {
            // Add particles
            let particles_to_add = new_count - current_count;
            let new_particles = Self::initialize_particles(particles_to_add, &self.settings);

            // Add corresponding Rapier bodies and colliders
            for p in &new_particles {
                let rb = RigidBodyBuilder::dynamic()
                    .translation(vector![p.position[0], p.position[1]])
                    .linvel(vector![p.velocity[0], p.velocity[1]])
                    .lock_rotations() // Keep particles as circles, no rotation
                    .linear_damping(1.0 - self.settings.energy_damping)
                    .build();
                let rb_handle = self.rapier.bodies.insert(rb);

                let collider = ColliderBuilder::ball(self.settings.particle_size)
                    .restitution(1.0 - self.settings.collision_damping)
                    .friction(0.05) // Lower friction for smoother particle movement
                    .density(1.0) // Uniform density for all particles
                    .sensor(false) // Make sure it's not a sensor
                    .build();
                self.rapier.colliders.insert_with_parent(
                    collider,
                    rb_handle,
                    &mut self.rapier.bodies,
                );

                self.rapier.body_handles.push(rb_handle);
            }

            self.particles.extend(new_particles);
        } else if new_count < current_count {
            // Remove particles and corresponding Rapier bodies
            self.particles.truncate(new_count as usize);

            // Remove excess rigid bodies and colliders
            while self.rapier.body_handles.len() > new_count as usize {
                if let Some(handle) = self.rapier.body_handles.pop() {
                    self.rapier.bodies.remove(
                        handle,
                        &mut self.rapier.island_manager,
                        &mut self.rapier.colliders,
                        &mut self.rapier.impulse_joints,
                        &mut self.rapier.multibody_joints,
                        true,
                    );
                }
            }
        }

        // Update settings
        self.settings.particle_count = new_count;

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

        Ok(())
    }

    fn recreate_bind_groups(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
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

    fn update_collider_sizes(&mut self) {
        // Recreate all colliders with the new particle size
        // First, collect all collider handles to remove
        let mut collider_handles_to_remove = Vec::new();
        for handle in &self.rapier.body_handles {
            if let Some(rb) = self.rapier.bodies.get(*handle) {
                collider_handles_to_remove.extend(rb.colliders());
            }
        }

        // Remove all colliders
        for collider_handle in collider_handles_to_remove {
            self.rapier.colliders.remove(
                collider_handle,
                &mut self.rapier.island_manager,
                &mut self.rapier.bodies,
                true,
            );
        }

        // Create new colliders with updated size and proper collision properties
        for handle in self.rapier.body_handles.iter() {
            if let Some(_rb) = self.rapier.bodies.get_mut(*handle) {
                let collider = ColliderBuilder::ball(self.settings.particle_size)
                    .restitution(1.0 - self.settings.collision_damping)
                    .friction(0.05) // Lower friction for smoother particle movement
                    .density(1.0) // Uniform density for all particles
                    .sensor(false) // Make sure it's not a sensor
                    .build();
                self.rapier.colliders.insert_with_parent(
                    collider,
                    *handle,
                    &mut self.rapier.bodies,
                );
            }
        }
    }

    /// Step Rapier physics world and sync positions/velocities to self.particles
    pub fn step_rapier_and_sync_particles(&mut self) {
        // Apply mouse interaction forces before physics step
        if self.state.mouse_pressed {
            self.apply_mouse_interaction_forces();
        }

        // Step Rapier physics world with proper collision detection
        self.rapier.step();

        // Sync positions/velocities to self.particles using stored handles
        for (i, &handle) in self.rapier.body_handles.iter().enumerate() {
            if let Some(rb) = self.rapier.bodies.get(handle) {
                let pos = rb.translation();
                let vel = rb.linvel();

                // More aggressive bounds checking to prevent physics engine panic
                let clamped_pos = [pos.x.clamp(-0.99, 0.99), pos.y.clamp(-0.99, 0.99)];

                // More aggressive velocity clamping
                let clamped_vel = [vel.x.clamp(-3.0, 3.0), vel.y.clamp(-3.0, 3.0)];

                // Update particle data
                if i < self.particles.len() {
                    self.particles[i].position = clamped_pos;
                    self.particles[i].velocity = clamped_vel;
                }
            }
        }

        // Update Rapier bodies in a separate loop to avoid borrow conflicts
        for &handle in &self.rapier.body_handles {
            if let Some(rb) = self.rapier.bodies.get(handle) {
                let pos = rb.translation();
                let vel = rb.linvel();

                let clamped_pos = [pos.x.clamp(-0.99, 0.99), pos.y.clamp(-0.99, 0.99)];

                let clamped_vel = [vel.x.clamp(-3.0, 3.0), vel.y.clamp(-3.0, 3.0)];

                // Always update Rapier body to ensure it stays within bounds
                if let Some(rb_mut) = self.rapier.bodies.get_mut(handle) {
                    rb_mut.set_translation(vector![clamped_pos[0], clamped_pos[1]], true);
                    rb_mut.set_linvel(vector![clamped_vel[0], clamped_vel[1]], true);
                }
            }
        }
    }

    /// Calculate particle density for coloring based on nearby particles using spatial hashing
    fn calculate_particle_density(&mut self) {
        // Scale density radius by particle size so that tightly packed particles
        // always register as dense regardless of their absolute size
        let scaled_density_radius =
            self.settings.density_radius * (self.settings.particle_size / 0.01); // Normalize to reference size
        let density_radius_sq = scaled_density_radius * scaled_density_radius;

        // Use spatial hashing for O(n) performance instead of O(n²)
        let cell_size = scaled_density_radius * 2.0;
        let mut spatial_grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
            std::collections::HashMap::new();

        // Build spatial grid
        for (i, particle) in self.particles.iter().enumerate() {
            // Clamp positions to prevent integer overflow in cell calculation
            let clamped_x = particle.position[0].clamp(-1000.0, 1000.0);
            let clamped_y = particle.position[1].clamp(-1000.0, 1000.0);
            let cell_x = (clamped_x / cell_size).floor() as i32;
            let cell_y = (clamped_y / cell_size).floor() as i32;
            spatial_grid.entry((cell_x, cell_y)).or_default().push(i);
        }

        // Calculate density using spatial grid
        // First collect all positions to avoid borrow checker issues
        let positions: Vec<[f32; 2]> = self.particles.iter().map(|p| p.position).collect();

        for (i, particle) in self.particles.iter_mut().enumerate() {
            let mut density = 0.0;
            let pos_i = particle.position;

            // Clamp positions to prevent integer overflow in cell calculation
            let clamped_x = pos_i[0].clamp(-1000.0, 1000.0);
            let clamped_y = pos_i[1].clamp(-1000.0, 1000.0);
            let cell_x = (clamped_x / cell_size).floor() as i32;
            let cell_y = (clamped_y / cell_size).floor() as i32;

            // Check current cell and neighboring cells
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(particles_in_cell) = spatial_grid.get(&(cell_x + dx, cell_y + dy)) {
                        for &j in particles_in_cell {
                            if i != j {
                                let pos_j = positions[j];

                                // Calculate wrapped distance
                                let mut dx = pos_i[0] - pos_j[0];
                                let mut dy = pos_i[1] - pos_j[1];

                                // Wrap distances to account for toroidal space
                                if dx > 1.0 {
                                    dx -= 2.0;
                                } else if dx < -1.0 {
                                    dx += 2.0;
                                }
                                if dy > 1.0 {
                                    dy -= 2.0;
                                } else if dy < -1.0 {
                                    dy += 2.0;
                                }

                                let dist_sq = dx * dx + dy * dy;

                                if dist_sq < density_radius_sq {
                                    density += 1.0 / (1.0 + dist_sq);
                                }
                            }
                        }
                    }
                }
            }

            particle.density = density;
        }
    }

    /// Calculate particle velocity magnitude for coloring
    fn calculate_particle_velocity(&mut self) {
        for particle in &mut self.particles {
            let velocity_magnitude = (particle.velocity[0] * particle.velocity[0]
                + particle.velocity[1] * particle.velocity[1])
                .sqrt();
            // Scale velocity for better visualization (typical velocities are 0.1-0.5)
            particle.density = velocity_magnitude * 8.0; // Reuse density field for velocity
        }
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

    /// Select particles to grab based on cursor position
    fn select_particles_to_grab(&mut self, world_x: f32, world_y: f32) {
        // Clamp cursor position to valid bounds
        let cursor_x = world_x.clamp(-1.0, 1.0);
        let cursor_y = world_y.clamp(-1.0, 1.0);
        let cursor_pos = vector![cursor_x, cursor_y];
        let cursor_radius = self.state.cursor_size;

        // Clear any previously grabbed particles
        self.state.grabbed_particles.clear();

        // Find particles within cursor radius
        for (i, &handle) in self.rapier.body_handles.iter().enumerate() {
            if let Some(rb) = self.rapier.bodies.get(handle) {
                let pos = rb.translation();

                // Safety check: skip if position is invalid
                if pos.x.is_nan() || pos.y.is_nan() || pos.x.is_infinite() || pos.y.is_infinite() {
                    continue;
                }

                // Ensure position is within valid bounds
                let clamped_pos = vector![pos.x.clamp(-1.0, 1.0), pos.y.clamp(-1.0, 1.0)];

                // Calculate distance to cursor (no wrapping for grab selection)
                let dx = cursor_pos.x - clamped_pos.x;
                let dy = cursor_pos.y - clamped_pos.y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < cursor_radius * cursor_radius {
                    self.state.grabbed_particles.push(i);
                }
            }
        }

        // Limit the number of particles that can be grabbed at once to prevent performance issues
        if self.state.grabbed_particles.len() > 10 {
            self.state.grabbed_particles.truncate(10);
        }
    }

    /// Apply mouse interaction forces to particles
    fn apply_mouse_interaction_forces(&mut self) {
        // Clamp cursor position to valid bounds
        let cursor_x = self.state.mouse_position[0].clamp(-1.0, 1.0);
        let cursor_y = self.state.mouse_position[1].clamp(-1.0, 1.0);
        let cursor_pos = vector![cursor_x, cursor_y];
        let cursor_radius = self.state.cursor_size;
        let force_strength = self.state.cursor_strength;

        match self.state.mouse_mode {
            1 => {
                // Attract mode: pull nearby particles toward the cursor
                for &handle in self.rapier.body_handles.iter() {
                    if let Some(rb) = self.rapier.bodies.get_mut(handle) {
                        let pos = rb.translation();

                        // Safety check: skip if position is invalid
                        if pos.x.is_nan()
                            || pos.y.is_nan()
                            || pos.x.is_infinite()
                            || pos.y.is_infinite()
                        {
                            continue;
                        }

                        // Ensure position is within valid bounds
                        let clamped_pos = vector![pos.x.clamp(-1.0, 1.0), pos.y.clamp(-1.0, 1.0)];

                        // Calculate distance to cursor (no wrapping for attract mode)
                        let dx = cursor_pos.x - clamped_pos.x;
                        let dy = cursor_pos.y - clamped_pos.y;
                        let diff = vector![dx, dy];
                        let dist = diff.norm();

                        if dist < cursor_radius && dist > 0.001 {
                            let dt = self.rapier.integration_parameters.dt;

                            // Attraction force: weaker when closer to prevent overshooting
                            // Force decreases quadratically with proximity to provide smooth attraction
                            let distance_factor = (dist / cursor_radius).clamp(0.1, 1.0); // Minimum 10% force when very close
                            let attract_force =
                                diff.normalize() * force_strength * distance_factor * 0.3; // Scale down overall force
                            let attract_impulse = attract_force * dt; // Convert to impulse

                            // Limit impulse magnitude to prevent overshooting
                            let max_impulse = 0.02; // Much lower limit to prevent launching
                            let clamped_impulse = if attract_impulse.norm() > max_impulse {
                                attract_impulse.normalize() * max_impulse
                            } else {
                                attract_impulse
                            };

                            rb.apply_impulse(clamped_impulse, true);
                        }
                    }
                }
            }
            2 => {
                // Repel mode: push particles away from cursor
                for &handle in self.rapier.body_handles.iter() {
                    if let Some(rb) = self.rapier.bodies.get_mut(handle) {
                        let pos = rb.translation();

                        // Safety check: skip if position is invalid
                        if pos.x.is_nan()
                            || pos.y.is_nan()
                            || pos.x.is_infinite()
                            || pos.y.is_infinite()
                        {
                            continue;
                        }

                        // Ensure position is within valid bounds
                        let clamped_pos = vector![pos.x.clamp(-1.0, 1.0), pos.y.clamp(-1.0, 1.0)];

                        // Calculate distance to cursor (no wrapping for repel mode)
                        let dx = clamped_pos.x - cursor_pos.x;
                        let dy = clamped_pos.y - cursor_pos.y;
                        let diff = vector![dx, dy];
                        let dist = diff.norm();

                        if dist < cursor_radius && dist > 0.001 {
                            let dt = self.rapier.integration_parameters.dt;
                            let repel_force =
                                diff.normalize() * force_strength * (1.0 - dist / cursor_radius);
                            let repel_impulse = repel_force * dt; // Convert to impulse

                            // Safety check: limit impulse magnitude
                            let max_impulse = 0.1; // Maximum impulse per frame
                            let clamped_impulse = if repel_impulse.norm() > max_impulse {
                                repel_impulse.normalize() * max_impulse
                            } else {
                                repel_impulse
                            };

                            rb.apply_impulse(clamped_impulse, true);
                        }
                    }
                }
            }
            _ => {}
        }
    }
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
        // Step physics simulation
        self.step_rapier_and_sync_particles();

        // Calculate particle coloring based on mode
        match self.settings.coloring_mode.as_str() {
            "velocity" => self.calculate_particle_velocity(),
            _ => self.calculate_particle_density(), // Default to density
        }

        // Update particle buffer with new positions and density
        queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&self.particles),
        );

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
                    // Update Rapier gravity
                    self.rapier
                        .update_gravity(self.settings.gravitational_constant);
                    tracing::debug!("Updated Rapier gravity to {:?}", self.rapier.gravity);
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
                    // Update Rapier collider sizes
                    self.update_collider_sizes();
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
                    // Update Rapier damping on all bodies
                    self.rapier
                        .update_energy_damping(self.settings.energy_damping);
                    tracing::debug!(
                        "Updated Rapier energy damping to {}",
                        self.settings.energy_damping
                    );
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
                    // Update collider restitution for all particles
                    self.rapier
                        .update_collision_damping(self.settings.collision_damping);
                    tracing::debug!(
                        "Updated Rapier collision damping to {}",
                        self.settings.collision_damping
                    );
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

        // Store previous position before updating to new position
        if self.state.mouse_pressed {
            self.state.mouse_previous_position = self.state.mouse_position;
        } else {
            // First time pressing, set previous to current to avoid initial jump
            self.state.mouse_previous_position = [clamped_x, clamped_y];
        }

        // Encode mouse button into mode: 0 none, 1 left(grab), 2 right(repel)
        let mode = match mouse_button {
            2 => 2u32, // Right click for repulsion
            _ => 1u32, // Left/middle defaults to grab
        };

        self.state.mouse_pressed = true;
        self.state.mouse_mode = mode;
        self.state.mouse_position = [clamped_x, clamped_y];

        // If this is a grab action, select particles to grab (allow continuous grabbing)
        if mode == 1 {
            self.select_particles_to_grab(clamped_x, clamped_y);
        }

        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.state.mouse_pressed = false;
        self.state.mouse_mode = 0;

        // Release all grabbed particles
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
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Clear all existing rigid bodies and colliders by creating new instances
        self.rapier.reset(self.settings.gravitational_constant);

        // Recreate boundary walls
        self.rapier.create_boundary_walls();

        // Reinitialize particles
        self.particles = Self::initialize_particles(self.settings.particle_count, &self.settings);

        // Recreate Rapier bodies and colliders for new particles
        self.rapier.create_particle_bodies(
            &self.particles,
            self.settings.particle_size,
            self.settings.energy_damping,
            self.settings.collision_damping,
        );

        // Update particle buffer
        queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&self.particles),
        );

        // Reset camera
        self.camera.reset();

        // Reset state
        self.state.reset();

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
