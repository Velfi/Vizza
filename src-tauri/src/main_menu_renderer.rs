use std::sync::Arc;
use wgpu::{Device, Queue, RenderPipeline, SurfaceConfiguration, BindGroup, Buffer, BindGroupLayout};

pub struct MainMenuRenderer {
    render_pipeline: RenderPipeline,
    time_buffer: Buffer,
    time_bind_group: BindGroup,
    start_time: std::time::Instant,
}

impl MainMenuRenderer {
    pub fn new(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Vertex shader for a full-screen quad
        let vertex_shader_source = r#"
            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) uv: vec2<f32>,
            };

            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 6>(
                    vec2<f32>(-1.0, -1.0),
                    vec2<f32>(1.0, -1.0),
                    vec2<f32>(-1.0, 1.0),
                    vec2<f32>(-1.0, 1.0),
                    vec2<f32>(1.0, -1.0),
                    vec2<f32>(1.0, 1.0),
                );
                var uv = (pos[vertex_index] + vec2<f32>(1.0, 1.0)) * 0.5;
                var out: VertexOutput;
                out.clip_position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
                out.uv = uv;
                return out;
            }
        "#;

        // Fragment shader with animated plasma effect
        let fragment_shader_source = r#"
            @group(0) @binding(0)
            var<uniform> time: f32;

            @fragment
            fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
                let color = 0.5 + 0.5 * vec3<f32>(
                    sin(uv.x * 10.0 + time),
                    sin(uv.y * 10.0 + time * 1.2),
                    sin((uv.x + uv.y) * 10.0 + time * 0.7)
                );
                return vec4<f32>(color, 1.0);
            }
        "#;

        // Create the time uniform buffer and bind group
        let time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Time Buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let time_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Time Bind Group Layout"),
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
            label: Some("Time Bind Group"),
            layout: &time_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: time_buffer.as_entire_binding(),
            }],
        });

        // Create shaders
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Menu Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(vertex_shader_source.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Menu Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(fragment_shader_source.into()),
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main Menu Pipeline Layout"),
            bind_group_layouts: &[&time_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Menu Render Pipeline"),
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

        let start_time = std::time::Instant::now();

        Ok(Self {
            render_pipeline,
            time_buffer,
            time_bind_group,
            start_time,
        })
    }

    /// Call this every frame, passing the current time in seconds (as f32)
    pub fn render(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &wgpu::TextureView,
        time_seconds: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update the time buffer
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
            render_pass.draw(0..6, 0..1); // Draw 6 vertices for the full-screen quad
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn get_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }
}
