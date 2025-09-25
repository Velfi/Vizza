use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_voronoi_ca_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    effect_name: String,
    enabled: bool,
    params: serde_json::Value,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let simulation = sim_manager.voronoi_ca_simulation_mut()?;
    match effect_name.as_str() {
        "blur_filter" => {
            simulation.post_processing_state.blur_filter.enabled = enabled;
            if let Some(radius) = params.get("radius").and_then(|v| v.as_f64()) {
                simulation.post_processing_state.blur_filter.radius = radius as f32;
            }
            if let Some(sigma) = params.get("sigma").and_then(|v| v.as_f64()) {
                simulation.post_processing_state.blur_filter.sigma = sigma as f32;
            }
            Ok("Post processing state updated".to_string())
        }
        _ => Err("Unknown post-processing effect".to_string()),
    }
}

#[tauri::command]
pub async fn get_voronoi_ca_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;
    let simulation = sim_manager.voronoi_ca_simulation()?;
    Ok(serde_json::json!({
        "blur_filter": {
            "enabled": simulation.post_processing_state.blur_filter.enabled,
            "radius": simulation.post_processing_state.blur_filter.radius,
            "sigma": simulation.post_processing_state.blur_filter.sigma,
        }
    }))
}

#[tauri::command]
pub async fn update_voronoi_ca_border_width(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    border_width: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let simulation = sim_manager.voronoi_ca_simulation_mut()?;
    simulation.border_width = border_width.clamp(0.0, 1000.0);
    Ok("Border width updated".to_string())
}
