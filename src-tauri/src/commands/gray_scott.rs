use crate::simulation::SimulationManager;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_gray_scott_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    effect_name: String,
    enabled: bool,
    params: Value,
) -> Result<String, String> {
    tracing::debug!(
        "update_gray_scott_post_processing_state called: {} = {}",
        effect_name,
        enabled
    );
    let mut sim_manager = manager.lock().await;
    let simulation = sim_manager.gray_scott_simulation_mut()?;

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
}

// TODO make a generic image loading command
#[tauri::command]
pub async fn load_gray_scott_nutrient_image(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    image_path: String,
) -> Result<String, String> {
    tracing::info!(
        "load_gray_scott_nutrient_image called with path: {}",
        image_path
    );

    let mut sim_manager = manager.lock().await;
    let gpu = gpu_context.lock().await;

    let sim = sim_manager.gray_scott_simulation_mut()?;
    sim.load_nutrient_image(&gpu.queue, &image_path)
        .map_err(|e| {
            tracing::error!("Failed to load Gray-Scott nutrient image: {}", e);
            e.to_string()
        })?;
    Ok("Gray-Scott nutrient image loaded".to_string())
}

#[tauri::command]
pub async fn get_gray_scott_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;
    let simulation = sim_manager.gray_scott_simulation()?;

    Ok(serde_json::json!({
        "blur_filter": {
            "enabled": simulation.post_processing_state.blur_filter.enabled,
            "radius": simulation.post_processing_state.blur_filter.radius,
            "sigma": simulation.post_processing_state.blur_filter.sigma,
        }
    }))
}

#[tauri::command]
pub async fn start_gray_scott_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let sim = sim_manager.gray_scott_simulation_mut()?;

    // Reuse device enumeration from SM webcam module
    let devices = crate::simulations::shared::webcam::WebcamCapture::get_available_devices();
    if devices.is_empty() {
        return Err("No webcam devices available".to_string());
    }
    let device_index = devices[0];
    sim.start_webcam_capture(device_index)
        .map_err(|e| e.to_string())?;
    Ok("Gray-Scott webcam started".to_string())
}

#[tauri::command]
pub async fn stop_gray_scott_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let sim = sim_manager.gray_scott_simulation_mut()?;
    sim.stop_webcam_capture();
    Ok("Gray-Scott webcam stopped".to_string())
}

#[tauri::command]
pub async fn get_available_gray_scott_webcam_devices(
    _manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<i32>, String> {
    Ok(crate::simulations::shared::WebcamCapture::get_available_devices())
}
