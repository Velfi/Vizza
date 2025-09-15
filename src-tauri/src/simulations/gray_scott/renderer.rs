use crate::error::SimulationResult;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::{
    BindGroupBuilder, CommonBindGroupLayouts, RenderPipelineBuilder, ShaderManager,
};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::Settings;
use super::shaders::RENDER_INFINITE_SHADER;
use crate::simulations::shared::camera::Camera;

#[derive(Debug)]
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface_config: SurfaceConfiguration,
    width: u32,
    height: u32,
    settings: Settings,

    lut_buffer: wgpu::Buffer,
    background_color_buffer: wgpu::Buffer,
    render_params_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    render_infinite_pipeline: wgpu::RenderPipeline,
    background_render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    background_bind_group_layout: wgpu::BindGroupLayout,
    pub camera: Camera,
}

impl Renderer {
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

    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        sim_width: u32,
        sim_height: u32,
        lut_manager: &crate::simulations::shared::ColorSchemeManager,
        app_settings: &crate::commands::app_settings::AppSettings,
    ) -> SimulationResult<Self> {
        let settings = Settings::default();

        // Create LUT buffer (convert u8 to u32 for shader compatibility)
        let lut_data = lut_manager.get_default();
        let lut_data_u32 = lut_data.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create background color buffer (black by default)
        let background_color_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Background Color Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // Black background
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create render parameters buffer
        let filtering_mode = match app_settings.texture_filtering {
            crate::commands::app_settings::TextureFiltering::Nearest => 0u32,
            crate::commands::app_settings::TextureFiltering::Linear => 1u32,
            crate::commands::app_settings::TextureFiltering::Lanczos => 2u32,
        };
        let render_params = [filtering_mode, 0u32, 0u32, 0u32];
        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Params Buffer"),
            contents: bytemuck::cast_slice(&render_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create sampler for texture sampling (non-filtering for Rg32Float)
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Simulation Data Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create simulation data bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                resource_helpers::texture_entry(3, wgpu::ShaderStages::FRAGMENT, wgpu::TextureSampleType::Float { filterable: false }, wgpu::TextureViewDimension::D2),
                resource_helpers::sampler_entry(4, wgpu::ShaderStages::FRAGMENT, wgpu::SamplerBindingType::NonFiltering),
                resource_helpers::storage_buffer_entry(5, wgpu::ShaderStages::FRAGMENT, true),
                resource_helpers::uniform_buffer_entry(6, wgpu::ShaderStages::FRAGMENT),
                resource_helpers::uniform_buffer_entry(7, wgpu::ShaderStages::FRAGMENT),
            ],
        });

        // Initialize GPU utilities
        let mut shader_manager = ShaderManager::new();
        let common_layouts = CommonBindGroupLayouts::new(device);

        // Create camera bind group layout (using common layout)
        let camera_bind_group_layout = common_layouts.camera.clone();

        // Create background bind group layout (using common layout)
        let background_bind_group_layout = common_layouts.uniform_buffer.clone();

        // Initialize camera with appropriate settings for Gray Scott simulation
        // Gray Scott operates in [0,1] UV space, so we want to view that area
        // Use physical pixels for camera viewport (surface configuration dimensions)
        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )?;

        // Load shaders using shader manager
        let shader_infinite = shader_manager.load_shader(
            device,
            "gray_scott_render_infinite",
            RENDER_INFINITE_SHADER,
        );

        let background_shader = shader_manager.load_shader(
            device,
            "gray_scott_background",
            crate::simulations::gray_scott::shaders::BACKGROUND_RENDER_SHADER,
        );

        // Create infinite render pipeline using GPU utilities
        let render_infinite_pipeline = RenderPipelineBuilder::new(device.clone())
            .with_shader(shader_infinite)
            .with_bind_group_layouts(vec![
                bind_group_layout.clone(),
                camera_bind_group_layout.clone(),
            ])
            .with_fragment_targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            })
            .with_label("Render Infinite Pipeline".to_string())
            .build();

        // Create background render pipeline using GPU utilities
        let background_render_pipeline = RenderPipelineBuilder::new(device.clone())
            .with_shader(background_shader)
            .with_bind_group_layouts(vec![
                background_bind_group_layout.clone(),
                camera_bind_group_layout.clone(),
            ])
            .with_fragment_targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .with_primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            })
            .with_label("Background Render Pipeline".to_string())
            .build();

        Ok(Self {
            device: Arc::clone(device),
            queue: Arc::clone(queue),
            surface_config: surface_config.clone(),
            width: sim_width,
            height: sim_height,
            settings,
            lut_buffer,
            background_color_buffer,
            render_params_buffer,
            sampler,
            render_infinite_pipeline,
            background_render_pipeline,
            bind_group_layout,
            camera_bind_group_layout,
            background_bind_group_layout,
            camera,
        })
    }

    pub fn update_settings(&mut self, settings: &Settings, _queue: &Arc<Queue>) {
        self.settings = settings.clone();
        // LUT management is now handled by the simulation manager
    }

    pub fn update_lut(
        &mut self,
        lut_data: &crate::simulations::shared::ColorScheme,
        queue: &Queue,
    ) {
        let lut_data_u32 = lut_data.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
    }

    pub fn create_bind_group(
        &self,
        simulation_texture: &wgpu::Texture,
        params_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let texture_view = simulation_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(2, &self.background_color_buffer),
                resource_helpers::texture_view_entry(3, &texture_view),
                resource_helpers::sampler_bind_entry(4, &self.sampler),
                resource_helpers::buffer_entry(5, &self.lut_buffer),
                resource_helpers::buffer_entry(6, params_buffer),
                resource_helpers::buffer_entry(7, &self.render_params_buffer),
            ],
        })
    }

    pub fn create_camera_bind_group(&self) -> wgpu::BindGroup {
        BindGroupBuilder::new(&self.device, &self.camera_bind_group_layout)
            .add_buffer(0, self.camera.buffer())
            .with_label("Camera Bind Group".to_string())
            .build()
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        simulation_texture: &wgpu::Texture,
        params_buffer: &wgpu::Buffer,
        background_bind_group: &wgpu::BindGroup,
    ) -> Result<(), wgpu::SurfaceError> {
        // Update camera data on GPU
        self.camera.upload_to_gpu(&self.queue);

        let bind_group = self.create_bind_group(simulation_texture, params_buffer);
        let camera_bind_group = self.create_camera_bind_group();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Gray Scott Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gray Scott Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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

            // Render background first
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, background_bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1);

            // Then render main simulation content with infinite tiling
            let tile_count = self.calculate_tile_count();
            let total_instances = tile_count * tile_count;
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn resize(&mut self, new_config: &SurfaceConfiguration) -> SimulationResult<()> {
        self.surface_config = new_config.clone();
        self.width = new_config.width;
        self.height = new_config.height;

        // Update camera viewport with physical pixels
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        Ok(())
    }

    pub fn get_surface_config(&self) -> &SurfaceConfiguration {
        &self.surface_config
    }

    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    pub fn background_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.background_bind_group_layout
    }
}
