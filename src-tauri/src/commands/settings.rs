use crate::GpuContext;
use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_simulation_setting(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    setting_name: String,
    value: serde_json::Value,
) -> Result<String, String> {
    tracing::debug!(
        "update_simulation_setting called with setting_name: '{}', value: {:?}",
        setting_name,
        value
    );

    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.update_setting(
        &setting_name,
        value.clone(),
        &gpu_ctx.device,
        &gpu_ctx.queue,
    ) {
        Ok(_) => {
            tracing::debug!("Setting '{}' updated to {:?}", setting_name, value);
            Ok(format!("Setting '{}' updated successfully", setting_name))
        }
        Err(e) => {
            tracing::error!("Failed to update setting '{}': {}", setting_name, e);
            Err(format!("Failed to update setting: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_current_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;

    match sim_manager.get_current_settings() {
        Some(settings) => Ok(settings),
        None => Err("No simulation running".to_string()),
    }
}

#[tauri::command]
pub async fn get_current_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;

    match sim_manager.get_current_state() {
        Some(state) => Ok(state),
        None => Err("No simulation running".to_string()),
    }
}

#[tauri::command]
pub async fn randomize_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.randomize_settings(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::debug!("Settings randomized successfully");
            Ok("Settings randomized successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to randomize settings: {}", e);
            Err(format!("Failed to randomize settings: {}", e))
        }
    }
}
