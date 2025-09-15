use crate::error::SimulationResult;
use crate::simulations::shared::gpu_utils::resource_helpers;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{Device, Queue};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NoiseParams {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub noise_strength: f32,
}

#[derive(Debug)]
pub struct NoiseSeedCompute {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl NoiseSeedCompute {
    pub fn new(device: &Arc<Device>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Noise Seed Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("noise_seed.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Noise Seed Bind Group Layout"),
            entries: &[
                // UVs texture (read-write)
                resource_helpers::storage_texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::WriteOnly, wgpu::TextureFormat::Rgba32Float),
                // Params buffer (uniform)
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Noise Seed Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Noise Seed Pipeline"),
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

    #[allow(clippy::too_many_arguments)]
    pub fn seed_noise(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        uvs_texture: &wgpu::Texture,
        width: u32,
        height: u32,
        seed: u32,
        noise_strength: f32,
    ) -> SimulationResult<()> {
        // Create params buffer
        let params = NoiseParams {
            width,
            height,
            seed,
            noise_strength,
        };

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Noise Params Buffer"),
            size: std::mem::size_of::<NoiseParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Create bind group
        let texture_view = uvs_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Noise Seed Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &texture_view),
                resource_helpers::buffer_entry(1, &params_buffer),
            ],
        });

        // Execute compute pass
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Gray Scott Noise Seed Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gray Scott Noise Seed Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch with 8x8 workgroup size
            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
