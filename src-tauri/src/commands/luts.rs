use crate::simulation::manager::SimulationManager;
use crate::simulations::shared::lut::LutData;
use crate::SimulationType;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn apply_lut_by_name(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    lut_name: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_lut(&lut_name, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("LUT '{}' applied successfully", lut_name);
            Ok(format!("LUT '{}' applied successfully", lut_name))
        }
        Err(e) => {
            tracing::error!("Failed to apply LUT '{}': {}", lut_name, e);
            Err(format!("Failed to apply LUT '{}': {}", lut_name, e))
        }
    }
}

#[tauri::command]
pub async fn apply_lut(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    lut_name: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_lut(&lut_name, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("LUT '{}' applied successfully", lut_name);
            Ok(format!("LUT '{}' applied successfully", lut_name))
        }
        Err(e) => {
            tracing::error!("Failed to apply LUT '{}': {}", lut_name, e);
            Err(format!("Failed to apply LUT '{}': {}", lut_name, e))
        }
    }
}

#[tauri::command]
pub async fn toggle_lut_reversed(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reverse_current_lut(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("LUT reversed successfully");
            Ok("LUT reversed successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reverse LUT: {}", e);
            Err(format!("Failed to reverse LUT: {}", e))
        }
    }
}

#[tauri::command]
pub async fn apply_custom_lut(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    lut_data: Vec<f32>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Convert f32 values to u8 bytes (0-255 range)
    let byte_data: Vec<u8> = lut_data
        .iter()
        .map(|&f| (f.clamp(0.0, 255.0)) as u8)
        .collect();

    // Create LutData from the byte data
    let lut_data = LutData::from_bytes("custom_lut".to_string(), &byte_data)
        .map_err(|e| format!("Failed to create LUT data: {}", e))?;

    // Apply the custom LUT to any running simulation
    match sim_manager.apply_custom_lut(&lut_data, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Custom LUT applied successfully");
            Ok("Custom LUT applied successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to apply custom LUT: {}", e);
            Err(format!("Failed to apply custom LUT: {}", e))
        }
    }
}

#[tauri::command]
pub async fn save_custom_lut(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    name: String,
    lut_data: Vec<f32>,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;

    // Convert f32 values to u8 bytes (0-255 range)
    let byte_data: Vec<u8> = lut_data
        .iter()
        .map(|&f| (f.clamp(0.0, 255.0)) as u8)
        .collect();

    // Create LutData from the byte data
    let lut_data = LutData::from_bytes(name.clone(), &byte_data)
        .map_err(|e| format!("Failed to create LUT data: {}", e))?;

    match sim_manager.lut_manager.save_custom(&name, &lut_data) {
        Ok(_) => {
            tracing::info!("Custom LUT '{}' saved successfully", name);
            Ok(format!("Custom LUT '{}' saved successfully", name))
        }
        Err(e) => {
            tracing::error!("Failed to save custom LUT '{}': {}", name, e);
            Err(format!("Failed to save custom LUT '{}': {}", name, e))
        }
    }
}

#[tauri::command]
pub async fn update_gradient_preview(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    lut_data: Vec<f32>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Convert f32 values to u8 bytes (0-255 range) and create LutData
    let byte_data: Vec<u8> = lut_data
        .iter()
        .map(|&f| (f.clamp(0.0, 255.0)) as u8)
        .collect();

    let lut_data = LutData::from_bytes("gradient_preview".to_string(), &byte_data)
        .map_err(|e| format!("Failed to create LUT data: {}", e))?;

    // Apply the preview LUT to any running simulation
    match sim_manager.apply_custom_lut(&lut_data, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Gradient preview updated successfully");
            Ok("Gradient preview updated successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to update gradient preview: {}", e);
            Err(format!("Failed to update gradient preview: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_available_luts(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_available_luts())
}

#[tauri::command]
pub async fn get_current_lut_colors(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<Vec<u8>>, String> {
    let sim_manager = manager.lock().await;
    
    if let Some(SimulationType::ParticleLife(simulation)) = &sim_manager.current_simulation {
        let species_colors = &simulation.state.species_colors;
        let mut colors = Vec::with_capacity(species_colors.len());

        for &[r, g, b, _a] in species_colors {
            colors.push(vec![(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]);
        }
        
        Ok(colors)
    } else {
        Err("No particle life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn get_species_colors(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<[f32; 4]>, String> {
    let sim_manager = manager.lock().await;
    if let Some(SimulationType::ParticleLife(simulation)) = &sim_manager.current_simulation {
        Ok(simulation.state.species_colors.clone())
    } else {
        Err("No particle life simulation running".to_string())
    }
}
