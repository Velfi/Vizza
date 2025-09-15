use crate::simulation::SimulationManager;
use crate::simulations::traits::{Simulation, SimulationType};
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn start_moire_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::debug!("start_moire_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "moire".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Moiré simulation started successfully");

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

            Ok("Moiré simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn randomize_moire_settings(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::debug!("randomize_moire_settings called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::Moire(simulation)) = &mut sim_manager.current_simulation {
        simulation
            .randomize_settings(&gpu_ctx.device, &gpu_ctx.queue)
            .map_err(|e| format!("Failed to randomize settings: {}", e))?;
        tracing::info!("Moiré settings randomized");
        Ok("Moiré settings randomized successfully".to_string())
    } else {
        Err("This command is only available for Moiré simulation".to_string())
    }
}

// reset_moire_flow command removed

#[tauri::command]
pub async fn load_moire_image(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    image_path: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::Moire(sim)) = &mut sim_manager.current_simulation {
        sim.load_image_from_path(&gpu_ctx.device, &gpu_ctx.queue, &image_path)
            .map_err(|e| format!("Failed to load moiré image: {}", e))?;
        Ok("Moiré image loaded successfully".to_string())
    } else {
        Err("This command is only available for Moiré simulation".to_string())
    }
}
