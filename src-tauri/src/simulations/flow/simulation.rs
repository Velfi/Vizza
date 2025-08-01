use super::settings::{NoiseType, Settings};
use super::shaders::{
    BACKGROUND_RENDER_SHADER, PARTICLE_RENDER_SHADER, PARTICLE_UPDATE_SHADER,
    RENDER_INFINITE_SHADER, TRAIL_DECAY_DIFFUSION_SHADER, TRAIL_RENDER_SHADER,
};
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::{AverageColorResources, LutManager, PostProcessingState, PostProcessingResources};
use crate::simulations::traits::Simulation;
use bytemuck::{Pod, Zeroable};
use noise::{
    Billow, Checkerboard, Cylinders, Fbm, MultiFractal, NoiseFn, OpenSimplex, Perlin, RidgedMulti, Value as ValueNoise, Worley,
};
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

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Particle {
    pub position: [f32; 2],
    pub age: f32,
    pub color: [f32; 4],
    pub my_parent_was: u32, // 0 = autospawned, 1 = spawned by brush
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct FlowVector {
    pub position: [f32; 2],
    pub direction: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimParams {
    pub particle_limit: u32, // Kept for backward compatibility, no longer used for limiting
    pub autospawn_limit: u32, // New setting for limiting autospawned particles
    pub vector_count: u32,
    pub particle_lifetime: f32,
    pub particle_speed: f32,
    pub noise_seed: u32,
    pub time: f32,
    pub width: f32,
    pub height: f32,
    pub noise_scale: f32,
    pub noise_x: f32,
    pub noise_y: f32,
    pub vector_magnitude: f32,
    pub trail_decay_rate: f32,
    pub trail_deposition_rate: f32,
    pub trail_diffusion_rate: f32,
    pub trail_wash_out_rate: f32,
    pub trail_map_width: u32,
    pub trail_map_height: u32,
    pub particle_shape: u32, // 0=Circle, 1=Square, 2=Triangle, 3=Star, 4=Diamond
    pub particle_size: u32,  // Particle size in pixels
    pub background_type: u32, // 0=Black, 1=White, 2=Vector Field
    pub screen_width: u32,   // Screen width in pixels
    pub screen_height: u32,  // Screen height in pixels
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_active: u32, // 0=inactive, 1=attract, 2=repel
    pub cursor_size: u32,
    pub cursor_strength: f32,
    pub particle_autospawn: u32,  // 0=disabled, 1=enabled
    pub particle_spawn_rate: f32, // 0.0 = no spawn, 1.0 = full spawn rate
    pub display_mode: u32,        // 0=Age, 1=Random, 2=Direction
}

#[derive(Debug)]
pub struct FlowModel {
    pub settings: Settings,

    // GPU resources
    pub particle_buffer: wgpu::Buffer,
    pub flow_vector_buffer: wgpu::Buffer,
    pub sim_params_buffer: wgpu::Buffer,
    pub lut_buffer: wgpu::Buffer,
    pub background_color_buffer: wgpu::Buffer,

    // Trail system
    pub trail_texture: wgpu::Texture,
    pub trail_texture_view: wgpu::TextureView,
    pub trail_sampler: wgpu::Sampler,

    // Particle update pipeline
    pub particle_update_pipeline: wgpu::ComputePipeline,
    pub particle_update_pipeline_layout: wgpu::PipelineLayout,
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
    pub lut_manager: Arc<LutManager>,
    pub time: f32,
    pub particles: Vec<Particle>,
    pub flow_vectors: Vec<FlowVector>,
    pub gui_visible: bool,
    pub trail_map_width: u32,
    pub trail_map_height: u32,

    // Mouse interaction state
    pub cursor_active_mode: u32, // 0 = inactive, 1 = attract, 2 = repel
    pub cursor_world_x: f32,
    pub cursor_world_y: f32,
    pub cursor_size: u32,
    pub cursor_strength: f32,

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

    // Post-processing state and resources
    pub post_processing_state: PostProcessingState,
    pub post_processing_resources: PostProcessingResources,
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
    fn generate_flow_direction(
        pos: [f32; 2],
        noise_type: NoiseType,
        noise_scale: f64,
        noise_x: f64,
        noise_y: f64,
        noise_seed: u32,
    ) -> [f32; 2] {
        let sample_pos = [pos[0] as f64 * noise_scale + noise_x, pos[1] as f64 * noise_scale + noise_y];

        let noise_value = match noise_type {
            NoiseType::OpenSimplex => {
                let opensimplex = OpenSimplex::new(noise_seed);
                opensimplex.get(sample_pos)
            }
            NoiseType::Worley => {
                let worley = Worley::new(noise_seed);
                worley.get(sample_pos)
            }
            NoiseType::Value => {
                let value = ValueNoise::new(noise_seed);
                value.get(sample_pos)
            }
            NoiseType::Fbm => {
                let fbm = Fbm::<Perlin>::new(noise_seed)
                    .set_octaves(6)
                    .set_frequency(noise_scale)
                    .set_lacunarity(2.0)
                    .set_persistence(0.5);
                fbm.get(sample_pos)
            }
            NoiseType::FBMBillow => {
                let fbm = Fbm::<Perlin>::new(noise_seed)
                    .set_octaves(8)
                    .set_frequency(noise_scale)
                    .set_lacunarity(2.5)
                    .set_persistence(0.7);
                fbm.get(sample_pos)
            }
            NoiseType::FBMClouds => {
                let fbm = Fbm::<Perlin>::new(noise_seed)
                    .set_octaves(4)
                    .set_frequency(noise_scale)
                    .set_lacunarity(1.8)
                    .set_persistence(0.3);
                fbm.get(sample_pos)
            }
            NoiseType::FBMRidged => {
                let fbm = Fbm::<Perlin>::new(noise_seed)
                    .set_octaves(10)
                    .set_frequency(noise_scale)
                    .set_lacunarity(3.0)
                    .set_persistence(0.9);
                fbm.get(sample_pos)
            }
            NoiseType::Billow => {
                let billow = Billow::<Perlin>::new(noise_seed)
                    .set_octaves(6)
                    .set_frequency(noise_scale)
                    .set_lacunarity(2.0)
                    .set_persistence(0.5);
                billow.get(sample_pos)
            }
            NoiseType::RidgedMulti => {
                let ridged = RidgedMulti::<Perlin>::new(noise_seed)
                    .set_octaves(6)
                    .set_frequency(noise_scale)
                    .set_lacunarity(2.0);
                ridged.get(sample_pos)
            }
            NoiseType::Cylinders => {
                let cylinders = Cylinders::new();
                cylinders.get(sample_pos)
            }
            NoiseType::Checkerboard => {
                let checkerboard = Checkerboard::new(16);
                checkerboard.get(sample_pos)
            }
        };

        // Create flow direction from noise value
        let angle = noise_value * std::f64::consts::TAU;
        [angle.cos() as f32, angle.sin() as f32]
    }

    // Regenerate flow vectors with current settings
    fn regenerate_flow_vectors(
        &mut self,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        let grid_size = 128; // 128x128 grid of vectors
        let mut flow_vectors = Vec::with_capacity(grid_size * grid_size);

        for y in 0..grid_size {
            for x in 0..grid_size {
                let world_x = (x as f32 / (grid_size - 1) as f32) * 2.0 - 1.0;
                let world_y = (y as f32 / (grid_size - 1) as f32) * 2.0 - 1.0;

                // Use noise crate for flow direction
                let direction = Self::generate_flow_direction(
                    [world_x, world_y],
                    self.settings.noise_type,
                    self.settings.noise_scale,
                    self.settings.noise_x,
                    self.settings.noise_y,
                    self.settings.noise_seed,
                );
                let direction = [
                    direction[0] * self.settings.vector_magnitude,
                    direction[1] * self.settings.vector_magnitude,
                ];

                flow_vectors.push(FlowVector {
                    position: [world_x, world_y],
                    direction,
                });
            }
        }

        queue.write_buffer(
            &self.flow_vector_buffer,
            0,
            bytemuck::cast_slice(&flow_vectors),
        );
        self.flow_vectors = flow_vectors;

        // Update sim params with new vector count
        let sim_params = SimParams {
            particle_limit: self.settings.particle_limit,
            autospawn_limit: self.settings.autospawn_limit,
            vector_count: self.flow_vectors.len() as u32,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
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
            background_type: self.settings.background as u32,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_active: self.cursor_active_mode,
            cursor_size: self.cursor_size,
            cursor_strength: self.cursor_strength,
            particle_autospawn: self.settings.particle_autospawn as u32,
            particle_spawn_rate: self.settings.particle_spawn_rate,
            display_mode: self.settings.display_mode as u32,
        };

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        Ok(())
    }

    pub fn new(
        device: &Arc<Device>,
        _queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> Result<Self, crate::error::SimulationError> {
        // Initialize camera
        let camera = Camera::new(
            device,
            surface_config.width as f32,
            surface_config.height as f32,
        )?;

        // Initialize particles with random positions and ages
        // Use a much larger buffer for unlimited particles (1 million particles)
        let max_particles = 1_000_000;
        let mut particles = Vec::with_capacity(max_particles);

        // No longer need LUT data since we use LUT in shader
        RNG.with(|rng| {
            let mut rng = rng.borrow_mut();
            // Initialize with autospawn_limit particles
            for _ in 0..settings.autospawn_limit {
                let x = rng.random_range(-1.0..1.0);
                let y = rng.random_range(-1.0..1.0);
                let age = rng.random_range(0.0..settings.particle_lifetime * 0.1); // Start with 10% of lifetime max

                // No longer need to generate colors here since we use LUT in shader
                let color = [0.0, 0.0, 0.0, 0.9]; // Placeholder color, will be overridden by LUT

                let particle = Particle {
                    position: [x, y],
                    age,
                    color,
                    my_parent_was: 0, // Autospawned
                };
                particles.push(particle);
            }
        });

        // Initialize flow vectors with simple grid
        let grid_size = 128; // 128x128 grid of vectors
        let mut flow_vectors = Vec::with_capacity(grid_size * grid_size);

        for y in 0..grid_size {
            for x in 0..grid_size {
                let world_x = (x as f32 / (grid_size - 1) as f32) * 2.0 - 1.0;
                let world_y = (y as f32 / (grid_size - 1) as f32) * 2.0 - 1.0;

                // Use noise crate for flow direction
                let direction = Self::generate_flow_direction(
                    [world_x, world_y],
                    settings.noise_type,
                    settings.noise_scale,
                    settings.noise_x,
                    settings.noise_y,
                    settings.noise_seed,
                );
                let direction = [
                    direction[0] * settings.vector_magnitude,
                    direction[1] * settings.vector_magnitude,
                ];

                flow_vectors.push(FlowVector {
                    position: [world_x, world_y],
                    direction,
                });
            }
        }

        // Create GPU buffers
        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let flow_vector_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flow Vector Buffer"),
            contents: bytemuck::cast_slice(&flow_vectors),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let sim_params = SimParams {
            particle_limit: settings.particle_limit,
            autospawn_limit: settings.autospawn_limit,
            vector_count: flow_vectors.len() as u32,
            particle_lifetime: settings.particle_lifetime,
            particle_speed: settings.particle_speed,
            noise_seed: settings.noise_seed,
            time: 0.0,
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
            background_type: settings.background as u32,
            screen_width: surface_config.width,
            screen_height: surface_config.height,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_active: 0,
            cursor_size: 10,
            cursor_strength: 0.4,
            particle_autospawn: settings.particle_autospawn as u32,
            particle_spawn_rate: settings.particle_spawn_rate,
            display_mode: settings.display_mode as u32,
        };

        let sim_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sim Params Buffer"),
            contents: bytemuck::cast_slice(&[sim_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lut_data = lut_manager
            .get(&settings.current_lut)
            .unwrap_or_else(|_| lut_manager.get_default());
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data.to_u32_buffer()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create background color buffer (black by default)
        let background_color_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Background Color Buffer"),
                contents: bytemuck::cast_slice(&[0.0f32, 0.0f32, 0.0f32, 1.0f32]), // Black background
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create trail texture
        let trail_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Trail Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let trail_texture_view = trail_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Trail Texture View"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });

        let trail_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Trail Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create particle update pipeline
        let particle_update_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Particle Update Shader"),
            source: wgpu::ShaderSource::Wgsl(PARTICLE_UPDATE_SHADER.into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let particle_update_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Update Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let particle_update_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Flow Particle Update Pipeline"),
                layout: Some(&particle_update_pipeline_layout),
                module: &particle_update_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        let particle_update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Update Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: flow_vector_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&trail_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

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
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
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
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&trail_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: flow_vector_buffer.as_entire_binding(),
                    },
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
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

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
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
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
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
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&trail_texture_view),
                },
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: flow_vector_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lut_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_params_buffer.as_entire_binding(),
                },
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
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear, // Enable mipmap filtering
            ..Default::default()
        });

        // Create infinite render pipeline for truly infinite tiling
        let render_infinite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flow Render Infinite Shader"),
            source: wgpu::ShaderSource::Wgsl(RENDER_INFINITE_SHADER.into()),
        });

        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Flow Render Infinite Bind Group Layout"),
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

        let render_infinite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Flow Render Infinite Pipeline Layout"),
                bind_group_layouts: &[
                    &render_infinite_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_infinite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flow Render Infinite Bind Group"),
            layout: &render_infinite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&display_mipmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&display_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: background_color_buffer.as_entire_binding(),
                },
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

        // Use the same camera for both offscreen and infinite rendering

        Ok(Self {
            settings,
            particle_buffer,
            flow_vector_buffer,
            sim_params_buffer,
            lut_buffer,
            background_color_buffer,
            trail_texture,
            trail_texture_view,
            trail_sampler,
            particle_update_pipeline,
            particle_update_pipeline_layout,
            particle_update_bind_group,
            trail_decay_diffusion_pipeline,
            trail_decay_diffusion_bind_group,
            particle_render_pipeline,
            particle_render_bind_group,
            camera_bind_group,

            camera,
            lut_manager: Arc::new(lut_manager.clone()),
            time: 0.0,
            particles,
            flow_vectors,
            gui_visible: true,
            trail_map_width: surface_config.width,
            trail_map_height: surface_config.height,
            trail_render_pipeline,
            trail_display_render_pipeline,
            trail_render_bind_group,
            background_render_pipeline,
            background_display_render_pipeline,
            background_render_bind_group,
            particle_display_render_pipeline,

            // Initialize mouse interaction state
            cursor_active_mode: 0, // Inactive
            cursor_world_x: 0.0,
            cursor_world_y: 0.0,
            cursor_size: 10,
            cursor_strength: 0.4,

            display_texture,
            display_view,
            display_mipmap_texture,
            display_mipmap_view,
            display_sampler,
            render_infinite_bind_group,
            render_infinite_pipeline,
            average_color_resources,
            post_processing_state: PostProcessingState::default(),
            post_processing_resources: PostProcessingResources::new(device, surface_config)?,
        })
    }

    fn write_sim_params(&self, queue: &Arc<wgpu::Queue>) {
        let sim_params = SimParams {
            particle_limit: self.settings.particle_limit,
            autospawn_limit: self.settings.autospawn_limit,
            vector_count: self.flow_vectors.len() as u32,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
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
            background_type: self.settings.background as u32,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_active: self.cursor_active_mode,
            cursor_size: self.cursor_size,
            cursor_strength: self.cursor_strength,
            particle_autospawn: self.settings.particle_autospawn as u32,
            particle_spawn_rate: self.settings.particle_spawn_rate,
            display_mode: self.settings.display_mode as u32,
        };

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );
    }

    fn calculate_average_color(&self, device: &Arc<Device>, queue: &Arc<Queue>) {
        self.average_color_resources
            .calculate_average_color(device, queue, &self.display_texture);

        // Wait for the GPU work to complete
        device.poll(wgpu::Maintain::Wait);

        // Read the result and update the background color buffer
        if let Some(average_color) = self.average_color_resources.get_average_color() {
            queue.write_buffer(
                &self.background_color_buffer,
                0,
                bytemuck::cast_slice(&[average_color]),
            );
        }
        // Unmap the staging buffer after reading
        self.average_color_resources.unmap_staging_buffer();
    }

    fn update_background_color(&self, queue: &Arc<wgpu::Queue>) {
        // This method is now only used for initial setup
        // The actual average color is calculated in calculate_average_color
        let lut = self
            .lut_manager
            .get(&self.settings.current_lut)
            .unwrap_or_else(|_| self.lut_manager.get_default());

        let colors = lut.get_colors(1); // Get just the first color
        if let Some(color) = colors.first() {
            let background_color = [
                color[0],
                color[1],
                color[2],
                color[3],
            ];
            queue.write_buffer(
                &self.background_color_buffer,
                0,
                bytemuck::cast_slice(&background_color),
            );
        }
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
                layout: &self.post_processing_resources.blur_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(input_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.post_processing_resources.blur_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.post_processing_resources.blur_params_buffer.as_entire_binding(),
                    },
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
    ) -> crate::error::SimulationResult<()> {
        // Update simulation time
        self.time += 0.016; // ~60 FPS

        // Update simulation parameters using the centralized method
        self.write_sim_params(queue);

        // Update background color from LUT
        self.update_background_color(queue);

        // Update camera and upload to GPU
        self.camera.update(0.016);
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
            // Use max_particles for compute dispatch (1 million particles)
            let active_particles = self.particles.len() as u32;
            compute_pass.dispatch_workgroups(active_particles.div_ceil(64), 1, 1);
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
            if self.settings.show_particles {
                render_pass.set_pipeline(&self.particle_render_pipeline);
                render_pass.set_bind_group(0, &self.particle_render_bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                let active_particles = self.particles.len() as u32;
                render_pass.draw(0..6, 0..active_particles); // Single instance per particle
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

    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> crate::error::SimulationResult<()> {
        // Update camera and upload to GPU (same as normal render)
        self.camera.update(0.016);
        self.camera.upload_to_gpu(queue);

        // Update background color from LUT
        self.update_background_color(queue);

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
            if self.settings.show_particles {
                render_pass.set_pipeline(&self.particle_render_pipeline);
                render_pass.set_bind_group(0, &self.particle_render_bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                let active_particles = self.particles.len() as u32;
                render_pass.draw(0..6, 0..active_particles); // Single instance per particle
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
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.flow_vector_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.trail_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.lut_buffer.as_entire_binding(),
                },
            ],
        });

        self.trail_decay_diffusion_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Trail Decay Diffusion Bind Group"),
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Trail Update Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::ReadWrite,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
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
                    ],
                }),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.sim_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.trail_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.flow_vector_buffer.as_entire_binding(),
                    },
                ],
            });

        self.trail_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Trail Render Bind Group"),
            layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Trail Render Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            }),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.trail_texture_view),
                },
            ],
        });

        // Update sim params with new dimensions
        let sim_params = SimParams {
            particle_limit: self.settings.particle_limit,
            autospawn_limit: self.settings.autospawn_limit,
            vector_count: self.flow_vectors.len() as u32,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
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
            background_type: self.settings.background as u32,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_active: self.cursor_active_mode,
            cursor_size: self.cursor_size,
            cursor_strength: self.cursor_strength,
            particle_autospawn: self.settings.particle_autospawn as u32,
            particle_spawn_rate: self.settings.particle_spawn_rate,
            display_mode: self.settings.display_mode as u32,
        };

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        // Recreate camera bind group with updated camera buffer
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        self.camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera.buffer().as_entire_binding(),
            }],
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
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let render_infinite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Flow Render Infinite Bind Group Layout"),
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
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.display_mipmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.display_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.background_color_buffer.as_entire_binding(),
                },
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
            "noiseType" => {
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
                    self.regenerate_flow_vectors(queue)?;
                }
            }
            "noiseSeed" => {
                if let Some(seed) = value.as_u64() {
                    self.settings.noise_seed = seed as u32;
                    // Regenerate flow vectors with new seed
                    self.regenerate_flow_vectors(queue)?;
                }
            }
            "noiseScale" => {
                if let Some(scale) = value.as_f64() {
                    self.settings.noise_scale = scale;
                    // Regenerate flow vectors with new scale
                    self.regenerate_flow_vectors(queue)?;
                }
            }
            "noiseX" => {
                if let Some(x) = value.as_f64() {
                    self.settings.noise_x = x;
                    // Regenerate flow vectors with new X scale
                    self.regenerate_flow_vectors(queue)?;
                }
            }
            "noiseY" => {
                if let Some(y) = value.as_f64() {
                    self.settings.noise_y = y;
                    // Regenerate flow vectors with new Y scale
                    self.regenerate_flow_vectors(queue)?;
                }
            }
            "vectorSpacing" => {
                if let Some(spacing) = value.as_f64() {
                    self.settings.vector_spacing = spacing as f32;
                }
            }
            "vectorMagnitude" => {
                if let Some(magnitude) = value.as_f64() {
                    self.settings.vector_magnitude = magnitude as f32;
                    // Regenerate flow vectors with new magnitude
                    self.regenerate_flow_vectors(queue)?;
                }
            }
            "particleLimit" | "particleCount" => {
                // particle_limit is kept for backward compatibility but no longer used for limiting
                if let Some(count) = value.as_u64() {
                    self.settings.particle_limit = count as u32;
                }
            }
            "autospawnLimit" => {
                if let Some(count) = value.as_u64() {
                    let new_count = count as u32;
                    if new_count != self.settings.autospawn_limit {
                        self.settings.autospawn_limit = new_count;

                        // Update sim params with new autospawn limit
                        let sim_params = SimParams {
                            particle_limit: self.settings.particle_limit,
                            autospawn_limit: self.settings.autospawn_limit,
                            vector_count: self.flow_vectors.len() as u32,
                            particle_lifetime: self.settings.particle_lifetime,
                            particle_speed: self.settings.particle_speed,
                            noise_seed: self.settings.noise_seed,
                            time: self.time,
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
                            background_type: self.settings.background as u32,
                            screen_width: self.trail_map_width,
                            screen_height: self.trail_map_height,
                            cursor_x: self.cursor_world_x,
                            cursor_y: self.cursor_world_y,
                            cursor_active: self.cursor_active_mode,
                            cursor_size: self.cursor_size,
                            cursor_strength: self.cursor_strength,
                            particle_autospawn: self.settings.particle_autospawn as u32,
                            particle_spawn_rate: self.settings.particle_spawn_rate,
                            display_mode: self.settings.display_mode as u32,
                        };

                        queue.write_buffer(
                            &self.sim_params_buffer,
                            0,
                            bytemuck::cast_slice(&[sim_params]),
                        );
                    }
                }
            }
            "particleLifetime" => {
                if let Some(lifetime) = value.as_f64() {
                    self.settings.particle_lifetime = lifetime as f32;
                }
            }
            "particleSpeed" => {
                if let Some(speed) = value.as_f64() {
                    self.settings.particle_speed = speed as f32;
                }
            }
            "particleSize" => {
                if let Some(size) = value.as_u64() {
                    self.settings.particle_size = size as u32;
                }
            }
            "background" => {
                if let Some(background_str) = value.as_str() {
                    self.settings.background = match background_str {
                        "Black" => super::settings::Background::Black,
                        "White" => super::settings::Background::White,
                        "Vector Field" => super::settings::Background::Vectors,
                        _ => super::settings::Background::Vectors,
                    };
                }
            }
            "currentLut" => {
                if let Some(lut_name) = value.as_str() {
                    self.settings.current_lut = lut_name.to_string();
                    let mut lut_data = self
                        .lut_manager
                        .get(&self.settings.current_lut)
                        .unwrap_or_else(|_| self.lut_manager.get_default());

                    // Apply reversal if needed
                    if self.settings.lut_reversed {
                        lut_data = lut_data.reversed();
                    }

                    queue.write_buffer(
                        &self.lut_buffer,
                        0,
                        bytemuck::cast_slice(&lut_data.to_u32_buffer()),
                    );

                    // Recreate the compute pipeline to ensure compatibility with the bind group layout
                    let particle_update_shader =
                        device.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some("Flow Particle Update Shader"),
                            source: wgpu::ShaderSource::Wgsl(PARTICLE_UPDATE_SHADER.into()),
                        });

                    self.particle_update_pipeline =
                        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("Flow Particle Update Pipeline"),
                            layout: Some(&self.particle_update_pipeline_layout),
                            module: &particle_update_shader,
                            entry_point: Some("main"),
                            compilation_options: Default::default(),
                            cache: None,
                        });
                }
            }
            "lutReversed" => {
                if let Some(reversed) = value.as_bool() {
                    self.settings.lut_reversed = reversed;

                    // Reload the current LUT with new reversal state
                    let mut lut_data = self
                        .lut_manager
                        .get(&self.settings.current_lut)
                        .unwrap_or_else(|_| self.lut_manager.get_default());

                    // Apply reversal if needed
                    if self.settings.lut_reversed {
                        lut_data = lut_data.reversed();
                    }

                    queue.write_buffer(
                        &self.lut_buffer,
                        0,
                        bytemuck::cast_slice(&lut_data.to_u32_buffer()),
                    );

                    // Recreate the compute pipeline to ensure compatibility with the bind group layout
                    let particle_update_shader =
                        device.create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some("Flow Particle Update Shader"),
                            source: wgpu::ShaderSource::Wgsl(PARTICLE_UPDATE_SHADER.into()),
                        });

                    self.particle_update_pipeline =
                        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("Flow Particle Update Pipeline"),
                            layout: Some(&self.particle_update_pipeline_layout),
                            module: &particle_update_shader,
                            entry_point: Some("main"),
                            compilation_options: Default::default(),
                            cache: None,
                        });
                }
            }

            "trailDecayRate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_decay_rate = rate as f32;
                }
            }
            "trailDepositionRate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_deposition_rate = rate as f32;
                }
            }
            "trailDiffusionRate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_diffusion_rate = rate as f32;
                }
            }
            "trailWashOutRate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.trail_wash_out_rate = rate as f32;
                }
            }
            "particleShape" => {
                if let Some(shape_str) = value.as_str() {
                    self.settings.particle_shape = match shape_str {
                        "Circle" => super::settings::ParticleShape::Circle,
                        "Square" => super::settings::ParticleShape::Square,
                        "Triangle" => super::settings::ParticleShape::Triangle,
                        "Flower" => super::settings::ParticleShape::Star,
                        "Diamond" => super::settings::ParticleShape::Diamond,
                        _ => super::settings::ParticleShape::Circle,
                    };
                }
            }
            "cursorSize" => {
                if let Some(size) = value.as_f64() {
                    self.cursor_size = (size as u32).clamp(10, 500); // Clamp to reasonable range
                }
            }
            "cursorStrength" => {
                if let Some(strength) = value.as_f64() {
                    self.cursor_strength = (strength as f32).clamp(0.0, 1.0); // Clamp to 0.0-1.0 range
                }
            }
            "particleAutospawn" => {
                if let Some(autospawn) = value.as_bool() {
                    self.settings.particle_autospawn = autospawn;
                }
            }
            "particleSpawnRate" => {
                if let Some(rate) = value.as_f64() {
                    self.settings.particle_spawn_rate = (rate as f32).clamp(0.0, 1.0);
                }
            }
            "showParticles" => {
                if let Some(show) = value.as_bool() {
                    self.settings.show_particles = show;
                }
            }
            "displayMode" => {
                if let Some(mode_str) = value.as_str() {
                    self.settings.display_mode = match mode_str {
                        "Age" => super::settings::DisplayMode::Age,
                        "Random" => super::settings::DisplayMode::Random,
                        "Direction" => super::settings::DisplayMode::Direction,
                        _ => super::settings::DisplayMode::Age,
                    };
                }
            }
            _ => {}
        }

        // After handling the specific setting, always update the GPU uniform so changes take effect immediately
        self.write_sim_params(queue);

        Ok(())
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&self.settings).unwrap_or_default()
    }

    fn get_state(&self) -> serde_json::Value {
        serde_json::json!({
            "time": self.time,
            "guiVisible": self.gui_visible,
            "cursorSize": self.cursor_size,
            "cursorStrength": self.cursor_strength,
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Determine cursor mode based on mouse_button
        let cursor_mode = if mouse_button == 0 {
            1 // left click = spawn particles
        } else if mouse_button == 2 {
            2 // right click = destroy particles
        } else {
            0 // middle click or other = no interaction
        };

        // If we're trying to spawn particles but have none, create a small batch
        if cursor_mode == 1 && self.settings.particle_limit == 0 {
            // Create a small initial batch of particles for painting
            let initial_particle_limit = 100;
            self.settings.particle_limit = initial_particle_limit;

            // Create initial particles for painting
            let mut particles = Vec::with_capacity(initial_particle_limit as usize);

            // No longer need LUT data since we use LUT in shader
            RNG.with(|rng| {
                let mut rng = rng.borrow_mut();
                for _ in 0..initial_particle_limit {
                    // Create particles in a small area around the cursor
                    let radius = 0.05; // Small radius around cursor
                    let angle = rng.random_range(0.0..std::f32::consts::TAU);
                    let distance = rng.random_range(0.0..radius);

                    let x = world_x + angle.cos() * distance;
                    let y = world_y + angle.sin() * distance;
                    let age = 0.0; // Start fresh

                    // No longer need to generate colors here since we use LUT in shader
                    let color = [0.0, 0.0, 0.0, 0.9]; // Placeholder color, will be overridden by LUT

                    let particle = Particle {
                        position: [x, y],
                        age,
                        color,
                        my_parent_was: 1, // Brush-spawned
                    };
                    particles.push(particle);
                }
            });

            // Update the particles vector
            self.particles = particles;

            // Recreate particle buffer with new particles
            self.particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particle Buffer"),
                contents: bytemuck::cast_slice(&self.particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            // Update sim params with new particle count
            let sim_params = SimParams {
                particle_limit: self.settings.particle_limit,
                autospawn_limit: self.settings.autospawn_limit,
                vector_count: self.flow_vectors.len() as u32,
                particle_lifetime: self.settings.particle_lifetime,
                particle_speed: self.settings.particle_speed,
                noise_seed: self.settings.noise_seed,
                time: self.time,
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
                background_type: self.settings.background as u32,
                screen_width: self.trail_map_width,
                screen_height: self.trail_map_height,
                cursor_x: world_x,
                cursor_y: world_y,
                cursor_active: cursor_mode,
                cursor_size: self.cursor_size,
                cursor_strength: self.cursor_strength,
                particle_autospawn: self.settings.particle_autospawn as u32,
                particle_spawn_rate: self.settings.particle_spawn_rate,
                display_mode: self.settings.display_mode as u32,
            };

            queue.write_buffer(
                &self.sim_params_buffer,
                0,
                bytemuck::cast_slice(&[sim_params]),
            );

            tracing::debug!(
                initial_particle_limit = initial_particle_limit,
                world_x = world_x,
                world_y = world_y,
                "Created initial particles for painting after kill all"
            );
        }

        // Store cursor values in the model
        self.cursor_active_mode = cursor_mode;
        self.cursor_world_x = world_x;
        self.cursor_world_y = world_y;
        // Don't override cursor_size and cursor_strength - let them be controlled by frontend

        tracing::debug!(
            world_x = world_x,
            world_y = world_y,
            cursor_mode = cursor_mode,
            cursor_mode_name = match cursor_mode {
                0 => "inactive",
                1 => "spawn",
                2 => "destroy",
                _ => "unknown",
            },
            cursor_size = self.cursor_size,
            "Flow mouse interaction updated"
        );

        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // Turn off cursor interaction and reset position
        self.cursor_active_mode = 0;
        self.cursor_world_x = 0.0;
        self.cursor_world_y = 0.0;

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
        _queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            self.settings = new_settings;
        }
        Ok(())
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        self.time = 0.0;

        // Reset particles
        let mut particles = Vec::with_capacity(self.settings.particle_limit as usize);

        // No longer need LUT data since we use LUT in shader
        RNG.with(|rng| {
            let mut rng = rng.borrow_mut();
            for _ in 0..self.settings.particle_limit {
                let x = rng.random_range(-1.0..1.0);
                let y = rng.random_range(-1.0..1.0);
                let age = rng.random_range(0.0..self.settings.particle_lifetime);

                // No longer need to generate colors here since we use LUT in shader
                let color = [0.0, 0.0, 0.0, 0.9]; // Placeholder color, will be overridden by LUT

                let particle = Particle {
                    position: [x, y],
                    age,
                    color,
                    my_parent_was: 0, // Autospawned
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

        // Also clear the trail texture view for rendering
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        queue.submit(std::iter::once(encoder.finish()));

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
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        let mut rng = rand::rng();

        self.settings.noise_seed = rng.random();
        self.settings.noise_scale = rng.random_range(0.5..3.0);
        self.settings.vector_magnitude = rng.random_range(0.05..0.2);
        self.settings.particle_speed = rng.random_range(0.01..0.05);

        // Regenerate flow vectors with new settings
        self.regenerate_flow_vectors(queue)?;

        Ok(())
    }
}

impl FlowModel {
    pub fn kill_all_particles(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> crate::error::SimulationResult<()> {
        // If auto-spawn is disabled, don't spawn any particles
        if !self.settings.particle_autospawn {
            // Update the particles vector
            self.particles = Vec::new();

            // Recreate particle buffer with empty particles
            self.particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Empty Particle Buffer"),
                contents: bytemuck::cast_slice(&self.particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            // Update sim params with zero particle count
            let sim_params = SimParams {
                particle_limit: 0,
                autospawn_limit: self.settings.autospawn_limit,
                vector_count: self.flow_vectors.len() as u32,
                particle_lifetime: self.settings.particle_lifetime,
                particle_speed: self.settings.particle_speed,
                noise_seed: self.settings.noise_seed,
                time: self.time,
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
                background_type: self.settings.background as u32,
                screen_width: self.trail_map_width,
                screen_height: self.trail_map_height,
                cursor_x: self.cursor_world_x,
                cursor_y: self.cursor_world_y,
                cursor_active: self.cursor_active_mode,
                cursor_size: self.cursor_size,
                cursor_strength: self.cursor_strength,
                particle_autospawn: self.settings.particle_autospawn as u32,
                particle_spawn_rate: self.settings.particle_spawn_rate,
                display_mode: self.settings.display_mode as u32,
            };

            queue.write_buffer(
                &self.sim_params_buffer,
                0,
                bytemuck::cast_slice(&[sim_params]),
            );

            // Recreate bind groups with the new empty buffer
            self.particle_update_bind_group =
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Particle Update Bind Group"),
                    layout: &self.particle_update_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.particle_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.flow_vector_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.sim_params_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&self.trail_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: self.lut_buffer.as_entire_binding(),
                        },
                    ],
                });

            // Also recreate particle render bind group
            self.particle_render_bind_group =
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Particle Render Bind Group"),
                    layout: &self.particle_render_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.particle_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.camera.buffer().as_entire_binding(),
                        },
                    ],
                });

            return Ok(());
        }

        // Auto-spawn is enabled - reset particles to initial state with proper random generation
        let mut particles = Vec::with_capacity(self.settings.particle_limit as usize);
        let mut rng = rand::rng();

        // No longer need LUT data since we use LUT in shader
        for _ in 0..self.settings.particle_limit {
            // Use proper random number generation
            let x = rng.random_range(-1.0..1.0);
            let y = rng.random_range(-1.0..1.0);
            let age = rng.random_range(0.0..self.settings.particle_lifetime * 0.1); // Start with 10% of lifetime max

            // No longer need to generate colors here since we use LUT in shader
            let color = [0.0, 0.0, 0.0, 0.9]; // Placeholder color, will be overridden by LUT

            let particle = Particle {
                position: [x, y],
                age,
                color,
                my_parent_was: 1, // Brush-spawned
            };
            particles.push(particle);
        }

        // Update the particles vector
        self.particles = particles;

        // Recreate particle buffer with reset particles
        self.particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Reset Particle Buffer"),
            contents: bytemuck::cast_slice(&self.particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Update sim params with original particle count
        let sim_params = SimParams {
            particle_limit: self.settings.particle_limit,
            autospawn_limit: self.settings.autospawn_limit,
            vector_count: self.flow_vectors.len() as u32,
            particle_lifetime: self.settings.particle_lifetime,
            particle_speed: self.settings.particle_speed,
            noise_seed: self.settings.noise_seed,
            time: self.time,
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
            background_type: self.settings.background as u32,
            screen_width: self.trail_map_width,
            screen_height: self.trail_map_height,
            cursor_x: self.cursor_world_x,
            cursor_y: self.cursor_world_y,
            cursor_active: self.cursor_active_mode,
            cursor_size: self.cursor_size,
            cursor_strength: self.cursor_strength,
            particle_autospawn: self.settings.particle_autospawn as u32,
            particle_spawn_rate: self.settings.particle_spawn_rate,
            display_mode: self.settings.display_mode as u32,
        };

        queue.write_buffer(
            &self.sim_params_buffer,
            0,
            bytemuck::cast_slice(&[sim_params]),
        );

        // Recreate bind groups with the new particles
        self.particle_update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Update Bind Group"),
            layout: &self.particle_update_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.flow_vector_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.trail_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.lut_buffer.as_entire_binding(),
                },
            ],
        });

        // Also recreate particle render bind group
        self.particle_render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Render Bind Group"),
            layout: &self.particle_render_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.sim_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.lut_buffer.as_entire_binding(),
                },
            ],
        });

        // Force GPU to finish all commands to ensure buffer updates are complete
        device.poll(wgpu::Maintain::Wait);

        tracing::debug!("All particles removed from buffer");
        Ok(())
    }
}
