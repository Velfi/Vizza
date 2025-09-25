use crate::error::{SimulationError, SimulationResult};
use crate::simulations::gray_scott::state::{MaskPattern, MaskTarget};
use crate::simulations::shared::ImageFitMode;
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::Settings;
use super::shaders::noise_seed::NoiseSeedCompute;
use super::shaders::paint_compute::PaintCompute;
use super::shaders::{BACKGROUND_RENDER_SHADER, REACTION_DIFFUSION_SHADER, RENDER_INFINITE_SHADER};
use super::state::State;
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::coordinates::TextureCoords;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::ping_pong_textures::PingPongTextures;
use crate::simulations::shared::{
    BindGroupBuilder, CommonBindGroupLayouts, RenderPipelineBuilder, ShaderManager,
};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SimulationParams {
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub delta_u: f32,
    pub delta_v: f32,

    pub timestep: f32,
    pub width: u32,
    pub height: u32,

    // Mask system
    pub mask_pattern: u32,
    pub mask_target: u32,
    pub mask_strength: f32,
    pub mask_mirror_horizontal: u32,
    pub mask_mirror_vertical: u32,
    pub mask_invert_tone: u32,

    // Adaptive timestep parameters
    pub max_timestep: f32,
    pub stability_factor: f32,
    pub enable_adaptive_timestep: u32,
}

// Uniform used by the render shader (matches simulations/shared/infinite_render.wgsl SimulationParams)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RenderSimulationParams {
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub delta_u: f32,
    pub delta_v: f32,
    pub timestep: f32,
    pub width: u32,
    pub height: u32,

    // Mask system
    pub mask_pattern: u32,
    pub mask_target: u32,
    pub mask_strength: f32,
    pub mask_mirror_horizontal: u32,
    pub mask_mirror_vertical: u32,
    pub mask_invert_tone: u32,

    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_size: f32,
    pub cursor_strength: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct BackgroundParams {
    pub background_type: u32, // 0 = black, 1 = white, 2 = gradient
    pub gradient_enabled: u32,
    pub gradient_type: u32,
    pub gradient_strength: f32,
    pub gradient_center_x: f32,
    pub gradient_center_y: f32,
    pub gradient_size: f32,
    pub gradient_angle: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UVPair {
    u: f32,
    v: f32,
    _pad1: f32,
    _pad2: f32,
}

#[derive(Debug)]
pub struct PostProcessingState {
    pub blur_filter: BlurFilterState,
}

#[derive(Debug)]
pub struct BlurFilterState {
    pub enabled: bool,
    pub radius: f32,
    pub sigma: f32,
}

#[derive(Debug)]
pub struct GrayScottModel {
    // Presentation
    surface_config: SurfaceConfiguration,
    pub camera: Camera,
    render_infinite_pipeline: wgpu::RenderPipeline,
    background_render_pipeline: wgpu::RenderPipeline,
    render_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    // Color scheme and render params
    lut_buffer: wgpu::Buffer,
    background_color_buffer: wgpu::Buffer,
    texture_render_params_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    device: Arc<Device>,
    queue: Arc<Queue>,
    pub settings: Settings,
    pub state: State,
    pub width: u32,
    pub height: u32,
    simulation_textures: PingPongTextures,
    params_buffer: wgpu::Buffer,
    // Separate uniform for render shader (size must match WGSL SimulationParams in infinite_render.wgsl)
    render_params_buffer: wgpu::Buffer,
    bind_groups: [wgpu::BindGroup; 2], // Double buffering
    compute_pipeline: wgpu::ComputePipeline,
    noise_seed_compute: NoiseSeedCompute,
    paint_compute: PaintCompute,
    last_frame_time: std::time::Instant,

    // Background parameters
    background_bind_group: wgpu::BindGroup,

    // Post processing state
    pub post_processing_state: PostProcessingState,
    // Mask image buffer and state
    mask_image_buffer: Option<wgpu::Buffer>,
    mask_image_original: Option<image::DynamicImage>,

    // Webcam capture for live mask
    pub webcam_capture: crate::simulations::shared::WebcamCapture,
}

impl GrayScottModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        width: u32,
        height: u32,
        settings: Settings,
        state: State,
        color_scheme_manager: &crate::simulations::shared::ColorSchemeManager,
        app_settings: &crate::commands::app_settings::AppSettings,
    ) -> SimulationResult<Self> {
        let vec_capacity = (width * height) as usize;
        let mut uvs: Vec<UVPair> = std::iter::repeat_n(
            UVPair {
                u: 1.0,
                v: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            },
            vec_capacity,
        )
        .collect();

        // Add some initial perturbations to start the reaction-diffusion process
        let center_x = width as i32 / 2;
        let center_y = height as i32 / 2;
        let radius = 10;

        for y in -radius..=radius {
            for x in -radius..=radius {
                let nx = center_x + x;
                let ny = center_y + y;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let distance = ((x * x + y * y) as f32).sqrt() / radius as f32;
                    let factor = if distance < 1.0 {
                        (1.0 - distance * distance).powf(2.0)
                    } else {
                        0.0
                    };

                    let index = (ny * width as i32 + nx) as usize;
                    uvs[index] = UVPair {
                        u: 0.5,
                        v: 0.99 * factor,
                        _pad1: 0.0,
                        _pad2: 0.0,
                    };
                }
            }
        }

        // Create ping-pong textures for simulation data
        let simulation_textures = PingPongTextures::new(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba16Float,
            "Gray-Scott UVs",
        );

        // Write initial UVs data to both textures
        for texture in simulation_textures.textures() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&uvs),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 16), // 4 f32 values * 4 bytes each
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let params = SimulationParams {
            feed_rate: settings.feed_rate,
            kill_rate: settings.kill_rate,
            delta_u: settings.diffusion_rate_u,
            delta_v: settings.diffusion_rate_v,
            timestep: settings.timestep,
            width,
            height,

            // Mask system
            mask_pattern: state.mask_pattern as u32,
            mask_target: state.mask_target as u32,
            mask_strength: state.mask_strength,
            mask_mirror_horizontal: state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: state.mask_mirror_vertical as u32,
            mask_invert_tone: state.mask_invert_tone as u32,

            // Adaptive timestep parameters
            max_timestep: settings.max_timestep,
            stability_factor: settings.stability_factor,
            enable_adaptive_timestep: settings.enable_adaptive_timestep as u32,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create render params buffer (matches the render shader's expected struct layout)
        let render_params = RenderSimulationParams {
            feed_rate: settings.feed_rate,
            kill_rate: settings.kill_rate,
            delta_u: settings.diffusion_rate_u,
            delta_v: settings.diffusion_rate_v,
            timestep: settings.timestep,
            width,
            height,

            // Mask system
            mask_pattern: state.mask_pattern as u32,
            mask_target: state.mask_target as u32,
            mask_strength: state.mask_strength,
            mask_mirror_horizontal: state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: state.mask_mirror_vertical as u32,
            mask_invert_tone: state.mask_invert_tone as u32,

            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: state.cursor_size,
            cursor_strength: state.cursor_strength,
        };
        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GrayScott Render Params Buffer"),
            contents: bytemuck::cast_slice(&[render_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout and pipeline
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[
                resource_helpers::storage_texture_entry(
                    0,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::ReadOnly,
                    wgpu::TextureFormat::Rgba16Float,
                ),
                resource_helpers::storage_texture_entry(
                    1,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba16Float,
                ),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true), // Optional gradient map buffer
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(REACTION_DIFFUSION_SHADER.into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Create gradient buffer (matches simulation size)
        let gradient_buffer_size =
            (width as usize * height as usize * std::mem::size_of::<f32>()) as u64;
        let gradient_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrayScott Gradient Buffer"),
            size: gradient_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create bind groups for both textures (input/output swapped)
        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 0"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &simulation_textures.views()[0]), // input
                    resource_helpers::texture_view_entry(1, &simulation_textures.views()[1]), // output
                    resource_helpers::buffer_entry(2, &params_buffer),
                    resource_helpers::buffer_entry(3, &gradient_buffer),
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 1"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &simulation_textures.views()[1]), // input
                    resource_helpers::texture_view_entry(1, &simulation_textures.views()[0]), // output
                    resource_helpers::buffer_entry(2, &params_buffer),
                    resource_helpers::buffer_entry(3, &gradient_buffer),
                ],
            }),
        ];

        // Init color scheme buffer and render params similar to other sims
        let mut shader_manager = ShaderManager::new();
        let common_layouts = CommonBindGroupLayouts::new(device);

        // Color scheme buffer (u32 planar)
        let lut_data = color_scheme_manager.get_default();
        let lut_u32 = lut_data.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GrayScott Color Scheme Buffer"),
            contents: bytemuck::cast_slice(&lut_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let background_color_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GrayScott Background Color Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let filtering_mode = match app_settings.texture_filtering {
            crate::commands::app_settings::TextureFiltering::Nearest => 0u32,
            crate::commands::app_settings::TextureFiltering::Linear => 1u32,
            crate::commands::app_settings::TextureFiltering::Lanczos => 2u32,
        };
        let texture_render_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GrayScott Texture Render Params"),
                contents: bytemuck::cast_slice(&[filtering_mode, 0u32, 0u32, 0u32]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("GrayScott Simulation Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Bind group layouts matching shared infinite_render.wgsl
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GrayScott Render Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::texture_entry(
                        3,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        4,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    resource_helpers::storage_buffer_entry(5, wgpu::ShaderStages::FRAGMENT, true),
                    resource_helpers::uniform_buffer_entry(6, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::uniform_buffer_entry(7, wgpu::ShaderStages::FRAGMENT),
                ],
            });
        let camera_bind_group_layout = common_layouts.camera.clone();
        let background_bind_group_layout = common_layouts.uniform_buffer.clone();

        // Camera
        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )?;

        // Shaders
        let shader_infinite = shader_manager.load_shader(
            device,
            "gray_scott_render_infinite",
            RENDER_INFINITE_SHADER,
        );
        let background_shader =
            shader_manager.load_shader(device, "gray_scott_background", BACKGROUND_RENDER_SHADER);

        // Pipelines
        let render_infinite_pipeline = RenderPipelineBuilder::new(Arc::clone(device))
            .with_shader(shader_infinite)
            .with_bind_group_layouts(vec![
                render_bind_group_layout.clone(),
                camera_bind_group_layout.clone(),
            ])
            .with_fragment_targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .with_label("GrayScott Render Infinite".to_string())
            .build();

        let background_render_pipeline = RenderPipelineBuilder::new(Arc::clone(device))
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
            .with_label("GrayScott Background Render".to_string())
            .build();
        let noise_seed_compute = NoiseSeedCompute::new(device);

        // Create background parameters
        let background_params = BackgroundParams {
            background_type: 0,  // Black background by default
            gradient_enabled: 0, // No gradient by default
            gradient_type: 0,
            gradient_strength: 1.0,
            gradient_center_x: 0.0,
            gradient_center_y: 0.0,
            gradient_size: 1.0,
            gradient_angle: 0.0,
        };
        let background_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Background Params Buffer"),
                contents: bytemuck::bytes_of(&background_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create background bind group
        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Bind Group"),
            layout: &background_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, &background_params_buffer)],
        });

        // Apply initial color scheme before creating simulation
        let current_color_scheme = "MATPLOTLIB_prism";
        let color_scheme_reversed = false;
        if let Ok(lut_data) = color_scheme_manager.get(&current_color_scheme) {
            let lut_u32 = if color_scheme_reversed {
                let mut data = lut_data.clone();
                data.reverse();
                data.to_u32_buffer()
            } else {
                lut_data.to_u32_buffer()
            };
            queue.write_buffer(&lut_buffer, 0, bytemuck::cast_slice(&lut_u32));
        }

        // Update state with initial values
        let mut state = state;
        state.current_color_scheme = current_color_scheme.to_string();
        state.color_scheme_reversed = color_scheme_reversed;

        // Initialize simulation
        let simulation = Self {
            surface_config: surface_config.clone(),
            camera,
            render_infinite_pipeline,
            background_render_pipeline,
            render_bind_group_layout,
            camera_bind_group_layout,
            lut_buffer,
            background_color_buffer,
            texture_render_params_buffer,
            sampler,
            device: Arc::clone(device),
            queue: Arc::clone(queue),
            settings,
            width,
            height,
            simulation_textures,
            params_buffer,
            render_params_buffer,
            bind_groups,
            compute_pipeline,
            noise_seed_compute,
            paint_compute: PaintCompute::new(device),
            last_frame_time: std::time::Instant::now(),
            state,
            background_bind_group,
            post_processing_state: PostProcessingState {
                blur_filter: BlurFilterState {
                    enabled: false,
                    radius: 1.0,
                    sigma: 1.0,
                },
            },
            mask_image_buffer: Some(gradient_buffer),
            mask_image_original: None,
            webcam_capture: crate::simulations::shared::WebcamCapture::new(),
        };

        Ok(simulation)
    }

    pub fn update_settings(&mut self, new_settings: Settings, queue: &Arc<Queue>) {
        self.settings = new_settings;

        // Update params buffer
        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            max_timestep: self.settings.max_timestep,
            stability_factor: self.settings.stability_factor,
            enable_adaptive_timestep: self.settings.enable_adaptive_timestep as u32,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        // Update render params buffer as well to keep them in sync
        let render_params = RenderSimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            // New mask system
            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: self.state.cursor_size,
            cursor_strength: self.state.cursor_strength,
        };
        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );
    }

    /// Update simulation parameters when state changes
    pub fn update_simulation_params(&mut self, queue: &Arc<Queue>) -> SimulationResult<()> {
        // Update params buffer
        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            max_timestep: self.settings.max_timestep,
            stability_factor: self.settings.stability_factor,
            enable_adaptive_timestep: self.settings.enable_adaptive_timestep as u32,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Update render params buffer as well to keep them in sync
        let render_params = RenderSimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: self.state.cursor_size,
            cursor_strength: self.state.cursor_strength,
        };

        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );

        Ok(())
    }

    /// Update app settings (like texture filtering mode)
    pub fn update_app_settings(
        &mut self,
        app_settings: &crate::commands::AppSettings,
        queue: &Arc<Queue>,
    ) {
        let filtering_mode = match app_settings.texture_filtering {
            crate::commands::app_settings::TextureFiltering::Nearest => 0u32,
            crate::commands::app_settings::TextureFiltering::Linear => 1u32,
            crate::commands::app_settings::TextureFiltering::Lanczos => 2u32,
        };

        // Update the texture render params buffer with the new filtering mode
        queue.write_buffer(
            &self.texture_render_params_buffer,
            0,
            bytemuck::cast_slice(&[filtering_mode, 0u32, 0u32, 0u32]),
        );
    }

    pub fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        // Update renderer first
        // Update surface config and camera viewport
        self.surface_config = new_config.clone();
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        // Use full surface resolution; ensure a minimum size
        let new_sim_width = new_config.width.max(256);
        let new_sim_height = new_config.height.max(256);

        // Only recreate buffers if dimensions actually changed
        if new_sim_width != self.width || new_sim_height != self.height {
            tracing::info!(
                "Gray-Scott simulation resolution changed from {}x{} to {}x{}",
                self.width,
                self.height,
                new_sim_width,
                new_sim_height
            );

            // Update dimensions
            self.width = new_sim_width;
            self.height = new_sim_height;

            // Recreate simulation buffers with new dimensions
            Self::recreate_simulation_buffers(self, device, queue)?;
        }

        Ok(())
    }

    /// Recreate simulation textures with new dimensions
    pub fn recreate_simulation_buffers(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Create new ping-pong textures with new dimensions
        let new_simulation_textures = PingPongTextures::new(
            device,
            self.width,
            self.height,
            wgpu::TextureFormat::Rgba16Float,
            "Gray-Scott UVs (Resized)",
        );

        // Initialize with default UV values
        let vec_capacity = (self.width * self.height) as usize;
        let uvs: Vec<UVPair> = std::iter::repeat_n(
            UVPair {
                u: 1.0,
                v: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            },
            vec_capacity,
        )
        .collect();

        // Write initial data to both textures
        for texture in new_simulation_textures.textures() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&uvs),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * 16), // 4 f32 values * 4 bytes each
                    rows_per_image: Some(self.height),
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        // Create new gradient buffer
        let gradient_buffer_size =
            (self.width as usize * self.height as usize * std::mem::size_of::<f32>()) as u64;
        let new_gradient_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrayScott Gradient Buffer (Resized)"),
            size: gradient_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Update params buffer with new dimensions
        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            // New mask system
            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            // Adaptive timestep parameters
            max_timestep: self.settings.max_timestep,
            stability_factor: self.settings.stability_factor,
            enable_adaptive_timestep: self.settings.enable_adaptive_timestep as u32,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        // Also update render params dimensions
        let render_params = RenderSimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            // New mask system
            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: self.state.cursor_size,
            cursor_strength: self.state.cursor_strength,
        };
        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );

        // Create new bind groups with new textures
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout (Resized)"),
            entries: &[
                resource_helpers::storage_texture_entry(
                    0,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::ReadOnly,
                    wgpu::TextureFormat::Rgba16Float,
                ),
                resource_helpers::storage_texture_entry(
                    1,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba16Float,
                ),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true),
            ],
        });

        let new_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 0 (Resized)"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &new_simulation_textures.views()[0]), // input
                    resource_helpers::texture_view_entry(1, &new_simulation_textures.views()[1]), // output
                    resource_helpers::buffer_entry(2, &self.params_buffer),
                    resource_helpers::buffer_entry(3, &new_gradient_buffer),
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 1 (Resized)"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &new_simulation_textures.views()[1]), // input
                    resource_helpers::texture_view_entry(1, &new_simulation_textures.views()[0]), // output
                    resource_helpers::buffer_entry(2, &self.params_buffer),
                    resource_helpers::buffer_entry(3, &new_gradient_buffer),
                ],
            }),
        ];

        // Replace old textures with new ones
        self.simulation_textures = new_simulation_textures;
        self.mask_image_buffer = Some(new_gradient_buffer);
        self.bind_groups = new_bind_groups;

        // If we have a gradient image, reprocess it for the new resolution
        if self.mask_image_original.is_some() {
            if let Err(e) = self.reprocess_nutrient_image_with_current_fit_mode(queue) {
                tracing::warn!("Failed to reprocess gradient image after resize: {}", e);
            }
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        let vec_capacity = (self.width * self.height) as usize;
        let uvs: Vec<UVPair> = std::iter::repeat_n(
            UVPair {
                u: 1.0,
                v: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            },
            vec_capacity,
        )
        .collect();

        for texture in self.simulation_textures.textures() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&uvs),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * 16), // 4 f32 values * 4 bytes each
                    rows_per_image: Some(self.height),
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn seed_random_noise(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Generate a random seed for this noise generation
        let seed = rand::random::<u32>();

        // Use GPU-based noise seeding for both textures
        for texture in self.simulation_textures.textures() {
            self.noise_seed_compute.seed_noise(
                device,
                queue,
                texture,
                self.width,
                self.height,
                seed,
                1.0, // Full noise strength
            )?;
        }

        Ok(())
    }

    /// Load an external image, convert to grayscale in [0,1], fit to sim size, and upload
    pub fn load_nutrient_image(
        &mut self,
        queue: &Arc<Queue>,
        image_path: &str,
    ) -> SimulationResult<()> {
        let img = image::open(image_path).map_err(|e| {
            SimulationError::InvalidParameter(format!("Failed to open image: {}", e))
        })?;

        // Store the original image and reprocess with current fit mode
        self.mask_image_original = Some(img);
        self.reprocess_nutrient_image_with_current_fit_mode(queue)?;

        tracing::info!("Gray-Scott nutrient image loaded successfully");
        Ok(())
    }

    /// Reprocess the loaded image with the current fit mode and strength settings
    pub fn reprocess_nutrient_image_with_current_fit_mode(
        &mut self,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if let Some(original_img) = &self.mask_image_original {
            let target_w = self.width as u32;
            let target_h = self.height as u32;

            // Convert to grayscale
            let gray = original_img.to_luma8();

            // Tone inversion is handled in the shader

            // Apply fit mode
            let resized = match self.state.mask_image_fit_mode {
                ImageFitMode::Stretch => image::imageops::resize(
                    &gray,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                ),
                ImageFitMode::Center => {
                    // Center the image without stretching
                    let mut buffer = image::ImageBuffer::new(target_w, target_h);
                    let img_w = gray.width();
                    let img_h = gray.height();

                    let start_x = if img_w > target_w {
                        0
                    } else {
                        (target_w - img_w) / 2
                    };
                    let start_y = if img_h > target_h {
                        0
                    } else {
                        (target_h - img_h) / 2
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            let src_x = if img_w > target_w {
                                x * img_w / target_w
                            } else {
                                x.saturating_sub(start_x)
                            };
                            let src_y = if img_h > target_h {
                                y * img_h / target_h
                            } else {
                                y.saturating_sub(start_y)
                            };

                            if src_x < img_w && src_y < img_h {
                                buffer.put_pixel(x, y, *gray.get_pixel(src_x, src_y));
                            } else {
                                buffer.put_pixel(x, y, image::Luma([0]));
                            }
                        }
                    }
                    buffer
                }
                ImageFitMode::FitH => {
                    // Fit horizontally, maintain aspect ratio
                    let aspect_ratio = gray.height() as f32 / gray.width() as f32;
                    let new_height = (target_w as f32 * aspect_ratio) as u32;
                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        new_height,
                        image::imageops::FilterType::Lanczos3,
                    );

                    // Center vertically
                    let mut buffer = image::ImageBuffer::new(target_w, target_h);
                    let start_y = if new_height > target_h {
                        0
                    } else {
                        (target_h - new_height) / 2
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            if y >= start_y && y < start_y + new_height {
                                buffer.put_pixel(x, y, *resized.get_pixel(x, y - start_y));
                            } else {
                                buffer.put_pixel(x, y, image::Luma([0]));
                            }
                        }
                    }
                    buffer
                }
                ImageFitMode::FitV => {
                    // Fit vertically, maintain aspect ratio
                    let aspect_ratio = gray.width() as f32 / gray.height() as f32;
                    let new_width = (target_h as f32 * aspect_ratio) as u32;
                    let resized = image::imageops::resize(
                        &gray,
                        new_width,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );

                    // Center horizontally
                    let mut buffer = image::ImageBuffer::new(target_w, target_h);
                    let start_x = if new_width > target_w {
                        0
                    } else {
                        (target_w - new_width) / 2
                    };

                    for y in 0..target_h {
                        for x in 0..target_w {
                            if x >= start_x && x < start_x + new_width {
                                buffer.put_pixel(x, y, *resized.get_pixel(x - start_x, y));
                            } else {
                                buffer.put_pixel(x, y, image::Luma([0]));
                            }
                        }
                    }
                    buffer
                }
            };

            // Convert to f32 buffer
            let mut buffer = vec![0.0f32; (target_w * target_h) as usize];
            for y in 0..target_h {
                for x in 0..target_w {
                    let p = resized.get_pixel(x, y)[0] as f32 / 255.0;
                    buffer[(y * target_w + x) as usize] = p;
                }
            }

            // Note: Mirror and reversal controls are now handled in the shader
            // to avoid double-application of transformations

            self.state.mask_image_base = Some(buffer.clone());
            self.state.mask_image_raw = Some(buffer.clone());

            // Upload to GPU
            if let Some(grad_buf) = &self.mask_image_buffer {
                queue.write_buffer(grad_buf, 0, bytemuck::cast_slice::<f32, u8>(&buffer));
                tracing::info!("Reprocessed gradient image uploaded to GPU buffer");
            } else {
                tracing::error!("No gradient buffer available for reprocessing!");
                return Err(SimulationError::InvalidParameter(
                    "No gradient buffer available".to_string(),
                ));
            }

            self.state.mask_image_needs_upload = false;
            tracing::info!("Gray-Scott nutrient image reprocessed successfully");
        }
        Ok(())
    }

    pub(crate) fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "feed_rate" => {
                if let Some(v) = value.as_f64() {
                    self.settings.feed_rate = v as f32;
                }
            }
            "kill_rate" => {
                if let Some(v) = value.as_f64() {
                    self.settings.kill_rate = v as f32;
                }
            }
            "diffusion_rate_u" => {
                if let Some(v) = value.as_f64() {
                    self.settings.diffusion_rate_u = v as f32;
                }
            }
            "diffusion_rate_v" => {
                if let Some(v) = value.as_f64() {
                    self.settings.diffusion_rate_v = v as f32;
                }
            }
            "timestep" => {
                if let Some(v) = value.as_f64() {
                    self.settings.timestep = v as f32;
                }
            }
            "mask_pattern" => {
                if let Some(v) = value.as_str() {
                    if let Some(parsed) = MaskPattern::from_str(v) {
                        self.state.mask_pattern = parsed;
                    }
                }
            }
            "mask_target" => {
                if let Some(v) = value.as_str() {
                    if let Some(parsed) = MaskTarget::from_str(v) {
                        self.state.mask_target = parsed;
                    }
                }
            }
            "mask_strength" => {
                if let Some(v) = value.as_f64() {
                    self.state.mask_strength = v as f32;
                }
            }
            "mask_reversed" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_reversed = v;
                    // No need to reprocess image - shader handles reversal
                }
            }
            "image_fit_mode" => {
                if let Some(v) = value.as_str() {
                    if let Some(mode) = ImageFitMode::from_str(v) {
                        self.state.mask_image_fit_mode = mode;
                    }
                    // Reprocess the image if one is loaded
                    if self.mask_image_original.is_some() {
                        if let Err(e) = self.reprocess_nutrient_image_with_current_fit_mode(queue) {
                            tracing::error!("Failed to reprocess gradient image: {}", e);
                        }
                    }
                }
            }
            "mask_mirror_horizontal" => {
                if let Some(v) = value.as_bool() {
                    self.state.mask_mirror_horizontal = v;
                    // No need to reprocess image - shader handles mirroring
                }
            }
            "cursor_size" => {
                if let Some(v) = value.as_f64() {
                    self.state.cursor_size = v as f32;
                }
            }
            "cursor_strength" => {
                if let Some(v) = value.as_f64() {
                    self.state.cursor_strength = v as f32;
                }
            }
            _ => {}
        }

        // Update params buffer
        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            max_timestep: self.settings.max_timestep,
            stability_factor: self.settings.stability_factor,
            enable_adaptive_timestep: self.settings.enable_adaptive_timestep as u32,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        Ok(())
    }

    pub(crate) fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        _delta_time: f32,
    ) -> SimulationResult<()> {
        if self.webcam_capture.is_active {
            if let Err(e) = self.update_mask_from_webcam(queue) {
                tracing::warn!("Gray-Scott webcam gradient update failed: {}", e);
            }
        }
        // Calculate delta time
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // Update camera for smooth movement
        self.camera.update(delta_time);

        // Run compute pass
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Gray Scott Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gray Scott Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                self.simulation_textures
                    .get_bind_group(&self.bind_groups[0], &self.bind_groups[1]),
                &[],
            );
            compute_pass.dispatch_workgroups(self.width, self.height, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Swap textures for next frame
        self.simulation_textures.swap();

        // Render background and infinite tiling
        self.camera.upload_to_gpu(&self.queue);
        let camera_bind_group = BindGroupBuilder::new(&self.device, &self.camera_bind_group_layout)
            .add_buffer(0, self.camera.buffer())
            .with_label("GrayScott Camera Bind Group".to_string())
            .build();

        let sim_texture = self.simulation_textures.current_texture();
        let texture_view = sim_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let render_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GrayScott Render Bind Group"),
            layout: &self.render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(2, &self.background_color_buffer),
                resource_helpers::texture_view_entry(3, &texture_view),
                resource_helpers::sampler_bind_entry(4, &self.sampler),
                resource_helpers::buffer_entry(5, &self.lut_buffer),
                resource_helpers::buffer_entry(6, &self.params_buffer),
                resource_helpers::buffer_entry(7, &self.render_params_buffer),
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Gray Scott Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gray Scott Render Pass"),
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

            // Background
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1);

            // Infinite tiling
            let tile_count = {
                // match shader logic: see infinite_render.wgsl calculate_tile_count
                let zoom = self.camera.zoom;
                let visible_world_size = 2.0 / zoom;
                let tiles_needed = (visible_world_size / 2.0).ceil() as u32 + 6;
                let min_tiles = if zoom < 0.1 { 7 } else { 5 };
                tiles_needed.max(min_tiles).min(1024)
            };
            let total_instances = tile_count * tile_count;
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &render_bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn start_webcam_capture(&mut self, device_index: i32) -> SimulationResult<()> {
        if self.mask_image_buffer.is_none() {
            return Err(SimulationError::InvalidParameter(
                "Gray-Scott has no gradient buffer".to_string(),
            ));
        }
        self.webcam_capture
            .set_target_dimensions(self.width, self.height);
        self.webcam_capture.start_capture(device_index)
    }

    pub fn stop_webcam_capture(&mut self) {
        self.webcam_capture.stop_capture();
    }

    pub fn update_mask_from_webcam(&mut self, queue: &Arc<Queue>) -> SimulationResult<()> {
        if let Some(frame_data) = self.webcam_capture.get_latest_frame_data() {
            let buffer = self.webcam_capture.frame_data_to_gradient_buffer(
                &frame_data,
                self.width,
                self.height,
            )?;
            // Note: Mirroring (horizontal/vertical) and reversal are handled in the shader.
            // Keep the uploaded buffer as-is to avoid double application of transforms.
            let processed = buffer;
            if let Some(grad_buf) = &self.mask_image_buffer {
                queue.write_buffer(grad_buf, 0, bytemuck::cast_slice::<f32, u8>(&processed));
            }
            self.state.mask_image_raw = Some(processed);
            self.state.mask_image_needs_upload = false;
        }
        Ok(())
    }

    pub fn update_cursor_position(
        &mut self,
        x: f32,
        y: f32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // x and y are world coordinates in [-1,1]; convert to texture space [0,1] for the render params
        let texture_x = (x + 1.0) * 0.5;
        let texture_y = (y + 1.0) * 0.5;

        let render_params = RenderSimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,

            // New mask system
            mask_pattern: self.state.mask_pattern as u32,
            mask_target: self.state.mask_target as u32,
            mask_strength: self.state.mask_strength,
            mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
            mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
            mask_invert_tone: self.state.mask_invert_tone as u32,

            cursor_x: texture_x,
            cursor_y: texture_y,
            cursor_size: self.state.cursor_size,
            cursor_strength: self.state.cursor_strength,
        };
        queue.write_buffer(
            &self.render_params_buffer,
            0,
            bytemuck::cast_slice(&[render_params]),
        );

        Ok(())
    }

    pub fn handle_mouse_interaction(
        &mut self,
        texture_x: f32,
        texture_y: f32,
        mouse_button: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // texture_x and texture_y are in [0,1] range
        self.update_cursor_position(texture_x, texture_y, queue)?;

        let texture_coords = TextureCoords::new(texture_x, texture_y);

        // Debug output
        tracing::trace!(
            "Gray-Scott handle_mouse_interaction: texture=({:.3}, {:.3}), button={}, valid={}",
            texture_x,
            texture_y,
            mouse_button,
            texture_coords.is_valid()
        );

        // Check if coordinates are within valid texture bounds
        if !texture_coords.is_valid() {
            tracing::trace!("Mouse interaction outside simulation bounds, ignoring");
            return Ok(()); // Outside simulation bounds
        }

        // Use GPU-based painting
        self.paint_compute.paint(
            device,
            queue,
            self.simulation_textures.current_texture(),
            texture_x,
            texture_y,
            self.state.cursor_size,
            self.state.cursor_strength,
            mouse_button,
            self.width,
            self.height,
        )?;

        Ok(())
    }

    fn handle_mouse_release(&mut self, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // For Gray-Scott, mouse release doesn't need special handling
        // The cursor position is already updated in handle_mouse_interaction
        // and the interaction is immediate (no continuous effect)
        tracing::trace!("Gray-Scott mouse release: no special handling needed");
        Ok(())
    }

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

    pub(crate) fn toggle_gui(&mut self) -> bool {
        self.state.gui_visible = !self.state.gui_visible;
        self.state.gui_visible
    }

    pub(crate) fn is_gui_visible(&self) -> bool {
        self.state.gui_visible
    }
}

impl crate::simulations::traits::Simulation for GrayScottModel {
    fn render_frame_paused(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Calculate delta time
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // Update camera for smooth movement
        self.camera.update(delta_time);

        // Skip compute pass - just render current state
        // Render the current state - pass the current texture (which contains the latest results)
        // Render paused frame using the same pass
        self.camera.upload_to_gpu(&self.queue);
        let camera_bind_group = BindGroupBuilder::new(&self.device, &self.camera_bind_group_layout)
            .add_buffer(0, self.camera.buffer())
            .with_label("GrayScott Camera Bind Group".to_string())
            .build();

        let sim_texture = self.simulation_textures.current_texture();
        let texture_view = sim_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let render_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GrayScott Render Bind Group"),
            layout: &self.render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(2, &self.background_color_buffer),
                resource_helpers::texture_view_entry(3, &texture_view),
                resource_helpers::sampler_bind_entry(4, &self.sampler),
                resource_helpers::buffer_entry(5, &self.lut_buffer),
                resource_helpers::buffer_entry(6, &self.params_buffer),
                resource_helpers::buffer_entry(7, &self.render_params_buffer),
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Gray Scott Render Encoder (Paused)"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gray Scott Render Pass (Paused)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1);

            let tile_count = {
                let zoom = self.camera.zoom;
                let visible_world_size = 2.0 / zoom;
                let tiles_needed = (visible_world_size / 2.0).ceil() as u32 + 6;
                let min_tiles = if zoom < 0.1 { 7 } else { 5 };
                tiles_needed.max(min_tiles).min(1024)
            };
            let total_instances = tile_count * tile_count;
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &render_bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
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
        GrayScottModel::resize(self, device, queue, new_config)
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
            "current_color_scheme" => {
                if let Some(lut_name) = value.as_str() {
                    self.state.current_color_scheme = lut_name.to_string();
                    let color_scheme_manager =
                        crate::simulations::shared::ColorSchemeManager::new();
                    let mut lut_data = color_scheme_manager
                        .get(&self.state.current_color_scheme)
                        .unwrap_or_else(|_| color_scheme_manager.get_default());

                    // Apply reversal if needed
                    if self.state.color_scheme_reversed {
                        lut_data.reverse();
                    }

                    let lut_u32 = lut_data.to_u32_buffer();
                    queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_u32));
                }
            }
            "color_scheme_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    self.state.color_scheme_reversed = reversed;
                    let color_scheme_manager =
                        crate::simulations::shared::ColorSchemeManager::new();
                    let mut lut_data = color_scheme_manager
                        .get(&self.state.current_color_scheme)
                        .unwrap_or_else(|_| color_scheme_manager.get_default());

                    // Apply reversal if needed
                    if self.state.color_scheme_reversed {
                        lut_data.reverse();
                    }

                    let lut_u32 = lut_data.to_u32_buffer();
                    queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_u32));
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.state.cursor_size = size as f32;
                    // Keep render params buffer in sync
                    let render_params = RenderSimulationParams {
                        feed_rate: self.settings.feed_rate,
                        kill_rate: self.settings.kill_rate,
                        delta_u: self.settings.diffusion_rate_u,
                        delta_v: self.settings.diffusion_rate_v,
                        timestep: self.settings.timestep,
                        width: self.width,
                        height: self.height,

                        // Mask system
                        mask_pattern: self.state.mask_pattern as u32,
                        mask_target: self.state.mask_target as u32,
                        mask_strength: self.state.mask_strength,
                        mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
                        mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
                        mask_invert_tone: self.state.mask_invert_tone as u32,

                        cursor_x: 0.0,
                        cursor_y: 0.0,
                        cursor_size: self.state.cursor_size,
                        cursor_strength: self.state.cursor_strength,
                    };
                    queue.write_buffer(
                        &self.render_params_buffer,
                        0,
                        bytemuck::cast_slice(&[render_params]),
                    );
                }
            }
            "cursor_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.cursor_strength = strength as f32;
                    // Keep render params buffer in sync
                    let render_params = RenderSimulationParams {
                        feed_rate: self.settings.feed_rate,
                        kill_rate: self.settings.kill_rate,
                        delta_u: self.settings.diffusion_rate_u,
                        delta_v: self.settings.diffusion_rate_v,
                        timestep: self.settings.timestep,
                        width: self.width,
                        height: self.height,

                        // New mask system
                        mask_pattern: self.state.mask_pattern as u32,
                        mask_target: self.state.mask_target as u32,
                        mask_strength: self.state.mask_strength,
                        mask_mirror_horizontal: self.state.mask_mirror_horizontal as u32,
                        mask_mirror_vertical: self.state.mask_mirror_vertical as u32,
                        mask_invert_tone: self.state.mask_invert_tone as u32,

                        cursor_x: 0.0,
                        cursor_y: 0.0,
                        cursor_size: self.state.cursor_size,
                        cursor_strength: self.state.cursor_strength,
                    };
                    queue.write_buffer(
                        &self.render_params_buffer,
                        0,
                        bytemuck::cast_slice(&[render_params]),
                    );
                }
            }
            "mask_pattern" => {
                if let Some(pattern_str) = value.as_str() {
                    if let Some(pattern) = MaskPattern::from_str(pattern_str) {
                        self.state.mask_pattern = pattern;
                        self.update_simulation_params(queue)?;
                    } else {
                        tracing::warn!("Unknown mask pattern: {}", pattern_str);
                    }
                }
            }
            "mask_target" => {
                if let Some(target_str) = value.as_str() {
                    if let Some(target) = MaskTarget::from_str(target_str) {
                        self.state.mask_target = target;
                        self.update_simulation_params(queue)?;
                    } else {
                        tracing::warn!("Unknown mask target: {}", target_str);
                    }
                }
            }
            "mask_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.mask_strength = strength as f32;
                    self.update_simulation_params(queue)?;
                }
            }
            "mask_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    self.state.mask_reversed = reversed;
                    self.update_simulation_params(queue)?;
                }
            }
            "mask_image_fit_mode" => {
                if let Some(fit_mode_str) = value.as_str() {
                    if let Some(fit_mode) = ImageFitMode::from_str(fit_mode_str) {
                        self.state.mask_image_fit_mode = fit_mode;
                        // Reprocess the image with the new fit mode if we have one loaded
                        if self.mask_image_original.is_some() {
                            self.reprocess_nutrient_image_with_current_fit_mode(queue)?;
                        }
                    } else {
                        tracing::warn!("Unknown fit mode: {}", fit_mode_str);
                    }
                }
            }
            "mask_mirror_horizontal" => {
                if let Some(mirror) = value.as_bool() {
                    self.state.mask_mirror_horizontal = mirror;
                    self.update_simulation_params(queue)?;
                    // No need to reprocess image - shader handles mirroring
                }
            }
            "mask_mirror_vertical" => {
                if let Some(mirror) = value.as_bool() {
                    self.state.mask_mirror_vertical = mirror;
                    self.update_simulation_params(queue)?;
                    // No need to reprocess image - shader handles mirroring
                }
            }
            "mask_invert_tone" => {
                if let Some(invert) = value.as_bool() {
                    self.state.mask_invert_tone = invert;
                    self.update_simulation_params(queue)?;
                    // No need to reprocess image - shader handles tone inversion
                }
            }
            _ => {
                tracing::warn!("Unknown state parameter for GrayScott: {}", state_name);
            }
        }
        Ok(())
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&self.settings).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::to_value(&self.state).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Convert world coordinates to texture coordinates
        let texture_x = (world_x + 1.0) * 0.5;
        let texture_y = (world_y + 1.0) * 0.5;
        GrayScottModel::handle_mouse_interaction(
            self,
            texture_x,
            texture_y,
            mouse_button,
            _device,
            queue,
        )
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        GrayScottModel::handle_mouse_release(self, queue)
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        GrayScottModel::pan_camera(self, delta_x, delta_y);
    }

    fn zoom_camera(&mut self, delta: f32) {
        GrayScottModel::zoom_camera(self, delta);
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        GrayScottModel::zoom_camera_to_cursor(self, delta, cursor_x, cursor_y);
    }

    fn reset_camera(&mut self) {
        GrayScottModel::reset_camera(self);
    }

    fn get_camera_state(&self) -> serde_json::Value {
        serde_json::json!({
            "position": [self.camera.position[0], self.camera.position[1]],
            "zoom": self.camera.zoom
        })
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let new_settings: Settings =
            serde_json::from_value(settings).map_err(SimulationError::Serialization)?;
        self.update_settings(new_settings, queue);
        Ok(())
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // This would need to be implemented with the preset manager
        // For now, we'll return an error indicating it needs to be implemented
        Err("Preset saving not yet implemented for GrayScottModel".into())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // This would need to be implemented with the preset manager
        // For now, we'll return an error indicating it needs to be implemented
        Err("Preset loading not yet implemented for GrayScottModel".into())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No-op for Gray-Scott
        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        GrayScottModel::toggle_gui(self)
    }

    fn is_gui_visible(&self) -> bool {
        GrayScottModel::is_gui_visible(self)
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

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let mut lut_data = color_scheme.clone();
        if self.state.color_scheme_reversed {
            lut_data.reverse();
        }
        let lut_u32 = lut_data.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_u32));
        Ok(())
    }
}
