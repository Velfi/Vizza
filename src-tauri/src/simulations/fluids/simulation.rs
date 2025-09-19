use crate::error::{SimulationError, SimulationResult};
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::gpu_utils::{resource_helpers, BindGroupBuilder, CommonBindGroupLayouts, ComputePipelineBuilder, ShaderManager};
use crate::simulations::shared::{AverageColorResources, ColorScheme, ColorSchemeManager, PostProcessingResources, PostProcessingState};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::renderer::Renderer;
use super::settings::Settings;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AdvectParams {
    dt: f32,
    dissipation: f32,
    grid_w: u32,
    grid_h: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GridParams {
    half_cell: f32,
    _pad0: [f32; 3],
    grid_w: u32,
    grid_h: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct JacobiParams {
    alpha: f32,
    beta: f32,
    grid_w: u32,
    grid_h: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DyeParams {
    dt: f32,
    decay: f32,
    grid_w: u32,
    grid_h: u32,
}

#[derive(Debug)]
pub struct FluidsModel {
    pub settings: Settings,
    pub renderer: Renderer,
    pub camera: Camera,
    pub lut_manager: Arc<ColorSchemeManager>,

    // Simulation resolution
    pub width: u32,
    pub height: u32,

    // Textures
    pub velocity_textures: crate::simulations::shared::ping_pong_textures::PingPongTextures,
    pub pressure_textures: crate::simulations::shared::ping_pong_textures::PingPongTextures,
    pub dye_textures: crate::simulations::shared::ping_pong_textures::PingPongTextures,

    // Sampler
    pub linear_sampler: wgpu::Sampler,

    // Uniform buffers
    pub advect_params: wgpu::Buffer,
    pub grid_params: wgpu::Buffer,
    pub jacobi_params: wgpu::Buffer,
    pub dye_params: wgpu::Buffer,

    // Pipelines and bind groups
    pub advect_velocity_pipeline: wgpu::ComputePipeline,
    pub advect_velocity_bind_group: wgpu::BindGroup,

    pub divergence_pipeline: wgpu::ComputePipeline,
    pub divergence_bind_group: wgpu::BindGroup,
    pub divergence_texture: wgpu::Texture,
    pub divergence_view: wgpu::TextureView,

    pub jacobi_pipeline: wgpu::ComputePipeline,
    pub jacobi_bind_group_a: wgpu::BindGroup,
    pub jacobi_bind_group_b: wgpu::BindGroup,

    pub projection_pipeline: wgpu::ComputePipeline,
    pub projection_bind_group: wgpu::BindGroup,

    pub advect_dye_pipeline: wgpu::ComputePipeline,
    pub advect_dye_bind_group: wgpu::BindGroup,

    // Seeding
    pub seed_dye_pipeline: wgpu::ComputePipeline,
    pub seed_velocity_pipeline: wgpu::ComputePipeline,
    pub seed_params: wgpu::Buffer,

    // Render target for display
    pub display_texture: wgpu::Texture,
    pub display_view: wgpu::TextureView,
    pub display_sampler: wgpu::Sampler,
    pub render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,

    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
    pub average_color_resources: AverageColorResources,
}

impl FluidsModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        app_settings: &crate::commands::AppSettings,
        lut_manager: &ColorSchemeManager,
    ) -> SimulationResult<Self> {
        // Camera over world [-1,1]
        let camera = Camera::new(device, surface_config.width as f32, surface_config.height as f32)?;

        // Compute sim resolution
        let sim_w = ((surface_config.width as f32) * settings.simulation_resolution_scale).max(256.0) as u32;
        let sim_h = ((surface_config.height as f32) * settings.simulation_resolution_scale).max(256.0) as u32;

        // Create ping-pong textures
        let velocity_textures = crate::simulations::shared::ping_pong_textures::PingPongTextures::new(
            device,
            sim_w,
            sim_h,
            wgpu::TextureFormat::Rg16Float,
            "Fluids Velocity",
        );
        let pressure_textures = crate::simulations::shared::ping_pong_textures::PingPongTextures::new(
            device,
            sim_w,
            sim_h,
            wgpu::TextureFormat::R16Float,
            "Fluids Pressure",
        );
        let dye_textures = crate::simulations::shared::ping_pong_textures::PingPongTextures::new(
            device,
            sim_w,
            sim_h,
            wgpu::TextureFormat::Rgba16Float,
            "Fluids Dye",
        );

        // Divergence texture
        let divergence_texture = resource_helpers::create_storage_texture(
            device,
            "Fluids Divergence",
            sim_w,
            sim_h,
            wgpu::TextureFormat::R16Float,
        );
        let divergence_view = divergence_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let linear_sampler = resource_helpers::create_linear_sampler(
            device,
            "Fluids Linear Sampler",
            app_settings.texture_filtering.into(),
        );

        // Uniform buffers
        let advect_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Advect Params"),
            contents: bytemuck::bytes_of(&AdvectParams { dt: settings.time_step, dissipation: 1.0, grid_w: sim_w, grid_h: sim_h }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let grid_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Params"),
            contents: bytemuck::bytes_of(&GridParams { half_cell: 0.5, _pad0: [0.0;3], grid_w: sim_w, grid_h: sim_h }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let jacobi_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Jacobi Params"),
            contents: bytemuck::bytes_of(&JacobiParams { alpha: -1.0, beta: 4.0, grid_w: sim_w, grid_h: sim_h }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let dye_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dye Params"),
            contents: bytemuck::bytes_of(&DyeParams { dt: settings.time_step, decay: settings.dye_decay, grid_w: sim_w, grid_h: sim_h }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Shaders
        let mut shader_manager = ShaderManager::new();
        let advect_shader = shader_manager.load_shader(device, "fluids_advect", super::shaders::ADVECT_SHADER);
        let divergence_shader = shader_manager.load_shader(device, "fluids_divergence", super::shaders::DIVERGENCE_SHADER);
        let jacobi_shader = shader_manager.load_shader(device, "fluids_jacobi", super::shaders::JACOBI_SHADER);
        let projection_shader = shader_manager.load_shader(device, "fluids_projection", super::shaders::PROJECTION_SHADER);
        let dye_advect_shader = shader_manager.load_shader(device, "fluids_dye_advect", super::shaders::DYE_ADVECT_SHADER);
        let seed_dye_shader = shader_manager.load_shader(device, "fluids_seed_dye", super::shaders::SEED_DYE_SHADER);
        let seed_velocity_shader = shader_manager.load_shader(device, "fluids_seed_velocity", super::shaders::SEED_VELOCITY_SHADER);

        // Bind group layouts
        let advect_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluids Advect BGL"),
            entries: &[
                resource_helpers::texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::texture_entry(1, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::storage_texture_entry(2, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::WriteOnly, wgpu::TextureFormat::Rgba16Float),
                resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::COMPUTE),
                resource_helpers::sampler_entry(4, wgpu::ShaderStages::COMPUTE, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let divergence_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluids Divergence BGL"),
            entries: &[
                resource_helpers::texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::storage_texture_entry(1, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::WriteOnly, wgpu::TextureFormat::R16Float),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                resource_helpers::sampler_entry(3, wgpu::ShaderStages::COMPUTE, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let jacobi_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluids Jacobi BGL"),
            entries: &[
                resource_helpers::texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::texture_entry(1, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::storage_texture_entry(2, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::WriteOnly, wgpu::TextureFormat::R16Float),
                resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::COMPUTE),
                resource_helpers::sampler_entry(4, wgpu::ShaderStages::COMPUTE, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let projection_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluids Projection BGL"),
            entries: &[
                resource_helpers::texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::texture_entry(1, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::storage_texture_entry(2, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::WriteOnly, wgpu::TextureFormat::Rg16Float),
                resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::COMPUTE),
                resource_helpers::sampler_entry(4, wgpu::ShaderStages::COMPUTE, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let dye_advect_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluids Dye Advect BGL"),
            entries: &[
                resource_helpers::texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::texture_entry(1, wgpu::ShaderStages::COMPUTE, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::storage_texture_entry(2, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::WriteOnly, wgpu::TextureFormat::Rgba16Float),
                resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::COMPUTE),
                resource_helpers::sampler_entry(4, wgpu::ShaderStages::COMPUTE, wgpu::SamplerBindingType::Filtering),
            ],
        });

        // Pipelines
        let advect_velocity_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(advect_shader)
            .with_bind_group_layouts(vec![advect_bgl.clone()])
            .with_label("Fluids Advect Velocity".to_string())
            .build();
        let divergence_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(divergence_shader)
            .with_bind_group_layouts(vec![divergence_bgl.clone()])
            .with_label("Fluids Divergence".to_string())
            .build();
        let jacobi_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(jacobi_shader)
            .with_bind_group_layouts(vec![jacobi_bgl.clone()])
            .with_label("Fluids Jacobi".to_string())
            .build();
        let projection_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(projection_shader)
            .with_bind_group_layouts(vec![projection_bgl.clone()])
            .with_label("Fluids Projection".to_string())
            .build();
        let advect_dye_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(dye_advect_shader)
            .with_bind_group_layouts(vec![dye_advect_bgl.clone()])
            .with_label("Fluids Advect Dye".to_string())
            .build();

        let seed_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Fluids Seed BGL"),
            entries: &[
                resource_helpers::storage_texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::ReadWrite, wgpu::TextureFormat::Rgba16Float),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
            ]
        });
        let seed_vel_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Fluids SeedVel BGL"),
            entries: &[
                resource_helpers::storage_texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::ReadWrite, wgpu::TextureFormat::Rg16Float),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
            ]
        });
        let seed_dye_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(seed_dye_shader)
            .with_bind_group_layouts(vec![seed_bgl.clone()])
            .with_label("Fluids Seed Dye".to_string())
            .build();
        let seed_velocity_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(seed_velocity_shader)
            .with_bind_group_layouts(vec![seed_vel_bgl.clone()])
            .with_label("Fluids Seed Velocity".to_string())
            .build();

        let seed_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fluids Seed Params"),
            contents: bytemuck::cast_slice(&[0.5f32, 0.5, settings.force_radius, settings.force_strength, sim_w as f32, sim_h as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind groups
        let advect_velocity_bind_group = BindGroupBuilder::new(device, &advect_bgl)
            .add_texture_view(0, velocity_textures.current_view())
            .add_texture_view(1, velocity_textures.current_view())
            .add_texture_view(2, velocity_textures.inactive_view())
            .add_buffer(3, &advect_params)
            .with_label("Fluids Advect Velocity BG".to_string())
            .build();

        let divergence_bind_group = BindGroupBuilder::new(device, &divergence_bgl)
            .add_texture_view(0, velocity_textures.current_view())
            .add_texture_view(1, &divergence_view)
            .add_buffer(2, &grid_params)
            .with_label("Fluids Divergence BG".to_string())
            .build();

        let jacobi_bind_group_a = BindGroupBuilder::new(device, &jacobi_bgl)
            .add_texture_view(0, pressure_textures.current_view())
            .add_texture_view(1, &divergence_view)
            .add_texture_view(2, pressure_textures.inactive_view())
            .add_buffer(3, &jacobi_params)
            .with_label("Fluids Jacobi BG A".to_string())
            .build();
        let jacobi_bind_group_b = BindGroupBuilder::new(device, &jacobi_bgl)
            .add_texture_view(0, pressure_textures.inactive_view())
            .add_texture_view(1, &divergence_view)
            .add_texture_view(2, pressure_textures.current_view())
            .add_buffer(3, &jacobi_params)
            .with_label("Fluids Jacobi BG B".to_string())
            .build();

        let projection_bind_group = BindGroupBuilder::new(device, &projection_bgl)
            .add_texture_view(0, pressure_textures.current_view())
            .add_texture_view(1, velocity_textures.current_view())
            .add_texture_view(2, velocity_textures.inactive_view())
            .add_buffer(3, &grid_params)
            .with_label("Fluids Projection BG".to_string())
            .build();

        let advect_dye_bind_group = BindGroupBuilder::new(device, &dye_advect_bgl)
            .add_texture_view(0, velocity_textures.current_view())
            .add_texture_view(1, dye_textures.current_view())
            .add_texture_view(2, dye_textures.inactive_view())
            .add_buffer(3, &dye_params)
            .with_label("Fluids Advect Dye BG".to_string())
            .build();

        // Display target: use existing shared infinite renderer to sample dye texture
        let display_texture = resource_helpers::create_storage_texture(
            device,
            "Fluids Display",
            surface_config.width,
            surface_config.height,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let display_sampler = resource_helpers::create_linear_sampler(
            device,
            "Fluids Display Sampler",
            app_settings.texture_filtering.into(),
        );

        // Use shared infinite render shader for texture sampling (fs_main_texture)
        let mut shader_mgr2 = ShaderManager::new();
        let render_shader = shader_mgr2.load_shader(
            device,
            "fluids_render_texture",
            crate::simulations::shared::INFINITE_RENDER_SHADER,
        );

        // Minimal layout for fs_main_texture: display_tex, display_sampler, render_params
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fluids Render BGL"),
            entries: &[
                resource_helpers::texture_entry(0, wgpu::ShaderStages::FRAGMENT, wgpu::TextureSampleType::Float { filterable: true }, wgpu::TextureViewDimension::D2),
                resource_helpers::sampler_entry(1, wgpu::ShaderStages::FRAGMENT, wgpu::SamplerBindingType::Filtering),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });

        // Render params buffer: filtering mode
        let filtering_mode: u32 = app_settings.texture_filtering.into();
        let render_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Fluids Render Params",
            &[filtering_mode, 0u32, 0u32, 0u32],
        );

        let camera_bind_group = BindGroupBuilder::new(device, &CommonBindGroupLayouts::new(device).camera)
            .add_buffer(0, camera.buffer())
            .with_label("Fluids Camera BG".to_string())
            .build();

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fluids Render BG"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, dye_textures.current_view()),
                resource_helpers::sampler_bind_entry(1, &display_sampler),
                resource_helpers::buffer_entry(2, &render_params_buffer),
            ],
        });

        // Create render pipeline that uses fs_main_texture
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fluids Render Pipeline Layout"),
            bind_group_layouts: &[
                &render_bind_group_layout,
                &CommonBindGroupLayouts::new(device).camera,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fluids Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main_texture"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let renderer = Renderer::new(device, queue, surface_config)?;

        Ok(Self {
            settings,
            renderer,
            camera,
            lut_manager: Arc::new(lut_manager.clone()),
            width: sim_w,
            height: sim_h,
            velocity_textures,
            pressure_textures,
            dye_textures,
            linear_sampler,
            advect_params,
            grid_params,
            jacobi_params,
            dye_params,
            advect_velocity_pipeline,
            advect_velocity_bind_group,
            divergence_pipeline,
            divergence_bind_group,
            divergence_texture,
            divergence_view,
            jacobi_pipeline,
            jacobi_bind_group_a,
            jacobi_bind_group_b,
            projection_pipeline,
            projection_bind_group,
            advect_dye_pipeline,
            advect_dye_bind_group,
            display_texture,
            display_view,
            display_sampler,
            render_bind_group_layout,
            render_bind_group,
            render_pipeline,
            seed_dye_pipeline,
            seed_velocity_pipeline,
            seed_params,
            post_processing_state: PostProcessingState::default(),
            post_processing_resources: PostProcessingResources::new(device, surface_config.format),
            average_color_resources: AverageColorResources::new(device, surface_config.format),
        })
    }

    fn dispatch_wh(&self) -> (u32, u32) {
        (self.width.div_ceil(16), self.height.div_ceil(16))
    }

    pub(crate) fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        // Update dt
        let adv = AdvectParams { dt: self.settings.time_step.min(delta_time.max(0.001)), dissipation: 1.0, grid_w: self.width, grid_h: self.height };
        queue.write_buffer(&self.advect_params, 0, bytemuck::bytes_of(&adv));
        let dye = DyeParams { dt: self.settings.time_step.min(delta_time.max(0.001)), decay: self.settings.dye_decay, grid_w: self.width, grid_h: self.height };
        queue.write_buffer(&self.dye_params, 0, bytemuck::bytes_of(&dye));

        let (wg_x, wg_y) = self.dispatch_wh();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Fluids Encoder") });
        {
            // Advect velocity (self-advection)
            {
                let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Fluids Advect Velocity"), timestamp_writes: None });
                c.set_pipeline(&self.advect_velocity_pipeline);
                c.set_bind_group(0, &self.advect_velocity_bind_group, &[]);
                c.dispatch_workgroups(wg_x, wg_y, 1);
            }
            // Compute divergence
            {
                let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Fluids Divergence"), timestamp_writes: None });
                c.set_pipeline(&self.divergence_pipeline);
                c.set_bind_group(0, &self.divergence_bind_group, &[]);
                c.dispatch_workgroups(wg_x, wg_y, 1);
            }
            // Jacobi pressure solve
            for i in 0..self.settings.pressure_iterations { let _ = i; {
                    let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Fluids Jacobi"), timestamp_writes: None });
                    c.set_pipeline(&self.jacobi_pipeline);
                    c.set_bind_group(0, if (i % 2)==0 { &self.jacobi_bind_group_a } else { &self.jacobi_bind_group_b }, &[]);
                    c.dispatch_workgroups(wg_x, wg_y, 1);
                }
            }
            // Projection
            {
                let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Fluids Projection"), timestamp_writes: None });
                c.set_pipeline(&self.projection_pipeline);
                c.set_bind_group(0, &self.projection_bind_group, &[]);
                c.dispatch_workgroups(wg_x, wg_y, 1);
            }
            // Advect dye with velocity
            {
                let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Fluids Advect Dye"), timestamp_writes: None });
                c.set_pipeline(&self.advect_dye_pipeline);
                c.set_bind_group(0, &self.advect_dye_bind_group, &[]);
                c.dispatch_workgroups(wg_x, wg_y, 1);
            }
        }
        queue.submit(std::iter::once(encoder.finish()));

        // Swap ping-pong textures for next frame
        self.velocity_textures.swap();
        self.pressure_textures.swap();
        self.dye_textures.swap();

        // Render dye texture with infinite tiling
        // Update camera GPU
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        let camera_bg = BindGroupBuilder::new(device, &CommonBindGroupLayouts::new(device).camera)
            .add_buffer(0, self.camera.buffer())
            .with_label("Fluids Camera BG Render".to_string())
            .build();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Fluids Render Encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Fluids Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.render_bind_group, &[]);
            pass.set_bind_group(1, &camera_bg, &[]);
            let tiles = 5u32; // simple fixed tiling for now
            pass.draw(0..6, 0..tiles*tiles);
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

impl crate::simulations::traits::Simulation for FluidsModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        self.render_frame(device, queue, surface_view, delta_time)
    }

    fn render_frame_paused(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        _surface_view: &TextureView,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn resize(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        _new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn update_setting(
        &mut self,
        _setting_name: &str,
        _value: Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn update_state(
        &mut self,
        _state_name: &str,
        _value: Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn get_settings(&self) -> Value { serde_json::to_value(&self.settings).unwrap_or_else(|_| serde_json::json!({})) }
    fn get_state(&self) -> Value { serde_json::json!({}) }
    fn handle_mouse_interaction(&mut self, x: f32, y: f32, _mouse_button: u32, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
        // x,y in world [-1,1] -> uv [0,1]
        let uv = ((x + 1.0) * 0.5, (y + 1.0) * 0.5);
        // Update seed params
        let seed = [uv.0 as f32, uv.1 as f32, self.settings.force_radius, self.settings.force_strength, self.width as f32, self.height as f32];
        queue.write_buffer(&self.seed_params, 0, bytemuck::cast_slice(&seed));

        let (wg_x, wg_y) = self.dispatch_wh();
        let seed_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Fluids Seed BGLTmp"),
            entries: &[
                resource_helpers::storage_texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::ReadWrite, wgpu::TextureFormat::Rgba16Float),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
            ]
        });
        let seed_vel_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Fluids SeedVel BGLTmp"),
            entries: &[
                resource_helpers::storage_texture_entry(0, wgpu::ShaderStages::COMPUTE, wgpu::StorageTextureAccess::ReadWrite, wgpu::TextureFormat::Rg16Float),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
            ]
        });
        let seed_dye_bg = BindGroupBuilder::new(device, &seed_bgl)
            .add_texture_view(0, self.dye_textures.current_view())
            .add_buffer(1, &self.seed_params)
            .with_label("Fluids Seed Dye BG".to_string())
            .build();
        let seed_vel_bg = BindGroupBuilder::new(device, &seed_vel_bgl)
            .add_texture_view(0, self.velocity_textures.current_view())
            .add_buffer(1, &self.seed_params)
            .with_label("Fluids Seed Vel BG".to_string())
            .build();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Fluids Seed Encoder")});
        {
            let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{ label: Some("Seed Dye"), timestamp_writes: None});
            c.set_pipeline(&self.seed_dye_pipeline);
            c.set_bind_group(0, &seed_dye_bg, &[]);
            c.dispatch_workgroups(wg_x, wg_y, 1);
        }
        {
            let mut c = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{ label: Some("Seed Vel"), timestamp_writes: None});
            c.set_pipeline(&self.seed_velocity_pipeline);
            c.set_bind_group(0, &seed_vel_bg, &[]);
            c.dispatch_workgroups(wg_x, wg_y, 1);
        }
        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
    fn handle_mouse_release(&mut self, _mouse_button: u32, _queue: &Arc<Queue>) -> SimulationResult<()> { Ok(()) }
    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> { Err(SimulationError::InvalidParameter("Preset not implemented".into())) }
    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> { Err(SimulationError::InvalidParameter("Preset not implemented".into())) }
    fn apply_settings(&mut self, _settings: serde_json::Value, _device: &Arc<Device>, _queue: &Arc<Queue>) -> SimulationResult<()> { Ok(()) }
    fn reset_runtime_state(&mut self, _device: &Arc<Device>, _queue: &Arc<Queue>) -> SimulationResult<()> { Ok(()) }
    fn randomize_settings(&mut self, _device: &Arc<Device>, _queue: &Arc<Queue>) -> SimulationResult<()> { Ok(()) }
    fn update_color_scheme(&mut self, _color_scheme: &ColorScheme, _device: &Arc<Device>, _queue: &Arc<Queue>) -> SimulationResult<()> { Ok(()) }
}

