use crate::error::{SimulationError, SimulationResult};
use bytemuck::{Pod, Zeroable};
use serde_json::Value;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use super::settings::Settings;
use super::shaders;
use super::shaders::noise_seed::NoiseSeedCompute;
use crate::simulations::shared::{camera::Camera, LutData, LutManager};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct SimParams {
    pub width: u32,
    pub height: u32,
    pub attraction_distance: f32,
    pub kill_distance: f32,
    pub segment_length: f32,
    pub max_attractors: u32,
    pub max_nodes: u32,
    pub open_venation: u32, // 0 = closed, 1 = open
    pub enable_vein_thickening: u32,
    pub min_thickness: f32,
    pub max_thickness: f32,
    pub random_seed: u32,
    pub growth_speed: f32,
    pub delta_time: f32,
    pub frame_count: u32,
    pub enable_opacity_blending: u32,
    pub min_opacity: f32,
    pub max_opacity: f32,
    // Curve rendering parameters
    pub curve_tension: f32, // Default curve tension (0.0 = straight, 1.0 = tight)
    pub curve_segments: u32, // Number of segments to subdivide curves into
    pub _padding: f32,
}

impl SimParams {
    pub fn new(width: u32, height: u32, settings: &Settings, frame_count: u32) -> Self {
        Self {
            width,
            height,
            attraction_distance: settings.attraction_distance * 0.01, // Scale down for normalized coords
            kill_distance: settings.kill_distance * 0.01, // Scale down for normalized coords
            segment_length: settings.segment_length * 0.01, // Scale down for normalized coords
            max_attractors: settings.max_attractors,
            max_nodes: settings.max_nodes,
            open_venation: if settings.open_venation { 1 } else { 0 },
            enable_vein_thickening: if settings.enable_vein_thickening { 1 } else { 0 },
            min_thickness: settings.min_thickness,
            max_thickness: settings.max_thickness,
            random_seed: settings.random_seed,
            growth_speed: settings.growth_speed,
            delta_time: 0.016, // ~60 FPS
            frame_count,
            enable_opacity_blending: if settings.enable_opacity_blending { 1 } else { 0 },
            min_opacity: settings.min_opacity,
            max_opacity: settings.max_opacity,
            curve_tension: settings.curve_tension,
            curve_segments: settings.curve_segments,
            _padding: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Attractor {
    pub position: [f32; 2],
    pub is_active: u32, // 0 = inactive, 1 = active
    pub influence_count: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Node {
    pub position: [f32; 2],
    pub parent_index: u32, // u32::MAX if root
    pub child_count: u32,
    pub thickness: f32,
    pub is_active: u32, // 0 = inactive, 1 = active, 2 = growing tip
    pub generation: u32,
    pub accumulated_direction: [f32; 2],
    pub influence_count: u32,
    pub path_length: f32, // Total path length from root to this node
    // Curve control points for smooth rendering
    pub control_point_1: [f32; 2], // First control point for cubic Bézier
    pub control_point_2: [f32; 2], // Second control point for cubic Bézier
    pub curve_tension: f32, // Controls how tight the curve is (0.0 = straight line, 1.0 = tight curve)
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MouseParams {
    pub is_active: u32, // 0=inactive, 1=attract, 2=repel
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub density: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

#[derive(Debug)]
pub struct SpaceColonizationModel {
    // GPU resources
    pub params_buffer: wgpu::Buffer,
    pub attractors_buffer: wgpu::Buffer,
    pub nodes_buffer: wgpu::Buffer,
    pub counters_buffer: wgpu::Buffer,
    pub mouse_buffer: wgpu::Buffer,
    pub lut_buffer: Arc<wgpu::Buffer>,
    
    // Compute pipelines
    pub init_attractors_pipeline: wgpu::ComputePipeline,
    pub init_nodes_pipeline: wgpu::ComputePipeline,
    pub reset_influences_pipeline: wgpu::ComputePipeline,
    pub calculate_influences_pipeline: wgpu::ComputePipeline,
    pub grow_nodes_pipeline: wgpu::ComputePipeline,
    pub prune_attractors_pipeline: wgpu::ComputePipeline,
    pub update_thickness_pipeline: wgpu::ComputePipeline,
    
    // Noise seeding
    pub noise_seed_compute: NoiseSeedCompute,
    
    // Render pipeline
    pub render_pipeline: wgpu::RenderPipeline,
    
    // Bind groups
    pub compute_bind_group: wgpu::BindGroup,
    pub render_bind_group: wgpu::BindGroup,
    pub camera_bind_group: wgpu::BindGroup,
    
    // Simulation state
    pub settings: Settings,
    pub current_width: u32,
    pub current_height: u32,
    pub frame_count: u32,
    pub show_gui: bool,
    pub current_lut_name: String,
    pub lut_reversed: bool,
    
    // Camera
    pub camera: Camera,
    
    // Mouse interaction
    pub mouse_active_mode: u32, // 0=inactive, 1=place attractors
    pub mouse_world_x: f32,
    pub mouse_world_y: f32,
    
    // Initialization flags
    pub initialized: bool,
}

impl SpaceColonizationModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        lut_manager: &LutManager,
    ) -> SimulationResult<Self> {
        let width = surface_config.width;
        let height = surface_config.height;
        
        // Create buffers
        let params = SimParams::new(width, height, &settings, 0);
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Space Colonization Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Attractors buffer
        let attractors_size = (settings.max_attractors as usize * std::mem::size_of::<Attractor>()) as u64;
        let attractors_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Attractors Buffer"),
            size: attractors_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Nodes buffer
        let node_size = std::mem::size_of::<Node>();
        let nodes_size = (settings.max_nodes as usize * node_size) as u64;
        let nodes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Nodes Buffer"),
            size: nodes_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        
        // Counters buffer (atomic counters for active attractors, nodes, etc.)
        let counters_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Counters Buffer"),
            size: 4 * 4, // 4 u32 counters
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Mouse params buffer
        let mouse_params = MouseParams {
            is_active: 0,
            x: 0.0,
            y: 0.0,
            size: settings.mouse_attractor_size,
            density: settings.mouse_attractor_density,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        let mouse_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mouse Params Buffer"),
            contents: bytemuck::bytes_of(&mouse_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // LUT buffer
        let lut_data = lut_manager.get("MATPLOTLIB_viridis")?;
        let lut_data_u32 = lut_data.to_u32_buffer();
        let lut_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LUT Buffer"),
            contents: bytemuck::cast_slice(&lut_data_u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let lut_buffer = Arc::new(lut_buffer);
        
        // Create camera
        let mut camera = Camera::new(device, width as f32, height as f32)?;
        
        // Set camera to center on the simulation area and zoom to fit
        // The camera expects [-1,1] world space, so we center at (0,0) and zoom appropriately
        camera.position = [0.0, 0.0];
        camera.zoom = 1.0; // Start with no zoom
        camera.upload_to_gpu(queue);
        
        // Create compute shaders and pipelines - simplified version for now
        let compute_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Space Colonization Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::COMPUTE_SHADER.into()),
        });
        
        // Create a simple bind group layout (we'll expand this later)
        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Space Colonization Compute Bind Group Layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
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
        });
        
        // Create compute pipeline layout
        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Space Colonization Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipelines (we'll create minimal ones for now)
        let init_attractors_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Init Attractors Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("init_attractors"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let init_nodes_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Init Nodes Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("init_nodes"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let reset_influences_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Reset Influences Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("reset_influences"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let calculate_influences_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Calculate Influences Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("calculate_influences"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let grow_nodes_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Grow Nodes Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("grow_nodes"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let prune_attractors_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Prune Attractors Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("prune_attractors"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let update_thickness_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Update Thickness Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader_module,
            entry_point: Some("update_thickness"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        // Create noise seed compute
        let noise_seed_compute = NoiseSeedCompute::new(device);
        
        // Load separate shader modules for vertex and fragment
        let vertex_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Space Colonization Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/vertex.wgsl").into()),
        });
        let fragment_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Space Colonization Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fragment.wgsl").into()),
        });
        
        // Camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Fragment bind group layout
        let fragment_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Space Colonization Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &fragment_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Space Colonization Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader_module,
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
        
        // Create bind groups
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Space Colonization Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: attractors_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: nodes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: counters_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mouse_buffer.as_entire_binding(),
                },
            ],
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nodes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &fragment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lut_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut simulation = Self {
            params_buffer,
            attractors_buffer,
            nodes_buffer,
            counters_buffer,
            mouse_buffer,
            lut_buffer,
            init_attractors_pipeline,
            init_nodes_pipeline,
            reset_influences_pipeline,
            calculate_influences_pipeline,
            grow_nodes_pipeline,
            prune_attractors_pipeline,
            update_thickness_pipeline,
            noise_seed_compute,
            render_pipeline,
            compute_bind_group,
            render_bind_group,
            camera_bind_group,
            settings,
            current_width: width,
            current_height: height,
            frame_count: 0,
            show_gui: false,
            current_lut_name: "MATPLOTLIB_viridis".to_string(),
            lut_reversed: false,
            camera,
            mouse_active_mode: 0,
            mouse_world_x: 0.0,
            mouse_world_y: 0.0,
            initialized: false,
        };
        
        // Load and apply initial LUT
        simulation.load_initial_lut(lut_manager, queue)?;
        
        Ok(simulation)
    }
    
    /// Initialize the simulation (run once)
    pub fn initialize(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> SimulationResult<()> {
        if self.initialized {
            return Ok(());
        }
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Space Colonization Init Encoder"),
        });
        
        // Initialize attractors using the compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Init Attractors Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.init_attractors_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_attractors + 63) / 64; // Round up to nearest 64
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        // Initialize nodes
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Init Nodes Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.init_nodes_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64; // Round up to nearest 64
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        self.initialized = true;
        Ok(())
    }
    
    /// Update LUT data
    pub fn update_lut(&mut self, lut_data: &LutData, queue: &Queue) {
        let lut_data_u32 = lut_data.to_u32_buffer();
        queue.write_buffer(&self.lut_buffer, 0, bytemuck::cast_slice(&lut_data_u32));
    }
    
    /// Seed noise for attractor placement
    pub fn seed_noise(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Generate a random seed for this noise generation
        let seed = rand::random::<u32>();
        
        // Update the settings with the new seed so it's reflected in the UI
        self.settings.random_seed = seed;
        
        // Convert attractor pattern to u32
        let attractor_pattern = match self.settings.attractor_pattern {
            super::settings::AttractorPattern::Random => 0,
            super::settings::AttractorPattern::Clustered => 1,
            super::settings::AttractorPattern::Grid => 2,
            super::settings::AttractorPattern::Circular => 3,
            super::settings::AttractorPattern::Boundary => 4,
            super::settings::AttractorPattern::ImageBased => 0, // Default to Random for now
            super::settings::AttractorPattern::Leaf => 5, // Leaf-specific pattern
        };
        
        // Use GPU-based noise seeding for attractors
        self.noise_seed_compute.seed_noise(
            device,
            queue,
            &self.attractors_buffer,
            self.current_width,
            self.current_height,
            seed,
            1.0, // Full noise strength
            self.settings.max_attractors,
            attractor_pattern,
        )?;
        
        Ok(())
    }
    
    /// Load and apply initial LUT
    pub fn load_initial_lut(&mut self, lut_manager: &LutManager, queue: &Queue) -> SimulationResult<()> {
        // Load the default LUT
        let mut lut_data = lut_manager.get(&self.current_lut_name)
            .map_err(|e| SimulationError::InitializationFailed(format!("Failed to load initial LUT '{}': {}", self.current_lut_name, e)))?;
        
        // Apply reversal if needed
        if self.lut_reversed {
            lut_data.reverse();
        }
        
        // Apply to GPU buffer
        self.update_lut(&lut_data, queue);
        
        Ok(())
    }
    
    /// Get background color from the current LUT
    pub fn get_background_color(&self, lut_manager: &LutManager) -> wgpu::Color {
        match lut_manager.get(&self.current_lut_name) {
            Ok(lut_data) => {
                // Create RGBA buffer from the LUT data
                let mut colors = Vec::with_capacity(256);
                for i in 0..256 {
                    colors.push([
                        lut_data.red[i],
                        lut_data.green[i],
                        lut_data.blue[i],
                        255, // Alpha
                    ]);
                }
                
                if self.lut_reversed {
                    colors.reverse();
                }
                
                // Use the first color in the LUT as background
                if let Some(&[r, g, b, a]) = colors.first() {
                    wgpu::Color {
                        r: r as f64 / 255.0,
                        g: g as f64 / 255.0,
                        b: b as f64 / 255.0,
                        a: a as f64 / 255.0,
                    }
                } else {
                    wgpu::Color::BLACK
                }
            }
            Err(_) => wgpu::Color::BLACK,
        }
    }
    
    /// Render frame with LUT-based background color
    pub fn render_frame_with_lut(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        lut_manager: &LutManager,
    ) -> SimulationResult<()> {
        // Initialize if needed
        if !self.initialized {
            self.initialize(device, queue)?;
        }
        
        // Update frame count and params
        self.frame_count += 1;
        let params = SimParams::new(self.current_width, self.current_height, &self.settings, self.frame_count);
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Space Colonization Frame Encoder"),
        });
        
        // Compute passes for the space colonization algorithm
        // Re-enable compute passes
        // 1. Reset influences
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reset Influences Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.reset_influences_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // 2. Calculate influences between attractors and nodes
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Calculate Influences Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.calculate_influences_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_attractors + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // 3. Grow new nodes based on influences
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Grow Nodes Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.grow_nodes_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        // 4. Prune attractors that are too close to nodes
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Prune Attractors Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.prune_attractors_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_attractors + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // 5. Update node thickness if enabled
        if self.settings.enable_vein_thickening {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Update Thickness Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.update_thickness_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // Submit compute passes first
        queue.submit(std::iter::once(encoder.finish()));
        
        // Create new encoder for render pass
        let mut render_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Space Colonization Render Encoder"),
        });
        
        // Render pass to draw the branch network
        {
            let mut render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Space Colonization Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.get_background_color(lut_manager)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_bind_group, &[]);
            
            // Render the branch network using instanced rendering
            // Each node with a valid parent creates a segment
            let num_segments = self.settings.max_nodes as usize; // We'll let the vertex shader filter

            // Set up a static index buffer for the quad (two triangles)
            let quad_indices: [u16; 6] = [0, 1, 2, 2, 1, 3];
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Quad Index Buffer"),
                contents: bytemuck::cast_slice(&quad_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            // Draw call: 6 indices per instance, num_segments instances
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..num_segments as u32);
        }
        
        queue.submit(std::iter::once(render_encoder.finish()));
        Ok(())
    }
}

impl crate::simulations::traits::Simulation for SpaceColonizationModel {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Initialize if needed
        if !self.initialized {
            self.initialize(device, queue)?;
        }
        
        // Update frame count and params
        self.frame_count += 1;
        let params = SimParams::new(self.current_width, self.current_height, &self.settings, self.frame_count);
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Space Colonization Frame Encoder"),
        });
        
        // Compute passes for the space colonization algorithm
        // Re-enable compute passes
        // 1. Reset influences
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reset Influences Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.reset_influences_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // 2. Calculate influences between attractors and nodes
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Calculate Influences Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.calculate_influences_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_attractors + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // 3. Grow new nodes based on influences
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Grow Nodes Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.grow_nodes_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        // 4. Prune attractors that are too close to nodes
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Prune Attractors Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.prune_attractors_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_attractors + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // 5. Update node thickness if enabled
        if self.settings.enable_vein_thickening {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Update Thickness Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.update_thickness_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = (self.settings.max_nodes + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        // Submit compute passes first
        queue.submit(std::iter::once(encoder.finish()));
        
        // Create new encoder for render pass
        let mut render_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Space Colonization Render Encoder"),
        });
        
        // Render pass to draw the branch network
        {
            let mut render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Space Colonization Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
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
            
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.render_bind_group, &[]);
            
            // Render the branch network using instanced rendering
            // Each node with a valid parent creates a segment
            let num_segments = self.settings.max_nodes as usize; // We'll let the vertex shader filter

            // Set up a static index buffer for the quad (two triangles)
            let quad_indices: [u16; 6] = [0, 1, 2, 2, 1, 3];
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Quad Index Buffer"),
                contents: bytemuck::cast_slice(&quad_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            // Draw call: 6 indices per instance, num_segments instances
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..num_segments as u32);
        }
        
        queue.submit(std::iter::once(render_encoder.finish()));
        Ok(())
    }
    
    fn resize(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.current_width = new_config.width;
        self.current_height = new_config.height;
        self.camera.resize(new_config.width as f32, new_config.height as f32);
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
            "attraction_distance" => {
                if let Some(val) = value.as_f64() {
                    self.settings.attraction_distance = val as f32;
                }
            }
            "kill_distance" => {
                if let Some(val) = value.as_f64() {
                    self.settings.kill_distance = val as f32;
                }
            }
            "segment_length" => {
                if let Some(val) = value.as_f64() {
                    self.settings.segment_length = val as f32;
                }
            }
            "open_venation" => {
                if let Some(val) = value.as_bool() {
                    self.settings.open_venation = val;
                }
            }
            "enable_vein_thickening" => {
                if let Some(val) = value.as_bool() {
                    self.settings.enable_vein_thickening = val;
                }
            }
            "min_thickness" => {
                if let Some(val) = value.as_f64() {
                    self.settings.min_thickness = val as f32;
                }
            }
            "max_thickness" => {
                if let Some(val) = value.as_f64() {
                    self.settings.max_thickness = val as f32;
                }
            }
            "enable_opacity_blending" => {
                if let Some(val) = value.as_bool() {
                    self.settings.enable_opacity_blending = val;
                }
            }
            "min_opacity" => {
                if let Some(val) = value.as_f64() {
                    self.settings.min_opacity = val as f32;
                }
            }
            "max_opacity" => {
                if let Some(val) = value.as_f64() {
                    self.settings.max_opacity = val as f32;
                }
            }
            "growth_speed" => {
                if let Some(val) = value.as_f64() {
                    self.settings.growth_speed = val as f32;
                }
            }
            "random_seed" => {
                if let Some(val) = value.as_u64() {
                    self.settings.random_seed = val as u32;
                    // Reset simulation when seed changes
                    self.initialized = false;
                }
            }
            "curve_tension" => {
                if let Some(val) = value.as_f64() {
                    self.settings.curve_tension = val as f32;
                }
            }
            "curve_segments" => {
                if let Some(val) = value.as_u64() {
                    self.settings.curve_segments = val as u32;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }
    
    fn get_state(&self) -> Value {
        serde_json::json!({
            "frame_count": self.frame_count,
            "initialized": self.initialized,
            "current_width": self.current_width,
            "current_height": self.current_height,
            "show_gui": self.show_gui,
            "current_lut_name": self.current_lut_name,
            "lut_reversed": self.lut_reversed,
            "camera": self.camera.get_state()
        })
    }
    
    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if !self.settings.interactive_attractors {
            return Ok(());
        }
        
        // Convert pixel coordinates to normalized coordinates
        let normalized_x = (world_x / self.current_width as f32) * 2.0 - 1.0;
        let normalized_y = 1.0 - (world_y / self.current_height as f32) * 2.0;
        
        self.mouse_world_x = normalized_x;
        self.mouse_world_y = normalized_y;
        
        match mouse_button {
            0 => { // Left click - place attractors
                self.mouse_active_mode = 1;
                let mouse_params = MouseParams {
                    is_active: 1,
                    x: normalized_x,
                    y: normalized_y,
                    size: self.settings.mouse_attractor_size * 0.01, // Scale for normalized coords
                    density: self.settings.mouse_attractor_density,
                    _pad1: 0,
                    _pad2: 0,
                    _pad3: 0,
                };
                queue.write_buffer(&self.mouse_buffer, 0, bytemuck::bytes_of(&mouse_params));
            }
            2 => { // Right click - place root node
                // For simplicity, we'll just reset the simulation for now
                self.initialized = false;
            }
            _ => {
                self.mouse_active_mode = 0;
            }
        }
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
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            let old_seed = self.settings.random_seed;
            self.settings = new_settings;
            
            // If seed changed, reinitialize
            if old_seed != self.settings.random_seed {
                self.initialized = false;
            }
        }
        Ok(())
    }
    
    fn reset_runtime_state(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.frame_count = 0;
        self.initialized = false;
        Ok(())
    }
    
    fn toggle_gui(&mut self) -> bool {
        self.show_gui = !self.show_gui;
        self.show_gui
    }
    
    fn is_gui_visible(&self) -> bool {
        self.show_gui
    }
    
    fn randomize_settings(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        self.settings.randomize();
        self.initialized = false; // Trigger re-initialization
        Ok(())
    }
} 