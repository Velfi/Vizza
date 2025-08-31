use std::sync::Arc;
use wgpu::{BindGroup, Buffer, ComputePipeline, Device, Queue};

#[derive(Debug)]
pub struct AverageColorResources {
    pub buffer: Buffer,
    pub staging_buffer: Buffer,
    pub bind_group: BindGroup,
    pub pipeline: ComputePipeline,
}

impl AverageColorResources {
    pub fn new(
        device: &Arc<Device>,
        _display_texture: &wgpu::Texture,
        display_view: &wgpu::TextureView,
        label: &str,
    ) -> Self {
        let average_color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Average Color Buffer", label)),
            size: std::mem::size_of::<[u32; 4]>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let average_color_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Average Color Staging Buffer", label)),
            size: std::mem::size_of::<[u32; 4]>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create average color compute shader
        let average_color_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{} Average Color Shader", label)),
            source: wgpu::ShaderSource::Wgsl(include_str!("average_color.wgsl").into()),
        });

        let average_color_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{} Average Color Pipeline", label)),
                layout: None,
                module: &average_color_shader,
                entry_point: Some("main"),
                cache: None,
                compilation_options: Default::default(),
            });

        let average_color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{} Average Color Bind Group", label)),
            layout: &average_color_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(display_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &average_color_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            buffer: average_color_buffer,
            staging_buffer: average_color_staging_buffer,
            bind_group: average_color_bind_group,
            pipeline: average_color_pipeline,
        }
    }

    pub fn calculate_average_color(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        display_texture: &wgpu::Texture,
    ) {
        // Reset the average color buffer
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[0u32; 4]));

        // Create compute encoder
        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Average Color Compute Encoder"),
        });

        {
            let mut compute_pass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Average Color Compute Pass"),
                    timestamp_writes: None,
                });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(
                display_texture.width().div_ceil(16),
                display_texture.height().div_ceil(16),
                1,
            );
        }

        // Copy result to staging buffer for reading
        compute_encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &self.staging_buffer,
            0,
            std::mem::size_of::<[u32; 4]>() as u64,
        );

        queue.submit(std::iter::once(compute_encoder.finish()));

        // Map the staging buffer for reading - start the async operation
        self.staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
    }

    pub fn get_average_color(&self) -> Option<[f32; 4]> {
        // Check if buffer is mapped by trying to get mapped range
        // If not mapped, this will return early with None
        let slice = self.staging_buffer.slice(..);

        // We can't safely check if the buffer is mapped without potentially panicking,
        // so we'll assume the caller has properly waited for mapping
        let data = slice.get_mapped_range();

        let values: [u32; 4] = *bytemuck::from_bytes(&data);
        let pixel_count = values[3] as f32;

        // Drop the mapped range before returning
        drop(data);

        if pixel_count > 0.0 {
            let result = [
                values[0] as f32 / pixel_count / 255.0,
                values[1] as f32 / pixel_count / 255.0,
                values[2] as f32 / pixel_count / 255.0,
                1.0,
            ];
            Some(result)
        } else {
            None
        }
    }

    pub fn wait_for_mapping(&self, device: &Arc<Device>) {
        // Poll the device until the mapping is complete
        // Since we can't safely check mapping status without panicking,
        // we'll poll a fixed number of times with delays
        for _ in 0..10 {
            device.poll(wgpu::wgt::PollType::Wait).expect("Failed to poll device");

            // Small delay to allow async operations to complete
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Final poll to ensure completion
        device.poll(wgpu::wgt::PollType::Wait).expect("Failed to poll device");
    }

    pub fn unmap_staging_buffer(&self) {
        self.staging_buffer.unmap();
    }
}
