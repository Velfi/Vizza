use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use rand::Rng;
use serde_json::Value;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, Buffer, ComputePipeline, Device, Queue, RenderPipeline, SurfaceConfiguration,
    Texture, TextureView,
};

use crate::commands::app_settings::AppSettings;
use crate::error::SimulationResult;
use crate::simulations::shared::camera::Camera;
use crate::simulations::traits::Simulation;

use super::shaders::{
    COMPUTE_SHADER, COMPUTE_UPDATE_SHADER, GRID_CLEAR_SHADER, GRID_POPULATE_SHADER,
    VCA_INFINITE_RENDER_SHADER, VORONOI_RENDER_SHADER,
};
use crate::simulations::shared::post_processing::{PostProcessingResources, PostProcessingState};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    state: f32,
    pad0: f32,
    age: f32,
    alive_neighbors: u32,
    dead_neighbors: u32,
    pad1: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct VoronoiParams {
    count: f32,
    color_mode: f32,
    neighbor_radius: f32,
    // Borders enabled flag (1.0 = on, 0.0 = off)
    border_enabled: f32,
    // Border threshold for detection (0.0-1.0)
    border_threshold: f32,
    // Texture filtering mode: 0=Nearest, 1=Linear, 2=Lanczos (TODO treated as Linear here)
    filter_mode: f32,
    resolution_x: f32,
    resolution_y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    drift: f32,
    rule_type: u32,
    neighbor_radius: f32,
    alive_threshold: f32,
    _pad0: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GridParams {
    particle_count: u32,
    grid_width: u32,
    grid_height: u32,
    cell_capacity: u32,
    cell_size: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

#[derive(Debug)]
pub struct VoronoiCASimulation {
    voronoi_render_pipeline: RenderPipeline,
    // Compute
    compute_pipeline: ComputePipeline, // neighbor counting with grid
    compute_update_pipeline: ComputePipeline, // state update
    uniform_buffer: Buffer,
    // Bind groups
    compute_neighbor_bg: BindGroup,
    compute_update_bg: BindGroup,
    vertex_buffer: Buffer,
    // Spatial grid resources
    grid_indices: Buffer,
    grid_counts: Buffer,
    grid_params: Buffer,
    grid_clear_pipeline: ComputePipeline,
    grid_populate_pipeline: ComputePipeline,
    grid_clear_bg: BindGroup,
    grid_populate_bg: BindGroup,
    voronoi_params: Buffer,
    voronoi_render_bg: BindGroup,
    num_points: u32,
    time_accum: f32,
    time_scale: f32,
    drift: f32,
    neighbor_radius: f32,
    alive_threshold: f32,
    resolution: [f32; 2],
    gui_visible: bool,
    points: Vec<Vertex>,
    // Run-and-tumble state
    headings: Vec<f32>,         // radians
    run_time_left: Vec<f32>,    // seconds
    tumble_time_left: Vec<f32>, // seconds
    run_speed: f32,             // pixels per second
    avg_run_time: f32,          // seconds (mean of exponential)
    tumble_time: f32,           // seconds (fixed)
    // Cursor config (settings)
    cursor_size: f32,
    cursor_strength: f32,
    // Post-processing
    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
    // LUT + coloring
    pub lut_buffer: Buffer,
    pub current_lut_name: String,
    pub lut_reversed: bool,
    color_mode: u32, // 0=Random, 1=Density, 2=Age
    borders_enabled: bool,
    pub border_threshold: f32, // Border detection threshold (0.0-1.0)
    app_settings: crate::commands::app_settings::AppSettings,
    // Camera
    pub camera: Camera,
    camera_bind_group: BindGroup,
    // Offscreen display for infinite tiling
    display_texture: Texture,
    display_view: TextureView,
    display_sampler: wgpu::Sampler,
    texture_render_params_buffer: Buffer,
    render_infinite_pipeline: RenderPipeline,
    render_infinite_bind_group: BindGroup,
}

impl VoronoiCASimulation {
    /// Calculate dynamic tile count for infinite rendering based on camera zoom
    fn calculate_tile_count(&self) -> u32 {
        let visible_world_size = 2.0 / self.camera.zoom.max(1e-6);
        let mut tiles_needed = (visible_world_size / 2.0).ceil() as u32 + 6; // padding
        let min_tiles = if self.camera.zoom < 0.1 { 7 } else { 5 };
        if tiles_needed < min_tiles {
            tiles_needed = min_tiles;
        }
        tiles_needed.min(1024)
    }

    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        app_settings: &AppSettings,
    ) -> SimulationResult<Self> {
        let width = surface_config.width.max(1) as f32;
        let height = surface_config.height.max(1) as f32;

        let uniforms = Uniforms {
            resolution: [width, height],
            time: 0.0,
            drift: app_settings.default_camera_sensitivity,
            rule_type: 0,
            neighbor_radius: 60.0,
            alive_threshold: 0.5,
            _pad0: 0,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VoronoiCA Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group layouts

        let compute_update_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
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
                label: Some("voronoi_ca_compute_update_bgl"),
            });

        // Neighbor count BG layout: vertices, uniforms, grid_indices, grid_counts, grid_params
        let compute_neighbor_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        // vertices
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // uniforms
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // grid_indices
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // grid_counts
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // grid params
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("voronoi_ca_compute_neighbor_bgl"),
            });

        // Bind groups

        // compute_bind_group will be created after the vertex buffer is created

        // Shaders
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VoronoiCA Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
        });
        let compute_update_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VoronoiCA Compute Update Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPUTE_UPDATE_SHADER.into()),
        });
        let voronoi_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Voronoi Render"),
            source: wgpu::ShaderSource::Wgsl(VORONOI_RENDER_SHADER.into()),
        });

        // Pipelines
        // Remove legacy instanced render pipeline; VCA uses fullscreen Voronoi pipeline only

        // Pipeline for neighbor counting (needs neighbor layout)
        let compute_neighbor_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VoronoiCA Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_neighbor_bgl],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VoronoiCA Compute Pipeline"),
            layout: Some(&compute_neighbor_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        // Pipeline for state update (2-bindings layout)
        let compute_update_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VoronoiCA Compute Update Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("VoronoiCA Compute Update PL"),
                        bind_group_layouts: &[&compute_update_bgl],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &compute_update_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // Initialize points
        let mut rng = rand::rng();
        let num_points = 300u32; // modest default
        let mut points: Vec<Vertex> = Vec::with_capacity(num_points as usize);
        let mut headings: Vec<f32> = Vec::with_capacity(num_points as usize);
        let mut run_time_left: Vec<f32> = Vec::with_capacity(num_points as usize);
        let mut tumble_time_left: Vec<f32> = Vec::with_capacity(num_points as usize);
        for _ in 0..num_points {
            points.push(Vertex {
                position: [
                    rng.random_range(0.0..(width)),
                    rng.random_range(0.0..(height)),
                ],
                state: if rng.random::<f32>() > 0.7 { 1.0 } else { 0.0 },
                pad0: 0.0,
                age: 0.0,
                alive_neighbors: 0,
                dead_neighbors: 0,
                pad1: 0,
            });
            headings.push(rng.random::<f32>() * std::f32::consts::TAU);
            // Exponential sample for initial run duration
            let u: f32 = (1.0 - rng.random::<f32>()).max(1e-6);
            let run_sample = -1.5 * u.ln(); // avg 1.5s default (will be overwritten below when assigning fields)
            run_time_left.push(run_sample);
            tumble_time_left.push(0.0);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VoronoiCA Vertex Buffer"),
            contents: bytemuck::cast_slice(&points),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });

        // Create spatial grid buffers (fixed cell size and capacity)
        let cell_capacity: u32 = 64;
        let cell_size: f32 = (uniforms.neighbor_radius * 1.25).max(8.0); // tie to radius to bound neighbor cells
        let grid_width = ((width + cell_size - 1.0) / cell_size).ceil() as u32;
        let grid_height = ((height + cell_size - 1.0) / cell_size).ceil() as u32;
        let num_cells = grid_width * grid_height;
        let grid_counts = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Grid Counts"),
            size: (std::mem::size_of::<u32>() as u64) * (num_cells as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let grid_indices_len = (num_cells as u64) * (cell_capacity as u64);
        let grid_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Grid Indices"),
            size: (std::mem::size_of::<u32>() as u64) * grid_indices_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let grid_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Grid Params"),
            size: std::mem::size_of::<GridParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize grid params
        let gp = GridParams {
            particle_count: num_points,
            grid_width,
            grid_height,
            cell_capacity,
            cell_size,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        queue.write_buffer(&grid_params, 0, bytemuck::bytes_of(&gp));

        // Create grid pipelines and bind groups
        let grid_clear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Grid Clear"),
            source: wgpu::ShaderSource::Wgsl(GRID_CLEAR_SHADER.into()),
        });
        let grid_populate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Grid Populate"),
            source: wgpu::ShaderSource::Wgsl(GRID_POPULATE_SHADER.into()),
        });

        let grid_clear_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
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
            label: Some("VCA Grid Clear BGL"),
        });
        let grid_populate_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("VCA Grid Populate BGL"),
        });
        let grid_clear_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VCA Grid Clear Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("VCA Grid Clear PL"),
                        bind_group_layouts: &[&grid_clear_bgl],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &grid_clear_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let grid_populate_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VCA Grid Populate Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("VCA Grid Populate PL"),
                        bind_group_layouts: &[&grid_populate_bgl],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &grid_populate_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let grid_clear_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Grid Clear BG"),
            layout: &grid_clear_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_counts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grid_params.as_entire_binding(),
                },
            ],
        });
        let grid_populate_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Grid Populate BG"),
            layout: &grid_populate_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: grid_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_counts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: grid_params.as_entire_binding(),
                },
            ],
        });

        // Compute bind groups
        let compute_neighbor_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VoronoiCA Compute Neighbor BG"),
            layout: &compute_neighbor_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: grid_counts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: grid_params.as_entire_binding(),
                },
            ],
        });
        let compute_update_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VoronoiCA Compute Update BG"),
            layout: &compute_update_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let voronoi_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VCA Voronoi Render BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    // params
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // vertices (states)
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // LUT buffer
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // no separate fill/blit path in the simplified renderer
        let voronoi_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Voronoi Render Params"),
            size: std::mem::size_of::<VoronoiParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // init params
        let initial_params = VoronoiParams {
            count: 0.0,
            color_mode: 0.0,
            neighbor_radius: 60.0,
            border_enabled: 1.0,
            border_threshold: 0.5,
            filter_mode: match app_settings.texture_filtering {
                crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
            },
            resolution_x: width,
            resolution_y: height,
        };
        queue.write_buffer(&voronoi_params, 0, bytemuck::bytes_of(&initial_params));

        // Create LUT buffer with default LUT
        let lut_manager = crate::simulations::shared::LutManager::new();
        let default_lut_name = "MATPLOTLIB_YlGnBu".to_string();
        let lut_data = lut_manager.get(&default_lut_name).unwrap();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VCA LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data.to_u32_buffer()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let voronoi_render_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Voronoi Render BG"),
            layout: &voronoi_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: voronoi_params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        // (removed)

        // Voronoi fullscreen render pipeline
        let voronoi_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VCA Voronoi Render PL"),
                bind_group_layouts: &[&voronoi_bgl],
                push_constant_ranges: &[],
            });
        let voronoi_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VCA Voronoi Render Pipeline"),
                layout: Some(&voronoi_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &voronoi_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &voronoi_render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
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

        // Create offscreen display texture that we'll tile infinitely to the surface
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VCA Display Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("VCA Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: app_settings.texture_filtering.into(),
            min_filter: app_settings.texture_filtering.into(),
            mipmap_filter: app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // Infinite render pipeline using VCA-specific shader
        let render_infinite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Render Infinite Shader"),
            source: wgpu::ShaderSource::Wgsl(VCA_INFINITE_RENDER_SHADER.into()),
        });

        // Bind group for texture+sampler + render params (filtering mode)
        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VCA Render Infinite BGL"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
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

        // Camera and bind group
        let camera = Camera::new(device, width, height)?;
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VCA Camera BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Camera BG"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
        });

        let render_infinite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VCA Render Infinite PL"),
                bind_group_layouts: &[
                    &render_infinite_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VCA Render Infinite Pipeline"),
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
                        blend: None,
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
        // Render params buffer (filtering mode)
        let filtering_mode_u32: u32 = match app_settings.texture_filtering {
            crate::commands::app_settings::TextureFiltering::Nearest => 0,
            crate::commands::app_settings::TextureFiltering::Linear => 1,
            crate::commands::app_settings::TextureFiltering::Lanczos => 2,
        };
        let texture_render_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("VCA Texture Render Params Buffer"),
                contents: bytemuck::cast_slice(&[filtering_mode_u32, 0u32, 0u32, 0u32]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Render Infinite BG"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&display_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&display_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: texture_render_params_buffer.as_entire_binding(),
                },
            ],
        });

        let post_processing_resources = PostProcessingResources::new(device, surface_config)?;
        // Camera was created above
        let post_processing_state = PostProcessingState::default();

        Ok(Self {
            voronoi_render_pipeline,
            compute_pipeline,
            compute_update_pipeline,
            uniform_buffer,
            compute_neighbor_bg,
            compute_update_bg,
            vertex_buffer,
            grid_indices,
            grid_counts,
            grid_params,
            grid_clear_pipeline,
            grid_populate_pipeline,
            grid_clear_bg,
            grid_populate_bg,
            voronoi_params,
            voronoi_render_bg,
            num_points,
            time_accum: 0.0,
            time_scale: 1.0,
            drift: 1.0,
            neighbor_radius: 60.0,
            alive_threshold: 0.5,
            resolution: [width, height],
            gui_visible: true,
            points,
            // LUT + coloring defaults
            lut_buffer,
            current_lut_name: default_lut_name,
            lut_reversed: false,
            color_mode: 1,
            borders_enabled: true,
            border_threshold: 0.96,
            app_settings: app_settings.clone(),
            camera,
            camera_bind_group,
            display_texture,
            display_view,
            display_sampler,
            texture_render_params_buffer,
            render_infinite_pipeline,
            render_infinite_bind_group,
            post_processing_state,
            post_processing_resources,
            // run-and-tumble defaults
            headings,
            run_time_left,
            tumble_time_left,
            run_speed: 10.0,   // px/sec
            avg_run_time: 1.5, // seconds
            tumble_time: 0.2,  // seconds
            cursor_size: 0.20,
            cursor_strength: 1.0,
        })
    }

    fn rebuild_points(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_count: u32,
    ) -> SimulationResult<()> {
        // Recreate points and motion arrays
        let mut rng = rand::rng();
        let mut points: Vec<Vertex> = Vec::with_capacity(new_count as usize);
        let mut headings: Vec<f32> = Vec::with_capacity(new_count as usize);
        let mut run_time_left: Vec<f32> = Vec::with_capacity(new_count as usize);
        let mut tumble_time_left: Vec<f32> = Vec::with_capacity(new_count as usize);
        for _ in 0..new_count {
            points.push(Vertex {
                position: [
                    rng.random_range(0.0..(self.resolution[0] as f32)),
                    rng.random_range(0.0..(self.resolution[1] as f32)),
                ],
                state: if rng.random::<f32>() > 0.7 { 1.0 } else { 0.0 },
                pad0: 0.0,
                age: 0.0,
                alive_neighbors: 0,
                dead_neighbors: 0,
                pad1: 0,
            });
            headings.push(rng.random::<f32>() * std::f32::consts::TAU);
            // Exponential sample for initial run duration
            let u: f32 = (1.0 - rng.random::<f32>()).max(1e-6);
            let run_sample = -self.avg_run_time.max(0.05) * u.ln();
            run_time_left.push(run_sample);
            tumble_time_left.push(0.0);
        }

        // Recreate GPU vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Vertex Buffer"),
            size: (std::mem::size_of::<Vertex>() * new_count as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&points));

        // Recreate bind groups that reference vertex buffer
        // Neighbor count BG
        let compute_neighbor_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("voronoi_ca_compute_neighbor_bgl_tmp"),
            });
        self.compute_neighbor_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_neighbor_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.grid_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.grid_counts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.grid_params.as_entire_binding(),
                },
            ],
            label: Some("voronoi_ca_compute_neighbor_bg"),
        });
        // Update pass BG
        let compute_update_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
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
                label: Some("voronoi_ca_compute_update_bgl_tmp"),
            });
        self.compute_update_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_update_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("voronoi_ca_compute_update_bg"),
        });

        let voronoi_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VCA Voronoi Render BGL Rebind"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    // params
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // vertices (states)
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // LUT buffer
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        self.voronoi_render_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Voronoi Render BG Rebind"),
            layout: &voronoi_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.voronoi_params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.lut_buffer.as_entire_binding(),
                },
            ],
        });

        // Recreate grid populate BG which also depends on vertex buffer
        let grid_populate_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("VCA Grid Populate BGL Rebind"),
        });
        self.grid_populate_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Grid Populate BG Rebind"),
            layout: &grid_populate_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.grid_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.grid_counts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.grid_params.as_entire_binding(),
                },
            ],
        });

        // Update fields
        self.points = points;
        self.headings = headings;
        self.run_time_left = run_time_left;
        self.tumble_time_left = tumble_time_left;
        self.vertex_buffer = vertex_buffer;
        self.num_points = new_count;
        Ok(())
    }
}

impl Simulation for VoronoiCASimulation {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        // Update time and uniforms
        let dt = delta_time * self.time_scale.max(0.0);
        self.time_accum += dt;
        let uniforms = Uniforms {
            resolution: self.resolution,
            time: self.time_accum,
            drift: self.drift,
            rule_type: 0,
            neighbor_radius: self.neighbor_radius,
            alive_threshold: self.alive_threshold,
            _pad0: 0,
        };
        // Only update time/drift fields; write the full struct for simplicity
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Run-and-tumble update on CPU and write back to GPU
        let mut rng = rand::rng();
        let min_x = 0.0f32;
        let min_y = 0.0f32;
        let size_x = self.resolution[0] as f32;
        let size_y = self.resolution[1] as f32;
        let speed = self.run_speed.max(0.0) * self.drift.max(0.0); // px/sec
        for i in 0..(self.num_points as usize) {
            // Progress timers
            if self.tumble_time_left[i] > 0.0 {
                self.tumble_time_left[i] = (self.tumble_time_left[i] - dt).max(0.0);
                // small random heading jitter while tumbling
                self.headings[i] += (rng.random::<f32>() - 0.5) * 0.5 * dt; // ~0.5 rad/s jitter
                if self.tumble_time_left[i] == 0.0 {
                    // choose a new heading uniformly
                    self.headings[i] = rng.random::<f32>() * std::f32::consts::TAU;
                    // sample new run duration from exponential with mean avg_run_time
                    let u: f32 = (1.0 - rng.random::<f32>()).max(1e-6);
                    self.run_time_left[i] = -self.avg_run_time.max(0.05) * u.ln();
                }
            } else if self.run_time_left[i] > 0.0 {
                // running: move forward with toroidal wrap
                self.run_time_left[i] = (self.run_time_left[i] - dt).max(0.0);
                let dx = self.headings[i].cos() * speed * dt;
                let dy = self.headings[i].sin() * speed * dt;
                self.points[i].position[0] =
                    (self.points[i].position[0] + dx - min_x).rem_euclid(size_x) + min_x;
                self.points[i].position[1] =
                    (self.points[i].position[1] + dy - min_y).rem_euclid(size_y) + min_y;
                if self.run_time_left[i] == 0.0 {
                    self.tumble_time_left[i] = self.tumble_time.max(0.0);
                }
            } else {
                // initialize state if both zero
                let u: f32 = (1.0 - rng.random::<f32>()).max(1e-6);
                self.run_time_left[i] = -self.avg_run_time.max(0.05) * u.ln();
            }
        }
        // Sync updated CPU-side positions to the GPU so compute uses latest data
        // This writes the entire array for simplicity; can be optimized later
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.points));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VoronoiCA Encoder"),
        });

        // Update grid params each frame (particle count and possibly cell size)
        let gp = GridParams {
            particle_count: self.num_points,
            grid_width: (((self.resolution[0] as f32) + (self.neighbor_radius * 1.25).max(8.0) - 1.0)
                / (self.neighbor_radius * 1.25).max(8.0))
            .ceil() as u32,
            grid_height: (((self.resolution[1] as f32) + (self.neighbor_radius * 1.25).max(8.0) - 1.0)
                / (self.neighbor_radius * 1.25).max(8.0))
            .ceil() as u32,
            cell_capacity: 64,
            cell_size: (self.neighbor_radius * 1.25).max(8.0),
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        queue.write_buffer(&self.grid_params, 0, bytemuck::bytes_of(&gp));

        // Clear and populate spatial grid
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Grid Clear"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.grid_clear_pipeline);
            cpass.set_bind_group(0, &self.grid_clear_bg, &[]);
            let total_cells = gp.grid_width * gp.grid_height;
            cpass.dispatch_workgroups(total_cells.div_ceil(64), 1, 1);
        }
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Grid Populate"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.grid_populate_pipeline);
            cpass.set_bind_group(0, &self.grid_populate_bg, &[]);
            cpass.dispatch_workgroups(self.num_points.div_ceil(64), 1, 1);
        }

        // Compute passes: 1) neighbor count, 2) state update
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VoronoiCA Neighbor Count"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_neighbor_bg, &[]);
            let wg_count = self.num_points.div_ceil(64);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VoronoiCA State Update"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_update_pipeline);
            cpass.set_bind_group(0, &self.compute_update_bg, &[]);
            let wg_count = self.num_points.div_ceil(64);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }

        // Update uniform params (count and color mode)
        let params = VoronoiParams {
            count: self.num_points as f32,
            color_mode: self.color_mode as f32,
            neighbor_radius: self.neighbor_radius,
            border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
            border_threshold: self.border_threshold,
            filter_mode: match self.app_settings.texture_filtering {
                crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
            },
            resolution_x: self.resolution[0],
            resolution_y: self.resolution[1],
        };
        queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));

        // Update camera and upload to GPU
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        // Fullscreen Voronoi render to offscreen display texture (uses EDT texture)
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VoronoiCA Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
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
            rpass.set_pipeline(&self.voronoi_render_pipeline);
            // Update params before draw
            let params = VoronoiParams {
                count: self.num_points as f32,
                color_mode: self.color_mode as f32,
                neighbor_radius: self.neighbor_radius,
                border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
                border_threshold: self.border_threshold,
                filter_mode: match self.app_settings.texture_filtering {
                    crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                    crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                    crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
                },
                resolution_x: self.resolution[0],
                resolution_y: self.resolution[1],
            };
            queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));

            rpass.set_bind_group(0, &self.voronoi_render_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Optional blur: display_view -> intermediate_view, then copy back to display
        if self.post_processing_state.blur_filter.enabled {
            let radius = self.post_processing_state.blur_filter.radius;
            let sigma = self.post_processing_state.blur_filter.sigma;
            self.post_processing_resources.update_blur_params(
                queue,
                radius,
                sigma,
                self.resolution[0] as u32,
                self.resolution[1] as u32,
            );

            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VCA Blur Bind Group"),
                layout: &self
                    .post_processing_resources
                    .blur_pipeline
                    .get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.display_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &self.post_processing_resources.blur_sampler,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self
                            .post_processing_resources
                            .blur_params_buffer
                            .as_entire_binding(),
                    },
                ],
            });

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("VCA PostProcess Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.post_processing_resources.intermediate_view,
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
                rpass.set_pipeline(&self.post_processing_resources.blur_pipeline);
                rpass.set_bind_group(0, &blur_bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }

            encoder.copy_texture_to_texture(
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
                    width: self.resolution[0] as u32,
                    height: self.resolution[1] as u32,
                    depth_or_array_layers: 1,
                },
            );
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Infinite tiling pass to the surface
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VCA Infinite Surface Encoder"),
        });
        {
            let mut rpass = encoder2.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VCA Infinite Surface Render Pass"),
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
            rpass.set_pipeline(&self.render_infinite_pipeline);
            rpass.set_bind_group(0, &self.render_infinite_bind_group, &[]);
            rpass.set_bind_group(1, &self.camera_bind_group, &[]);
            rpass.draw(0..6, 0..total_instances);
        }
        queue.submit(std::iter::once(encoder2.finish()));
        Ok(())
    }

    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Update camera
        self.camera.update(0.016);
        self.camera.upload_to_gpu(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VoronoiCA Static Encoder"),
        });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VoronoiCA Render Pass Static"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
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
            rpass.set_pipeline(&self.voronoi_render_pipeline);

            rpass.draw(0..3, 0..1);
        }

        // Infinite tiling pass to the surface
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VCA Static Infinite Surface Encoder"),
        });
        {
            let mut rpass = encoder2.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VCA Static Infinite Surface Render Pass"),
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
            rpass.set_pipeline(&self.render_infinite_pipeline);
            rpass.set_bind_group(0, &self.render_infinite_bind_group, &[]);
            rpass.set_bind_group(1, &self.camera_bind_group, &[]);
            rpass.draw(0..6, 0..total_instances);
        }
        queue.submit(std::iter::once(encoder2.finish()));

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        // Update resolution and camera
        self.resolution = [
            new_config.width.max(1) as f32,
            new_config.height.max(1) as f32,
        ];
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        // Redistribute points to match new resolution
        let mut rng = rand::rng();
        for i in 0..(self.num_points as usize) {
            self.points[i].position = [
                rng.random_range(0.0..self.resolution[0]),
                rng.random_range(0.0..self.resolution[1]),
            ];
        }
        // Update GPU buffer with new positions
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.points));

        // Recreate display texture and related bind group
        self.display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VCA Display Texture"),
            size: wgpu::Extent3d {
                width: new_config.width,
                height: new_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: new_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.display_view = self
            .display_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("VCA Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: self.app_settings.texture_filtering.into(),
            min_filter: self.app_settings.texture_filtering.into(),
            mipmap_filter: self.app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // Recreate infinite bind group and pipeline for new surface format
        let render_infinite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Render Infinite Shader"),
            source: wgpu::ShaderSource::Wgsl(VCA_INFINITE_RENDER_SHADER.into()),
        });
        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VCA Render Infinite BGL"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
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
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VCA Camera BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let render_infinite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VCA Render Infinite PL"),
                bind_group_layouts: &[
                    &render_infinite_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        self.render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VCA Render Infinite Pipeline"),
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
                        blend: None,
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
        self.render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Render Infinite BG"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.display_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.display_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.texture_render_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Recreate camera bind group
        self.camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Camera BG"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera.buffer().as_entire_binding(),
            }],
        });

        // Resize post-processing resources
        self.post_processing_resources.resize(device, new_config)?;

        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "drift" => {
                if let Some(v) = value.as_f64() {
                    self.drift = v as f32;
                }
            }
            "neighborRadius" => {
                if let Some(v) = value.as_f64() {
                    self.neighbor_radius = v as f32;
                }
            }
            "aliveThreshold" => {
                if let Some(v) = value.as_f64() {
                    self.alive_threshold = v as f32;
                }
            }
            "runSpeed" => {
                if let Some(v) = value.as_f64() {
                    self.run_speed = v as f32;
                }
            }
            "avgRunTime" => {
                if let Some(v) = value.as_f64() {
                    self.avg_run_time = v as f32;
                }
            }
            "tumbleTime" => {
                if let Some(v) = value.as_f64() {
                    self.tumble_time = v as f32;
                }
            }
            "timeScale" => {
                if let Some(v) = value.as_f64() {
                    self.time_scale = v as f32;
                }
            }
            "numPoints" => {
                if let Some(v) = value.as_u64() {
                    let new_count = (v as u32).max(1);
                    self.rebuild_points(_device, _queue, new_count)?;
                }
            }
            "cursor_size" => {
                if let Some(v) = value.as_f64() {
                    self.cursor_size = v as f32;
                }
            }
            "cursor_strength" => {
                if let Some(v) = value.as_f64() {
                    self.cursor_strength = v as f32;
                }
            }
            "coloringMode" => {
                if let Some(s) = value.as_str() {
                    self.color_mode = match s {
                        "Random" => 0,
                        "Density" => 1,
                        "Age" => 2,
                        "Binary" => 3,
                        _ => self.color_mode,
                    };
                }
            }
            "bordersEnabled" => {
                if let Some(b) = value.as_bool() {
                    self.borders_enabled = b;
                }
            }
            "borderThreshold" => {
                if let Some(v) = value.as_f64() {
                    self.border_threshold = v as f32;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::json!({
            "drift": self.drift,
            "neighborRadius": self.neighbor_radius,
            "aliveThreshold": self.alive_threshold,
            "runSpeed": self.run_speed,
            "avgRunTime": self.avg_run_time,
            "tumbleTime": self.tumble_time,
            "cursor_size": self.cursor_size,
            "cursor_strength": self.cursor_strength,
            "currentLutName": self.current_lut_name,
            "lutReversed": self.lut_reversed,
            "coloringMode": match self.color_mode { 0 => "Random", 1 => "Density", 2 => "Age", 3 => "Binary", _ => "Random" },
            "bordersEnabled": self.borders_enabled,
            "borderThreshold": self.border_threshold
        })
    }

    fn get_state(&self) -> Value {
        serde_json::json!({ "num_points": self.num_points })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Convert world coordinates to simulation coordinates
        // World coords are in [-1,1] range, need to convert to [0, width] x [0, height]
        // Account for camera position and zoom
        let camera_x = self.camera.position[0];
        let camera_y = self.camera.position[1];
        let zoom = self.camera.zoom;

        // Use cursor size as world space radius, convert to simulation space
        let world_radius = self.cursor_size;
        let sim_radius = world_radius * zoom * (self.resolution[0].min(self.resolution[1]) as f32) * 0.5;
        let radius_sq = sim_radius * sim_radius;

        // Apply camera transform to get local coordinates in [-1,1] range
        let local_x = (world_x - camera_x) / zoom;
        let local_y = (world_y - camera_y) / zoom;

        // Convert from [-1,1] range to [0, resolution] range
        // Flip Y axis: world Y increases upward, simulation Y increases downward
        let x = (local_x + 1.0) * 0.5 * self.resolution[0] as f32;
        let y = (1.0 - local_y) * 0.5 * self.resolution[1] as f32;

        // Buttons: 0 = left => set alive, 2 = right => set dead
        let set_alive = mouse_button == 0;
        let set_dead = mouse_button == 2;
        if !(set_alive || set_dead) {
            return Ok(());
        }

        tracing::debug!(
            "VCA mouse interaction: world=({:.3}, {:.3}), sim=({:.1}, {:.1}), button={}, world_radius={:.3}, sim_radius={:.1}",
            world_x, world_y, x, y, mouse_button, world_radius, sim_radius
        );

        // Apply toroidal wrapping for distance calculation (matching the render shader)
        let w = self.resolution[0] as f32;
        let h = self.resolution[1] as f32;

        for v in &mut self.points {
            let mut dx = v.position[0] - x;
            let mut dy = v.position[1] - y;

            // Apply toroidal wrapping (same as in render shader)
            if dx > 0.5 * w {
                dx -= w;
            }
            if dx < -0.5 * w {
                dx += w;
            }
            if dy > 0.5 * h {
                dy -= h;
            }
            if dy < -0.5 * h {
                dy += h;
            }

            let d2 = dx * dx + dy * dy;
            if d2 <= radius_sq {
                if set_alive {
                    v.state = 1.0;
                } else if set_dead {
                    v.state = 0.0;
                }
            }
        }
        
        // Count how many points were modified
        let modified_count = self.points.iter().filter(|p| {
            let mut dx = p.position[0] - x;
            let mut dy = p.position[1] - y;
            if dx > 0.5 * w { dx -= w; }
            if dx < -0.5 * w { dx += w; }
            if dy > 0.5 * h { dy -= h; }
            if dy < -0.5 * h { dy += h; }
            let d2 = dx * dx + dy * dy;
            d2 <= radius_sq
        }).count();
        
        tracing::debug!("VCA mouse interaction: modified {} points", modified_count);
        
        // Push updated states to GPU immediately so painting is visible next frame
        _queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.points));
        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
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

    fn get_camera_state(&self) -> Value {
        serde_json::json!({ "position": [self.camera.position[0], self.camera.position[1]], "zoom": self.camera.zoom })
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        Ok(())
    }

    fn apply_settings(
        &mut self,
        _settings: serde_json::Value,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.gui_visible = !self.gui_visible;
        self.gui_visible
    }

    fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Randomize drift a bit as an example
        self.drift = 0.2 + rand::rng().random::<f32>() * 0.8;
        Ok(())
    }
}
