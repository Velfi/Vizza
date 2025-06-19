use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::buffer_pool::BufferPool;
use super::render::{bind_group_manager::BindGroupManager, pipeline_manager::PipelineManager};
use super::settings::Settings;
use super::workgroup_optimizer::WorkgroupConfig;
use crate::simulations::shared::{LutData, LutManager, LutHandler, camera::Camera};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimSizeUniform {
    pub width: u32,
    pub height: u32,
    pub decay_rate: f32,
    pub agent_jitter: f32,
    pub agent_speed_min: f32,
    pub agent_speed_max: f32,
    pub agent_turn_rate: f32,
    pub agent_sensor_angle: f32,
    pub agent_sensor_distance: f32,
    pub diffusion_rate: f32,
    pub pheromone_deposition_rate: f32,
    pub gradient_enabled: u32,
    pub gradient_type: u32,
    pub gradient_strength: f32,
    pub gradient_center_x: f32,
    pub gradient_center_y: f32,
    pub gradient_size: f32,
    pub gradient_angle: f32,
    pub random_seed: u32,
    pub _pad1: u32,
}

impl SimSizeUniform {
    pub fn new(width: u32, height: u32, decay_rate: f32, settings: &Settings) -> Self {
        Self {
            width,
            height,
            decay_rate,
            agent_jitter: settings.agent_jitter,
            agent_speed_min: settings.agent_speed_min,
            agent_speed_max: settings.agent_speed_max,
            agent_turn_rate: settings.agent_turn_rate,
            agent_sensor_angle: settings.agent_sensor_angle,
            agent_sensor_distance: settings.agent_sensor_distance,
            diffusion_rate: settings.pheromone_diffusion_rate,
            pheromone_deposition_rate: settings.pheromone_deposition_rate,
            gradient_enabled: if settings.gradient_type == super::settings::GradientType::Disabled {
                0
            } else {
                1
            },
            gradient_type: match settings.gradient_type {
                super::settings::GradientType::Disabled => 0,
                super::settings::GradientType::Linear => 1,
                super::settings::GradientType::Radial => 2,
                super::settings::GradientType::Ellipse => 3,
                super::settings::GradientType::Spiral => 4,
                super::settings::GradientType::Checkerboard => 5,
            },
            gradient_strength: settings.gradient_strength,
            gradient_center_x: settings.gradient_center_x,
            gradient_center_y: settings.gradient_center_y,
            gradient_size: settings.gradient_size,
            gradient_angle: settings.gradient_angle,
            random_seed: settings.random_seed,
            _pad1: 0,
        }
    }
}

#[derive(Debug)]
/// SlimeMoldModel manages simulation-specific GPU resources and logic
/// while using Tauri's shared GPU context (device, queue, surface config)
pub struct SlimeMoldModel {
    // Simulation-specific GPU resources
    pub bind_group_manager: BindGroupManager,
    pub pipeline_manager: PipelineManager,
    pub agent_buffer: wgpu::Buffer,
    pub trail_map_buffer: wgpu::Buffer,
    pub gradient_buffer: wgpu::Buffer,
    pub sim_size_buffer: Arc<wgpu::Buffer>,
    pub lut_buffer: Arc<wgpu::Buffer>,
    pub display_texture: wgpu::Texture,
    pub display_view: TextureView,
    pub display_sampler: wgpu::Sampler,
    pub workgroup_config: WorkgroupConfig,
    pub buffer_pool: BufferPool,

    // Simulation state
    pub settings: Settings,
    pub agent_count: usize,
    pub lut_reversed: bool,
    pub current_lut_name: String,

    // Buffer size tracking for pool management
    pub current_trail_map_size: u64,
    pub current_gradient_buffer_size: u64,
    pub current_agent_buffer_size: u64,

    // Dimension tracking for resize scaling
    pub current_width: u32,
    pub current_height: u32,
    show_gui: bool,

    // Camera for viewport control
    pub camera: Camera,
}

impl SlimeMoldModel {
    /// Create a new slime mold simulation using Tauri's shared GPU resources
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
        agent_count: usize,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let physical_width = surface_config.width;
        let physical_height = surface_config.height;

        // Check if the trail map buffer size would exceed GPU limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let trail_map_size = (physical_width * physical_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;
        
        // If buffer would be too large, scale down the resolution
        let (effective_width, effective_height) = if trail_map_size_bytes > max_storage_buffer_size {
            let scale_factor = (max_storage_buffer_size as f64 / trail_map_size_bytes as f64).sqrt();
            let new_width = (physical_width as f64 * scale_factor * 0.95) as u32; // 95% to be safe
            let new_height = (physical_height as f64 * scale_factor * 0.95) as u32;
            tracing::warn!(
                "Trail map buffer size {} bytes exceeds GPU limit {} bytes. Scaling down from {}x{} to {}x{}",
                trail_map_size_bytes, max_storage_buffer_size, physical_width, physical_height, new_width, new_height
            );
            (new_width, new_height)
        } else {
            (physical_width, physical_height)
        };

        // Create simulation-specific buffers
        let agent_buffer = create_agent_buffer(
            device,
            agent_count,
            effective_width,
            effective_height,
            &settings,
        );

        let trail_map_size = (effective_width * effective_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;
        let trail_map_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Buffer"),
            size: trail_map_size_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        // Initialize trail map with some random values
        {
            let mut view = trail_map_buffer.slice(..).get_mapped_range_mut();
            let view_slice = bytemuck::cast_slice_mut::<u8, f32>(&mut view);
            for cell in view_slice.iter_mut() {
                *cell = rand::random::<f32>() * 0.1; // Small initial values
            }
        }
        trail_map_buffer.unmap();

        let gradient_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gradient Buffer"),
            size: trail_map_size_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create display texture
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        let texture_width = effective_width.min(max_texture_dimension);
        let texture_height = effective_height.min(max_texture_dimension);
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Display Texture"),
            size: wgpu::Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create uniform buffer
        let sim_size_uniform = SimSizeUniform::new(
            effective_width,
            effective_height,
            settings.pheromone_decay_rate,
            &settings,
        );
        let sim_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sim Size Uniform Buffer"),
            contents: bytemuck::cast_slice(&[sim_size_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sim_size_buffer = Arc::new(sim_size_buffer);

        // Create LUT buffer
        let lut_data = lut_manager.get("MATPLOTLIB_cubehelix")?;
        let lut_data_u32 = lut_data.to_u32_buffer();

        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let lut_buffer = Arc::new(lut_buffer);

        // Create display sampler
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create workgroup config
        let workgroup_config = WorkgroupConfig::new(device, adapter_info);

        // Create pipeline manager
        let pipeline_manager = PipelineManager::new(device, &workgroup_config, surface_config.format);

        // Create camera
        let camera = Camera::new(device, effective_width as f32, effective_height as f32)?;

        // Create bind group manager
        let bind_group_manager = BindGroupManager::new(
            device,
            &pipeline_manager.compute_bind_group_layout,
            &pipeline_manager.display_bind_group_layout,
            &pipeline_manager.render_bind_group_layout,
            &pipeline_manager.camera_bind_group_layout,
            &agent_buffer,
            &trail_map_buffer,
            &gradient_buffer,
            &sim_size_buffer,
            &display_view,
            &display_sampler,
            &lut_buffer,
            camera.buffer(),
        );

        // Create buffer pool
        let buffer_pool = BufferPool::new();

        let agent_buffer_size_bytes = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
        let mut simulation = Self {
            bind_group_manager,
            pipeline_manager,
            agent_buffer,
            trail_map_buffer,
            gradient_buffer,
            sim_size_buffer,
            lut_buffer,
            display_texture,
            display_view,
            display_sampler,
            workgroup_config,
            buffer_pool,
            settings,
            agent_count,
            current_lut_name: "MATPLOTLIB_cubehelix".to_string(),
            lut_reversed: true,
            current_trail_map_size: trail_map_size_bytes,
            current_gradient_buffer_size: trail_map_size_bytes,
            current_agent_buffer_size: agent_buffer_size_bytes,
            current_width: effective_width,
            current_height: effective_height,
            show_gui: false,
            camera,
        };

        if let Ok(mut lut_data) = lut_manager.get(&simulation.current_lut_name) {
            if simulation.lut_reversed {
                lut_data.reverse();
            }
            simulation.update_lut(&lut_data, queue);
        }

        // Initialize agents using GPU compute shader instead of CPU
        simulation.reset_agents(device, queue)?;

        Ok(simulation)
    }

    /// Update simulation with new surface configuration (e.g., window resize)
    pub fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let physical_width = new_config.width;
        let physical_height = new_config.height;

        // Check if the trail map buffer size would exceed GPU limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let trail_map_size = (physical_width * physical_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;
        
        // If buffer would be too large, scale down the resolution
        let (effective_width, effective_height) = if trail_map_size_bytes > max_storage_buffer_size {
            let scale_factor = (max_storage_buffer_size as f64 / trail_map_size_bytes as f64).sqrt();
            let new_width = (physical_width as f64 * scale_factor * 0.95) as u32; // 95% to be safe
            let new_height = (physical_height as f64 * scale_factor * 0.95) as u32;
            tracing::warn!(
                "Trail map buffer size {} bytes exceeds GPU limit {} bytes. Scaling down from {}x{} to {}x{}",
                trail_map_size_bytes, max_storage_buffer_size, physical_width, physical_height, new_width, new_height
            );
            (new_width, new_height)
        } else {
            (physical_width, physical_height)
        };

        // Calculate new buffer sizes
        let trail_map_size = (effective_width * effective_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;
        let agent_buffer_size_bytes = (self.agent_count * 4 * std::mem::size_of::<f32>()) as u64;

        // Return old buffers to pool before creating new ones
        let old_trail_map_buffer = std::mem::replace(
            &mut self.trail_map_buffer,
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Trail Map Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );
        self.buffer_pool.return_buffer(
            old_trail_map_buffer,
            self.current_trail_map_size,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        let old_gradient_buffer = std::mem::replace(
            &mut self.gradient_buffer,
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Gradient Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );
        self.buffer_pool.return_buffer(
            old_gradient_buffer,
            self.current_gradient_buffer_size,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        // Get new buffers from pool (or create new if none available)
        self.trail_map_buffer = self.buffer_pool.get_buffer(
            device,
            Some("Trail Map Buffer"),
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        self.gradient_buffer = self.buffer_pool.get_buffer(
            device,
            Some("Gradient Buffer"),
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        // For agent buffer, we need special handling to preserve and scale existing positions
        // Store the old buffer before replacing it
        let old_agent_buffer = std::mem::replace(
            &mut self.agent_buffer,
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Agent Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );

        // Create new agent buffer and scale existing positions
        self.agent_buffer = create_agent_buffer_with_scaling(
            &mut self.buffer_pool,
            device,
            queue,
            &old_agent_buffer,
            self.agent_count,
            self.current_width,
            self.current_height,
            effective_width,
            effective_height,
        );

        // Return old buffer to pool after scaling is complete
        self.buffer_pool.return_buffer(
            old_agent_buffer,
            self.current_agent_buffer_size,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        // Update current sizes and dimensions
        self.current_trail_map_size = trail_map_size_bytes;
        self.current_gradient_buffer_size = trail_map_size_bytes;
        self.current_agent_buffer_size = agent_buffer_size_bytes;
        self.current_width = effective_width;
        self.current_height = effective_height;

        // Recreate display texture with new dimensions
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        let texture_width = effective_width.min(max_texture_dimension);
        let texture_height = effective_height.min(max_texture_dimension);
        self.display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Display Texture"),
            size: wgpu::Extent3d {
                width: texture_width,
                height: texture_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.display_view = self
            .display_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update bind groups with new buffers and texture view
        self.recreate_bind_groups(device);

        // Resize camera
        self.camera.resize(effective_width as f32, effective_height as f32);

        Ok(())
    }

    /// Render a single frame of the simulation
    pub fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update camera for smooth movement
        self.camera.update(0.016); // Assume 60 FPS for now
        self.camera.upload_to_gpu(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Render Encoder"),
        });

        // Run compute passes for simulation
        self.run_compute_passes(&mut encoder);

        // First render to display texture
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Display Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.display_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.display_bind_group, &[]);
            let (workgroups_x, workgroups_y) = self
                .workgroup_config
                .workgroups_2d(self.display_texture.width(), self.display_texture.height());
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Then render display texture to surface
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slime Mold Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline_manager.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group_manager.render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group_manager.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Full-screen quad
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Run the compute passes for the simulation
    fn run_compute_passes(&self, encoder: &mut wgpu::CommandEncoder) {
        // Agent update pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Agent Update Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);

            // For large agent counts, use 2D dispatch to avoid 65535 workgroup limit
            let workgroup_size = self.workgroup_config.compute_2d.0 * self.workgroup_config.compute_2d.1;
            let total_workgroups = (self.agent_count as u32).div_ceil(workgroup_size);

            // Calculate 2D dispatch grid
            let max_workgroups_per_dim = 65535;
            let workgroups_x = total_workgroups.min(max_workgroups_per_dim);
            let workgroups_y = total_workgroups.div_ceil(max_workgroups_per_dim);

            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Decay pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Decay Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.decay_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);
            let workgroups_x = self.display_texture.width().div_ceil(16);
            let workgroups_y = self.display_texture.height().div_ceil(16);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Diffusion pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Diffusion Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.diffuse_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);
            let workgroups_x = self.display_texture.width().div_ceil(16);
            let workgroups_y = self.display_texture.height().div_ceil(16);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
    }

    /// Update simulation settings
    pub fn update_settings(&mut self, new_settings: Settings, queue: &Arc<Queue>) {
        self.settings = new_settings;
        update_settings(
            &self.settings,
            &self.sim_size_buffer,
            queue,
            self.display_texture.width(),
            self.display_texture.height(),
        );
    }

    /// Update the LUT (color lookup table)
    pub fn update_lut(&mut self, lut_data: &LutData, queue: &Queue) {
        let lut_data_u32 = lut_data.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
    }

    /// Reset trail map to zero
    pub fn reset_trails(&self, queue: &Arc<Queue>) {
        reset_trails(
            &self.trail_map_buffer,
            queue,
            self.display_texture.width(),
            self.display_texture.height(),
        );
    }

    /// Reset agents to random positions using GPU compute shader
    pub fn reset_agents(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update the random seed to ensure different randomization
        self.settings.random_seed = rand::random::<u32>();
        
        // Update the sim size buffer with the new random seed
        let sim_size = SimSizeUniform::new(
            self.current_width,
            self.current_height,
            self.settings.pheromone_decay_rate,
            &self.settings,
        );
        queue.write_buffer(&self.sim_size_buffer, 0, bytemuck::cast_slice(&[sim_size]));

        // Dispatch the reset agents compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Reset Agents Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reset Agents Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline_manager.reset_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);

            // For large agent counts, use 2D dispatch to avoid 65535 workgroup limit
            let workgroup_size = 64; // From shader workgroup_size(64, 1, 1)
            let total_workgroups = (self.agent_count as u32).div_ceil(workgroup_size);

            // Calculate 2D dispatch grid
            let max_workgroups_per_dim = 65535;
            let workgroups_x = total_workgroups.min(max_workgroups_per_dim);
            let workgroups_y = total_workgroups.div_ceil(max_workgroups_per_dim);

            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn seed_random_noise(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // For slime mold, seeding noise means resetting agents to random positions
        // and clearing the trail map
        self.reset_trails(queue);
        self.reset_agents(device, queue)?;
        Ok(())
    }

    /// Update agent speeds to new random values within the current min/max range
    pub fn update_agent_speeds(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) {
        tracing::info!(
            "Updating {} agent speeds to range [{}, {}]",
            self.agent_count,
            self.settings.agent_speed_min,
            self.settings.agent_speed_max
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Update Agent Speeds Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Update Agent Speeds Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.update_speeds_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);

            // For large agent counts, use 2D dispatch to avoid 65535 workgroup limit
            let workgroup_size = self.workgroup_config.compute_2d.0 * self.workgroup_config.compute_2d.1;
            let total_workgroups = (self.agent_count as u32).div_ceil(workgroup_size);

            // Calculate 2D dispatch grid
            let max_workgroups_per_dim = 65535;
            let workgroups_x = total_workgroups.min(max_workgroups_per_dim);
            let workgroups_y = total_workgroups.div_ceil(max_workgroups_per_dim);

            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        tracing::info!("GPU agent speeds update dispatch completed");
    }

    /// Update a single setting by name
    pub fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use super::settings::GradientType;

        match setting_name {
            "pheromone_decay_rate" => {
                if let Some(v) = value.as_f64() {
                    self.settings.pheromone_decay_rate = v as f32;
                }
            }
            "pheromone_deposition_rate" => {
                if let Some(v) = value.as_f64() {
                    self.settings.pheromone_deposition_rate = v as f32;
                }
            }
            "pheromone_diffusion_rate" => {
                if let Some(v) = value.as_f64() {
                    self.settings.pheromone_diffusion_rate = v as f32;
                }
            }
            "decay_frequency" => {
                if let Some(v) = value.as_u64() {
                    self.settings.decay_frequency = v as u32;
                }
            }
            "diffusion_frequency" => {
                if let Some(v) = value.as_u64() {
                    self.settings.diffusion_frequency = v as u32;
                }
            }
            "agent_speed_min" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_speed_min = v as f32;
                    // Update all agent speeds to new range
                    self.update_agent_speeds(device, queue);
                }
            }
            "agent_speed_max" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_speed_max = v as f32;
                    // Update all agent speeds to new range
                    self.update_agent_speeds(device, queue);
                }
            }
            "agent_turn_rate" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_turn_rate = v as f32;
                }
            }
            "agent_jitter" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_jitter = v as f32;
                }
            }
            "agent_sensor_angle" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_sensor_angle = v as f32;
                }
            }
            "agent_sensor_distance" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_sensor_distance = v as f32;
                }
            }
            "gradient_type" => {
                if let Some(v) = value.as_str() {
                    self.settings.gradient_type = match v {
                        "disabled" => GradientType::Disabled,
                        "linear" => GradientType::Linear,
                        "radial" => GradientType::Radial,
                        "ellipse" => GradientType::Ellipse,
                        "spiral" => GradientType::Spiral,
                        "checkerboard" => GradientType::Checkerboard,
                        _ => GradientType::Disabled,
                    };
                }
            }
            "gradient_strength" => {
                if let Some(v) = value.as_f64() {
                    self.settings.gradient_strength = v as f32;
                }
            }
            "gradient_center_x" => {
                if let Some(v) = value.as_f64() {
                    self.settings.gradient_center_x = v as f32;
                }
            }
            "gradient_center_y" => {
                if let Some(v) = value.as_f64() {
                    self.settings.gradient_center_y = v as f32;
                }
            }
            "gradient_size" => {
                if let Some(v) = value.as_f64() {
                    self.settings.gradient_size = v as f32;
                }
            }
            "gradient_angle" => {
                if let Some(v) = value.as_f64() {
                    self.settings.gradient_angle = v as f32;
                }
            }
            "random_seed" => {
                if let Some(v) = value.as_u64() {
                    self.settings.random_seed = v as u32;
                }
            }
            _ => {
                return Err(format!("Unknown setting: {}", setting_name).into());
            }
        }

        // Update the GPU uniforms with the new settings
        update_settings(
            &self.settings,
            &self.sim_size_buffer,
            queue,
            self.display_texture.width(),
            self.display_texture.height(),
        );

        Ok(())
    }

    /// Update agent count (requires buffer recreation)
    pub async fn update_agent_count(
        &mut self,
        count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.agent_count = count as usize;

        // Recreate the agent buffer with new count
        let agent_buffer_size_bytes = (self.agent_count * 4 * std::mem::size_of::<f32>()) as u64;

        // Return old buffer to pool
        let old_agent_buffer = std::mem::replace(
            &mut self.agent_buffer,
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Agent Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );
        self.buffer_pool.return_buffer(
            old_agent_buffer,
            self.current_agent_buffer_size,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        // Create new agent buffer with new count
        let physical_width = surface_config.width;
        let physical_height = surface_config.height;

        self.agent_buffer = create_agent_buffer_pooled(
            &mut self.buffer_pool,
            device,
            queue,
            self.agent_count,
            physical_width,
            physical_height,
            &self.settings,
        );

        self.current_agent_buffer_size = agent_buffer_size_bytes;

        // Recreate bind groups with new agent buffer
        self.recreate_bind_groups(device);

        // Initialize agents using GPU compute shader
        self.reset_agents(device, queue)?;

        Ok(())
    }

    /// Recreate bind groups (called after buffer/texture changes)
    fn recreate_bind_groups(&mut self, device: &Arc<Device>) {
        self.bind_group_manager = BindGroupManager::new(
            device,
            &self.pipeline_manager.compute_bind_group_layout,
            &self.pipeline_manager.display_bind_group_layout,
            &self.pipeline_manager.render_bind_group_layout,
            &self.pipeline_manager.camera_bind_group_layout,
            &self.agent_buffer,
            &self.trail_map_buffer,
            &self.gradient_buffer,
            &self.sim_size_buffer,
            &self.display_view,
            &self.display_sampler,
            &self.lut_buffer,
            self.camera.buffer(),
        );
    }

    pub(crate) fn toggle_gui(&mut self) -> bool {
        self.show_gui = !self.show_gui;
        self.show_gui
    }

    pub(crate) fn is_gui_visible(&self) -> bool {
        self.show_gui
    }

    // Camera control methods
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        tracing::debug!("Slime mold pan_camera called: delta=({:.2}, {:.2})", delta_x, delta_y);
        self.camera.pan(delta_x, delta_y);
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        tracing::debug!("Slime mold zoom_camera called: delta={:.2}", delta);
        self.camera.zoom(delta);
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        tracing::debug!("Slime mold zoom_camera_to_cursor called: delta={:.2}, cursor=({:.2}, {:.2})", delta, cursor_x, cursor_y);
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    pub fn reset_camera(&mut self) {
        tracing::debug!("Slime mold reset_camera called");
        self.camera.reset();
    }
}

impl Drop for SlimeMoldModel {
    fn drop(&mut self) {
        // Clean up buffer pool
        self.buffer_pool.clear();
    }
}

impl LutHandler for SlimeMoldModel {
    fn get_name_of_active_lut(&self) -> &str {
        &self.current_lut_name
    }

    fn is_lut_reversed(&self) -> bool {
        self.lut_reversed
    }

    fn set_lut_reversed(&mut self, reversed: bool) {
        self.lut_reversed = reversed;
    }

    fn set_active_lut(&mut self, lut_data: &LutData, queue: &Queue) {
        let lut_data_to_apply = if self.lut_reversed {
            lut_data.reversed()
        } else {
            lut_data.clone()
        };
        
        let lut_data_u32 = lut_data_to_apply.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
        self.current_lut_name = lut_data.name.clone();
    }
}

impl crate::simulations::traits::Simulation for SlimeMoldModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.render_frame(device, queue, surface_view)
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.resize(device, queue, new_config)
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.update_setting(setting_name, value, device, queue)
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&self.settings).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::json!({
            "agent_count": self.agent_count,
            "current_width": self.current_width,
            "current_height": self.current_height,
            "lut_reversed": self.lut_reversed,
            "current_lut_name": self.current_lut_name,
            "show_gui": self.show_gui,
            "camera": {
                "position": self.camera.position,
                "zoom": self.camera.zoom
            }
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _is_seeding: bool,
        _queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Slime mold doesn't currently support mouse interaction
        // This could be implemented in the future for seeding agents
        Ok(())
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.pan_camera(delta_x, delta_y);
    }

    fn zoom_camera(&mut self, delta: f32) {
        self.zoom_camera(delta);
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.zoom_camera_to_cursor(delta, cursor_x, cursor_y);
    }

    fn reset_camera(&mut self) {
        self.reset_camera();
    }

    fn get_camera_state(&self) -> serde_json::Value {
        serde_json::json!({
            "position": [self.camera.position[0], self.camera.position[1]],
            "zoom": self.camera.zoom
        })
    }

    fn save_preset(&self, _preset_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // This would need to be implemented with the preset manager
        // For now, we'll return an error indicating it needs to be implemented
        Err("Preset saving not yet implemented for SlimeMoldModel".into())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        // This would need to be implemented with the preset manager
        // For now, we'll return an error indicating it needs to be implemented
        Err("Preset loading not yet implemented for SlimeMoldModel".into())
    }

    fn reset_runtime_state(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_trails(queue);
        // Note: reset_agents requires a device, but we don't have one in this context
        // This is a limitation of the current trait design
        // In practice, this would be called from a context where we have access to the device
        Ok(())
    }

    fn get_simulation_type(&self) -> &str {
        "slime_mold"
    }

    fn is_running(&self) -> bool {
        true // SlimeMoldModel is always considered running when instantiated
    }

    fn get_agent_count(&self) -> Option<u32> {
        Some(self.agent_count as u32)
    }

    async fn update_agent_count(
        &mut self,
        count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.update_agent_count(count, device, queue, surface_config).await
    }

    fn toggle_gui(&mut self) -> bool {
        self.toggle_gui()
    }

    fn is_gui_visible(&self) -> bool {
        self.is_gui_visible()
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Randomize the settings
        self.settings.randomize();
        self.update_settings(self.settings.clone(), queue);
        Ok(())
    }

    fn apply_settings(&mut self, settings: serde_json::Value, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        let new_settings: Settings = serde_json::from_value(settings)?;
        self.update_settings(new_settings, queue);
        Ok(())
    }
}

// Helper functions (moved from gpu_state.rs)

fn create_agent_buffer(
    device: &wgpu::Device,
    agent_count: usize,
    _physical_width: u32,
    _physical_height: u32,
    _settings: &Settings,
) -> wgpu::Buffer {
    // Create buffer without CPU initialization - GPU will initialize via reset shader
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Agent Buffer"),
        size: (agent_count * 4 * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_agent_buffer_pooled(
    buffer_pool: &mut BufferPool,
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    agent_count: usize,
    _physical_width: u32,
    _physical_height: u32,
    _settings: &Settings,
) -> wgpu::Buffer {
    let size = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
    let usage =
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;

    // Get buffer from pool - GPU will initialize via reset shader
    buffer_pool.get_buffer(device, Some("Agent Buffer"), size, usage)
}

fn create_agent_buffer_with_scaling(
    buffer_pool: &mut BufferPool,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    old_buffer: &wgpu::Buffer,
    agent_count: usize,
    old_width: u32,
    old_height: u32,
    new_width: u32,
    new_height: u32,
) -> wgpu::Buffer {
    let size = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
    let usage =
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;

    // Get new buffer from pool
    let new_buffer = buffer_pool.get_buffer(device, Some("Scaled Agent Buffer"), size, usage);

    // Calculate scaling factors
    let scale_x = new_width as f32 / old_width as f32;
    let scale_y = new_height as f32 / old_height as f32;

    // Create read staging buffer
    let read_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Agent Scaling Read Staging Buffer"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Create write staging buffer
    let write_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Agent Scaling Write Staging Buffer"),
        size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: true,
    });

    // Copy old buffer to read staging
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Agent Scaling Copy Old"),
    });
    encoder.copy_buffer_to_buffer(old_buffer, 0, &read_staging_buffer, 0, size);
    queue.submit(std::iter::once(encoder.finish()));

    // Wait for copy to complete and map for reading
    let (sender, receiver) = std::sync::mpsc::channel();
    read_staging_buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    device.poll(wgpu::Maintain::Wait);
    receiver.recv().unwrap().unwrap();

    // Read old data and scale positions
    {
        let buffer_slice = read_staging_buffer.slice(..).get_mapped_range();
        let old_agent_data: &[f32] = bytemuck::cast_slice(&buffer_slice);

        let mut write_buffer_slice = write_staging_buffer.slice(..).get_mapped_range_mut();
        let new_agent_data: &mut [f32] = bytemuck::cast_slice_mut(&mut write_buffer_slice);

        for i in 0..agent_count {
            let base_idx = i * 4;

            // Scale X and Y positions, clamp to new boundaries
            new_agent_data[base_idx] = (old_agent_data[base_idx] * scale_x)
                .min(new_width as f32)
                .max(0.0);
            new_agent_data[base_idx + 1] = (old_agent_data[base_idx + 1] * scale_y)
                .min(new_height as f32)
                .max(0.0);

            // Keep angle unchanged
            new_agent_data[base_idx + 2] = old_agent_data[base_idx + 2];

            // Keep speed unchanged
            new_agent_data[base_idx + 3] = old_agent_data[base_idx + 3];
        }

        drop(write_buffer_slice);
        write_staging_buffer.unmap();
    }

    read_staging_buffer.unmap();

    // Copy scaled data to final buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Agent Scaling Copy New"),
    });
    encoder.copy_buffer_to_buffer(&write_staging_buffer, 0, &new_buffer, 0, size);
    queue.submit(std::iter::once(encoder.finish()));

    new_buffer
}

fn reset_trails(
    trail_map_buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    physical_width: u32,
    physical_height: u32,
) {
    let size = (physical_width * physical_height) as usize * std::mem::size_of::<f32>();
    let zero_data = vec![0u8; size];
    queue.write_buffer(trail_map_buffer, 0, &zero_data);
}

fn update_settings(
    settings: &Settings,
    sim_size_buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    physical_width: u32,
    physical_height: u32,
) {
    let sim_size_uniform = SimSizeUniform::new(
        physical_width,
        physical_height,
        settings.pheromone_decay_rate,
        settings,
    );
    queue.write_buffer(
        sim_size_buffer,
        0,
        bytemuck::cast_slice(&[sim_size_uniform]),
    );
}
