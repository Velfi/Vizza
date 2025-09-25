use super::settings::{
    BackgroundColorMode, ForegroundColorMode, NoiseType, Settings, VectorFieldType,
};
use super::shaders::{
    BACKGROUND_RENDER_SHADER, FLOW_VECTOR_COMPUTE_SHADER, PARTICLE_RENDER_SHADER,
    PARTICLE_UPDATE_SHADER, RENDER_INFINITE_SHADER, SHAPE_DRAWING_SHADER,
    TRAIL_DECAY_DIFFUSION_SHADER, TRAIL_RENDER_SHADER,
};
use crate::commands::AppSettings;
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::{
    AverageColorResources, BindGroupBuilder, ColorSchemeManager, CommonBindGroupLayouts,
    ComputePipelineBuilder, PostProcessingResources, PostProcessingState, ShaderManager,
};
use crate::simulations::traits::Simulation;
use bytemuck::{Pod, Zeroable};

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

thread_local! {
    static RNG: RefCell<StdRng> = {
        let mut thread_rng = rand::rng();
        RefCell::new(StdRng::from_rng(&mut thread_rng))
    };
}

pub(crate) const DEFAULT_FLOW_FIELD_RESOLUTION: u32 = 128;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Default)]
pub struct Particle {
    pub position: [f32; 2],
    pub age: f32,
    pub lut_index: u32,
    pub is_alive: u32,   // 0=dead, 1=alive
    pub spawn_type: u32, // 0=autospawn, 1=brush
    pub _pad0: u32,      // Padding to keep 32-byte stride aligned with WGSL
    pub _pad1: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct FlowVector {
    pub position: [f32; 2],
    pub direction: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct FlowVectorParams {
    pub grid_size: u32,
    pub vector_field_type: u32, // 0=Noise, 1=Image
    pub noise_type: u32,
    pub noise_scale: f32,
    pub noise_x: f32,
    pub noise_y: f32,
    pub noise_seed: u32,
    pub time: f32,
    pub noise_dt_multiplier: f32,
    pub vector_magnitude: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Default)]
pub struct SpawnControl {
    pub autospawn_allowed: u32,
    pub brush_allowed: u32,
    pub autospawn_count: u32,
    pub brush_count: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Default)]
pub struct SimParams {
    pub autospawn_pool_size: u32, // Size of autospawn pool
    pub autospawn_rate: u32,      // Particles per second for autospawn
    pub brush_pool_size: u32,     // Size of brush pool
    pub brush_spawn_rate: u32,    // Particles per second when cursor is active
    pub cursor_size: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub display_mode: u32, // 0=Age, 1=Random, 2=Direction
    pub flow_field_resolution: u32,
    pub height: f32,
    pub mouse_button_down: u32, // 0=not held, 1=left click held, 2=right click held
    pub noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    pub noise_scale: f32,
    pub noise_seed: u32,
    pub noise_x: f32,
    pub noise_y: f32,
    pub particle_autospawn: u32, // 0=disabled, 1=enabled
    pub particle_lifetime: f32,
    pub particle_shape: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond
    pub particle_size: u32,  // Particle size in pixels
    pub particle_speed: f32,
    pub screen_height: u32, // Screen height in pixels
    pub screen_width: u32,  // Screen width in pixels
    pub time: f32,
    pub total_pool_size: u32, // Total number of particles (autospawn + brush)
    pub trail_decay_rate: f32,
    pub trail_deposition_rate: f32,
    pub trail_diffusion_rate: f32,
    pub trail_map_height: u32,
    pub trail_map_width: u32,
    pub trail_wash_out_rate: f32,
    pub vector_magnitude: f32,
    pub width: f32,
    pub delta_time: f32,
    pub _padding_1: u32,
    pub _padding_2: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ShapeParams {
    pub center_x: f32,
    pub center_y: f32,
    pub size: f32,
    pub shape_type: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond, 5=Line
    pub color: [f32; 4],
    pub intensity: f32,
    pub antialiasing_width: f32, // Width of antialiasing edge in pixels
    pub rotation: f32,           // Rotation angle in radians
    pub aspect_ratio: f32,       // For ellipses and rectangles
    pub trail_map_width: u32,
    pub trail_map_height: u32,
    pub _padding_0: u32,
    pub _padding_1: u32,
}

#[derive(Debug)]
pub struct FlowModel {
    pub settings: Settings,
    pub state: super::state::State,

    // GPU utilities
    pub shader_manager: ShaderManager,
    pub common_layouts: CommonBindGroupLayouts,

    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub flow_vector_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub lut_buffer: wgpu::Buffer,
    pub background_color_buffer: wgpu::Buffer,
    pub spawn_control_buffer: wgpu::Buffer,

    // Trail system
    pub trail_texture: wgpu::Texture,
    pub trail_texture_view: wgpu::TextureView,
    pub trail_sampler: wgpu::Sampler,

    // Particle update pipeline
    pub particle_update_pipeline: wgpu::ComputePipeline,
    pub particle_update_bind_group: wgpu::BindGroup,

    // Trail decay and diffusion pipeline
    pub trail_decay_diffusion_pipeline: wgpu::ComputePipeline,
    pub trail_decay_diffusion_bind_group: wgpu::BindGroup,

    // Particle render pipeline
    pub particle_render_pipeline: wgpu::RenderPipeline,
    pub particle_render_bind_group: wgpu::BindGroup,
    pub camera_bind_group: wgpu::BindGroup,

    // Trail render pipeline
    pub trail_render_pipeline: wgpu::RenderPipeline,
    pub trail_display_render_pipeline: wgpu::RenderPipeline,
    pub trail_render_bind_group: wgpu::BindGroup,

    // Background render pipeline
    pub background_render_pipeline: wgpu::RenderPipeline,
    pub background_display_render_pipeline: wgpu::RenderPipeline,
    pub background_render_bind_group: wgpu::BindGroup,

    // Particle render pipeline
    pub particle_display_render_pipeline: wgpu::RenderPipeline,

    // Runtime state
    pub camera: Camera,
    pub color_scheme_manager: Arc<ColorSchemeManager>,
    pub time: f32,
    pub delta_time: f32,
    pub autospawn_accumulator: f32,
    pub brush_spawn_accumulator: f32,
    pub noise_dt_multiplier: f32, // Multiplier for time when calculating noise position
    pub particles: Vec<Particle>,
    pub flow_vectors: Vec<FlowVector>,
    pub gui_visible: bool,
    pub trail_map_width: u32,
    pub trail_map_height: u32,
    pub trail_map_filtering: super::settings::TrailMapFiltering,

    // Particle pool management
    pub autospawn_pool_size: u32,
    pub brush_pool_size: u32,
    pub total_pool_size: u32,

    // Mouse interaction state
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,
    pub cursor_size: f32,
    pub mouse_button_down: u32, // 0 = not held, 1 = left click held, 2 = right click held

    // Add display textures for infinite compositing
    pub display_texture: wgpu::Texture, // Single mipmap for rendering
    pub display_view: wgpu::TextureView,
    pub display_mipmap_texture: wgpu::Texture, // Multiple mipmaps for sampling
    pub display_mipmap_view: wgpu::TextureView,
    pub display_sampler: wgpu::Sampler,
    pub render_infinite_bind_group: wgpu::BindGroup,
    pub render_infinite_pipeline: wgpu::RenderPipeline,

    // Average color calculation for infinite rendering
    pub average_color_resources: AverageColorResources,
    pub average_color_uniform_buffer: wgpu::Buffer,

    // Post-processing state and resources
    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
    pub app_settings: AppSettings,

    // GPU flow vector generation
    pub flow_vector_compute_pipeline: wgpu::ComputePipeline,
    pub flow_vector_compute_bind_group: wgpu::BindGroup,
    pub flow_vector_params_buffer: wgpu::Buffer,
    pub flow_field_resolution: u32,

    // Image-based vector field support
    pub vector_field_image_texture: Option<wgpu::Texture>,
    pub vector_field_image_view: Option<wgpu::TextureView>,
    pub vector_field_image_sampler: Option<wgpu::Sampler>,
    pub vector_field_image_original: Option<image::DynamicImage>,
    pub vector_field_image_needs_upload: bool,

    // Default texture for when no image is loaded
    pub default_vector_field_texture: wgpu::Texture,
    pub default_vector_field_view: wgpu::TextureView,
    pub default_vector_field_sampler: wgpu::Sampler,

    // Shape drawing system
    pub shape_drawing_pipeline: wgpu::ComputePipeline,
    pub shape_drawing_bind_group: wgpu::BindGroup,
    pub shape_params_buffer: wgpu::Buffer,
    pub shape_drawing_enabled: bool,

    // Webcam capture for image-based vector fields
    pub webcam_capture: crate::simulations::shared::WebcamCapture,
}

impl FlowModel {
    // Calculate how many tiles we need based on zoom level
    fn calculate_tile_count(&self) -> u32 {
        // At zoom 1.0, we need at least 5x5 tiles
        // As zoom decreases (zooming out), we need more tiles
        // Each tile covers 2.0 world units, so we need enough tiles to cover the visible area
        let visible_world_size = 2.0 / self.camera.zoom; // World size visible on screen
        let tiles_needed = (visible_world_size / 2.0).ceil() as u32 + 6; // +6 for extra padding at extreme zoom levels
        let min_tiles = if self.camera.zoom < 0.1 { 7 } else { 5 }; // More tiles needed at extreme zoom out
        // Allow more tiles for proper infinite tiling, but cap at reasonable limit
        tiles_needed.max(min_tiles).min(1024) // Cap at 200x200 for performance (40,000 instances max)
    }

    // Generate flow direction using the noise crate

    // Regenerate flow vectors with current settings
    pub fn regenerate_flow_vectors(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Update params with current time
        self.update_flow_vector_params(queue);

        // Dispatch compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flow Vector Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Flow Vector Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.flow_vector_compute_pipeline);
            compute_pass.set_bind_group(0, &self.flow_vector_compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(128u32.div_ceil(16), 128u32.div_ceil(16), 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Update sim params with new flow field resolution
        let sim_params = self.create_runtime_sim_params();

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        Ok(())
    }

    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        app_settings: &AppSettings,
        color_scheme_manager: &ColorSchemeManager,
    ) -> Result<Self, crate::error::SimulationError> {
        // Initialize camera
        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )?;

        // Initialize particles with random positions and ages
        // Simplified particle system with separate pools
        let autospawn_pool_size = settings.total_pool_size / 2; // Half for autospawn
        let brush_pool_size = settings.total_pool_size / 2; // Half for brush
        let total_pool_size = autospawn_pool_size + brush_pool_size;
        let mut particles = Vec::with_capacity(total_pool_size as usize);

        // Initialize autospawn pool
        RNG.with(|_rng| {
            for _ in 0..autospawn_pool_size {
                let particle = Particle {
                    position: [0.0, 0.0],            // Will be set when spawned
                    age: settings.particle_lifetime, // Start dead
                    lut_index: 0,                    // No color
                    is_alive: 0,                     // Dead particles are inactive
                    spawn_type: 0,                   // Autospawn particles
                    _pad0: 0,
                    _pad1: 0,
                };
                particles.push(particle);
            }

            // Initialize brush pool with dead particles and staggered spawn times
            for _ in 0..brush_pool_size {
                let particle = Particle {
                    position: [0.0, 0.0],            // Will be set when spawned
                    age: settings.particle_lifetime, // Start dead
                    lut_index: 0,                    // No color
                    is_alive: 0,                     // Dead particles are inactive
                    spawn_type: 1,                   // Brush particles
                    _pad0: 0,
                    _pad1: 0,
                };
                particles.push(particle);
            }
        });

        // Initialize empty flow vectors (will be generated by GPU)
        let flow_vectors = Vec::new();

        // Create GPU buffers
        let particle_buffer = resource_helpers::create_storage_buffer_with_data(
            device,
            "Particle Buffer",
            &particles,
        );

        let flow_vector_buffer = resource_helpers::create_storage_buffer(
            device,
            "Flow Vector Buffer",
            std::mem::size_of::<FlowVector>() as u64
                * (DEFAULT_FLOW_FIELD_RESOLUTION * DEFAULT_FLOW_FIELD_RESOLUTION) as u64,
            false,
        );

        let sim_params = Self::create_default_sim_params_static(
            &settings,
            surface_config,
            ForegroundColorMode::Age,
            autospawn_pool_size,
            brush_pool_size,
        );

        let sim_params_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Sim Params Buffer",
            &[sim_params],
        );

        let lut_data = color_scheme_manager
            .get("MATPLOTLIB_terrain")
            .expect("MATPLOTLIB_terrain color scheme should exist");
        let lut_buffer = resource_helpers::create_storage_buffer_with_data(
            device,
            "Color Scheme Buffer",
            &lut_data.to_u32_buffer(),
        );

        // Create spawn control buffer
        let spawn_control_init = SpawnControl {
            autospawn_allowed: 0,
            brush_allowed: 0,
            autospawn_count: 0,
            brush_count: 0,
        };
        let spawn_control_buffer = resource_helpers::create_storage_buffer_with_data(
            device,
            "Spawn Control Buffer",
            &[spawn_control_init],
        );

        // Create background color buffer (will be updated based on background setting)
        let background_color_buffer = resource_helpers::create_uniform_buffer_with_data(
            device,
            "Background Color Buffer",
            &[0.0f32, 0.0f32, 0.0f32, 1.0f32], // Temporary black, will be updated
        );

        // Create trail texture
        let trail_texture = resource_helpers::create_storage_texture(
            device,
            "Trail Texture",
            surface_config.width,
            surface_config.height,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let trail_texture_view = trail_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Trail Texture View"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });

        let trail_sampler = resource_helpers::create_linear_sampler(
            device,
            "Trail Sampler",
            app_settings.texture_filtering.into(),
        );

        // Initialize GPU utilities
        let mut shader_manager = ShaderManager::new();
        let common_layouts = CommonBindGroupLayouts::new(device);

        // Create particle update pipeline using GPU utilities
        let particle_update_shader =
            shader_manager.load_shader(device, "flow_particle_update", PARTICLE_UPDATE_SHADER);

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                    resource_helpers::storage_texture_entry(
                        3,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::StorageTextureAccess::ReadWrite,
                        wgpu::TextureFormat::Rgba8Unorm,
                    ),
                    resource_helpers::storage_buffer_entry(4, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::storage_buffer_entry(5, wgpu::ShaderStages::COMPUTE, false),
                ],
            });

        let particle_update_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(particle_update_shader)
            .with_bind_group_layouts(vec![compute_bind_group_layout.clone()])
            .with_label("Flow Particle Update Pipeline".to_string())
            .build();

        let particle_update_bind_group = BindGroupBuilder::new(device, &compute_bind_group_layout)
            .add_buffer(0, &particle_buffer)
            .add_buffer(1, &flow_vector_buffer)
            .add_buffer(2, &sim_params_buffer)
            .add_texture_view(3, &trail_texture_view)
            .add_buffer(4, &lut_buffer)
            .add_buffer(5, &spawn_control_buffer)
            .with_label("Particle Update Bind Group".to_string())
            .build();

        // Create trail decay and diffusion pipeline
        let trail_decay_diffusion_shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Trail Decay Diffusion Shader"),
                source: wgpu::ShaderSource::Wgsl(TRAIL_DECAY_DIFFUSION_SHADER.into()),
            });

        let trail_update_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Trail Update Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::COMPUTE),
                    resource_helpers::storage_texture_entry(
                        1,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::StorageTextureAccess::ReadWrite,
                        wgpu::TextureFormat::Rgba8Unorm,
                    ),
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, true),
                ],
            });

        let trail_decay_diffusion_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Flow Trail Decay Diffusion Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Trail Decay Diffusion Pipeline Layout"),
                        bind_group_layouts: &[&trail_update_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &trail_decay_diffusion_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        let trail_decay_diffusion_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Trail Decay Diffusion Bind Group"),
                layout: &trail_update_bind_group_layout,
                entries: &[
                    resource_helpers::buffer_entry(0, &sim_params_buffer),
                    resource_helpers::texture_view_entry(1, &trail_texture_view),
                    resource_helpers::buffer_entry(2, &flow_vector_buffer),
                ],
            });

        // Create particle render pipeline
        let particle_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Particle Render Shader"),
            source: wgpu::ShaderSource::Wgsl(PARTICLE_RENDER_SHADER.into()),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(
                        0,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        true,
                    ),
                    resource_helpers::uniform_buffer_entry(
                        1,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ),
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::FRAGMENT, true),
                ],
            });

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                )],
            });

        let particle_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Particle Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Particle Render Pipeline Layout"),
                        bind_group_layouts: &[&render_bind_group_layout, &camera_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &particle_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        let particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &particle_buffer),
                resource_helpers::buffer_entry(1, &sim_params_buffer),
                resource_helpers::buffer_entry(2, &lut_buffer),
            ],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, camera.buffer())],
        });

        // Create trail render pipeline
        let trail_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Trail Render Shader"),
            source: wgpu::ShaderSource::Wgsl(TRAIL_RENDER_SHADER.into()),
        });

        let trail_render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Trail Render Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(
                        0,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ),
                    resource_helpers::storage_texture_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::StorageTextureAccess::ReadOnly,
                        wgpu::TextureFormat::Rgba8Unorm,
                    ),
                ],
            });

        let trail_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Trail Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Trail Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &trail_render_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &trail_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &trail_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        let trail_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Trail Render Bind Group"),
            layout: &trail_render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &sim_params_buffer),
                resource_helpers::texture_view_entry(1, &trail_texture_view),
            ],
        });

        // Create background render pipeline
        let background_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Background Render Shader"),
            source: wgpu::ShaderSource::Wgsl(BACKGROUND_RENDER_SHADER.into()),
        });

        let background_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Background Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(
                        0,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        true,
                    ),
                    resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::FRAGMENT, true),
                    resource_helpers::uniform_buffer_entry(
                        2,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ),
                    resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        let background_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Background Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Background Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &background_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &background_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &background_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        let background_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Render Bind Group"),
            layout: &background_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &flow_vector_buffer),
                resource_helpers::buffer_entry(1, &lut_buffer),
                resource_helpers::buffer_entry(2, &sim_params_buffer),
                resource_helpers::buffer_entry(3, &background_color_buffer),
            ],
        });

        // Create background render pipeline for display texture (Rgba8Unorm format)
        let background_display_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Background Display Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Background Display Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &background_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &background_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &background_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        // Create trail render pipeline for display texture (Rgba8Unorm format)
        let trail_display_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Trail Display Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Trail Display Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &trail_render_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &trail_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &trail_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        // Create particle render pipeline for display texture (Rgba8Unorm format)
        let particle_display_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Particle Display Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Particle Display Render Pipeline Layout"),
                        bind_group_layouts: &[&render_bind_group_layout, &camera_bind_group_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                vertex: wgpu::VertexState {
                    module: &particle_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        // Create display texture for rendering and sampling
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Flow Display Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Use the same texture for both rendering and sampling (no mipmaps for now)
        let display_mipmap_texture = display_texture.clone();
        let display_mipmap_view = display_view.clone();
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Flow Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: app_settings.texture_filtering.into(),
            min_filter: app_settings.texture_filtering.into(),
            mipmap_filter: app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // No explicit filtering-mode uniform needed; sampler already uses app setting

        // Create infinite render pipeline for truly infinite tiling
        let render_infinite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Render Infinite Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_INFINITE_SHADER.into()),
        });

        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Flow Render Infinite Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        let render_infinite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Flow Render Infinite Pipeline Layout"),
                bind_group_layouts: &[
                    &render_infinite_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Create average color uniform buffer for infinite render shader
        let average_color_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Average Color Uniform Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // Initialize with black
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Render Infinite Bind Group"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &display_mipmap_view),
                resource_helpers::sampler_bind_entry(1, &display_sampler),
                resource_helpers::buffer_entry(2, &average_color_uniform_buffer),
                resource_helpers::buffer_entry(3, &background_color_buffer),
            ],
        });

        let render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Render Infinite Pipeline"),
                layout: Some(&render_infinite_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &render_infinite_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &render_infinite_shader,
                    entry_point: Some("fs_main_texture"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        // Create average color calculation resources
        let average_color_resources =
            AverageColorResources::new(device, &display_texture, &display_view, "Flow");

        // Create GPU flow vector generation resources
        let flow_vector_compute_shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Flow Vector Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                    FLOW_VECTOR_COMPUTE_SHADER,
                )),
            });

        let flow_vector_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Flow Vector Compute Pipeline"),
                layout: None,
                module: &flow_vector_compute_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Create flow vector params buffer
        let flow_vector_params = FlowVectorParams {
            grid_size: 128,
            vector_field_type: match settings.vector_field_type {
                VectorFieldType::Noise => 0,
                VectorFieldType::Image => 1,
            },
            noise_type: 0, // Will be set based on settings
            noise_scale: settings.noise_scale as f32,
            noise_x: settings.noise_x as f32,
            noise_y: settings.noise_y as f32,
            noise_seed: settings.noise_seed,
            time: 0.0,
            noise_dt_multiplier: settings.noise_dt_multiplier,
            vector_magnitude: settings.vector_magnitude,
        };

        let flow_vector_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flow Vector Params Buffer"),
                contents: bytemuck::cast_slice(&[flow_vector_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create a default 1x1 white texture for when no image is loaded
        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Vector Field Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let default_texture_view =
            default_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Default Vector Field Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Initialize with white pixel (value 255 = angle 2π)
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &default_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8], // White pixel
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        // Create flow vector compute bind group
        let flow_vector_compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Vector Compute Bind Group"),
            layout: &flow_vector_compute_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::buffer_entry(0, &flow_vector_buffer),
                resource_helpers::buffer_entry(1, &flow_vector_params_buffer),
                resource_helpers::texture_view_entry(2, &default_texture_view),
                resource_helpers::sampler_bind_entry(3, &default_sampler),
            ],
        });

        // Create shape drawing pipeline and resources
        let shape_drawing_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shape Drawing Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHAPE_DRAWING_SHADER)),
        });

        let shape_drawing_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Shape Drawing Pipeline"),
                layout: None,
                module: &shape_drawing_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Create shape params buffer
        let shape_params = ShapeParams {
            center_x: 0.0,
            center_y: 0.0,
            size: 0.1,
            shape_type: 0, // Circle
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 1.0,
            antialiasing_width: 2.0,
            rotation: 0.0,
            aspect_ratio: 1.0,
            trail_map_width: surface_config.width,
            trail_map_height: surface_config.height,
            _padding_0: 0,
            _padding_1: 0,
        };

        let shape_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shape Params Buffer"),
            contents: bytemuck::cast_slice(&[shape_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create shape drawing bind group
        let shape_drawing_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shape Drawing Bind Group"),
            layout: &shape_drawing_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::texture_view_entry(0, &trail_texture_view),
                resource_helpers::buffer_entry(1, &shape_params_buffer),
            ],
        });

        // Use the same camera for both offscreen and infinite rendering

        // Create the FlowModel instance
        let flow_model = Self {
            settings: settings.clone(),
            state: super::state::State {
                time: 0.0,
                delta_time: 0.016,
                autospawn_accumulator: 0.0,
                brush_spawn_accumulator: 0.0,
                noise_dt_multiplier: settings.noise_dt_multiplier,
                gui_visible: true,
                trail_map_width: surface_config.width,
                trail_map_height: surface_config.height,
                background_color_mode: BackgroundColorMode::ColorScheme,
                current_color_scheme: "MATPLOTLIB_terrain".to_string(),
                color_scheme_reversed: false,
                show_particles: true,
                foreground_color_mode: settings.foreground_color_mode,
                trail_map_filtering: super::settings::TrailMapFiltering::Nearest,
                autospawn_pool_size,
                brush_pool_size,
                total_pool_size,
                cursor_world_x: 0.0,
                cursor_world_y: 0.0,
                cursor_size: 0.33,
                mouse_button_down: 0,
                flow_field_resolution: DEFAULT_FLOW_FIELD_RESOLUTION,
                shape_drawing_enabled: false,
                camera_position: [0.0, 0.0],
                camera_zoom: 1.0,
                simulation_time: 0.0,
                is_running: true,
            },
            shader_manager,
            common_layouts,
            particle_buffer,
            flow_vector_buffer,
            sim_params_buffer,
            lut_buffer,
            background_color_buffer,
            spawn_control_buffer,

            trail_texture,
            trail_texture_view,
            trail_sampler,
            particle_update_pipeline,
            particle_update_bind_group,
            trail_decay_diffusion_pipeline,
            trail_decay_diffusion_bind_group,
            particle_render_pipeline,
            particle_render_bind_group,
            camera_bind_group,

            camera,
            color_scheme_manager: Arc::new(color_scheme_manager.clone()),
            time: 0.0,
            delta_time: 0.016,
            autospawn_accumulator: 0.0,
            brush_spawn_accumulator: 0.0,
            noise_dt_multiplier: settings.noise_dt_multiplier,
            particles,
            flow_vectors,
            gui_visible: true,
            trail_map_width: surface_config.width,
            trail_map_height: surface_config.height,
            trail_map_filtering: super::settings::TrailMapFiltering::Nearest,
            trail_render_pipeline,
            trail_display_render_pipeline,
            trail_render_bind_group,
            background_render_pipeline,
            background_display_render_pipeline,
            background_render_bind_group,
            particle_display_render_pipeline,

            // Initialize mouse interaction state
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
            cursor_size: 0.33,
            mouse_button_down: 0,

            display_texture,
            display_view,
            display_mipmap_texture,
            display_mipmap_view,
            display_sampler,
            render_infinite_bind_group,
            render_infinite_pipeline,
            average_color_resources,
            average_color_uniform_buffer,
            post_processing_state: PostProcessingState::default(),
            post_processing_resources: PostProcessingResources::new(device, surface_config)?,
            app_settings: app_settings.clone(),

            // GPU flow vector generation
            flow_vector_compute_pipeline,
            flow_vector_compute_bind_group,
            flow_vector_params_buffer,
            flow_field_resolution: DEFAULT_FLOW_FIELD_RESOLUTION,

            // Image-based vector field support
            vector_field_image_texture: None,
            vector_field_image_view: None,
            vector_field_image_sampler: None,
            vector_field_image_original: None,
            vector_field_image_needs_upload: false,

            // Default texture for when no image is loaded
            default_vector_field_texture: default_texture,
            default_vector_field_view: default_texture_view,
            default_vector_field_sampler: default_sampler,

            // Shape drawing system
            shape_drawing_pipeline,
            shape_drawing_bind_group,
            shape_params_buffer,
            shape_drawing_enabled: false,

            // Particle pool management
            autospawn_pool_size,
            brush_pool_size,
            total_pool_size,

            // Webcam capture
            webcam_capture: Default::default(),
        };

        // Update background color buffer to reflect the default white background
        flow_model.update_background_color(queue);

        Ok(flow_model)
    }

    /// Start webcam capture for Flow (image-based vector field)
    pub fn start_webcam_capture(
        &mut self,
        device_index: i32,
    ) -> crate::error::SimulationResult<()> {
        self.webcam_capture
            .set_target_dimensions(self.trail_map_width, self.trail_map_height);
        self.webcam_capture.start_capture(device_index)
    }

    /// Stop webcam capture
    pub fn stop_webcam_capture(&mut self) {
        self.webcam_capture.stop_capture();
    }

    /// List available webcam device indices
    pub fn get_available_webcam_devices(&self) -> Vec<i32> {
        crate::simulations::shared::WebcamCapture::get_available_devices()
    }

    /// Upload latest webcam frame into the vector field image texture
    pub fn update_vector_field_from_webcam(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        if let Some(frame_data) = self.webcam_capture.get_latest_frame_data() {
            let target_w = self.trail_map_width;
            let target_h = self.trail_map_height;

            let mut processed = frame_data;

            // Guard against race during resize: skip mismatched frames
            let expected_len = (target_w as usize) * (target_h as usize);
            if processed.len() != expected_len {
                tracing::debug!(
                    "Skipping webcam frame due to size mismatch: got {} bytes, expected {} ({}x{})",
                    processed.len(),
                    expected_len,
                    target_w,
                    target_h
                );
                return Ok(());
            }

            if self.settings.image_mirror_horizontal {
                let w = target_w as usize;
                let h = target_h as usize;
                for y in 0..h {
                    processed[y * w..(y + 1) * w].reverse();
                }
            }

            if self.settings.image_mirror_vertical {
                let w = target_w as usize;
                let h = target_h as usize;
                for y_top in 0..(h / 2) {
                    let y_bottom = h - 1 - y_top;
                    let top_start = y_top * w;
                    let bottom_start = y_bottom * w;
                    for x in 0..w {
                        processed.swap(top_start + x, bottom_start + x);
                    }
                }
            }

            if self.settings.image_invert_tone {
                for px in &mut processed {
                    *px = 255u8.saturating_sub(*px);
                }
            }

            // Ensure texture exists and matches size
            let recreate = match &self.vector_field_image_texture {
                Some(tex) => {
                    let size = tex.size();
                    size.width != target_w || size.height != target_h
                }
                None => true,
            };

            if recreate {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Vector Field Image Texture (Webcam)"),
                    size: wgpu::Extent3d {
                        width: target_w,
                        height: target_h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });

                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("Vector Field Image Sampler (Webcam)"),
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                });

                self.vector_field_image_texture = Some(texture);
                self.vector_field_image_view = Some(view);
                self.vector_field_image_sampler = Some(sampler);
                self.update_flow_vector_bind_group(device)?;
            }

            if let Some(texture) = &self.vector_field_image_texture {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &processed,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(target_w),
                        rows_per_image: Some(target_h),
                    },
                    wgpu::Extent3d {
                        width: target_w,
                        height: target_h,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        Ok(())
    }

    fn write_sim_params(&self, queue: &Arc<wgpu::Queue>) {
        let sim_params =
            self.create_sim_params_with_flow_resolution(self.flow_vectors.len() as u32);

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
    }

    fn update_flow_vector_params(&self, queue: &Arc<Queue>) {
        let noise_type = match self.settings.noise_type {
            NoiseType::OpenSimplex => 0,
            NoiseType::Worley => 1,
            NoiseType::Value => 2,
            NoiseType::Fbm => 3,
            NoiseType::FBMBillow => 4,
            NoiseType::FBMClouds => 5,
            NoiseType::FBMRidged => 6,
            NoiseType::Billow => 7,
            NoiseType::RidgedMulti => 8,
            NoiseType::Cylinders => 9,
            NoiseType::Checkerboard => 10,
        };

        let params = FlowVectorParams {
            grid_size: 128,
            vector_field_type: match self.settings.vector_field_type {
                VectorFieldType::Noise => 0,
                VectorFieldType::Image => 1,
            },
            noise_type,
            noise_scale: self.settings.noise_scale as f32,
            noise_x: self.settings.noise_x as f32,
            noise_y: self.settings.noise_y as f32,
            noise_seed: self.settings.noise_seed,
            time: self.time,
            noise_dt_multiplier: self.settings.noise_dt_multiplier,
            vector_magnitude: self.settings.vector_magnitude,
        };

        queue.write_buffer(
            &self.flow_vector_params_buffer,
            0,
            bytemuck::cast_slice(&[params]),
        );
    }

    fn calculate_background_color(&self) -> [f32; 4] {
        match self.state.background_color_mode {
            super::settings::BackgroundColorMode::Black => [0.0f32, 0.0f32, 0.0f32, 1.0f32],
            super::settings::BackgroundColorMode::White => [1.0f32, 1.0f32, 1.0f32, 1.0f32],
            super::settings::BackgroundColorMode::Gray18 => [0.18f32, 0.18f32, 0.18f32, 1.0f32],
            super::settings::BackgroundColorMode::ColorScheme => {
                let mut lut = self
                    .color_scheme_manager
                    .get(&self.state.current_color_scheme)
                    .unwrap_or_else(|_| self.color_scheme_manager.get_default());

                // Apply reversal if needed
                if self.state.color_scheme_reversed {
                    lut = lut.reversed();
                }

                // Use the last color from the color scheme for background
                if let Some(color) = lut.get_last_color() {
                    [color[0], color[1], color[2], color[3]]
                } else {
                    [0.0f32, 0.0f32, 0.0f32, 1.0f32] // Fallback to black
                }
            }
        }
    }

    fn update_background_color(&self, queue: &Arc<wgpu::Queue>) {
        let background_color = self.calculate_background_color();
        queue.write_buffer(
            &self.background_color_buffer,
            0,
            bytemuck::cast_slice(&background_color),
        );
    }

    fn calculate_average_color(&self, device: &Arc<Device>, queue: &Arc<Queue>) {
        self.average_color_resources
            .calculate_average_color(device, queue, &self.display_texture);

        // Wait for the GPU work to complete
        device
            .poll(wgpu::wgt::PollType::Wait)
            .expect("Failed to poll device");

        // Read the result and update the average color uniform buffer for the infinite render shader
        if let Some(average_color) = self.average_color_resources.get_average_color() {
            queue.write_buffer(
                &self.average_color_uniform_buffer,
                0,
                bytemuck::cast_slice(&[average_color]),
            );
        }
        // Unmap the staging buffer after reading
        self.average_color_resources.unmap_staging_buffer();
    }

    fn apply_post_processing(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        input_texture_view: &wgpu::TextureView,
        output_texture_view: &wgpu::TextureView,
    ) -> crate::error::SimulationResult<()> {
        // Apply blur filter if enabled
        if self.post_processing_state.blur_filter.enabled {
            // Update blur parameters
            self.post_processing_resources.update_blur_params(
                queue,
                self.post_processing_state.blur_filter.radius,
                self.post_processing_state.blur_filter.sigma,
                self.trail_map_width,
                self.trail_map_height,
            );

            // Create a new bind group with the input texture
            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Bind Group"),
                layout: &self
                    .post_processing_resources
                    .blur_pipeline
                    .get_bind_group_layout(0),
                entries: &[
                    resource_helpers::texture_view_entry(0, input_texture_view),
                    resource_helpers::sampler_bind_entry(
                        1,
                        &self.post_processing_resources.blur_sampler,
                    ),
                    resource_helpers::buffer_entry(
                        2,
                        &self.post_processing_resources.blur_params_buffer,
                    ),
                ],
            });

            // Create blur render pass
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Post Processing Blur Encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Post Processing Blur Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: output_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&self.post_processing_resources.blur_pipeline);
                render_pass.set_bind_group(0, &blur_bind_group, &[]);
                render_pass.draw(0..6, 0..1);
            }

            queue.submit(std::iter::once(encoder.finish()));
        }

        Ok(())
    }
}

impl Simulation for FlowModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> crate::error::SimulationResult<()> {
        // Store actual delta time for GPU uniforms
        self.delta_time = delta_time;

        // Update simulation time with overflow protection
        let new_time = self.time + delta_time;
        if new_time.is_finite() {
            self.time = new_time;
            self.state.time = new_time;
        } else {
            // If overflow occurs, reset to 0
            self.time = 0.0;
            self.state.time = 0.0;
        }

        // If webcam is active, update vector field image from webcam first
        if self.webcam_capture.is_active {
            let _ = self.update_vector_field_from_webcam(device, queue);
            // Ensure Image mode so the compute uses the texture
            self.settings.vector_field_type = super::settings::VectorFieldType::Image;
        }

        // Update simulation parameters using the centralized method
        self.write_sim_params(queue);

        // Generate flow vectors on GPU every frame for time-varying flow field
        self.regenerate_flow_vectors(device, queue)?;

        // Update camera and upload to GPU
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        // Run trail decay and diffusion compute pass (parallelized)
        let mut trail_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flow Trail Decay Diffusion Encoder"),
        });

        {
            let mut compute_pass = trail_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Flow Trail Decay Diffusion Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.trail_decay_diffusion_pipeline);
            compute_pass.set_bind_group(0, &self.trail_decay_diffusion_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.trail_map_width.div_ceil(16),
                self.trail_map_height.div_ceil(16),
                1,
            );
        }

        queue.submit(std::iter::once(trail_encoder.finish()));

        // Prepare spawn control: accumulator -> integer tickets
        let autospawn_rate = self.settings.autospawn_rate as f32;
        self.autospawn_accumulator += autospawn_rate * delta_time;
        let mut autospawn_allowed = self.autospawn_accumulator.floor() as u32;
        self.autospawn_accumulator -= autospawn_allowed as f32;

        // Brush tickets only when left mouse is held
        let brush_rate = if self.mouse_button_down == 1 {
            self.settings.brush_spawn_rate as f32
        } else {
            0.0
        };
        self.brush_spawn_accumulator += brush_rate * delta_time;
        let mut brush_allowed = self.brush_spawn_accumulator.floor() as u32;
        self.brush_spawn_accumulator -= brush_allowed as f32;

        // Clamp by a reasonable maximum per frame to avoid long frames
        let max_per_frame = 100000u32;
        if autospawn_allowed > max_per_frame {
            autospawn_allowed = max_per_frame;
        }
        if brush_allowed > max_per_frame {
            brush_allowed = max_per_frame;
        }

        // Reset GPU counters and set allowed quotas
        let spawn_control = SpawnControl {
            autospawn_allowed,
            brush_allowed,
            autospawn_count: 0,
            brush_count: 0,
        };
        queue.write_buffer(
            &self.spawn_control_buffer,
            0,
            bytemuck::cast_slice(&[spawn_control]),
        );

        // Run particle update compute pass
        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flow Particle Update Encoder"),
        });

        {
            let mut compute_pass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Flow Particle Update Pass"),
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&self.particle_update_pipeline);
            compute_pass.set_bind_group(0, &self.particle_update_bind_group, &[]);
            // Use total pool size for compute dispatch
            compute_pass.dispatch_workgroups(self.total_pool_size.div_ceil(64), 1, 1);
        }

        queue.submit(std::iter::once(compute_encoder.finish()));

        // 1. Render trails, background, and particles to display texture (offscreen)
        let mut offscreen_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Flow Offscreen Encoder"),
            });
        {
            let mut render_pass =
                offscreen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Flow Offscreen Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
            // Render background
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Single instance
            // Render trails
            render_pass.set_pipeline(&self.trail_render_pipeline);
            render_pass.set_bind_group(0, &self.trail_render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Single instance
            // Render particles (if enabled)
            if self.state.show_particles {
                render_pass.set_pipeline(&self.particle_render_pipeline);
                render_pass.set_bind_group(0, &self.particle_render_bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                render_pass.draw(0..6, 0..self.total_pool_size); // Single instance per particle
            }
        }
        queue.submit(std::iter::once(offscreen_encoder.finish()));

        // 2. Apply post-processing if enabled
        if self.post_processing_state.blur_filter.enabled {
            // Apply post-processing: input from display_view, output to intermediate_view
            self.apply_post_processing(
                device,
                queue,
                &self.display_view,
                &self.post_processing_resources.intermediate_view,
            )?;

            // Copy the blurred result back to the display texture
            let mut copy_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Post Processing Copy Encoder"),
            });

            copy_encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.post_processing_resources.intermediate_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.display_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.trail_map_width,
                    height: self.trail_map_height,
                    depth_or_array_layers: 1,
                },
            );

            queue.submit(std::iter::once(copy_encoder.finish()));
        }

        // 3. Calculate average color from the display texture
        self.calculate_average_color(device, queue);

        // No need to copy since we're using the same texture for rendering and sampling

        // 4. Render display texture to surface with infinite tiling
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flow Infinite Surface Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Flow Infinite Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &self.render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances); // Dynamic grid based on zoom
        }
        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> crate::error::SimulationResult<()> {
        // Update camera and upload to GPU (same as normal render)
        self.camera.update(0.016);
        self.camera.upload_to_gpu(queue);

        // 1. Render background, trails, and particles to display texture (offscreen) without updating simulation state
        let mut offscreen_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Flow Static Offscreen Encoder"),
            });
        {
            let mut render_pass =
                offscreen_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Flow Static Offscreen Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
            // Render background
            render_pass.set_pipeline(&self.background_render_pipeline);
            render_pass.set_bind_group(0, &self.background_render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Single instance
            // Render trails
            render_pass.set_pipeline(&self.trail_render_pipeline);
            render_pass.set_bind_group(0, &self.trail_render_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..1); // Single instance
            // Render particles (if enabled)
            if self.state.show_particles {
                render_pass.set_pipeline(&self.particle_render_pipeline);
                render_pass.set_bind_group(0, &self.particle_render_bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                render_pass.draw(0..6, 0..self.total_pool_size); // Single instance per particle
            }
        }
        queue.submit(std::iter::once(offscreen_encoder.finish()));

        // 2. Calculate average color from the display texture
        self.calculate_average_color(device, queue);

        // 3. Render display texture to surface with infinite tiling
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flow Static Infinite Surface Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Flow Static Infinite Surface Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_infinite_pipeline);
            render_pass.set_bind_group(0, &self.render_infinite_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..6, 0..total_instances); // Dynamic grid based on zoom
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> crate::error::SimulationResult<()> {
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        // Recreate trail texture with new dimensions
        self.trail_map_width = new_config.width;
        self.trail_map_height = new_config.height;

        // Ensure webcam capture resizes frames to match new trail dimensions
        self.webcam_capture
            .set_target_dimensions(self.trail_map_width, self.trail_map_height);

        self.trail_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Trail Texture"),
            size: wgpu::Extent3d {
                width: new_config.width,
                height: new_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.trail_texture_view = self
            .trail_texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("Trail Texture View"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                format: Some(wgpu::TextureFormat::Rgba8Unorm),
                ..Default::default()
            });

        // Recreate bind groups that reference the trail texture
        self.particle_update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Update Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
                    resource_helpers::storage_texture_entry(
                        3,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::StorageTextureAccess::ReadWrite,
                        wgpu::TextureFormat::Rgba8Unorm,
                    ),
                    resource_helpers::storage_buffer_entry(4, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::storage_buffer_entry(5, wgpu::ShaderStages::COMPUTE, false),
                ],
            }),
            entries: &[
                resource_helpers::buffer_entry(0, &self.particle_buffer),
                resource_helpers::buffer_entry(1, &self.flow_vector_buffer),
                resource_helpers::buffer_entry(2, &self.sim_params_buffer),
                resource_helpers::texture_view_entry(3, &self.trail_texture_view),
                resource_helpers::buffer_entry(4, &self.lut_buffer),
                resource_helpers::buffer_entry(5, &self.spawn_control_buffer),
            ],
        });

        self.trail_decay_diffusion_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Trail Decay Diffusion Bind Group"),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Trail Update Bind Group Layout"),
                    entries: &[
                        resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::COMPUTE),
                        resource_helpers::storage_texture_entry(
                            1,
                            wgpu::ShaderStages::COMPUTE,
                            wgpu::StorageTextureAccess::ReadWrite,
                            wgpu::TextureFormat::Rgba8Unorm,
                        ),
                        resource_helpers::storage_buffer_entry(
                            2,
                            wgpu::ShaderStages::COMPUTE,
                            true,
                        ),
                    ],
                }),
                entries: &[
                    resource_helpers::buffer_entry(0, &self.sim_params_buffer),
                    resource_helpers::texture_view_entry(1, &self.trail_texture_view),
                    resource_helpers::buffer_entry(2, &self.flow_vector_buffer),
                ],
            });

        self.trail_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Trail Render Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Trail Render Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(
                        0,
                        wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ),
                    resource_helpers::storage_texture_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::StorageTextureAccess::ReadOnly,
                        wgpu::TextureFormat::Rgba8Unorm,
                    ),
                ],
            }),
            entries: &[
                resource_helpers::buffer_entry(0, &self.sim_params_buffer),
                resource_helpers::texture_view_entry(1, &self.trail_texture_view),
            ],
        });

        // Update sim params with new dimensions
        let sim_params = self.create_runtime_sim_params();

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        // Recreate camera bind group with updated camera buffer
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                )],
            });

        self.camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, self.camera.buffer())],
        });

        // Create display texture for rendering and sampling
        self.display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Flow Display Texture"),
            size: wgpu::Extent3d {
                width: new_config.width,
                height: new_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.display_view = self
            .display_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Use the same texture for both rendering and sampling
        self.display_mipmap_texture = self.display_texture.clone();
        self.display_mipmap_view = self.display_view.clone();
        self.display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Flow Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: self.app_settings.texture_filtering.into(),
            min_filter: self.app_settings.texture_filtering.into(),
            mipmap_filter: self.app_settings.texture_filtering.into(),
            ..Default::default()
        });

        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Flow Render Infinite Bind Group Layout"),
                entries: &[
                    resource_helpers::texture_entry(
                        0,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        // Recreate infinite render pipeline with new surface format
        let render_infinite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Render Infinite Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_INFINITE_SHADER.into()),
        });

        let render_infinite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Flow Render Infinite Pipeline Layout"),
                bind_group_layouts: &[
                    &render_infinite_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        self.render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Flow Render Infinite Pipeline"),
                layout: Some(&render_infinite_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &render_infinite_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &render_infinite_shader,
                    entry_point: Some("fs_main_texture"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: new_config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
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

        self.render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Render Infinite Bind Group"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &self.display_mipmap_view),
                resource_helpers::sampler_bind_entry(1, &self.display_sampler),
                resource_helpers::buffer_entry(2, &self.average_color_uniform_buffer),
                resource_helpers::buffer_entry(3, &self.background_color_buffer),
            ],
        });

        // Camera bind group is already updated above, no separate 3x3 camera needed

        // Recreate post-processing resources for new size
        self.post_processing_resources.resize(device, new_config)?;

        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        match setting_name {
            "noise_type" => {
                if let Some(noise_type_str) = value.as_str() {
                    self.settings.noise_type = match noise_type_str {
                        "OpenSimplex" => NoiseType::OpenSimplex,
                        "Worley" => NoiseType::Worley,
                        "Value" => NoiseType::Value,
                        "FBM" => NoiseType::Fbm,
                        "FBMBillow" => NoiseType::FBMBillow,
                        "FBMClouds" => NoiseType::FBMClouds,
                        "FBMRidged" => NoiseType::FBMRidged,
                        "Billow" => NoiseType::Billow,
                        "RidgedMulti" => NoiseType::RidgedMulti,
                        "Cylinders" => NoiseType::Cylinders,
                        "Checkerboard" => NoiseType::Checkerboard,
                        _ => NoiseType::OpenSimplex,
                    };
                    // Regenerate flow vectors with new noise type
                    self.regenerate_flow_vectors(device, queue)?;
                }
            }
            "noise_seed" => {
                if let Some(seed) = value.as_u64() {
                    self.settings.noise_seed = seed as u32;
                    // Regenerate flow vectors with new seed
                    self.regenerate_flow_vectors(device, queue)?;
                }
            }
            "noise_scale" => {
                if let Some(scale) = value.as_f64() {
                    self.settings.noise_scale = scale;
                    // Regenerate flow vectors with new scale
                    self.regenerate_flow_vectors(device, queue)?;
                }
            }
            "noise_x" => {
                if let Some(x) = value.as_f64() {
                    self.settings.noise_x = x;
                    // Regenerate flow vectors with new X scale
                    self.regenerate_flow_vectors(device, queue)?;
                }
            }
            "noise_y" => {
                if let Some(y) = value.as_f64() {
                    self.settings.noise_y = y;
                    // Regenerate flow vectors with new Y scale
                    self.regenerate_flow_vectors(device, queue)?;
                }
            }
            "noise_dt_multiplier" => {
                if let Some(multiplier) = value.as_f64() {
                    self.settings.noise_dt_multiplier = multiplier as f32;
                    // Update sim params with new multiplier
                    self.write_sim_params(queue);
                }
            }

            "vector_magnitude" => {
                if let Some(magnitude) = value.as_f64() {
                    self.settings.vector_magnitude = magnitude as f32;
                    // Regenerate flow vectors with new magnitude
                    self.regenerate_flow_vectors(device, queue)?;
                }
            }

            "autospawn_limit" => {
                if let Some(count) = value.as_u64() {
                    let new_count = count as u32;
                    if new_count != self.settings.total_pool_size {
                        self.settings.total_pool_size = new_count;

                        // Update sim params with new autospawn limit
                        let sim_params = self.create_runtime_sim_params();

                        queue.write_buffer(
                            &self.sim_params_buffer,
                            0,
                            bytemuck::cast_slice(&[sim_params]),
                        );
                    }
                }
            }
            "particle_lifetime" => {
                if let Some(lifetime) = value.as_f64() {
                    self.settings.particle_lifetime = lifetime as f32;
                }
            }
            "particle_speed" => {
                if let Some(speed) = value.as_f64() {
                    self.settings.particle_speed = speed as f32;
                }
            }
            "particle_size" => {
                if let Some(size) = value.as_u64() {
                    self.settings.particle_size = size as u32;
                }
            }

            "trail_decay_rate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_decay_rate = rate as f32;
                }
            }
            "trail_deposition_rate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_deposition_rate = rate as f32;
                }
            }
            "trail_diffusion_rate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_diffusion_rate = rate as f32;
                }
            }
            "trail_wash_out_rate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_wash_out_rate = rate as f32;
                }
            }
            "particle_shape" => {
                if let Some(shape_str) = value.as_str() {
                    self.settings.particle_shape = match shape_str {
                        "Circle" => super::settings::ParticleShape::Circle,
                        "Square" => super::settings::ParticleShape::Square,
                        "Triangle" => super::settings::ParticleShape::Triangle,
                        "Flower" | "Star" => super::settings::ParticleShape::Star,
                        "Diamond" => super::settings::ParticleShape::Diamond,
                        _ => super::settings::ParticleShape::Circle,
                    };
                }
            }

            "particle_autospawn" => {
                if let Some(autospawn) = value.as_bool() {
                    self.settings.particle_autospawn = autospawn;
                }
            }
            "autospawn_rate" => {
                if let Some(rate) = value.as_u64() {
                    self.settings.autospawn_rate = rate as u32;
                }
            }
            "brush_spawn_rate" => {
                if let Some(rate) = value.as_u64() {
                    self.settings.brush_spawn_rate = rate as u32;
                }
            }

            "trail_map_filtering" => {
                if let Some(filtering_str) = value.as_str() {
                    self.trail_map_filtering = match filtering_str {
                        "Nearest" => super::settings::TrailMapFiltering::Nearest,
                        "Linear" => super::settings::TrailMapFiltering::Linear,
                        _ => super::settings::TrailMapFiltering::Nearest,
                    };
                    // Update the trail sampler with new filtering
                    self.update_trail_sampler(device);
                }
            }
            _ => {}
        }

        // After handling the specific setting, always update the GPU uniform so changes take effect immediately
        self.write_sim_params(queue);

        Ok(())
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        match state_name {
            "current_color_scheme" => {
                if let Some(color_scheme_name) = value.as_str() {
                    self.state.current_color_scheme = color_scheme_name.to_string();
                    let mut color_scheme_data = self
                        .color_scheme_manager
                        .get(&self.state.current_color_scheme)
                        .unwrap_or_else(|_| self.color_scheme_manager.get_default());

                    // Apply reversal if needed
                    if self.state.color_scheme_reversed {
                        color_scheme_data = color_scheme_data.reversed();
                    }

                    queue.write_buffer(
                        &self.lut_buffer,
                        0,
                        bytemuck::cast_slice(&color_scheme_data.to_u32_buffer()),
                    );

                    // Update background color if ColorScheme background is selected
                    if self.state.background_color_mode
                        == super::settings::BackgroundColorMode::ColorScheme
                    {
                        // Update background color first
                        self.update_background_color(queue);
                    }
                }
            }
            "color_scheme_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    self.state.color_scheme_reversed = reversed;

                    // Reload the current color scheme with new reversal state
                    let mut color_scheme_data = self
                        .color_scheme_manager
                        .get(&self.state.current_color_scheme)
                        .unwrap_or_else(|_| self.color_scheme_manager.get_default());

                    // Apply reversal if needed
                    if self.state.color_scheme_reversed {
                        color_scheme_data = color_scheme_data.reversed();
                    }

                    queue.write_buffer(
                        &self.lut_buffer,
                        0,
                        bytemuck::cast_slice(&color_scheme_data.to_u32_buffer()),
                    );

                    // Update background color if ColorScheme background is selected
                    if self.state.background_color_mode
                        == super::settings::BackgroundColorMode::ColorScheme
                    {
                        // Update background color first
                        self.update_background_color(queue);
                    }
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.cursor_size = size as f32;
                    self.state.cursor_size = size as f32;
                }
            }
            "background_color_mode" => {
                if let Some(background_str) = value.as_str() {
                    let background_mode = match background_str {
                        "Black" => super::settings::BackgroundColorMode::Black,
                        "White" => super::settings::BackgroundColorMode::White,
                        "Gray18" => super::settings::BackgroundColorMode::Gray18,
                        "Color Scheme" => super::settings::BackgroundColorMode::ColorScheme,
                        _ => panic!("Unknown background color mode: {}", background_str),
                    };
                    self.state.background_color_mode = background_mode;
                    // Update background color
                    self.update_background_color(queue);
                }
            }
            "foreground_color_mode" => {
                if let Some(mode_str) = value.as_str() {
                    let foreground_mode = match mode_str {
                        "Age" => super::settings::ForegroundColorMode::Age,
                        "Random" => super::settings::ForegroundColorMode::Random,
                        "Direction" => super::settings::ForegroundColorMode::Direction,
                        _ => super::settings::ForegroundColorMode::Age,
                    };
                    self.state.foreground_color_mode = foreground_mode;
                }
            }
            "show_particles" => {
                if let Some(show) = value.as_bool() {
                    self.state.show_particles = show;
                }
            }
            _ => {
                tracing::warn!("Unknown state parameter: {}", state_name);
            }
        }

        // Update simulation parameters to reflect state changes
        self.write_sim_params(queue);

        Ok(())
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&self.settings).unwrap_or_default()
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::to_value(&self.state).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Store cursor values in the model
        self.cursor_world_x = world_x;
        self.cursor_world_y = world_y;

        // Set mouse_button_down state based on which button is pressed
        self.mouse_button_down = match mouse_button {
            0 => 1, // Left click = spawn particles
            2 => 2, // Right click = destroy particles
            _ => 0, // Other buttons = no action
        };

        // Update sim params with cursor state
        let sim_params = self.create_sim_params_with_cursor(world_x, world_y);

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Turn off cursor interaction and reset position
        self.cursor_world_x = 0.0;
        self.cursor_world_y = 0.0;
        self.mouse_button_down = 0; // Set mouse button to not held

        tracing::debug!("Flow mouse release: cursor interaction disabled");

        Ok(())
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
    }

    fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    fn reset_camera(&mut self) {
        self.camera.reset();
    }

    fn get_camera_state(&self) -> serde_json::Value {
        serde_json::json!({
            "position": [self.camera.position[0], self.camera.position[1]],
            "zoom": self.camera.zoom,
        })
    }

    fn save_preset(&self, _preset_name: &str) -> crate::error::SimulationResult<()> {
        Ok(())
    }

    fn load_preset(
        &mut self,
        _preset_name: &str,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        Ok(())
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            self.settings = new_settings;

            // Update GPU buffers after applying new settings
            self.update_background_color(queue);
            self.write_sim_params(queue);
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        self.time = 0.0;
        self.state.time = 0.0;
        // Reset particles
        let mut particles = Vec::with_capacity(self.settings.total_pool_size as usize);

        RNG.with(|_rng| {
            // Initialize autospawn pool with dead particles and staggered spawn times
            for _ in 0..self.autospawn_pool_size {
                let particle = Particle {
                    position: [0.0, 0.0],                 // Will be set when spawned
                    age: self.settings.particle_lifetime, // Start dead
                    lut_index: 0,                         // No color
                    is_alive: 0,                          // Dead particles are inactive
                    spawn_type: 0,                        // Autospawn particles
                    _pad0: 0,
                    _pad1: 0,
                };
                particles.push(particle);
            }

            // Initialize brush pool with dead particles and staggered spawn times
            for _ in 0..self.brush_pool_size {
                let particle = Particle {
                    position: [0.0, 0.0],                 // Will be set when spawned
                    age: self.settings.particle_lifetime, // Start dead
                    lut_index: 0,                         // No color
                    is_alive: 0,                          // Dead particles are inactive
                    spawn_type: 1,                        // Brush particles
                    _pad0: 0,
                    _pad1: 0,
                };
                particles.push(particle);
            }
        });

        queue.write_buffer(&self.particle_buffer, 0, bytemuck::cast_slice(&particles));
        self.particles = particles;

        // Reset trail map - clear texture with zeros
        let zero_data = vec![0u8; (self.trail_map_width * self.trail_map_height * 4) as usize];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.trail_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &zero_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.trail_map_width * 4),
                rows_per_image: Some(self.trail_map_height),
            },
            wgpu::Extent3d {
                width: self.trail_map_width,
                height: self.trail_map_height,
                depth_or_array_layers: 1,
            },
        );

        // Also clear the trail texture view for rendering with the correct background color
        let background_color = self.calculate_background_color();
        let clear_color = wgpu::Color {
            r: background_color[0] as f64,
            g: background_color[1] as f64,
            b: background_color[2] as f64,
            a: background_color[3] as f64,
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Trail Texture Encoder"),
        });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Trail Texture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.trail_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        queue.submit(std::iter::once(encoder.finish()));

        // Update background color after reset
        self.update_background_color(queue);

        // Update simulation parameters after reset
        self.write_sim_params(queue);

        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.state.gui_visible = self.gui_visible;
        self.gui_visible
    }

    fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        let mut rng = rand::rng();

        // Randomize noise type
        let noise_types = [
            crate::simulations::flow::settings::NoiseType::OpenSimplex,
            crate::simulations::flow::settings::NoiseType::Worley,
            crate::simulations::flow::settings::NoiseType::Value,
            crate::simulations::flow::settings::NoiseType::Fbm,
            crate::simulations::flow::settings::NoiseType::FBMBillow,
            crate::simulations::flow::settings::NoiseType::FBMClouds,
            crate::simulations::flow::settings::NoiseType::FBMRidged,
            crate::simulations::flow::settings::NoiseType::Billow,
            crate::simulations::flow::settings::NoiseType::RidgedMulti,
            crate::simulations::flow::settings::NoiseType::Cylinders,
            crate::simulations::flow::settings::NoiseType::Checkerboard,
        ];
        self.settings.noise_type = noise_types[rng.random_range(0..noise_types.len())];

        self.settings.noise_seed = rng.random();
        self.settings.noise_scale = rng.random_range(0.5..3.0);
        self.settings.noise_x = rng.random_range(-100.0..100.0);
        self.settings.noise_y = rng.random_range(-100.0..100.0);
        self.settings.vector_magnitude = rng.random_range(0.05..0.2);

        // Regenerate flow vectors with new settings
        self.regenerate_flow_vectors(device, queue)?;

        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Direct-write the color scheme data to the Flow buffer for immediate preview
        let data_u32 = color_scheme.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&data_u32));
        Ok(())
    }
}

impl FlowModel {
    // Draw an antialiased shape onto the trail map
    pub fn draw_antialiased_shape(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        center_x: f32,
        center_y: f32,
        size: f32,
        shape_type: u32,
        color: [f32; 4],
        intensity: f32,
        antialiasing_width: f32,
        rotation: f32,
    ) -> crate::error::SimulationResult<()> {
        // Update shape parameters
        let shape_params = ShapeParams {
            center_x,
            center_y,
            size,
            shape_type,
            color,
            intensity,
            antialiasing_width,
            rotation,
            aspect_ratio: 1.0,
            trail_map_width: self.trail_map_width,
            trail_map_height: self.trail_map_height,
            _padding_0: 0,
            _padding_1: 0,
        };

        // Update the shape parameters buffer
        queue.write_buffer(
            &self.shape_params_buffer,
            0,
            bytemuck::cast_slice(&[shape_params]),
        );

        // Dispatch the shape drawing compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Shape Drawing Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Shape Drawing Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.shape_drawing_pipeline);
            compute_pass.set_bind_group(0, &self.shape_drawing_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.trail_map_width.div_ceil(8),
                self.trail_map_height.div_ceil(8),
                1,
            );
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn kill_all_particles(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Set all particles to dead state (age = lifetime)
        for particle in &mut self.particles {
            particle.age = self.settings.particle_lifetime;
            particle.lut_index = 0; // Clear color index
            particle.is_alive = 0; // Mark as inactive
            particle.spawn_type = 0; // Reset spawn type
        }

        // Update particle buffer with dead particles
        queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&self.particles),
        );

        // Update active particle counts

        // Update sim params with zero active particle counts
        let sim_params = self.create_sim_params_with_counts(0, 0);

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        Ok(())
    }

    fn update_trail_sampler(&mut self, device: &Arc<Device>) {
        self.trail_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Trail Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: self.app_settings.texture_filtering.into(),
            min_filter: self.app_settings.texture_filtering.into(),
            mipmap_filter: self.app_settings.texture_filtering.into(),
            ..Default::default()
        });
    }

    /// Load an image for vector field generation
    pub fn load_vector_field_image_from_path(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        path: &str,
    ) -> crate::error::SimulationResult<()> {
        tracing::info!("Loading vector field image from: {}", path);

        let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;

        self.load_vector_field_image_from_data(device, queue, img)
    }

    /// Load an image from raw data for vector field generation
    pub fn load_vector_field_image_from_data(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        img: image::DynamicImage,
    ) -> crate::error::SimulationResult<()> {
        tracing::info!(
            "Processing vector field image: {}x{}",
            img.width(),
            img.height()
        );

        // Store the original image
        self.vector_field_image_original = Some(img.clone());

        // Process the image with current fit mode
        self.reprocess_vector_field_image_with_current_fit_mode(device, queue)?;

        // Update the bind group with the new image texture
        self.update_flow_vector_bind_group(device)?;

        tracing::info!("Vector field image loaded successfully");
        Ok(())
    }

    /// Reprocess the stored original image with the current fit mode
    pub fn reprocess_vector_field_image_with_current_fit_mode(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        if let Some(original_img) = &self.vector_field_image_original {
            let (target_w, target_h) = (self.trail_map_width as u32, self.trail_map_height as u32);

            // Convert to Luma8 (grayscale)
            let gray = original_img.to_luma8();
            let fit_mode = self.settings.image_fit_mode;

            // Process the image based on fit mode
            let processed_img = match fit_mode {
                crate::simulations::shared::ImageFitMode::Stretch => {
                    // Resize exactly to target size
                    image::imageops::resize(
                        &gray,
                        target_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    )
                }
                crate::simulations::shared::ImageFitMode::Center => {
                    // Center the image and pad with black
                    let mut canvas = image::ImageBuffer::new(target_w, target_h);
                    let (img_w, img_h) = (gray.width(), gray.height());

                    let start_x = if img_w < target_w {
                        (target_w - img_w) / 2
                    } else {
                        0
                    };
                    let start_y = if img_h < target_h {
                        (target_h - img_h) / 2
                    } else {
                        0
                    };

                    for (x, y, pixel) in gray.enumerate_pixels() {
                        let canvas_x = start_x + x;
                        let canvas_y = start_y + y;
                        if canvas_x < target_w && canvas_y < target_h {
                            canvas.put_pixel(canvas_x, canvas_y, *pixel);
                        }
                    }
                    canvas
                }
                crate::simulations::shared::ImageFitMode::FitH => {
                    // Fit horizontally, center vertically
                    let scale = target_w as f32 / gray.width() as f32;
                    let new_height = (gray.height() as f32 * scale) as u32;
                    let resized = image::imageops::resize(
                        &gray,
                        target_w,
                        new_height,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let mut canvas = image::ImageBuffer::new(target_w, target_h);
                    let start_y = if new_height < target_h {
                        (target_h - new_height) / 2
                    } else {
                        0
                    };

                    for (x, y, pixel) in resized.enumerate_pixels() {
                        let canvas_y = start_y + y;
                        if canvas_y < target_h {
                            canvas.put_pixel(x, canvas_y, *pixel);
                        }
                    }
                    canvas
                }
                crate::simulations::shared::ImageFitMode::FitV => {
                    // Fit vertically, center horizontally
                    let scale = target_h as f32 / gray.height() as f32;
                    let new_width = (gray.width() as f32 * scale) as u32;
                    let resized = image::imageops::resize(
                        &gray,
                        new_width,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );

                    let mut canvas = image::ImageBuffer::new(target_w, target_h);
                    let start_x = if new_width < target_w {
                        (target_w - new_width) / 2
                    } else {
                        0
                    };

                    for (x, y, pixel) in resized.enumerate_pixels() {
                        let canvas_x = start_x + x;
                        if canvas_x < target_w {
                            canvas.put_pixel(canvas_x, y, *pixel);
                        }
                    }
                    canvas
                }
            };

            // Apply mirror and invert transformations
            let mut final_img = processed_img;

            if self.settings.image_mirror_horizontal {
                image::imageops::flip_horizontal_in_place(&mut final_img);
            }

            if self.settings.image_mirror_vertical {
                image::imageops::flip_vertical_in_place(&mut final_img);
            }

            if self.settings.image_invert_tone {
                for pixel in final_img.pixels_mut() {
                    pixel.0[0] = 255 - pixel.0[0];
                }
            }

            // Create GPU texture
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Vector Field Image Texture"),
                size: wgpu::Extent3d {
                    width: target_w,
                    height: target_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Vector Field Image Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            // Upload image data to GPU
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &final_img.into_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(target_w),
                    rows_per_image: Some(target_h),
                },
                wgpu::Extent3d {
                    width: target_w,
                    height: target_h,
                    depth_or_array_layers: 1,
                },
            );

            // Update the model with new texture resources
            self.vector_field_image_texture = Some(texture);
            self.vector_field_image_view = Some(view);
            self.vector_field_image_sampler = Some(sampler);
            self.vector_field_image_needs_upload = false;

            // Update the bind group with the new texture
            self.update_flow_vector_bind_group(device)?;

            Ok(())
        } else {
            Err("No original image data available".into())
        }
    }

    /// Update the flow vector compute bind group with the current image texture
    fn update_flow_vector_bind_group(
        &mut self,
        device: &Arc<Device>,
    ) -> crate::error::SimulationResult<()> {
        // Use image texture if available, otherwise use default texture
        let (texture_view, sampler) = if let (Some(view), Some(sampler)) = (
            &self.vector_field_image_view,
            &self.vector_field_image_sampler,
        ) {
            (view, sampler)
        } else {
            (
                &self.default_vector_field_view,
                &self.default_vector_field_sampler,
            )
        };

        // Recreate the bind group with the current texture
        self.flow_vector_compute_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Flow Vector Compute Bind Group"),
                layout: &self.flow_vector_compute_pipeline.get_bind_group_layout(0),
                entries: &[
                    resource_helpers::buffer_entry(0, &self.flow_vector_buffer),
                    resource_helpers::buffer_entry(1, &self.flow_vector_params_buffer),
                    resource_helpers::texture_view_entry(2, texture_view),
                    resource_helpers::sampler_bind_entry(3, sampler),
                ],
            });

        Ok(())
    }

    /// Create SimParams with default values for initialization (static method for constructor)
    fn create_default_sim_params_static(
        settings: &Settings,
        surface_config: &wgpu::SurfaceConfiguration,
        foreground_color_mode: ForegroundColorMode,
        autospawn_pool_size: u32,
        brush_pool_size: u32,
    ) -> SimParams {
        SimParams {
            total_pool_size: settings.total_pool_size,
            flow_field_resolution: DEFAULT_FLOW_FIELD_RESOLUTION,
            particle_lifetime: settings.particle_lifetime,
            particle_speed: settings.particle_speed,
            noise_seed: settings.noise_seed,
            time: 0.0,
            delta_time: 0.016,
            noise_dt_multiplier: 1.0,
            width: 2.0,
            height: 2.0,
            noise_scale: settings.noise_scale as f32,
            noise_x: settings.noise_x as f32,
            noise_y: settings.noise_y as f32,
            vector_magnitude: settings.vector_magnitude,
            trail_decay_rate: settings.trail_decay_rate,
            trail_deposition_rate: settings.trail_deposition_rate,
            trail_diffusion_rate: settings.trail_diffusion_rate,
            trail_wash_out_rate: settings.trail_wash_out_rate,
            trail_map_width: surface_config.width,
            trail_map_height: surface_config.height,
            particle_shape: settings.particle_shape as u32,
            particle_size: settings.particle_size,
            screen_width: surface_config.width,
            screen_height: surface_config.height,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_size: 0.25,
            mouse_button_down: 0,
            particle_autospawn: settings.particle_autospawn as u32,
            autospawn_rate: settings.autospawn_rate,
            brush_spawn_rate: settings.brush_spawn_rate,
            display_mode: foreground_color_mode as u32,
            autospawn_pool_size,
            brush_pool_size,
            _padding_1: 0,
            _padding_2: 0,
        }
    }

    /// Create SimParams with current runtime state
    fn create_runtime_sim_params(&self) -> SimParams {
        SimParams {
            total_pool_size: self.settings.total_pool_size,
            flow_field_resolution: self.flow_field_resolution,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
            delta_time: self.delta_time,
            noise_dt_multiplier: self.settings.noise_dt_multiplier,
            width: 2.0,
            height: 2.0,
            noise_scale: self.settings.noise_scale as f32,
            noise_x: self.settings.noise_x as f32,
            noise_y: self.settings.noise_y as f32,
            vector_magnitude: self.settings.vector_magnitude,
            trail_decay_rate: self.settings.trail_decay_rate,
            trail_deposition_rate: self.settings.trail_deposition_rate,
            trail_diffusion_rate: self.settings.trail_diffusion_rate,
            trail_wash_out_rate: self.settings.trail_wash_out_rate,
            trail_map_width: self.trail_map_width,
            trail_map_height: self.trail_map_height,
            particle_shape: self.settings.particle_shape as u32,
            particle_size: self.settings.particle_size,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_size: self.cursor_size,
            mouse_button_down: self.mouse_button_down,
            particle_autospawn: self.settings.particle_autospawn as u32,
            autospawn_rate: self.settings.autospawn_rate,
            brush_spawn_rate: self.settings.brush_spawn_rate,
            display_mode: self.state.foreground_color_mode as u32,
            autospawn_pool_size: self.autospawn_pool_size,
            brush_pool_size: self.brush_pool_size,
            _padding_1: 0,
            _padding_2: 0,
        }
    }

    /// Create SimParams with custom flow field resolution
    fn create_sim_params_with_flow_resolution(&self, flow_field_resolution: u32) -> SimParams {
        SimParams {
            total_pool_size: self.settings.total_pool_size,
            flow_field_resolution,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
            delta_time: self.delta_time,
            noise_dt_multiplier: self.settings.noise_dt_multiplier,
            width: 2.0,
            height: 2.0,
            noise_scale: self.settings.noise_scale as f32,
            noise_x: self.settings.noise_x as f32,
            noise_y: self.settings.noise_y as f32,
            vector_magnitude: self.settings.vector_magnitude,
            trail_decay_rate: self.settings.trail_decay_rate,
            trail_deposition_rate: self.settings.trail_deposition_rate,
            trail_diffusion_rate: self.settings.trail_diffusion_rate,
            trail_wash_out_rate: self.settings.trail_wash_out_rate,
            trail_map_width: self.trail_map_width,
            trail_map_height: self.trail_map_height,
            particle_shape: self.settings.particle_shape as u32,
            particle_size: self.settings.particle_size,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_size: self.cursor_size,
            mouse_button_down: self.mouse_button_down,
            particle_autospawn: self.settings.particle_autospawn as u32,
            autospawn_rate: self.settings.autospawn_rate,
            brush_spawn_rate: self.settings.brush_spawn_rate,
            display_mode: self.state.foreground_color_mode as u32,
            autospawn_pool_size: self.autospawn_pool_size,
            brush_pool_size: self.brush_pool_size,
            _padding_1: 0,
            _padding_2: 0,
        }
    }

    /// Create SimParams with custom cursor coordinates
    fn create_sim_params_with_cursor(&self, cursor_x: f32, cursor_y: f32) -> SimParams {
        SimParams {
            total_pool_size: self.settings.total_pool_size,
            flow_field_resolution: self.flow_field_resolution,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
            delta_time: self.delta_time,
            noise_dt_multiplier: self.settings.noise_dt_multiplier,
            width: 2.0,
            height: 2.0,
            noise_scale: self.settings.noise_scale as f32,
            noise_x: self.settings.noise_x as f32,
            noise_y: self.settings.noise_y as f32,
            vector_magnitude: self.settings.vector_magnitude,
            trail_decay_rate: self.settings.trail_decay_rate,
            trail_deposition_rate: self.settings.trail_deposition_rate,
            trail_diffusion_rate: self.settings.trail_diffusion_rate,
            trail_wash_out_rate: self.settings.trail_wash_out_rate,
            trail_map_width: self.trail_map_width,
            trail_map_height: self.trail_map_height,
            particle_shape: self.settings.particle_shape as u32,
            particle_size: self.settings.particle_size,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x,
            cursor_y,
            cursor_size: self.cursor_size,
            mouse_button_down: self.mouse_button_down,
            particle_autospawn: self.settings.particle_autospawn as u32,
            autospawn_rate: self.settings.autospawn_rate,
            brush_spawn_rate: self.settings.brush_spawn_rate,
            display_mode: self.state.foreground_color_mode as u32,
            autospawn_pool_size: self.autospawn_pool_size,
            brush_pool_size: self.brush_pool_size,
            _padding_1: 0,
            _padding_2: 0,
        }
    }

    /// Create SimParams with custom active particle counts
    fn create_sim_params_with_counts(&self, autospawn_count: u32, brush_count: u32) -> SimParams {
        SimParams {
            total_pool_size: self.settings.total_pool_size,
            flow_field_resolution: self.flow_field_resolution,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
            delta_time: self.delta_time,
            noise_dt_multiplier: self.settings.noise_dt_multiplier,
            width: 2.0,
            height: 2.0,
            noise_scale: self.settings.noise_scale as f32,
            noise_x: self.settings.noise_x as f32,
            noise_y: self.settings.noise_y as f32,
            vector_magnitude: self.settings.vector_magnitude,
            trail_decay_rate: self.settings.trail_decay_rate,
            trail_deposition_rate: self.settings.trail_deposition_rate,
            trail_diffusion_rate: self.settings.trail_diffusion_rate,
            trail_wash_out_rate: self.settings.trail_wash_out_rate,
            trail_map_width: self.trail_map_width,
            trail_map_height: self.trail_map_height,
            particle_shape: self.settings.particle_shape as u32,
            particle_size: self.settings.particle_size,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_size: self.cursor_size,
            mouse_button_down: self.mouse_button_down,
            particle_autospawn: self.settings.particle_autospawn as u32,
            autospawn_rate: self.settings.autospawn_rate,
            brush_spawn_rate: self.settings.brush_spawn_rate,
            display_mode: self.state.foreground_color_mode as u32,
            autospawn_pool_size: autospawn_count,
            brush_pool_size: brush_count,
            _padding_1: 0,
            _padding_2: 0,
        }
    }
}
