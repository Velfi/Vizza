use crate::commands::app_settings::AppSettings;
use crate::error::SimulationResult;
use crate::simulations::gradient::shaders::GRADIENT_SHADER;
use crate::simulations::shared::{BindGroupBuilder, ColorScheme, RenderPipelineBuilder};
use crate::simulations::traits::Simulation;
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, Buffer, BufferUsages, Device, Queue, RenderPipeline, SurfaceConfiguration,
    TextureFormat, TextureView,
};

#[derive(Debug)]
pub struct GradientSimulation {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
    lut_buffer: Buffer,
    lut_bind_group: BindGroup,
    current_lut: Option<ColorScheme>,
    // State
    pub display_mode: u32, // 0 = smooth, 1 = dithered
    params_buffer: Buffer,
    // GUI state
    gui_visible: bool,
}

impl GradientSimulation {
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        _app_settings: &AppSettings,
    ) -> Self {
        // Create vertex buffer for a full-screen quad
        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 0.0, // position, uv
            1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gradient Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gradient Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        // Create LUT buffer
        let lut_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gradient LUT Buffer"),
            size: 256 * 3 * 4, // 256 RGB values as u32 (4 bytes each)
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create custom bind group layout for gradient (LUT + params)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // LUT buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Params buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("gradient_bind_group_layout"),
        });

        // Create parameters buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gradient Params Buffer"),
            size: 16, // 4 u32s = 16 bytes
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize parameters with smooth display mode
        let params_data = [0u32, 0u32, 0u32, 0u32]; // display_mode = 0 (smooth), padding
        queue.write_buffer(&params_buffer, 0, bytemuck::cast_slice(&params_data));

        // Create bind group using GPU utilities
        let lut_bind_group = BindGroupBuilder::new(device, &bind_group_layout)
            .add_buffer(0, &lut_buffer)
            .add_buffer(1, &params_buffer)
            .with_label("Gradient LUT Bind Group".to_string())
            .build();

        // Create render pipeline using GPU utilities
        let shader = Arc::new(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gradient"),
            source: wgpu::ShaderSource::Wgsl(GRADIENT_SHADER.into()),
        }));

        let render_pipeline = RenderPipelineBuilder::new(Arc::new(device.clone()))
            .with_shader(shader)
            .with_bind_group_layouts(vec![bind_group_layout.clone()])
            .with_vertex_buffer_layouts(vec![wgpu::VertexBufferLayout {
                array_stride: 16, // 4 floats * 4 bytes
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2, // position
                    },
                    wgpu::VertexAttribute {
                        offset: 8,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2, // uv
                    },
                ],
            }])
            .with_fragment_targets(vec![Some(wgpu::ColorTargetState {
                format,
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
            .with_label("Gradient Render Pipeline".to_string())
            .build();

        // Initialize with a default gradient LUT using standard format
        let mut default_lut = Vec::with_capacity(256 * 3);
        for i in 0..256 {
            default_lut.push(i as u32); // Red channel: 0 to 255
        }
        for i in 0..256 {
            default_lut.push(i as u32); // Green channel: 0 to 255
        }
        for i in 0..256 {
            default_lut.push(i as u32); // Blue channel: 0 to 255
        }
        queue.write_buffer(&lut_buffer, 0, bytemuck::cast_slice(&default_lut));

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: 6,
            lut_buffer,
            lut_bind_group: lut_bind_group.clone(),
            current_lut: None,
            display_mode: 0, // Start with smooth mode
            params_buffer,
            gui_visible: true, // Start with GUI visible
        }
    }

    pub fn update_lut(&mut self, _device: &Device, queue: &Queue, color_scheme: &ColorScheme) {
        // Update LUT buffer with new data using standard format
        let lut_data_u32 = color_scheme.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
        self.current_lut = Some(color_scheme.clone());
    }

    pub fn set_display_mode(&mut self, mode: u32, queue: &Queue) {
        self.display_mode = mode;
        let params_data = [mode, 0u32, 0u32, 0u32]; // display_mode, padding
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&params_data));
    }
}

impl Simulation for GradientSimulation {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        _delta_time: f32,
    ) -> SimulationResult<()> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Gradient Render Encoder"),
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Gradient Render Pass"),
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
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.lut_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        drop(render_pass);
        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        self.render_frame(device, queue, surface_view, 0.0)
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        match state_name {
            "displayMode" => {
                if let Some(mode) = value.as_u64() {
                    self.set_display_mode(mode as u32, queue);
                }
            }
            _ => {
                tracing::warn!("Unknown state parameter for Gradient: {}", state_name);
            }
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::json!({})
    }

    fn get_state(&self) -> Value {
        serde_json::json!({})
    }

    fn handle_mouse_interaction(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _mouse_button: u32,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No mouse interaction for gradient
        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // No mouse interaction for gradient
        Ok(())
    }

    // Camera methods use default implementations (no-op)

    fn get_camera_state(&self) -> Value {
        serde_json::json!({})
    }

    fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.gui_visible
    }

    fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    fn resize(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        _new_config: &SurfaceConfiguration,
    ) -> crate::error::SimulationResult<()> {
        // No resize needed for gradient simulation
        Ok(())
    }

    fn update_setting(
        &mut self,
        _setting_name: &str,
        _value: Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // No settings for gradient simulation
        Ok(())
    }

    fn save_preset(&self, _preset_name: &str) -> crate::error::SimulationResult<()> {
        // No presets for gradient simulation
        Ok(())
    }

    fn load_preset(
        &mut self,
        _preset_name: &str,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // No presets for gradient simulation
        Ok(())
    }

    fn apply_settings(
        &mut self,
        _settings: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // No settings for gradient simulation
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // No runtime state to reset for gradient simulation
        Ok(())
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // No settings to randomize for gradient simulation
        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Update color scheme buffer with new data using standard format
        let color_scheme_data_u32 = color_scheme.to_u32_buffer();
        queue.write_buffer(
            &self.lut_buffer,
            0,
            bytemuck::cast_slice(&color_scheme_data_u32),
        );
        self.current_lut = Some(color_scheme.clone());
        Ok(())
    }
}
