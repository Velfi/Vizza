use crate::simulation::SimulationManager;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn reset_trails(
    sim_manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<(), String> {
    let mut sim_manager = sim_manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    match sim_manager.reset_trails(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn reset_agents(
    sim_manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<(), String> {
    let mut sim_manager = sim_manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;
    match sim_manager.reset_agents(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn reset_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_simulation(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            // Wait for GPU operations to complete before returning
            gpu_ctx
                .device
                .poll(wgpu::wgt::PollType::Wait)
                .expect("Failed to poll device");

            tracing::info!("Simulation reset successfully");
            Ok("Simulation reset successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reset simulation: {}", e);
            Err(format!("Failed to reset simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn reset_runtime_state(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::debug!("reset_runtime_state called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reset_runtime_state(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            // Wait for GPU operations to complete before returning
            gpu_ctx
                .device
                .poll(wgpu::wgt::PollType::Wait)
                .expect("Failed to poll device");

            tracing::info!("Runtime state reset successfully");
            Ok("Runtime state reset successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reset runtime state: {}", e);
            Err(format!("Failed to reset runtime state: {}", e))
        }
    }
}

#[tauri::command]
pub async fn reset_graphics_resources(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("Resetting graphics resources for main menu");

    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Stop any running simulation and render loop
    sim_manager.stop_simulation();
    sim_manager.stop_render_loop();

    // Wait for GPU operations to complete
    gpu_ctx
        .device
        .poll(wgpu::wgt::PollType::Wait)
        .expect("Failed to poll device");

    drop(sim_manager);
    drop(gpu_ctx);

    tracing::info!("Graphics resources reset successfully");
    Ok("Graphics resources reset successfully".to_string())
}
