use crate::GpuContext;
use crate::simulation::SimulationManager;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_slime_mold_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    effect_name: String,
    enabled: bool,
    params: Value,
) -> Result<String, String> {
    tracing::debug!(
        "update_slime_mold_post_processing_state called: {} = {}",
        effect_name,
        enabled
    );
    let mut sim_manager = manager.lock().await;
    let simulation = sim_manager.slime_mold_simulation_mut()?;

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

#[tauri::command]
pub async fn get_slime_mold_post_processing_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<serde_json::Value, String> {
    let sim_manager = manager.lock().await;
    let simulation = sim_manager.slime_mold_simulation()?;

    Ok(serde_json::json!({
        "blur_filter": {
            "enabled": simulation.post_processing_state.blur_filter.enabled,
            "radius": simulation.post_processing_state.blur_filter.radius,
            "sigma": simulation.post_processing_state.blur_filter.sigma,
        }
    }))
}

// Migrated Slime Mold-specific commands from simulations/slime_mold/commands.rs
#[tauri::command]
pub async fn update_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    count: u32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    if let Some(crate::simulations::traits::SimulationType::SlimeMold(simulation)) =
        &mut sim_manager.current_simulation
    {
        match simulation
            .update_agent_count(count, &gpu_ctx.device, &gpu_ctx.queue, &surface_config)
            .await
        {
            Ok(_) => Ok(format!("Agent count updated to {}", count)),
            Err(e) => Err(format!("Failed to update agent count: {}", e)),
        }
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn get_current_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Option<u32>, String> {
    let sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(simulation)) =
        &sim_manager.current_simulation
    {
        Ok(simulation.get_agent_count())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn load_slime_mold_mask_image(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    image_path: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu = gpu_context.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        sim.load_mask_image_from_path(&gpu.device, &gpu.queue, &image_path)
            .map_err(|e| e.to_string())?;
        Ok("Mask image loaded".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn set_slime_mold_mask_image_fit_mode(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    fit_mode: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        sim.state.mask_image_fit_mode = fit_mode
            .parse::<crate::simulations::shared::ImageFitMode>()
            .expect("Invalid image fit mode");
        // Reprocess the image with the new fit mode
        sim.reprocess_mask_image_with_current_fit_mode();
        Ok("Mask image fit mode set and image reprocessed".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn load_slime_mold_position_image(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    image_path: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu = gpu_context.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        sim.load_position_image_from_path(&gpu.device, &gpu.queue, &image_path)
            .map_err(|e| e.to_string())?;
        Ok("Position image loaded".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn set_slime_mold_position_image_fit_mode(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    fit_mode: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        sim.settings.position_image_fit_mode = fit_mode
            .parse::<crate::simulations::shared::ImageFitMode>()
            .expect("Invalid position image fit mode");
        // Reprocess the image with the new fit mode
        sim.reprocess_position_image_with_current_fit_mode();
        Ok("Position image fit mode set and image reprocessed".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn start_slime_mold_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        let available_devices = sim.get_available_webcam_devices();
        if available_devices.is_empty() {
            return Err("No webcam devices available".to_string());
        }
        let device_index = available_devices[0];
        match sim.start_webcam_capture(device_index) {
            Ok(_) => Ok("Webcam capture started".to_string()),
            Err(e) => Err(format!("Failed to start webcam capture: {}", e)),
        }
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn stop_slime_mold_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        sim.stop_webcam_capture();
        Ok("Webcam capture stopped".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn get_available_webcam_devices(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<i32>, String> {
    let sim_manager = manager.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &sim_manager.current_simulation
    {
        Ok(sim.get_available_webcam_devices())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn update_slime_mold_background_mode(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    background_mode: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    if let Some(crate::simulations::traits::SimulationType::SlimeMold(sim)) =
        &mut sim_manager.current_simulation
    {
        sim.settings.background_mode = match background_mode.as_str() {
            "black" => crate::simulations::slime_mold::settings::BackgroundMode::Black,
            "white" => crate::simulations::slime_mold::settings::BackgroundMode::White,
            _ => return Err(format!("Invalid background mode: {}", background_mode)),
        };
        sim.update_background_params(&gpu_ctx.queue);
        Ok(format!("Background mode updated to: {}", background_mode))
    } else {
        Err("No slime mold simulation running".to_string())
    }
}
