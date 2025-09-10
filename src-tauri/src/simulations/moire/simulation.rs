//! # Moiré Simulation Module
//!
//! Implements a moiré pattern visualization with fluid advection. This simulation
//! creates complex visual effects by combining mathematical pattern generation
//! and flow fields.
//!
//! ## Technical Overview
//!
//! The simulation uses a compute shader to:
//! 1. Generate moiré interference patterns from overlapping grids
//! 2. Map these patterns to colors using LUT sampling
//! 3. Apply fluid advection for temporal evolution
//! 4. Render the results with color modulation

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupLayoutDescriptor, Buffer,
    BufferDescriptor, BufferUsages, ComputePipeline, ComputePipelineDescriptor, Device, FilterMode,
    PipelineLayoutDescriptor, Queue, RenderPipeline, RenderPipelineDescriptor, SamplerDescriptor,
    ShaderStages, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::commands::AppSettings;
use crate::error::SimulationResult;
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::{ColorScheme, ColorSchemeManager};
use crate::simulations::traits::Simulation;

use super::settings::Settings;
use super::shaders::{COMPUTE_SHADER, RENDER_INFINITE_SHADER};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    time: f32,
    width: f32,
    height: f32,
    base_freq: f32,
    moire_amount: f32,
    moire_rotation: f32,
    moire_scale: f32,
    moire_interference: f32,
    moire_rotation3: f32,
    moire_scale3: f32,
    moire_weight3: f32,
    color_scheme_reversed: f32,
    advect_strength: f32,
    advect_speed: f32,
    curl: f32,
    decay: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RenderParams {
    filtering_mode: u32, // 0 = nearest, 1 = linear, 2 = lanczos
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

#[derive(Debug)]
pub struct MoireModel {
    pub settings: Settings,

    // GPU resources
    compute_pipeline: ComputePipeline,
    render_infinite_pipeline: RenderPipeline,

    // Textures for double buffering
    texture1: Texture,
    texture2: Texture,
    current_texture: usize,

    // Buffers
    params_buffer: Buffer,
    lut_buffer: Buffer,
    texture_render_params_buffer: Buffer,

    // Bind groups
    compute_bind_group1: BindGroup,
    compute_bind_group2: BindGroup,
    render_bind_group1: BindGroup,
    render_bind_group2: BindGroup,
    render_infinite_bind_group1: BindGroup,
    render_infinite_bind_group2: BindGroup,
    camera_bind_group: BindGroup,

    // Camera for infinite rendering
    camera: Camera,

    // Simulation state
    time: f32,
    width: u32,
    height: u32,
}

impl MoireModel {
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
        tiles_needed.max(min_tiles).min(1024) // Cap at 1024x1024 for performance
    }

    /// Create textures for the given dimensions
    fn create_textures(
        device: &Arc<Device>,
        width: u32,
        height: u32,
    ) -> SimulationResult<(Texture, Texture)> {
        let texture_desc = TextureDescriptor {
            label: Some("Moiré Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let texture1 = device.create_texture(&texture_desc);
        let texture2 = device.create_texture(&texture_desc);
        Ok((texture1, texture2))
    }

    pub fn new(
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        _app_settings: &Arc<AppSettings>,
        _lut_manager: &ColorSchemeManager,
    ) -> SimulationResult<Self> {
        let width = surface_config.width;
        let height = surface_config.height;

        // Initialize camera for infinite rendering
        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )?;

        // Create shader modules
        let compute_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Moiré Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
        });

        let render_infinite_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Moiré Render Infinite Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_INFINITE_SHADER.into()),
        });

        // Create textures for double buffering
        let (texture1, texture2) = Self::create_textures(device, width, height)?;

        // Create sampler
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Moiré Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // Create buffers
        let params_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Moiré Params Buffer"),
            size: std::mem::size_of::<Params>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize LUT buffer with default color scheme
        let default_lut = _lut_manager
            .get(&settings.color_scheme_name)
            .unwrap_or_else(|_| _lut_manager.get_default());
        let lut_data = default_lut.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Moiré LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Create texture render params buffer
        let render_params = RenderParams {
            filtering_mode: 1, // Linear filtering
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        let texture_render_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Moiré Texture Render Params Buffer"),
                contents: bytemuck::cast_slice(&[render_params]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        // Create bind group layouts
        let compute_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Moiré Compute Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_texture_entry(
                        0,
                        ShaderStages::COMPUTE,
                        wgpu::StorageTextureAccess::WriteOnly,
                        TextureFormat::Rgba8Unorm,
                    ),
                    resource_helpers::uniform_buffer_entry(1, ShaderStages::COMPUTE),
                    resource_helpers::storage_buffer_entry(2, ShaderStages::COMPUTE, true),
                    resource_helpers::texture_entry(
                        3,
                        ShaderStages::COMPUTE,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        4,
                        ShaderStages::COMPUTE,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        let render_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Moiré Render Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        // Create infinite render bind group layout
        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Moiré Render Infinite Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    resource_helpers::uniform_buffer_entry(2, ShaderStages::FRAGMENT),
                ],
            });

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    ShaderStages::VERTEX,
                )],
            });

        // Create pipelines
        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Moiré Compute Pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Moiré Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            })),
            module: &compute_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create infinite render pipeline
        let render_infinite_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Moiré Render Infinite Pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Moiré Render Infinite Pipeline Layout"),
                bind_group_layouts: &[
                    &render_infinite_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &render_infinite_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_infinite_module,
                entry_point: Some("fs_main_texture"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
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

        // Create texture views
        let texture1_view = texture1.create_view(&TextureViewDescriptor::default());
        let texture2_view = texture2.create_view(&TextureViewDescriptor::default());

        // Create bind groups
        let compute_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 1"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::buffer_entry(1, &params_buffer),
                resource_helpers::buffer_entry(2, &lut_buffer),
                resource_helpers::texture_view_entry(3, &texture2_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
            ],
        });

        let compute_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 2"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture2_view),
                resource_helpers::buffer_entry(1, &params_buffer),
                resource_helpers::buffer_entry(2, &lut_buffer),
                resource_helpers::texture_view_entry(3, &texture1_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
            ],
        });

        let render_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Bind Group 1"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
            ],
        });

        let render_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Bind Group 2"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture2_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
            ],
        });

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, camera.buffer())],
        });

        // Create infinite render bind groups
        let render_infinite_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Infinite Bind Group 1"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
                resource_helpers::buffer_entry(2, &texture_render_params_buffer),
            ],
        });

        let render_infinite_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Infinite Bind Group 2"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture2_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
                resource_helpers::buffer_entry(2, &texture_render_params_buffer),
            ],
        });

        Ok(Self {
            settings,
            compute_pipeline,
            render_infinite_pipeline,
            texture1,
            texture2,
            current_texture: 0,
            params_buffer,
            lut_buffer,
            texture_render_params_buffer,
            compute_bind_group1,
            compute_bind_group2,
            render_bind_group1,
            render_bind_group2,
            render_infinite_bind_group1,
            render_infinite_bind_group2,
            camera_bind_group,
            camera,
            time: 0.0,
            width,
            height,
        })
    }

    pub fn reset_flow(&self, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Moiré Reset Flow"),
        });

        // Clear both textures
        {
            let _clear_pass1 = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass 1"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture1.create_view(&TextureViewDescriptor::default()),
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
        }

        {
            let _clear_pass2 = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass 2"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture2.create_view(&TextureViewDescriptor::default()),
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
        }

        queue.submit([encoder.finish()]);
        Ok(())
    }

    fn update_params(&self, queue: &Arc<Queue>) {
        let params = Params {
            time: self.time,
            width: self.width as f32,
            height: self.height as f32,
            base_freq: self.settings.base_freq,
            moire_amount: self.settings.moire_amount,
            moire_rotation: self.settings.moire_rotation,
            moire_scale: self.settings.moire_scale,
            moire_interference: self.settings.moire_interference,
            moire_rotation3: self.settings.moire_rotation3,
            moire_scale3: self.settings.moire_scale3,
            moire_weight3: self.settings.moire_weight3,
            color_scheme_reversed: if self.settings.color_scheme_reversed {
                1.0
            } else {
                0.0
            },
            advect_strength: self.settings.advect_strength,
            advect_speed: self.settings.advect_speed,
            curl: self.settings.curl,
            decay: self.settings.decay,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
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
}

impl Simulation for MoireModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        self.time += delta_time * self.settings.speed;
        self.update_params(queue);

        // Update camera and upload to GPU
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Moiré Render"),
        });

        // Compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Moiré Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);

            let compute_bind_group = if self.current_texture == 0 {
                &self.compute_bind_group1
            } else {
                &self.compute_bind_group2
            };

            compute_pass.set_bind_group(0, compute_bind_group, &[]);
            compute_pass.dispatch_workgroups((self.width + 7) / 8, (self.height + 7) / 8, 1);
        }

        // Infinite render pass
        {
            let tile_count = self.calculate_tile_count();
            let total_instances = tile_count * tile_count;

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Moiré Infinite Render Pass"),
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

            render_pass.set_pipeline(&self.render_infinite_pipeline);

            let render_infinite_bind_group = if self.current_texture == 0 {
                &self.render_infinite_bind_group1
            } else {
                &self.render_infinite_bind_group2
            };

            render_pass.set_bind_group(0, render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }

        queue.submit([encoder.finish()]);

        // Swap textures for double buffering
        self.current_texture = 1 - self.current_texture;

        Ok(())
    }

    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Just render without updating time
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Moiré Render Paused"),
        });

        {
            let tile_count = self.calculate_tile_count();
            let total_instances = tile_count * tile_count;

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Moiré Infinite Render Pass Paused"),
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

            render_pass.set_pipeline(&self.render_infinite_pipeline);

            let render_infinite_bind_group = if self.current_texture == 0 {
                &self.render_infinite_bind_group1
            } else {
                &self.render_infinite_bind_group2
            };

            render_pass.set_bind_group(0, render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }

        queue.submit([encoder.finish()]);
        Ok(())
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.width = new_config.width;
        self.height = new_config.height;

        // Update camera viewport with new dimensions
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        // Recreate textures with new dimensions
        let (new_texture1, new_texture2) = Self::create_textures(device, self.width, self.height)?;
        self.texture1 = new_texture1;
        self.texture2 = new_texture2;

        // Create new texture views
        let texture1_view = self.texture1.create_view(&TextureViewDescriptor::default());
        let texture2_view = self.texture2.create_view(&TextureViewDescriptor::default());

        // Create sampler (reuse existing one)
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Moiré Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        // Recreate bind group layouts
        let compute_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Moiré Compute Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_texture_entry(
                        0,
                        ShaderStages::COMPUTE,
                        wgpu::StorageTextureAccess::WriteOnly,
                        TextureFormat::Rgba8Unorm,
                    ),
                    resource_helpers::uniform_buffer_entry(1, ShaderStages::COMPUTE),
                    resource_helpers::storage_buffer_entry(2, ShaderStages::COMPUTE, true),
                    resource_helpers::texture_entry(
                        3,
                        ShaderStages::COMPUTE,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        4,
                        ShaderStages::COMPUTE,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        let render_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Moiré Render Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Moiré Render Infinite Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    resource_helpers::uniform_buffer_entry(2, ShaderStages::FRAGMENT),
                ],
            });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    ShaderStages::VERTEX,
                )],
            });

        // Recreate compute bind groups
        self.compute_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 1"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::buffer_entry(1, &self.params_buffer),
                resource_helpers::buffer_entry(2, &self.lut_buffer),
                resource_helpers::texture_view_entry(3, &texture2_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
            ],
        });

        self.compute_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 2"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture2_view),
                resource_helpers::buffer_entry(1, &self.params_buffer),
                resource_helpers::buffer_entry(2, &self.lut_buffer),
                resource_helpers::texture_view_entry(3, &texture1_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
            ],
        });

        // Recreate render bind groups
        self.render_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Bind Group 1"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
            ],
        });

        self.render_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Bind Group 2"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture2_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
            ],
        });

        // Recreate camera bind group
        self.camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, self.camera.buffer())],
        });

        // Recreate infinite render bind groups
        self.render_infinite_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Infinite Bind Group 1"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
                resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
            ],
        });

        self.render_infinite_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Infinite Bind Group 2"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture2_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
                resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
            ],
        });

        // Clear the new textures
        self.reset_flow(device, queue)?;

        Ok(())
    }

    fn handle_mouse_interaction(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _mouse_button: u32,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No mouse interaction for this simulation
        Ok(())
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&self.settings).unwrap_or_default()
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.settings = serde_json::from_value(settings)?;
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.time = 0.0;
        self.reset_flow(device, queue)?;
        Ok(())
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Randomize moiré parameters
        self.settings.base_freq = 5.0 + rand::random::<f32>() * 45.0; // 5-50
        self.settings.moire_amount = rand::random::<f32>(); // 0-1
        self.settings.moire_rotation = rand::random::<f32>() * 3.14159; // 0-π
        self.settings.moire_scale = 0.8 + rand::random::<f32>() * 0.4; // 0.8-1.2
        self.settings.moire_interference = rand::random::<f32>(); // 0-1
        self.settings.moire_rotation3 = (rand::random::<f32>() - 0.5) * 3.14159; // -π/2 to π/2
        self.settings.moire_scale3 = 0.8 + rand::random::<f32>() * 0.4; // 0.8-1.2
        self.settings.moire_weight3 = rand::random::<f32>(); // 0-1
        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update the LUT buffer with the new color scheme data
        let lut_data = color_scheme.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data));
        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "speed" => self.settings.speed = value.as_f64().unwrap_or(0.01) as f32,
            "base_freq" => self.settings.base_freq = value.as_f64().unwrap_or(20.0) as f32,
            "moire_amount" => self.settings.moire_amount = value.as_f64().unwrap_or(0.5) as f32,
            "moire_rotation" => self.settings.moire_rotation = value.as_f64().unwrap_or(0.2) as f32,
            "moire_scale" => self.settings.moire_scale = value.as_f64().unwrap_or(1.05) as f32,
            "moire_interference" => {
                self.settings.moire_interference = value.as_f64().unwrap_or(0.5) as f32
            }
            "moire_rotation3" => {
                self.settings.moire_rotation3 = value.as_f64().unwrap_or(-0.1) as f32
            }
            "moire_scale3" => self.settings.moire_scale3 = value.as_f64().unwrap_or(1.1) as f32,
            "moire_weight3" => self.settings.moire_weight3 = value.as_f64().unwrap_or(0.3) as f32,
            "advect_strength" => {
                self.settings.advect_strength = value.as_f64().unwrap_or(0.3) as f32
            }
            "advect_speed" => self.settings.advect_speed = value.as_f64().unwrap_or(1.0) as f32,
            "curl" => self.settings.curl = value.as_f64().unwrap_or(0.5) as f32,
            "decay" => self.settings.decay = value.as_f64().unwrap_or(0.99) as f32,
            "color_scheme_name" => {
                self.settings.color_scheme_name = value.as_str().unwrap_or("viridis").to_string()
            }
            "color_scheme_reversed" => {
                self.settings.color_scheme_reversed = value.as_bool().unwrap_or(false)
            }
            _ => return Err(format!("Unknown setting: {}", setting_name).into()),
        }
        Ok(())
    }

    fn update_state(
        &mut self,
        _state_name: &str,
        _value: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Moiré simulation doesn't have updateable state
        Ok(())
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::json!({
            "time": self.time,
            "width": self.width,
            "height": self.height
        })
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No mouse interaction for this simulation
        Ok(())
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // Preset saving is handled by the preset manager
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // Preset loading is handled by the preset manager
        Ok(())
    }
}
