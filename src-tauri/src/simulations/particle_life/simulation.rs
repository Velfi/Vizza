use crate::error::{SimulationError, SimulationResult};
use crate::simulations::shared::gpu_utils::resource_helpers;
use crate::simulations::shared::{
    BindGroupBuilder, ColorMode, ColorSchemeManager, ComputePipelineBuilder, PositionGenerator,
    camera::Camera,
    post_processing::{PostProcessingResources, PostProcessingState},
};
use bytemuck::{Pod, Zeroable};
use rand::{Rng, SeedableRng};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::{MatrixGenerator, Settings, TypeGenerator};
use super::shaders;
use super::state::{Particle, State};
use crate::simulations::traits::Simulation;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ForceUpdateParams {
    pub species_a: u32,
    pub species_b: u32,
    pub new_force: f32,
    pub species_count: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct InitParams {
    pub start_index: u32,
    pub spawn_count: u32,
    pub species_count: u32,
    pub width: f32,
    pub height: f32,
    pub random_seed: u32,
    pub position_generator: u32, // 0=Random, 1=Center, 2=UniformCircle, etc.
    pub type_generator: u32,     // 0=Random, 1=Randomize10Percent, etc.
    pub _pad1: u32,
    pub _pad2: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ForceRandomizeParams {
    pub species_count: u32,
    pub random_seed: u32,
    pub min_force: f32,
    pub max_force: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct FadeUniforms {
    pub fade_amount: f32, // Amount to subtract from alpha each frame (0.0 = no fade, higher = faster fade)
    pub _pad1: f32,       // Padding for 16-byte alignment
    pub _pad2: f32,       // Padding for 16-byte alignment
    pub _pad3: f32,       // Padding for 16-byte alignment
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct BackgroundParams {
    pub background_color: [f32; 4], // RGBA color values
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ViewportParams {
    pub world_bounds: [f32; 4], // [left, bottom, right, top] in world coordinates
    pub texture_size: [f32; 2], // [width, height] of offscreen texture
    pub _pad1: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimParams {
    pub particle_count: u32,
    pub species_count: u32,
    pub max_force: f32,
    pub max_distance: f32,
    pub friction: f32,
    pub wrap_edges: u32,
    pub width: f32,
    pub height: f32,
    pub random_seed: u32,
    pub dt: f32,       // Time step for simulation
    pub beta: f32,     // Transition point between repulsion and attraction zones
    pub cursor_x: f32, // Cursor position in world coordinates
    pub cursor_y: f32,
    pub cursor_size: f32,     // Cursor interaction radius
    pub cursor_strength: f32, // Cursor force strength
    pub cursor_active: u32, // Whether cursor interaction is active (0 = inactive, 1 = attract, 2 = repel)
    pub brownian_motion: f32, // Brownian motion strength (0.0-1.0)
    pub particle_size: f32, // Particle size in world space units
    pub aspect_ratio: f32,  // Screen aspect ratio for cursor distance calculation
    pub _pad1: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct TileParams {
    pub tile_x: i32,
    pub tile_y: i32,
    pub camera_zoom: f32,
    pub _pad0: f32,             // Padding for 16-byte alignment
    pub world_bounds: [f32; 4], // [left, bottom, right, top] for this tile
    pub texture_size: [f32; 2], // [width, height] of tile texture
    pub _pad1: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CameraAwareParams {
    pub camera_zoom: f32,
    pub _pad0: f32, // Padding for 16-byte alignment
    pub camera_position: [f32; 2],
    pub viewport_size: [f32; 2],
    pub tile_size: f32, // Size of each tile in world units
    pub max_tiles: u32, // Maximum number of tiles to render
    pub _pad1: f32,
    pub _pad2: f32,
}

impl SimParams {
    pub fn new(
        width: u32,
        height: u32,
        particle_count: u32,
        settings: &Settings,
        state: &State,
    ) -> Self {
        let aspect_ratio = width as f32 / height as f32;

        Self {
            particle_count,
            species_count: settings.species_count,
            max_force: settings.max_force,
            max_distance: settings.max_distance,
            friction: settings.friction,
            wrap_edges: if settings.wrap_edges { 1 } else { 0 },
            width: width as f32,
            height: height as f32,
            random_seed: state.random_seed,
            dt: state.dt,
            beta: settings.force_beta,
            cursor_x: 0.0, // Initialize cursor position to center
            cursor_y: 0.0,
            cursor_size: state.cursor_size,
            cursor_strength: state.cursor_strength,
            cursor_active: 0, // Start with cursor interaction inactive
            brownian_motion: settings.brownian_motion,
            particle_size: state.particle_size,
            aspect_ratio,
            _pad1: 0,
        }
    }
}

/// Particle Life simulation model
#[derive(Debug)]
pub struct ParticleLifeModel {
    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub force_matrix_buffer: wgpu::Buffer,
    pub lut_buffer: Arc<wgpu::Buffer>,
    pub lut_size_buffer: wgpu::Buffer,
    pub color_mode_buffer: wgpu::Buffer,
    pub species_colors_buffer: wgpu::Buffer,

    // Compute pipeline
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,

    // Initialization pipeline
    pub init_pipeline: wgpu::ComputePipeline,
    pub init_bind_group: wgpu::BindGroup,
    pub init_bind_group_layout: wgpu::BindGroupLayout,
    pub init_params_buffer: wgpu::Buffer,

    // Force matrix update pipelines
    pub force_update_pipeline: wgpu::ComputePipeline,
    pub force_update_params_buffer: wgpu::Buffer,
    pub force_update_bind_group: wgpu::BindGroup,

    // Force matrix randomization pipeline
    pub force_randomize_pipeline: wgpu::ComputePipeline,
    pub force_randomize_params_buffer: wgpu::Buffer,
    pub force_randomize_bind_group: wgpu::BindGroup,

    // Render pipeline
    pub render_pipeline: wgpu::RenderPipeline,
    pub render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_particles_bind_group_layout: wgpu::BindGroupLayout,
    pub render_bind_group: wgpu::BindGroup,
    pub lut_bind_group: wgpu::BindGroup,
    pub camera_bind_group: wgpu::BindGroup,

    // Tile render pipeline for improved camera-aware rendering
    pub tile_render_pipeline: wgpu::RenderPipeline,
    pub tile_render_bind_group_layout: wgpu::BindGroupLayout,
    pub tile_render_bind_group: wgpu::BindGroup,
    pub tile_params_buffer: wgpu::Buffer,

    // Offscreen render pipeline for display texture (MSAA)
    pub offscreen_render_pipeline: wgpu::RenderPipeline,
    // Non-MSAA render pipeline for direct display texture rendering
    pub display_render_pipeline: wgpu::RenderPipeline,
    // Trail render pipeline for trail texture (uses surface format)
    pub trail_render_pipeline: wgpu::RenderPipeline,

    // Fade pipeline for traces
    pub fade_pipeline: wgpu::RenderPipeline,
    pub fade_bind_group_layout: wgpu::BindGroupLayout,
    pub fade_bind_group: wgpu::BindGroup,
    pub fade_uniforms_buffer: wgpu::Buffer,

    // Trail textures for persistent trails (ping-pong buffers)
    pub trail_texture_a: wgpu::Texture,
    pub trail_texture_view_a: wgpu::TextureView,
    pub trail_texture_b: wgpu::Texture,
    pub trail_texture_view_b: wgpu::TextureView,
    pub current_trail_is_a: bool, // true = A is current write target, B is read source

    // Blit pipeline to copy trail texture to surface
    pub blit_pipeline: wgpu::RenderPipeline,
    // Display blit pipeline to copy trail texture to display texture (Rgba8Unorm format)
    pub display_blit_pipeline: wgpu::RenderPipeline,
    pub blit_bind_group_layout: wgpu::BindGroupLayout,
    pub blit_bind_group: wgpu::BindGroup,

    // Background render pipeline for offscreen rendering
    pub background_render_pipeline: wgpu::RenderPipeline,
    pub background_bind_group_layout: wgpu::BindGroupLayout,
    pub background_bind_group: wgpu::BindGroup,
    pub background_params_buffer: wgpu::Buffer,

    // Viewport parameters for camera-aware rendering
    pub viewport_params_buffer: wgpu::Buffer,

    // Display texture for offscreen rendering
    pub display_texture: wgpu::Texture,
    pub display_view: wgpu::TextureView,
    pub display_bind_group: wgpu::BindGroup,

    // MSAA texture for anti-aliasing particle rendering
    pub msaa_texture: wgpu::Texture,
    pub msaa_view: wgpu::TextureView,

    // Post effect texture for post-processing
    pub post_effect_texture: wgpu::Texture,
    pub post_effect_view: wgpu::TextureView,
    pub post_effect_bind_group: wgpu::BindGroup,

    // Infinite render pipeline for final display
    pub render_infinite_pipeline: wgpu::RenderPipeline,
    pub render_infinite_bind_group: wgpu::BindGroup,
    pub render_infinite_display_bind_group: wgpu::BindGroup,

    // Post effect pipeline
    pub post_effect_pipeline: wgpu::RenderPipeline,
    pub post_effect_params_buffer: wgpu::Buffer,
    pub post_effect_bind_group_layout: wgpu::BindGroupLayout,
    pub infinite_render_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_render_params_buffer: wgpu::Buffer,

    // Samplers
    pub blit_sampler: wgpu::Sampler,
    pub post_effect_sampler: wgpu::Sampler,
    pub display_sampler: wgpu::Sampler,

    // Simulation state and settings
    pub settings: Settings,
    pub state: State,
    pub gui_visible: bool,

    // LUT management
    pub color_scheme_manager: Arc<ColorSchemeManager>, // Store reference to color scheme manager

    // Dimensions
    pub width: u32,
    pub height: u32,

    // Camera for viewport control
    pub camera: Camera,

    // Frame timing for smooth camera movement
    last_frame_time: std::time::Instant,

    // Cursor interaction state
    pub cursor_active_mode: u32, // 0=inactive, 1=attract, 2=repel
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,

    // Adaptive resolution tracking
    pub current_resolution_scale: f32,
    pub last_zoom_level: f32,
    pub base_surface_width: u32,
    pub base_surface_height: u32,

    // Camera-aware parameters
    pub camera_aware_params_buffer: wgpu::Buffer,

    // Post-processing state and resources
    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
}

impl ParticleLifeModel {
    /// Calculate adaptive resolution scale based on zoom level for fixed tile rendering
    fn calculate_resolution_scale(zoom: f32, base_width: u32, base_height: u32) -> f32 {
        // Handle the full zoom range (0.005 to 50.0 from camera limits)
        let zoom = zoom.clamp(0.005, 50.0);

        // For fixed tile rendering, scale resolution based on zoom to maintain detail
        // Higher zoom = need more detail per tile, lower zoom = can use less detail per tile
        let scale = if zoom >= 1.0 {
            // When zoomed in, scale up for crisp detail
            (zoom * 0.8).min(4.0) // Cap at 4x for reasonable memory usage
        } else {
            // When zoomed out, maintain base resolution since tiling handles the coverage
            // Don't scale down too much or particles become invisible
            (zoom * 0.5 + 0.5).max(0.5) // Minimum 0.5x scale
        };

        // Apply 2x resolution boost for better visual quality
        let scale = scale * 2.0;

        // Calculate maximum allowed scale based on WebGPU limits (8192x8192)
        let max_width_scale = 8192.0 / base_width as f32;
        let max_height_scale = 8192.0 / base_height as f32;
        let max_allowed_scale = max_width_scale.min(max_height_scale);

        let min_scale = 1.0; // Minimum scale for visibility (doubled from 0.5)
        let max_scale = max_allowed_scale.min(8.0); // Reasonable maximum for fixed tiles (doubled from 4.0)

        scale.clamp(min_scale, max_scale)
    }

    /// Check if resolution needs to be updated based on zoom change
    fn should_update_resolution(&self) -> bool {
        let new_scale = Self::calculate_resolution_scale(
            self.camera.zoom,
            self.base_surface_width,
            self.base_surface_height,
        );
        let scale_diff = (new_scale - self.current_resolution_scale).abs();

        // Use adaptive threshold based on zoom level
        // At high zoom levels, be more sensitive to changes for better detail
        // At low zoom levels, be less sensitive for performance
        let threshold = if self.camera.zoom >= 1.0 {
            0.05 // More sensitive at high zoom (5% change)
        } else {
            0.15 // Less sensitive at low zoom (15% change)
        };

        scale_diff > threshold
    }

    /// Update display textures with new resolution
    fn update_resolution(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
        let new_scale = Self::calculate_resolution_scale(
            self.camera.zoom,
            self.base_surface_width,
            self.base_surface_height,
        );
        let new_width = (self.base_surface_width as f32 * new_scale) as u32;
        let new_height = (self.base_surface_height as f32 * new_scale) as u32;

        // Only recreate if dimensions actually changed
        if new_width != self.display_texture.width() || new_height != self.display_texture.height()
        {
            // Recreate display texture with new dimensions
            self.display_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Display Texture"),
                size: wgpu::Extent3d {
                    width: new_width,
                    height: new_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            self.display_view = self
                .display_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate post effect texture with new dimensions
            self.post_effect_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Post Effect Texture"),
                size: wgpu::Extent3d {
                    width: new_width,
                    height: new_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            self.post_effect_view = self
                .post_effect_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate MSAA texture with new dimensions
            self.msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: new_width,
                    height: new_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4, // 4x MSAA
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa_view = self
                .msaa_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate trail textures with new dimensions
            self.trail_texture_a = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Trail Texture A"),
                size: wgpu::Extent3d {
                    width: new_width,
                    height: new_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            self.trail_texture_view_a =
                self.trail_texture_a
                    .create_view(&wgpu::TextureViewDescriptor {
                        label: Some("Trail Texture View A"),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        format: Some(wgpu::TextureFormat::Rgba8Unorm),
                        ..Default::default()
                    });

            self.trail_texture_b = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Trail Texture B"),
                size: wgpu::Extent3d {
                    width: new_width,
                    height: new_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            self.trail_texture_view_b =
                self.trail_texture_b
                    .create_view(&wgpu::TextureViewDescriptor {
                        label: Some("Trail Texture View B"),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        format: Some(wgpu::TextureFormat::Rgba8Unorm),
                        ..Default::default()
                    });

            // Update dimensions
            self.width = new_width;
            self.height = new_height;

            // Recreate bind groups that reference the display texture
            self.recreate_display_bind_groups(device)?;

            self.current_resolution_scale = new_scale;
        }

        self.last_zoom_level = self.camera.zoom;
        Ok(())
    }

    /// Recreate bind groups that reference display textures
    fn recreate_display_bind_groups(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
        // Recreate display bind group
        self.display_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Display Bind Group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &self.display_view),
                resource_helpers::sampler_bind_entry(1, &self.blit_sampler),
            ],
        });

        // Recreate post effect bind group
        self.post_effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Effect Bind Group"),
            layout: &self.post_effect_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &self.post_effect_view),
                resource_helpers::sampler_bind_entry(1, &self.post_effect_sampler),
                resource_helpers::buffer_entry(2, &self.post_effect_params_buffer),
            ],
        });

        // Recreate infinite render bind groups
        self.render_infinite_display_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Infinite Display Bind Group"),
                layout: &self.infinite_render_bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &self.display_view),
                    resource_helpers::sampler_bind_entry(1, &self.blit_sampler),
                    resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
                ],
            });

        self.render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Infinite Bind Group"),
            layout: &self.infinite_render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &self.post_effect_view),
                resource_helpers::sampler_bind_entry(1, &self.blit_sampler),
                resource_helpers::buffer_entry(2, &self.texture_render_params_buffer),
            ],
        });

        Ok(())
    }

    /// Check if post-effect parameters are at default values (no processing needed)
    fn needs_post_effects(&self) -> bool {
        // Default values: brightness=1.0, contrast=1.0, saturation=1.0, gamma=1.0
        // If all parameters are at defaults, we can skip the expensive post-effect pass
        false // Currently hardcoded to defaults, so never needed
    }

    /// Flatten 2D force matrix to 1D array for GPU
    pub fn flatten_force_matrix(force_matrix: &[Vec<f32>]) -> Vec<f32> {
        let mut flattened = Vec::new();

        for row in force_matrix {
            for &force in row {
                flattened.push(force);
            }
        }

        flattened
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        _adapter_info: &wgpu::AdapterInfo,
        particle_count: usize,
        settings: Settings,
        app_settings: &crate::commands::app_settings::AppSettings,
        color_scheme_manager: &ColorSchemeManager,
        color_mode: ColorMode,
    ) -> SimulationResult<Self> {
        let width = surface_config.width;
        let height = surface_config.height;

        // Use a proper default LUT name instead of hardcoding
        let default_color_scheme = "MATPLOTLIB_ocean";

        // Get LUT and calculate colors first
        let lut = color_scheme_manager
            .get(default_color_scheme)
            .map_err(|e| {
                SimulationError::InitializationFailed(format!(
                    "Failed to load default LUT '{}': {}",
                    default_color_scheme, e
                ))
            })?;

        // Create LUT buffer
        let (lut_colors, current_color_scheme) = if color_mode == ColorMode::ColorScheme {
            // Sample <num_species> + 1 equidistant stops
            // Use first color as background; remaining colors for species
            let raw_colors = lut
                .get_colors(settings.species_count as usize + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            let mut reordered_colors = Vec::with_capacity(settings.species_count as usize + 1);

            // Species colors are indices 1..=species_count
            for color in raw_colors
                .iter()
                .skip(1)
                .take(settings.species_count as usize)
            {
                reordered_colors.push(*color);
            }

            // Background color (index 0) is appended last
            reordered_colors.push(raw_colors[0]);

            tracing::trace!(
                "Constructor LUT mode: using first as background; stored {} colors for {} species (+ background)",
                reordered_colors.len(),
                settings.species_count
            );
            (reordered_colors, lut.name.clone())
        } else {
            let colors = lut
                .get_colors(settings.species_count as usize)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();
            tracing::trace!(
                "Constructor non-LUT mode: got {} colors for {} species",
                colors.len(),
                settings.species_count
            );
            (colors, lut.name.clone())
        };

        // Create initial state with proper LUT colors
        let state = State {
            particle_count,
            particles: vec![], // Empty - will be initialized on GPU
            random_seed: 0,
            dt: 0.016,
            cursor_size: 0.5,
            cursor_strength: 5.0,
            traces_enabled: false,
            trace_fade: 0.48,
            edge_fade_strength: 1.0,
            position_generator: PositionGenerator::Random,
            type_generator: TypeGenerator::Random,
            matrix_generator: MatrixGenerator::Random,
            current_color_scheme,
            color_scheme_reversed: true,
            background_color_mode: color_mode,
            species_colors: lut_colors.clone(),
            particle_size: 4.0,
            trail_map_filtering: super::settings::TrailMapFiltering::Nearest,
        };

        // Check buffer size limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let particle_buffer_size = (particle_count * std::mem::size_of::<Particle>()) as u64;

        if particle_buffer_size > max_storage_buffer_size {
            return Err(SimulationError::BufferTooLarge {
                requested: particle_buffer_size,
                max_available: max_storage_buffer_size,
            });
        }

        // Create empty particle buffer (will be initialized on GPU)
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: particle_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create simulation parameters buffer
        let sim_params = SimParams::new(width, height, particle_count as u32, &settings, &state);
        let sim_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sim Params Buffer"),
            contents: bytemuck::cast_slice(&[sim_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create force matrix buffer (flatten 2D matrix to 1D array)
        let force_matrix_data = Self::flatten_force_matrix(&settings.force_matrix);
        let force_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let lut_data_u32 = state
            .species_colors
            .iter()
            .flat_map(|&[r, g, b, a]| [r, g, b, a])
            .collect::<Vec<_>>();
        let lut_buffer = Arc::new(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LUT Buffer"),
                contents: bytemuck::cast_slice(&lut_data_u32),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
        );

        // Create compute shader and pipeline using GPU utilities
        let compute_shader = Arc::new(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_life_compute"),
            source: wgpu::ShaderSource::Wgsl(shaders::COMPUTE_SHADER.into()),
        }));

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Particle Life Compute Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                    resource_helpers::storage_buffer_entry(2, wgpu::ShaderStages::COMPUTE, true),
                ],
            });

        let compute_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(compute_shader)
            .with_bind_group_layouts(vec![compute_bind_group_layout.clone()])
            .with_label("Particle Life Compute Pipeline".to_string())
            .build();

        let compute_bind_group = BindGroupBuilder::new(device, &compute_bind_group_layout)
            .add_buffer(0, &particle_buffer)
            .add_buffer(1, &sim_params_buffer)
            .add_buffer(2, &force_matrix_buffer)
            .with_label("Particle Life Compute Bind Group".to_string())
            .build();

        // Create initialization compute shader and pipeline using GPU utilities
        let init_shader = Arc::new(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_life_init"),
            source: wgpu::ShaderSource::Wgsl(shaders::INIT_SHADER.into()),
        }));

        let init_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Particle Life Init Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
            });

        let init_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(init_shader)
            .with_bind_group_layouts(vec![init_bind_group_layout.clone()])
            .with_label("Particle Life Init Pipeline".to_string())
            .build();

        // Create init params buffer
        let init_params = InitParams {
            start_index: 0,
            spawn_count: particle_count as u32,
            species_count: settings.species_count,
            width: width as f32,
            height: height as f32,
            random_seed: state.random_seed,
            position_generator: state.position_generator as u32,
            type_generator: state.type_generator as u32,
            _pad1: 0,
            _pad2: 0,
        };

        let init_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Init Params Buffer"),
            contents: bytemuck::cast_slice(&[init_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let init_bind_group = BindGroupBuilder::new(device, &init_bind_group_layout)
            .add_buffer(0, &particle_buffer)
            .add_buffer(1, &init_params_buffer)
            .with_label("Particle Life Init Bind Group".to_string())
            .build();

        // Create force update compute shader and pipeline using GPU utilities
        let force_update_shader =
            Arc::new(device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("particle_life_force_update"),
                source: wgpu::ShaderSource::Wgsl(shaders::FORCE_UPDATE_SHADER.into()),
            }));

        let force_update_params = ForceUpdateParams {
            species_a: 0,
            species_b: 0,
            new_force: 0.0,
            species_count: settings.species_count,
        };
        let force_update_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Force Update Params Buffer"),
                contents: bytemuck::cast_slice(&[force_update_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let force_update_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Force Update Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
            });

        let force_update_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(force_update_shader)
            .with_bind_group_layouts(vec![force_update_bind_group_layout.clone()])
            .with_label("Force Update Pipeline".to_string())
            .build();

        let force_update_bind_group =
            BindGroupBuilder::new(device, &force_update_bind_group_layout)
                .add_buffer(0, &force_matrix_buffer)
                .add_buffer(1, &force_update_params_buffer)
                .with_label("Force Update Bind Group".to_string())
                .build();

        // Create force randomization compute shader and pipeline using GPU utilities
        let force_randomize_shader =
            Arc::new(device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("particle_life_force_randomize"),
                source: wgpu::ShaderSource::Wgsl(shaders::FORCE_RANDOMIZE_SHADER.into()),
            }));

        let force_randomize_params = ForceRandomizeParams {
            species_count: settings.species_count,
            random_seed: state.random_seed,
            min_force: -1.0,
            max_force: 1.0,
        };
        let force_randomize_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Force Randomize Params Buffer"),
                contents: bytemuck::cast_slice(&[force_randomize_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let force_randomize_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Force Randomize Bind Group Layout"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::COMPUTE),
                ],
            });

        let force_randomize_pipeline = ComputePipelineBuilder::new(device.clone())
            .with_shader(force_randomize_shader)
            .with_bind_group_layouts(vec![force_randomize_bind_group_layout.clone()])
            .with_label("Force Randomize Pipeline".to_string())
            .build();

        let force_randomize_bind_group =
            BindGroupBuilder::new(device, &force_randomize_bind_group_layout)
                .add_buffer(0, &force_matrix_buffer)
                .add_buffer(1, &force_randomize_params_buffer)
                .with_label("Force Randomize Bind Group".to_string())
                .build();

        // Create render shaders and pipeline
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::VERTEX_SHADER.into()),
        });

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FRAGMENT_SHADER.into()),
        });

        // Render bind group layout (particles + sim params)
        let render_bind_group_layout_particles =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout (Particles)"),
                entries: &[
                    resource_helpers::storage_buffer_entry(0, wgpu::ShaderStages::VERTEX, true),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::VERTEX),
                ],
            });

        // LUT bind group layout (species colors + color mode)
        let lut_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Species Colors Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        let render_bind_group_layout = lut_bind_group_layout.clone();

        // Camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::VERTEX),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::VERTEX),
                ],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Life Render Pipeline Layout"),
                bind_group_layouts: &[
                    &render_bind_group_layout_particles,
                    &render_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Life Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create tile render shader
        let tile_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Life Tile Render Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::TILE_RENDER_SHADER.into()),
        });

        // Create tile render bind group layout (Group 1: tile_params and camera_aware_params)
        let tile_render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Tile Render Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::VERTEX),
                    resource_helpers::uniform_buffer_entry(1, wgpu::ShaderStages::VERTEX),
                ],
            });

        // Create tile render pipeline layout
        let tile_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Tile Render Pipeline Layout"),
                bind_group_layouts: &[
                    &render_bind_group_layout_particles, // Group 0: particles and sim_params
                    &tile_render_bind_group_layout, // Group 1: tile_params and camera_aware_params
                    &lut_bind_group_layout,         // Group 2: species_colors
                ],
                push_constant_ranges: &[],
            });

        // Create tile render pipeline
        let tile_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Life Tile Render Pipeline"),
            layout: Some(&tile_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &tile_render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &tile_render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create offscreen render pipeline layout (with camera)
        let offscreen_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Life Offscreen Render Pipeline Layout"),
                bind_group_layouts: &[
                    &render_bind_group_layout_particles,
                    &render_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Create offscreen render pipeline for display texture (Rgba8Unorm format) - MSAA version
        let offscreen_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Particle Life Offscreen Render Pipeline"),
                layout: Some(&offscreen_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: Some("main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::REPLACE),
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
                    count: 4, // 4x MSAA
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        // Create display render pipeline for direct display texture rendering (non-MSAA)
        let display_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Particle Life Display Render Pipeline"),
                layout: Some(&offscreen_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: Some("main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::REPLACE),
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
                multisample: wgpu::MultisampleState::default(), // No MSAA
                multiview: None,
                cache: None,
            });

        // Create trail render pipeline for trail texture (uses surface format)
        let trail_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Particle Life Trail Render Pipeline"),
                layout: Some(&offscreen_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: Some("main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm, // Use RGBA format for proper alpha support
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Use alpha blending for trails
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
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Create bind groups
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout_particles,
            entries: &[
                resource_helpers::buffer_entry(0, &particle_buffer),
                resource_helpers::buffer_entry(1, &sim_params_buffer),
            ],
        });

        // Create a simple LUT texture for now (we'll implement proper LUT support later)
        let lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("LUT Texture"),
            size: wgpu::Extent3d {
                width: state.species_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload LUT colors to texture
        let lut_data: Vec<u8> = state
            .species_colors
            .iter()
            .flat_map(|&[r, g, b, a]| {
                [
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (a * 255.0) as u8,
                ]
            })
            .collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &lut_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &lut_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(state.species_colors.len() as u32 * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: state.species_colors.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        // Create LUT size uniform buffer
        let lut_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Size Buffer"),
            contents: bytemuck::cast_slice(&[state.species_colors.len() as u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create color mode uniform buffer (16 bytes to match shader struct)
        let color_mode_value = match color_mode {
            ColorMode::Gray18 => 0u32,
            ColorMode::White => 1u32,
            ColorMode::Black => 2u32,
            ColorMode::ColorScheme => 3u32,
        };
        let color_mode_data = [color_mode_value, 0u32, 0u32, 0u32]; // 16 bytes with padding
        let color_mode_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Color Mode Buffer"),
            contents: bytemuck::cast_slice(&color_mode_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create species colors buffer
        let species_colors_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Species Colors Buffer"),
            size: (9 * std::mem::size_of::<[f32; 4]>()) as u64, // Allocate space for 9 colors (background + 8 species)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create initial species colors data (pad to 9 colors)
        let mut species_colors_data = vec![[0.0f32, 0.0f32, 0.0f32, 1.0f32]; 9];
        for (i, &color) in state.species_colors.iter().enumerate().take(9) {
            if i < settings.species_count as usize {
                species_colors_data[i] = color;
            }
        }

        // Upload initial species colors to GPU buffer
        let species_colors_bytes = bytemuck::cast_slice(&species_colors_data);
        queue.write_buffer(&species_colors_buffer, 0, species_colors_bytes);

        // Create initial species colors bind group
        let lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Species Colors Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &species_colors_buffer),
                resource_helpers::buffer_entry(1, &color_mode_buffer),
            ],
        });

        // Create camera
        let camera = Camera::new(device, width as f32, height as f32)?;

        // Create viewport parameters buffer for fixed tile rendering
        let viewport_params = ViewportParams {
            world_bounds: [-1.0, -1.0, 1.0, 1.0], // Fixed 2x2 world unit tile
            texture_size: [width as f32, height as f32],
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let viewport_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Viewport Params Buffer"),
            contents: bytemuck::cast_slice(&[viewport_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, camera.buffer()),
                resource_helpers::buffer_entry(1, &viewport_params_buffer),
            ],
        });

        // Create fade pipeline for traces
        let fade_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fade Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_VERTEX_SHADER.into()),
        });

        let fade_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fade Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_FRAGMENT_SHADER.into()),
        });

        let fade_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Fade Bind Group Layout"),
                entries: &[
                    resource_helpers::uniform_buffer_entry(0, wgpu::ShaderStages::FRAGMENT),
                    resource_helpers::texture_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    resource_helpers::sampler_entry(
                        2,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                ],
            });

        let fade_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fade Pipeline Layout"),
            bind_group_layouts: &[&fade_bind_group_layout],
            push_constant_ranges: &[],
        });

        let fade_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fade Pipeline"),
            layout: Some(&fade_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fade_vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fade_fragment_shader,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm, // Use RGBA format to match trail texture
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create fade uniforms buffer
        let fade_uniforms = FadeUniforms {
            fade_amount: 0.01,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        let fade_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fade Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[fade_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create temporary placeholder texture and sampler for fade bind group
        let placeholder_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Placeholder Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let placeholder_texture_view =
            placeholder_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let placeholder_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Placeholder Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: app_settings.texture_filtering.into(),
            min_filter: app_settings.texture_filtering.into(),
            mipmap_filter: app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // Create a temporary fade bind group with placeholder resources - will be updated later with display texture
        let fade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fade Bind Group"),
            layout: &fade_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &fade_uniforms_buffer),
                resource_helpers::texture_view_entry(1, &placeholder_texture_view),
                resource_helpers::sampler_bind_entry(2, &placeholder_sampler),
            ],
        });

        // Trail textures for persistent trails (ping-pong buffers)
        let trail_texture_a = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Trail Texture A"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Use RGBA format for proper alpha support
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let trail_texture_view_a = trail_texture_a.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Trail Texture View A"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(wgpu::TextureFormat::Rgba8Unorm), // Use RGBA format for proper alpha support
            ..Default::default()
        });

        let trail_texture_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Trail Texture B"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, // Use RGBA format for proper alpha support
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let trail_texture_view_b = trail_texture_b.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Trail Texture View B"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(wgpu::TextureFormat::Rgba8Unorm), // Use RGBA format for proper alpha support
            ..Default::default()
        });

        // Blit pipeline to copy trail texture to surface
        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Blit Bind Group Layout"),
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
                ],
            });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create blit shaders
        let blit_vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::FADE_VERTEX_SHADER.into()), // Reuse fade vertex shader
        });

        let blit_fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
                @group(0) @binding(0) var trail_texture: texture_2d<f32>;
                @group(0) @binding(1) var trail_sampler: sampler;

                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                }

                @fragment
                fn main(input: VertexOutput) -> @location(0) vec4<f32> {
                    return textureSample(trail_texture, trail_sampler, input.uv);
                }
                "#
                .into(),
            ),
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_fragment_shader,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None, // No blending for blit
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create display blit pipeline for copying trail texture to display texture (Rgba8Unorm format)
        let display_blit_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Display Blit Pipeline"),
                layout: Some(&blit_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &blit_vertex_shader,
                    entry_point: Some("main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &blit_fragment_shader,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None, // No blending for blit
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
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Create sampler for blit
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blit Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: app_settings.texture_filtering.into(),
            min_filter: app_settings.texture_filtering.into(),
            mipmap_filter: app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // Create sampler for post effect
        let post_effect_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Post Effect Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: app_settings.texture_filtering.into(),
            min_filter: app_settings.texture_filtering.into(),
            mipmap_filter: app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // Create sampler for display
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Display Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: app_settings.texture_filtering.into(),
            min_filter: app_settings.texture_filtering.into(),
            mipmap_filter: app_settings.texture_filtering.into(),
            ..Default::default()
        });

        // Blit bind group (initially uses texture A)
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &blit_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &trail_texture_view_a),
                resource_helpers::sampler_bind_entry(1, &blit_sampler),
            ],
        });

        // Create background render shader
        let background_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background Render Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::BACKGROUND_RENDER_SHADER.into()),
        });

        // Create background bind group layout
        let background_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Background Bind Group Layout"),
                entries: &[resource_helpers::uniform_buffer_entry(
                    0,
                    wgpu::ShaderStages::FRAGMENT,
                )],
            });

        // Create background render pipeline
        let background_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Background Render Pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Background Render Pipeline Layout"),
                        bind_group_layouts: &[&background_bind_group_layout],
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
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
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

        // Create background parameters buffer
        let background_params = BackgroundParams {
            background_color: [0.0, 0.0, 0.0, 1.0], // Black background by default
        };
        let background_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Background Params Buffer"),
                contents: bytemuck::cast_slice(&[background_params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create background bind group
        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Bind Group"),
            layout: &background_bind_group_layout,
            entries: &[resource_helpers::buffer_entry(0, &background_params_buffer)],
        });

        // Create display texture for offscreen rendering
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Display Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let display_view = display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create MSAA texture for anti-aliasing particle rendering
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4, // 4x MSAA
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create display bind group for blitting display texture to surface
        let display_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Display Bind Group"),
            layout: &blit_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &display_view),
                resource_helpers::sampler_bind_entry(1, &blit_sampler),
            ],
        });

        // Post effect texture for post-processing
        let post_effect_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Post Effect Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let post_effect_view =
            post_effect_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Post effect parameters buffer
        let post_effect_params = [1.0f32, 1.0f32, 1.0f32, 1.0f32]; // brightness, contrast, saturation, gamma
        let post_effect_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Post Effect Params Buffer"),
                contents: bytemuck::cast_slice(&post_effect_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Texture render parameters buffer for infinite render shader
        let texture_render_params = [1u32, 0u32, 0u32, 0u32]; // filtering_mode, _pad1, _pad2, _pad3
        let texture_render_params_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Texture Render Params Buffer"),
                contents: bytemuck::cast_slice(&texture_render_params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create post effect shader
        let post_effect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Effect Shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulations::particle_life::shaders::POST_EFFECT_SHADER.into(),
            ),
        });

        // Create post effect bind group layout
        let post_effect_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Post Effect Bind Group Layout"),
                entries: &[
                    // Display texture
                    resource_helpers::texture_entry(
                        0,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    // Display sampler
                    resource_helpers::sampler_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    // Post effect params
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        // Create post effect pipeline layout
        let post_effect_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Post Effect Pipeline Layout"),
                bind_group_layouts: &[&post_effect_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create post effect pipeline
        let post_effect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Post Effect Pipeline"),
            layout: Some(&post_effect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &post_effect_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_effect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create infinite render shader
        let infinite_render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Infinite Render Shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulations::particle_life::shaders::INFINITE_RENDER_SHADER.into(),
            ),
        });

        // Create infinite render bind group layout
        let infinite_render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Infinite Render Bind Group Layout"),
                entries: &[
                    // Display texture
                    resource_helpers::texture_entry(
                        0,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::TextureSampleType::Float { filterable: true },
                        wgpu::TextureViewDimension::D2,
                    ),
                    // Display sampler
                    resource_helpers::sampler_entry(
                        1,
                        wgpu::ShaderStages::FRAGMENT,
                        wgpu::SamplerBindingType::Filtering,
                    ),
                    // Texture render params
                    resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                ],
            });

        // Create infinite render pipeline layout
        let infinite_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Infinite Render Pipeline Layout"),
                bind_group_layouts: &[
                    &infinite_render_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Create infinite render pipeline
        let render_infinite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Infinite Render Pipeline"),
                layout: Some(&infinite_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &infinite_render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &infinite_render_shader,
                    entry_point: Some("fs_main_texture"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
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
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Create post effect bind group
        let post_effect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Effect Bind Group"),
            layout: &post_effect_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &display_view),
                resource_helpers::sampler_bind_entry(1, &blit_sampler),
                resource_helpers::buffer_entry(2, &post_effect_params_buffer),
            ],
        });

        // Create infinite render bind group (uses post-effect texture)
        let render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Infinite Render Bind Group"),
            layout: &infinite_render_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, &post_effect_view),
                resource_helpers::sampler_bind_entry(1, &blit_sampler),
                resource_helpers::buffer_entry(2, &texture_render_params_buffer),
            ],
        });

        // Create infinite render bind group for display texture (bypasses post-effects)
        let render_infinite_display_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Infinite Render Display Bind Group"),
                layout: &infinite_render_bind_group_layout,
                entries: &[
                    resource_helpers::texture_view_entry(0, &display_view),
                    resource_helpers::sampler_bind_entry(1, &blit_sampler),
                    resource_helpers::buffer_entry(2, &texture_render_params_buffer),
                ],
            });

        let mut result = Self {
            particle_buffer: particle_buffer.clone(),
            sim_params_buffer: sim_params_buffer.clone(),
            force_matrix_buffer,
            lut_buffer,
            lut_size_buffer,
            color_mode_buffer,
            species_colors_buffer: species_colors_buffer.clone(),
            compute_pipeline,
            compute_bind_group,
            compute_bind_group_layout,
            init_pipeline,
            init_bind_group,
            init_bind_group_layout,
            init_params_buffer,
            force_update_pipeline,
            force_update_params_buffer,
            force_update_bind_group,
            force_randomize_pipeline,
            force_randomize_params_buffer,
            force_randomize_bind_group,
            render_pipeline,
            render_bind_group_layout,
            render_particles_bind_group_layout: render_bind_group_layout_particles,
            render_bind_group,
            lut_bind_group,
            camera_bind_group,
            tile_render_pipeline,
            tile_render_bind_group_layout: tile_render_bind_group_layout.clone(),
            tile_render_bind_group: {
                let tile_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Tile Params Buffer"),
                    size: std::mem::size_of::<TileParams>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let camera_aware_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Camera Aware Params Buffer"),
                    size: std::mem::size_of::<CameraAwareParams>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Tile Render Bind Group"),
                    layout: &tile_render_bind_group_layout,
                    entries: &[
                        resource_helpers::buffer_entry(0, &tile_params_buffer),
                        resource_helpers::buffer_entry(1, &camera_aware_params_buffer),
                    ],
                })
            },
            tile_params_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Tile Params Buffer"),
                size: std::mem::size_of::<TileParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            offscreen_render_pipeline,
            display_render_pipeline,
            trail_render_pipeline,
            fade_pipeline,
            fade_bind_group_layout,
            fade_bind_group,
            fade_uniforms_buffer,
            trail_texture_a,
            trail_texture_view_a,
            trail_texture_b,
            trail_texture_view_b,
            current_trail_is_a: true,
            blit_pipeline,
            display_blit_pipeline,
            blit_bind_group_layout,
            blit_bind_group,
            background_render_pipeline,
            background_bind_group_layout,
            background_bind_group,
            background_params_buffer,
            viewport_params_buffer,
            display_texture,
            display_view,
            display_bind_group,
            msaa_texture,
            msaa_view,
            post_effect_texture,
            post_effect_view,
            post_effect_bind_group,
            render_infinite_pipeline,
            render_infinite_bind_group,
            render_infinite_display_bind_group,
            post_effect_pipeline,
            post_effect_params_buffer,
            post_effect_bind_group_layout,
            infinite_render_bind_group_layout,
            texture_render_params_buffer,
            blit_sampler,
            post_effect_sampler,
            display_sampler,
            settings,
            state,
            gui_visible: true,
            color_scheme_manager: Arc::new(color_scheme_manager.clone()),
            width,
            height,
            camera,
            last_frame_time: std::time::Instant::now(),
            cursor_active_mode: 0,
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
            current_resolution_scale: 1.0,
            last_zoom_level: 1.0,
            base_surface_width: surface_config.width,
            base_surface_height: surface_config.height,
            camera_aware_params_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Camera Aware Params Buffer"),
                size: std::mem::size_of::<CameraAwareParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            post_processing_state: PostProcessingState::default(),
            post_processing_resources: PostProcessingResources::new(device, surface_config)?,
        };

        // Initialize LUT and species colors properly
        let color_scheme_manager_clone = result.color_scheme_manager.clone();
        result.update_lut(
            device,
            queue,
            &color_scheme_manager_clone,
            color_mode,
            Some("MATPLOTLIB_ocean"),
            true,
        )?;

        // Initialize background parameters
        result.update_background_params(queue);

        // Initialize particles on GPU
        result.initialize_particles_gpu(device, queue)?;

        // Initialize trail texture with background color
        let background_color = match color_mode {
            ColorMode::Gray18 => wgpu::Color {
                r: 0.18,
                g: 0.18,
                b: 0.18,
                a: 1.0,
            },
            ColorMode::White => wgpu::Color::WHITE,
            ColorMode::Black => wgpu::Color::BLACK,
            ColorMode::ColorScheme => {
                if !result.state.species_colors.is_empty() {
                    // Background is appended at the end (index = species_count)
                    let bg_index = (result.settings.species_count as usize)
                        .min(result.state.species_colors.len() - 1);
                    let [r, g, b, a] = result.state.species_colors[bg_index];
                    wgpu::Color {
                        r: r.into(),
                        g: g.into(),
                        b: b.into(),
                        a: a.into(),
                    }
                } else {
                    wgpu::Color::BLACK
                }
            }
        };
        result.clear_trail_texture(device, queue, background_color);

        // Update fade bind group with display texture
        result.update_fade_bind_group(device);

        Ok(result)
    }

    fn update_sim_params(&mut self, _device: &Arc<Device>, queue: &Arc<Queue>) {
        let mut sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );

        // Override with stored cursor values if cursor is active
        sim_params.cursor_x = self.cursor_world_x;
        sim_params.cursor_y = self.cursor_world_y;
        sim_params.cursor_active = self.cursor_active_mode;
        if self.cursor_active_mode > 0 {
            sim_params.cursor_strength =
                self.state.cursor_strength * self.settings.max_force * 10.0;
        }
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
    }

    fn update_viewport_params(&mut self, queue: &Arc<Queue>) {
        // Use fixed world region for texture (matches infinite renderer tile size)
        // Each tile in the infinite renderer represents a 2x2 world unit region
        let world_left = -1.0;
        let world_right = 1.0;
        let world_bottom = -1.0;
        let world_top = 1.0;

        let viewport_params = ViewportParams {
            world_bounds: [world_left, world_bottom, world_right, world_top],
            texture_size: [
                self.display_texture.width() as f32,
                self.display_texture.height() as f32,
            ],
            _pad1: 0.0,
            _pad2: 0.0,
        };

        queue.write_buffer(
            &self.viewport_params_buffer,
            0,
            bytemuck::cast_slice(&[viewport_params]),
        );
    }

    fn initialize_particles_gpu(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update init params with current particle count
        let init_params = InitParams {
            start_index: 0,
            spawn_count: self.state.particle_count as u32,
            species_count: self.settings.species_count,
            width: self.width as f32,
            height: self.height as f32,
            random_seed: self.state.random_seed,
            position_generator: self.state.position_generator as u32,
            type_generator: self.state.type_generator as u32,
            _pad1: 0,
            _pad2: 0,
        };

        queue.write_buffer(
            &self.init_params_buffer,
            0,
            bytemuck::cast_slice(&[init_params]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Init Encoder"),
        });

        {
            let mut init_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Particle Life Init Pass"),
                timestamp_writes: None,
            });

            init_pass.set_pipeline(&self.init_pipeline);
            init_pass.set_bind_group(0, &self.init_bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups = self.state.particle_count.div_ceil(workgroup_size);
            init_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn reset_particles_gpu(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        tracing::info!(
            "Resetting particles with count: {}",
            self.state.particle_count
        );

        // Update random seed for reset
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.state.random_seed as u64);
        self.state.random_seed = rng.random();

        // Update sim params with new random seed and current particle count
        self.update_sim_params(device, queue);

        tracing::info!(
            "Reinitializing {} particles on GPU",
            self.state.particle_count
        );
        // Re-initialize particles on GPU
        self.initialize_particles_gpu(device, queue)?;

        // Ensure GPU operations complete
        device
            .poll(wgpu::wgt::PollType::Wait)
            .expect("Failed to poll device");

        tracing::info!("Particle reset complete");
        Ok(())
    }

    pub fn update_force_element_gpu(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        species_a: u32,
        species_b: u32,
        new_force: f32,
    ) -> SimulationResult<()> {
        let update_params = ForceUpdateParams {
            species_a,
            species_b,
            new_force,
            species_count: self.settings.species_count,
        };

        // Update the uniform buffer
        queue.write_buffer(
            &self.force_update_params_buffer,
            0,
            bytemuck::cast_slice(&[update_params]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Force Update Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Update Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.force_update_pipeline);
            compute_pass.set_bind_group(0, &self.force_update_bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    pub fn randomize_force_matrix_gpu(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update random seed
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.state.random_seed as u64);
        let new_seed = rng.random();

        let randomize_params = ForceRandomizeParams {
            species_count: self.settings.species_count,
            random_seed: new_seed,
            min_force: -1.0,
            max_force: 1.0,
        };

        // Update the uniform buffer
        queue.write_buffer(
            &self.force_randomize_params_buffer,
            0,
            bytemuck::cast_slice(&[randomize_params]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Force Randomize Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Randomize Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.force_randomize_pipeline);
            compute_pass.set_bind_group(0, &self.force_randomize_bind_group, &[]);

            // Dispatch with enough workgroups to cover the species matrix
            let workgroup_size = 8;
            let num_workgroups = self.settings.species_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups, num_workgroups, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn recreate_bind_groups_with_force_matrix(&mut self, device: &Arc<Device>) {
        // Recreate compute bind group with new force matrix
        self.compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Life Compute Bind Group"),
            layout: &self.compute_pipeline.get_bind_group_layout(0),
            entries: &[
                resource_helpers::buffer_entry(0, &self.particle_buffer),
                resource_helpers::buffer_entry(1, &self.sim_params_buffer),
                resource_helpers::buffer_entry(2, &self.force_matrix_buffer),
            ],
        });
    }

    /// Update the LUT with new settings
    pub fn update_lut(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        color_scheme_manager: &ColorSchemeManager,
        color_mode: ColorMode,
        color_scheme: Option<&str>,
        color_scheme_reversed: bool,
    ) -> SimulationResult<()> {
        // Update color mode
        self.state.background_color_mode = color_mode;

        // Get LUT name and validate
        let color_scheme = color_scheme.unwrap_or(&self.state.current_color_scheme);
        if color_scheme.is_empty() {
            return Err(SimulationError::InvalidSetting {
                setting_name: "lut_name".to_string(),
                message: "LUT name is empty but LUT color mode is enabled".to_string(),
            });
        }

        let mut lut = color_scheme_manager.get(color_scheme).map_err(|e| {
            SimulationError::InvalidSetting {
                setting_name: "lut_name".to_string(),
                message: format!("Failed to load LUT '{}': {}", color_scheme, e),
            }
        })?;

        if color_scheme_reversed {
            lut = lut.reversed();
        }

        // Compute species colors based on color mode
        let species_count = self.settings.species_count as usize;
        let mut species_colors = Vec::with_capacity(species_count);

        if color_mode == ColorMode::ColorScheme {
            // Sample <species_count> + 1 colors: first is background, remaining are species
            let raw_colors = lut
                .get_colors(species_count + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            // Species colors from indices 1..=species_count
            for color in raw_colors.iter().skip(1).take(species_count) {
                species_colors.push(*color);
            }

            // Append background color at the end
            species_colors.push(raw_colors[0]);

            tracing::debug!(
                "LUT mode: using first as background; got {} colors for {} species (+ background)",
                species_colors.len(),
                species_count
            );
        } else {
            // Get species_count colors for non-LUT mode
            let lut_colors = lut
                .get_colors(species_count)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            // Direct mapping for non-LUT mode
            for species_index in 0..species_count {
                let color = lut_colors
                    .get(species_index)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                species_colors.push(color);
            }

            tracing::debug!(
                "Non-LUT mode: got {} colors for {} species",
                species_colors.len(),
                species_count
            );
        }

        // Update stored colors and LUT info
        self.state.species_colors = species_colors;
        // Store the original LUT name, not the reversed LUT name
        self.state.current_color_scheme = color_scheme.to_string();
        self.state.color_scheme_reversed = color_scheme_reversed;

        tracing::debug!(
            "Updated LUT: name={}, reversed={}, species_colors.len={}",
            self.state.current_color_scheme,
            self.state.color_scheme_reversed,
            self.state.species_colors.len()
        );

        // Update species colors on GPU
        self.update_species_colors_gpu(device, queue)?;

        Ok(())
    }

    /// Update species colors on GPU
    fn update_species_colors_gpu(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // In LUT mode, we have species_count + 1 colors (background + species)
        // In non-LUT mode, we have species_count colors
        let total_colors = self.state.species_colors.len();

        // Prepare species colors data (pad to 9 colors to accommodate LUT mode)
        let mut species_colors_data = [[0.0f32, 0.0f32, 0.0f32, 1.0f32]; 9];
        for (i, &color) in self.state.species_colors.iter().enumerate().take(9) {
            species_colors_data[i] = color;
        }

        // Upload species colors to GPU buffer
        let species_colors_bytes = bytemuck::cast_slice(&species_colors_data);
        queue.write_buffer(&self.species_colors_buffer, 0, species_colors_bytes);

        // Update color mode buffer (16 bytes to match shader struct)
        let color_mode_value = match self.state.background_color_mode {
            ColorMode::Gray18 => 0u32,
            ColorMode::White => 1u32,
            ColorMode::Black => 2u32,
            ColorMode::ColorScheme => 3u32,
        };
        let color_mode_data = [color_mode_value, 0u32, 0u32, 0u32]; // 16 bytes with padding
        queue.write_buffer(
            &self.color_mode_buffer,
            0,
            bytemuck::cast_slice(&color_mode_data),
        );

        // Update species colors bind group
        self.lut_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Species Colors Bind Group"),
            layout: &self.render_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &self.species_colors_buffer),
                resource_helpers::buffer_entry(1, &self.color_mode_buffer),
            ],
        });

        tracing::debug!(
            "Updated GPU colors: total_colors={}, color_mode={:?}",
            total_colors,
            self.state.background_color_mode
        );

        Ok(())
    }

    /// Get the current LUT size for shader uniform
    pub fn get_lut_size(&self) -> u32 {
        self.state.species_colors.len() as u32
    }

    /// Update fade uniforms for trace rendering
    fn update_fade_uniforms(&self, queue: &Arc<Queue>, fade_amount: f32) {
        let fade_uniforms = FadeUniforms {
            fade_amount,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        queue.write_buffer(
            &self.fade_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[fade_uniforms]),
        );
    }

    fn update_fade_bind_group(&mut self, device: &Arc<Device>) {
        // Read from the previous trail texture (opposite of current write target)
        let read_texture_view = self.get_read_trail_texture_view();

        self.fade_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fade Bind Group"),
            layout: &self.fade_bind_group_layout,
            entries: &[
                resource_helpers::buffer_entry(0, &self.fade_uniforms_buffer),
                resource_helpers::texture_view_entry(1, read_texture_view),
                resource_helpers::sampler_bind_entry(2, &self.display_sampler),
            ],
        });
    }

    /// Get the trail texture view to write to (render target)
    fn get_write_trail_texture_view(&self) -> &wgpu::TextureView {
        if self.current_trail_is_a {
            &self.trail_texture_view_a
        } else {
            &self.trail_texture_view_b
        }
    }

    /// Get the trail texture view to read from (for fade shader)
    fn get_read_trail_texture_view(&self) -> &wgpu::TextureView {
        if self.current_trail_is_a {
            &self.trail_texture_view_b // Read from B when writing to A
        } else {
            &self.trail_texture_view_a // Read from A when writing to B
        }
    }

    /// Swap the ping-pong trail textures after each frame
    fn swap_trail_textures(&mut self) {
        self.current_trail_is_a = !self.current_trail_is_a;
    }

    /// Update blit bind group to read from the current completed trail texture
    fn update_blit_bind_group(&mut self, device: &Arc<Device>) {
        // For blitting, we want to read from the texture we just finished writing to
        let read_texture_view = self.get_write_trail_texture_view();

        self.blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                resource_helpers::texture_view_entry(0, read_texture_view),
                resource_helpers::sampler_bind_entry(1, &self.display_sampler),
            ],
        });
    }

    /// Update background color based on color mode
    pub fn update_background_params(&mut self, queue: &Arc<Queue>) {
        // Get background color based on color mode
        let background_color = match self.state.background_color_mode {
            ColorMode::Black => [0.0, 0.0, 0.0, 1.0],     // Black
            ColorMode::White => [1.0, 1.0, 1.0, 1.0],     // White
            ColorMode::Gray18 => [0.18, 0.18, 0.18, 1.0], // Gray18
            ColorMode::ColorScheme => {
                // Background color is appended at the end of species_colors in LUT mode
                if !self.state.species_colors.is_empty() {
                    // Prefer the species_count index when present, otherwise fallback to last
                    let bg_index = (self.settings.species_count as usize)
                        .min(self.state.species_colors.len() - 1);
                    self.state.species_colors[bg_index]
                } else {
                    [0.0, 0.0, 0.0, 1.0] // Fallback to black
                }
            }
        };

        // Update background parameters
        let background_params = BackgroundParams { background_color };

        queue.write_buffer(
            &self.background_params_buffer,
            0,
            bytemuck::cast_slice(&[background_params]),
        );
    }

    pub fn clear_trail_texture(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        background_color: wgpu::Color,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Trail Texture Encoder"),
        });

        // Clear both trail textures
        {
            let _render_pass_a = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Trail Texture A Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.trail_texture_view_a,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        {
            let _render_pass_b = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Trail Texture B Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.trail_texture_view_b,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_color),
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
    }

    /// Calculate which tiles are visible based on camera position and zoom
    ///
    /// Calculate how many tiles we need based on zoom level
    fn calculate_tile_count(zoom: f32) -> i32 {
        // At zoom 1.0, we need at least 7x7 tiles
        // As zoom decreases (zooming out), we need more tiles
        // Each tile covers 2.0 world units, so we need enough tiles to cover the visible area
        let visible_world_size = 2.0 / zoom; // World size visible on screen
        let tiles_needed = (visible_world_size / 2.0).ceil() as i32 + 8; // +8 for extra padding to prevent gaps
        let min_tiles = if zoom < 0.1 { 9 } else { 7 }; // More tiles needed at extreme zoom out
        // Allow more tiles for proper infinite tiling, but cap at reasonable limit
        ((tiles_needed).max(min_tiles)).min(1024) // Cap at 1024 for performance
    }

    /// Update camera-aware parameters for tile-based rendering
    fn update_camera_aware_params(&mut self, queue: &Arc<Queue>) {
        let camera_aware_params = CameraAwareParams {
            camera_zoom: self.camera.zoom,
            _pad0: 0.0,
            camera_position: self.camera.position,
            viewport_size: [self.camera.viewport_width, self.camera.viewport_height],
            tile_size: 2.0,
            max_tiles: 64, // Limit to prevent performance issues
            _pad1: 0.0,
            _pad2: 0.0,
        };

        queue.write_buffer(
            &self.camera_aware_params_buffer,
            0,
            bytemuck::cast_slice(&[camera_aware_params]),
        );
    }

    fn apply_post_processing(
        &self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        input_texture_view: &wgpu::TextureView,
        output_texture_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> crate::error::SimulationResult<()> {
        // Apply blur filter if enabled
        if self.post_processing_state.blur_filter.enabled {
            // Update blur parameters
            self.post_processing_resources.update_blur_params(
                queue,
                self.post_processing_state.blur_filter.radius,
                self.post_processing_state.blur_filter.sigma,
                self.width,
                self.height,
            );

            // Create blur bind group with the input texture
            let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Post Processing Blur Bind Group"),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Blur Bind Group Layout"),
                    entries: &[
                        // Input texture
                        resource_helpers::texture_entry(
                            0,
                            wgpu::ShaderStages::FRAGMENT,
                            wgpu::TextureSampleType::Float { filterable: true },
                            wgpu::TextureViewDimension::D2,
                        ),
                        // Sampler
                        resource_helpers::sampler_entry(
                            1,
                            wgpu::ShaderStages::FRAGMENT,
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        // Parameters
                        resource_helpers::uniform_buffer_entry(2, wgpu::ShaderStages::FRAGMENT),
                    ],
                }),
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

            // Apply blur filter using the provided encoder
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
        }

        Ok(())
    }
}

impl Simulation for ParticleLifeModel {
    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Calculate delta time for smooth camera movement
        let current_time = std::time::Instant::now();
        let delta_time = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;

        // Use actual delta time for real-time simulation
        // Only clamp to prevent extreme jumps when tab is inactive
        let delta_time = delta_time.min(1.0); // Max 1 second jump

        // Update camera with smoothing using actual delta time
        self.camera.update(delta_time);

        // Update camera
        self.camera.upload_to_gpu(queue);

        // Update viewport parameters for camera-aware rendering
        self.update_viewport_params(queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Static Render Encoder"),
        });

        // Skip compute pass - just render current particle positions

        // Step 1: Render background to display texture (offscreen)
        {
            let mut background_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Static Background Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), // Clear to transparent, background shader will fill
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            background_pass.set_pipeline(&self.background_render_pipeline);
            background_pass.set_bind_group(0, &self.background_bind_group, &[]);
            background_pass.draw(0..6, 0..1); // Fullscreen triangle
        }

        // Step 2: Render particles to display texture (offscreen)
        if self.state.traces_enabled {
            // For static rendering, just render to current trail texture without fade
            let write_texture_view = self.get_write_trail_texture_view();
            let mut trail_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Static Trail Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: write_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve previous trail content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // For static rendering, don't add new trails - just render existing particles
            trail_render_pass.set_pipeline(&self.trail_render_pipeline);
            trail_render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            trail_render_pass.set_bind_group(1, &self.lut_bind_group, &[]);

            let particle_count = self.state.particle_count as u32;
            trail_render_pass.draw(0..6, 0..particle_count);
            drop(trail_render_pass);

            // Now blit trail texture to display texture
            let mut display_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Static Display Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load existing background
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            display_render_pass.set_pipeline(&self.blit_pipeline);
            display_render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
            display_render_pass.draw(0..3, 0..1);
        } else {
            // When trails are disabled, render particles directly to display texture
            let mut display_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Static Display Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.display_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load existing background
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            display_render_pass.set_pipeline(&self.display_render_pipeline);
            display_render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            display_render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            display_render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

            let particle_count = self.state.particle_count as u32;
            display_render_pass.draw(0..6, 0..particle_count);
        }

        // Step 3: Apply post-processing if enabled
        if self.post_processing_state.blur_filter.enabled {
            // Apply post-processing: input from display_view, output to intermediate_view
            self.apply_post_processing(
                device,
                queue,
                &self.display_view,
                &self.post_processing_resources.intermediate_view,
                &mut encoder,
            )?;

            // Copy the blurred result back to the display texture using the main encoder
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
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        // Step 4: Render post effects from display texture to post-effect texture (offscreen)
        // Skip expensive post-effect pass when using default parameters
        if self.needs_post_effects() {
            let mut post_effect_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Static Post Effect Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.post_effect_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            post_effect_pass.set_pipeline(&self.post_effect_pipeline);
            post_effect_pass.set_bind_group(0, &self.post_effect_bind_group, &[]);
            post_effect_pass.draw(0..6, 0..1);
        }

        // Step 4: Render texture to surface with infinite renderer
        // Use display texture directly when post-effects are disabled for better performance
        {
            let tile_count = Self::calculate_tile_count(self.camera.zoom);
            let total_instances = (tile_count * tile_count) as u32;

            let mut surface_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Static Surface Render Pass"),
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            surface_render_pass.set_pipeline(&self.render_infinite_pipeline);
            let bind_group = if self.needs_post_effects() {
                &self.render_infinite_bind_group
            } else {
                &self.render_infinite_display_bind_group
            };
            surface_render_pass.set_bind_group(0, bind_group, &[]);
            surface_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            surface_render_pass.draw(0..6, 0..total_instances);
        }

        queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        // Check if resolution needs to be updated based on zoom level
        if self.should_update_resolution() {
            self.update_resolution(device)?;
        }

        // Use provided delta time for smooth camera movement
        // Only clamp to prevent extreme jumps when tab is inactive
        let delta_time = delta_time.min(1.0); // Max 1 second jump

        // Update GPU buffers with current state
        self.update_sim_params(device, queue);

        // Update camera with smoothing using actual delta time
        self.camera.update(delta_time);

        // Update camera
        self.camera.upload_to_gpu(queue);

        // Update camera-aware parameters for tile-based rendering
        self.update_camera_aware_params(queue);

        // Update background parameters
        self.update_background_params(queue);

        // Use separate command encoders to avoid texture usage conflicts
        // First encoder: compute and trail rendering
        let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Compute Encoder"),
        });

        // Single physics step per frame for proper timing
        {
            let mut compute_pass =
                compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Particle Life Compute Pass"),
                    timestamp_writes: None,
                });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups = self.state.particle_count.div_ceil(workgroup_size);
            compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        // Step 1: Render background to display texture (offscreen)
        {
            let mut background_pass =
                compute_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Background Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), // Clear to transparent, background shader will fill
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            background_pass.set_pipeline(&self.background_render_pipeline);
            background_pass.set_bind_group(0, &self.background_bind_group, &[]);
            background_pass.draw(0..6, 0..1); // Fullscreen triangle
        }

        // Step 2: Render particles to display texture (offscreen)
        if self.state.traces_enabled {
            // Update bind groups for ping-pong rendering
            self.update_fade_bind_group(device);

            // When trails are enabled, render to trail texture first
            // Use ping-pong textures to avoid read/write conflicts
            let write_texture_view = self.get_write_trail_texture_view();

            // Get background color for clearing
            let background_color = match self.state.background_color_mode {
                ColorMode::Black => wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                ColorMode::White => wgpu::Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                ColorMode::Gray18 => wgpu::Color {
                    r: 0.18,
                    g: 0.18,
                    b: 0.18,
                    a: 1.0,
                },
                ColorMode::ColorScheme => {
                    if !self.state.species_colors.is_empty() {
                        // Background is appended at the end (index = species_count)
                        let bg_index = (self.settings.species_count as usize)
                            .min(self.state.species_colors.len() - 1);
                        let bg = self.state.species_colors[bg_index];
                        wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: bg[3] as f64,
                        }
                    } else {
                        wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }
                    }
                }
            };

            let mut trail_render_pass =
                compute_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Trail Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: write_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(background_color), // Clear with background color
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            // Always copy previous trail content (with or without fading)
            // Calculate fade amount: convert trace_fade (0-1) to subtraction amount per frame
            let fade_amount = if self.state.trace_fade < 1.0 {
                // Invert trace_fade so 0.0 = fast fade, 1.0 = no fade
                // Scale to reasonable subtraction range (0.001 to 0.1 per frame)
                let fade_strength = 1.0 - self.state.trace_fade;
                fade_strength * 0.1 // Maximum fade of 0.1 alpha per frame
            } else {
                0.0 // No fading
            };

            self.update_fade_uniforms(queue, fade_amount);

            // Apply fade effect - reads from previous texture, writes to current
            trail_render_pass.set_pipeline(&self.fade_pipeline);
            trail_render_pass.set_bind_group(0, &self.fade_bind_group, &[]);
            trail_render_pass.draw(0..3, 0..1);

            // Then render particles on top
            trail_render_pass.set_pipeline(&self.trail_render_pipeline);
            trail_render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            trail_render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            trail_render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

            let particle_count = self.state.particle_count as u32;
            trail_render_pass.draw(0..6, 0..particle_count);
            drop(trail_render_pass);
        } else {
            // When trails are disabled, render particles directly to display texture (preserving background)
            let mut particle_render_pass =
                compute_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Particle Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Preserve background
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            particle_render_pass.set_pipeline(&self.display_render_pipeline);
            particle_render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            particle_render_pass.set_bind_group(1, &self.lut_bind_group, &[]);
            particle_render_pass.set_bind_group(2, &self.camera_bind_group, &[]);

            let particle_count = self.state.particle_count as u32;
            particle_render_pass.draw(0..6, 0..particle_count);
        }

        // Submit the first encoder to ensure trail texture writes are complete
        queue.submit(std::iter::once(compute_encoder.finish()));

        // Second encoder: blit and final rendering (can now safely read trail texture)
        let mut render_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Life Render Encoder"),
        });

        // Blit trail texture to display texture if trails are enabled
        if self.state.traces_enabled {
            // Update blit bind group to read from the texture we just wrote to
            self.update_blit_bind_group(device);

            let mut display_render_pass =
                render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Display Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.display_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Load existing background
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            display_render_pass.set_pipeline(&self.display_blit_pipeline);
            display_render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
            display_render_pass.draw(0..3, 0..1);
        }

        // Step 3: Render post effects from display texture to post-effect texture (offscreen)
        // Skip expensive post-effect pass when using default parameters
        if self.needs_post_effects() {
            let mut post_effect_pass =
                render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Post Effect Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.post_effect_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            post_effect_pass.set_pipeline(&self.post_effect_pipeline);
            post_effect_pass.set_bind_group(0, &self.post_effect_bind_group, &[]);
            post_effect_pass.draw(0..6, 0..1);
        }

        // Step 4: Render texture to surface with infinite renderer
        // Use display texture directly when post-effects are disabled for better performance
        {
            let tile_count = Self::calculate_tile_count(self.camera.zoom);
            let total_instances = (tile_count * tile_count) as u32;

            let mut surface_render_pass =
                render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Surface Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            surface_render_pass.set_pipeline(&self.render_infinite_pipeline);
            surface_render_pass.set_bind_group(0, &self.render_infinite_display_bind_group, &[]);
            surface_render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            // Draw a fullscreen quad with multiple instances for tiling
            surface_render_pass.draw(0..6, 0..total_instances);
        }

        queue.submit(std::iter::once(render_encoder.finish()));

        // Swap ping-pong trail textures for next frame (only if trails are enabled)
        if self.state.traces_enabled {
            self.swap_trail_textures();
        }

        Ok(())
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        // Update base surface dimensions
        self.base_surface_width = new_config.width;
        self.base_surface_height = new_config.height;

        // Update resolution based on current zoom level
        self.update_resolution(device)?;

        // Update camera viewport
        self.camera
            .resize(new_config.width as f32, new_config.height as f32);

        // Update post-processing resources
        self.post_processing_resources.resize(device, new_config)?;

        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "species_count" => {
                if let Some(count) = value.as_u64() {
                    let old_count = self.settings.species_count;
                    self.settings.set_species_count(count as u32);

                    // Recreate force matrix buffer with new size
                    let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
                    self.force_matrix_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Force Matrix Buffer"),
                            contents: bytemuck::cast_slice(&force_matrix_data),
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        });

                    // Recreate bind groups that use this buffer
                    self.recreate_bind_groups_with_force_matrix(device);

                    // Update LUT colors for new species count
                    let current_lut_name = self.state.current_color_scheme.clone();
                    let color_scheme_reversed = self.state.color_scheme_reversed;
                    let color_scheme_manager = self.color_scheme_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &color_scheme_manager,
                        self.state.background_color_mode,
                        Some(&current_lut_name),
                        color_scheme_reversed,
                    )?;

                    // Respawn all particles to ensure proper species distribution
                    self.initialize_particles_gpu(device, queue)?;

                    tracing::info!(
                        "Updated species count from {} to {} (respawned all particles)",
                        old_count,
                        count
                    );
                }
            }
            "particle_count" => {
                if let Some(count) = value.as_u64() {
                    self.update_particle_count(count as u32, device, queue)?;
                }
            }
            "force_matrix" => {
                if let Some(matrix_array) = value.as_array() {
                    // Update CPU side for UI display
                    for (i, row) in matrix_array.iter().enumerate() {
                        if let Some(row_array) = row.as_array() {
                            for (j, val) in row_array.iter().enumerate() {
                                if let Some(force_val) = val.as_f64() {
                                    if i < self.settings.force_matrix.len()
                                        && j < self.settings.force_matrix[i].len()
                                    {
                                        self.settings.force_matrix[i][j] = force_val as f32;
                                    }
                                }
                            }
                        }
                    }
                    // Update entire LJ params buffer since we changed the force matrix
                    let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
                    queue.write_buffer(
                        &self.force_matrix_buffer,
                        0,
                        bytemuck::cast_slice(&force_matrix_data),
                    );
                }
            }
            "max_force" => {
                if let Some(force) = value.as_f64() {
                    self.settings.max_force = force as f32;
                }
            }
            "min_distance" => {
                if let Some(dist) = value.as_f64() {
                    self.settings.min_distance = dist as f32;
                }
            }
            "max_distance" => {
                if let Some(dist) = value.as_f64() {
                    self.settings.max_distance = dist as f32;
                }
            }
            "friction" => {
                if let Some(friction) = value.as_f64() {
                    self.settings.friction = friction as f32;
                }
            }
            "force_beta" => {
                if let Some(beta) = value.as_f64() {
                    self.settings.force_beta = beta as f32;
                }
            }
            "brownian_motion" => {
                if let Some(brownian) = value.as_f64() {
                    self.settings.brownian_motion = (brownian as f32).clamp(0.0, 1.0);
                }
            }
            "wrap_edges" => {
                if let Some(wrap) = value.as_bool() {
                    self.settings.wrap_edges = wrap;
                }
            }
            "dt" => {
                if let Some(dt) = value.as_f64() {
                    self.state.dt = dt as f32;
                }
            }
            "cursor_size" => {
                if let Some(size) = value.as_f64() {
                    self.state.cursor_size = size as f32;
                }
            }
            "cursor_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.cursor_strength = (strength as f32).clamp(0.0, 10.0);
                }
            }
            "traces_enabled" => {
                if let Some(enabled) = value.as_bool() {
                    self.state.traces_enabled = enabled;
                }
            }
            "trace_fade" => {
                if let Some(fade) = value.as_f64() {
                    self.state.trace_fade = fade as f32;
                }
            }
            "edge_fade_strength" => {
                if let Some(strength) = value.as_f64() {
                    self.state.edge_fade_strength = strength as f32;
                }
            }
            "random_seed" => {
                if let Some(seed) = value.as_u64() {
                    self.state.random_seed = seed as u32;
                }
            }
            "position_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Random" => PositionGenerator::Random,
                        "Center" => PositionGenerator::Center,
                        "UniformCircle" => PositionGenerator::UniformCircle,
                        "CenteredCircle" => PositionGenerator::CenteredCircle,
                        "Ring" => PositionGenerator::Ring,
                        "RainbowRing" => PositionGenerator::RainbowRing,
                        "ColorBattle" => PositionGenerator::ColorBattle,
                        "ColorWheel" => PositionGenerator::ColorWheel,
                        "Line" => PositionGenerator::Line,
                        "Spiral" => PositionGenerator::Spiral,
                        "RainbowSpiral" => PositionGenerator::RainbowSpiral,
                        _ => PositionGenerator::Random,
                    };
                    self.state.position_generator = generator;
                    // Regenerate particles with new position generator
                    self.initialize_particles_gpu(device, queue)?;
                }
            }
            "type_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Radial" => super::settings::TypeGenerator::Radial,
                        "Polar" => super::settings::TypeGenerator::Polar,
                        "StripesH" => super::settings::TypeGenerator::StripesH,
                        "StripesV" => super::settings::TypeGenerator::StripesV,
                        "Random" => super::settings::TypeGenerator::Random,
                        "LineH" => super::settings::TypeGenerator::LineH,
                        "LineV" => super::settings::TypeGenerator::LineV,
                        "Spiral" => super::settings::TypeGenerator::Spiral,
                        "Dithered" => super::settings::TypeGenerator::Dithered,
                        "WavyLineH" => super::settings::TypeGenerator::WavyLineH,
                        "WavyLineV" => super::settings::TypeGenerator::WavyLineV,
                        _ => super::settings::TypeGenerator::Random,
                    };
                    self.state.type_generator = generator;
                    // Regenerate particles with new type generator
                    self.initialize_particles_gpu(device, queue)?;
                }
            }
            "matrix_generator" => {
                if let Some(generator_str) = value.as_str() {
                    let generator = match generator_str {
                        "Random" => super::settings::MatrixGenerator::Random,
                        "Symmetry" => super::settings::MatrixGenerator::Symmetry,
                        "Chains" => super::settings::MatrixGenerator::Chains,
                        "Chains2" => super::settings::MatrixGenerator::Chains2,
                        "Chains3" => super::settings::MatrixGenerator::Chains3,
                        "Snakes" => super::settings::MatrixGenerator::Snakes,
                        "Zero" => super::settings::MatrixGenerator::Zero,
                        "PredatorPrey" => super::settings::MatrixGenerator::PredatorPrey,
                        "Symbiosis" => super::settings::MatrixGenerator::Symbiosis,
                        "Territorial" => super::settings::MatrixGenerator::Territorial,
                        "Magnetic" => super::settings::MatrixGenerator::Magnetic,
                        "Crystal" => super::settings::MatrixGenerator::Crystal,
                        "Wave" => super::settings::MatrixGenerator::Wave,
                        "Hierarchy" => super::settings::MatrixGenerator::Hierarchy,
                        "Clique" => super::settings::MatrixGenerator::Clique,
                        "AntiClique" => super::settings::MatrixGenerator::AntiClique,
                        "Fibonacci" => super::settings::MatrixGenerator::Fibonacci,
                        "Prime" => super::settings::MatrixGenerator::Prime,
                        "Fractal" => super::settings::MatrixGenerator::Fractal,
                        "RockPaperScissors" => super::settings::MatrixGenerator::RockPaperScissors,
                        "Cooperation" => super::settings::MatrixGenerator::Cooperation,
                        "Competition" => super::settings::MatrixGenerator::Competition,
                        _ => super::settings::MatrixGenerator::Random,
                    };
                    // Generate new force matrix before moving the generator
                    self.settings.randomize_force_matrix(&generator);
                    self.state.matrix_generator = generator;
                    self.recreate_bind_groups_with_force_matrix(device);
                    self.update_sim_params(device, queue);
                }
            }
            "background_color_mode" => {
                if let Some(mode_str) = value.as_str() {
                    let color_mode = match mode_str {
                        "Gray18" => ColorMode::Gray18,
                        "White" => ColorMode::White,
                        "Black" => ColorMode::Black,
                        "Color Scheme" => ColorMode::ColorScheme,
                        _ => ColorMode::ColorScheme,
                    };

                    // Update the state color mode
                    self.state.background_color_mode = color_mode;

                    // Update LUT with new color mode
                    let current_lut_name = self.state.current_color_scheme.clone();
                    let color_scheme_reversed = self.state.color_scheme_reversed;
                    let color_scheme_manager = self.color_scheme_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &color_scheme_manager,
                        color_mode,
                        Some(&current_lut_name),
                        color_scheme_reversed,
                    )?;

                    // Update background parameters for the new color mode
                    self.update_background_params(queue);
                }
            }
            "color_scheme" => {
                if let Some(color_scheme) = value.as_str() {
                    let color_mode = self.state.background_color_mode;
                    let color_scheme_reversed = self.state.color_scheme_reversed;
                    let color_scheme_manager = self.color_scheme_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &color_scheme_manager,
                        color_mode,
                        Some(color_scheme),
                        color_scheme_reversed,
                    )?;
                }
            }
            "color_scheme_reversed" => {
                if let Some(reversed) = value.as_bool() {
                    let color_mode = self.state.background_color_mode;
                    let current_lut_name = self.state.current_color_scheme.clone();
                    let color_scheme_manager = self.color_scheme_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &color_scheme_manager,
                        color_mode,
                        Some(&current_lut_name),
                        reversed,
                    )?;
                }
            }
            "particle_size" => {
                if let Some(size) = value.as_f64() {
                    self.state.particle_size = size as f32;
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
            "color_scheme" => {
                if let Some(color_scheme_name) = value.as_str() {
                    let color_mode = self.state.background_color_mode;
                    let color_scheme_reversed = self.state.color_scheme_reversed;
                    let color_scheme_manager = self.color_scheme_manager.clone();
                    self.update_lut(
                        device,
                        queue,
                        &color_scheme_manager,
                        color_mode,
                        Some(color_scheme_name),
                        color_scheme_reversed,
                    )?;
                }
            }
            _ => {
                tracing::warn!("Unknown state parameter for ParticleLife: {}", state_name);
            }
        }
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }

    fn get_state(&self) -> Value {
        serde_json::to_value(&self.state).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Determine cursor mode based on mouse_button
        let cursor_mode = if mouse_button == 0 {
            1 // left click = attract
        } else if mouse_button == 2 {
            2 // right click = repel
        } else {
            0 // middle click or other = no interaction
        };

        // Store coordinates directly - conversion is handled in the manager
        let sim_x = world_x;
        let sim_y = world_y;

        // Store cursor values in the model
        self.cursor_active_mode = cursor_mode;
        self.cursor_world_x = sim_x;
        self.cursor_world_y = sim_y;

        tracing::trace!(
            world_x = sim_x,
            world_y = sim_y,
            cursor_mode = cursor_mode,
            "Mouse interaction updated"
        );

        // Update sim params immediately with new cursor values
        let mut sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );

        // Override with cursor values
        sim_params.cursor_x = sim_x;
        sim_params.cursor_y = sim_y;
        sim_params.cursor_active = cursor_mode;
        if cursor_mode > 0 {
            sim_params.cursor_strength =
                self.state.cursor_strength * self.settings.max_force * 10.0;
        }

        // Upload to GPU immediately
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
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Turn off cursor interaction
        self.cursor_active_mode = 0;
        self.cursor_world_x = 0.0;
        self.cursor_world_y = 0.0;

        // Update sim params immediately with cursor disabled
        let mut sim_params = SimParams::new(
            self.width,
            self.height,
            self.state.particle_count as u32,
            &self.settings,
            &self.state,
        );

        // Reset cursor sim params to default values
        sim_params.cursor_x = 0.0;
        sim_params.cursor_y = 0.0;
        sim_params.cursor_active = 0;
        sim_params.cursor_strength = 0.0;

        // Upload to GPU immediately
        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

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
        self.camera.get_state()
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        // TODO: Implement preset saving
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // TODO: Implement preset loading
        Ok(())
    }

    fn apply_settings(
        &mut self,
        settings: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Deserialize the settings and apply them using update_setting for each field
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            // Apply each setting individually to ensure proper handling
            if let Ok(species_count) = serde_json::to_value(new_settings.species_count) {
                self.update_setting("species_count", species_count, device, queue)?;
            }
            if let Ok(force_matrix) = serde_json::to_value(new_settings.force_matrix) {
                self.update_setting("force_matrix", force_matrix, device, queue)?;
            }
            if let Ok(max_force) = serde_json::to_value(new_settings.max_force) {
                self.update_setting("max_force", max_force, device, queue)?;
            }
            if let Ok(min_distance) = serde_json::to_value(new_settings.min_distance) {
                self.update_setting("min_distance", min_distance, device, queue)?;
            }
            if let Ok(max_distance) = serde_json::to_value(new_settings.max_distance) {
                self.update_setting("max_distance", max_distance, device, queue)?;
            }
            if let Ok(friction) = serde_json::to_value(new_settings.friction) {
                self.update_setting("friction", friction, device, queue)?;
            }
            if let Ok(force_beta) = serde_json::to_value(new_settings.force_beta) {
                self.update_setting("force_beta", force_beta, device, queue)?;
            }
            if let Ok(brownian_motion) = serde_json::to_value(new_settings.brownian_motion) {
                self.update_setting("brownian_motion", brownian_motion, device, queue)?;
            }
            if let Ok(wrap_edges) = serde_json::to_value(new_settings.wrap_edges) {
                self.update_setting("wrap_edges", wrap_edges, device, queue)?;
            }
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Update random seed for reset
        use rand::Rng;
        let mut rng = rand::rng();
        self.state.random_seed = rng.random();

        // Update sim params with new random seed
        self.update_sim_params(device, queue);

        // Re-initialize particles on GPU with new random seed
        self.initialize_particles_gpu(device, queue)?;

        // Ensure GPU operations complete
        device
            .poll(wgpu::wgt::PollType::Wait)
            .expect("Failed to poll device");

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
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Store the current matrix generator to avoid borrowing issues
        let matrix_generator = self.state.matrix_generator;

        // Generate new force matrix using the current matrix generator
        self.settings.randomize_force_matrix(&matrix_generator);

        // Update the force matrix buffer on GPU
        let force_matrix_data = Self::flatten_force_matrix(&self.settings.force_matrix);
        queue.write_buffer(
            &self.force_matrix_buffer,
            0,
            bytemuck::cast_slice(&force_matrix_data),
        );

        // Update random seed for consistency
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.state.random_seed as u64);
        self.state.random_seed = rng.random();

        // Update sim params with new random seed
        self.update_sim_params(device, queue);

        // Note: Physics settings (max_force, distances, friction, wrap_edges)
        // are intentionally NOT randomized to preserve user's simulation setup
        // Note: particle_count and species_count are preserved

        Ok(())
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Apply provided color scheme data directly (used by Gradient Editor preview)
        // Respect current reversed flag but preserve original name in state
        let effective_lut = if self.state.color_scheme_reversed {
            color_scheme.reversed()
        } else {
            color_scheme.clone()
        };

        let species_count = self.settings.species_count as usize;
        let mut species_colors: Vec<[f32; 4]> = Vec::with_capacity(species_count + 1);

        if self.state.background_color_mode == ColorMode::ColorScheme {
            // Sample background + species; first is background
            let raw_colors = effective_lut
                .get_colors(species_count + 1)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();

            for color in raw_colors.iter().skip(1).take(species_count) {
                species_colors.push(*color);
            }
            species_colors.push(raw_colors[0]);
        } else {
            // Non-LUT display modes still sample species_count colors for species only
            species_colors = effective_lut
                .get_colors(species_count)
                .into_iter()
                .map(|v| [v[0], v[1], v[2], v[3]])
                .collect::<Vec<_>>();
        }

        // Update state (preserve original name regardless of reversal)
        self.state.species_colors = species_colors;
        self.state.current_color_scheme = color_scheme.name.clone();

        // Upload to GPU and refresh background color
        self.update_species_colors_gpu(device, queue)?;
        self.update_background_params(queue);

        Ok(())
    }
}

impl ParticleLifeModel {
    /// Update particle count by recreating buffer and respawning all particles
    pub fn update_particle_count(
        &mut self,
        new_count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        let new_count = new_count.clamp(1000, 100000);
        let old_count = self.state.particle_count as u32;

        if new_count == old_count {
            return Ok(());
        }

        // Update state
        self.state.particle_count = new_count as usize;

        // Check buffer size limits
        let max_storage_buffer_size = device.limits().max_storage_buffer_binding_size as u64;
        let new_particle_buffer_size =
            (new_count as usize * std::mem::size_of::<Particle>()) as u64;

        if new_particle_buffer_size > max_storage_buffer_size {
            return Err(SimulationError::BufferTooLarge {
                requested: new_particle_buffer_size,
                max_available: max_storage_buffer_size,
            });
        }

        // Create new particle buffer with new size
        let new_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: new_particle_buffer_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Replace the buffer
        self.particle_buffer = new_particle_buffer;

        // Recreate bind groups with new buffer
        self.recreate_bind_groups(device)?;

        // Update simulation parameters with new particle count BEFORE initializing particles
        self.update_sim_params(device, queue);

        // Respawn all particles with new count
        self.initialize_particles_gpu(device, queue)?;

        // Force GPU to finish all commands to ensure buffer updates are complete
        device
            .poll(wgpu::wgt::PollType::Wait)
            .expect("Failed to poll device");

        Ok(())
    }

    /// Recreate bind groups after particle buffer changes
    fn recreate_bind_groups(&mut self, device: &Arc<Device>) -> SimulationResult<()> {
        tracing::info!("Recreating compute bind group");
        // Recreate compute bind group
        self.compute_bind_group = resource_helpers::create_buffer_bind_group(
            device,
            &self.compute_pipeline.get_bind_group_layout(0),
            "Particle Life Compute Bind Group",
            &[
                &self.particle_buffer,
                &self.sim_params_buffer,
                &self.force_matrix_buffer,
            ],
        );

        tracing::info!("Recreating render bind group");
        // Recreate render bind group
        self.render_bind_group = resource_helpers::create_buffer_bind_group(
            device,
            &self.render_particles_bind_group_layout,
            "Particle Life Render Bind Group",
            &[&self.particle_buffer, &self.sim_params_buffer],
        );

        tracing::info!("Recreating init bind group");
        // Recreate init bind group (critical for particle initialization)
        self.init_bind_group = resource_helpers::create_buffer_bind_group(
            device,
            &self.init_bind_group_layout,
            "Particle Life Init Bind Group",
            &[&self.particle_buffer, &self.init_params_buffer],
        );

        tracing::info!("All bind groups recreated successfully");
        // Recreate LUT bind group to ensure it points to the current species_colors_buffer
        self.lut_bind_group = resource_helpers::create_buffer_bind_group(
            device,
            &self.render_bind_group_layout,
            "Species Colors Bind Group",
            &[&self.species_colors_buffer, &self.color_mode_buffer],
        );
        Ok(())
    }
}
