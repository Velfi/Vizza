use crate::GpuContext;
use crate::simulation::SimulationManager;
use crate::simulations::traits::SimulationType;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_agent_count(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    count: u32,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    // Check if we have a slime mold simulation running
    if let Some(SimulationType::SlimeMold(simulation)) = &mut sim_manager.current_simulation {
        match simulation
            .update_agent_count(count, &gpu_ctx.device, &gpu_ctx.queue, &surface_config)
            .await
        {
            Ok(_) => {
                tracing::info!("Agent count updated to {}", count);
                Ok(format!("Agent count updated to {}", count))
            }
            Err(e) => {
                tracing::error!("Failed to update agent count: {}", e);
                Err(format!("Failed to update agent count: {}", e))
            }
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
    if let Some(SimulationType::SlimeMold(simulation)) = &sim_manager.current_simulation {
        Ok(simulation.get_agent_count())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn load_slime_mold_gradient_image(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<GpuContext>>>,
    image_path: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu = gpu_context.lock().await;
    if let Some(SimulationType::SlimeMold(sim)) = &mut sim_manager.current_simulation {
        sim.load_gradient_image_from_path(&gpu.device, &gpu.queue, &image_path)
            .map_err(|e| e.to_string())?;
        Ok("Gradient image loaded".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn set_slime_mold_gradient_image_fit_mode(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    fit_mode: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(SimulationType::SlimeMold(sim)) = &mut sim_manager.current_simulation {
        sim.settings.gradient_image_fit_mode = match fit_mode.as_str() {
            "Stretch" | "stretch" => {
                crate::simulations::slime_mold::settings::GradientImageFitMode::Stretch
            }
            "Center" | "center" => {
                crate::simulations::slime_mold::settings::GradientImageFitMode::Center
            }
            _ => crate::simulations::slime_mold::settings::GradientImageFitMode::Stretch,
        };
        Ok("Gradient image fit mode set".to_string())
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
    if let Some(SimulationType::SlimeMold(sim)) = &mut sim_manager.current_simulation {
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
    if let Some(SimulationType::SlimeMold(sim)) = &mut sim_manager.current_simulation {
        sim.settings.position_image_fit_mode = match fit_mode.as_str() {
            "Stretch" | "stretch" => {
                crate::simulations::slime_mold::settings::GradientImageFitMode::Stretch
            }
            "Center" | "center" => {
                crate::simulations::slime_mold::settings::GradientImageFitMode::Center
            }
            "Fit H" | "fit h" => {
                crate::simulations::slime_mold::settings::GradientImageFitMode::FitH
            }
            "Fit V" | "fit v" => {
                crate::simulations::slime_mold::settings::GradientImageFitMode::FitV
            }
            _ => crate::simulations::slime_mold::settings::GradientImageFitMode::Stretch,
        };
        Ok("Position image fit mode set".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn start_slime_mold_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    tracing::info!("=== WEBCAM COMMAND CALLED ===");
    let mut sim_manager = manager.lock().await;
    if let Some(SimulationType::SlimeMold(sim)) = &mut sim_manager.current_simulation {
        tracing::info!("Found slime mold simulation, getting available devices...");
        // Find the first available webcam device
        let available_devices = sim.get_available_webcam_devices();
        tracing::info!("Available devices: {:?}", available_devices);
        if available_devices.is_empty() {
            tracing::error!("No webcam devices available");
            return Err("No webcam devices available".to_string());
        }

        let device_index = available_devices[0];

        match sim.start_webcam_capture(device_index) {
            Ok(_) => {
                tracing::info!(
                    "Webcam capture started successfully on device {}",
                    device_index
                );
                Ok("Webcam capture started".to_string())
            }
            Err(e) => {
                tracing::error!("Failed to start webcam capture: {}", e);
                Err(format!("Failed to start webcam capture: {}", e))
            }
        }
    } else {
        tracing::error!("No slime mold simulation running");
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn stop_slime_mold_webcam_capture(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    if let Some(SimulationType::SlimeMold(sim)) = &mut sim_manager.current_simulation {
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
    if let Some(SimulationType::SlimeMold(sim)) = &sim_manager.current_simulation {
        let devices = sim.get_available_webcam_devices();
        Ok(devices)
    } else {
        tracing::error!("No slime mold simulation running");
        Err("No slime mold simulation running".to_string())
    }
}
