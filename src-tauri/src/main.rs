// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Manager, WebviewWindow};
use wgpu::{Backends, Device, Instance, Queue, Surface, SurfaceConfiguration};

mod commands;
mod main_menu_renderer;
mod simulation;
mod simulations;

use main_menu_renderer::MainMenuRenderer;
use simulation::SimulationManager;

/// Unified GPU context managed by Tauri with surface
pub struct GpuContext {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub instance: Instance,
    pub adapter_info: wgpu::AdapterInfo,
    pub surface: Surface<'static>,
    pub surface_config: Arc<tokio::sync::Mutex<SurfaceConfiguration>>,
    pub main_menu_renderer: MainMenuRenderer,
}

impl GpuContext {
    pub async fn new_with_surface(
        window: &WebviewWindow,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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

        // Request device and queue with increased buffer size limit
        let limits = wgpu::Limits {
            max_buffer_size: 2_147_483_647, // 2 gigabytes - 1 byte
            max_storage_buffer_binding_size: 2_147_483_647, // 2 gigabyte binding size - 1 byte
            ..Default::default()
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
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

        println!(
            "Surface initialized successfully: {}x{}",
            surface_config.width, surface_config.height
        );

        // Create main menu renderer
        let device_arc = Arc::new(device);
        let main_menu_renderer = MainMenuRenderer::new(&device_arc, &surface_config)?;

        Ok(Self {
            device: device_arc,
            queue: Arc::new(queue),
            instance,
            adapter_info,
            surface,
            surface_config: Arc::new(tokio::sync::Mutex::new(surface_config)),
            main_menu_renderer,
        })
    }

    /// Update surface configuration for resize
    pub async fn resize_surface(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        self.surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {}", e))
    }
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(Arc::new(tokio::sync::Mutex::new(SimulationManager::new())))
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // Initialize GPU context
            let gpu_context = tauri::async_runtime::block_on(async {
                GpuContext::new_with_surface(&window).await.unwrap()
            });
            
            app.manage(Arc::new(tokio::sync::Mutex::new(gpu_context)));
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Simulation commands
            commands::start_slime_mold_simulation,
            commands::start_gray_scott_simulation,
            commands::pause_simulation,
            commands::resume_simulation,
            commands::destroy_simulation,
            commands::get_simulation_status,
            
            // Rendering commands
            commands::render_frame,
            commands::render_single_frame,
            commands::handle_window_resize,
            
            // Preset commands
            commands::get_available_presets,
            commands::get_presets_for_simulation_type,
            commands::apply_preset,
            commands::save_preset,
            commands::delete_preset,
            
            // LUT commands
            commands::apply_lut_by_name,
            commands::apply_lut,
            commands::toggle_lut_reversed,
            commands::apply_custom_lut,
            commands::save_custom_lut,
            commands::update_gradient_preview,
            commands::get_available_luts,
            
            // Camera commands
            commands::pan_camera,
            commands::zoom_camera,
            commands::zoom_camera_to_cursor,
            commands::reset_camera,
            commands::get_camera_state,
            commands::stop_camera_pan,
            
            // Settings commands
            commands::update_simulation_setting,
            commands::get_current_settings,
            commands::get_current_state,
            commands::randomize_settings,
            commands::update_agent_count,
            commands::get_current_agent_count,
            
            // Interaction commands
            commands::handle_mouse_interaction,
            commands::handle_mouse_interaction_screen,
            commands::update_cursor_position_screen,
            commands::seed_random_noise,
            
            // Utility commands
            commands::check_gpu_context_ready,
            commands::toggle_gui,
            commands::get_gui_state,
            commands::set_fps_limit,
            
            // Reset commands
            commands::reset_trails,
            commands::reset_agents,
            commands::reset_simulation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
