use std::sync::Arc;
use wgpu::{Device, Queue, RenderPipeline, Buffer, SurfaceConfiguration};

pub struct MainMenuRenderer {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

impl MainMenuRenderer {
    pub fn new(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Vertex shader source
        let vertex_shader_source = r#"
            struct VertexOutput {
                @builtin(position) clip_position: vec4<f32>,
                @location(0) color: vec3<f32>,
            }

            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
                var out: VertexOutput;
                
                // Triangle vertices in clip space
                if (vertex_index == 0u) {
                    out.clip_position = vec4<f32>(0.0, 0.5, 0.0, 1.0);  // Top vertex
                } else if (vertex_index == 1u) {
                    out.clip_position = vec4<f32>(-0.5, -0.5, 0.0, 1.0); // Bottom left
                } else {
                    out.clip_position = vec4<f32>(0.5, -0.5, 0.0, 1.0);  // Bottom right
                }
                
                // Rainbow colors for each vertex
                if (vertex_index == 0u) {
                    out.color = vec3<f32>(1.0, 0.0, 0.0); // Red
                } else if (vertex_index == 1u) {
                    out.color = vec3<f32>(0.0, 1.0, 0.0); // Green
                } else {
                    out.color = vec3<f32>(0.0, 0.0, 1.0); // Blue
                }
                
                return out;
            }
        "#;

        // Fragment shader source
        let fragment_shader_source = r#"
            @fragment
            fn fs_main(@location(0) color: vec3<f32>) -> @location(0) vec4<f32> {
                return vec4<f32>(color, 1.0);
            }
        "#;

        // Create shaders
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Menu Triangle Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(vertex_shader_source.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main Menu Triangle Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(fragment_shader_source.into()),
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Menu Triangle Render Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Main Menu Triangle Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),
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

        // Create empty vertex buffer (we generate vertices in the shader)
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Main Menu Triangle Vertex Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        Ok(Self {
            render_pipeline,
            vertex_buffer,
        })
    }

    pub fn render(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Menu Triangle Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Triangle Render Pass"),
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
            render_pass.draw(0..3, 0..1); // Draw 3 vertices for the triangle
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}