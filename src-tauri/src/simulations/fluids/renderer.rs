use crate::error::SimulationResult;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::{BindGroupBuilder, CommonBindGroupLayouts, RenderPipelineBuilder, ShaderManager};
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

#[derive(Debug)]
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface_config: SurfaceConfiguration,
    pub background_color_buffer: wgpu::Buffer,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub background_bind_group_layout: wgpu::BindGroupLayout,
    pub render_infinite_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> SimulationResult<Self> {
        let background_color_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Fluids Background Color",
            &[0.0f32, 0.0f32, 0.0f32, 1.0f32],
        );

        let mut shader_manager = ShaderManager::new();
        let common_layouts = CommonBindGroupLayouts::new(device);
        let background_shader = shader_manager.load_shader(
            device,
            "fluids_background",
            crate::simulations::fluids::shaders::BACKGROUND_RENDER_SHADER,
        );

        let background_bind_group_layout = common_layouts.uniform_buffer.clone();
        let camera_bind_group_layout = common_layouts.camera.clone();

        let render_infinite_pipeline = RenderPipelineBuilder::new(device.clone())
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
            .with_label("Fluids Background Pipeline".to_string())
            .build();

        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            surface_config: surface_config.clone(),
            background_color_buffer,
            camera_bind_group_layout,
            background_bind_group_layout,
            render_infinite_pipeline,
        })
    }

    pub fn resize(&mut self, new_config: &SurfaceConfiguration) -> SimulationResult<()> {
        self.surface_config = new_config.clone();
        Ok(())
    }

    pub fn render_background(
        &self,
        view: &TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) -> Result<(), wgpu::SurfaceError> {
        let background_bind_group = BindGroupBuilder::new(&self.device, &self.background_bind_group_layout)
            .add_buffer(0, &self.background_color_buffer)
            .with_label("Fluids Background Bind Group".to_string())
            .build();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Fluids Background Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fluids Background Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            pass.set_pipeline(&self.render_infinite_pipeline);
            pass.set_bind_group(0, &background_bind_group, &[]);
            pass.set_bind_group(1, camera_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

