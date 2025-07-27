use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, Device, TextureView,
};

#[derive(Debug)]
pub struct BindGroupManager {
    pub compute_bind_group: BindGroup,
    pub display_bind_group: BindGroup,
    pub render_bind_group: BindGroup,
    pub camera_bind_group: BindGroup,
    pub gradient_bind_group: BindGroup,
}

impl BindGroupManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &Device,
        compute_bind_group_layout: &BindGroupLayout,
        display_bind_group_layout: &BindGroupLayout,
        render_bind_group_layout: &BindGroupLayout,
        camera_bind_group_layout: &BindGroupLayout,
        gradient_bind_group_layout: &BindGroupLayout,
        agent_buffer: &Buffer,
        trail_map_buffer: &Buffer,
        gradient_buffer: &Buffer,
        sim_size_buffer: &Buffer,
        display_view: &TextureView,
        display_sampler: &wgpu::Sampler,
        lut_buffer: &Buffer,
        camera_buffer: &Buffer,
        cursor_buffer: &Buffer,
        background_color_buffer: &Buffer,
    ) -> Self {
        Self {
            compute_bind_group: Self::create_compute_bind_group(
                device,
                compute_bind_group_layout,
                agent_buffer,
                trail_map_buffer,
                gradient_buffer,
                sim_size_buffer,
                cursor_buffer,
            ),
            display_bind_group: Self::create_display_bind_group(
                device,
                display_bind_group_layout,
                trail_map_buffer,
                gradient_buffer,
                display_view,
                sim_size_buffer,
                lut_buffer,
            ),
            render_bind_group: Self::create_render_bind_group(
                device,
                render_bind_group_layout,
                display_view,
                display_sampler,
                background_color_buffer,
            ),
            camera_bind_group: Self::create_camera_bind_group(
                device,
                camera_bind_group_layout,
                camera_buffer,
            ),
            gradient_bind_group: Self::create_gradient_bind_group(
                device,
                gradient_bind_group_layout,
                gradient_buffer,
                sim_size_buffer,
            ),
        }
    }

    fn create_compute_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        agent_buffer: &Buffer,
        trail_map_buffer: &Buffer,
        gradient_buffer: &Buffer,
        sim_size_buffer: &Buffer,
        cursor_buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: agent_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: trail_map_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: sim_size_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: gradient_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: cursor_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_display_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        trail_map_buffer: &Buffer,
        gradient_buffer: &Buffer,
        display_view: &TextureView,
        sim_size_buffer: &Buffer,
        lut_buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Display Compute Bind Group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: trail_map_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(display_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: sim_size_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: lut_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: gradient_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_render_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        display_view: &TextureView,
        display_sampler: &wgpu::Sampler,
        background_color_buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(display_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(display_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: background_color_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_camera_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }

    fn create_gradient_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        gradient_buffer: &Buffer,
        sim_size_buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Gradient Bind Group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 2,
                    resource: sim_size_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: gradient_buffer.as_entire_binding(),
                },
            ],
        })
    }
}
