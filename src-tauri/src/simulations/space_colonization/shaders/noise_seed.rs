use crate::error::SimulationResult;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{Device, Queue};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct NoiseParams {
    pub width: u32,
    pub height: u32,
    pub seed: u32,
    pub noise_strength: f32,
    pub max_attractors: u32,
    pub attractor_pattern: u32, // 0=Random, 1=Clustered, 2=Grid, 3=Circular, 4=Boundary
}

#[derive(Debug)]
pub struct NoiseSeedCompute {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl NoiseSeedCompute {
    pub fn new(device: &Arc<Device>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Space Colonization Noise Seed Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("noise_seed.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Space Colonization Noise Seed Bind Group Layout"),
            entries: &[
                // Attractors buffer (read-write)
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
            label: Some("Space Colonization Noise Seed Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Space Colonization Noise Seed Pipeline"),
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
        attractors_buffer: &wgpu::Buffer,
        width: u32,
        height: u32,
        seed: u32,
        noise_strength: f32,
        max_attractors: u32,
        attractor_pattern: u32,
    ) -> SimulationResult<()> {
        // Create params buffer
        let params = NoiseParams {
            width,
            height,
            seed,
            noise_strength,
            max_attractors,
            attractor_pattern,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Space Colonization Noise Seed Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Space Colonization Noise Seed Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: attractors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create command encoder and dispatch
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Space Colonization Noise Seed Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Space Colonization Noise Seed Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        let workgroups = (max_attractors + 63) / 64; // Round up to nearest 64
        compute_pass.dispatch_workgroups(workgroups, 1, 1);

        drop(compute_pass);
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
} 