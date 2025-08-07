use crate::simulation::SimulationManager;
use serde_json::Value;
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

#[tauri::command]
pub async fn update_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    effect_name: String,
    enabled: bool,
    params: Value,
) -> Result<String, String> {
    tracing::debug!(
        "update_post_processing_state called: {} = {}",
        effect_name,
        enabled
    );
    let mut sim_manager = manager.lock().await;

    if let Some(crate::simulations::traits::SimulationType::Flow(simulation)) =
        &mut sim_manager.current_simulation
    {
        match effect_name.as_str() {
            "blur_filter" => {
                simulation.post_processing_state.blur_filter.enabled = enabled;
                if let Some(radius) = params.get("radius").and_then(|v| v.as_f64()) {
                    simulation.post_processing_state.blur_filter.radius = radius as f32;
                }
                if let Some(sigma) = params.get("sigma").and_then(|v| v.as_f64()) {
                    simulation.post_processing_state.blur_filter.sigma = sigma as f32;
                }
                tracing::info!(
                    "Blur filter updated: enabled={}, radius={}, sigma={}",
                    enabled,
                    simulation.post_processing_state.blur_filter.radius,
                    simulation.post_processing_state.blur_filter.sigma
                );
                Ok("Post processing state updated successfully".to_string())
            }
            _ => Err(format!("Unknown post processing effect: {}", effect_name)),
        }
    } else {
        Err("This command is only available for Flow simulation".to_string())
    }
}

#[tauri::command]
pub async fn get_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Value, String> {
    tracing::debug!("get_post_processing_state called");
    let sim_manager = manager.lock().await;

    if let Some(crate::simulations::traits::SimulationType::Flow(simulation)) =
        &sim_manager.current_simulation
    {
        let state = serde_json::json!({
            "blur_filter": {
                "enabled": simulation.post_processing_state.blur_filter.enabled,
                "radius": simulation.post_processing_state.blur_filter.radius,
                "sigma": simulation.post_processing_state.blur_filter.sigma,
            }
        });
        Ok(state)
    } else {
        Err("This command is only available for Flow simulation".to_string())
    }
}

#[tauri::command]
pub async fn draw_antialiased_shape(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    center_x: f32,
    center_y: f32,
    size: f32,
    shape_type: u32,
    color: Vec<f32>, // RGBA color as array
    intensity: f32,
    antialiasing_width: f32,
    rotation: f32,
) -> Result<String, String> {
    tracing::debug!(
        "draw_antialiased_shape called at ({}, {}) with size {}",
        center_x,
        center_y,
        size
    );

    // Validate color array
    if color.len() != 4 {
        return Err("Color must be an RGBA array with 4 values".to_string());
    }

    let color_array = [color[0], color[1], color[2], color[3]];

    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(crate::simulations::traits::SimulationType::Flow(simulation)) =
        &mut sim_manager.current_simulation
    {
        simulation
            .draw_antialiased_shape(
                &gpu_ctx.device,
                &gpu_ctx.queue,
                center_x,
                center_y,
                size,
                shape_type,
                color_array,
                intensity,
                antialiasing_width,
                rotation,
            )
            .map_err(|e| format!("Failed to draw shape: {}", e))?;

        tracing::info!("Antialiased shape drawn successfully");
        Ok("Shape drawn successfully".to_string())
    } else {
        Err("This command is only available for Flow simulation".to_string())
    }
}
