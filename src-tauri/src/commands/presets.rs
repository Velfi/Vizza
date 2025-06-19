use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_available_presets(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_available_presets())
}

#[tauri::command]
pub async fn get_presets_for_simulation_type(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    simulation_type: String,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_presets_for_simulation_type(&simulation_type))
}

#[tauri::command]
pub async fn apply_preset(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    preset_name: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_preset(&preset_name, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Preset '{}' applied successfully", preset_name);
            Ok(format!("Preset '{}' applied successfully", preset_name))
        }
        Err(e) => {
            tracing::error!("Failed to apply preset '{}': {}", preset_name, e);
            Err(format!("Failed to apply preset '{}': {}", preset_name, e))
        }
    }
}

#[tauri::command]
pub async fn save_preset(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    preset_name: String,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;

    // Get current settings (not state) for saving
    if let Some(settings) = sim_manager.get_current_settings() {
        match sim_manager.save_preset(&preset_name, &settings) {
            Ok(_) => {
                tracing::info!("Preset '{}' saved successfully", preset_name);
                Ok(format!("Preset '{}' saved successfully", preset_name))
            }
            Err(e) => {
                tracing::error!("Failed to save preset '{}': {}", preset_name, e);
                Err(format!("Failed to save preset '{}': {}", preset_name, e))
            }
        }
    } else {
        Err("No simulation running to save preset from".to_string())
    }
}

#[tauri::command]
pub async fn delete_preset(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    preset_name: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;

    match sim_manager.delete_preset(&preset_name) {
        Ok(_) => {
            tracing::info!("Preset '{}' deleted successfully", preset_name);
            Ok(format!("Preset '{}' deleted successfully", preset_name))
        }
        Err(e) => {
            tracing::error!("Failed to delete preset '{}': {}", preset_name, e);
            Err(format!("Failed to delete preset '{}': {}", preset_name, e))
        }
    }
} 