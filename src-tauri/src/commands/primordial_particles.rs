use crate::simulation::SimulationManager;
use serde_json::Value;
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn start_primordial_particles_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::debug!("start_primordial_particles_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "primordial_particles".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Primordial Particles simulation started successfully");

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

            Ok("Primordial Particles simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_primordial_particles_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    effect_name: String,
    enabled: bool,
    params: Value,
) -> Result<String, String> {
    tracing::debug!(
        "update_primordial_particles_post_processing called: {} = {}",
        effect_name,
        enabled
    );
    let mut sim_manager = manager.lock().await;

    let simulation = sim_manager.primordial_particles_simulation_mut()?;
    match effect_name.as_str() {
        "blur_filter" => {
            simulation.post_processing.blur_filter.enabled = enabled;
            if let Some(radius) = params.get("radius").and_then(|v| v.as_f64()) {
                simulation.post_processing.blur_filter.radius = radius as f32;
            }
            if let Some(sigma) = params.get("sigma").and_then(|v| v.as_f64()) {
                simulation.post_processing.blur_filter.sigma = sigma as f32;
            }
            tracing::info!(
                "Blur filter updated: enabled={}, radius={}, sigma={}",
                enabled,
                simulation.post_processing.blur_filter.radius,
                simulation.post_processing.blur_filter.sigma
            );
            Ok("Post processing state updated successfully".to_string())
        }
        _ => Err(format!("Unknown post processing effect: {}", effect_name)),
    }
}

#[tauri::command]
pub async fn get_primordial_particles_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    tracing::debug!("get_primordial_particles_post_processing_state called");
    let sim_manager = manager.lock().await;

    let simulation = sim_manager.primordial_particles_simulation()?;
    Ok(serde_json::json!({
        "blur_filter": {
            "enabled": simulation.post_processing.blur_filter.enabled,
            "radius": simulation.post_processing.blur_filter.radius,
            "sigma": simulation.post_processing.blur_filter.sigma,
        }
    }))
}
