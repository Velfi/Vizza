use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration};
use wgpu::util::DeviceExt;
use bytemuck;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlurFilter {
    pub enabled: bool,
    pub order: u32,
    pub radius: f32,
    pub sigma: f32,
}

impl Default for BlurFilter {
    fn default() -> Self {
        Self {
            enabled: false,
            order: 0,
            radius: 5.0,
            sigma: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingState {
    pub blur_filter: BlurFilter,
}

impl Default for PostProcessingState {
    fn default() -> Self {
        Self {
            blur_filter: BlurFilter::default(),
        }
    }
}

#[derive(Debug)]
pub struct PostProcessingResources {
    // Blur filter resources
    pub blur_pipeline: wgpu::RenderPipeline,
    pub blur_bind_group: wgpu::BindGroup,
    pub blur_params_buffer: wgpu::Buffer,
    
    // Intermediate textures for post-processing chain
    pub intermediate_texture: wgpu::Texture,
    pub intermediate_view: wgpu::TextureView,
    pub blur_sampler: wgpu::Sampler,
}

impl PostProcessingResources {
    pub fn new(
        device: &Arc<Device>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<Self, crate::error::SimulationError> {
        // Create blur parameters buffer
        let blur_params = [0.0f32, 0.0f32, 0.0f32, 0.0f32]; // radius, sigma, width, height
        let blur_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blur Parameters Buffer"),
            contents: bytemuck::cast_slice(&blur_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create intermediate texture for post-processing chain
        let intermediate_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Post Processing Intermediate Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let intermediate_view = intermediate_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create blur shader
        let blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blur Filter Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blur_filter.wgsl").into()),
        });

        // Create blur pipeline layout
        let blur_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blur Bind Group Layout"),
            entries: &[
                // Input texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blur Pipeline Layout"),
            bind_group_layouts: &[&blur_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create blur pipeline
        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blur Pipeline"),
            layout: Some(&blur_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &blur_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blur_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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
        });

        // Create blur sampler
        let blur_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blur Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create blur bind group (will be updated with actual textures)
        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur Bind Group"),
            layout: &blur_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&intermediate_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blur_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: blur_params_buffer.as_entire_binding(),
                },
            ],
        });

        Ok(Self {
            blur_pipeline,
            blur_bind_group,
            blur_params_buffer,
            intermediate_texture,
            intermediate_view,
            blur_sampler,
        })
    }

    pub fn update_blur_params(&self, queue: &Arc<Queue>, radius: f32, sigma: f32, width: u32, height: u32) {
        let params = [radius, sigma, width as f32, height as f32];
        queue.write_buffer(&self.blur_params_buffer, 0, bytemuck::cast_slice(&params));
    }

    pub fn resize(&mut self, device: &Arc<Device>, surface_config: &SurfaceConfiguration) -> Result<(), crate::error::SimulationError> {
        // Recreate intermediate texture
        self.intermediate_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Post Processing Intermediate Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.intermediate_view = self.intermediate_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(())
    }
} 