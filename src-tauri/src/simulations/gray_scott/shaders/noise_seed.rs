use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{Device, Queue};
use crate::error::SimulationResult;
use rand;

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
                // UVs buffer (read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Params buffer (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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

    pub fn seed_noise(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        uvs_buffer: &wgpu::Buffer,
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Noise Seed Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uvs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
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

    pub fn seed_random_noise(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        u_buffer: &wgpu::Buffer,
        v_buffer: &wgpu::Buffer,
    ) -> SimulationResult<()> {
        // Generate random seed and strength
        let seed = rand::random::<u32>();
        let noise_strength = 0.1; // Default noise strength
        
        // Get buffer size to determine width/height
        let buffer_size = u_buffer.size() as usize;
        let width = (buffer_size as f32).sqrt() as u32;
        let height = width;
        
        self.seed_noise(device, queue, u_buffer, width, height, seed, noise_strength)
    }
}
