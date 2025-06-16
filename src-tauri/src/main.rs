// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Emitter, Manager, State, WebviewWindow};
use wgpu::{Backends, Device, Instance, Queue, Surface, SurfaceConfiguration};

mod main_menu_renderer;
mod simulation_manager;
mod simulations;

use main_menu_renderer::MainMenuRenderer;
use simulation_manager::SimulationManager;

use crate::simulations::shared::LutData;
use crate::simulations::particle_life::shaders::hsv_to_rgb;

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
        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 2_147_483_648; // 2 gigabytes
        limits.max_storage_buffer_binding_size = 2_147_483_648; // 2 gigabyte binding size

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

#[tauri::command]
async fn start_slime_mold_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("start_slime_mold_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "slime_mold".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Slime mold simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Slime mold simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
async fn start_gray_scott_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("start_gray_scott_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "gray_scott".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Gray-Scott simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Gray-Scott simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
async fn start_particle_life_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("start_particle_life_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "particle_life".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Particle Life simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Particle Life simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
async fn stop_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;
    sim_manager.stop_render_loop();
    Ok("Simulation paused".to_string())
}

#[tauri::command]
async fn resume_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("resume_simulation called");
    let sim_manager = manager.lock().await;

    // Only resume if simulation exists
    if sim_manager.is_running() {
        // Restart the render loop
        sim_manager.start_render_loop(
            app.clone(),
            gpu_context.inner().clone(),
            manager.inner().clone(),
        );

        // Emit event to notify frontend that simulation is resumed
        if let Err(e) = app.emit("simulation-resumed", ()) {
            tracing::warn!("Failed to emit simulation-resumed event: {}", e);
        }

        Ok("Simulation resumed successfully".to_string())
    } else {
        Err("No simulation to resume".to_string())
    }
}

#[tauri::command]
async fn destroy_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    tracing::info!("destroy_simulation called");
    let mut sim_manager = manager.lock().await;
    sim_manager.stop_render_loop();
    sim_manager.stop_simulation();
    Ok("Simulation destroyed".to_string())
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
        // Render triangle when no simulation is running
        match gpu_ctx.get_current_texture() {
            Ok(output) => {
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                match gpu_ctx
                    .main_menu_renderer
                    .render(&gpu_ctx.device, &gpu_ctx.queue, &view, sim_manager.get_time())
                {
                    Ok(_) => {
                        output.present();
                        return Ok("Triangle rendered".to_string());
                    }
                    Err(e) => {
                        tracing::error!("Failed to render triangle: {}", e);
                        return Err(format!("Failed to render triangle: {}", e));
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to get surface texture for triangle: {}", e);
                return Err(format!("Failed to get surface texture for triangle: {}", e));
            }
        }
    }

    // Get surface texture
    match gpu_ctx.get_current_texture() {
        Ok(output) => {
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
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
    gpu_ctx
        .resize_surface(width, height)
        .await
        .map_err(|e| format!("Failed to resize surface: {}", e))?;

    // Get updated surface config
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    // Notify simulation manager of resize
    match sim_manager.handle_resize(&gpu_ctx.device, &gpu_ctx.queue, &surface_config) {
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

#[tauri::command]
async fn update_simulation_setting(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    setting_name: String,
    value: serde_json::Value,
) -> Result<String, String> {
    tracing::info!(
        "update_simulation_setting called: {} = {:?}",
        setting_name,
        value
    );
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_setting(&setting_name, value, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok(format!("Setting {} updated successfully", setting_name)),
        Err(e) => {
            tracing::error!("Failed to update setting {}: {}", setting_name, e);
            Err(format!("Failed to update setting {}: {}", setting_name, e))
        }
    }
}

#[tauri::command]
async fn update_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    count: u32,
) -> Result<String, String> {
    tracing::info!("update_agent_count called with count: {}", count);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .update_agent_count(count, &gpu_ctx.device, &gpu_ctx.queue, &surface_config)
        .await
    {
        Ok(_) => Ok(format!("Agent count updated to {}", count)),
        Err(e) => {
            tracing::error!("Failed to update agent count: {}", e);
            Err(format!("Failed to update agent count: {}", e))
        }
    }
}

#[tauri::command]
async fn get_available_presets(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_available_presets())
}

#[tauri::command]
async fn apply_preset(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    preset_name: String,
) -> Result<String, String> {
    tracing::info!("apply_preset called: {}", preset_name);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_preset(&preset_name, &gpu_ctx.queue) {
        Ok(_) => Ok(format!("Preset '{}' applied successfully", preset_name)),
        Err(e) => {
            tracing::error!("Failed to apply preset {}: {}", preset_name, e);
            Err(format!("Failed to apply preset {}: {}", preset_name, e))
        }
    }
}

#[tauri::command]
async fn save_preset(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    preset_name: String,
) -> Result<String, String> {
    tracing::info!("save_preset called: {}", preset_name);
    let sim_manager = manager.lock().await;

    if let Some(current_settings) = sim_manager.get_current_settings() {
        match sim_manager.save_preset(&preset_name, &current_settings) {
            Ok(_) => Ok(format!("Preset '{}' saved successfully", preset_name)),
            Err(e) => {
                tracing::error!("Failed to save preset {}: {}", preset_name, e);
                Err(format!("Failed to save preset {}: {}", preset_name, e))
            }
        }
    } else {
        Err("No active simulation to save preset from".to_string())
    }
}

#[tauri::command]
async fn delete_preset(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    preset_name: String,
) -> Result<String, String> {
    tracing::info!("delete_preset called: {}", preset_name);
    let mut sim_manager = manager.lock().await;

    match sim_manager.delete_preset(&preset_name) {
        Ok(_) => Ok(format!("Preset '{}' deleted successfully", preset_name)),
        Err(e) => {
            tracing::error!("Failed to delete preset {}: {}", preset_name, e);
            Err(format!("Failed to delete preset {}: {}", preset_name, e))
        }
    }
}

#[tauri::command]
async fn get_available_luts(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_available_luts())
}

#[tauri::command]
async fn apply_lut_by_index(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    lut_index: usize,
) -> Result<String, String> {
    tracing::info!("apply_lut_by_index called: {}", lut_index);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_lut_by_index(lut_index, &gpu_ctx.queue) {
        Ok(_) => Ok(format!("LUT at index {} applied successfully", lut_index)),
        Err(e) => {
            tracing::error!("Failed to apply LUT at index {}: {}", lut_index, e);
            Err(format!("Failed to apply LUT at index {}: {}", lut_index, e))
        }
    }
}

#[tauri::command]
async fn apply_lut_by_name(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    lut_name: String,
) -> Result<String, String> {
    tracing::info!("apply_lut_by_name called: {}", lut_name);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_lut(&lut_name, &gpu_ctx.queue) {
        Ok(_) => Ok(format!("LUT '{}' applied successfully", lut_name)),
        Err(e) => {
            tracing::error!("Failed to apply LUT {}: {}", lut_name, e);
            Err(format!("Failed to apply LUT {}: {}", lut_name, e))
        }
    }
}

#[tauri::command]
async fn toggle_lut_reversed(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("toggle_lut_reversed called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reverse_current_lut(&gpu_ctx.queue) {
        Ok(_) => Ok("LUT reversed toggled successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to toggle LUT reversed: {}", e);
            Err(format!("Failed to toggle LUT reversed: {}", e))
        }
    }
}

#[tauri::command]
async fn get_current_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<serde_json::Value>, String> {
    let sim_manager = manager.lock().await;
    if let Some(settings) = sim_manager.get_current_settings() {
        match serde_json::to_value(&settings) {
            Ok(json_settings) => Ok(Some(json_settings)),
            Err(e) => Err(format!("Failed to serialize settings: {}", e)),
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn get_current_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<u32>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_current_agent_count())
}

#[tauri::command]
async fn set_fps_limit(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    enabled: bool,
    limit: u32,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;
    sim_manager.set_fps_limit(enabled, limit);
    Ok(format!("FPS limit set to {} (enabled: {})", limit, enabled))
}

#[tauri::command]
async fn reset_trails(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("reset_trails called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_trails(&gpu_ctx.queue) {
        Ok(_) => Ok("Trails reset successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to reset trails: {}", e);
            Err(format!("Failed to reset trails: {}", e))
        }
    }
}

#[tauri::command]
async fn reset_agents(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("reset_agents called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_agents(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Agents reset successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to reset agents: {}", e);
            Err(format!("Failed to reset agents: {}", e))
        }
    }
}

#[tauri::command]
async fn reset_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("reset_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_simulation(&gpu_ctx.queue) {
        Ok(_) => Ok("Simulation reset successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to reset simulation: {}", e);
            Err(format!("Failed to reset simulation: {}", e))
        }
    }
}

#[tauri::command]
async fn randomize_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.randomize_settings(&gpu_ctx.queue) {
        Ok(_) => Ok("Settings randomized successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to randomize settings: {}", e);
            Err(format!("Failed to randomize settings: {}", e))
        }
    }
}

#[tauri::command]
async fn apply_custom_lut(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    lut_data: Vec<f32>,
) -> Result<String, String> {
    debug_assert_eq!(lut_data.len(), 768, "LUT data must contain 768 values");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(simulation) = &mut sim_manager.slime_mold_state {
        // Convert f32 values to u8 bytes (0-255 range)
        let byte_data: Vec<u8> = lut_data
            .iter()
            .map(|&f| (f.clamp(0.0, 255.0)) as u8)
            .collect();

        // Create LutData from the byte data
        let lut_data = LutData::from_bytes("unnamed".to_string(), &byte_data)
            .map_err(|e| format!("Failed to create LUT data: {}", e))?;

        simulation.update_lut(&lut_data, &gpu_ctx.queue);
        Ok("Custom LUT applied successfully".to_string())
    } else {
        Err("No simulation running".to_string())
    }
}

#[tauri::command]
async fn save_custom_lut(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    name: String,
    lut_data: Vec<f32>,
) -> Result<String, String> {
    debug_assert_eq!(lut_data.len(), 768, "LUT data must contain 768 values");
    let sim_manager = manager.lock().await;

    // Convert f32 values to u8 bytes (0-255 range)
    let byte_data: Vec<u8> = lut_data
        .iter()
        .map(|&f| (f.clamp(0.0, 255.0)) as u8)
        .collect();

    // Create LutData from the byte data
    let lut_data = LutData::from_bytes(name.clone(), &byte_data)
        .map_err(|e| format!("Failed to create LUT data: {}", e))?;

    match sim_manager.lut_manager.save_custom_lut(&name, &lut_data) {
        Ok(_) => Ok(format!("Custom LUT '{}' saved successfully", name)),
        Err(e) => {
            tracing::error!("Failed to save custom LUT {}: {}", name, e);
            Err(format!("Failed to save custom LUT {}: {}", name, e))
        }
    }
}

#[tauri::command]
async fn handle_mouse_interaction(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    x: f32,
    y: f32,
    is_seeding: bool,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.handle_mouse_interaction(x, y, is_seeding, &gpu_ctx.queue) {
        Ok(_) => Ok("Mouse interaction handled successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to handle mouse interaction: {}", e);
            Err(format!("Failed to handle mouse interaction: {}", e))
        }
    }
}

#[tauri::command]
async fn update_cursor_position(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    x: f32,
    y: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_cursor_position(x, y, &gpu_ctx.queue) {
        Ok(_) => Ok("Cursor position updated successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to update cursor position: {}", e);
            Err(format!("Failed to update cursor position: {}", e))
        }
    }
}

#[tauri::command]
async fn update_cursor_position_screen(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    screen_x: f32,
    screen_y: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_cursor_position_screen(screen_x, screen_y, &gpu_ctx.queue) {
        Ok(_) => Ok("Cursor position updated successfully from screen coordinates".to_string()),
        Err(e) => {
            tracing::error!(
                "Failed to update cursor position from screen coordinates: {}",
                e
            );
            Err(format!(
                "Failed to update cursor position from screen coordinates: {}",
                e
            ))
        }
    }
}

#[tauri::command]
async fn seed_random_noise(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.seed_random_noise(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Random noise seeded successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to seed random noise: {}", e);
            Err(format!("Failed to seed random noise: {}", e))
        }
    }
}

#[tauri::command]
async fn pan_camera(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    delta_x: f32,
    delta_y: f32,
) -> Result<String, String> {
    tracing::debug!(
        "Tauri pan_camera command received: delta=({:.2}, {:.2})",
        delta_x,
        delta_y
    );
    let mut sim_manager = manager.lock().await;

    match sim_manager.pan_camera(delta_x, delta_y) {
        Ok(_) => {
            tracing::debug!("Camera pan command executed successfully");
            Ok("Camera panned successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to pan camera: {}", e);
            Err(format!("Failed to pan camera: {}", e))
        }
    }
}

#[tauri::command]
async fn zoom_camera(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    delta: f32,
) -> Result<String, String> {
    tracing::debug!("Tauri zoom_camera command received: delta={:.2}", delta);
    let mut sim_manager = manager.lock().await;

    match sim_manager.zoom_camera(delta) {
        Ok(_) => {
            tracing::debug!("Camera zoom command executed successfully");
            Ok("Camera zoomed successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to zoom camera: {}", e);
            Err(format!("Failed to zoom camera: {}", e))
        }
    }
}

#[tauri::command]
async fn zoom_camera_to_cursor(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    delta: f32,
    cursor_x: f32,
    cursor_y: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    match sim_manager.zoom_camera_to_cursor(delta, cursor_x, cursor_y) {
        Ok(_) => Ok("Camera zoomed to cursor successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to zoom camera to cursor: {}", e);
            Err(format!("Failed to zoom camera to cursor: {}", e))
        }
    }
}

#[tauri::command]
async fn reset_camera(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    match sim_manager.reset_camera() {
        Ok(_) => Ok("Camera reset successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to reset camera: {}", e);
            Err(format!("Failed to reset camera: {}", e))
        }
    }
}

#[tauri::command]
async fn stop_camera_pan(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    tracing::trace!("Tauri stop_camera_pan command received");
    let mut sim_manager = manager.lock().await;

    match sim_manager.stop_camera_pan() {
        Ok(_) => {
            tracing::trace!("Camera pan stop command executed successfully");
            Ok("Camera pan stopped successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to stop camera pan: {}", e);
            Err(format!("Failed to stop camera pan: {}", e))
        }
    }
}

#[tauri::command]
async fn get_camera_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;
    if let Some(sim) = &sim_manager.gray_scott_state {
        let cam = &sim.renderer.camera;
        let state = serde_json::json!({
            "position": cam.position,
            "zoom": cam.zoom,
            "viewport_width": cam.viewport_width,
            "viewport_height": cam.viewport_height,
            "aspect_ratio": cam.uniform_data().aspect_ratio
        });
        Ok(state)
    } else {
        Err("No Gray-Scott simulation running".to_string())
    }
}

#[tauri::command]
async fn generate_matrix(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    generator: Option<String>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("generate_matrix called with generator: {:?}", generator);
    let mut sim_manager = manager.lock().await;

    if let Some(simulation) = &mut sim_manager.particle_life_state {
        // Set the generator if provided
        if let Some(generator_name) = generator {
            if let Err(e) = simulation.set_matrix_generator(&generator_name) {
                tracing::error!("Failed to set matrix generator: {}", e);
                return Err(format!("Failed to set matrix generator: {}", e));
            }
        }

        let gpu_ctx = gpu_context.lock().await;

        // Generate a new matrix using the selected generator
        match simulation.generate_matrix_with_selected_generator(&gpu_ctx.device, &gpu_ctx.queue) {
            Ok(_) => Ok("Matrix generated successfully".to_string()),
            Err(e) => {
                tracing::error!("Failed to generate matrix: {}", e);
                Err(format!("Failed to generate matrix: {}", e))
            }
        }
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn get_matrix_values(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<Vec<f64>>, String> {
    let sim_manager = manager.lock().await;

    if let Some(simulation) = &sim_manager.particle_life_state {
        let matrix_size = simulation.physics().matrix.size();
        let mut matrix_values = vec![vec![0.0; matrix_size]; matrix_size];

        for i in 0..matrix_size {
            for j in 0..matrix_size {
                matrix_values[i][j] = simulation.physics().matrix.get(i, j);
            }
        }

        Ok(matrix_values)
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn zero_matrix(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Check if we have a particle life simulation running
    if let Some(simulation) = &mut sim_manager.particle_life_state {
        match simulation.zero_matrix(&gpu_ctx.device, &gpu_ctx.queue) {
            Ok(_) => Ok("Matrix zeroed successfully".to_string()),
            Err(e) => Err(format!("Failed to zero matrix: {}", e)),
        }
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn toggle_particle_life_gui(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    // Check if we have a particle life simulation running
    if let Some(simulation) = &mut sim_manager.particle_life_state {
        simulation.toggle_gui();
        let visible = simulation.is_gui_visible();
        Ok(format!(
            "GUI {} successfully",
            if visible { "shown" } else { "hidden" }
        ))
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn get_particle_life_gui_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<bool, String> {
    let sim_manager = manager.lock().await;

    // Check if we have a particle life simulation running
    if let Some(simulation) = &sim_manager.particle_life_state {
        Ok(simulation.is_gui_visible())
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn reset_positions(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("reset_positions called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_positions(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Particle positions reset successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to reset positions: {}", e);
            Err(format!("Failed to reset positions: {}", e))
        }
    }
}

#[tauri::command]
async fn reset_types(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("reset_types called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(simulation) = &mut sim_manager.particle_life_state {
        simulation
            .reset_types(&gpu_ctx.device, &gpu_ctx.queue)
            .map_err(|e| format!("Failed to reset types: {}", e))?;
        Ok("Types reset successfully".to_string())
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn redistribute_particle_types(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("redistribute_particle_types called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.redistribute_particle_types(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Particle types redistributed evenly successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to redistribute particle types: {}", e);
            Err(format!("Failed to redistribute particle types: {}", e))
        }
    }
}

#[tauri::command]
async fn set_particle_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    count: usize,
) -> Result<String, String> {
    tracing::info!("set_particle_count called with count: {}", count);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.set_particle_count(count, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok(format!("Particle count set to {} successfully", count)),
        Err(e) => {
            tracing::error!("Failed to set particle count: {}", e);
            Err(format!("Failed to set particle count: {}", e))
        }
    }
}

#[tauri::command]
async fn handle_mouse_press(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    x: f32,
    y: f32,
    button: String, // "left", "right", or "middle"
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    let mouse_button = match button.as_str() {
        "left" => crate::simulations::particle_life::MouseButton::Left,
        "right" => crate::simulations::particle_life::MouseButton::Right,
        "middle" => crate::simulations::particle_life::MouseButton::Middle,
        _ => return Err(format!("Unknown mouse button: {}", button)),
    };

    match sim_manager.handle_mouse_press(x, y, mouse_button) {
        Ok(_) => Ok("Mouse press handled successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to handle mouse press: {}", e);
            Err(format!("Failed to handle mouse press: {}", e))
        }
    }
}

#[tauri::command]
async fn handle_mouse_release(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    button: String, // "left", "right", or "middle"
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    let mouse_button = match button.as_str() {
        "left" => crate::simulations::particle_life::MouseButton::Left,
        "right" => crate::simulations::particle_life::MouseButton::Right,
        "middle" => crate::simulations::particle_life::MouseButton::Middle,
        _ => return Err(format!("Unknown mouse button: {}", button)),
    };

    match sim_manager.handle_mouse_release(mouse_button) {
        Ok(_) => Ok("Mouse release handled successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to handle mouse release: {}", e);
            Err(format!("Failed to handle mouse release: {}", e))
        }
    }
}

#[tauri::command]
async fn handle_mouse_move(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    x: f32,
    y: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    match sim_manager.handle_mouse_move(x, y) {
        Ok(_) => Ok("Mouse move handled successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to handle mouse move: {}", e);
            Err(format!("Failed to handle mouse move: {}", e))
        }
    }
}

#[tauri::command]
async fn get_particle_type_colors(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<Vec<u8>>, String> {
    let sim_manager = manager.lock().await;

    if let Some(simulation) = &sim_manager.particle_life_state {
        let matrix_size = simulation.physics().matrix.size();
        let mut colors: Vec<Vec<u8>> = Vec::with_capacity(matrix_size);

        // Get the current LUT data from the simulation manager
        if let Some(lut_data) = sim_manager.get_current_lut_data() {
            for i in 0..matrix_size {
                // Map particle type to LUT index using same logic as shader
                let lut_index_normalized = if matrix_size > 1 {
                    i as f32 / (matrix_size - 1) as f32
                } else {
                    0.0
                };
                let lut_index = (lut_index_normalized * 255.0).clamp(0.0, 255.0) as usize;
                
                // Get RGB values from LUT
                let r = lut_data.red[lut_index];
                let g = lut_data.green[lut_index];
                let b = lut_data.blue[lut_index];
                
                colors.push(vec![r, g, b]);
            }
        } else {
            // Fallback to rainbow colors if no LUT data available
            for i in 0..matrix_size {
                let hue = (i as f32 / matrix_size.max(1) as f32) * 360.0;
                let rgb = hsv_to_rgb(hue, 1.0, 1.0);
                colors.push(vec![
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                ]);
            }
        }

        Ok(colors)
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
async fn update_interaction_matrix(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    matrix: Vec<Vec<f32>>,
) -> Result<String, String> {
    tracing::info!("update_interaction_matrix called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_interaction_matrix(&matrix, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Interaction matrix updated successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to update interaction matrix: {}", e);
            Err(format!("Failed to update interaction matrix: {}", e))
        }
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
            window
                .set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
                    0, 0,
                )))
                .expect("Failed to set window position");

            // Set window size to active monitor dimensions
            if let Ok(Some(monitor)) = window.current_monitor() {
                let size = monitor.size();
                if let Err(e) = window.set_size(tauri::Size::Physical(*size)) {
                    tracing::warn!("Failed to set window size to monitor dimensions: {}", e);
                } else {
                    tracing::info!(
                        "Window sized to monitor dimensions: {}x{}",
                        size.width,
                        size.height
                    );

                    // Force a resize event to ensure GPU surface is properly configured
                    // This will be handled by the frontend resize listener in App.svelte
                }
            }

            // Initialize GPU context with surface on main thread (synchronously)
            let app_handle = app.handle().clone();
            match tauri::async_runtime::block_on(GpuContext::new_with_surface(&window)) {
                Ok(gpu_context) => {
                    let gpu_context = Arc::new(tokio::sync::Mutex::new(gpu_context));
                    app.manage(gpu_context);
                    tracing::info!("GPU context with surface initialized successfully");
                    // Emit event to frontend that GPU context is ready
                    let _ = app_handle.emit("gpu-context-ready", ());
                }
                Err(e) => {
                    tracing::error!("Failed to initialize GPU context: {}", e);
                    return Err(e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_slime_mold_simulation,
            start_gray_scott_simulation,
            start_particle_life_simulation,
            stop_simulation,
            resume_simulation,
            destroy_simulation,
            get_simulation_status,
            render_frame,
            handle_window_resize,
            check_gpu_context_ready,
            update_simulation_setting,
            update_agent_count,
            get_available_presets,
            apply_preset,
            save_preset,
            delete_preset,
            get_available_luts,
            apply_lut_by_index,
            apply_lut_by_name,
            toggle_lut_reversed,
            get_current_settings,
            get_current_agent_count,
            set_fps_limit,
            reset_trails,
            reset_agents,
            reset_simulation,
            randomize_settings,
            apply_custom_lut,
            save_custom_lut,
            handle_mouse_interaction,
            update_cursor_position,
            update_cursor_position_screen,
            seed_random_noise,
            pan_camera,
            zoom_camera,
            zoom_camera_to_cursor,
            reset_camera,
            stop_camera_pan,
            get_camera_state,
            generate_matrix,
            get_matrix_values,
            zero_matrix,
            toggle_particle_life_gui,
            get_particle_life_gui_state,
            reset_positions,
            reset_types,
            redistribute_particle_types,
            set_particle_count,
            handle_mouse_press,
            handle_mouse_release,
            handle_mouse_move,
            get_particle_type_colors,
            update_interaction_matrix,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
