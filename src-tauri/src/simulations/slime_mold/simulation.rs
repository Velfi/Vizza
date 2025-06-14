use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::buffer_pool::BufferPool;
use super::render::{bind_group_manager::BindGroupManager, pipeline_manager::PipelineManager};
use super::settings::Settings;
use super::workgroup_optimizer::WorkgroupConfig;
use crate::simulations::shared::{LutData, LutManager};

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

/// SlimeMoldSimulation manages simulation-specific GPU resources and logic
/// while using Tauri's shared GPU context (device, queue, surface config)
pub struct SlimeMoldSimulation {
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
    pub current_lut_index: usize,
    pub lut_reversed: bool,

    // Buffer size tracking for pool management
    pub current_trail_map_size: u64,
    pub current_gradient_buffer_size: u64,
    pub current_agent_buffer_size: u64,

    // Dimension tracking for resize scaling
    pub current_width: u32,
    pub current_height: u32,
}

impl SlimeMoldSimulation {
    /// Create a new slime mold simulation using Tauri's shared GPU resources
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
        agent_count: usize,
        settings: Settings,
        lut_manager: &LutManager,
        available_luts: &[String],
        current_lut_index: usize,
        lut_reversed: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let physical_width = surface_config.width;
        let physical_height = surface_config.height;

        // Create simulation-specific buffers
        let agent_buffer = create_agent_buffer(
            device,
            agent_count,
            physical_width,
            physical_height,
            &settings,
        );

        let trail_map_size = (physical_width * physical_height) as usize;
        let trail_map_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Buffer"),
            size: (trail_map_size * std::mem::size_of::<f32>()) as u64,
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
            size: (trail_map_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create display texture
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        let texture_width = physical_width.min(max_texture_dimension);
        let texture_height = physical_height.min(max_texture_dimension);
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
            physical_width,
            physical_height,
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
        let lut_data = if current_lut_index < available_luts.len() {
            lut_manager.load_lut(&available_luts[current_lut_index])?
        } else {
            return Err("Invalid LUT index".into());
        };

        let mut lut_data_combined = Vec::with_capacity(768);
        lut_data_combined.extend_from_slice(&lut_data.red);
        lut_data_combined.extend_from_slice(&lut_data.green);
        lut_data_combined.extend_from_slice(&lut_data.blue);
        let lut_data_u32: Vec<u32> = lut_data_combined.iter().map(|&x| x as u32).collect();

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
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create workgroup config
        let workgroup_config = WorkgroupConfig::new(device, adapter_info);

        // Create pipeline manager
        let pipeline_manager = PipelineManager::new(device, &workgroup_config);

        // Create bind group manager
        let bind_group_manager = BindGroupManager::new(
            device,
            &pipeline_manager.compute_bind_group_layout,
            &pipeline_manager.display_bind_group_layout,
            &pipeline_manager.render_bind_group_layout,
            &agent_buffer,
            &trail_map_buffer,
            &gradient_buffer,
            &sim_size_buffer,
            &display_view,
            &display_sampler,
            &lut_buffer,
        );

        // Create buffer pool
        let buffer_pool = BufferPool::new();

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
            current_lut_index,
            lut_reversed,
            current_trail_map_size: trail_map_size as u64,
            current_gradient_buffer_size: trail_map_size as u64,
            current_agent_buffer_size: agent_count as u64,
            current_width: physical_width,
            current_height: physical_height,
        };

        // Initialize agents using GPU compute shader instead of CPU
        simulation.reset_agents(device, queue);

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

        // Calculate new buffer sizes
        let trail_map_size = (physical_width * physical_height) as usize;
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
            physical_width,
            physical_height,
        );

        // Return old buffer to pool
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
        self.current_width = physical_width;
        self.current_height = physical_height;

        // Recreate display texture with new dimensions
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        let texture_width = physical_width.min(max_texture_dimension);
        let texture_height = physical_height.min(max_texture_dimension);
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

        Ok(())
    }

    /// Render a single frame of the simulation
    pub fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Render Encoder"),
        });

        // Run compute passes for simulation
        self.run_compute_passes(&mut encoder);

        // First render to display texture
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Display Pass"),
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
                label: Some("Agent Update Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);
            
            // For large agent counts, use 2D dispatch to avoid 65535 workgroup limit
            let workgroup_size = 16 * 16; // 256 threads per workgroup
            let total_workgroups = (self.agent_count as u32 + workgroup_size - 1) / workgroup_size;
            
            // Calculate 2D dispatch grid
            let max_workgroups_per_dim = 65535;
            let workgroups_x = total_workgroups.min(max_workgroups_per_dim);
            let workgroups_y = (total_workgroups + max_workgroups_per_dim - 1) / max_workgroups_per_dim;
            
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Decay pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Decay Pass"),
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
                label: Some("Diffusion Pass"),
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
    pub fn update_lut(&mut self, lut_data: &LutData, queue: &Arc<Queue>) {
        let mut lut_data_combined = Vec::with_capacity(768);
        lut_data_combined.extend_from_slice(&lut_data.red);
        lut_data_combined.extend_from_slice(&lut_data.green);
        lut_data_combined.extend_from_slice(&lut_data.blue);
        let lut_data_u32: Vec<u32> = lut_data_combined.iter().map(|&x| x as u32).collect();
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
    pub fn reset_agents(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) {
        tracing::info!("Resetting {} agents using GPU compute shader", self.agent_count);
        
        // Generate a new random seed for this reset
        let new_seed = rand::random::<u32>();
        self.settings.random_seed = new_seed;
        
        // Update the settings buffer with the new seed
        update_settings(
            &self.settings,
            &self.sim_size_buffer,
            queue,
            self.display_texture.width(),
            self.display_texture.height(),
        );
        
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
            
            // Calculate workgroups for agents (using 2D dispatch to handle large counts)
            let workgroup_size = 64; // From shader workgroup_size
            let total_workgroups = (self.agent_count as u32 + workgroup_size - 1) / workgroup_size;
            
            // GPU workgroup limit is 65535 per dimension, so use 2D dispatch if needed
            let max_workgroups_per_dim = 65535;
            let workgroups_x = total_workgroups.min(max_workgroups_per_dim);
            let workgroups_y = (total_workgroups + max_workgroups_per_dim - 1) / max_workgroups_per_dim;
            
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        tracing::info!("GPU agent reset dispatch completed with seed: {}", new_seed);
    }

    /// Update a single setting by name
    pub fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
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
                }
            }
            "agent_speed_max" => {
                if let Some(v) = value.as_f64() {
                    self.settings.agent_speed_max = v as f32;
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
        self.reset_agents(device, queue);

        Ok(())
    }

    /// Recreate bind groups (called after buffer/texture changes)
    fn recreate_bind_groups(&mut self, device: &Arc<Device>) {
        self.bind_group_manager = BindGroupManager::new(
            device,
            &self.pipeline_manager.compute_bind_group_layout,
            &self.pipeline_manager.display_bind_group_layout,
            &self.pipeline_manager.render_bind_group_layout,
            &self.agent_buffer,
            &self.trail_map_buffer,
            &self.gradient_buffer,
            &self.sim_size_buffer,
            &self.display_view,
            &self.display_sampler,
            &self.lut_buffer,
        );
    }
}

impl Drop for SlimeMoldSimulation {
    fn drop(&mut self) {
        // Clean up buffer pool
        self.buffer_pool.clear();
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

    // Create staging buffer to read old data and write scaled data
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Agent Scaling Staging Buffer"),
        size,
        usage: wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::MAP_READ
            | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: false,
    });

    // Copy old buffer to staging for reading
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Agent Scaling Copy Old"),
    });
    encoder.copy_buffer_to_buffer(old_buffer, 0, &staging_buffer, 0, size);
    queue.submit(std::iter::once(encoder.finish()));

    // Wait for copy to complete and map for reading
    let (sender, receiver) = std::sync::mpsc::channel();
    staging_buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
    device.poll(wgpu::Maintain::Wait);
    receiver.recv().unwrap().unwrap();

    // Read old data and scale positions
    {
        let buffer_slice = staging_buffer.slice(..).get_mapped_range();
        let old_agent_data: &[f32] = bytemuck::cast_slice(&buffer_slice);

        // Create new buffer for scaled data
        let new_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Agent Scaling New Staging Buffer"),
            size,
            usage: wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });

        let mut new_buffer_slice = new_staging_buffer.slice(..).get_mapped_range_mut();
        let new_agent_data: &mut [f32] = bytemuck::cast_slice_mut(&mut new_buffer_slice);

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

        drop(new_buffer_slice);
        new_staging_buffer.unmap();

        // Copy scaled data to final buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Agent Scaling Copy New"),
        });
        encoder.copy_buffer_to_buffer(&new_staging_buffer, 0, &new_buffer, 0, size);
        queue.submit(std::iter::once(encoder.finish()));
    }

    staging_buffer.unmap();

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
