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

    // Do not force immediate surface reconfigure here. Render loop will acquire,
    // and on failure it performs a safe recovery path that reconfigures and resizes.
    // For observability, log the new size only.
    if let Ok(size) = window.inner_size() {
        tracing::debug!(
            "Fullscreen toggled; window size now {}x{} (defer surface resize to render loop)",
            size.width,
            size.height
        );
    }

    Ok("Toggled fullscreen".to_string())
}
