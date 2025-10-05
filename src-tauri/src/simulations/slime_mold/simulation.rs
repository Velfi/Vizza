use crate::commands::app_settings::AppSettings;
use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::buffer_pool::BufferPool;
use super::render::{bind_group_manager::BindGroupManager, pipeline_manager::PipelineManager};
use super::settings::Settings;
use super::state::{MaskPattern, MaskTarget, State as SlimeMoldState};
use super::workgroup_optimizer::WorkgroupConfig;
use crate::simulations::shared::ImageFitMode;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::post_processing::{PostProcessingResources, PostProcessingState};
use crate::simulations::shared::{
    ColorScheme, ColorSchemeManager, camera::Camera, ping_pong_buffers::PingPongBuffers,
};

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
    pub mask_pattern: u32,

    pub mask_target: u32,
    pub mask_strength: f32,
    pub mask_curve: f32,
    pub mask_mirror_horizontal: u32,

    pub mask_mirror_vertical: u32,
    pub mask_invert_tone: u32,
    pub random_seed: u32,
    pub position_generator: u32, // Position generator type for agent initialization
}

impl SimSizeUniform {
    pub fn new(
        width: u32,
        height: u32,
        decay_rate: f32,
        settings: &Settings,
        state: &SlimeMoldState,
        position_generator: &crate::simulations::shared::SlimeMoldPositionGenerator,
    ) -> Self {
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
            mask_pattern: u32::from(state.mask_pattern),
            mask_target: u32::from(state.mask_target),
            mask_strength: state.mask_strength,
            mask_curve: state.mask_curve,
            mask_mirror_horizontal: if state.mask_mirror_horizontal { 1 } else { 0 },
            mask_mirror_vertical: if state.mask_mirror_vertical { 1 } else { 0 },
            mask_invert_tone: if state.mask_invert_tone { 1 } else { 0 },
            random_seed: settings.random_seed,
            position_generator: position_generator.as_u32(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CursorParams {
    pub is_active: u32, // 0=inactive, 1=attract, 2=repel
    pub x: f32,
    pub y: f32,
    pub strength: f32,

    pub size: f32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct BackgroundParams {
    pub background_type: u32, // 0 = black, 1 = white
    pub mask_enabled: u32,
    pub mask_pattern: u32,
    pub mask_strength: f32,

    pub mask_mirror_horizontal: u32,
    pub mask_mirror_vertical: u32,
    pub mask_invert_tone: u32,
    pub _pad0: u32,
}

#[derive(Debug)]
/// SlimeMoldModel manages simulation-specific GPU resources and logic
/// while using Tauri's shared GPU context (device, queue, surface config)
pub struct SlimeMoldModel {
    // Simulation-specific GPU resources
    pub bind_group_manager: BindGroupManager,
    pub pipeline_manager: PipelineManager,
    pub agent_buffer: wgpu::Buffer,
    pub trail_map_buffers: PingPongBuffers, // Ping-pong buffers for diffusion
    pub mask_buffer: wgpu::Buffer,
    pub sim_size_buffer: Arc<wgpu::Buffer>,
    pub lut_buffer: Arc<wgpu::Buffer>,
    pub display_texture: wgpu::Texture,
    pub display_view: TextureView,
    pub display_sampler: wgpu::Sampler,
    pub workgroup_config: WorkgroupConfig,
    pub buffer_pool: BufferPool,

    // Simulation state
    pub settings: Settings,
    pub state: SlimeMoldState,
    pub agent_count: usize,
    pub color_scheme_reversed: bool,
    pub current_color_scheme: String,
    pub position_generator: crate::simulations::shared::SlimeMoldPositionGenerator,
    pub trail_map_filtering: super::settings::TrailMapFiltering,

    // Buffer size tracking for pool management
    pub current_trail_map_size: u64,
    pub current_mask_buffer_size: u64,
    pub current_agent_buffer_size: u64,

    // Dimension tracking for resize scaling
    pub current_width: u32,
    pub current_height: u32,
    gui_visible: bool,

    // Camera for viewport control
    pub camera: Camera,

    // Resize debouncing
    pub last_resize_time: std::time::Instant,
    pub resize_debounce_threshold: std::time::Duration,

    // Add cursor interaction state to SlimeMoldModel
    pub cursor_active_mode: u32, // 0=inactive, 1=attract, 2=repel
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,
    pub cursor_buffer: wgpu::Buffer, // buffer for CursorParams

    // Cursor configuration (runtime state, not saved in presets)
    pub cursor_size: f32,
    pub cursor_strength: f32,

    // Background parameters
    pub background_params_buffer: wgpu::Buffer,
    pub background_bind_group: wgpu::BindGroup,
    pub background_color_buffer: wgpu::Buffer,
    pub average_color_buffer: wgpu::Buffer,
    pub average_color_staging_buffer: wgpu::Buffer,
    pub average_color_bind_group: wgpu::BindGroup,
    pub average_color_uniform_buffer: wgpu::Buffer,
    pub color_scheme_manager: Arc<ColorSchemeManager>,
    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
    pub app_settings: AppSettings,
    // Raw grayscale (0..1) image for image-based mask, sized to current sim dims
    pub mask_image_raw: Option<Vec<f32>>,
    // Original grayscale values (0..1) before strength is applied, for reprocessing
    pub mask_image_base: Option<Vec<f32>>,
    // Original image data for reprocessing with different fit modes
    pub mask_image_original: Option<image::DynamicImage>,
    // Flag to indicate mask image needs to be re-uploaded to GPU
    pub mask_image_needs_upload: bool,
    // Original image data for position generation
    pub position_image_original: Option<image::DynamicImage>,
    // Raw grayscale (0..1) image for position generation, sized to current sim dims
    pub position_image_raw: Option<Vec<f32>>,
    // Flag to indicate position image needs to be re-uploaded to GPU
    pub position_image_needs_upload: bool,
    // Webcam capture for real-time mask input
    pub webcam_capture: crate::simulations::shared::WebcamCapture,
}

impl SlimeMoldModel {
    /// Calculate the number of tiles needed for infinite rendering based on zoom level
    fn calculate_tile_count(&self) -> u32 {
        let zoom = self.camera.zoom;
        // At zoom 1.0, we need at least 5x5 tiles
        // As zoom decreases (zooming out), we need more tiles
        // Each tile covers 2.0 world units, so we need enough tiles to cover the visible area
        let visible_world_size = 2.0 / zoom; // World size visible on screen
        let tiles_needed = (visible_world_size / 2.0).ceil() as u32 + 6; // +6 for extra padding at extreme zoom levels
        let min_tiles = if zoom < 0.1 { 7 } else { 5 }; // More tiles needed at extreme zoom out
        // Allow more tiles for proper infinite tiling, but cap at reasonable limit
        tiles_needed.max(min_tiles).min(1024) // Cap at 200x200 for performance
    }

    /// Create a new slime mold simulation using Tauri's shared GPU resources
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
        agent_count: usize,
        settings: Settings,
        app_settings: &AppSettings,
        color_scheme_manager: &ColorSchemeManager,
    ) -> SimulationResult<Self> {
        let physical_width = surface_config.width;
        let physical_height = surface_config.height;

        // Check if the trail map buffer size would exceed GPU limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let trail_map_size = (physical_width * physical_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;

        // If buffer would be too large, scale down the resolution
        let (effective_width, effective_height) = if trail_map_size_bytes > max_storage_buffer_size
        {
            let scale_factor =
                (max_storage_buffer_size as f64 / trail_map_size_bytes as f64).sqrt();
            let new_width = (physical_width as f64 * scale_factor * 0.95) as u32; // 95% to be safe
            let new_height = (physical_height as f64 * scale_factor * 0.95) as u32;
            tracing::warn!(
                "Trail map buffer size {} bytes exceeds GPU limit {} bytes. Scaling down from {}x{} to {}x{}",
                trail_map_size_bytes,
                max_storage_buffer_size,
                physical_width,
                physical_height,
                new_width,
                new_height
            );
            (new_width, new_height)
        } else {
            (physical_width, physical_height)
        };

        // Create simulation-specific buffers
        let agent_buffer = create_agent_buffer(device, agent_count);

        let trail_map_size = (effective_width * effective_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;

        // Create ping-pong buffers for trail map diffusion
        let trail_map_buffers = PingPongBuffers::new(
            device,
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            "Trail Map",
        );

        // Initialize the current buffer with some random values
        {
            let mut data = vec![0.0f32; trail_map_size];
            for cell in data.iter_mut() {
                *cell = rand::random::<f32>() * 0.1; // Small initial values
            }
            queue.write_buffer(
                trail_map_buffers.current_buffer(),
                0,
                bytemuck::cast_slice(&data),
            );
        }

        // Initialize the inactive buffer with zeros
        {
            let data = vec![0.0f32; trail_map_size];
            queue.write_buffer(
                trail_map_buffers.inactive_buffer(),
                0,
                bytemuck::cast_slice(&data),
            );
        }

        let mask_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mask Buffer"),
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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create uniform buffer
        let default_state = SlimeMoldState::default();
        let sim_size_uniform = SimSizeUniform::new(
            effective_width,
            effective_height,
            settings.pheromone_decay_rate,
            &settings,
            &default_state,
            &crate::simulations::shared::SlimeMoldPositionGenerator::Random,
        );
        let sim_size_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Sim Size Uniform Buffer",
            &[sim_size_uniform],
        );
        let sim_size_buffer = Arc::new(sim_size_buffer);

        // Create color scheme buffer
        let lut_data = color_scheme_manager.get("MATPLOTLIB_cubehelix")?;
        let lut_data_u32 = lut_data.to_u32_buffer();

        let lut_buffer = resource_helpers::create_storage_buffer_with_data(
            device,
            "Color Scheme Buffer",
            &lut_data_u32,
        );
        let lut_buffer = Arc::new(lut_buffer);

        // Create display sampler
        let display_sampler = resource_helpers::create_linear_sampler(
            device,
            "Display Sampler",
            app_settings.texture_filtering.into(),
        );

        // Create workgroup config
        let workgroup_config = WorkgroupConfig::new(device, adapter_info);

        // Create pipeline manager
        let pipeline_manager =
            PipelineManager::new(device, &workgroup_config, surface_config.format);

        // Create camera
        let camera = Camera::new(device, effective_width as f32, effective_height as f32)?;

        // Create cursor buffer
        let cursor_params = CursorParams {
            is_active: 0,
            x: 0.0,
            y: 0.0,
            strength: 10.0, // reasonable default
            size: 80.0,     // reasonable default
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        let cursor_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cursor Params Buffer"),
            contents: bytemuck::bytes_of(&cursor_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create background parameters
        let background_params = BackgroundParams {
            background_type: u32::from(settings.background_mode),
            mask_enabled: if default_state.mask_pattern == MaskPattern::Disabled {
                0
            } else {
                1
            },
            mask_pattern: u32::from(default_state.mask_pattern),
            mask_strength: default_state.mask_strength,
            mask_mirror_horizontal: if default_state.mask_mirror_horizontal {
                1
            } else {
                0
            },
            mask_mirror_vertical: if default_state.mask_mirror_vertical {
                1
            } else {
                0
            },
            mask_invert_tone: if default_state.mask_invert_tone { 1 } else { 0 },
            _pad0: 0,
        };
        let background_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Background Params Buffer"),
                contents: bytemuck::bytes_of(&background_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create background color buffer (black by default)
        let background_color_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold Background Color Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // Black background
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create average color buffer for calculating frame average
        let average_color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slime Mold Average Color Buffer"),
            contents: bytemuck::cast_slice(&[0u32, 0u32, 0u32, 0u32]), // Initialize to zero (atomic f32 as u32)
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        // Create staging buffer for reading back average color data
        let average_color_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slime Mold Average Color Staging Buffer"),
            size: std::mem::size_of::<[u32; 4]>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create average color uniform buffer for infinite render shader
        let average_color_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Slime Mold Average Color Uniform Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // Initialize with black
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group manager
        let bind_group_manager = BindGroupManager::new(
            device,
            &pipeline_manager.compute_bind_group_layout,
            &pipeline_manager.display_bind_group_layout,
            &pipeline_manager.render_bind_group_layout,
            &pipeline_manager.camera_bind_group_layout,
            &pipeline_manager.gradient_bind_group_layout,
            &agent_buffer,
            trail_map_buffers.current_buffer(),
            trail_map_buffers.inactive_buffer(),
            &mask_buffer,
            &sim_size_buffer,
            &display_view,
            &display_sampler,
            &lut_buffer,
            camera.buffer(),
            &cursor_buffer,
            &background_color_buffer,
            &average_color_uniform_buffer,
        );

        // Create background bind group
        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Bind Group"),
            layout: &pipeline_manager.background_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, &background_params_buffer)],
        });

        // Create average color bind group
        let average_color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Average Color Bind Group"),
            layout: &pipeline_manager.average_color_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &display_view),
                resource_helpers::buffer_entry(1, &average_color_buffer),
            ],
        });

        // Create buffer pool
        let buffer_pool = BufferPool::new();

        let agent_buffer_size_bytes = (agent_count * 4 * std::mem::size_of::<f32>()) as u64;
        let post_processing_state = PostProcessingState::default();
        let post_processing_resources = PostProcessingResources::new(device, surface_config)?;
        let mut simulation = Self {
            bind_group_manager,
            pipeline_manager,
            agent_buffer,
            trail_map_buffers,
            mask_buffer,
            sim_size_buffer,
            lut_buffer,
            display_texture,
            display_view,
            display_sampler,
            workgroup_config,
            buffer_pool,
            settings,
            state: default_state,
            agent_count,
            current_color_scheme: "MATPLOTLIB_cubehelix".to_string(),
            color_scheme_reversed: true,
            current_trail_map_size: trail_map_size_bytes,
            current_mask_buffer_size: trail_map_size_bytes,
            current_agent_buffer_size: agent_buffer_size_bytes,
            current_width: effective_width,
            current_height: effective_height,
            gui_visible: true,
            camera,
            last_resize_time: std::time::Instant::now(),
            resize_debounce_threshold: std::time::Duration::from_millis(500),
            cursor_active_mode: 0,
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
            cursor_buffer,
            cursor_size: 300.0,   // Default cursor size
            cursor_strength: 5.0, // Default cursor strength
            position_generator: crate::simulations::shared::SlimeMoldPositionGenerator::Random,
            trail_map_filtering: super::settings::TrailMapFiltering::Nearest,
            background_params_buffer,
            background_bind_group,
            background_color_buffer,
            average_color_buffer,
            average_color_staging_buffer,
            average_color_bind_group,
            average_color_uniform_buffer,
            color_scheme_manager: Arc::new(color_scheme_manager.clone()),
            post_processing_state,
            post_processing_resources,
            app_settings: app_settings.clone(),
            mask_image_raw: None,
            mask_image_base: None,
            mask_image_original: None,
            mask_image_needs_upload: false,
            position_image_original: None,
            position_image_raw: None,
            position_image_needs_upload: false,
            webcam_capture: crate::simulations::shared::WebcamCapture::new(),
        };

        if let Ok(mut lut_data) = color_scheme_manager.get(&simulation.current_color_scheme) {
            if simulation.color_scheme_reversed {
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
    ) -> SimulationResult<()> {
        let physical_width = new_config.width;
        let physical_height = new_config.height;

        // Check if the trail map buffer size would exceed GPU limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let trail_map_size = (physical_width * physical_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;

        // If buffer would be too large, scale down the resolution
        let (effective_width, effective_height) = if trail_map_size_bytes > max_storage_buffer_size
        {
            let scale_factor =
                (max_storage_buffer_size as f64 / trail_map_size_bytes as f64).sqrt();
            let new_width = (physical_width as f64 * scale_factor * 0.95) as u32; // 95% to be safe
            let new_height = (physical_height as f64 * scale_factor * 0.95) as u32;
            tracing::warn!(
                "Trail map buffer size {} bytes exceeds GPU limit {} bytes. Scaling down from {}x{} to {}x{}",
                trail_map_size_bytes,
                max_storage_buffer_size,
                physical_width,
                physical_height,
                new_width,
                new_height
            );
            (new_width, new_height)
        } else {
            (physical_width, physical_height)
        };

        // Early return if dimensions haven't changed significantly
        let width_diff = effective_width.abs_diff(self.current_width);
        let height_diff = effective_height.abs_diff(self.current_height);
        let total_pixel_change =
            width_diff * self.current_height + height_diff * self.current_width;

        // If change is less than 1% of total pixels, skip resize
        let total_pixels = self.current_width * self.current_height;
        if total_pixel_change < total_pixels / 100 {
            return Ok(());
        }

        // Debounce rapid resize events
        let now = std::time::Instant::now();
        if now.duration_since(self.last_resize_time) < self.resize_debounce_threshold {
            // Update the last resize time but don't actually resize
            self.last_resize_time = now;
            return Ok(());
        }
        self.last_resize_time = now;

        tracing::info!(
            "Resizing slime mold from {}x{} to {}x{}",
            self.current_width,
            self.current_height,
            effective_width,
            effective_height
        );

        // Calculate new buffer sizes
        let trail_map_size = (effective_width * effective_height) as usize;
        let trail_map_size_bytes = (trail_map_size * std::mem::size_of::<f32>()) as u64;
        let agent_buffer_size_bytes = (self.agent_count * 4 * std::mem::size_of::<f32>()) as u64;

        // Validate buffer sizes to prevent overruns
        if trail_map_size_bytes > max_storage_buffer_size {
            return Err(format!(
                "New trail map buffer size {} bytes exceeds GPU limit {} bytes",
                trail_map_size_bytes, max_storage_buffer_size
            )
            .into());
        }

        // Store old buffers for scaling
        let old_trail_map_buffers = std::mem::replace(
            &mut self.trail_map_buffers,
            PingPongBuffers::new(
                device,
                1, // Temporary size
                wgpu::BufferUsages::STORAGE,
                "Temp Trail Map",
            ),
        );

        let old_mask_buffer = std::mem::replace(
            &mut self.mask_buffer,
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Temp Mask Buffer"),
                size: 1,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        );

        // Create new ping-pong buffers
        self.trail_map_buffers = PingPongBuffers::new(
            device,
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            "Trail Map",
        );

        self.mask_buffer = self.buffer_pool.get_buffer(
            device,
            Some("Mask Buffer"),
            trail_map_size_bytes,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );

        // Scale trail map data from old dimensions to new dimensions
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scale_trail_map_data(
                device,
                queue,
                old_trail_map_buffers.current_buffer(),
                self.trail_map_buffers.current_buffer(),
                self.current_width,
                self.current_height,
                effective_width,
                effective_height,
            );
        })) {
            tracing::error!("Failed to scale trail map data: {:?}", e);
            // If scaling fails, just reset the trail map
            reset_trails(
                self.trail_map_buffers.current_buffer(),
                queue,
                effective_width,
                effective_height,
            );
        }

        // Scale mask data from old dimensions to new dimensions
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scale_trail_map_data(
                device,
                queue,
                &old_mask_buffer,
                &self.mask_buffer,
                self.current_width,
                self.current_height,
                effective_width,
                effective_height,
            );
        })) {
            tracing::error!("Failed to scale mask data: {:?}", e);
            // If scaling fails, just reset the mask
            reset_trails(&self.mask_buffer, queue, effective_width, effective_height);
        }

        // Return old buffers to pool after scaling is complete
        // Note: PingPongBuffers contains two buffers, but we can't return them individually
        // The old buffers will be dropped automatically when old_trail_map_buffers goes out of scope

        self.buffer_pool.return_buffer(
            old_mask_buffer,
            self.current_mask_buffer_size,
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
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
        })) {
            tracing::error!("Failed to scale agent buffer: {:?}", e);
            // If scaling fails, create a new agent buffer and reset agents
            self.agent_buffer = create_agent_buffer(device, self.agent_count);
            // Reset agents to new positions
            if let Err(e) = self.reset_agents(device, queue) {
                tracing::error!("Failed to reset agents after resize: {}", e);
            }
        }

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
        self.current_mask_buffer_size = trail_map_size_bytes;
        self.current_agent_buffer_size = agent_buffer_size_bytes;
        self.current_width = effective_width;
        self.current_height = effective_height;

        // Update sim_size_buffer with new dimensions
        let sim_size_uniform = SimSizeUniform::new(
            effective_width,
            effective_height,
            self.settings.pheromone_decay_rate,
            &self.settings,
            &self.state,
            &self.position_generator,
        );
        queue.write_buffer(
            &self.sim_size_buffer,
            0,
            bytemuck::cast_slice(&[sim_size_uniform]),
        );

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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.display_view = self
            .display_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update bind groups with new buffers and texture view
        self.recreate_bind_groups(device);

        // Resize camera
        self.camera
            .resize(effective_width as f32, effective_height as f32);

        self.post_processing_resources.resize(device, new_config)?;

        tracing::info!("Slime mold resize completed successfully");
        Ok(())
    }

    /// Render a single frame of the simulation
    pub fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        // Update camera for smooth movement
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        // Update background parameters
        self.update_background_params(queue);
        self.update_background_color(queue);

        // Upload mask image if it needs to be re-uploaded
        if self.mask_image_needs_upload {
            if let Some(buffer) = &self.mask_image_raw {
                queue.write_buffer(
                    &self.mask_buffer,
                    0,
                    bytemuck::cast_slice::<f32, u8>(buffer),
                );
                self.mask_image_needs_upload = false;
            }
        }

        // Update mask from webcam if active
        if self.webcam_capture.is_active {
            // Update webcam frame first
            if let Err(e) = self.webcam_capture.update_frame() {
                tracing::warn!("Failed to update webcam frame: {}", e);
            }

            // Then update mask buffer
            if let Err(e) = self.update_mask_from_webcam(queue) {
                tracing::warn!("Failed to update mask from webcam: {}", e);
            }
        }

        // Upload position image if it needs to be re-uploaded (single shared image buffer).
        if self.position_image_needs_upload {
            if let Some(buffer) = &self.position_image_raw {
                queue.write_buffer(
                    &self.mask_buffer,
                    0,
                    bytemuck::cast_slice::<f32, u8>(buffer),
                );
                self.position_image_needs_upload = false;
            }
        }

        // Run compute passes for simulation (agent updates, trail decay, etc.)
        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Compute Encoder"),
        });
        self.run_compute_passes(&mut compute_encoder);
        queue.submit(std::iter::once(compute_encoder.finish()));

        // 1. Render background to offscreen texture
        let mut background_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Slime Mold Background Encoder"),
            });
        {
            let mut render_pass =
                background_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Slime Mold Background Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            // Render background
            render_pass.set_pipeline(&self.pipeline_manager.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group_manager.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(std::iter::once(background_encoder.finish()));

        // 2. Generate main simulation content to offscreen texture
        let mut display_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Display Encoder"),
        });
        {
            let mut compute_pass =
                display_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        queue.submit(std::iter::once(display_encoder.finish()));

        // 2. Render offscreen texture to surface with infinite tiling
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Infinite Surface Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slime Mold Infinite Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline_manager.render_infinite_pipeline);
            render_pass.set_bind_group(0, &self.bind_group_manager.render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group_manager.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }
        queue.submit(std::iter::once(encoder.finish()));

        if self.post_processing_state.blur_filter.enabled {
            self.apply_post_processing(
                device,
                queue,
                &self.display_view,
                &self.post_processing_resources.intermediate_view,
            )?;
            // Copy result back to display_view if needed
        }

        Ok(())
    }

    /// Run the compute passes for the simulation
    fn run_compute_passes(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // Mask pass (if enabled)
        if self.state.mask_pattern != MaskPattern::Disabled
            && self.state.mask_pattern != MaskPattern::Image
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Mask Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.gradient_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.gradient_bind_group, &[]);

            let total_pixels = self.display_texture.width() * self.display_texture.height();
            let workgroup_size = self.workgroup_config.compute_1d;
            let workgroups = total_pixels.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Agent update pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Agent Update Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.compute_bind_group, &[]);

            // For large agent counts, use 2D dispatch to avoid 65535 workgroup limit
            let workgroup_size =
                self.workgroup_config.compute_2d.0 * self.workgroup_config.compute_2d.1;
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

        // Diffusion pass with ping-pong buffering
        {
            // Swap buffers before diffusion (current becomes source, inactive becomes destination)
            self.trail_map_buffers.swap();

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Diffusion Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.diffuse_pipeline);

            // Use the appropriate bind group based on current buffer state
            let bind_group = if self.trail_map_buffers.current_index() == 0 {
                &self.bind_group_manager.compute_bind_group
            } else {
                &self.bind_group_manager.compute_bind_group_b
            };
            compute_pass.set_bind_group(0, bind_group, &[]);

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
            &self.state,
            &self.sim_size_buffer,
            queue,
            self.display_texture.width(),
            self.display_texture.height(),
            &self.position_generator,
        );
    }

    /// Update the LUT (color lookup table) - deprecated, use update_color_scheme
    pub fn update_lut(&mut self, lut_data: &ColorScheme, queue: &Queue) {
        let lut_data_u32 = lut_data.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
    }

    /// Reset trail map to zero
    pub fn reset_trails(&self, queue: &Arc<Queue>) {
        reset_trails(
            self.trail_map_buffers.current_buffer(),
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
    ) -> SimulationResult<()> {
        // Update the random seed to ensure different randomization
        self.settings.random_seed = rand::random::<u32>();

        // Update the sim size buffer with the new random seed
        let sim_size = SimSizeUniform::new(
            self.current_width,
            self.current_height,
            self.settings.pheromone_decay_rate,
            &self.settings,
            &self.state,
            &self.position_generator,
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

    /// Update agent speeds to new random values within the current min/max range
    pub fn update_agent_speeds(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) {
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
            let workgroup_size =
                self.workgroup_config.compute_2d.0 * self.workgroup_config.compute_2d.1;
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
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
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
            "mask_pattern" => {
                if let Some(v) = value.as_str() {
                    // Accept display-case or snake/lowercase
                    self.state.mask_pattern =
                        MaskPattern::from_str(v).expect("Invalid mask pattern");

                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );

                    // Force mask regeneration immediately
                    self.regenerate_mask(device, queue);
                }
            }
            "mask_target" => {
                if let Some(v) = value.as_str() {
                    // Accept display-case or snake/lowercase
                    self.state.mask_target = MaskTarget::from_str(v).expect("Invalid mask target");

                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_strength" => {
                if let Some(v) = value.as_f64() {
                    self.state.mask_strength = v as f32;

                    // Update uniform buffer to reflect the new mask strength
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_curve" => {
                if let Some(v) = value.as_f64() {
                    self.state.mask_curve = v as f32;
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_reversed" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_reversed = v;

                    // Update uniform buffer to reflect the new mask reversed setting
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_mirror_horizontal" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_mirror_horizontal = v;

                    // Update uniform buffer to reflect the new mask mirror horizontal setting
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );

                    // Force mask regeneration immediately
                    self.regenerate_mask(device, queue);
                }
            }
            "mask_mirror_vertical" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_mirror_vertical = v;

                    // Update uniform buffer to reflect the new mask mirror vertical setting
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );

                    // Force mask regeneration immediately
                    self.regenerate_mask(device, queue);
                }
            }
            "mask_invert_tone" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_invert_tone = v;
                    self.update_background_params(queue);

                    // Update uniform buffer to reflect the new mask invert tone setting
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );

                    // Force mask regeneration immediately
                    self.regenerate_mask(device, queue);
                }
            }
            "mask_image_fit_mode" => {
                if let Some(v) = value.as_str() {
                    self.state.mask_image_fit_mode =
                        v.parse::<ImageFitMode>().expect("Invalid image fit mode");

                    // If we have a loaded image, reprocess it with the new fit mode
                    if self.state.mask_pattern == MaskPattern::Image
                        && self.mask_image_raw.is_some()
                    {
                        // Reprocess the stored raw image data with new fit mode
                        self.reprocess_mask_image_with_current_fit_mode();
                    }
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.cursor_size = (size as f32).clamp(10.0, 500.0); // Clamp to reasonable range
                    self.update_cursor_params(queue);
                    return Ok(()); // Return early to avoid updating GPU uniforms unnecessarily
                }
            }
            "cursor_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.cursor_strength = (strength as f32).clamp(0.0, 50.0); // Clamp to reasonable range
                    self.update_cursor_params(queue);
                    return Ok(()); // Return early to avoid updating GPU uniforms unnecessarily
                }
            }
            "position_image_fit_mode" => {
                if let Some(v) = value.as_str() {
                    self.settings.position_image_fit_mode = match v {
                        "Stretch" | "stretch" => ImageFitMode::Stretch,
                        "Center" | "center" => ImageFitMode::Center,
                        "Fit H" | "fit h" => ImageFitMode::FitH,
                        "Fit V" | "fit v" => ImageFitMode::FitV,
                        _ => unreachable!(),
                    };

                    // If we have a loaded position image, reprocess it with the new fit mode
                    if self.position_generator
                        == crate::simulations::shared::SlimeMoldPositionGenerator::Image
                        && self.position_image_original.is_some()
                    {
                        // Reprocess the stored position image with new fit mode
                        self.reprocess_position_image_with_current_fit_mode();
                    }
                }
            }
            "random_seed" => {
                if let Some(v) = value.as_u64() {
                    self.settings.random_seed = v as u32;
                }
            }
            "position_generator" => {
                if let Some(generator_str) = value.as_str() {
                    if let Some(selected_generator) =
                        crate::simulations::shared::SlimeMoldPositionGenerator::from_str(
                            generator_str,
                        )
                    {
                        self.position_generator = selected_generator;
                    } else {
                        self.position_generator =
                            crate::simulations::shared::SlimeMoldPositionGenerator::Random;
                    }
                }
            }
            "trailMapFiltering" => {
                if let Some(filtering_str) = value.as_str() {
                    if let Some(f) = super::settings::TrailMapFiltering::from_str(filtering_str) {
                        self.trail_map_filtering = f;
                    } else {
                        self.trail_map_filtering = super::settings::TrailMapFiltering::Nearest;
                    }
                    self.update_display_sampler(device);
                }
            }
            _ => {
                return Err(format!("Unknown setting: {}", setting_name).into());
            }
        }

        // Update the GPU uniforms with the new settings
        update_settings(
            &self.settings,
            &self.state,
            &self.sim_size_buffer,
            queue,
            self.display_texture.width(),
            self.display_texture.height(),
            &self.position_generator,
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
    ) -> SimulationResult<()> {
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
            &self.pipeline_manager.gradient_bind_group_layout,
            &self.agent_buffer,
            self.trail_map_buffers.current_buffer(),
            self.trail_map_buffers.inactive_buffer(),
            &self.mask_buffer,
            &self.sim_size_buffer,
            &self.display_view,
            &self.display_sampler,
            &self.lut_buffer,
            self.camera.buffer(),
            &self.cursor_buffer,
            &self.background_color_buffer,
            &self.average_color_uniform_buffer,
        );
    }

    pub(crate) fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.gui_visible
    }

    pub(crate) fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    pub fn get_agent_count(&self) -> Option<u32> {
        Some(self.agent_count as u32)
    }

    // Camera control methods
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    pub fn reset_camera(&mut self) {
        self.camera.reset();
    }

    /// Update the cursor state and upload to GPU (to be used in compute shader)
    pub fn update_cursor_params(&mut self, queue: &Arc<Queue>) {
        let params = CursorParams {
            is_active: self.cursor_active_mode,
            x: self.cursor_world_x,
            y: self.cursor_world_y,
            strength: self.cursor_strength,
            size: self.cursor_size,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        queue.write_buffer(&self.cursor_buffer, 0, bytemuck::bytes_of(&params));
    }

    /// Force regeneration of the mask pattern
    pub fn regenerate_mask(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) {
        // Only regenerate if mask is enabled and not image-based
        if self.state.mask_pattern != MaskPattern::Disabled
            && self.state.mask_pattern != MaskPattern::Image
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Force Mask Regeneration Encoder"),
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Force Mask Regeneration Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&self.pipeline_manager.gradient_pipeline);
                compute_pass.set_bind_group(0, &self.bind_group_manager.gradient_bind_group, &[]);

                let total_pixels = self.display_texture.width() * self.display_texture.height();
                let workgroup_size = self.workgroup_config.compute_1d;
                let workgroups = total_pixels.div_ceil(workgroup_size);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            } // compute_pass is dropped here

            queue.submit(std::iter::once(encoder.finish()));
        }
    }

    /// Load an external image, convert to grayscale, fit to current sim size, and upload to mask buffer
    pub fn load_mask_image_from_path(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
        image_path: &str,
    ) -> SimulationResult<()> {
        let img = image::open(image_path).map_err(|e| {
            SimulationError::InvalidParameter(format!("Failed to open image: {}", e))
        })?;

        // Store original image for reprocessing
        self.mask_image_original = Some(img.clone());

        let (target_w, target_h) = (self.current_width as u32, self.current_height as u32);

        // Convert to Luma8 (grayscale)
        let gray = img.to_luma8();

        let fit_mode = self.state.mask_image_fit_mode;

        // Prepare buffer of size target_w * target_h
        let mut buffer = vec![0.0f32; (target_w * target_h) as usize];

        match fit_mode {
            ImageFitMode::Stretch => {
                // Resize exactly to target size
                let resized = image::imageops::resize(
                    &gray,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                );
                for y in 0..target_h {
                    for x in 0..target_w {
                        let p = resized.get_pixel(x, y)[0] as f32 / 255.0;
                        buffer[(y * target_w + x) as usize] = p;
                    }
                }
            }
            ImageFitMode::Center => {
                // Paste without scaling, centered, cropping if larger, padding if smaller
                let src_w = gray.width();
                let src_h = gray.height();
                let offset_x = if target_w > src_w {
                    ((target_w - src_w) / 2) as i64
                } else {
                    -(((src_w - target_w) / 2) as i64)
                };
                let offset_y = if target_h > src_h {
                    ((target_h - src_h) / 2) as i64
                } else {
                    -(((src_h - target_h) / 2) as i64)
                };

                for ty in 0..target_h as i64 {
                    for tx in 0..target_w as i64 {
                        let sx = tx - offset_x;
                        let sy = ty - offset_y;
                        let v = if sx >= 0 && sy >= 0 && (sx as u32) < src_w && (sy as u32) < src_h
                        {
                            gray.get_pixel(sx as u32, sy as u32)[0] as f32 / 255.0
                        } else {
                            0.0
                        };
                        buffer[(ty as u32 * target_w + tx as u32) as usize] = v;
                    }
                }
            }
            ImageFitMode::FitH => {
                // Fit horizontally, maintain aspect ratio, center vertically
                let src_w = gray.width() as f32;
                let src_h = gray.height() as f32;
                let target_w_f = target_w as f32;

                // Calculate scale to fit width
                let scale = target_w_f / src_w;
                let scaled_h = (src_h * scale) as u32;

                // Resize to fit width
                let resized = image::imageops::resize(
                    &gray,
                    target_w,
                    scaled_h,
                    image::imageops::FilterType::Lanczos3,
                );

                // Center vertically
                let offset_y = if scaled_h < target_h {
                    ((target_h - scaled_h) / 2) as i64
                } else {
                    0
                };

                for y in 0..target_h {
                    for x in 0..target_w {
                        let src_y = (y as i64 - offset_y) as u32;
                        let v = if src_y < scaled_h {
                            resized.get_pixel(x, src_y)[0] as f32 / 255.0
                        } else {
                            0.0
                        };
                        buffer[(y * target_w + x) as usize] = v;
                    }
                }
            }
            ImageFitMode::FitV => {
                // Fit vertically, maintain aspect ratio, center horizontally
                let src_w = gray.width() as f32;
                let src_h = gray.height() as f32;
                let target_h_f = target_h as f32;

                // Calculate scale to fit height
                let scale = target_h_f / src_h;
                let scaled_w = (src_w * scale) as u32;

                // Resize to fit height
                let resized = image::imageops::resize(
                    &gray,
                    scaled_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                );

                // Center horizontally
                let offset_x = if scaled_w < target_w {
                    ((target_w - scaled_w) / 2) as i64
                } else {
                    0
                };

                for y in 0..target_h {
                    for x in 0..target_w {
                        let src_x = (x as i64 - offset_x) as u32;
                        let v = if src_x < scaled_w {
                            resized.get_pixel(src_x, y)[0] as f32 / 255.0
                        } else {
                            0.0
                        };
                        buffer[(y * target_w + x) as usize] = v;
                    }
                }
            }
        }

        // Store base grayscale values (no CPU-side strength/curve)
        self.mask_image_base = Some(buffer.clone());

        // Store raw grayscale; shader applies strength/curve/invert/mirror
        self.mask_image_raw = Some(buffer.clone());

        // Upload to GPU buffer
        queue.write_buffer(
            &self.mask_buffer,
            0,
            bytemuck::cast_slice::<f32, u8>(&buffer),
        );

        // Mark that image is uploaded and doesn't need re-upload
        self.mask_image_needs_upload = false;

        Ok(())
    }

    /// Load an external image for position generation, convert to grayscale, fit to current sim size
    pub fn load_position_image_from_path(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
        image_path: &str,
    ) -> SimulationResult<()> {
        let img = image::open(image_path).map_err(|e| {
            SimulationError::InvalidParameter(format!("Failed to open image: {}", e))
        })?;

        // Store original image for reprocessing
        self.position_image_original = Some(img.clone());

        let (target_w, target_h) = (self.current_width as u32, self.current_height as u32);

        // Convert to Luma8 (grayscale)
        let gray = img.to_luma8();

        let fit_mode = self.settings.position_image_fit_mode;

        // Prepare buffer of size target_w * target_h
        let mut buffer = vec![0.0f32; (target_w * target_h) as usize];

        match fit_mode {
            ImageFitMode::Stretch => {
                // Resize exactly to target size
                let resized = image::imageops::resize(
                    &gray,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                );
                for y in 0..target_h {
                    for x in 0..target_w {
                        let p = resized.get_pixel(x, y)[0] as f32 / 255.0;
                        buffer[(y * target_w + x) as usize] = p;
                    }
                }
            }
            ImageFitMode::Center => {
                // Paste without scaling, centered, cropping if larger, padding if smaller
                let src_w = gray.width();
                let src_h = gray.height();
                let offset_x = if target_w > src_w {
                    ((target_w - src_w) / 2) as i64
                } else {
                    -(((src_w - target_w) / 2) as i64)
                };
                let offset_y = if target_h > src_h {
                    ((target_h - src_h) / 2) as i64
                } else {
                    -(((src_h - target_h) / 2) as i64)
                };

                for ty in 0..target_h as i64 {
                    for tx in 0..target_w as i64 {
                        let sx = tx - offset_x;
                        let sy = ty - offset_y;
                        let v = if sx >= 0 && sy >= 0 && (sx as u32) < src_w && (sy as u32) < src_h
                        {
                            gray.get_pixel(sx as u32, sy as u32)[0] as f32 / 255.0
                        } else {
                            0.0
                        };
                        buffer[(ty as u32 * target_w + tx as u32) as usize] = v;
                    }
                }
            }
            ImageFitMode::FitH => {
                // Fit horizontally, maintain aspect ratio, center vertically
                let src_w = gray.width() as f32;
                let src_h = gray.height() as f32;
                let target_w_f = target_w as f32;

                // Calculate scale to fit width
                let scale = target_w_f / src_w;
                let scaled_h = (src_h * scale) as u32;

                // Resize to fit width
                let resized = image::imageops::resize(
                    &gray,
                    target_w,
                    scaled_h,
                    image::imageops::FilterType::Lanczos3,
                );

                // Center vertically
                let offset_y = if scaled_h < target_h {
                    ((target_h - scaled_h) / 2) as i64
                } else {
                    0
                };

                for y in 0..target_h {
                    for x in 0..target_w {
                        let src_y = (y as i64 - offset_y) as u32;
                        let v = if src_y < scaled_h {
                            resized.get_pixel(x, src_y)[0] as f32 / 255.0
                        } else {
                            0.0
                        };
                        buffer[(y * target_w + x) as usize] = v;
                    }
                }
            }
            ImageFitMode::FitV => {
                // Fit vertically, maintain aspect ratio, center horizontally
                let src_w = gray.width() as f32;
                let src_h = gray.height() as f32;
                let target_h_f = target_h as f32;

                // Calculate scale to fit height
                let scale = target_h_f / src_h;
                let scaled_w = (src_w * scale) as u32;

                // Resize to fit height
                let resized = image::imageops::resize(
                    &gray,
                    scaled_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                );

                // Center horizontally
                let offset_x = if scaled_w < target_w {
                    ((target_w - scaled_w) / 2) as i64
                } else {
                    0
                };

                for y in 0..target_h {
                    for x in 0..target_w {
                        let src_x = (x as i64 - offset_x) as u32;
                        let v = if src_x < scaled_w {
                            resized.get_pixel(src_x, y)[0] as f32 / 255.0
                        } else {
                            0.0
                        };
                        buffer[(y * target_w + x) as usize] = v;
                    }
                }
            }
        }

        // Store the processed image data
        self.position_image_raw = Some(buffer.clone());

        // Upload to GPU buffer (reuse mask buffer for now, since position generation uses mask_map)
        queue.write_buffer(
            &self.mask_buffer,
            0,
            bytemuck::cast_slice::<f32, u8>(&buffer),
        );

        // Mark that image is uploaded and doesn't need re-upload
        self.position_image_needs_upload = false;

        Ok(())
    }

    /// Reprocess the stored original image with the current fit mode
    pub fn reprocess_mask_image_with_current_fit_mode(&mut self) {
        if let Some(original_img) = &self.mask_image_original {
            let (target_w, target_h) = (self.current_width as u32, self.current_height as u32);

            // Convert to Luma8 (grayscale)
            let gray = original_img.to_luma8();

            let fit_mode = self.state.mask_image_fit_mode;

            // Prepare buffer of size target_w * target_h
            let mut buffer = vec![0.0f32; (target_w * target_h) as usize];

            // Apply the same processing logic as in load_gradient_image_from_path
            match fit_mode {
                ImageFitMode::Stretch => {
                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );
                    for y in 0..target_h {
                        for x in 0..target_w {
                            let p = resized.get_pixel(x, y)[0] as f32 / 255.0;
                            buffer[(y * target_w + x) as usize] = p;
                        }
                    }
                }
                ImageFitMode::Center => {
                    let src_w = gray.width();
                    let src_h = gray.height();
                    let offset_x = if target_w > src_w {
                        ((target_w - src_w) / 2) as i64
                    } else {
                        -(((src_w - target_w) / 2) as i64)
                    };
                    let offset_y = if target_h > src_h {
                        ((target_h - src_h) / 2) as i64
                    } else {
                        -(((src_h - target_h) / 2) as i64)
                    };

                    for ty in 0..target_h as i64 {
                        for tx in 0..target_w as i64 {
                            let sx = tx - offset_x;
                            let sy = ty - offset_y;
                            let v =
                                if sx >= 0 && sy >= 0 && (sx as u32) < src_w && (sy as u32) < src_h
                                {
                                    gray.get_pixel(sx as u32, sy as u32)[0] as f32 / 255.0
                                } else {
                                    0.0
                                };
                            buffer[(ty as u32 * target_w + tx as u32) as usize] = v;
                        }
                    }
                }
                ImageFitMode::FitH => {
                    let src_w = gray.width() as f32;
                    let src_h = gray.height() as f32;
                    let target_w_f = target_w as f32;

                    let scale = target_w_f / src_w;
                    let scaled_h = (src_h * scale) as u32;

                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        scaled_h,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let offset_y = if scaled_h < target_h {
                        ((target_h - scaled_h) / 2) as i64
                    } else {
                        0
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            let src_y = (y as i64 - offset_y) as u32;
                            let v = if src_y < scaled_h {
                                resized.get_pixel(x, src_y)[0] as f32 / 255.0
                            } else {
                                0.0
                            };
                            buffer[(y * target_w + x) as usize] = v;
                        }
                    }
                }
                ImageFitMode::FitV => {
                    let src_w = gray.width() as f32;
                    let src_h = gray.height() as f32;
                    let target_h_f = target_h as f32;

                    let scale = target_h_f / src_h;
                    let scaled_w = (src_w * scale) as u32;

                    let resized = image::imageops::resize(
                        &gray,
                        scaled_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let offset_x = if scaled_w < target_w {
                        ((target_w - scaled_w) / 2) as i64
                    } else {
                        0
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            let src_x = (x as i64 - offset_x) as u32;
                            let v = if src_x < scaled_w {
                                resized.get_pixel(src_x, y)[0] as f32 / 255.0
                            } else {
                                0.0
                            };
                            buffer[(y * target_w + x) as usize] = v;
                        }
                    }
                }
            }

            // Mirror, mask reversal, and tone inversion are all handled in shaders

            // Store base grayscale values (no CPU-side strength)
            self.mask_image_base = Some(buffer.clone());

            // Store raw grayscale; shader applies strength/curve/invert/mirror
            self.mask_image_raw = Some(buffer.clone());

            // Mark that the image needs to be re-uploaded to GPU
            self.mask_image_needs_upload = true;
        }
    }

    /// Reprocess the stored position image with the current fit mode
    pub fn reprocess_position_image_with_current_fit_mode(&mut self) {
        if let Some(original_img) = &self.position_image_original {
            let (target_w, target_h) = (self.current_width as u32, self.current_height as u32);

            // Convert to Luma8 (grayscale)
            let gray = original_img.to_luma8();
            let fit_mode = self.settings.position_image_fit_mode;

            // Prepare buffer of size target_w * target_h
            let mut buffer = vec![0.0f32; (target_w * target_h) as usize];

            // Apply the same processing logic as in load_position_image_from_path
            match fit_mode {
                ImageFitMode::Stretch => {
                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );
                    for y in 0..target_h {
                        for x in 0..target_w {
                            let p = resized.get_pixel(x, y)[0] as f32 / 255.0;
                            buffer[(y * target_w + x) as usize] = p;
                        }
                    }
                }
                ImageFitMode::Center => {
                    let src_w = gray.width();
                    let src_h = gray.height();
                    let offset_x = if target_w > src_w {
                        ((target_w - src_w) / 2) as i64
                    } else {
                        -(((src_w - target_w) / 2) as i64)
                    };
                    let offset_y = if target_h > src_h {
                        ((target_h - src_h) / 2) as i64
                    } else {
                        -(((src_h - target_h) / 2) as i64)
                    };

                    for ty in 0..target_h as i64 {
                        for tx in 0..target_w as i64 {
                            let sx = tx - offset_x;
                            let sy = ty - offset_y;
                            let v =
                                if sx >= 0 && sy >= 0 && (sx as u32) < src_w && (sy as u32) < src_h
                                {
                                    gray.get_pixel(sx as u32, sy as u32)[0] as f32 / 255.0
                                } else {
                                    0.0
                                };
                            buffer[(ty as u32 * target_w + tx as u32) as usize] = v;
                        }
                    }
                }
                ImageFitMode::FitH => {
                    let src_w = gray.width() as f32;
                    let src_h = gray.height() as f32;
                    let target_w_f = target_w as f32;

                    let scale = target_w_f / src_w;
                    let scaled_h = (src_h * scale) as u32;

                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        scaled_h,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let offset_y = if scaled_h < target_h {
                        ((target_h - scaled_h) / 2) as i64
                    } else {
                        0
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            let src_y = (y as i64 - offset_y) as u32;
                            let v = if src_y < scaled_h {
                                resized.get_pixel(x, src_y)[0] as f32 / 255.0
                            } else {
                                0.0
                            };
                            buffer[(y * target_w + x) as usize] = v;
                        }
                    }
                }
                ImageFitMode::FitV => {
                    let src_w = gray.width() as f32;
                    let src_h = gray.height() as f32;
                    let target_h_f = target_h as f32;

                    let scale = target_h_f / src_h;
                    let scaled_w = (src_w * scale) as u32;

                    let resized = image::imageops::resize(
                        &gray,
                        scaled_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let offset_x = if scaled_w < target_w {
                        ((target_w - scaled_w) / 2) as i64
                    } else {
                        0
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            let src_x = (x as i64 - offset_x) as u32;
                            let v = if src_x < scaled_w {
                                resized.get_pixel(src_x, y)[0] as f32 / 255.0
                            } else {
                                0.0
                            };
                            buffer[(y * target_w + x) as usize] = v;
                        }
                    }
                }
            }

            // Store the reprocessed image data
            self.position_image_raw = Some(buffer);

            // Mark that the image needs to be re-uploaded to GPU
            self.position_image_needs_upload = true;
        }
    }

    /// Update the background parameters and upload to GPU
    pub fn update_background_params(&mut self, queue: &Arc<Queue>) {
        let background_params = BackgroundParams {
            background_type: u32::from(self.settings.background_mode),
            mask_enabled: if self.state.mask_pattern == MaskPattern::Disabled {
                0
            } else {
                1
            },
            mask_pattern: u32::from(self.state.mask_pattern),
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: if self.state.mask_mirror_horizontal {
                1
            } else {
                0
            },
            mask_mirror_vertical: if self.state.mask_mirror_vertical {
                1
            } else {
                0
            },
            mask_invert_tone: if self.state.mask_invert_tone { 1 } else { 0 },
            _pad0: 0,
        };
        queue.write_buffer(
            &self.background_params_buffer,
            0,
            bytemuck::bytes_of(&background_params),
        );
    }

    fn update_background_color(&self, queue: &Arc<Queue>) {
        // This method is now only used for initial setup
        // The actual average color is calculated in calculate_average_color
        let lut = self
            .color_scheme_manager
            .get(&self.current_color_scheme)
            .unwrap_or_else(|_| self.color_scheme_manager.get_default());

        // Use a color from the middle of the LUT as initial background
        let colors = lut.get_colors(128);
        if let Some(color) = colors.first() {
            let background_color = [color[0], color[1], color[2], color[3]];
            queue.write_buffer(
                &self.background_color_buffer,
                0,
                bytemuck::cast_slice(&[background_color]),
            );
        }
    }

    fn apply_post_processing(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        input_texture_view: &wgpu::TextureView,
        output_texture_view: &wgpu::TextureView,
    ) -> crate::error::SimulationResult<()> {
        if self.post_processing_state.blur_filter.enabled {
            self.post_processing_resources.update_blur_params(
                queue,
                self.post_processing_state.blur_filter.radius,
                self.post_processing_state.blur_filter.sigma,
                self.current_width,
                self.current_height,
            );
            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Post Processing Blur Bind Group"),
                layout: &self
                    .post_processing_resources
                    .blur_pipeline
                    .get_bind_group_layout(0),
                entries: &[
                    resource_helpers::texture_view_entry(0, input_texture_view),
                    resource_helpers::sampler_bind_entry(
                        1,
                        &self.post_processing_resources.blur_sampler,
                    ),
                    resource_helpers::buffer_entry(
                        2,
                        &self.post_processing_resources.blur_params_buffer,
                    ),
                ],
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Post Processing Blur Encoder"),
            });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Post Processing Blur Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: output_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                render_pass.set_pipeline(&self.post_processing_resources.blur_pipeline);
                render_pass.set_bind_group(0, &blur_bind_group, &[]);
                render_pass.draw(0..6, 0..1);
            }
            queue.submit(std::iter::once(encoder.finish()));
        }
        Ok(())
    }
}

impl Drop for SlimeMoldModel {
    fn drop(&mut self) {
        // Clean up buffer pool
        self.buffer_pool.clear();
    }
}

impl crate::simulations::traits::Simulation for SlimeMoldModel {
    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Update camera for smooth movement
        self.camera.update(0.016); // Assume 60 FPS for now
        self.camera.upload_to_gpu(queue);

        // Generate display texture first
        let mut display_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Static Display Encoder"),
        });
        {
            let mut compute_pass =
                display_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Slime Mold Static Display Pass"),
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&self.pipeline_manager.display_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.display_bind_group, &[]);
            let (workgroups_x, workgroups_y) = self
                .workgroup_config
                .workgroups_2d(self.display_texture.width(), self.display_texture.height());
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        queue.submit(std::iter::once(display_encoder.finish()));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Slime Mold Static Render Encoder"),
        });

        // Skip compute passes for simulation - just render current state

        // First render to display texture
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Slime Mold Static Display Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline_manager.display_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group_manager.display_bind_group, &[]);
            let (workgroups_x, workgroups_y) = self
                .workgroup_config
                .workgroups_2d(self.display_texture.width(), self.display_texture.height());
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Then render display texture to surface with 3x3 instanced rendering
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slime Mold Static Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Use infinite instanced rendering with dynamic tile count
            let tile_count = self.calculate_tile_count();
            let total_instances = tile_count * tile_count;
            render_pass.set_pipeline(&self.pipeline_manager.render_infinite_pipeline);
            render_pass.set_bind_group(0, &self.bind_group_manager.render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.bind_group_manager.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances); // Dynamic grid based on zoom
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        self.render_frame(device, queue, surface_view, delta_time)
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.resize(device, queue, new_config)
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.update_setting(setting_name, value, device, queue)
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        match state_name {
            "mask_pattern" => {
                if let Some(v) = value.as_str() {
                    if let Some(pattern) = MaskPattern::from_str(v) {
                        self.state.mask_pattern = pattern;
                        update_settings(
                            &self.settings,
                            &self.state,
                            &self.sim_size_buffer,
                            queue,
                            self.display_texture.width(),
                            self.display_texture.height(),
                            &self.position_generator,
                        );
                        // Regenerate non-image mask immediately
                        self.regenerate_mask(_device, queue);
                    } else {
                        tracing::warn!("Invalid mask pattern: {}", v);
                    }
                }
            }
            "mask_mirror_horizontal" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_mirror_horizontal = v;
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_mirror_vertical" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_mirror_vertical = v;
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_invert_tone" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_invert_tone = v;
                    // Keep background params in sync (even if not used by all pipelines)
                    self.update_background_params(queue);
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_target" => {
                if let Some(v) = value.as_str() {
                    if let Some(target) = MaskTarget::from_str(v) {
                        self.state.mask_target = target;
                        update_settings(
                            &self.settings,
                            &self.state,
                            &self.sim_size_buffer,
                            queue,
                            self.display_texture.width(),
                            self.display_texture.height(),
                            &self.position_generator,
                        );
                    } else {
                        tracing::warn!("Invalid mask target: {}", v);
                    }
                }
            }
            "mask_strength" => {
                if let Some(v) = value.as_f64() {
                    self.state.mask_strength = v as f32;
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "mask_curve" => {
                if let Some(v) = value.as_f64() {
                    self.state.mask_curve = v as f32;
                    update_settings(
                        &self.settings,
                        &self.state,
                        &self.sim_size_buffer,
                        queue,
                        self.display_texture.width(),
                        self.display_texture.height(),
                        &self.position_generator,
                    );
                }
            }
            "current_color_scheme" => {
                if let Some(lut_name) = value.as_str() {
                    self.current_color_scheme = lut_name.to_string();
                    let lut_data = self
                        .color_scheme_manager
                        .get(&self.current_color_scheme)
                        .unwrap_or_else(|_| self.color_scheme_manager.get_default());

                    // Apply reversal if needed
                    let mut data_u32 = lut_data.to_u32_buffer();
                    if self.color_scheme_reversed {
                        data_u32[0..256].reverse();
                        data_u32[256..512].reverse();
                        data_u32[512..768].reverse();
                    }

                    queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&data_u32));
                }
            }
            "color_scheme_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    self.color_scheme_reversed = reversed;
                    let lut_data = self
                        .color_scheme_manager
                        .get(&self.current_color_scheme)
                        .unwrap_or_else(|_| self.color_scheme_manager.get_default());

                    // Apply reversal if needed
                    let mut data_u32 = lut_data.to_u32_buffer();
                    if self.color_scheme_reversed {
                        data_u32[0..256].reverse();
                        data_u32[256..512].reverse();
                        data_u32[512..768].reverse();
                    }

                    queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&data_u32));
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.cursor_size = size as f32;
                }
            }
            "cursor_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.cursor_strength = strength as f32;
                }
            }
            _ => {
                tracing::warn!("Unknown state parameter for SlimeMold: {}", state_name);
            }
        }
        Ok(())
    }

    fn get_settings(&self) -> serde_json::Value {
        // Return settings as-is, matching other sims (no snake_case conversion)
        serde_json::to_value(&self.settings).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::json!({
            "agent_count": self.agent_count,
            "current_width": self.current_width,
            "current_height": self.current_height,
            "color_scheme_reversed": self.color_scheme_reversed,
            "current_color_scheme": self.current_color_scheme,
            "gui_visible": self.gui_visible,
            "cursor_size": self.cursor_size,
            "cursor_strength": self.cursor_strength,
            "position_generator": crate::simulations::shared::SlimeMoldPositionGenerator::as_str(&self.position_generator),
            "trail_map_filtering": super::settings::TrailMapFiltering::as_str(&self.trail_map_filtering),
            "mask_pattern": self.state.mask_pattern.as_str(),
            "mask_target": self.state.mask_target.as_str(),
            "mask_strength": self.state.mask_strength,
            "mask_curve": self.state.mask_curve,
            "mask_reversed": self.state.mask_reversed,
            "mask_mirror_horizontal": self.state.mask_mirror_horizontal,
            "mask_mirror_vertical": self.state.mask_mirror_vertical,
            "mask_invert_tone": self.state.mask_invert_tone,
            "mask_image_fit_mode": self.state.mask_image_fit_mode.as_str(),
            "camera": {
                "position": self.camera.position,
                "zoom": self.camera.zoom
            },
            "simulation_time": 0.0,
            "is_running": true
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Determine cursor mode based on mouse_button
        let cursor_mode = if mouse_button == 0 {
            1 // left click = attract
        } else if mouse_button == 2 {
            2 // right click = repel
        } else {
            0 // middle click or other = no interaction
        };

        // Convert world coordinates [-1, 1] to simulation pixel coordinates [0, width] x [0, height]
        // World space is [-1, 1] where (-1, -1) is bottom-left and (1, 1) is top-right
        // Simulation space is [0, width] x [0, height] where (0, 0) is top-left
        let sim_x = ((world_x + 1.0) * 0.5) * self.current_width as f32;
        let sim_y = ((1.0 - world_y) * 0.5) * self.current_height as f32; // Flip Y axis

        self.cursor_active_mode = cursor_mode;
        self.cursor_world_x = sim_x;
        self.cursor_world_y = sim_y;

        tracing::debug!(
            "Slime mold cursor interaction: world=({:.3}, {:.3}), sim=({:.1}, {:.1}), mode={}, dimensions={}x{}",
            world_x,
            world_y,
            sim_x,
            sim_y,
            cursor_mode,
            self.current_width,
            self.current_height
        );

        self.update_cursor_params(queue);
        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Turn off cursor interaction
        self.cursor_active_mode = 0;
        self.cursor_world_x = 0.0;
        self.cursor_world_y = 0.0;

        tracing::debug!("Slime Mold mouse release: cursor interaction disabled");

        // Update cursor parameters on GPU
        self.update_cursor_params(queue);
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

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // This would need to be implemented with the preset manager
        // For now, we'll return an error indicating it needs to be implemented
        Err("Preset saving not yet implemented for SlimeMoldModel".into())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // This would need to be implemented with the preset manager
        // For now, we'll return an error indicating it needs to be implemented
        Err("Preset loading not yet implemented for SlimeMoldModel".into())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.reset_trails(queue);
        Ok(())
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
    ) -> SimulationResult<()> {
        self.settings.randomize();
        self.update_settings(self.settings.clone(), queue);
        Ok(())
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let new_settings: Settings =
            serde_json::from_value(settings).map_err(|e| SimulationError::InvalidSetting {
                setting_name: "settings".to_string(),
                message: e.to_string(),
            })?;
        self.update_settings(new_settings, queue);
        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let color_scheme_data_u32 = color_scheme.to_u32_buffer();
        queue.write_buffer(
            &self.lut_buffer,
            0,
            bytemuck::cast_slice(&color_scheme_data_u32),
        );
        Ok(())
    }
}

// Helper functions (moved from gpu_state.rs)

fn create_agent_buffer(device: &wgpu::Device, agent_count: usize) -> wgpu::Buffer {
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

#[allow(clippy::too_many_arguments)]
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

    // Use separate staging buffers to avoid usage conflicts
    let read_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Agent Scaling Read Staging Buffer"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

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
    device
        .poll(wgpu::wgt::PollType::Wait)
        .expect("Failed to poll device");
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
    state: &SlimeMoldState,
    sim_size_buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    physical_width: u32,
    physical_height: u32,
    position_generator: &crate::simulations::shared::SlimeMoldPositionGenerator,
) {
    let sim_size_uniform = SimSizeUniform::new(
        physical_width,
        physical_height,
        settings.pheromone_decay_rate,
        settings,
        state,
        position_generator,
    );
    queue.write_buffer(
        sim_size_buffer,
        0,
        bytemuck::cast_slice(&[sim_size_uniform]),
    );
}

#[allow(clippy::too_many_arguments)]
fn scale_trail_map_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    old_buffer: &wgpu::Buffer,
    new_buffer: &wgpu::Buffer,
    old_width: u32,
    old_height: u32,
    new_width: u32,
    new_height: u32,
) {
    let old_size = (old_width * old_height) as usize * std::mem::size_of::<f32>();
    let new_size = (new_width * new_height) as usize * std::mem::size_of::<f32>();

    // For small size changes, use a more efficient approach
    if new_size <= old_size * 2 && old_size <= new_size * 2 {
        // Use separate staging buffers to avoid usage conflicts
        let read_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Scaling Read Staging Buffer"),
            size: old_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let write_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Scaling Write Staging Buffer"),
            size: new_size as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: true,
        });

        // Copy old buffer to read staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Trail Map Scaling Copy Old"),
        });
        encoder.copy_buffer_to_buffer(old_buffer, 0, &read_staging_buffer, 0, old_size as u64);
        queue.submit(std::iter::once(encoder.finish()));

        // Wait for copy to complete and map for reading
        let (sender, receiver) = std::sync::mpsc::channel();
        read_staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        device
            .poll(wgpu::wgt::PollType::Wait)
            .expect("Failed to poll device");
        receiver.recv().unwrap().unwrap();

        // Read old data and scale to new dimensions
        {
            let buffer_slice = read_staging_buffer.slice(..).get_mapped_range();
            let old_trail_data: &[f32] = bytemuck::cast_slice(&buffer_slice);

            let mut write_buffer_slice = write_staging_buffer.slice(..).get_mapped_range_mut();
            let new_trail_data: &mut [f32] = bytemuck::cast_slice_mut(&mut write_buffer_slice);

            // Initialize new buffer with zeros
            for element in new_trail_data.iter_mut() {
                *element = 0.0;
            }

            // Scale old data to new dimensions using nearest neighbor sampling
            for new_y in 0..new_height {
                for new_x in 0..new_width {
                    // Map new coordinates to old coordinates
                    let old_x = (new_x as f32 * old_width as f32 / new_width as f32) as u32;
                    let old_y = (new_y as f32 * old_height as f32 / new_height as f32) as u32;

                    // Clamp to old dimensions
                    let old_x = old_x.min(old_width - 1);
                    let old_y = old_y.min(old_height - 1);

                    // Copy value from old position to new position
                    let old_idx = (old_y * old_width + old_x) as usize;
                    let new_idx = (new_y * new_width + new_x) as usize;

                    if old_idx < old_trail_data.len() && new_idx < new_trail_data.len() {
                        new_trail_data[new_idx] = old_trail_data[old_idx];
                    }
                }
            }

            drop(write_buffer_slice);
            write_staging_buffer.unmap();
        }

        read_staging_buffer.unmap();

        // Copy scaled data to final buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Trail Map Scaling Copy New"),
        });
        encoder.copy_buffer_to_buffer(&write_staging_buffer, 0, new_buffer, 0, new_size as u64);
        queue.submit(std::iter::once(encoder.finish()));
    } else {
        // For large size changes, use separate buffers to avoid memory issues
        // Create read staging buffer for old data
        let read_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Scaling Read Staging Buffer"),
            size: old_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create write staging buffer for new data
        let write_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Trail Map Scaling Write Staging Buffer"),
            size: new_size as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: true,
        });

        // Copy old buffer to read staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Trail Map Scaling Copy Old"),
        });
        encoder.copy_buffer_to_buffer(old_buffer, 0, &read_staging_buffer, 0, old_size as u64);
        queue.submit(std::iter::once(encoder.finish()));

        // Wait for copy to complete and map for reading
        let (sender, receiver) = std::sync::mpsc::channel();
        read_staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        device
            .poll(wgpu::wgt::PollType::Wait)
            .expect("Failed to poll device");
        receiver.recv().unwrap().unwrap();

        // Read old data and scale to new dimensions
        {
            let buffer_slice = read_staging_buffer.slice(..).get_mapped_range();
            let old_trail_data: &[f32] = bytemuck::cast_slice(&buffer_slice);

            let mut write_buffer_slice = write_staging_buffer.slice(..).get_mapped_range_mut();
            let new_trail_data: &mut [f32] = bytemuck::cast_slice_mut(&mut write_buffer_slice);

            // Initialize new buffer with zeros
            for element in new_trail_data.iter_mut() {
                *element = 0.0;
            }

            // Scale old data to new dimensions using nearest neighbor sampling
            for new_y in 0..new_height {
                for new_x in 0..new_width {
                    // Map new coordinates to old coordinates
                    let old_x = (new_x as f32 * old_width as f32 / new_width as f32) as u32;
                    let old_y = (new_y as f32 * old_height as f32 / new_height as f32) as u32;

                    // Clamp to old dimensions
                    let old_x = old_x.min(old_width - 1);
                    let old_y = old_y.min(old_height - 1);

                    // Copy value from old position to new position
                    let old_idx = (old_y * old_width + old_x) as usize;
                    let new_idx = (new_y * new_width + new_x) as usize;

                    if old_idx < old_trail_data.len() && new_idx < new_trail_data.len() {
                        new_trail_data[new_idx] = old_trail_data[old_idx];
                    }
                }
            }

            drop(write_buffer_slice);
            write_staging_buffer.unmap();
        }

        read_staging_buffer.unmap();

        // Copy scaled data to final buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Trail Map Scaling Copy New"),
        });
        encoder.copy_buffer_to_buffer(&write_staging_buffer, 0, new_buffer, 0, new_size as u64);
        queue.submit(std::iter::once(encoder.finish()));
    }
}

impl SlimeMoldModel {
    pub fn update_display_sampler(&mut self, device: &Arc<Device>) {
        self.display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: self.app_settings.texture_filtering.into(),
            min_filter: self.app_settings.texture_filtering.into(),
            mipmap_filter: self.app_settings.texture_filtering.into(),
            ..Default::default()
        });
    }

    /// Start webcam capture for mask input
    pub fn start_webcam_capture(&mut self, device_index: i32) -> SimulationResult<()> {
        self.webcam_capture
            .set_target_dimensions(self.current_width, self.current_height);
        let result = self.webcam_capture.start_capture(device_index);
        if let Err(e) = &result {
            tracing::error!("webcam_capture.start_capture failed: {}", e);
        }
        result
    }

    /// Stop webcam capture
    pub fn stop_webcam_capture(&mut self) {
        self.webcam_capture.stop_capture();
    }

    /// Update mask buffer with latest webcam frame
    pub fn update_mask_from_webcam(&mut self, queue: &Arc<Queue>) -> SimulationResult<()> {
        if let Some(frame_data) = self.webcam_capture.get_latest_frame_data() {
            let buffer = self.webcam_capture.frame_data_to_gradient_buffer(
                &frame_data,
                self.current_width,
                self.current_height,
            )?;

            // Tone inversion, mirroring, and mask strength/curve handled in the shader

            // Upload raw grayscale to GPU buffer
            queue.write_buffer(
                &self.mask_buffer,
                0,
                bytemuck::cast_slice::<f32, u8>(&buffer),
            );

            // Store for reprocessing if needed
            self.mask_image_raw = Some(buffer);
            self.mask_image_needs_upload = false;
        } else {
            // No new data this tick: keep the last uploaded mask intact
        }
        Ok(())
    }

    /// Check if webcam is available
    pub fn is_webcam_available(&self, device_index: i32) -> bool {
        crate::simulations::shared::WebcamCapture::is_webcam_available(device_index)
    }

    /// Get list of available webcam devices
    pub fn get_available_webcam_devices(&self) -> Vec<i32> {
        crate::simulations::shared::WebcamCapture::get_available_devices()
    }
}
