use crate::error::SimulationResult;
use crate::simulations::shared::LutManager;
use crate::simulations::traits::Simulation;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, Device, Queue, RenderPipeline, SurfaceConfiguration, TextureView};

#[derive(Debug)]
pub struct MainMenuModel {
    render_pipeline: RenderPipeline,
    time_buffer: Buffer,
    time_bind_group: BindGroup,
    lut_bind_group: BindGroup,
    start_time: Instant,
    gui_visible: bool,
}

impl MainMenuModel {
    pub fn new(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
        lut_manager: &LutManager,
    ) -> SimulationResult<Self> {
        // Create the time uniform buffer and bind group
        let time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Main Menu Time Buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let time_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Main Menu Time Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let time_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Menu Time Bind Group"),
            layout: &time_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
        });

        // Create LUT buffer from a random LUT and create bind group
        let lut_data = lut_manager.get_random_lut()?;
        let lut_data_u32 = lut_data.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Main Menu LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let lut_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Main Menu LUT Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Main Menu LUT Bind Group"),
            layout: &lut_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lut_buffer.as_entire_binding(),
            }],
        });

        // Create shaders
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Menu Background Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulations::main_menu::shaders::VERTEX_SHADER.into(),
            ),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Menu Background Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulations::main_menu::shaders::FRAGMENT_SHADER.into(),
            ),
        });

        // Create render pipeline with both bind groups
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Main Menu Background Pipeline Layout"),
                bind_group_layouts: &[&time_bind_group_layout, &lut_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Menu Background Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
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

        let start_time = Instant::now();

        Ok(Self {
            render_pipeline,
            time_buffer,
            time_bind_group,
            lut_bind_group,
            start_time,
            gui_visible: false,
        })
    }

    fn get_time(&self) -> f32 {
        // 20x slower than real time
        self.start_time.elapsed().as_secs_f32() * 0.03
    }
}

impl Simulation for MainMenuModel {
    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // For static rendering, just render with current time (don't advance animation)
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Menu Background Static Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Background Static Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.time_bind_group, &[]);
            render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Draw 6 vertices for the full-screen quad
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Update the time buffer
        let time_seconds = self.get_time();
        queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[time_seconds]));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Menu Background Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Background Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.time_bind_group, &[]);
            render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Draw 6 vertices for the full-screen quad
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn resize(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        _new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        // No resize handling needed for this simulation
        Ok(())
    }

    fn update_setting(
        &mut self,
        _setting_name: &str,
        _value: Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No configurable settings for this simulation
        Ok(())
    }

    fn get_settings(&self) -> Value {
        // No settings for this simulation
        serde_json::json!({})
    }

    fn get_state(&self) -> Value {
        serde_json::json!({
            "time": self.get_time(),
            "gui_visible": self.gui_visible
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No mouse interaction for this simulation
        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No mouse interaction for this simulation
        Ok(())
    }

    fn pan_camera(&mut self, _delta_x: f32, _delta_y: f32) {
        // No camera for this simulation
    }

    fn zoom_camera(&mut self, _delta: f32) {
        // No camera for this simulation
    }

    fn zoom_camera_to_cursor(&mut self, _delta: f32, _cursor_x: f32, _cursor_y: f32) {
        // No camera for this simulation
    }

    fn reset_camera(&mut self) {
        // No camera for this simulation
    }

    fn get_camera_state(&self) -> Value {
        // No camera for this simulation
        serde_json::json!({})
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // No presets for this simulation
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // No presets for this simulation
        Ok(())
    }

    fn apply_settings(
        &mut self,
        _settings: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No settings for this simulation
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No-op for Main Menu
        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.gui_visible
    }

    fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No settings to randomize for this simulation
        Ok(())
    }
}
