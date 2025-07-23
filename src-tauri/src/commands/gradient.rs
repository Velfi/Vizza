use crate::simulation::SimulationManager;
use crate::simulations::traits::SimulationType;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn set_gradient_display_mode(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    mode: u32,
) -> Result<String, String> {
    tracing::debug!("set_gradient_display_mode called with mode: {}", mode);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::Gradient(simulation)) = &mut sim_manager.current_simulation {
        simulation.set_display_mode(mode, &gpu_ctx.queue);

        let mode_name = match mode {
            0 => "smooth",
            1 => "dithered",
            _ => "unknown",
        };

        tracing::info!("Gradient display mode set to: {}", mode_name);
        Ok(format!("Gradient display mode set to: {}", mode_name))
    } else {
        Err("This command is only available for Gradient simulation".to_string())
    }
}
