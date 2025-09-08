use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::renderer::Renderer;
use super::settings::{GradientImageFitMode, NutrientPattern, Settings};
use super::shaders::REACTION_DIFFUSION_SHADER;
use super::shaders::noise_seed::NoiseSeedCompute;
use crate::simulations::shared::coordinates::TextureCoords;
use crate::simulations::shared::gpu_utils::resource_helpers;

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
    pub nutrient_pattern: u32,
    pub is_nutrient_pattern_reversed: u32,
    // Adaptive timestep parameters
    pub max_timestep: f32,
    pub stability_factor: f32,
    pub enable_adaptive_timestep: u32,
    // Dependency tracking parameters
    pub change_threshold: f32,
    pub enable_selective_updates: u32,
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
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CellState {
    uv: UVPair,
    change_magnitude: f32,
    last_update: u32,
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
    pub renderer: Renderer,
    pub settings: Settings,
    pub width: u32,
    pub height: u32,
    pub color_scheme_reversed: bool,
    pub current_color_scheme_name: String,
    uvs_buffers: [wgpu::Buffer; 2], // Double buffering
    current_buffer: usize,
    params_buffer: wgpu::Buffer,
    bind_groups: [wgpu::BindGroup; 2], // Double buffering
    compute_pipeline: wgpu::ComputePipeline,
    noise_seed_compute: NoiseSeedCompute,
    last_frame_time: std::time::Instant,
    gui_visible: bool,

    // Cursor configuration (runtime state, not saved in presets)
    pub cursor_size: f32,
    pub cursor_strength: f32,

    // Background parameters
    pub background_params_buffer: wgpu::Buffer,
    pub background_bind_group: wgpu::BindGroup,

    // Post processing state
    pub post_processing_state: PostProcessingState,
    // Gradient image buffer and state
    pub gradient_buffer: Option<wgpu::Buffer>,
    pub gradient_image_original: Option<image::DynamicImage>,
    pub gradient_image_base: Option<Vec<f32>>, // before strength mapping
    pub gradient_image_raw: Option<Vec<f32>>,  // uploaded values
    pub gradient_image_needs_upload: bool,

    // Cell states buffer (required by shader)
    pub cell_states_buffer: wgpu::Buffer,

    // Webcam capture for live gradient
    pub webcam_capture: crate::simulations::slime_mold::webcam::WebcamCapture,
}

impl GrayScottModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        width: u32,
        height: u32,
        settings: Settings,
        color_scheme_manager: &crate::simulations::shared::ColorSchemeManager,
        app_settings: &crate::commands::app_settings::AppSettings,
    ) -> SimulationResult<Self> {
        let vec_capacity = (width * height) as usize;
        let mut uvs: Vec<UVPair> =
            std::iter::repeat_n(UVPair { u: 1.0, v: 0.0 }, vec_capacity).collect();

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
                    };
                }
            }
        }

        // Create double buffers
        let uvs_buffers = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UVs Buffer 0"),
                size: (vec_capacity * std::mem::size_of::<UVPair>()) as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UVs Buffer 1"),
                size: (vec_capacity * std::mem::size_of::<UVPair>()) as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            }),
        ];

        // Write initial UVs data to both buffers
        for buffer in &uvs_buffers {
            let slice = buffer.slice(..);
            slice
                .get_mapped_range_mut()
                .copy_from_slice(bytemuck::cast_slice(&uvs));
            buffer.unmap();
        }

        let params = SimulationParams {
            feed_rate: settings.feed_rate,
            kill_rate: settings.kill_rate,
            delta_u: settings.diffusion_rate_u,
            delta_v: settings.diffusion_rate_v,
            timestep: settings.timestep,
            width,
            height,
            nutrient_pattern: settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: settings.nutrient_pattern_reversed as u32,
            // Adaptive timestep parameters
            max_timestep: 1.0,
            stability_factor: 0.5,
            enable_adaptive_timestep: 1,
            // Dependency tracking parameters
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout and pipeline
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::storage_buffer_entry(4, wgpu::ShaderStages::COMPUTE, true), // Optional gradient map buffer
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

        // Create cell states buffer (required by shader)
        let cell_states_size =
            (width as usize * height as usize * std::mem::size_of::<CellState>()) as u64;
        let cell_states_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrayScott Cell States Buffer"),
            size: cell_states_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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

        // Create bind groups for both buffers (input/output swapped)
        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 0"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::buffer_entry(0, &uvs_buffers[0]), // input
                    resource_helpers::buffer_entry(1, &uvs_buffers[1]), // output
                    resource_helpers::buffer_entry(2, &params_buffer),
                    resource_helpers::buffer_entry(3, &cell_states_buffer),
                    resource_helpers::buffer_entry(4, &gradient_buffer),
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 1"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::buffer_entry(0, &uvs_buffers[1]), // input
                    resource_helpers::buffer_entry(1, &uvs_buffers[0]), // output
                    resource_helpers::buffer_entry(2, &params_buffer),
                    resource_helpers::buffer_entry(3, &cell_states_buffer),
                    resource_helpers::buffer_entry(4, &gradient_buffer),
                ],
            }),
        ];

        let renderer = Renderer::new(
            device,
            queue,
            surface_config,
            width,
            height,
            color_scheme_manager,
            app_settings,
        )?;
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
            layout: renderer.background_bind_group_layout(),
            entries: &[resource_helpers::buffer_entry(0, &background_params_buffer)],
        });

        // Initialize LUT
        let mut simulation = Self {
            renderer,
            settings,
            width,
            height,
            current_color_scheme_name: "MATPLOTLIB_prism".to_string(),
            color_scheme_reversed: false,
            uvs_buffers,
            current_buffer: 0,
            params_buffer,
            bind_groups,
            compute_pipeline,
            noise_seed_compute,
            last_frame_time: std::time::Instant::now(),
            gui_visible: true,
            cursor_size: 40.0,
            cursor_strength: 0.5,
            background_params_buffer,
            background_bind_group,
            post_processing_state: PostProcessingState {
                blur_filter: BlurFilterState {
                    enabled: false,
                    radius: 1.0,
                    sigma: 1.0,
                },
            },
            gradient_buffer: Some(gradient_buffer),
            gradient_image_original: None,
            gradient_image_base: None,
            gradient_image_raw: None,
            gradient_image_needs_upload: false,
            cell_states_buffer,
            webcam_capture: crate::simulations::slime_mold::webcam::WebcamCapture::new(),
        };

        // Apply initial LUT
        if let Ok(mut lut_data) = color_scheme_manager.get(&simulation.current_color_scheme_name) {
            if simulation.color_scheme_reversed {
                lut_data.reverse();
            }
            simulation.renderer.update_lut(&lut_data, queue);
        }

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
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            // Adaptive timestep parameters
            max_timestep: 1.0,
            stability_factor: 0.5,
            enable_adaptive_timestep: 1,
            // Dependency tracking parameters
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        self.renderer.update_settings(&self.settings, queue);
    }

    pub fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        // Update renderer first
        self.renderer.resize(new_config)?;

        // Calculate new simulation dimensions based on resolution scale
        let new_sim_width =
            (new_config.width as f32 * self.settings.simulation_resolution_scale) as u32;
        let new_sim_height =
            (new_config.height as f32 * self.settings.simulation_resolution_scale) as u32;

        // Ensure minimum resolution
        let new_sim_width = new_sim_width.max(256);
        let new_sim_height = new_sim_height.max(256);

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

    /// Recreate simulation buffers with new dimensions
    pub fn recreate_simulation_buffers(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let vec_capacity = (self.width * self.height) as usize;

        // Create new UV buffers with new dimensions
        let new_uvs_buffers = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UVs Buffer 0 (Resized)"),
                size: (vec_capacity * std::mem::size_of::<UVPair>()) as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UVs Buffer 1 (Resized)"),
                size: (vec_capacity * std::mem::size_of::<UVPair>()) as u64,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            }),
        ];

        // Initialize with default UV values
        let uvs: Vec<UVPair> =
            std::iter::repeat_n(UVPair { u: 1.0, v: 0.0 }, vec_capacity).collect();

        // Write initial data to both buffers
        for buffer in &new_uvs_buffers {
            let slice = buffer.slice(..);
            slice
                .get_mapped_range_mut()
                .copy_from_slice(bytemuck::cast_slice(&uvs));
            buffer.unmap();
        }

        // Create new cell states buffer
        let cell_states_size =
            (self.width as usize * self.height as usize * std::mem::size_of::<CellState>()) as u64;
        let new_cell_states_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrayScott Cell States Buffer (Resized)"),
            size: cell_states_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            // Adaptive timestep parameters
            max_timestep: 1.0,
            stability_factor: 0.5,
            enable_adaptive_timestep: 1,
            // Dependency tracking parameters
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Create new bind groups with new buffers
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout (Resized)"),
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::storage_buffer_entry(4, wgpu::ShaderStages::COMPUTE, true),
            ],
        });

        let new_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 0 (Resized)"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::buffer_entry(0, &new_uvs_buffers[0]), // input
                    resource_helpers::buffer_entry(1, &new_uvs_buffers[1]), // output
                    resource_helpers::buffer_entry(2, &self.params_buffer),
                    resource_helpers::buffer_entry(3, &new_cell_states_buffer),
                    resource_helpers::buffer_entry(4, &new_gradient_buffer),
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 1 (Resized)"),
                layout: &bind_group_layout,
                entries: &[
                    resource_helpers::buffer_entry(0, &new_uvs_buffers[1]), // input
                    resource_helpers::buffer_entry(1, &new_uvs_buffers[0]), // output
                    resource_helpers::buffer_entry(2, &self.params_buffer),
                    resource_helpers::buffer_entry(3, &new_cell_states_buffer),
                    resource_helpers::buffer_entry(4, &new_gradient_buffer),
                ],
            }),
        ];

        // Replace old buffers with new ones
        self.uvs_buffers = new_uvs_buffers;
        self.cell_states_buffer = new_cell_states_buffer;
        self.gradient_buffer = Some(new_gradient_buffer);
        self.bind_groups = new_bind_groups;
        self.current_buffer = 0; // Reset to buffer 0

        // If we have a gradient image, reprocess it for the new resolution
        if self.gradient_image_original.is_some() {
            if let Err(e) = self.reprocess_nutrient_image_with_current_fit_mode(queue) {
                tracing::warn!("Failed to reprocess gradient image after resize: {}", e);
            }
        }

        tracing::info!(
            "Gray-Scott simulation buffers recreated successfully for {}x{}",
            self.width,
            self.height
        );
        Ok(())
    }

    pub fn reset(&mut self) {
        let vec_capacity = (self.width * self.height) as usize;
        let uvs: Vec<UVPair> =
            std::iter::repeat_n(UVPair { u: 1.0, v: 0.0 }, vec_capacity).collect();

        for buffer in &self.uvs_buffers {
            self.renderer
                .queue()
                .write_buffer(buffer, 0, bytemuck::cast_slice(&uvs));
        }
    }

    pub fn seed_random_noise(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Generate a random seed for this noise generation
        let seed = rand::random::<u32>();

        // Use GPU-based noise seeding for both buffers
        for buffer in &self.uvs_buffers {
            self.noise_seed_compute.seed_noise(
                device,
                queue,
                buffer,
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
        tracing::info!("Loading Gray-Scott nutrient image from: {}", image_path);

        let img = image::open(image_path).map_err(|e| {
            SimulationError::InvalidParameter(format!("Failed to open image: {}", e))
        })?;

        let target_w = self.width as u32;
        let target_h = self.height as u32;

        tracing::info!("Resizing image to {}x{}", target_w, target_h);

        // Store the original image and reprocess with current fit mode
        self.gradient_image_original = Some(img);
        self.reprocess_nutrient_image_with_current_fit_mode(queue)?;

        tracing::info!("Gray-Scott nutrient image loaded successfully");
        Ok(())
    }

    /// Reprocess the loaded image with the current fit mode and strength settings
    pub fn reprocess_nutrient_image_with_current_fit_mode(
        &mut self,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if let Some(original_img) = &self.gradient_image_original {
            let target_w = self.width as u32;
            let target_h = self.height as u32;

            tracing::info!(
                "Reprocessing Gray-Scott nutrient image with fit mode: {:?}",
                self.settings.gradient_image_fit_mode
            );

            // Convert to grayscale
            let gray = original_img.to_luma8();

            // Apply fit mode
            let resized = match self.settings.gradient_image_fit_mode {
                GradientImageFitMode::Stretch => image::imageops::resize(
                    &gray,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Lanczos3,
                ),
                GradientImageFitMode::Center => {
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
                GradientImageFitMode::FitH => {
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
                GradientImageFitMode::FitV => {
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

            // Apply mirror/invert controls
            if self.settings.gradient_image_mirror_horizontal {
                let w = target_w as usize;
                let h = target_h as usize;
                for y in 0..h {
                    let row = &mut buffer[y * w..(y + 1) * w];
                    row.reverse();
                }
            }
            if self.settings.gradient_image_invert_tone {
                for v in buffer.iter_mut() {
                    *v = 1.0 - *v;
                }
            }

            self.gradient_image_base = Some(buffer.clone());
            self.gradient_image_raw = Some(buffer.clone());

            // Upload to GPU
            if let Some(grad_buf) = &self.gradient_buffer {
                queue.write_buffer(grad_buf, 0, bytemuck::cast_slice::<f32, u8>(&buffer));
                tracing::info!("Reprocessed gradient image uploaded to GPU buffer");
            } else {
                tracing::error!("No gradient buffer available for reprocessing!");
                return Err(SimulationError::InvalidParameter(
                    "No gradient buffer available".to_string(),
                ));
            }

            self.gradient_image_needs_upload = false;
            tracing::info!("Gray-Scott nutrient image reprocessed successfully");
        }
        Ok(())
    }

    pub(crate) fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
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
            "nutrient_pattern" => {
                if let Some(v) = value.as_str() {
                    self.settings.nutrient_pattern = match v {
                        // Handle lowercase internal values
                        "uniform" => NutrientPattern::Uniform,
                        "checkerboard" => NutrientPattern::Checkerboard,
                        "diagonal_gradient" => NutrientPattern::DiagonalGradient,
                        "radial_gradient" => NutrientPattern::RadialGradient,
                        "vertical_stripes" => NutrientPattern::VerticalStripes,
                        "horizontal_stripes" => NutrientPattern::HorizontalStripes,
                        "enhanced_noise" => NutrientPattern::EnhancedNoise,
                        "wave_function" => NutrientPattern::WaveFunction,
                        "cosine_grid" => NutrientPattern::CosineGrid,
                        "image_gradient" => NutrientPattern::ImageGradient,
                        // Handle capitalized display names from frontend
                        "Uniform" => NutrientPattern::Uniform,
                        "Checkerboard" => NutrientPattern::Checkerboard,
                        "Diagonal Gradient" => NutrientPattern::DiagonalGradient,
                        "Radial Gradient" => NutrientPattern::RadialGradient,
                        "Vertical Stripes" => NutrientPattern::VerticalStripes,
                        "Horizontal Stripes" => NutrientPattern::HorizontalStripes,
                        "Enhanced Noise" => NutrientPattern::EnhancedNoise,
                        "Wave Function" => NutrientPattern::WaveFunction,
                        "Cosine Grid" => NutrientPattern::CosineGrid,
                        "Image Gradient" => NutrientPattern::ImageGradient,
                        _ => NutrientPattern::Uniform,
                    };
                } else if let Some(v) = value.as_u64() {
                    // Also support numeric values for backward compatibility
                    self.settings.nutrient_pattern = match v {
                        0 => NutrientPattern::Uniform,
                        1 => NutrientPattern::Checkerboard,
                        2 => NutrientPattern::DiagonalGradient,
                        3 => NutrientPattern::RadialGradient,
                        4 => NutrientPattern::VerticalStripes,
                        5 => NutrientPattern::HorizontalStripes,
                        6 => NutrientPattern::EnhancedNoise,
                        7 => NutrientPattern::WaveFunction,
                        8 => NutrientPattern::CosineGrid,
                        9 => NutrientPattern::ImageGradient,
                        _ => NutrientPattern::Uniform,
                    };
                }
            }
            "nutrient_pattern_reversed" => {
                if let Some(v) = value.as_bool() {
                    self.settings.nutrient_pattern_reversed = v;
                }
            }
            "gradient_image_fit_mode" => {
                if let Some(v) = value.as_str() {
                    self.settings.gradient_image_fit_mode = match v {
                        "Stretch" => GradientImageFitMode::Stretch,
                        "Center" => GradientImageFitMode::Center,
                        "Fit H" => GradientImageFitMode::FitH,
                        "Fit V" => GradientImageFitMode::FitV,
                        _ => GradientImageFitMode::Stretch,
                    };
                    // Reprocess the image if one is loaded
                    if self.gradient_image_original.is_some() {
                        if let Err(e) = self.reprocess_nutrient_image_with_current_fit_mode(queue) {
                            tracing::error!("Failed to reprocess gradient image: {}", e);
                        }
                    }
                }
            }
            "gradient_image_mirror_horizontal" => {
                if let Some(v) = value.as_bool() {
                    self.settings.gradient_image_mirror_horizontal = v;
                    if self.gradient_image_original.is_some() {
                        if let Err(e) = self.reprocess_nutrient_image_with_current_fit_mode(queue) {
                            tracing::error!("Failed to reprocess gradient image: {}", e);
                        }
                    }
                }
            }
            "gradient_image_invert_tone" => {
                if let Some(v) = value.as_bool() {
                    self.settings.gradient_image_invert_tone = v;
                    if self.gradient_image_original.is_some() {
                        if let Err(e) = self.reprocess_nutrient_image_with_current_fit_mode(queue) {
                            tracing::error!("Failed to reprocess gradient image: {}", e);
                        }
                    }
                }
            }
            "cursor_size" => {
                if let Some(v) = value.as_f64() {
                    self.cursor_size = v as f32;
                }
            }
            "cursor_strength" => {
                if let Some(v) = value.as_f64() {
                    self.cursor_strength = v as f32;
                }
            }
            "simulation_resolution_scale" => {
                if let Some(v) = value.as_f64() {
                    let old_scale = self.settings.simulation_resolution_scale;
                    self.settings.simulation_resolution_scale = v as f32;
                    tracing::debug!(
                        "Simulation resolution scale updated from {:.2} to {:.2}",
                        old_scale,
                        v
                    );

                    // Trigger immediate resize to apply the new resolution scale
                    // We need to get the current surface config from the renderer
                    let current_config = self.renderer.get_surface_config().clone();
                    if let Err(e) = GrayScottModel::resize(self, device, queue, &current_config) {
                        tracing::error!(
                            "Failed to resize simulation after resolution scale change: {}",
                            e
                        );
                        return Err(e);
                    }
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
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            // Adaptive timestep parameters
            max_timestep: 1.0,
            stability_factor: 0.5,
            enable_adaptive_timestep: 1,
            // Dependency tracking parameters
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        self.renderer.update_settings(&self.settings, queue);

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
            if let Err(e) = self.update_gradient_from_webcam(queue) {
                tracing::warn!("Gray-Scott webcam gradient update failed: {}", e);
            }
        }
        // Calculate delta time
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        // Update camera for smooth movement
        self.renderer.camera.update(delta_time);

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
            compute_pass.set_bind_group(0, &self.bind_groups[self.current_buffer], &[]);
            compute_pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Swap buffers for next frame
        self.current_buffer = 1 - self.current_buffer;

        // Render the current state - pass the output buffer (which contains the latest results)
        let output_buffer = &self.uvs_buffers[self.current_buffer];
        self.renderer
            .render(
                surface_view,
                output_buffer,
                &self.params_buffer,
                &self.background_bind_group,
            )
            .map_err(|e| SimulationError::Gpu(Box::new(e)))
    }

    pub fn start_webcam_capture(&mut self, device_index: i32) -> SimulationResult<()> {
        if self.gradient_buffer.is_none() {
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

    pub fn update_gradient_from_webcam(&mut self, queue: &Arc<Queue>) -> SimulationResult<()> {
        if let Some(frame_data) = self.webcam_capture.get_latest_frame_data() {
            let mut buffer = self.webcam_capture.frame_data_to_gradient_buffer(
                &frame_data,
                self.width,
                self.height,
            )?;
            if self.settings.gradient_image_mirror_horizontal {
                let w = self.width as usize;
                let h = self.height as usize;
                for y in 0..h {
                    let row = &mut buffer[y * w..(y + 1) * w];
                    row.reverse();
                }
            }
            if self.settings.gradient_image_invert_tone {
                for v in buffer.iter_mut() {
                    *v = 1.0 - *v;
                }
            }
            let processed = buffer;
            if let Some(grad_buf) = &self.gradient_buffer {
                queue.write_buffer(grad_buf, 0, bytemuck::cast_slice::<f32, u8>(&processed));
            }
            self.gradient_image_raw = Some(processed);
            self.gradient_image_needs_upload = false;
        }
        Ok(())
    }

    pub fn update_cursor_position(
        &mut self,
        _x: f32,
        _y: f32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // x and y are world coordinates, pass them directly to shader
        // The shader will convert them to view space to match input.world_pos

        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            timestep: self.settings.timestep,
            width: self.width,
            height: self.height,
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            // Adaptive timestep parameters
            max_timestep: 1.0,
            stability_factor: 0.5,
            enable_adaptive_timestep: 1,
            // Dependency tracking parameters
            change_threshold: 0.001,
            enable_selective_updates: 0,
        };

        // Update params buffer
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        Ok(())
    }

    pub fn handle_mouse_interaction(
        &mut self,
        texture_x: f32,
        texture_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // texture_x and texture_y are in [0,1] range
        // Update cursor position (for UI feedback, etc.)
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

        let tx = (texture_coords.x * self.width as f32) as i32;
        let ty = (texture_coords.y * self.height as f32) as i32;

        // Apply interaction in a circular area
        let radius = self.cursor_size as i32; // Use configurable cursor size

        // Collect all updates into a batch
        let mut updates: Vec<(usize, UVPair)> = Vec::new();

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let px = tx + dx;
                let py = ty + dy;

                // Check bounds
                if px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32 {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();
                    if distance <= radius as f32 {
                        let index = (py * self.width as i32 + px) as usize;
                        let factor = (1.0 - (distance / radius as f32)) * self.cursor_strength;

                        let uv_pair = if mouse_button == 0 {
                            // Left mouse button: seed the reaction with higher V concentration
                            UVPair {
                                u: 0.2 + 0.3 * factor,
                                v: 0.8 + 0.2 * factor,
                            }
                        } else if mouse_button == 2 {
                            // Right mouse button: create voids/erase
                            UVPair { u: 1.0, v: 0.0 }
                        } else {
                            // Middle mouse button or other: no effect
                            continue;
                        };

                        updates.push((index, uv_pair));
                    }
                }
            }
        }

        // Batch write all updates at once
        if !updates.is_empty() {
            // Group consecutive updates for more efficient buffer writes
            updates.sort_by_key(|&(index, _)| index);

            // Find consecutive ranges and batch them
            let mut i = 0;
            while i < updates.len() {
                let start_idx = updates[i].0;
                let mut end_idx = start_idx;
                let mut batch_data = vec![updates[i].1];

                // Collect consecutive indices
                let mut j = i + 1;
                while j < updates.len() && updates[j].0 == end_idx + 1 {
                    end_idx = updates[j].0;
                    batch_data.push(updates[j].1);
                    j += 1;
                }

                // Write the batch
                let offset = (start_idx * std::mem::size_of::<UVPair>()) as u64;
                let data = bytemuck::cast_slice(&batch_data);

                // Write to both buffers for immediate visibility
                queue.write_buffer(&self.uvs_buffers[0], offset, data);
                queue.write_buffer(&self.uvs_buffers[1], offset, data);

                i = j;
            }
        }

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
        self.renderer.camera.pan(delta_x, delta_y);
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        self.renderer.camera.zoom(delta);
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.renderer
            .camera
            .zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    pub fn reset_camera(&mut self) {
        self.renderer.camera.reset();
    }

    pub(crate) fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.gui_visible
    }

    pub(crate) fn is_gui_visible(&self) -> bool {
        self.gui_visible
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
        self.renderer.camera.update(delta_time);

        // Skip compute pass - just render current state
        // Render the current state - pass the current buffer (which contains the latest results)
        let current_buffer = &self.uvs_buffers[self.current_buffer];
        self.renderer
            .render(
                surface_view,
                current_buffer,
                &self.params_buffer,
                &self.background_bind_group,
            )
            .map_err(|e| SimulationError::Gpu(Box::new(e)))
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
            "currentLut" => {
                if let Some(lut_name) = value.as_str() {
                    self.current_color_scheme_name = lut_name.to_string();
                    let lut_manager = crate::simulations::shared::ColorSchemeManager::new();
                    let mut lut_data = lut_manager
                        .get(&self.current_color_scheme_name)
                        .unwrap_or_else(|_| lut_manager.get_default());

                    // Apply reversal if needed
                    if self.color_scheme_reversed {
                        lut_data.reverse();
                    }

                    self.renderer.update_lut(&lut_data, queue);
                }
            }
            "lutReversed" => {
                if let Some(reversed) = value.as_bool() {
                    self.color_scheme_reversed = reversed;
                    let lut_manager = crate::simulations::shared::ColorSchemeManager::new();
                    let mut lut_data = lut_manager
                        .get(&self.current_color_scheme_name)
                        .unwrap_or_else(|_| lut_manager.get_default());

                    // Apply reversal if needed
                    if self.color_scheme_reversed {
                        lut_data.reverse();
                    }

                    self.renderer.update_lut(&lut_data, queue);
                }
            }
            "cursorSize" => {
                if let Some(size) = value.as_f64() {
                    self.cursor_size = size as f32;
                }
            }
            "cursorStrength" => {
                if let Some(strength) = value.as_f64() {
                    self.cursor_strength = strength as f32;
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
        serde_json::json!({
            "width": self.width,
            "height": self.height,
            "lut_reversed": self.color_scheme_reversed,
            "current_lut_name": self.current_color_scheme_name,
            "gui_visible": self.gui_visible,
            "cursor_size": self.cursor_size,
            "cursor_strength": self.cursor_strength,
            "camera": {
                "position": self.renderer.camera.position,
                "zoom": self.renderer.camera.zoom
            }
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
            "position": [self.renderer.camera.position[0], self.renderer.camera.position[1]],
            "zoom": self.renderer.camera.zoom
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
        self.renderer.update_lut(color_scheme, queue);
        Ok(())
    }
}
