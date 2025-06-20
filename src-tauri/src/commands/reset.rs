use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn reset_trails(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_trails(&gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Trails reset successfully");
            Ok("Trails reset successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reset trails: {}", e);
            Err(format!("Failed to reset trails: {}", e))
        }
    }
}

#[tauri::command]
pub async fn reset_agents(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_agents(&gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Agents reset successfully");
            Ok("Agents reset successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reset agents: {}", e);
            Err(format!("Failed to reset agents: {}", e))
        }
    }
}

#[tauri::command]
pub async fn reset_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_simulation(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Simulation reset successfully");
            Ok("Simulation reset successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reset simulation: {}", e);
            Err(format!("Failed to reset simulation: {}", e))
        }
    }
} 