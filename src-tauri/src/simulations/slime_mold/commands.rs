use crate::simulation::SimulationManager;
use crate::simulations::traits::SimulationType;
use crate::GpuContext;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    count: u32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    // Check if we have a slime mold simulation running
    if let Some(SimulationType::SlimeMold(simulation)) = &mut sim_manager.current_simulation {
        match simulation
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
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn get_current_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<u32>, String> {
    let sim_manager = manager.lock().await;
    if let Some(SimulationType::SlimeMold(simulation)) = &sim_manager.current_simulation {
        Ok(simulation.get_agent_count())
    } else {
        Ok(None)
    }
} 