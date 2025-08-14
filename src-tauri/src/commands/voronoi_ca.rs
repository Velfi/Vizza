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
    if let Some(crate::simulations::traits::SimulationType::VoronoiCA(simulation)) =
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
                Ok("Post processing state updated".to_string())
            }
            _ => Err("Unknown post-processing effect".to_string()),
        }
    } else {
        Err("This command is only available for Voronoi CA simulation".to_string())
    }
}

#[tauri::command]
pub async fn get_voronoi_ca_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::VoronoiCA(simulation)) =
        &sim_manager.current_simulation
    {
        Ok(serde_json::json!({
            "blur_filter": {
                "enabled": simulation.post_processing_state.blur_filter.enabled,
                "radius": simulation.post_processing_state.blur_filter.radius,
                "sigma": simulation.post_processing_state.blur_filter.sigma,
            }
        }))
    } else {
        Err("This command is only available for Voronoi CA simulation".to_string())
    }
}

#[tauri::command]
pub async fn update_voronoi_ca_border_threshold(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    border_threshold: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::VoronoiCA(simulation)) =
        &mut sim_manager.current_simulation
    {
        simulation.border_threshold = border_threshold.clamp(0.0, 1.0);
        Ok("Border threshold updated".to_string())
    } else {
        Err("This command is only available for Voronoi CA simulation".to_string())
    }
}
