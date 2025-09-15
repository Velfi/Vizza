use crate::error::SimulationResult;
use crate::simulations::shared::gpu_utils::resource_helpers;
use std::sync::Arc;
use wgpu::{Device, Queue};

#[derive(Debug)]
pub struct PaintCompute {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl PaintCompute {
    pub fn new(device: &Arc<Device>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gray-Scott Paint Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("paint.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Paint Bind Group Layout"),
            entries: &[
                resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_texture_entry(1, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::ReadWrite, wgpu::TextureFormat::Rgba32Float),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Paint Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Paint Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn paint(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        texture: &wgpu::Texture,
        mouse_x: f32,
        mouse_y: f32,
        cursor_size: f32,
        cursor_strength: f32,
        mouse_button: u32,
        width: u32,
        height: u32,
    ) -> SimulationResult<()> {
        // Create paint parameters
        let paint_params = PaintParams {
            mouse_x,
            mouse_y,
            cursor_size,
            cursor_strength,
            mouse_button,
            width,
            height,
            _pad1: 0,
        };

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Paint Params Buffer"),
            size: std::mem::size_of::<PaintParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&params_buffer, 0, bytemuck::cast_slice(&[paint_params]));

        // Create bind group
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Paint Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &params_buffer),
                resource_helpers::texture_view_entry(1, &texture_view),
            ],
        });

        // Execute paint pass
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Gray-Scott Paint Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gray-Scott Paint Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PaintParams {
    mouse_x: f32,
    mouse_y: f32,
    cursor_size: f32,
    cursor_strength: f32,
    mouse_button: u32,
    width: u32,
    height: u32,
    _pad1: u32,
}
