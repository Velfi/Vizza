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
    ShaderStages, SurfaceConfiguration, Texture, TextureFormat, TextureView, TextureViewDescriptor,
};

use crate::commands::AppSettings;
use crate::error::SimulationResult;
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::ping_pong_textures::PingPongTextures;
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
    generator_type: f32, // 0 = Linear, 1 = Radial
    base_freq: f32,
    moire_amount: f32,
    moire_rotation: f32,
    moire_scale: f32,
    moire_interference: f32,
    moire_rotation3: f32,
    moire_scale3: f32,
    moire_weight3: f32,
    radial_swirl_strength: f32,
    radial_starburst_count: f32,
    radial_center_brightness: f32,
    color_scheme_reversed: f32,
    advect_strength: f32,
    advect_speed: f32,
    curl: f32,
    decay: f32,
    image_loaded: f32,
    image_mode_enabled: f32,
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

    simulation_textures: PingPongTextures,

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

    // LUT management
    pub color_scheme_manager: Arc<ColorSchemeManager>,
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,

    // Simulation state
    time: f32,
    width: u32,
    height: u32,

    // Optional image resources
    image_texture: Option<Texture>,
    image_view: Option<TextureView>,
    image_original: Option<image::DynamicImage>,
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

    /// Create double buffer textures for the given dimensions
    fn create_double_buffer(
        device: &Arc<Device>,
        width: u32,
        height: u32,
    ) -> SimulationResult<PingPongTextures> {
        Ok(PingPongTextures::new(
            device,
            width,
            height,
            TextureFormat::Rgba8Unorm,
            "Moiré Texture",
        ))
    }

    pub fn new(
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        _app_settings: &Arc<AppSettings>,
        lut_manager: &ColorSchemeManager,
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

        // Create simulation textures
        let simulation_textures = Self::create_double_buffer(device, width, height)?;

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
        let default_lut_name = "ZELDA_Fordite";
        let default_lut = lut_manager.get(default_lut_name).unwrap();
        let lut_data = default_lut.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Moiré LUT Buffer for {}", default_lut_name)),
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
                    resource_helpers::texture_entry(
                        5,
                        ShaderStages::COMPUTE,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
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

        // Get texture views from simulation textures
        let texture_a_view = simulation_textures.current_view();
        let texture_b_view = simulation_textures.inactive_view();

        // Create bind groups
        let compute_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 1"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, texture_a_view),
                resource_helpers::buffer_entry(1, &params_buffer),
                resource_helpers::buffer_entry(2, &lut_buffer),
                resource_helpers::texture_view_entry(3, texture_b_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
                resource_helpers::texture_view_entry(5, texture_b_view),
            ],
        });

        let compute_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 2"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, texture_b_view),
                resource_helpers::buffer_entry(1, &params_buffer),
                resource_helpers::buffer_entry(2, &lut_buffer),
                resource_helpers::texture_view_entry(3, texture_a_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
                resource_helpers::texture_view_entry(5, texture_a_view),
            ],
        });

        let render_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Bind Group 1"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, texture_a_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
            ],
        });

        let render_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Bind Group 2"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, texture_b_view),
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
                resource_helpers::texture_view_entry(0, texture_a_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
                resource_helpers::buffer_entry(2, &texture_render_params_buffer),
            ],
        });

        let render_infinite_bind_group2 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Render Infinite Bind Group 2"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, texture_b_view),
                resource_helpers::sampler_bind_entry(1, &sampler),
                resource_helpers::buffer_entry(2, &texture_render_params_buffer),
            ],
        });

        Ok(Self {
            settings,
            compute_pipeline,
            render_infinite_pipeline,
            simulation_textures,
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
            color_scheme_manager: Arc::new(lut_manager.clone()),
            current_color_scheme: default_lut_name.to_string(),
            color_scheme_reversed: false,
            time: 0.0,
            width,
            height,
            image_texture: None,
            image_view: None,
            image_original: None,
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
                    view: self.simulation_textures.current_view(),
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
                    view: self.simulation_textures.inactive_view(),
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
        let generator_type = match self.settings.generator_type {
            super::settings::MoireGeneratorType::Linear => 0.0,
            super::settings::MoireGeneratorType::Radial => 1.0,
        };
        
        let params = Params {
            time: self.time,
            width: self.width as f32,
            height: self.height as f32,
            generator_type,
            base_freq: self.settings.base_freq,
            moire_amount: self.settings.moire_amount,
            moire_rotation: self.settings.moire_rotation,
            moire_scale: self.settings.moire_scale,
            moire_interference: self.settings.moire_interference,
            moire_rotation3: self.settings.moire_rotation3,
            moire_scale3: self.settings.moire_scale3,
            moire_weight3: self.settings.moire_weight3,
            radial_swirl_strength: self.settings.radial_swirl_strength,
            radial_starburst_count: self.settings.radial_starburst_count,
            radial_center_brightness: self.settings.radial_center_brightness,
            color_scheme_reversed: if self.color_scheme_reversed { 1.0 } else { 0.0 },
            advect_strength: self.settings.advect_strength,
            advect_speed: self.settings.advect_speed,
            curl: self.settings.curl,
            decay: self.settings.decay,
            image_loaded: if self.image_view.is_some() { 1.0 } else { 0.0 },
            image_mode_enabled: if self.settings.image_mode_enabled {
                1.0
            } else {
                0.0
            },
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
    }

    pub fn load_image_from_path(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        path: &str,
    ) -> SimulationResult<()> {
        let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;
        self.load_image_from_data(device, queue, img)
    }

    pub fn load_image_from_data(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        img: image::DynamicImage,
    ) -> SimulationResult<()> {
        self.image_original = Some(img.clone());
        self.reprocess_image_with_current_fit_mode(device, queue)
    }

    pub fn reprocess_image_with_current_fit_mode(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if let Some(original) = &self.image_original {
            let target_w = self.width.max(1);
            let target_h = self.height.max(1);

            let mut gray = original.to_luma8();
            if self.settings.image_mirror_horizontal {
                image::imageops::flip_horizontal_in_place(&mut gray);
            }
            if self.settings.image_invert_tone {
                for p in gray.pixels_mut() {
                    p.0[0] = 255 - p.0[0];
                }
            }

            let processed = match self.settings.image_fit_mode {
                super::settings::GradientImageFitMode::Stretch => image::imageops::resize(
                    &gray,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                ),
                super::settings::GradientImageFitMode::Center => {
                    let mut canvas = image::ImageBuffer::new(target_w, target_h);
                    let (img_w, img_h) = (gray.width(), gray.height());
                    let start_x = if img_w < target_w {
                        (target_w - img_w) / 2
                    } else {
                        0
                    };
                    let start_y = if img_h < target_h {
                        (target_h - img_h) / 2
                    } else {
                        0
                    };
                    for (x, y, pixel) in gray.enumerate_pixels() {
                        let cx = start_x + x;
                        let cy = start_y + y;
                        if cx < target_w && cy < target_h {
                            canvas.put_pixel(cx, cy, *pixel);
                        }
                    }
                    canvas
                }
                super::settings::GradientImageFitMode::FitH => {
                    let scale = target_w as f32 / gray.width() as f32;
                    let new_h = (gray.height() as f32 * scale) as u32;
                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        new_h,
                        image::imageops::FilterType::Lanczos3,
                    );
                    let mut canvas = image::ImageBuffer::new(target_w, target_h);
                    let start_y = if new_h < target_h {
                        (target_h - new_h) / 2
                    } else {
                        0
                    };
                    for (x, y, pixel) in resized.enumerate_pixels() {
                        let cy = start_y + y;
                        if cy < target_h {
                            canvas.put_pixel(x, cy, *pixel);
                        }
                    }
                    canvas
                }
                super::settings::GradientImageFitMode::FitV => {
                    let scale = target_h as f32 / gray.height() as f32;
                    let new_w = (gray.width() as f32 * scale) as u32;
                    let resized = image::imageops::resize(
                        &gray,
                        new_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );
                    let mut canvas = image::ImageBuffer::new(target_w, target_h);
                    let start_x = if new_w < target_w {
                        (target_w - new_w) / 2
                    } else {
                        0
                    };
                    for (x, y, pixel) in resized.enumerate_pixels() {
                        let cx = start_x + x;
                        if cx < target_w {
                            canvas.put_pixel(cx, y, *pixel);
                        }
                    }
                    canvas
                }
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Moire Image Texture"),
                size: wgpu::Extent3d {
                    width: target_w,
                    height: target_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&TextureViewDescriptor::default());

            // Upload grayscale data directly
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &processed.into_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(target_w),
                    rows_per_image: Some(target_h),
                },
                wgpu::Extent3d {
                    width: target_w,
                    height: target_h,
                    depth_or_array_layers: 1,
                },
            );

            self.image_texture = Some(texture);
            self.image_view = Some(view);

            // Rebuild compute bind groups to bind the image at binding 5
            self.rebuild_compute_bind_groups(device);
        }
        Ok(())
    }

    fn rebuild_compute_bind_groups(&mut self, device: &Arc<Device>) {
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
                    resource_helpers::texture_entry(
                        5,
                        ShaderStages::COMPUTE,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                ],
            });

        let texture1_view = self.simulation_textures.current_view();
        let texture2_view = self.simulation_textures.inactive_view();
        let image_view = self.image_view.as_ref().unwrap_or(&texture2_view);

        self.compute_bind_group1 = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Moiré Compute Bind Group 1"),
            layout: &compute_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture1_view),
                resource_helpers::buffer_entry(1, &self.params_buffer),
                resource_helpers::buffer_entry(2, &self.lut_buffer),
                resource_helpers::texture_view_entry(3, &texture2_view),
                resource_helpers::sampler_bind_entry(4, &sampler),
                resource_helpers::texture_view_entry(5, image_view),
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
                resource_helpers::texture_view_entry(5, image_view),
            ],
        });
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

    /// Get the current color scheme name
    pub fn get_current_lut_name(&self) -> &str {
        &self.current_color_scheme
    }

    /// Check if the current color scheme is reversed
    pub fn is_lut_reversed(&self) -> bool {
        self.color_scheme_reversed
    }

    /// Set the color scheme and update the LUT buffer
    pub fn set_color_scheme(
        &mut self,
        lut_name: &str,
        reversed: bool,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.current_color_scheme = lut_name.to_string();
        self.color_scheme_reversed = reversed;

        // Load the new color scheme
        let color_scheme = self
            .color_scheme_manager
            .get(lut_name)
            .map_err(|e| format!("Failed to load color scheme '{}': {}", lut_name, e))?;

        // Update the LUT buffer
        let mut lut_data = color_scheme.to_u32_buffer();

        // Apply reversal if needed
        if self.color_scheme_reversed {
            lut_data[0..256].reverse();
            lut_data[256..512].reverse();
            lut_data[512..768].reverse();
        }

        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data));

        Ok(())
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

            let compute_bind_group = self
                .simulation_textures
                .get_bind_group(&self.compute_bind_group1, &self.compute_bind_group2);

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

            let render_infinite_bind_group = self.simulation_textures.get_bind_group(
                &self.render_infinite_bind_group1,
                &self.render_infinite_bind_group2,
            );

            render_pass.set_bind_group(0, render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }

        queue.submit([encoder.finish()]);

        // Swap textures for double buffering
        self.simulation_textures.swap();

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

            let render_infinite_bind_group = self.simulation_textures.get_bind_group(
                &self.render_infinite_bind_group1,
                &self.render_infinite_bind_group2,
            );

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
        self.simulation_textures = Self::create_double_buffer(device, self.width, self.height)?;

        // Create new texture views
        let texture1_view = self.simulation_textures.current_view();
        let texture2_view = self.simulation_textures.inactive_view();

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

        // Reprocess image to match new size if present
        if self.image_original.is_some() {
            let _ = self.reprocess_image_with_current_fit_mode(device, queue);
        }

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
        let mut lut_data = color_scheme.to_u32_buffer();

        // Apply reversal if needed
        if self.color_scheme_reversed {
            lut_data[0..256].reverse();
            lut_data[256..512].reverse();
            lut_data[512..768].reverse();
        }

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
            "speed" => self.settings.speed = value.as_f64().unwrap() as f32,
            "generator_type" => {
                let s = value.as_str().unwrap_or("Linear");
                self.settings.generator_type = s.parse::<super::settings::MoireGeneratorType>()
                    .map_err(|e| format!("Invalid generator_type: {}", e))?;
            }
            "base_freq" => self.settings.base_freq = value.as_f64().unwrap() as f32,
            "moire_amount" => self.settings.moire_amount = value.as_f64().unwrap() as f32,
            "moire_rotation" => self.settings.moire_rotation = value.as_f64().unwrap() as f32,
            "moire_scale" => self.settings.moire_scale = value.as_f64().unwrap() as f32,
            "moire_interference" => {
                self.settings.moire_interference = value.as_f64().unwrap() as f32
            }
            "moire_rotation3" => self.settings.moire_rotation3 = value.as_f64().unwrap() as f32,
            "moire_scale3" => self.settings.moire_scale3 = value.as_f64().unwrap() as f32,
            "moire_weight3" => self.settings.moire_weight3 = value.as_f64().unwrap() as f32,
            "radial_swirl_strength" => self.settings.radial_swirl_strength = value.as_f64().unwrap() as f32,
            "radial_starburst_count" => self.settings.radial_starburst_count = value.as_f64().unwrap() as f32,
            "radial_center_brightness" => self.settings.radial_center_brightness = value.as_f64().unwrap() as f32,
            "advect_strength" => self.settings.advect_strength = value.as_f64().unwrap() as f32,
            "advect_speed" => self.settings.advect_speed = value.as_f64().unwrap() as f32,
            "curl" => self.settings.curl = value.as_f64().unwrap() as f32,
            "decay" => self.settings.decay = value.as_f64().unwrap_or(0.99) as f32,
            "image_mode_enabled" => {
                self.settings.image_mode_enabled = value.as_bool().unwrap_or(false)
            }
            "image_fit_mode" => {
                let s = value.as_str().unwrap_or("Stretch");
                self.settings.image_fit_mode = s.parse::<super::settings::GradientImageFitMode>()
                    .map_err(|e| format!("Invalid image_fit_mode: {}", e))?;
            }
            "image_mirror_horizontal" => {
                self.settings.image_mirror_horizontal = value.as_bool().unwrap_or(false)
            }
            "image_invert_tone" => {
                self.settings.image_invert_tone = value.as_bool().unwrap_or(false)
            }
            _ => return Err(format!("Unknown setting: {}", setting_name).into()),
        }
        Ok(())
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match state_name {
            "color_scheme_name" => {
                let lut_name = value.as_str().unwrap();
                self.current_color_scheme = lut_name.to_string();
                // The actual LUT update is handled by the color scheme manager
            }
            "color_scheme_reversed" => {
                self.color_scheme_reversed = value.as_bool().unwrap();
            }
            _ => return Err(format!("Unknown state: {}", state_name).into()),
        }
        Ok(())
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::json!({
            "time": self.time,
            "width": self.width,
            "height": self.height,
            "color_scheme_name": self.current_color_scheme,
            "color_scheme_reversed": self.color_scheme_reversed
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
