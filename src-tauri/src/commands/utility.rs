use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub async fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
pub async fn check_gpu_context_ready(
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<bool, String> {
    let _gpu_ctx = gpu_context.lock().await;
    // If we can lock the GPU context, it's ready
    Ok(true)
}

#[tauri::command]
pub async fn toggle_gui(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    sim_manager.toggle_gui();

    let state = if sim_manager.is_gui_visible() {
        "visible"
    } else {
        "hidden"
    };

    tracing::debug!("GUI toggled to {}", state);
    Ok(format!("GUI toggled to {}", state))
}

#[tauri::command]
pub async fn get_gui_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<bool, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.is_gui_visible())
}

#[tauri::command]
pub async fn set_fps_limit(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    enabled: bool,
    limit: u32,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;
    sim_manager.set_fps_limit(enabled, limit);

    if enabled {
        tracing::debug!("FPS limit set to {}", limit);
        Ok(format!("FPS limit set to {}", limit))
    } else {
        tracing::debug!("FPS limit disabled");
        Ok("FPS limit disabled".to_string())
    }
}

#[tauri::command]
pub async fn toggle_fullscreen(app: tauri::AppHandle) -> Result<String, String> {
    // Get the main window
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Check current fullscreen state
    let is_fullscreen = window
        .is_fullscreen()
        .map_err(|e| format!("Failed to get fullscreen state: {}", e))?;

    // Toggle fullscreen state
    if is_fullscreen {
        window
            .set_fullscreen(false)
            .map_err(|e| format!("Failed to exit fullscreen: {}", e))?;
        tracing::debug!("Exited fullscreen mode");
    } else {
        window
            .set_fullscreen(true)
            .map_err(|e| format!("Failed to enter fullscreen: {}", e))?;
        tracing::debug!("Entered fullscreen mode");
    }

    // After toggling, query the new window size and force a backend resize/reconfigure
    if let Ok(size) = window.inner_size() {
        let gpu_context_state = app.state::<Arc<tokio::sync::Mutex<crate::GpuContext>>>();
        let manager_state = app.state::<Arc<tokio::sync::Mutex<SimulationManager>>>();

        let width = size.width;
        let height = size.height;

        // Reconfigure surface and notify simulation using consistent lock order: manager first, then GPU context
        let mut errors: Vec<String> = Vec::new();
        let mut sim_manager = manager_state.lock().await;
        let gpu_ctx = gpu_context_state.lock().await;

        if let Err(e) = gpu_ctx.resize_surface(width, height).await {
            errors.push(format!("resize_surface failed: {}", e));
        } else {
            let surface_config = gpu_ctx.surface_config.lock().await.clone();
            if let Err(e) =
                sim_manager.handle_resize(&gpu_ctx.device, &gpu_ctx.queue, &surface_config)
            {
                errors.push(format!("simulation handle_resize failed: {}", e));
            }
        }

        if errors.is_empty() {
            tracing::debug!(
                "Forced resize after fullscreen toggle to {}x{}",
                width,
                height
            );
        } else {
            tracing::warn!("Fullscreen toggle post-resize warnings: {:?}", errors);
        }
    }

    Ok("Toggled fullscreen".to_string())
}
