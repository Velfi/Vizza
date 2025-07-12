use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn handle_mouse_interaction(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    x: f32,
    y: f32,
    mouse_button: u32, // 0 = left, 1 = middle, 2 = right
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.handle_mouse_interaction(x, y, mouse_button, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::debug!(
                "Mouse interaction handled at ({}, {}) with button {}",
                x,
                y,
                mouse_button
            );
            Ok("Mouse interaction handled successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to handle mouse interaction: {}", e);
            Err(format!("Failed to handle mouse interaction: {}", e))
        }
    }
}

#[tauri::command]
pub async fn handle_mouse_interaction_screen(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    screen_x: f32,
    screen_y: f32,
    mouse_button: u32, // 0 = left, 1 = middle, 2 = right
) -> Result<String, String> {
    tracing::trace!(
        "Mouse interaction: screen=({}, {}), button={}",
        screen_x,
        screen_y,
        mouse_button
    );

    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    sim_manager
        .handle_mouse_interaction_screen_coords(screen_x, screen_y, mouse_button, &gpu_ctx.queue)
        .map_err(|e| e.to_string())?;
    Ok("Mouse interaction handled".to_string())
}

#[tauri::command]
pub async fn handle_mouse_release(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    mouse_button: u32, // 0 = left, 1 = middle, 2 = right
) -> Result<String, String> {
    tracing::trace!("Mouse release: button={}", mouse_button);

    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    sim_manager
        .handle_mouse_release(mouse_button, &gpu_ctx.queue)
        .map_err(|e| e.to_string())?;
    Ok("Mouse release handled".to_string())
}

#[tauri::command]
pub async fn update_cursor_position_screen(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    screen_x: f32,
    screen_y: f32,
) -> Result<String, String> {
    let _sim_manager = manager.lock().await;
    let _gpu_ctx = gpu_context.lock().await;

    // This is a placeholder for future cursor position tracking
    // Currently, cursor position is handled through mouse interaction commands
    tracing::debug!("Cursor position updated to ({}, {})", screen_x, screen_y);
    Ok("Cursor position updated successfully".to_string())
}

#[tauri::command]
pub async fn seed_random_noise(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.seed_random_noise(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::debug!("Random noise seeded successfully");
            Ok("Random noise seeded successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to seed random noise: {}", e);
            Err(format!("Failed to seed random noise: {}", e))
        }
    }
}
