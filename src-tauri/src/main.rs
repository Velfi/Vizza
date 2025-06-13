// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow, Emitter};
use wgpu::{Backends, Device, Instance, Queue, Surface, SurfaceConfiguration, Adapter};
use tracing::{info, debug};

mod simulation_manager;
mod simulations;

use simulation_manager::SimulationManager;

/// Unified GPU context managed by Tauri with surface
pub struct GpuContext {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub instance: Instance,
    pub adapter_info: wgpu::AdapterInfo,
    pub surface: Surface<'static>,
    pub surface_config: Arc<tokio::sync::Mutex<SurfaceConfiguration>>,
}

impl GpuContext {
    pub async fn new_with_surface(window: &WebviewWindow) -> Result<Self, Box<dyn std::error::Error>> {
        // Create wgpu instance
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // Create surface from window (this must happen on main thread)
        let surface = instance.create_surface(window.clone())?;

        // Request adapter with surface
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find an appropriate adapter")?;

        // Get adapter info
        let adapter_info = adapter.get_info();
        println!("Using adapter: {:?}", adapter_info);

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        // Get window size and create surface config
        let window_size = window.inner_size()?;
        let surface_caps = surface.get_capabilities(&adapter);
        
        // Choose appropriate surface format
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Configure surface
        surface.configure(&device, &surface_config);
        
        println!("Surface initialized successfully: {}x{}", surface_config.width, surface_config.height);

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            instance,
            adapter_info,
            surface,
            surface_config: Arc::new(tokio::sync::Mutex::new(surface_config)),
        })
    }

    /// Update surface configuration for resize
    pub async fn resize_surface(&self, width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = self.surface_config.lock().await;
        config.width = width;
        config.height = height;
        
        // Reconfigure surface
        self.surface.configure(&self.device, &config);
        println!("Surface resized to: {}x{}", width, height);
        
        Ok(())
    }

    /// Get current surface texture for rendering
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, String> {
        self.surface.get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {}", e))
    }
}

#[tauri::command]
async fn start_slime_mold_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    info!("start_slime_mold_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    
    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();
    
    match sim_manager.start_simulation(
        "slime_mold".to_string(),
        &gpu_ctx.device,
        &gpu_ctx.queue,
        &surface_config,
        &gpu_ctx.adapter_info,
    ).await {
        Ok(_) => {
            info!("Slime mold simulation started successfully");
            Ok("Slime mold simulation started successfully".to_string())
        },
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        },
    }
}

#[tauri::command]
async fn stop_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    sim_manager.stop_simulation();
    Ok("Simulation stopped".to_string())
}

#[tauri::command]
async fn get_simulation_status(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_status())
}

#[tauri::command]
async fn render_frame(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    // debug!("render_frame called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    
    // Check if simulation is running
    if !sim_manager.is_running() {
        debug!("No simulation running");
        return Ok("No simulation running".to_string());
    }
    
    // Get surface texture
    match gpu_ctx.get_current_texture() {
        Ok(output) => {
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            match sim_manager.render(&gpu_ctx.device, &gpu_ctx.queue, &view) {
                Ok(_) => {
                    // debug!("Frame rendered successfully");
                    output.present();
                    Ok("Frame rendered successfully".to_string())
                }
                Err(e) => {
                    tracing::error!("Simulation render failed: {}", e);
                    Err(format!("Simulation render failed: {}", e))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get surface texture: {}", e);
            Err(format!("Failed to get surface texture: {}", e))
        }
    }
}

#[tauri::command]
async fn handle_window_resize(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    
    // Resize surface
    gpu_ctx.resize_surface(width, height).await
        .map_err(|e| format!("Failed to resize surface: {}", e))?;
    
    // Get updated surface config
    let surface_config = gpu_ctx.surface_config.lock().await.clone();
    
    // Notify simulation manager of resize
    match sim_manager.handle_resize(
        &gpu_ctx.device,
        &gpu_ctx.queue,
        &surface_config,
        &gpu_ctx.adapter_info,
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Resize failed: {}", e)),
    }
}

#[tauri::command]
async fn check_gpu_context_ready(
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<bool, String> {
    // Try to access the GPU context - if it exists and can be locked, it's ready
    match gpu_context.try_lock() {
        Ok(_) => Ok(true),
        Err(_) => Ok(false), // Still initializing or locked
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    tauri::Builder::default()
        .setup(|app| {
            // Create simulation manager
            let simulation_manager = Arc::new(tokio::sync::Mutex::new(SimulationManager::new()));
            app.manage(simulation_manager);

            // Get the main window
            let window = app.get_webview_window("main").unwrap();

            // Initialize GPU context with surface on main thread (synchronously)
            let app_handle = app.handle().clone();
            match tauri::async_runtime::block_on(GpuContext::new_with_surface(&window)) {
                Ok(gpu_context) => {
                    let gpu_context = Arc::new(tokio::sync::Mutex::new(gpu_context));
                    app.manage(gpu_context);
                    info!("GPU context with surface initialized successfully");
                    // Emit event to frontend that GPU context is ready
                    let _ = app_handle.emit("gpu-context-ready", ());
                }
                Err(e) => {
                    tracing::error!("Failed to initialize GPU context: {}", e);
                    return Err(e.into());
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_slime_mold_simulation,
            stop_simulation,
            get_simulation_status,
            render_frame,
            handle_window_resize,
            check_gpu_context_ready
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
