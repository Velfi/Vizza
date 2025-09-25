use crate::SimulationType;
use crate::simulation::manager::SimulationManager;
use crate::simulations::shared::color_scheme::ColorScheme;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn apply_color_scheme_by_name(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    color_scheme_name: String,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.apply_color_scheme(&color_scheme_name, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Color scheme '{}' applied successfully", color_scheme_name);
            Ok(format!(
                "Color scheme '{}' applied successfully",
                color_scheme_name
            ))
        }
        Err(e) => {
            tracing::error!(
                "Failed to apply color scheme '{}': {}",
                color_scheme_name,
                e
            );
            Err(format!(
                "Failed to apply color scheme '{}': {}",
                color_scheme_name, e
            ))
        }
    }
}

#[tauri::command]
pub async fn apply_color_scheme(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    color_scheme_data: Vec<u8>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    let color_scheme =
        ColorScheme::from_bytes("custom_color_scheme".to_string(), &color_scheme_data)
            .map_err(|e| format!("Failed to create color scheme from data: {}", e))?;

    match sim_manager.apply_custom_color_scheme(&color_scheme, &gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Custom color scheme applied successfully");
            Ok("Custom color scheme applied successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to apply custom color scheme: {}", e);
            Err(format!("Failed to apply custom color scheme: {}", e))
        }
    }
}

#[tauri::command]
pub async fn toggle_color_scheme_reversed(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    match sim_manager.reverse_current_color_scheme(&gpu_ctx.device, &gpu_ctx.queue) {
        Ok(_) => {
            tracing::info!("Color scheme reversed successfully");
            Ok("Color scheme reversed successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to reverse color scheme: {}", e);
            Err(format!("Failed to reverse color scheme: {}", e))
        }
    }
}

#[tauri::command]
pub async fn save_custom_color_scheme(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    name: String,
    color_scheme_data: Vec<u8>,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;

    // Create ColorSchemeData from the byte data
    let lut_data = ColorScheme::from_bytes(name.clone(), &color_scheme_data)
        .map_err(|e| format!("Failed to create color scheme data: {}", e))?;

    match sim_manager
        .color_scheme_manager
        .save_custom(&name, &lut_data)
    {
        Ok(_) => {
            tracing::info!("Custom color scheme '{}' saved successfully", name);
            Ok(format!("Custom color scheme '{}' saved successfully", name))
        }
        Err(e) => {
            tracing::error!("Failed to save custom color scheme '{}': {}", name, e);
            Err(format!(
                "Failed to save custom color scheme '{}': {}",
                name, e
            ))
        }
    }
}

#[tauri::command]
pub async fn update_gradient_preview(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    color_scheme_data: Vec<u8>,
) -> Result<String, String> {
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    let lut_data = ColorScheme::from_bytes("gradient_preview".to_string(), &color_scheme_data)
        .map_err(|e| format!("Failed to create color scheme data: {}", e))?;

    // Apply the preview color scheme to any running simulation
    match sim_manager.apply_custom_color_scheme(&lut_data, &gpu_ctx.device, &gpu_ctx.queue) {
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
pub async fn get_available_color_schemes(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<String>, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_available_color_schemes())
}

#[tauri::command]
pub async fn get_current_color_scheme_colors(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<Vec<Vec<u8>>, String> {
    let sim_manager = manager.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &sim_manager.current_simulation {
        let species_colors = &simulation.state.species_colors;
        let mut colors = Vec::with_capacity(species_colors.len());

        // Convert from linear RGB (GPU space) to sRGB for UI display
        fn linear_to_srgb(linear: f32) -> f32 {
            if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
        }

        for &[r_lin, g_lin, b_lin, _a] in species_colors {
            let r = (linear_to_srgb(r_lin).clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (linear_to_srgb(g_lin).clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (linear_to_srgb(b_lin).clamp(0.0, 1.0) * 255.0).round() as u8;
            colors.push(vec![r, g, b]);
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
