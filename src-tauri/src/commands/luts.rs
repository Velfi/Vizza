use std::sync::Arc;
use tauri::State;

use crate::simulation_manager::SimulationManager;
use crate::simulations::shared::LutData;
use crate::simulations::traits::SimulationType;

#[tauri::command]
pub async fn apply_lut_by_name(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    lut_name: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_lut(&lut_name, &gpu_ctx.queue) {
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

    match sim_manager.apply_lut(&lut_name, &gpu_ctx.queue) {
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

    match sim_manager.reverse_current_lut(&gpu_ctx.queue) {
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

    // Apply to slime mold simulation if it's running
    if let Some(SimulationType::SlimeMold(simulation)) = &mut sim_manager.current_simulation {
        simulation.update_lut(&lut_data, &gpu_ctx.queue);
        tracing::info!("Custom LUT applied to slime mold simulation");
        Ok("Custom LUT applied successfully".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
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
    colors: Vec<Vec<f32>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Convert gradient stops to LUT format (256 RGB values)
    let mut lut_data = Vec::with_capacity(768); // 256 * 3 = 768
    
    // For now, assume evenly spaced stops (the frontend will handle position interpolation)
    let num_stops = colors.len();
    
    for i in 0..256 {
        let t = i as f32 / 255.0;
        
        // Find the two stops that bound this position
        let mut left_stop = &colors[0];
        let mut right_stop = &colors[num_stops - 1];
        
        for j in 0..num_stops - 1 {
            let left_pos = j as f32 / (num_stops - 1) as f32;
            let right_pos = (j + 1) as f32 / (num_stops - 1) as f32;
            
            if left_pos <= t && right_pos >= t {
                left_stop = &colors[j];
                right_stop = &colors[j + 1];
                break;
            }
        }
        
        // Interpolate between the two colors
        let left_pos = 0.0; // Assuming evenly spaced stops for now
        let right_pos = 1.0;
        let interp_t = (t - left_pos) / (right_pos - left_pos);
        
        let r = left_stop[0] + (right_stop[0] - left_stop[0]) * interp_t;
        let g = left_stop[1] + (right_stop[1] - left_stop[1]) * interp_t;
        let b = left_stop[2] + (right_stop[2] - left_stop[2]) * interp_t;
        
        lut_data.push(r);
        lut_data.push(g);
        lut_data.push(b);
    }
    
    // Convert f32 values to u8 bytes (0-255 range) and create LutData
    let byte_data: Vec<u8> = lut_data
        .iter()
        .map(|&f| (f.clamp(0.0, 1.0) * 255.0) as u8)
        .collect();
    
    let lut_data = LutData::from_bytes("gradient_preview".to_string(), &byte_data)
        .map_err(|e| format!("Failed to create LUT data: {}", e))?;
    
    // Apply the preview LUT to the simulation
    if let Some(SimulationType::SlimeMold(simulation)) = &mut sim_manager.current_simulation {
        simulation.update_lut(&lut_data, &gpu_ctx.queue);
        tracing::info!("Gradient preview updated successfully");
        Ok("Gradient preview updated successfully".to_string())
    } else {
        Err("No slime mold simulation running".to_string())
    }
}

#[tauri::command]
pub async fn get_available_luts(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_available_luts())
} 