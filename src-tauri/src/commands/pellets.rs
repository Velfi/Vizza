use crate::simulation::SimulationManager;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_pellets_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    effect_name: String,
    enabled: bool,
    params: Value,
) -> Result<String, String> {
    tracing::debug!(
        "update_pellets_post_processing_state called: {} = {}",
        effect_name,
        enabled
    );
    let mut sim_manager = manager.lock().await;

    if let Some(crate::simulations::traits::SimulationType::Pellets(simulation)) =
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
            _ => Err(format!("Unknown effect: {}", effect_name)),
        }
    } else {
        Err("Pellets simulation not active".to_string())
    }
}

#[tauri::command]
pub async fn update_pellets_trails_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    enabled: bool,
    fade: f32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::Pellets(simulation)) =
        &mut sim_manager.current_simulation
    {
        simulation.state.trails_enabled = enabled;
        simulation.state.trail_fade = fade.clamp(0.0, 1.0);
        Ok("Pellets trails state updated".to_string())
    } else {
        Err("Pellets simulation not active".to_string())
    }
}

#[tauri::command]
pub async fn get_pellets_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;

    if let Some(crate::simulations::traits::SimulationType::Pellets(simulation)) =
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
        Err("Pellets simulation not active".to_string())
    }
}
