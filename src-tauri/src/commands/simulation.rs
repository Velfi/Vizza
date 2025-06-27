use crate::simulation::SimulationManager;
use crate::simulations::traits::SimulationType;
use std::sync::Arc;
use tauri::{Emitter, State};
use wgpu::util::DeviceExt;

#[tauri::command]
pub async fn start_slime_mold_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("start_slime_mold_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "slime_mold".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Slime mold simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Slime mold simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn start_particle_life_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("start_particle_life_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "particle_life".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Particle Life simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Particle Life simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn start_gray_scott_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("start_gray_scott_simulation called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    // Get current surface configuration
    let surface_config = gpu_ctx.surface_config.lock().await.clone();

    match sim_manager
        .start_simulation(
            "gray_scott".to_string(),
            &gpu_ctx.device,
            &gpu_ctx.queue,
            &surface_config,
            &gpu_ctx.adapter_info,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Gray-Scott simulation started successfully");

            // Start the backend render loop
            sim_manager.start_render_loop(
                app.clone(),
                gpu_context.inner().clone(),
                manager.inner().clone(),
            );

            // Emit event to notify frontend that simulation is initialized
            if let Err(e) = app.emit("simulation-initialized", ()) {
                tracing::warn!("Failed to emit simulation-initialized event: {}", e);
            }

            Ok("Gray-Scott simulation started successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to start simulation: {}", e);
            Err(format!("Failed to start simulation: {}", e))
        }
    }
}

#[tauri::command]
pub async fn pause_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    tracing::info!("pause_simulation called");
    let sim_manager = manager.lock().await;
    sim_manager.stop_render_loop();
    Ok("Simulation paused".to_string())
}

#[tauri::command]
pub async fn resume_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("resume_simulation called");
    let sim_manager = manager.lock().await;

    if sim_manager.is_running() {
        // Start the backend render loop
        sim_manager.start_render_loop(
            app.clone(),
            gpu_context.inner().clone(),
            manager.inner().clone(),
        );

        // Emit event to notify frontend that simulation is resumed
        if let Err(e) = app.emit("simulation-resumed", ()) {
            tracing::warn!("Failed to emit simulation-resumed event: {}", e);
        }

        Ok("Simulation resumed".to_string())
    } else {
        Err("No simulation to resume".to_string())
    }
}

#[tauri::command]
pub async fn destroy_simulation(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    tracing::info!("destroy_simulation called");
    let mut sim_manager = manager.lock().await;
    sim_manager.stop_simulation();
    sim_manager.stop_render_loop();
    Ok("Simulation destroyed".to_string())
}

#[tauri::command]
pub async fn get_simulation_status(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
) -> Result<String, String> {
    let sim_manager = manager.lock().await;
    Ok(sim_manager.get_status())
}

#[tauri::command]
pub async fn scale_force_matrix(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
    scale_factor: f32,
) -> Result<String, String> {
    tracing::info!("scale_force_matrix called with factor: {}", scale_factor);
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Scale the force matrix in settings
        simulation.settings.scale_force_matrix(scale_factor);
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix scaled by factor: {}", scale_factor);
        Ok(format!("Force matrix scaled by factor: {}", scale_factor))
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn flip_force_matrix_horizontal(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("flip_force_matrix_horizontal called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Flip the force matrix horizontally
        simulation.settings.flip_horizontal();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix flipped horizontally");
        Ok("Force matrix flipped horizontally".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn flip_force_matrix_vertical(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("flip_force_matrix_vertical called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Flip the force matrix vertically
        simulation.settings.flip_vertical();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix flipped vertically");
        Ok("Force matrix flipped vertically".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn rotate_force_matrix_clockwise(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("rotate_force_matrix_clockwise called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Rotate the force matrix clockwise
        simulation.settings.rotate_clockwise();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix rotated clockwise");
        Ok("Force matrix rotated clockwise".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn rotate_force_matrix_counterclockwise(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("rotate_force_matrix_counterclockwise called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Rotate the force matrix counterclockwise
        simulation.settings.rotate_counterclockwise();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix rotated counterclockwise");
        Ok("Force matrix rotated counterclockwise".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn shift_force_matrix_left(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("shift_force_matrix_left called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Shift the force matrix left
        simulation.settings.shift_left();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix shifted left");
        Ok("Force matrix shifted left".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn shift_force_matrix_right(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("shift_force_matrix_right called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Shift the force matrix right
        simulation.settings.shift_right();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix shifted right");
        Ok("Force matrix shifted right".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]
pub async fn shift_force_matrix_up(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("shift_force_matrix_up called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Shift the force matrix up
        simulation.settings.shift_up();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix shifted up");
        Ok("Force matrix shifted up".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}

#[tauri::command]  
pub async fn shift_force_matrix_down(
    manager: State<'_, Arc<tokio::sync::Mutex<SimulationManager>>>,
    gpu_context: State<'_, Arc<tokio::sync::Mutex<crate::GpuContext>>>,
) -> Result<String, String> {
    tracing::info!("shift_force_matrix_down called");
    let mut sim_manager = manager.lock().await;
    let gpu_ctx = gpu_context.lock().await;

    if let Some(SimulationType::ParticleLife(simulation)) = &mut sim_manager.current_simulation {
        // Shift the force matrix down
        simulation.settings.shift_down();
        
        // Update the force matrix buffer on GPU
        let force_matrix_data = crate::simulations::particle_life::simulation::ParticleLifeModel::flatten_force_matrix(&simulation.settings.force_matrix);
        simulation.force_matrix_buffer = gpu_ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Force Matrix Buffer"),
            contents: bytemuck::cast_slice(&force_matrix_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Recreate bind groups that use this buffer
        simulation.recreate_bind_groups_with_force_matrix(&gpu_ctx.device);

        tracing::info!("Force matrix shifted down");
        Ok("Force matrix shifted down".to_string())
    } else {
        Err("No Particle Life simulation running".to_string())
    }
}
