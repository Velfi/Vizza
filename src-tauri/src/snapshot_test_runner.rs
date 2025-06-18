use std::sync::Arc;
use wgpu::{Backends, Device, Instance, Queue, SurfaceConfiguration, TextureFormat};

use crate::simulations::shared::LutManager;
use crate::simulations::gray_scott::{self, GrayScottModel};
use crate::simulations::slime_mold::{self, SlimeMoldModel};
use crate::simulations::particle_life::{self, ParticleLifeModel};

/// Test-specific GPU context without surface
pub struct TestGpuContext {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub instance: Instance,
    pub adapter_info: wgpu::AdapterInfo,
}

impl TestGpuContext {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create wgpu instance
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // Request adapter without surface
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None, // No surface for tests
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find an appropriate adapter")?;

        // Get adapter info
        let adapter_info = adapter.get_info();
        println!("Using adapter for tests: {:?}", adapter_info);

        // Request device and queue with increased buffer size limit
        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 2_147_483_648; // 2 gigabytes
        limits.max_storage_buffer_binding_size = 2_147_483_648; // 2 gigabyte binding size

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            instance,
            adapter_info,
        })
    }
}

/// Configuration for snapshot tests
pub struct SnapshotTestConfig {
    pub width: u32,
    pub height: u32,
    pub iterations: u32,
}

impl Default for SnapshotTestConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            iterations: 1000,
        }
    }
}

/// Runs a simulation for a specified number of iterations and captures the final frame
pub async fn run_simulation_snapshot<F, S>(
    config: &SnapshotTestConfig,
    simulation_setup: F,
) -> Result<Vec<u8>, Box<dyn std::error::Error>>
where
    F: FnOnce(&Arc<Device>, &Arc<Queue>, &SurfaceConfiguration, &wgpu::AdapterInfo) -> Result<S, Box<dyn std::error::Error>>,
    S: SimulationRunner,
{
    // Initialize GPU context
    let gpu_context = TestGpuContext::new().await?;
    
    // Create a fake surface configuration for the simulation
    let surface_config = SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        format: TextureFormat::Bgra8UnormSrgb,
        width: config.width,
        height: config.height,
        present_mode: wgpu::PresentMode::Immediate,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    // Create simulation
    let mut simulation = simulation_setup(
        &gpu_context.device,
        &gpu_context.queue,
        &surface_config,
        &gpu_context.adapter_info,
    )?;

    // Create texture for rendering
    let texture = gpu_context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Snapshot Texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    // Run simulation for specified iterations
    for _ in 0..config.iterations {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        simulation.step_and_render(&gpu_context.device, &gpu_context.queue, &view)?;
    }

    // Capture the final frame
    let buffer_size = (config.width * config.height * 4) as u64;
    let buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Snapshot Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Snapshot Encoder"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(config.width * 4),
                rows_per_image: Some(config.height),
            },
        },
        wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
    );

    gpu_context.queue.submit(std::iter::once(encoder.finish()));

    // Read back the buffer
    let buffer_slice = buffer.slice(..);
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });

    gpu_context.device.poll(wgpu::Maintain::Wait);
    rx.receive().await.unwrap()?;

    let data = buffer_slice.get_mapped_range();
    let result = data.to_vec();

    drop(data);
    buffer.unmap();

    Ok(result)
}

/// Trait for simulations that can be run in tests
pub trait SimulationRunner {
    fn step_and_render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

// Implement SimulationRunner for each simulation type
impl SimulationRunner for SlimeMoldModel {
    fn step_and_render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.render_frame(device, queue, view)
    }
}

impl SimulationRunner for GrayScottModel {
    fn step_and_render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.render_frame(device, queue, view)
    }
}

impl SimulationRunner for ParticleLifeModel {
    fn step_and_render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.render_frame(device, queue, view)
    }
}

/// Convert raw BGRA8 data to an image
pub fn bgra_to_image(data: &[u8], width: u32, height: u32) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            // Convert BGRA to RGBA
            let pixel = image::Rgba([
                data[idx + 2], // R
                data[idx + 1], // G
                data[idx],     // B
                data[idx + 3], // A
            ]);
            img.put_pixel(x, y, pixel);
        }
    }
    
    img
}