use std::sync::Arc;
use tauri::State;

use crate::simulation_manager::SimulationManager;

#[tauri::command]
pub async fn update_simulation_setting(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    setting_name: String,
    value: serde_json::Value,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_setting(&setting_name, value.clone(), &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Setting '{}' updated to {:?}", setting_name, value);
            Ok(format!("Setting '{}' updated successfully", setting_name))
        }
        Err(e) => {
            tracing::error!("Failed to update setting '{}': {}", setting_name, e);
            Err(format!("Failed to update setting '{}': {}", setting_name, e))
        }
    }
}

#[tauri::command]
pub async fn get_current_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<serde_json::Value>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_current_settings())
}

#[tauri::command]
pub async fn get_current_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<serde_json::Value>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_current_state())
}

#[tauri::command]
pub async fn randomize_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.randomize_settings(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Settings randomized successfully");
            Ok("Settings randomized successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to randomize settings: {}", e);
            Err(format!("Failed to randomize settings: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    count: u32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .update_agent_count(count, &gpu_ctx.device, &gpu_ctx.queue, &surface_config)
        .await
    {
        Ok(_) => {
            tracing::info!("Agent count updated to {}", count);
            Ok(format!("Agent count updated to {}", count))
        }
        Err(e) => {
            tracing::error!("Failed to update agent count: {}", e);
            Err(format!("Failed to update agent count: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_current_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<u32>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_current_agent_count())
} 