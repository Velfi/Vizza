use crate::simulation::SimulationManager;
use crate::simulations::traits::{Simulation, SimulationType};
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn start_crystal_growth_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::debug!("start_crystal_growth_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "crystal_growth".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Crystal Growth simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Crystal Growth simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn randomize_crystal_growth_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::debug!("randomize_crystal_growth_settings called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::CrystalGrowth(simulation)) = &mut sim_manager.current_simulation {
        simulation
            .randomize_settings(&gpu_ctx.device, &gpu_ctx.queue)
            .map_err(|e| format!("Failed to randomize settings: {}", e))?;
        tracing::info!("Crystal Growth settings randomized");
        Ok("Crystal Growth settings randomized successfully".to_string())
    } else {
        Err("This command is only available for Crystal Growth simulation".to_string())
    }
}

#[tauri::command]
pub async fn reset_crystal_growth_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::debug!("reset_crystal_growth_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::CrystalGrowth(simulation)) = &mut sim_manager.current_simulation {
        simulation
            .reset_runtime_state(&gpu_ctx.device, &gpu_ctx.queue)
            .map_err(|e| format!("Failed to reset simulation: {}", e))?;
        tracing::info!("Crystal Growth simulation reset");
        Ok("Crystal Growth simulation reset successfully".to_string())
    } else {
        Err("This command is only available for Crystal Growth simulation".to_string())
    }
}
