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

    match sim_manager.handle_mouse_interaction(x, y, mouse_button, &gpu_ctx.device, &gpu_ctx.queue)
    {
        Ok(_) => Ok("Mouse interaction handled successfully".to_string()),
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
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    sim_manager
        .handle_mouse_interaction_screen_coords(
            screen_x,
            screen_y,
            mouse_button,
            &gpu_ctx.device,
            &gpu_ctx.queue,
        )
        .map_err(|e| e.to_string())?;
    Ok("Mouse interaction handled".to_string())
}

#[tauri::command]
pub async fn handle_mouse_release(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    mouse_button: u32, // 0 = left, 1 = middle, 2 = right
) -> Result<String, String> {
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
    _screen_x: f32,
    _screen_y: f32,
) -> Result<String, String> {
    let _sim_manager = manager.lock().await;
    let _gpu_ctx = gpu_context.lock().await;
    // Currently, cursor position is handled through mouse interaction commands
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
        Ok(_) => Ok("Random noise seeded successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to seed random noise: {}", e);
            Err(format!("Failed to seed random noise: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_cursor_size(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    size: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_cursor_size(size, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Cursor size updated successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to update cursor size: {}", e);
            Err(format!("Failed to update cursor size: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_cursor_strength(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    strength: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_cursor_strength(strength, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok("Cursor strength updated successfully".to_string()),
        Err(e) => {
            tracing::error!("Failed to update cursor strength: {}", e);
            Err(format!("Failed to update cursor strength: {}", e))
        }
    }
}
