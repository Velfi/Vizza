use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};
use wgpu::util::DeviceExt;

use super::settings::Settings;
use crate::simulations::shared::lut_manager::LutManager;
use crate::simulations::shared::camera::Camera;

pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface_config: SurfaceConfiguration,
    width: u32,
    height: u32,
    settings: Settings,
    lut_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera: Camera,
}

impl Renderer {
    pub fn new(
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let settings = Settings::default();
        let lut_manager = LutManager::new();
        let default_lut_name = "MATPLOTLIB_bone_r";
        let lut_data = lut_manager.load_lut(default_lut_name).unwrap_or_else(|_| {
            println!("Warning: LUT '{}' not found, using default", default_lut_name);
            lut_manager.get_default_lut()
        });

        // Create LUT buffer (convert u8 to u32 for shader compatibility)
        let lut_data_u32: Vec<u32> = lut_data.into_bytes().iter().map(|&x| x as u32).collect();
        let lut_buffer = _device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create simulation data bind group layout
        let bind_group_layout = _device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                // Binding 0: Simulation data (UVPair array)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: LUT data
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: Simulation parameters
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

        // Create camera bind group layout  
        let camera_bind_group_layout = _device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Initialize camera
        let camera = Camera::new(_device, width as f32, height as f32)?;

        // Create pipeline layout with both bind group layouts
        let pipeline_layout = _device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create shader
        let shader = _device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline = _device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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

        Ok(Self {
            device: Arc::clone(_device),
            queue: Arc::clone(_queue),
            surface_config: surface_config.clone(),
            width,
            height,
            settings,
            lut_buffer,
            render_pipeline,
            bind_group_layout,
            camera_bind_group_layout,
            camera,
        })
    }

    pub fn update_settings(&mut self, settings: &Settings, _queue: &Arc<Queue>) {
        self.settings = settings.clone();
        // LUT management is now handled by the simulation manager
    }

    pub fn update_lut(&mut self, lut_data: &crate::simulations::shared::LutData, queue: &Arc<Queue>) {
        let lut_data_u32: Vec<u32> = lut_data.clone().into_bytes().iter().map(|&x| x as u32).collect();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
    }

    pub fn create_bind_group(
        &self,
        simulation_buffer: &wgpu::Buffer,
        params_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: simulation_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.lut_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    pub fn create_camera_bind_group(&self) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &self.camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera.buffer().as_entire_binding(),
                },
            ],
        })
    }

    pub fn render(
        &mut self,
        view: &TextureView,
        simulation_buffer: &wgpu::Buffer,
        params_buffer: &wgpu::Buffer,
    ) -> Result<(), wgpu::SurfaceError> {
        // Update camera data on GPU
        self.camera.upload_to_gpu(&self.queue);
        
        let bind_group = self.create_bind_group(simulation_buffer, params_buffer);
        let camera_bind_group = self.create_camera_bind_group();
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(1, &camera_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.surface_config = new_config.clone();
        self.width = new_config.width;
        self.height = new_config.height;
        
        // Update camera viewport
        self.camera.resize(new_config.width as f32, new_config.height as f32);
        
        Ok(())
    }

    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }
} 