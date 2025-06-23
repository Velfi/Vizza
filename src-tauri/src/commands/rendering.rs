use crate::simulation::SimulationManager;
use crate::simulations::traits::Simulation;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn render_frame(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<(), String> {
    let sim_manager = manager.lock().await;
    let mut gpu_ctx = gpu_context.lock().await;

    // Check if simulation is running - if so, the backend render loop is handling rendering
    if sim_manager.is_running() {
        // Backend render loop is active, don't interfere
        return Ok(());
    }

    // Get current surface texture
    let surface_texture = gpu_ctx.get_current_texture()?;
    let surface_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // Render main menu background when no simulation is running
    let device = gpu_ctx.device.clone();
    let queue = gpu_ctx.queue.clone();
    let result = gpu_ctx
        .main_menu
        .render_frame(&device, &queue, &surface_view);

    match result {
        Ok(_) => {
            surface_texture.present();
            return Ok(());
        }
        Err(e) => {
            tracing::error!("Failed to render main menu background: {}", e);
            return Err(format!("Failed to render main menu background: {}", e));
        }
    }
}

#[tauri::command]
pub async fn render_single_frame(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<(), String> {
    let sim_manager = manager.lock().await;
    let mut gpu_ctx = gpu_context.lock().await;

    // Check if simulation is running - if so, the backend render loop is handling rendering
    if sim_manager.is_running() {
        // Backend render loop is active, don't interfere
        return Ok(());
    }

    // Get current surface texture
    let surface_texture = gpu_ctx.get_current_texture()?;
    let surface_view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // Render main menu background when no simulation is running
    let device = gpu_ctx.device.clone();
    let queue = gpu_ctx.queue.clone();
    let result = gpu_ctx
        .main_menu
        .render_frame(&device, &queue, &surface_view);

    match result {
        Ok(_) => {
            surface_texture.present();
            return Ok(());
        }
        Err(e) => {
            tracing::error!("Failed to render main menu background: {}", e);
            return Err(format!("Failed to render main menu background: {}", e));
        }
    }
}

#[tauri::command]
pub async fn handle_window_resize(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Resize the surface
    if let Err(e) = gpu_ctx.resize_surface(width, height).await {
        tracing::error!("Failed to resize surface: {}", e);
        return Err(format!("Failed to resize surface: {}", e));
    }

    // Get updated surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    // Handle simulation resize
    if let Err(e) = sim_manager.handle_resize(&gpu_ctx.device, &gpu_ctx.queue, &surface_config) {
        tracing::error!("Failed to handle simulation resize: {}", e);
        return Err(format!("Failed to handle simulation resize: {}", e));
    }

    tracing::info!("Window resized to {}x{}", width, height);
    Ok(())
}
