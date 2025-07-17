use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn kill_all_particles(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::debug!("kill_all_particles called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(crate::simulations::traits::SimulationType::Flow(simulation)) =
        &mut sim_manager.current_simulation
    {
        simulation
            .kill_all_particles(&gpu_ctx.device, &gpu_ctx.queue)
            .map_err(|e| format!("Failed to kill particles: {}", e))?;
        tracing::info!("All particles killed successfully");
        Ok("All particles killed successfully".to_string())
    } else {
        Err("This command is only available for Flow simulation".to_string())
    }
}
