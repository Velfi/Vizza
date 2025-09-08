use crate::simulation::SimulationManager;
use crate::simulations::traits::SimulationType;
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

    if let Some(crate::simulations::traits::SimulationType::GrayScott(simulation)) =
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
        Err("Gray Scott simulation not active".to_string())
    }
}

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

    match &mut sim_manager.current_simulation {
        Some(SimulationType::GrayScott(sim)) => {
            tracing::info!("Gray-Scott simulation found, loading image...");
            sim.load_nutrient_image(&gpu.queue, &image_path)
                .map_err(|e| {
                    tracing::error!("Failed to load Gray-Scott nutrient image: {}", e);
                    e.to_string()
                })?;
            tracing::info!("Gray-Scott nutrient image loaded successfully");
            Ok("Gray-Scott nutrient image loaded".to_string())
        }
        Some(other) => {
            tracing::error!(
                "Wrong simulation type: {:?}",
                std::any::type_name_of_val(other)
            );
            Err("No Gray-Scott simulation running".to_string())
        }
        None => {
            tracing::error!("No simulation running");
            Err("No simulation running".to_string())
        }
    }
}

#[tauri::command]
pub async fn get_gray_scott_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;

    if let Some(crate::simulations::traits::SimulationType::GrayScott(simulation)) =
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
        Err("Gray Scott simulation not active".to_string())
    }
}

#[tauri::command]
pub async fn start_gray_scott_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(SimulationType::GrayScott(sim)) = &mut sim_manager.current_simulation {
        // Reuse device enumeration from SM webcam module
        let devices =
            crate::simulations::slime_mold::webcam::WebcamCapture::get_available_devices();
        if devices.is_empty() {
            return Err("No webcam devices available".to_string());
        }
        let device_index = devices[0];
        sim.start_webcam_capture(device_index)
            .map_err(|e| e.to_string())?;
        Ok("Gray-Scott webcam started".to_string())
    } else {
        Err("No Gray-Scott simulation running".to_string())
    }
}

#[tauri::command]
pub async fn stop_gray_scott_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(SimulationType::GrayScott(sim)) = &mut sim_manager.current_simulation {
        sim.stop_webcam_capture();
        Ok("Gray-Scott webcam stopped".to_string())
    } else {
        Err("No Gray-Scott simulation running".to_string())
    }
}

#[tauri::command]
pub async fn get_available_gray_scott_webcam_devices(
    _manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<i32>, String> {
    Ok(crate::simulations::slime_mold::webcam::WebcamCapture::get_available_devices())
}
