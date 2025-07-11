use crate::simulation::SimulationManager;
use crate::GpuContext;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_ecosystem_population_data(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<serde_json::Value, String> {
    tracing::debug!("get_ecosystem_population_data called");
    let sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Use the manager method to get population data
    sim_manager
        .get_ecosystem_population(&gpu_ctx.device, &gpu_ctx.queue)
        .await
}

#[tauri::command]
pub async fn toggle_species_visibility(
    ecological_role: u32,
    variant: u32,
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
) -> Result<(), String> {
    tracing::debug!(
        "toggle_species_visibility called: role={}, variant={}",
        ecological_role,
        variant
    );

    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get the ecosystem simulation
    if let Some(ecosystem_sim) = sim_manager.get_ecosystem_simulation_mut() {
        ecosystem_sim
            .toggle_species_visibility(ecological_role, variant, &gpu_ctx.queue)
            .map_err(|e| format!("Failed to toggle species visibility: {}", e))?;
    } else {
        return Err("Ecosystem simulation not found".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn get_species_visibility_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<u32>, String> {
    tracing::debug!("get_species_visibility_state called");

    let sim_manager = manager.lock().await;

    // Get the ecosystem simulation
    if let Some(ecosystem_sim) = sim_manager.get_ecosystem_simulation() {
        Ok(ecosystem_sim.visibility_flags.clone())
    } else {
        return Err("Ecosystem simulation not found".to_string());
    }
}
