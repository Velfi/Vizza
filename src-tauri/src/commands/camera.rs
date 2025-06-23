use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn pan_camera(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    delta_x: f32,
    delta_y: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let _gpu_ctx = gpu_context.lock().await;

    sim_manager.pan_camera(delta_x, delta_y);
    tracing::debug!("Camera panned by ({}, {})", delta_x, delta_y);
    Ok("Camera panned successfully".to_string())
}

#[tauri::command]
pub async fn zoom_camera(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    delta: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let _gpu_ctx = gpu_context.lock().await;

    sim_manager.zoom_camera(delta);
    tracing::debug!("Camera zoomed by {}", delta);
    Ok("Camera zoomed successfully".to_string())
}

#[tauri::command]
pub async fn zoom_camera_to_cursor(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    delta: f32,
    cursor_x: f32,
    cursor_y: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let _gpu_ctx = gpu_context.lock().await;

    sim_manager.zoom_camera_to_cursor(delta, cursor_x, cursor_y);
    tracing::debug!(
        "Camera zoomed to cursor ({}, {}) by {}",
        cursor_x,
        cursor_y,
        delta
    );
    Ok("Camera zoomed to cursor successfully".to_string())
}

#[tauri::command]
pub async fn reset_camera(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let _gpu_ctx = gpu_context.lock().await;

    sim_manager.reset_camera();
    tracing::info!("Camera reset to default position");
    Ok("Camera reset successfully".to_string())
}

#[tauri::command]
pub async fn get_camera_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;

    if let Some(camera_state) = sim_manager.get_camera_state() {
        Ok(camera_state)
    } else {
        Err("No camera state available".to_string())
    }
}

#[tauri::command]
pub async fn stop_camera_pan() -> Result<String, String> {
    // This is a placeholder for future implementation
    // Currently, camera panning is handled through the simulation manager
    tracing::debug!("Camera pan stopped");
    Ok("Camera pan stopped".to_string())
}

#[tauri::command]
pub async fn set_camera_smoothing(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    smoothing_factor: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    sim_manager.set_camera_smoothing(smoothing_factor);
    tracing::debug!("Camera smoothing factor set to: {}", smoothing_factor);
    Ok("Camera smoothing factor updated".to_string())
}
