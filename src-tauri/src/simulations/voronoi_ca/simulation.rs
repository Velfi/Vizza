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
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::ping_pong_textures::PingPongTextures;
use crate::simulations::traits::Simulation;

use super::shaders::{
    ADJACENCY_BUILD_SHADER, ADJACENCY_COUNT_SHADER, BROWNIAN_SHADER, COMPUTE_UPDATE_SHADER,
    GRID_CLEAR_SHADER, GRID_POPULATE_SHADER, JFA_INIT_SHADER, JFA_ITERATION_SHADER,
    VCA_INFINITE_RENDER_SHADER, VORONOI_RENDER_JFA_SHADER,
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
    random_state: u32, // Per-point random state for independent brownian motion
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct VoronoiParams {
    count: f32,
    color_mode: f32,
    // Borders enabled flag (1.0 = on, 0.0 = off)
    border_enabled: f32,
    // Border width in pixels (0.0-10.0)
    border_width: f32,
    // Texture filtering mode: 0=Nearest, 1=Linear, 2=Lanczos (TODO treated as Linear here)
    filter_mode: f32,
    resolution_x: f32,
    resolution_y: f32,
    jump_distance: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    drift: f32,
    rule_type: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct BrownianParams {
    speed: f32,
    delta_time: f32,
}

/// Voronoi Cellular Automata simulation with intelligent JFA caching.
///
/// This simulation uses a Jump Flood Algorithm (JFA) to compute Voronoi diagrams
/// and applies cellular automata rules to evolve the system. The key improvement
/// is smart caching of JFA textures - they are only rebuilt when necessary
/// (e.g., when points change due to painting), providing much better performance
/// when paused compared to the previous approach of rebuilding every frame.
///
/// The rendering pipeline is split into modular stages:
/// - JFA computation (with quality/performance trade-offs)
/// - Display texture rendering
/// - Post-processing effects
/// - Surface rendering with infinite tiling
#[derive(Debug)]
pub struct VoronoiCASimulation {
    pub state: super::state::State,
    voronoi_render_jfa_pipeline: RenderPipeline,
    // Compute
    compute_update_pipeline: ComputePipeline, // state update
    brownian_pipeline: ComputePipeline,       // brownian motion
    uniform_buffer: Buffer,
    brownian_params_buffer: Buffer,
    // Bind groups
    compute_neighbor_bg: BindGroup,
    compute_update_bg: BindGroup,
    brownian_bg: BindGroup,
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

    num_points: u32,
    time_accum: f32,
    time_scale: f32,
    last_ca_update_time: f32,
    drift: f32,
    resolution: [f32; 2],
    gui_visible: bool,
    points: Vec<Vertex>,
    // Brownian motion parameters
    brownian_speed: f32, // pixels per second
    // Cursor config (settings)
    cursor_size: f32,
    cursor_strength: f32,
    // Post-processing
    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
    // LUT + coloring
    pub lut_buffer: Buffer,
    pub current_color_scheme: String,
    pub color_scheme_reversed: bool,
    color_mode: u32, // 0=Random, 1=Density, 2=Age
    borders_enabled: bool,
    pub border_width: f32, // Border width in pixels
    app_settings: crate::commands::app_settings::AppSettings,
    // VCA settings
    rulestring: String,
    // Camera
    pub camera: Camera,
    camera_bind_group: BindGroup,
    // Offscreen display for infinite tiling
    display_texture: Texture,
    display_view: TextureView,
    display_sampler: wgpu::Sampler,
    texture_render_params_buffer: Buffer,
    render_infinite_pipeline: RenderPipeline,
    // JFA resources
    jfa_textures: PingPongTextures,

    jfa_init_pipeline: ComputePipeline,
    jfa_iteration_pipeline: ComputePipeline,
    jfa_init_bg: BindGroup,
    jfa_iteration_bg_a: BindGroup,
    jfa_iteration_bg_b: BindGroup,
    /// Whether the current JFA texture is valid and can be reused.
    /// This flag prevents unnecessary JFA rebuilding when the simulation is paused
    /// and no points have changed. Invalidated when points change due to painting,
    /// point count changes, resolution changes, or simulation reset.
    has_valid_jfa: bool,

    /// Flag to skip CA state updates on the next frame after painting
    /// This prevents unwanted births when a cell is painted while running
    skip_next_state_update: bool,

    // Voronoi adjacency resources
    adjacency_neighbors: Buffer, // flattened u32 [num_points * MAX_NEIGHBORS]
    adjacency_degrees: Buffer,   // u32 [num_points]
    adjacency_build_pipeline: ComputePipeline,
    adjacency_count_pipeline: ComputePipeline,
    adjacency_build_bg: BindGroup,
    adjacency_count_bg: BindGroup,
}

impl VoronoiCASimulation {
    /// Get the current JFA texture view based on the current texture flag
    fn get_current_jfa_view(&self) -> &TextureView {
        self.jfa_textures.current_view()
    }

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

    /// Rebuild the JFA texture with current point positions.
    ///
    /// This method runs a reduced-quality JFA algorithm for faster feedback
    /// when painting. It uses fewer iterations (min_jump = 4) compared to
    /// the full-quality version (min_jump = 1) used during active simulation.
    ///
    /// The quality difference is minimal for painting operations but provides
    /// significant performance improvements.
    fn rebuild_jfa_texture(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> SimulationResult<()> {
        // Rebind JFA init bind group to ensure it targets the CURRENT ping-pong texture view
        // so that paused painting writes seed sites into the correct texture before iterations.
        self.jfa_init_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA JFA Init BG (Simple Rebind)"),
            layout: &self.jfa_init_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::buffer_entry(0, &self.vertex_buffer),
                resource_helpers::buffer_entry(1, &self.voronoi_params),
                resource_helpers::texture_view_entry(2, self.jfa_textures.current_view()),
            ],
        });

        // Initialize JFA with current points
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA JFA Init (Simple)"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.jfa_init_pipeline);
            cpass.set_bind_group(0, &self.jfa_init_bg, &[]);
            let wg_x = (self.resolution[0] as u32).div_ceil(8);
            let wg_y = (self.resolution[1] as u32).div_ceil(8);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Run JFA iterations. Use full-quality to avoid incorrect region labeling
        // when painting while paused (coarse jumps can over-assign cells to the
        // clicked site).
        let mut jump_distance =
            (self.resolution[0].max(self.resolution[1]) as u32).next_power_of_two();

        // Use full iterations for accuracy during painting
        let min_jump = 1; // Accurate JFA to prevent region bleed when paused

        while jump_distance >= min_jump {
            // Update params with current jump distance
            let params = VoronoiParams {
                count: self.num_points as f32,
                color_mode: self.color_mode as f32,
                border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
                border_width: self.border_width,
                filter_mode: match self.app_settings.texture_filtering {
                    crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                    crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                    crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
                },
                resolution_x: self.resolution[0],
                resolution_y: self.resolution[1],
                jump_distance: jump_distance as f32,
            };
            queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));

            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA JFA Iteration (Simple)"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.jfa_iteration_pipeline);

            // Use the appropriate bind group based on current texture state
            let bind_group = self
                .jfa_textures
                .get_bind_group(&self.jfa_iteration_bg_a, &self.jfa_iteration_bg_b);
            cpass.set_bind_group(0, bind_group, &[]);

            let wg_x = (self.resolution[0] as u32).div_ceil(8);
            let wg_y = (self.resolution[1] as u32).div_ceil(8);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);

            // Swap textures for next iteration
            self.jfa_textures.swap();
            jump_distance /= 2;
        }

        // After building JFA, immediately rebuild adjacency so paused painting has correct graph
        let current_jfa_view = self.get_current_jfa_view();
        self.adjacency_build_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Adjacency Build BG (Simple Rebind)"),
            layout: &self.adjacency_build_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::buffer_entry(0, &self.vertex_buffer),
                resource_helpers::buffer_entry(1, &self.uniform_buffer),
                resource_helpers::buffer_entry(2, &self.adjacency_neighbors),
                resource_helpers::buffer_entry(3, &self.adjacency_degrees),
                resource_helpers::texture_view_entry(4, current_jfa_view),
            ],
        });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Adjacency Build (Simple)"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.adjacency_build_pipeline);
            cpass.set_bind_group(0, &self.adjacency_build_bg, &[]);
            let wg_count = self.num_points.div_ceil(128);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }

        // Count neighbors so density-based coloring is correct while paused
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Adjacency Neighbor Count (Simple)"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.adjacency_count_pipeline);
            cpass.set_bind_group(0, &self.adjacency_count_bg, &[]);
            let wg_count = self.num_points.div_ceil(128);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }

        Ok(())
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
            rule_type: 0, // Will be updated when rulestring is set
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
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
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
                label: Some("voronoi_ca_compute_update_bgl"),
            });

        // Brownian bind group layout
        let brownian_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
            ],
            label: Some("voronoi_ca_brownian_bgl"),
        });

        // Neighbor count BG layout: vertices, uniforms, grid_indices, grid_counts, grid_params
        let compute_neighbor_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false), // vertices
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE), // uniforms
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, true), // grid_indices
                    resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true), // grid_counts
                    resource_helpers::uniform_buffer_entry(4, wgpu::ShaderStages::COMPUTE), // grid params
                ],
                label: Some("voronoi_ca_compute_neighbor_bgl"),
            });

        // Shaders
        let compute_update_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VoronoiCA Compute Update Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPUTE_UPDATE_SHADER.into()),
        });
        let brownian_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Brownian Shader"),
            source: wgpu::ShaderSource::Wgsl(BROWNIAN_SHADER.into()),
        });

        let voronoi_render_jfa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Voronoi Render JFA"),
            source: wgpu::ShaderSource::Wgsl(VORONOI_RENDER_JFA_SHADER.into()),
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

        // Pipeline for brownian motion
        let brownian_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VCA Brownian Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("VCA Brownian PL"),
                    bind_group_layouts: &[&brownian_bgl],
                    push_constant_ranges: &[],
                }),
            ),
            module: &brownian_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let mut rng = rand::rng();
        let num_points = 300u32;
        let mut points: Vec<Vertex> = Vec::with_capacity(num_points as usize);

        for _ in 0..num_points {
            points.push(Vertex {
                position: [
                    rng.random_range(0.0..(width)),
                    rng.random_range(0.0..(height)),
                ],
                state: if rng.random::<f32>() > 0.3 { 1.0 } else { 0.0 }, // More dead cells for painting
                pad0: 0.0,
                age: 0.0,
                alive_neighbors: 0,
                dead_neighbors: 0,
                random_state: rng.random::<u32>(),
            });
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VoronoiCA Vertex Buffer"),
            contents: bytemuck::cast_slice(&points),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
        });

        // Create brownian params buffer
        let brownian_params = BrownianParams {
            speed: 10.0,
            delta_time: 0.016, // Will be updated each frame
        };
        let brownian_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VCA Brownian Params Buffer"),
            contents: bytemuck::bytes_of(&brownian_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create spatial grid buffers with adaptive parameters
        let total_area = width * height;
        let density = num_points as f32 / total_area;
        let cell_capacity: u32 = if density > 0.01 {
            256
        } else if density > 0.005 {
            192
        } else {
            128
        };
        let base_cell_size: f32 = 90.0; // Fixed cell size for spatial partitioning
        let cell_size: f32 = if density > 0.01 {
            base_cell_size.max(8.0)
        } else {
            base_cell_size.max(12.0)
        };
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
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
            ],
            label: Some("VCA Grid Clear BGL"),
        });
        let grid_populate_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::COMPUTE),
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
        let grid_clear_bg = resource_helpers::create_buffer_bind_group(
            &device,
            &grid_clear_bgl,
            "VCA Grid Clear BG",
            &[&grid_counts, &grid_params],
        );
        let grid_populate_bg = resource_helpers::create_buffer_bind_group(
            &device,
            &grid_populate_bgl,
            "VCA Grid Populate BG",
            &[&vertex_buffer, &grid_indices, &grid_counts, &grid_params],
        );

        // Compute bind groups
        let compute_neighbor_bg = resource_helpers::create_buffer_bind_group(
            &device,
            &compute_neighbor_bgl,
            "VoronoiCA Compute Neighbor BG",
            &[
                &vertex_buffer,
                &uniform_buffer,
                &grid_indices,
                &grid_counts,
                &grid_params,
            ],
        );
        let compute_update_bg = resource_helpers::create_buffer_bind_group(
            &device,
            &compute_update_bgl,
            "VoronoiCA Compute Update BG",
            &[&vertex_buffer, &uniform_buffer],
        );

        let brownian_bg = resource_helpers::create_buffer_bind_group(
            &device,
            &brownian_bgl,
            "VCA Brownian BG",
            &[&vertex_buffer, &uniform_buffer, &brownian_params_buffer],
        );

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
            border_enabled: 1.0,
            border_width: 1.0,
            filter_mode: match app_settings.texture_filtering {
                crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
            },
            resolution_x: width,
            resolution_y: height,
            jump_distance: 0.0, // Not used in init
        };
        queue.write_buffer(&voronoi_params, 0, bytemuck::bytes_of(&initial_params));

        // Create LUT buffer with default LUT
        let color_scheme_manager = crate::simulations::shared::ColorSchemeManager::new();
        let default_lut_name = "MATPLOTLIB_YlGnBu".to_string();
        let lut_data = color_scheme_manager.get(&default_lut_name).unwrap();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("VCA LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data.to_u32_buffer()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create JFA render pipeline with additional bindings for JFA texture
        let voronoi_jfa_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("VCA Voronoi JFA Render BGL"),
            entries: &[
                resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                ),
                resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::FRAGMENT, true),
                resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::FRAGMENT, true),
                resource_helpers::texture_entry(
                    3,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::TextureSampleType::Float { filterable: false },
                    wgpu::TextureViewDimension::D2,
                ),
            ],
        });

        let voronoi_jfa_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("VCA Voronoi JFA Render PL"),
                bind_group_layouts: &[&voronoi_jfa_bgl],
                push_constant_ranges: &[],
            });

        let voronoi_render_jfa_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("VCA Voronoi JFA Render Pipeline"),
                layout: Some(&voronoi_jfa_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &voronoi_render_jfa_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &voronoi_render_jfa_shader,
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
                ],
            });

        // Camera and bind group
        let camera = Camera::new(device, width, height)?;
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VCA Camera BGL"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                )],
            });
        let camera_bind_group = resource_helpers::create_buffer_bind_group(
            &device,
            &camera_bind_group_layout,
            "VCA Camera BG",
            &[camera.buffer()],
        );

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

        let post_processing_resources = PostProcessingResources::new(device, surface_config)?;
        // Camera was created above
        let post_processing_state = PostProcessingState::default();

        // Create JFA textures using PingPongTextures
        let jfa_textures = PingPongTextures::new(
            device,
            surface_config.width,
            surface_config.height,
            wgpu::TextureFormat::Rgba32Float,
            "VCA JFA Texture",
        );

        // Create JFA shader modules
        let jfa_init_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA JFA Init Shader"),
            source: wgpu::ShaderSource::Wgsl(JFA_INIT_SHADER.into()),
        });
        let jfa_iteration_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA JFA Iteration Shader"),
            source: wgpu::ShaderSource::Wgsl(JFA_ITERATION_SHADER.into()),
        });

        // Create JFA bind group layouts
        let jfa_init_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_texture_entry(
                    2,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba32Float,
                ),
            ],
            label: Some("VCA JFA Init BGL"),
        });

        let jfa_iteration_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::COMPUTE),
                resource_helpers::texture_entry(
                    1,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::TextureSampleType::Float { filterable: false },
                    wgpu::TextureViewDimension::D2,
                ),
                resource_helpers::storage_texture_entry(
                    2,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba32Float,
                ),
            ],
            label: Some("VCA JFA Iteration BGL"),
        });

        // Create JFA pipelines
        let jfa_init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("VCA JFA Init Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("VCA JFA Init PL"),
                    bind_group_layouts: &[&jfa_init_bgl],
                    push_constant_ranges: &[],
                }),
            ),
            module: &jfa_init_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let jfa_iteration_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VCA JFA Iteration Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("VCA JFA Iteration PL"),
                        bind_group_layouts: &[&jfa_iteration_bgl],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &jfa_iteration_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // Create JFA bind groups
        let jfa_init_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA JFA Init BG"),
            layout: &jfa_init_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &voronoi_params),
                resource_helpers::texture_view_entry(2, jfa_textures.current_view()),
            ],
        });

        let jfa_iteration_bg_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA JFA Iteration BG A->B"),
            layout: &jfa_iteration_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &voronoi_params),
                resource_helpers::texture_view_entry(1, jfa_textures.current_view()),
                resource_helpers::texture_view_entry(2, jfa_textures.inactive_view()),
            ],
        });
        let jfa_iteration_bg_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA JFA Iteration BG B->A"),
            layout: &jfa_iteration_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &voronoi_params),
                resource_helpers::texture_view_entry(1, jfa_textures.inactive_view()),
                resource_helpers::texture_view_entry(2, jfa_textures.current_view()),
            ],
        });

        // Adjacency buffers
        const MAX_NEIGHBORS: u32 = 16;
        let adjacency_neighbors = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Adjacency Neighbors"),
            size: (std::mem::size_of::<u32>() as u64)
                * (num_points as u64)
                * (MAX_NEIGHBORS as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let adjacency_degrees = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Adjacency Degrees"),
            size: (std::mem::size_of::<u32>() as u64) * (num_points as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Adjacency shaders
        let adjacency_build_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Adjacency Build Shader"),
            source: wgpu::ShaderSource::Wgsl(ADJACENCY_BUILD_SHADER.into()),
        });
        let adjacency_count_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("VCA Adjacency Count Shader"),
            source: wgpu::ShaderSource::Wgsl(ADJACENCY_COUNT_SHADER.into()),
        });

        // Adjacency bind group layouts
        let adjacency_build_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true), // vertices read
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE), // uniforms
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, false), // neighbors write
                    resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, false), // degrees write
                    resource_helpers::texture_entry(
                        4,
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::TextureSampleType::Float { filterable: false },
                        wgpu::TextureViewDimension::D2,
                    ),
                ],
                label: Some("VCA Adjacency Build BGL"),
            });

        let adjacency_count_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false), // vertices rw
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE), // uniforms
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, true), // neighbors read
                    resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true), // degrees read
                ],
                label: Some("VCA Adjacency Count BGL"),
            });

        // Adjacency bind groups
        let adjacency_build_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Adjacency Build BG"),
            layout: &adjacency_build_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &uniform_buffer),
                resource_helpers::buffer_entry(2, &adjacency_neighbors),
                resource_helpers::buffer_entry(3, &adjacency_degrees),
                resource_helpers::texture_view_entry(4, jfa_textures.current_view()),
            ],
        });

        let adjacency_count_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Adjacency Count BG"),
            layout: &adjacency_count_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &uniform_buffer),
                resource_helpers::buffer_entry(2, &adjacency_neighbors),
                resource_helpers::buffer_entry(3, &adjacency_degrees),
            ],
        });

        // Adjacency pipelines
        let adjacency_build_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VCA Adjacency Build Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("VCA Adjacency Build PL"),
                        bind_group_layouts: &[&adjacency_build_bgl],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &adjacency_build_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let adjacency_count_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("VCA Adjacency Count Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("VCA Adjacency Count PL"),
                        bind_group_layouts: &[&adjacency_count_bgl],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &adjacency_count_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            state: super::state::State {
                num_points,
                time_accum: 0.0,
                time_scale: 1.0,
                last_ca_update_time: 0.0,
                drift: 1.0,
                resolution: [width, height],
                gui_visible: true,
                brownian_speed: 10.0,
                cursor_size: 0.1,
                cursor_strength: 1.0,
                current_color_scheme: default_lut_name.clone(),
                color_scheme_reversed: false,
                color_mode: 1,
                borders_enabled: true,
                border_width: 1.0,
                rulestring: "B3/S23".to_string(),
                camera_position: [0.0, 0.0],
                camera_zoom: 1.0,
                simulation_time: 0.0,
                is_running: true,
            },
            voronoi_render_jfa_pipeline,
            compute_update_pipeline,
            brownian_pipeline,
            uniform_buffer,
            brownian_params_buffer,
            compute_neighbor_bg,
            compute_update_bg,
            brownian_bg,
            vertex_buffer,
            grid_indices,
            grid_counts,
            grid_params,
            grid_clear_pipeline,
            grid_populate_pipeline,
            grid_clear_bg,
            grid_populate_bg,
            voronoi_params,

            num_points,
            time_accum: 0.0,
            time_scale: 1.0,
            last_ca_update_time: 0.0,
            drift: 1.0,
            resolution: [width, height],
            gui_visible: true,
            points,
            // LUT + coloring defaults
            lut_buffer,
            current_color_scheme: default_lut_name,
            color_scheme_reversed: false,
            color_mode: 1,
            borders_enabled: true,
            border_width: 1.0,
            app_settings: app_settings.clone(),
            rulestring: "B3/S23".to_string(), // Default to Conway's Game of Life
            camera,
            camera_bind_group,
            display_texture,
            display_view,
            display_sampler,
            texture_render_params_buffer,
            render_infinite_pipeline,
            post_processing_state,
            post_processing_resources,
            // JFA resources
            jfa_textures,

            jfa_init_pipeline,
            jfa_iteration_pipeline,
            jfa_init_bg,
            jfa_iteration_bg_a,
            jfa_iteration_bg_b,
            // brownian motion defaults
            brownian_speed: 10.0, // px/sec
            cursor_size: 0.15,    // Larger default cursor to actually reach cells
            cursor_strength: 1.0,
            has_valid_jfa: false,
            skip_next_state_update: false,

            adjacency_neighbors,
            adjacency_degrees,
            adjacency_build_pipeline,
            adjacency_count_pipeline,
            adjacency_build_bg,
            adjacency_count_bg,
        })
    }

    fn rebuild_points(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_count: u32,
    ) -> SimulationResult<()> {
        // Recreate points array
        let mut rng = rand::rng();
        let mut points: Vec<Vertex> = Vec::with_capacity(new_count as usize);
        for _ in 0..new_count {
            points.push(Vertex {
                position: [
                    rng.random_range(0.0..(self.resolution[0] as f32)),
                    rng.random_range(0.0..(self.resolution[1] as f32)),
                ],
                state: if rng.random::<f32>() > 0.3 { 1.0 } else { 0.0 }, // More dead cells for painting
                pad0: 0.0,
                age: 0.0,
                alive_neighbors: 0,
                dead_neighbors: 0,
                random_state: rng.random::<u32>(),
            });
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
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::uniform_buffer_entry(4, wgpu::ShaderStages::COMPUTE),
                ],
                label: Some("voronoi_ca_compute_neighbor_bgl_tmp"),
            });
        self.compute_neighbor_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_neighbor_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &self.uniform_buffer),
                resource_helpers::buffer_entry(2, &self.grid_indices),
                resource_helpers::buffer_entry(3, &self.grid_counts),
                resource_helpers::buffer_entry(4, &self.grid_params),
            ],
            label: Some("voronoi_ca_compute_neighbor_bg"),
        });
        // Update pass BG
        let compute_update_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
                label: Some("voronoi_ca_compute_update_bgl_tmp"),
            });
        self.compute_update_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_update_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &self.uniform_buffer),
            ],
            label: Some("voronoi_ca_compute_update_bg"),
        });

        // Recreate grid populate BG which also depends on vertex buffer
        let grid_populate_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::storage_buffer_entry(1, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(3, wgpu::ShaderStages::COMPUTE),
            ],
            label: Some("VCA Grid Populate BGL Rebind"),
        });
        self.grid_populate_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Grid Populate BG Rebind"),
            layout: &grid_populate_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &self.grid_indices),
                resource_helpers::buffer_entry(2, &self.grid_counts),
                resource_helpers::buffer_entry(3, &self.grid_params),
            ],
        });

        // Recreate JFA bind groups that reference the vertex buffer
        let jfa_init_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, true),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                resource_helpers::storage_texture_entry(
                    2,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba32Float,
                ),
            ],
            label: Some("VCA JFA Init BGL Rebind"),
        });

        self.jfa_init_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA JFA Init BG Rebind"),
            layout: &jfa_init_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &self.voronoi_params),
                resource_helpers::texture_view_entry(2, self.jfa_textures.current_view()),
            ],
        });

        // Recreate brownian bind group
        let brownian_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::COMPUTE),
            ],
            label: Some("voronoi_ca_brownian_bgl_rebuild"),
        });

        self.brownian_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Brownian BG Rebuild"),
            layout: &brownian_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &self.uniform_buffer),
                resource_helpers::buffer_entry(2, &self.brownian_params_buffer),
            ],
        });

        // Recreate adjacency buffers sized for new_count and rebind count BG
        const MAX_NEIGHBORS: u32 = 16;
        let adjacency_neighbors = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Adjacency Neighbors Rebuild"),
            size: (std::mem::size_of::<u32>() as u64) * (new_count as u64) * (MAX_NEIGHBORS as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let adjacency_degrees = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VCA Adjacency Degrees Rebuild"),
            size: (std::mem::size_of::<u32>() as u64) * (new_count as u64),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.adjacency_neighbors = adjacency_neighbors;
        self.adjacency_degrees = adjacency_degrees;

        // Recreate adjacency count BG (build BG will be rebound dynamically each frame)
        let adjacency_count_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, true),
                    resource_helpers::storage_buffer_entry(3, wgpu::ShaderStages::COMPUTE, true),
                ],
                label: Some("VCA Adjacency Count BGL Rebind"),
            });
        self.adjacency_count_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Adjacency Count BG Rebind"),
            layout: &adjacency_count_bgl,
            entries: &[
                resource_helpers::buffer_entry(0, &vertex_buffer),
                resource_helpers::buffer_entry(1, &self.uniform_buffer),
                resource_helpers::buffer_entry(2, &self.adjacency_neighbors),
                resource_helpers::buffer_entry(3, &self.adjacency_degrees),
            ],
        });

        // Update fields
        self.points = points;
        self.vertex_buffer = vertex_buffer;
        self.num_points = new_count;
        self.state.num_points = new_count;

        // Invalidate JFA since points have changed.
        // This ensures the new point configuration is properly visualized.
        self.has_valid_jfa = false;
        Ok(())
    }

    /// Parse rulestring (e.g., "B3/S23") and return rule_type
    fn parse_rulestring(rulestring: &str) -> u32 {
        let rulestring = rulestring.to_uppercase();

        // Map of rulestrings to rule types
        match rulestring.as_str() {
            "B1357/S1357" | "B1357S1357" => 0,     // Replicator
            "B2/S" | "B2S" => 1,                   // Seeds
            "B25/S4" | "B25S4" => 2,               // Small self-replicating pattern
            "B3/S012345678" | "B3S012345678" => 3, // Life without Death
            "B3/S23" | "B3S23" => 4,               // Conway's Game of Life
            "B3/S1234" | "B3S1234" => 5,           // Maze
            "B3/S12345" | "B3S12345" => 6,         // Mazectric
            "B34/S34" | "B34S34" => 7,             // 34 Life
            "B35678/S5678" | "B35678S5678" => 8,   // Diamoeba
            "B36/S125" | "B36S125" => 9,           // 2x2
            "B36/S23" | "B36S23" => 10,            // High Life
            "B368/S245" | "B368S245" => 11,        // Day & Night
            "B4678/S35678" | "B4678S35678" => 12,  // Anneal
            "B5678/S45678" | "B5678S45678" => 13,  // Vote
            "B6/S16" | "B6S16" => 14,              // Coral
            "B6/S1" | "B6S1" => 15,                // Long Life
            "B6/S12" | "B6S12" => 16,              // Stains
            "B6/S123" | "B6S123" => 17,            // Assimilation
            "B6/S15" | "B6S15" => 18,              // Pseudo Life
            "B6/S2" | "B6S2" => 19,                // Long Life
            "B7/S" | "B7S" => 20,                  // Seeds variant
            "B8/S" | "B8S" => 21,                  // Seeds variant
            "B9/S" | "B9S" => 22,                  // Seeds variant
            _ => 4,                                // Default to Conway's Game of Life
        }
    }
}

impl VoronoiCASimulation {
    /// Update the simulation state (particle movement, CA rules, JFA computation)
    pub fn update_simulation_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        delta_time: f32,
    ) -> SimulationResult<()> {
        // Update time and uniforms
        let dt = delta_time * self.time_scale.max(0.0);
        self.time_accum += dt;
        let uniforms = Uniforms {
            resolution: self.resolution,
            time: self.time_accum,
            drift: self.drift,
            rule_type: Self::parse_rulestring(&self.rulestring),
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        // Only update time/drift fields; write the full struct for simplicity
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Update brownian params for GPU compute shader
        let brownian_params = BrownianParams {
            speed: self.brownian_speed,
            delta_time: dt,
        };
        queue.write_buffer(
            &self.brownian_params_buffer,
            0,
            bytemuck::bytes_of(&brownian_params),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VoronoiCA Update Encoder"),
        });

        // Brownian motion on GPU
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Brownian"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.brownian_pipeline);
            cpass.set_bind_group(0, &self.brownian_bg, &[]);
            let wg_count = self.num_points.div_ceil(128);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }

        // Update grid params each frame with adaptive parameters based on point density
        let total_area = self.resolution[0] * self.resolution[1];
        let density = self.num_points as f32 / total_area;

        // Adaptive cell capacity based on density
        let cell_capacity = if density > 0.01 {
            256
        } else if density > 0.005 {
            192
        } else {
            128
        };

        // Adaptive cell size - smaller cells for high density to reduce neighbor search overhead
        let base_cell_size: f32 = 90.0; // Fixed cell size for spatial partitioning
        let cell_size = if density > 0.01 {
            base_cell_size.max(8.0) // Smaller cells for high density
        } else {
            base_cell_size.max(12.0) // Standard cells for low density
        };

        let gp = GridParams {
            particle_count: self.num_points,
            grid_width: ((self.resolution[0] + cell_size - 1.0) / cell_size).ceil() as u32,
            grid_height: ((self.resolution[1] + cell_size - 1.0) / cell_size).ceil() as u32,
            cell_capacity,
            cell_size,
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
            cpass.dispatch_workgroups(total_cells.div_ceil(128), 1, 1);
        }
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Grid Populate"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.grid_populate_pipeline);
            cpass.set_bind_group(0, &self.grid_populate_bg, &[]);
            cpass.dispatch_workgroups(self.num_points.div_ceil(128), 1, 1);
        }

        // Update uniform params (count and color mode)
        let params = VoronoiParams {
            count: self.num_points as f32,
            color_mode: self.color_mode as f32,
            border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
            border_width: self.border_width,
            filter_mode: match self.app_settings.texture_filtering {
                crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
            },
            resolution_x: self.resolution[0],
            resolution_y: self.resolution[1],
            jump_distance: 0.0, // Not used in this context
        };
        queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));

        // Update camera and upload to GPU
        self.camera.update(delta_time);
        self.camera.upload_to_gpu(queue);

        // JFA passes: initialize and iterate to build Voronoi diagram (needed before adjacency)
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA JFA Init"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.jfa_init_pipeline);
            cpass.set_bind_group(0, &self.jfa_init_bg, &[]);
            let wg_x = (self.resolution[0] as u32).div_ceil(8);
            let wg_y = (self.resolution[1] as u32).div_ceil(8);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // JFA iterations - perform multiple passes with decreasing jump distances
        let max_jump = (self.resolution[0].max(self.resolution[1]) as u32).next_power_of_two();
        let mut jump_distance = max_jump;

        // Use full JFA iterations for consistent visual quality
        let min_jump = 1;

        while jump_distance >= min_jump {
            // Update params with current jump distance
            let params = VoronoiParams {
                count: self.num_points as f32,
                color_mode: self.color_mode as f32,
                border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
                border_width: self.border_width,
                filter_mode: match self.app_settings.texture_filtering {
                    crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                    crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                    crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
                },
                resolution_x: self.resolution[0],
                resolution_y: self.resolution[1],
                jump_distance: jump_distance as f32,
            };
            queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));

            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA JFA Iteration"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.jfa_iteration_pipeline);

            // Use the appropriate bind group based on current texture state
            let bind_group = self
                .jfa_textures
                .get_bind_group(&self.jfa_iteration_bg_a, &self.jfa_iteration_bg_b);
            cpass.set_bind_group(0, bind_group, &[]);

            let wg_x = (self.resolution[0] as u32).div_ceil(8);
            let wg_y = (self.resolution[1] as u32).div_ceil(8);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);

            // Swap textures for next iteration
            self.jfa_textures.swap();
            jump_distance /= 2;
        }

        // Build adjacency from JFA
        let current_jfa_view = self.get_current_jfa_view();
        // Rebind adjacency build BG to current JFA view each frame
        self.adjacency_build_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Adjacency Build BG Dynamic"),
            layout: &self.adjacency_build_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::buffer_entry(0, &self.vertex_buffer),
                resource_helpers::buffer_entry(1, &self.uniform_buffer),
                resource_helpers::buffer_entry(2, &self.adjacency_neighbors),
                resource_helpers::buffer_entry(3, &self.adjacency_degrees),
                resource_helpers::texture_view_entry(4, current_jfa_view),
            ],
        });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Adjacency Build"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.adjacency_build_pipeline);
            cpass.set_bind_group(0, &self.adjacency_build_bg, &[]);
            let wg_count = self.num_points.div_ceil(128);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }

        // Count neighbors via adjacency graph
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VCA Adjacency Neighbor Count"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.adjacency_count_pipeline);
            cpass.set_bind_group(0, &self.adjacency_count_bg, &[]);
            let wg_count = self.num_points.div_ceil(128);
            cpass.dispatch_workgroups(wg_count, 1, 1);
        }

        // State update based on adjacency neighbor counts
        // Skip state update if we just painted to prevent unwanted births
        // Also respect time scale for CA update frequency
        let should_update_ca = !self.skip_next_state_update
            && (self.time_scale <= 0.0
                || self.time_accum - self.last_ca_update_time >= 1.0 / self.time_scale);

        if should_update_ca {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VoronoiCA State Update"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_update_pipeline);
            cpass.set_bind_group(0, &self.compute_update_bg, &[]);
            let wg_count = self.num_points.div_ceil(128);
            cpass.dispatch_workgroups(wg_count, 1, 1);
            self.last_ca_update_time = self.time_accum;
        } else if self.skip_next_state_update {
            // Reset the flag after skipping one frame
            self.skip_next_state_update = false;
        }

        // Mark JFA as valid after full update.
        // This allows the paused renderer to reuse the texture until changes occur.
        self.has_valid_jfa = true;

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Render the current simulation state to the display
    pub fn render_offscreen_only(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Ensure JFA is up to date before rendering
        if !self.has_valid_jfa {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("VCA JFA Rebuild Encoder (Offscreen)"),
            });
            self.rebuild_jfa_texture(device, queue, &mut encoder)?;
            queue.submit(std::iter::once(encoder.finish()));
            self.has_valid_jfa = true;
        }

        // Render Voronoi to the offscreen display texture
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VCA Offscreen Render Encoder"),
        });

        {
            let current_jfa_view = self.get_current_jfa_view();
            let voronoi_render_jfa_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VCA Voronoi JFA Render BG (Offscreen)"),
                layout: &self.voronoi_render_jfa_pipeline.get_bind_group_layout(0),
                entries: &[
                    resource_helpers::buffer_entry(0, &self.voronoi_params),
                    resource_helpers::buffer_entry(1, &self.vertex_buffer),
                    resource_helpers::buffer_entry(2, &self.lut_buffer),
                    resource_helpers::texture_view_entry(3, current_jfa_view),
                ],
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VoronoiCA Render Pass (Offscreen)"),
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
            rpass.set_pipeline(&self.voronoi_render_jfa_pipeline);
            let params = VoronoiParams {
                count: self.num_points as f32,
                color_mode: self.color_mode as f32,
                border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
                border_width: self.border_width,
                filter_mode: match self.app_settings.texture_filtering {
                    crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                    crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                    crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
                },
                resolution_x: self.resolution[0],
                resolution_y: self.resolution[1],
                jump_distance: 0.0,
            };
            queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));
            rpass.set_bind_group(0, &voronoi_render_jfa_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));

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

            let mut blur_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("VCA Blur Encoder (Offscreen)"),
            });

            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VCA Blur Bind Group (Offscreen)"),
                layout: &self
                    .post_processing_resources
                    .blur_pipeline
                    .get_bind_group_layout(0),
                entries: &[
                    resource_helpers::texture_view_entry(0, &self.display_view),
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

            {
                let mut rpass = blur_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("VCA PostProcess Pass (Offscreen)"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.post_processing_resources.intermediate_view,
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
                rpass.set_pipeline(&self.post_processing_resources.blur_pipeline);
                rpass.set_bind_group(0, &blur_bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }

            blur_encoder.copy_texture_to_texture(
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

            queue.submit(std::iter::once(blur_encoder.finish()));
        }

        Ok(())
    }

    /// Render the current simulation state to the display
    pub fn render_simulation_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Only rebuild JFA if we don't have a valid one or if points have changed
        // This avoids expensive JFA computation when just painting
        if !self.has_valid_jfa {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("VCA JFA Rebuild Encoder"),
            });
            self.rebuild_jfa_texture(device, queue, &mut encoder)?;
            queue.submit(std::iter::once(encoder.finish()));
            self.has_valid_jfa = true;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VCA Render Encoder"),
        });

        // Fullscreen Voronoi render to offscreen display texture (uses JFA texture)
        {
            // Create render bind group with current JFA texture using the pipeline's layout
            let current_jfa_view = self.get_current_jfa_view();
            let voronoi_render_jfa_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VCA Voronoi JFA Render BG Dynamic"),
                layout: &self.voronoi_render_jfa_pipeline.get_bind_group_layout(0),
                entries: &[
                    resource_helpers::buffer_entry(0, &self.voronoi_params),
                    resource_helpers::buffer_entry(1, &self.vertex_buffer),
                    resource_helpers::buffer_entry(2, &self.lut_buffer),
                    resource_helpers::texture_view_entry(3, current_jfa_view),
                ],
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VoronoiCA Render Pass"),
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
            rpass.set_pipeline(&self.voronoi_render_jfa_pipeline);
            // Update params before draw
            let params = VoronoiParams {
                count: self.num_points as f32,
                color_mode: self.color_mode as f32,
                border_enabled: if self.borders_enabled { 1.0 } else { 0.0 },
                border_width: self.border_width,
                filter_mode: match self.app_settings.texture_filtering {
                    crate::commands::app_settings::TextureFiltering::Nearest => 0.0,
                    crate::commands::app_settings::TextureFiltering::Linear => 1.0,
                    crate::commands::app_settings::TextureFiltering::Lanczos => 2.0,
                },
                resolution_x: self.resolution[0],
                resolution_y: self.resolution[1],
                jump_distance: 0.0, // Not used in render
            };
            queue.write_buffer(&self.voronoi_params, 0, bytemuck::bytes_of(&params));

            rpass.set_bind_group(0, &voronoi_render_jfa_bg, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Submit the main render pass first
        queue.submit(std::iter::once(encoder.finish()));

        // Optional blur: display_view -> intermediate_view, then copy back to display
        // This must be done in a separate command encoder to avoid texture usage conflicts
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

            let mut blur_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("VCA Blur Encoder"),
            });

            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VCA Blur Bind Group"),
                layout: &self
                    .post_processing_resources
                    .blur_pipeline
                    .get_bind_group_layout(0),
                entries: &[
                    resource_helpers::texture_view_entry(0, &self.display_view),
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

            {
                let mut rpass = blur_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("VCA PostProcess Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.post_processing_resources.intermediate_view,
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
                rpass.set_pipeline(&self.post_processing_resources.blur_pipeline);
                rpass.set_bind_group(0, &blur_bind_group, &[]);
                rpass.draw(0..6, 0..1);
            }

            blur_encoder.copy_texture_to_texture(
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

            queue.submit(std::iter::once(blur_encoder.finish()));
        }

        // Infinite tiling pass to the surface
        let tile_count = self.calculate_tile_count();
        let total_instances = tile_count * tile_count;

        let mut encoder2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VCA Infinite Surface Encoder"),
        });
        {
            // Create the infinite tiling bind group dynamically to avoid texture usage conflicts
            // This ensures the display_view is no longer being used as a color target
            let render_infinite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("VCA Render Infinite BG Dynamic"),
                layout: &self.render_infinite_pipeline.get_bind_group_layout(0),
                entries: &[
                    resource_helpers::texture_view_entry(0, &self.display_view),
                    resource_helpers::sampler_bind_entry(1, &self.display_sampler),
                    resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
                ],
            });

            let mut rpass = encoder2.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VCA Infinite Surface Render Pass"),
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
            rpass.set_pipeline(&self.render_infinite_pipeline);
            rpass.set_bind_group(0, &render_infinite_bg, &[]);
            rpass.set_bind_group(1, &self.camera_bind_group, &[]);
            rpass.draw(0..6, 0..total_instances);
        }
        queue.submit(std::iter::once(encoder2.finish()));
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
        // Update simulation state (particle movement, CA rules, JFA computation)
        self.update_simulation_state(device, queue, delta_time)?;

        // Render the current state
        self.render_simulation_state(device, queue, surface_view)?;

        Ok(())
    }

    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Only render the current state, no simulation updates
        self.render_simulation_state(device, queue, surface_view)?;

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

        // Invalidate JFA since resolution has changed.
        // This ensures the new resolution is properly handled.
        self.has_valid_jfa = false;

        // Recreate JFA textures for new resolution
        self.jfa_textures = PingPongTextures::new(
            device,
            new_config.width,
            new_config.height,
            wgpu::TextureFormat::Rgba32Float,
            "VCA JFA Texture",
        );

        // Recreate JFA bind groups with new textures
        self.jfa_init_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA JFA Init BG (Resize)"),
            layout: &self.jfa_init_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::buffer_entry(0, &self.vertex_buffer),
                resource_helpers::buffer_entry(1, &self.voronoi_params),
                resource_helpers::texture_view_entry(2, self.jfa_textures.current_view()),
            ],
        });

        // Recreate display texture for new resolution
        self.display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("VCA Display Texture (Resize)"),
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

        // Recreate post-processing resources for new resolution
        self.post_processing_resources = PostProcessingResources::new(device, new_config)?;

        // Note: render_infinite_bind_group is now created dynamically in render_simulation_state
        // to avoid texture usage conflicts

        // Recreate camera bind group
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("VCA Camera BGL (Resize)"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                )],
            });

        self.camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("VCA Camera BG (Resize)"),
            layout: &camera_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, self.camera.buffer())],
        });

        // Resize post-processing resources
        self.post_processing_resources.resize(device, new_config)?;

        Ok(())
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        tracing::info!(
            "VCA handle_mouse_interaction called: world=({:.3}, {:.3}), button={}",
            world_x,
            world_y,
            mouse_button
        );
        // Convert world coordinates to simulation coordinates
        // Incoming world coords are already in [-1,1] space (camera-adjusted).
        // Just map to [0, width] x [0, height] without reapplying camera transforms.
        // Use cursor size as world space radius, convert to simulation pixels
        let world_radius = self.cursor_size;
        let sim_radius = world_radius * (self.resolution[0].min(self.resolution[1])) * 0.5;
        let radius_sq = sim_radius * sim_radius;

        // Wrap world coords to the current 2x2 tile in [-1,1] so clicks map to the base texture
        let wrapped_world_x = (world_x + 1.0).rem_euclid(2.0) - 1.0;
        let wrapped_world_y = (world_y + 1.0).rem_euclid(2.0) - 1.0;

        // Convert from wrapped [-1,1] world to [0, resolution] pixels
        // Flip Y axis: world Y increases upward, simulation Y increases downward
        let x = (wrapped_world_x + 1.0) * 0.5 * self.resolution[0];
        let y = (1.0 - wrapped_world_y) * 0.5 * self.resolution[1];

        tracing::info!(
            "VCA coordinate conversion: wrapped_world=({:.3}, {:.3}) -> sim=({:.1}, {:.1})",
            wrapped_world_x,
            wrapped_world_y,
            x,
            y
        );

        // Buttons: 0 = left => set alive, 2 = right => set dead
        let set_alive = mouse_button == 0;
        let set_dead = mouse_button == 2;
        if !(set_alive || set_dead) {
            return Ok(());
        }

        tracing::info!(
            "VCA mouse interaction: world=({:.3}, {:.3}), sim=({:.1}, {:.1}), button={}, world_radius={:.3}, sim_radius={:.1}, cursor_size={:.3}",
            world_x,
            world_y,
            x,
            y,
            mouse_button,
            world_radius,
            sim_radius,
            self.cursor_size
        );

        // Affect all sites within the cursor radius (brush painting)
        // Use the same toroidal metric as the JFA init/shaders so the edited sites match the clicked cells
        let w = self.resolution[0];
        let h = self.resolution[1];
        let mut affected_count = 0;

        tracing::info!(
            "VCA searching {} points within radius {:.1} of ({:.1}, {:.1})",
            self.points.len(),
            sim_radius,
            x,
            y
        );

        for (i, v) in self.points.iter_mut().enumerate() {
            let mut dx = v.position[0] - x;
            let mut dy = v.position[1] - y;
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
                let old_state = v.state;
                if set_alive {
                    v.state = 1.0;
                } else if set_dead {
                    v.state = 0.0;
                }

                tracing::info!(
                    "VCA painting: point {} at ({:.1}, {:.1}), state {} -> {}, distance {:.1} <= {:.1}",
                    i,
                    v.position[0],
                    v.position[1],
                    old_state,
                    v.state,
                    d2.sqrt(),
                    sim_radius
                );

                affected_count += 1;
            }
        }

        if affected_count > 0 {
            tracing::info!("VCA painting: affected {} points", affected_count);

            // Push updated states to GPU FIRST so the compute shaders see the changes
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.points));

            // Invalidate JFA so it gets rebuilt on next render.
            // This ensures the visual changes from painting are immediately visible.
            self.has_valid_jfa = false;

            // Force an immediate render update when paused to show painting changes
            // This ensures visual feedback is immediate even when the simulation is paused
            // Render only to the offscreen display texture to avoid usage conflicts
            if let Err(e) = self.render_offscreen_only(device, queue) {
                tracing::warn!("Failed to render immediate painting update: {}", e);
            }

            // Skip neighbor count updates during painting to avoid confusing visual artifacts
            // The neighbor counts will be updated naturally when the simulation resumes or steps
            // This prevents the appearance of "multiple cells being painted" when only one was clicked

            // Also skip CA state updates during painting to prevent unwanted births
            // The CA rules will be applied naturally on the next simulation frame
            // This prevents dead cells from becoming alive due to the newly painted cell
            self.skip_next_state_update = true;
        } else {
            tracing::info!(
                "VCA painting: no points found within radius {:.1}",
                sim_radius
            );
        }
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
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Parse rulestring and update rule_type
        if let Some(rulestring) = settings.get("rulestring").and_then(|v| v.as_str()) {
            self.rulestring = rulestring.to_string();
            let rule_type = Self::parse_rulestring(&self.rulestring);

            // Update uniforms with new rule_type
            let uniforms = Uniforms {
                resolution: self.resolution,
                time: self.time_accum,
                drift: self.drift,
                rule_type,
                _pad0: 0,
                _pad1: 0,
                _pad2: 0,
            };
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        }

        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Reset time to start brownian motion from beginning
        self.time_accum = 0.0;

        // Set all cells to dead (state = 0.0) and reset random state
        let mut rng = rand::rng();
        for point in &mut self.points {
            point.state = 0.0;
            point.age = 0.0;
            point.alive_neighbors = 0;
            point.dead_neighbors = 0;
            point.random_state = rng.random::<u32>();
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.points));

        // Invalidate JFA so it gets rebuilt
        self.has_valid_jfa = false;

        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "rulestring" => {
                if let Some(s) = value.as_str() {
                    self.rulestring = s.to_string();
                    let rule_type = Self::parse_rulestring(&self.rulestring);
                    let uniforms = Uniforms {
                        resolution: self.resolution,
                        time: self.time_accum,
                        drift: self.drift,
                        rule_type,
                        _pad0: 0,
                        _pad1: 0,
                        _pad2: 0,
                    };
                    queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
                }
            }
            "drift" => {
                if let Some(v) = value.as_f64() {
                    self.drift = v as f32;
                    let uniforms = Uniforms {
                        resolution: self.resolution,
                        time: self.time_accum,
                        drift: self.drift,
                        rule_type: Self::parse_rulestring(&self.rulestring),
                        _pad0: 0,
                        _pad1: 0,
                        _pad2: 0,
                    };
                    queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
                }
            }
            "brownianSpeed" => {
                if let Some(v) = value.as_f64() {
                    self.brownian_speed = v as f32;
                }
            }
            "cursor_size" => {
                if let Some(v) = value.as_f64() {
                    self.cursor_size = v as f32;
                    self.state.cursor_size = v as f32;
                }
            }
            "cursor_strength" => {
                if let Some(v) = value.as_f64() {
                    self.cursor_strength = v as f32;
                    self.state.cursor_strength = v as f32;
                }
            }
            "coloringMode" => {
                if let Some(s) = value.as_str() {
                    self.color_mode = match s {
                        "Random" => 0,
                        "Density" => 1,
                        "Age" => 2,
                        "Binary" => 3,
                        _ => 0,
                    };
                }
            }
            "bordersEnabled" => {
                if let Some(b) = value.as_bool() {
                    self.borders_enabled = b;
                }
            }
            "borderWidth" => {
                if let Some(v) = value.as_f64() {
                    self.border_width = v as f32;
                }
            }
            "timeScale" => {
                if let Some(v) = value.as_f64() {
                    self.time_scale = v as f32;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        match state_name {
            "numPoints" => {
                if let Some(v) = value.as_u64().and_then(|v| v.try_into().ok()) {
                    self.rebuild_points(device, queue, v)?;
                }
            }
            _ => {
                tracing::error!("Unknown state parameter for VoronoiCA: {}", state_name);
            }
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::json!({
            "rulestring": self.rulestring,
            "drift": self.drift,
            "brownian_speed": self.brownian_speed,
            "time_scale": self.time_scale,
            "cursor_size": self.cursor_size,
            "cursor_strength": self.cursor_strength,
            "current_color_scheme": self.current_color_scheme,
            "color_scheme_reversed": self.color_scheme_reversed,
            "coloring_mode": match self.color_mode { 0 => "Random", 1 => "Density", 2 => "Age", 3 => "Binary", _ => "Random" },
            "borders_enabled": self.borders_enabled,
            "border_width": self.border_width
        })
    }

    fn get_state(&self) -> Value {
        serde_json::to_value(&self.state).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn is_gui_visible(&self) -> bool {
        self.gui_visible
    }

    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Direct-write the color scheme data to the VCA buffer for immediate preview
        let mut data_u32 = color_scheme.to_u32_buffer();
        if self.color_scheme_reversed {
            data_u32[0..256].reverse();
            data_u32[256..512].reverse();
            data_u32[512..768].reverse();
        }
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&data_u32));
        Ok(())
    }
}
