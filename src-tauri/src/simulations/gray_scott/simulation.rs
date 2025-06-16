use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::renderer::Renderer;
use super::settings::{NutrientPattern, Settings};
use super::shaders::noise_seed::NoiseSeedCompute;
use crate::simulations::shared::WorldCoords;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SimulationParams {
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub delta_u: f32,
    pub delta_v: f32,
    pub width: u32,
    pub height: u32,
    pub nutrient_pattern: u32,
    pub is_nutrient_pattern_reversed: u32,
    pub cursor_x: f32,
    pub cursor_y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UVPair {
    u: f32,
    v: f32,
}

pub struct GrayScottModel {
    pub renderer: Renderer,
    pub settings: Settings,
    pub width: u32,
    pub height: u32,
    pub current_lut_index: usize,
    pub lut_reversed: bool,
    uvs_buffers: [wgpu::Buffer; 2], // Double buffering
    current_buffer: usize,
    params_buffer: wgpu::Buffer,
    bind_groups: [wgpu::BindGroup; 2], // Double buffering
    compute_pipeline: wgpu::ComputePipeline,
    noise_seed_compute: NoiseSeedCompute,
    last_frame_time: std::time::Instant,
    show_gui: bool,
}

impl GrayScottModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        width: u32,
        height: u32,
        settings: Settings,
        lut_manager: &crate::simulations::shared::LutManager,
        available_luts: &[String],
        current_lut_index: usize,
        lut_reversed: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let vec_capacity = (width * height) as usize;
        let mut uvs: Vec<UVPair> = std::iter::repeat(UVPair { u: 1.0, v: 0.0 })
            .take(vec_capacity)
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
            width,
            height,
            nutrient_pattern: settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: settings.nutrient_pattern_reversed as u32,
            cursor_x: 0.0,
            cursor_y: 0.0,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/reaction_diffusion.wgsl").into(),
            ),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Create bind groups for both buffers (input/output swapped)
        let bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 0"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uvs_buffers[0].as_entire_binding(), // input
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uvs_buffers[1].as_entire_binding(), // output
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bind Group 1"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uvs_buffers[1].as_entire_binding(), // input
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uvs_buffers[0].as_entire_binding(), // output
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            }),
        ];

        let renderer = Renderer::new(device, queue, surface_config, width, height)?;
        let noise_seed_compute = NoiseSeedCompute::new(device);

        // Initialize LUT
        let mut simulation = Self {
            renderer,
            settings,
            width,
            height,
            current_lut_index,
            lut_reversed,
            uvs_buffers,
            current_buffer: 0,
            params_buffer,
            bind_groups,
            compute_pipeline,
            noise_seed_compute,
            last_frame_time: std::time::Instant::now(),
            show_gui: false,
        };

        // Apply initial LUT
        if let Some(lut_name) = available_luts.get(current_lut_index) {
            if let Ok(mut lut_data) = lut_manager.load_lut(lut_name) {
                if lut_reversed {
                    lut_data.reverse();
                }
                simulation.renderer.update_lut(&lut_data, queue);
            }
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
            width: self.width,
            height: self.height,
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            cursor_x: 0.0,
            cursor_y: 0.0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        self.renderer.update_settings(&self.settings, queue);
    }

    pub fn render(&mut self, view: &TextureView) -> Result<(), wgpu::SurfaceError> {
        // Run compute pass
        let mut encoder =
            self.renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Gray Scott Compute Encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gray Scott Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_groups[self.current_buffer], &[]);
            compute_pass.dispatch_workgroups((self.width + 7) / 8, (self.height + 7) / 8, 1);
        }

        self.renderer
            .queue()
            .submit(std::iter::once(encoder.finish()));
        self.current_buffer = 1 - self.current_buffer;

        // Render the current state - pass the output buffer (which contains the latest results)
        let output_buffer = &self.uvs_buffers[self.current_buffer];
        self.renderer
            .render(view, output_buffer, &self.params_buffer)
    }

    pub fn resize(
        &mut self,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.renderer.resize(new_config)?;
        Ok(())
    }

    pub fn set_nutrient_pattern(&mut self, pattern: NutrientPattern, is_reversed: bool) {
        self.settings.nutrient_pattern = pattern;
        self.settings.nutrient_pattern_reversed = is_reversed;

        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            width: self.width,
            height: self.height,
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            cursor_x: 0.0,
            cursor_y: 0.0,
        };

        self.renderer
            .queue()
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
    }

    pub fn set_initial_state(&mut self, x: i32, y: i32, u: f32, v: f32) {
        let index = (y * self.width as i32 + x) as usize;
        let uvs = [UVPair { u, v }];
        self.renderer.queue().write_buffer(
            &self.uvs_buffers[self.current_buffer],
            (index * std::mem::size_of::<UVPair>()) as u64,
            bytemuck::cast_slice(&uvs),
        );
    }

    pub fn reset(&mut self) {
        let vec_capacity = (self.width * self.height) as usize;
        let uvs: Vec<UVPair> = std::iter::repeat(UVPair { u: 1.0, v: 0.0 })
            .take(vec_capacity)
            .collect();

        for buffer in &self.uvs_buffers {
            self.renderer
                .queue()
                .write_buffer(buffer, 0, bytemuck::cast_slice(&uvs));
        }
    }

    pub fn seed_random_noise(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) {
        // Generate a random seed for this noise generation
        let seed = rand::random::<u32>();

        // Use GPU-based noise seeding for both buffers
        for buffer in &self.uvs_buffers {
            if let Err(e) = self.noise_seed_compute.seed_noise(
                device,
                queue,
                buffer,
                self.width,
                self.height,
                seed,
                1.0, // Full noise strength
            ) {
                tracing::error!("Failed to seed noise on GPU: {}", e);
            }
        }
    }

    pub(crate) fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                        _ => NutrientPattern::Uniform,
                    };
                }
            }
            "nutrient_pattern_reversed" => {
                if let Some(v) = value.as_bool() {
                    self.settings.nutrient_pattern_reversed = v;
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
            width: self.width,
            height: self.height,
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            cursor_x: 0.0,
            cursor_y: 0.0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        self.renderer.update_settings(&self.settings, queue);

        Ok(())
    }

    pub(crate) fn render_frame(
        &mut self,
        device: &Device,
        queue: &Queue,
        surface_view: &TextureView,
    ) -> Result<(), wgpu::SurfaceError> {
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
            compute_pass.dispatch_workgroups((self.width + 7) / 8, (self.height + 7) / 8, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Swap buffers for next frame
        self.current_buffer = 1 - self.current_buffer;

        // Render the current state - pass the output buffer (which contains the latest results)
        let output_buffer = &self.uvs_buffers[self.current_buffer];
        self.renderer
            .render(surface_view, output_buffer, &self.params_buffer)
    }

    pub fn update_lut(
        &mut self,
        lut_data: &crate::simulations::shared::LutData,
        queue: &Arc<Queue>,
    ) {
        self.renderer.update_lut(lut_data, queue);
    }

    pub fn update_cursor_position(
        &mut self,
        x: f32,
        y: f32,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // x and y are world coordinates, pass them directly to shader
        // The shader will convert them to view space to match input.world_pos

        let params = SimulationParams {
            feed_rate: self.settings.feed_rate,
            kill_rate: self.settings.kill_rate,
            delta_u: self.settings.diffusion_rate_u,
            delta_v: self.settings.diffusion_rate_v,
            width: self.width,
            height: self.height,
            nutrient_pattern: self.settings.nutrient_pattern as u32,
            is_nutrient_pattern_reversed: self.settings.nutrient_pattern_reversed as u32,
            cursor_x: x,
            cursor_y: y,
        };

        // Update params buffer
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        Ok(())
    }

    pub fn handle_mouse_interaction(
        &mut self,
        x: f32,
        y: f32,
        is_seeding: bool,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // x and y are already world coordinates from the frontend
        let world_coords = WorldCoords::new(x, y);

        // Update cursor position first
        self.update_cursor_position(world_coords.x, world_coords.y, queue)?;

        // Convert world coordinates to simulation texture coordinates
        let texture_coords = world_coords.to_texture_coords();

        // Check if coordinates are within valid texture bounds
        if !texture_coords.is_valid() {
            return Ok(()); // Outside simulation bounds
        }

        let tx = (texture_coords.x * self.width as f32) as i32;
        let ty = (texture_coords.y * self.height as f32) as i32;

        // Apply interaction in a circular area
        let radius = 15; // Size of the brush

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
                        let factor = 1.0 - (distance / radius as f32);

                        let uv_pair = if is_seeding {
                            // Left mouse button: seed the reaction with higher V concentration
                            UVPair {
                                u: 0.2 + 0.3 * factor,
                                v: 0.8 + 0.2 * factor,
                            }
                        } else {
                            // Right mouse button: create voids/erase
                            UVPair { u: 1.0, v: 0.0 }
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

    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        tracing::debug!(
            "Gray-Scott simulation pan_camera called: delta=({:.2}, {:.2})",
            delta_x,
            delta_y
        );
        self.renderer.camera.pan(delta_x, delta_y);
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        tracing::debug!(
            "Gray-Scott simulation zoom_camera called: delta={:.2}",
            delta
        );
        self.renderer.camera.zoom(delta);
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        tracing::debug!(
            "Gray-Scott simulation zoom_camera_to_cursor called: delta={:.2}, cursor=({:.2}, {:.2})",
            delta, cursor_x, cursor_y
        );
        self.renderer
            .camera
            .zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    pub fn stop_camera_pan(&mut self) -> Result<(), String> {
        self.renderer.camera.stop_pan();
        Ok(())
    }

    pub fn reset_camera(&mut self) {
        tracing::debug!("Gray-Scott simulation reset_camera called");
        self.renderer.camera.reset();
    }
    
    pub(crate) fn toggle_gui(&mut self) {
        self.show_gui = !self.show_gui;
    }

    pub(crate) fn is_gui_visible(&self) -> bool {
        self.show_gui
    }
}
