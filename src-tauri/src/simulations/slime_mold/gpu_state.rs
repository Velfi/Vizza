use super::lut_manager::{LutManager, LutData};
use super::settings::Settings;
use super::simulation::SimSizeUniform;
use super::workgroup_optimizer::WorkgroupConfig;
use super::buffer_pool::{BufferPool, BufferPoolStats};
use super::render::{bind_group_manager::BindGroupManager, pipeline_manager::PipelineManager};
use std::sync::Arc;
use tracing::debug;
use wgpu::util::DeviceExt;
use wgpu::{
    Backends, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureUsages,
    TextureView,
};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window};

pub struct GpuState {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub config: SurfaceConfiguration,
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
    pub current_trail_map_size: u64,
    pub current_gradient_buffer_size: u64,
    pub current_agent_buffer_size: u64,
}

impl GpuState {
    pub async fn new(
        event_loop: &ActiveEventLoop,
        window_width: u32,
        window_height: u32,
        window_fullscreen: bool,
        agent_count: usize,
        settings: &Settings,
        lut_manager: &LutManager,
        available_luts: &[String],
        current_lut_index: usize,
        lut_reversed: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create window
        let mut attributes = Window::default_attributes()
            .with_title("Physarum Simulation")
            .with_inner_size(winit::dpi::LogicalSize::new(window_width, window_height));
        if window_fullscreen {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let window = Arc::new(event_loop.create_window(attributes)?);

        // Initialize wgpu
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No compatible adapter found")?;

        let adapter_limits = adapter.limits();
        debug!(
            "Adapter max buffer size: {}",
            adapter_limits.max_buffer_size
        );
        debug!(
            "Adapter max storage buffer binding size: {}",
            adapter_limits.max_storage_buffer_binding_size
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    memory_hints: wgpu::MemoryHints::default(),
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits {
                        max_buffer_size: adapter_limits.max_buffer_size,
                        max_storage_buffer_binding_size: adapter_limits
                            .max_storage_buffer_binding_size,
                        ..wgpu::Limits::default()
                    },
                },
                None,
            )
            .await?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        debug!("Max texture dimension: {}", max_texture_dimension);

        let max_agents =
            (device.limits().max_buffer_size / (4 * std::mem::size_of::<f32>() as u64)) as usize;
        debug!("Max agents based on device limits: {}", max_agents);

        let scale_factor = window.scale_factor();
        let physical_width = (window_width as f64 * scale_factor) as u32;
        let physical_height = (window_height as f64 * scale_factor) as u32;

        // Configure the surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = wgpu::TextureFormat::Bgra8Unorm;
        let config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: physical_width,
            height: physical_height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create buffers
        let agent_buffer = create_agent_buffer(
            &device,
            agent_count,
            physical_width,
            physical_height,
            settings,
        );

        let trail_map_size = (physical_width * physical_height) as usize;
        let trail_map_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Buffer"),
            size: (trail_map_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gradient_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gradient Buffer"),
            size: (trail_map_size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create display texture
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
            settings,
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

        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data.red),
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
        let workgroup_config = WorkgroupConfig::new(&device, &adapter.get_info());

        // Create pipeline manager
        let pipeline_manager = PipelineManager::new(&device, &workgroup_config);

        // Create bind group manager
        let bind_group_manager = BindGroupManager::new(
            &device,
            &pipeline_manager.compute_bind_group_layout,
            &pipeline_manager.gradient_bind_group_layout,
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

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
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
            current_trail_map_size: trail_map_size as u64,
            current_gradient_buffer_size: trail_map_size as u64,
            current_agent_buffer_size: agent_count as u64,
        })
    }

    pub fn recreate_agent_buffer(&mut self, agent_count: usize, settings: &Settings) {
        // Return old buffer to pool
        let old_agent_buffer = std::mem::replace(
            &mut self.agent_buffer,
            // Temporary placeholder
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Agent Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        );
        self.buffer_pool.return_buffer(
            old_agent_buffer,
            self.current_agent_buffer_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        // Create new buffer using pool
        self.agent_buffer = create_agent_buffer_pooled(
            &mut self.buffer_pool,
            &self.device,
            &self.queue,
            agent_count,
            self.config.width,
            self.config.height,
            settings,
        );
        
        // Update current size
        self.current_agent_buffer_size = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
        
        self.recreate_bind_groups();
    }

    pub fn resize_buffers(&mut self, agent_count: usize, settings: &Settings) {
        // Buffer pools implemented! This significantly improves performance during buffer size changes
        self.config.width = self.window.inner_size().width;
        self.config.height = self.window.inner_size().height;
        self.surface.configure(&self.device, &self.config);

        // Calculate new buffer sizes
        let trail_map_size = (self.config.width * self.config.height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;
        let agent_buffer_size_bytes = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;

        // Return old buffers to pool before creating new ones
        let old_trail_map_buffer = std::mem::replace(
            &mut self.trail_map_buffer,
            // Temporary placeholder - will be replaced immediately
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Trail Map Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        );
        self.buffer_pool.return_buffer(
            old_trail_map_buffer,
            self.current_trail_map_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        let old_gradient_buffer = std::mem::replace(
            &mut self.gradient_buffer,
            // Temporary placeholder - will be replaced immediately
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Gradient Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        );
        self.buffer_pool.return_buffer(
            old_gradient_buffer,
            self.current_gradient_buffer_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        let old_agent_buffer = std::mem::replace(
            &mut self.agent_buffer,
            // Temporary placeholder - will be replaced immediately
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Agent Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        );
        self.buffer_pool.return_buffer(
            old_agent_buffer,
            self.current_agent_buffer_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        // Get new buffers from pool (or create new if none available)
        self.trail_map_buffer = self.buffer_pool.get_buffer(
            &self.device,
            Some("Trail Map Buffer"),
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        self.gradient_buffer = self.buffer_pool.get_buffer(
            &self.device,
            Some("Gradient Buffer"),
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        // For agent buffer, we need special handling since it needs initialization
        self.agent_buffer = create_agent_buffer_pooled(
            &mut self.buffer_pool,
            &self.device,
            &self.queue,
            agent_count,
            self.config.width,
            self.config.height,
            settings,
        );

        // Update current sizes
        self.current_trail_map_size = trail_map_size_bytes;
        self.current_gradient_buffer_size = trail_map_size_bytes;
        self.current_agent_buffer_size = agent_buffer_size_bytes;

        // Recreate display texture with new dimensions
        let max_texture_dimension = self.device.limits().max_texture_dimension_2d;
        let texture_width = self.config.width.min(max_texture_dimension);
        let texture_height = self.config.height.min(max_texture_dimension);
        self.display_texture = self.device.create_texture(&wgpu::TextureDescriptor {
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
        self.recreate_bind_groups();
    }

    pub fn update_lut(&mut self, lut_data: &LutData) {
        let mut lut_data_combined = Vec::with_capacity(768);
        lut_data_combined.extend_from_slice(&lut_data.red);
        lut_data_combined.extend_from_slice(&lut_data.green);
        lut_data_combined.extend_from_slice(&lut_data.blue);
        let lut_data_u32: Vec<u32> = lut_data_combined.iter().map(|&x| x as u32).collect();
        self.queue
            .write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
    }

    pub fn reset_trails(&self) {
        reset_trails(
            &self.trail_map_buffer,
            &self.queue,
            self.config.width,
            self.config.height,
        );
    }

    pub fn reset_agents(&self, settings: &Settings, agent_count: usize) {
        reset_agents(
            &self.agent_buffer,
            &self.queue,
            self.config.width,
            self.config.height,
            settings,
            agent_count,
        );
    }

    pub fn update_settings(&self, settings: &Settings) {
        update_settings(
            settings,
            &self.sim_size_buffer,
            &self.queue,
            self.config.width,
            self.config.height,
        );
    }

    pub fn reassign_agent_speeds(&self, agent_count: usize) {
        reassign_agent_speeds_gpu(
            &self.device,
            &self.queue,
            &self.pipeline_manager,
            &self.bind_group_manager,
            &self.workgroup_config,
            agent_count,
        );
    }

    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn create_command_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }

    pub fn submit(&self, command_buffer: wgpu::CommandBuffer) {
        self.queue.submit(std::iter::once(command_buffer));
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn bind_group_manager(&self) -> &BindGroupManager {
        &self.bind_group_manager
    }

    pub fn pipeline_manager(&self) -> &PipelineManager {
        &self.pipeline_manager
    }

    pub fn workgroup_config(&self) -> &WorkgroupConfig {
        &self.workgroup_config
    }

    /// Get buffer pool statistics for debugging
    pub fn buffer_pool_stats(&self) -> BufferPoolStats {
        self.buffer_pool.memory_stats()
    }

    /// Clear buffer pool (useful for freeing memory)
    pub fn clear_buffer_pool(&mut self) {
        self.buffer_pool.clear();
    }

    fn recreate_bind_groups(&mut self) {
        self.bind_group_manager = BindGroupManager::new(
            &self.device,
            &self.pipeline_manager.compute_bind_group_layout,
            &self.pipeline_manager.gradient_bind_group_layout,
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

impl Drop for GpuState {
    fn drop(&mut self) {
        // Return all buffers to pool before dropping
        // This isn't strictly necessary since everything will be dropped anyway,
        // but it's good practice and helps with debugging buffer pool usage
        
        debug!("Dropping GpuState, returning buffers to pool");
        
        // Create temporary placeholder buffers for the std::mem::replace calls
        let temp_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Drop Temp Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        
        let trail_map_buffer = std::mem::replace(&mut self.trail_map_buffer, temp_buffer.clone());
        self.buffer_pool.return_buffer(
            trail_map_buffer,
            self.current_trail_map_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        
        let gradient_buffer = std::mem::replace(&mut self.gradient_buffer, temp_buffer.clone());
        self.buffer_pool.return_buffer(
            gradient_buffer,
            self.current_gradient_buffer_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        
        let agent_buffer = std::mem::replace(&mut self.agent_buffer, temp_buffer);
        self.buffer_pool.return_buffer(
            agent_buffer,
            self.current_agent_buffer_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        
        let stats = self.buffer_pool.memory_stats();
        debug!("Buffer pool stats at drop: {:?}", stats);
    }
}

// Helper functions that need to be accessible
use bytemuck::cast_slice_mut;
use rand;

fn create_agent_buffer(
    device: &Device,
    agent_count: usize,
    physical_width: u32,
    physical_height: u32,
    settings: &Settings,
) -> wgpu::Buffer {
    let agent_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Agent Buffer"),
        size: (agent_count * 4 * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: true,
    });
    initialize_agent_buffer(&agent_buffer, agent_count, physical_width, physical_height, settings);
    agent_buffer
}

fn create_agent_buffer_pooled(
    buffer_pool: &mut BufferPool,
    device: &Device,
    queue: &Queue,
    agent_count: usize,
    physical_width: u32,
    physical_height: u32,
    settings: &Settings,
) -> wgpu::Buffer {
    let size = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST;
    
    // Get buffer from pool
    let agent_buffer = buffer_pool.get_buffer(device, Some("Agent Buffer"), size, usage);
    
    // We need to create a temporary buffer for initialization since pooled buffers
    // are not mapped at creation
    let temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Temp Agent Init Buffer"),
        size,
        usage: wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: true,
    });
    
    initialize_agent_buffer(&temp_buffer, agent_count, physical_width, physical_height, settings);
    
    // Copy from temp buffer to the pooled buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Agent Buffer Init Copy"),
    });
    encoder.copy_buffer_to_buffer(&temp_buffer, 0, &agent_buffer, 0, size);
    queue.submit(Some(encoder.finish()));
    
    agent_buffer
}

fn initialize_agent_buffer(
    buffer: &wgpu::Buffer,
    agent_count: usize,
    physical_width: u32,
    physical_height: u32,
    settings: &Settings,
) {
    // Initialize agents with random positions and angles
    {
        let mut agent_data = buffer.slice(..).get_mapped_range_mut();
        let agent_f32: &mut [f32] = cast_slice_mut(&mut agent_data);
        for i in 0..agent_count {
            let offset = i * 4;
            agent_f32[offset] = rand::random::<f32>() * physical_width as f32;
            agent_f32[offset + 1] = rand::random::<f32>() * physical_height as f32;
            // Use the agent_possible_starting_headings range from settings
            let heading_range = settings.agent_possible_starting_headings.end
                - settings.agent_possible_starting_headings.start;
            let heading_radians = (settings.agent_possible_starting_headings.start
                + rand::random::<f32>() * heading_range)
                * std::f32::consts::PI
                / 180.0;
            agent_f32[offset + 2] = heading_radians;
            let speed_range = settings.agent_speed_max - settings.agent_speed_min;
            agent_f32[offset + 3] = settings.agent_speed_min + rand::random::<f32>() * speed_range;
        }
    }
    buffer.unmap();
}

fn reset_trails(
    trail_map_buffer: &wgpu::Buffer,
    queue: &Queue,
    physical_width: u32,
    physical_height: u32,
) {
    let trail_map_size = (physical_width * physical_height) as usize;
    let clear_buffer = vec![0.0f32; trail_map_size];
    queue.write_buffer(trail_map_buffer, 0, bytemuck::cast_slice(&clear_buffer));
}

fn reset_agents(
    agent_buffer: &wgpu::Buffer,
    queue: &Queue,
    physical_width: u32,
    physical_height: u32,
    settings: &Settings,
    agent_count: usize,
) {
    let mut agent_data = Vec::with_capacity(agent_count * 4);
    for _i in 0..agent_count {
        // New random position
        agent_data.push(rand::random::<f32>() * physical_width as f32);
        agent_data.push(rand::random::<f32>() * physical_height as f32);

        // New random heading
        let heading_range = settings.agent_possible_starting_headings.end
            - settings.agent_possible_starting_headings.start;
        let heading_radians = (settings.agent_possible_starting_headings.start
            + rand::random::<f32>() * heading_range)
            * std::f32::consts::PI
            / 180.0;
        agent_data.push(heading_radians);

        // New random speed
        let speed_range = settings.agent_speed_max - settings.agent_speed_min;
        agent_data.push(settings.agent_speed_min + rand::random::<f32>() * speed_range);
    }

    queue.write_buffer(agent_buffer, 0, bytemuck::cast_slice(&agent_data));
}

fn update_settings(
    settings: &Settings,
    sim_size_buffer: &wgpu::Buffer,
    queue: &Queue,
    physical_width: u32,
    physical_height: u32,
) {
    let sim_size_uniform = SimSizeUniform::new(
        physical_width,
        physical_height,
        settings.pheromone_decay_rate,
        settings,
    );
    queue.write_buffer(sim_size_buffer, 0, bytemuck::bytes_of(&sim_size_uniform));
}

fn reassign_agent_speeds_gpu(
    device: &Device,
    queue: &Queue,
    pipeline_manager: &PipelineManager,
    bind_group_manager: &BindGroupManager,
    workgroup_config: &WorkgroupConfig,
    agent_count: usize,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Agent Speed Update Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Agent Speed Update Pass"),
            timestamp_writes: None,
        });

        // Use the dedicated speed update pipeline
        cpass.set_pipeline(&pipeline_manager.speed_update_pipeline);
        cpass.set_bind_group(0, &bind_group_manager.compute_bind_group, &[]);
        cpass.dispatch_workgroups(
            workgroup_config.workgroups_1d(agent_count as u32),
            1,
            1,
        );
    }

    queue.submit(Some(encoder.finish()));
}
