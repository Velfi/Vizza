use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use wgpu::{Device, Queue, SurfaceConfiguration};

use crate::error::{AppError, AppResult};
use crate::simulation::preset_manager::SimulationPresetManager;
use crate::simulations::gray_scott::{settings::Settings as GrayScottSettings, GrayScottModel};
use crate::simulations::particle_life::{
    settings::Settings as ParticleLifeSettings, simulation::ColorMode, ParticleLifeModel,
};
use crate::simulations::shared::LutData;
use crate::simulations::shared::{coordinates::ScreenCoords, LutManager, SimulationLutManager};
use crate::simulations::slime_mold::{settings::Settings as SlimeMoldSettings, SlimeMoldModel};
use crate::simulations::traits::{Simulation, SimulationType};

pub struct SimulationManager {
    pub current_simulation: Option<SimulationType>,
    pub preset_manager: SimulationPresetManager,
    pub lut_manager: LutManager,
    pub simulation_lut_manager: SimulationLutManager,
    pub render_loop_running: Arc<AtomicBool>,
    pub fps_limit_enabled: Arc<AtomicBool>,
    pub fps_limit: Arc<AtomicU32>,
    pub is_paused: Arc<AtomicBool>,
}

impl SimulationManager {
    pub fn new() -> Self {
        // Simulations start paused to prevent race conditions between initialization
        // and render loop startup. They are automatically unpaused after successful
        // initialization to ensure all GPU resources and state are ready.
        Self {
            current_simulation: None,
            preset_manager: SimulationPresetManager::new(),
            lut_manager: LutManager::new(),
            simulation_lut_manager: SimulationLutManager::new(),
            render_loop_running: Arc::new(AtomicBool::new(false)),
            fps_limit_enabled: Arc::new(AtomicBool::new(false)),
            fps_limit: Arc::new(AtomicU32::new(60)),
            is_paused: Arc::new(AtomicBool::new(true)), // Start paused to avoid race condition
        }
    }

    /// Get immutable reference to current simulation
    pub fn simulation(&self) -> Option<&SimulationType> {
        self.current_simulation.as_ref()
    }

    pub async fn start_simulation(
        &mut self,
        simulation_type: String,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
    ) -> AppResult<()> {
        match simulation_type.as_str() {
            "slime_mold" => {
                // Initialize slime mold simulation
                let settings = SlimeMoldSettings::default();
                let simulation = SlimeMoldModel::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    10_000_000,
                    settings,
                    &self.lut_manager,
                )?;

                self.current_simulation = Some(SimulationType::SlimeMold(simulation));

                // Automatically unpause after successful initialization
                self.resume();

                Ok(())
            }
            "gray_scott" => {
                // Initialize Gray-Scott simulation
                let settings = GrayScottSettings::default();

                let simulation = GrayScottModel::new(
                    device,
                    queue,
                    surface_config,
                    surface_config.width,
                    surface_config.height,
                    settings,
                    &self.lut_manager,
                )?;

                self.current_simulation = Some(SimulationType::GrayScott(simulation));

                // Automatically unpause after successful initialization
                self.resume();

                Ok(())
            }
            "particle_life" => {
                // Initialize Particle Life simulation
                let settings = ParticleLifeSettings::default();
                let simulation = ParticleLifeModel::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    15000, // Default particle count
                    settings,
                    &self.lut_manager,
                    ColorMode::Lut,
                )?;

                self.current_simulation = Some(SimulationType::ParticleLife(simulation));

                // Apply the "Default" preset to ensure consistent initial state
                if let Some(simulation) = &mut self.current_simulation {
                    self.preset_manager
                        .apply_preset(simulation, "Default", device, queue)
                        .map_err(|e| format!("Failed to apply Default preset: {}", e))?;
                    tracing::info!("Applied Default preset to Particle Life simulation");
                }

                // Automatically unpause after successful initialization
                self.resume();

                Ok(())
            }
            "ecosystem" => {
                // Initialize Ecosystem simulation
                let settings = crate::simulations::ecosystem::settings::Settings::default();
                let simulation = crate::simulations::ecosystem::simulation::EcosystemModel::new(
                    device,
                    queue,
                    surface_config,
                    settings.agent_count,
                    settings,
                    &self.lut_manager,
                )?;

                self.current_simulation = Some(SimulationType::Ecosystem(simulation));

                // Automatically unpause after successful initialization
                self.resume();

                Ok(())
            }
            _ => Err("Unknown simulation type".into()),
        }
    }

    pub fn stop_simulation(&mut self) {
        self.current_simulation = None;
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &wgpu::TextureView,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.render_frame(device, queue, surface_view)?;
        }
        Ok(())
    }

    pub fn render_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &wgpu::TextureView,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            // Render the current frame without updating simulation state
            simulation.render_frame_static(device, queue, surface_view)?;
        }
        Ok(())
    }

    pub fn handle_resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.resize(device, queue, new_config)?;
        }
        Ok(())
    }

    pub fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.handle_mouse_interaction(world_x, world_y, mouse_button, queue)?;
        }
        Ok(())
    }

    /// Handle mouse interaction using screen coordinates (physical pixels)
    pub fn handle_mouse_interaction_screen_coords(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        mouse_button: u32,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(simulation) => {
                    let camera = &simulation.renderer.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);

                    // Convert world coordinates [-1,1] to texture coordinates [0,1]
                    // Gray-Scott simulation expects texture coordinates in [0,1] range
                    let texture_x = (world.x + 1.0) * 0.5;
                    let texture_y = (world.y + 1.0) * 0.5;

                    tracing::debug!(
                        "Gray-Scott mouse interaction: screen=({:.2}, {:.2}), world=({:.3}, {:.3}), texture=({:.3}, {:.3}), button={}",
                        screen_x, screen_y, world.x, world.y, texture_x, texture_y, mouse_button
                    );

                    simulation.handle_mouse_interaction(
                        texture_x,
                        texture_y,
                        mouse_button,
                        queue,
                    )?;
                }
                SimulationType::ParticleLife(simulation) => {
                    // Handle mouse release special case before camera transformation
                    if screen_x == -9999.0 && screen_y == -9999.0 {
                        println!("🌍 ParticleLife mouse: RELEASE");
                        simulation.handle_mouse_interaction(
                            -9999.0,
                            -9999.0,
                            mouse_button,
                            queue,
                        )?;
                    } else {
                        let camera = &simulation.camera;
                        let screen = ScreenCoords::new(screen_x, screen_y);
                        let world = camera.screen_to_world(screen);

                        // Particles now live in [-1,1] world space, so use world coordinates directly
                        let particle_x = world.x;
                        let particle_y = world.y;

                        simulation.handle_mouse_interaction(
                            particle_x,
                            particle_y,
                            mouse_button,
                            queue,
                        )?;
                    }
                }
                SimulationType::SlimeMold(simulation) => {
                    // Handle mouse release special case before camera transformation
                    if screen_x == -9999.0 && screen_y == -9999.0 {
                        println!("🌍 SlimeMold mouse: RELEASE");
                        simulation.handle_mouse_interaction(
                            -9999.0,
                            -9999.0,
                            mouse_button,
                            queue,
                        )?;
                    } else {
                        let camera = &simulation.camera;
                        let screen = ScreenCoords::new(screen_x, screen_y);
                        let world = camera.screen_to_world(screen);
                        let world_x = world.x;
                        let world_y = world.y;
                        simulation.handle_mouse_interaction(
                            world_x,
                            world_y,
                            mouse_button,
                            queue,
                        )?;
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.current_simulation.is_some()
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::Relaxed);
    }

    pub fn get_status(&self) -> String {
        if self.current_simulation.is_some() {
            "Simulation Running"
        } else {
            "No Simulation Running"
        }
        .to_string()
    }

    pub fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.update_setting(setting_name, value.clone(), device, queue)?;
        }
        Ok(())
    }

    // Preset management methods
    pub fn get_available_presets(&self) -> Vec<String> {
        if let Some(simulation) = &self.current_simulation {
            let presets = self.preset_manager.get_available_presets(simulation);
            tracing::info!("Available presets for simulation: {:?}", presets);
            presets
        } else {
            tracing::warn!("No simulation running, returning empty preset list");
            vec![]
        }
    }

    pub fn get_presets_for_simulation_type(&self, simulation_type: &str) -> Vec<String> {
        if let Some(manager) = self.preset_manager.get_manager(simulation_type) {
            let presets = manager.get_preset_names();
            tracing::info!("{} presets: {:?}", simulation_type, presets);
            presets
        } else {
            tracing::warn!("Unknown simulation type: {}", simulation_type);
            vec![]
        }
    }

    pub fn apply_preset(
        &mut self,
        preset_name: &str,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            self.preset_manager
                .apply_preset(simulation, preset_name, device, queue)
                .map_err(AppError::Preset)?;
            simulation.reset_runtime_state(device, queue)?;
        }
        Ok(())
    }

    pub fn save_preset(&self, preset_name: &str, settings: &serde_json::Value) -> AppResult<()> {
        if let Some(simulation) = &self.current_simulation {
            self.preset_manager
                .save_preset(simulation, preset_name, settings)?;
        }
        Ok(())
    }

    pub fn delete_preset(&mut self, preset_name: &str) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            self.preset_manager.delete_preset(simulation, preset_name)?;
        }
        Ok(())
    }

    pub fn get_current_settings(&self) -> Option<serde_json::Value> {
        self.current_simulation
            .as_ref()
            .map(|simulation| simulation.get_settings())
    }

    pub fn get_current_state(&self) -> Option<serde_json::Value> {
        self.current_simulation
            .as_ref()
            .map(|simulation| simulation.get_state())
    }

    pub fn toggle_gui(&mut self) {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.toggle_gui();
        }
    }

    pub fn is_gui_visible(&self) -> bool {
        if let Some(simulation) = &self.current_simulation {
            simulation.is_gui_visible()
        } else {
            false
        }
    }

    // LUT management methods
    pub fn get_available_luts(&self) -> Vec<String> {
        self.simulation_lut_manager
            .get_available_luts(&self.lut_manager)
    }

    pub fn apply_lut(
        &mut self,
        lut_name: &str,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    // For slime mold, load the LUT data and apply it directly
                    let mut lut_data = self.lut_manager.get(lut_name).map_err(|e| {
                        AppError::Simulation(
                            format!("Failed to load LUT '{}': {}", lut_name, e).into(),
                        )
                    })?;

                    if simulation.lut_reversed {
                        lut_data.reverse();
                    }

                    simulation.update_lut(&lut_data, queue);
                    simulation.current_lut_name = lut_name.to_string();

                    tracing::info!("LUT '{}' applied to slime mold simulation", lut_name);
                }
                SimulationType::GrayScott(simulation) => {
                    // For Gray-Scott, load the LUT data and apply it to the renderer
                    let mut lut_data = self.lut_manager.get(lut_name).map_err(|e| {
                        AppError::Simulation(
                            format!("Failed to load LUT '{}': {}", lut_name, e).into(),
                        )
                    })?;

                    if simulation.lut_reversed {
                        lut_data.reverse();
                    }

                    simulation.renderer.update_lut(&lut_data, queue);
                    simulation.current_lut_name = lut_name.to_string();

                    tracing::info!("LUT '{}' applied to Gray-Scott simulation", lut_name);
                }
                SimulationType::ParticleLife(simulation) => {
                    // For particle life, use the existing update_setting method
                    simulation.update_setting("lut", serde_json::json!(lut_name), device, queue)?;
                }
                SimulationType::Ecosystem(_simulation) => {
                    // Ecosystem doesn't support LUT changes yet
                    tracing::warn!("LUT changes not yet implemented for ecosystem simulation");
                }
                SimulationType::MainMenu(_) => {
                    // Main menu doesn't support LUT changes
                    tracing::warn!("LUT changes not supported for main menu simulation");
                }
            }
        }
        Ok(())
    }

    pub fn reverse_current_lut(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    // Toggle the reversed flag and reload the LUT
                    simulation.lut_reversed = !simulation.lut_reversed;
                    let mut lut_data =
                        self.lut_manager
                            .get(&simulation.current_lut_name)
                            .map_err(|e| {
                                AppError::Simulation(
                                    format!(
                                        "Failed to load LUT '{}': {}",
                                        simulation.current_lut_name, e
                                    )
                                    .into(),
                                )
                            })?;

                    if simulation.lut_reversed {
                        lut_data.reverse();
                    }

                    simulation.update_lut(&lut_data, queue);
                    tracing::info!("LUT reversed for slime mold simulation");
                }
                SimulationType::GrayScott(simulation) => {
                    // Toggle the reversed flag and reload the LUT
                    simulation.lut_reversed = !simulation.lut_reversed;
                    let mut lut_data =
                        self.lut_manager
                            .get(&simulation.current_lut_name)
                            .map_err(|e| {
                                AppError::Simulation(
                                    format!(
                                        "Failed to load LUT '{}': {}",
                                        simulation.current_lut_name, e
                                    )
                                    .into(),
                                )
                            })?;

                    if simulation.lut_reversed {
                        lut_data.reverse();
                    }

                    simulation.renderer.update_lut(&lut_data, queue);
                    tracing::info!("LUT reversed for Gray-Scott simulation");
                }
                SimulationType::ParticleLife(simulation) => {
                    // For particle life, we need to update the LUT with reversed flag
                    let current_reversed = simulation.state.lut_reversed;
                    let color_mode = simulation.state.color_mode;
                    let current_lut_name = simulation.state.current_lut_name.clone();
                    let lut_manager = &self.lut_manager;
                    simulation.update_lut(
                        device,
                        queue,
                        lut_manager,
                        color_mode,
                        Some(&current_lut_name),
                        !current_reversed,
                    )?;
                }
                SimulationType::Ecosystem(_simulation) => {
                    // Ecosystem doesn't support LUT changes yet
                    tracing::warn!("LUT reversal not yet implemented for ecosystem simulation");
                }
                SimulationType::MainMenu(_) => {
                    // Main menu doesn't support LUT changes
                    tracing::warn!("LUT reversal not supported for main menu simulation");
                }
            }
        }
        Ok(())
    }

    /// Apply a custom LUT to any running simulation
    pub fn apply_custom_lut(
        &mut self,
        lut_data: &LutData,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    simulation.update_lut(lut_data, queue);
                    tracing::info!("Custom LUT applied to slime mold simulation");
                }
                SimulationType::GrayScott(simulation) => {
                    simulation.renderer.update_lut(lut_data, queue);
                    tracing::info!("Custom LUT applied to Gray-Scott simulation");
                }
                SimulationType::ParticleLife(simulation) => {
                    // For particle life, we need to temporarily store the custom LUT in the manager
                    // since the particle life update_lut method expects a LUT name, not LutData directly
                    let temp_lut_name = "gradient_preview";

                    // Store the custom LUT temporarily in the LUT manager
                    self.lut_manager
                        .save_custom(temp_lut_name, lut_data)
                        .map_err(|e| {
                            AppError::Simulation(
                                format!("Failed to store temporary LUT: {}", e).into(),
                            )
                        })?;

                    let color_mode = simulation.state.color_mode;
                    let lut_reversed = simulation.state.lut_reversed;

                    simulation.update_lut(
                        device,
                        queue,
                        &self.lut_manager,
                        color_mode,
                        Some(temp_lut_name),
                        lut_reversed,
                    )?;
                    tracing::info!("Custom LUT applied to particle life simulation");
                }
                SimulationType::Ecosystem(_simulation) => {
                    // Ecosystem doesn't support LUT changes yet
                    tracing::warn!("Custom LUT not yet implemented for ecosystem simulation");
                }
                SimulationType::MainMenu(_) => {
                    // Main menu doesn't support LUT changes
                    tracing::warn!("Custom LUT not supported for main menu simulation");
                }
            }
        } else {
            return Err(AppError::Simulation(
                "No simulation is currently running".into(),
            ));
        }
        Ok(())
    }

    // Render loop management
    pub fn start_render_loop(
        &self,
        app_handle: AppHandle,
        gpu_context: Arc<tokio::sync::Mutex<crate::GpuContext>>,
        manager: Arc<tokio::sync::Mutex<SimulationManager>>,
    ) {
        let render_loop_running = self.render_loop_running.clone();
        let fps_limit_enabled = self.fps_limit_enabled.clone();
        let fps_limit = self.fps_limit.clone();
        let is_paused = self.is_paused.clone();

        render_loop_running.store(true, Ordering::Relaxed);

        tokio::spawn(async move {
            let mut frame_count = 0u32;
            let mut last_fps_update = Instant::now();

            while render_loop_running.load(Ordering::Relaxed) {
                let frame_start = Instant::now();

                // Render frame (continue rendering even when paused to show camera changes)
                {
                    let mut sim_manager = manager.lock().await;
                    let gpu_ctx = gpu_context.lock().await;

                    if sim_manager.is_running() {
                        if let Ok(output) = gpu_ctx.get_current_texture() {
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let render_result = if is_paused.load(Ordering::Relaxed) {
                                // When paused, render without updating simulation state
                                sim_manager.render_paused(&gpu_ctx.device, &gpu_ctx.queue, &view)
                            } else {
                                // When running, render normally with simulation updates
                                sim_manager.render(&gpu_ctx.device, &gpu_ctx.queue, &view)
                            };

                            if render_result.is_ok() {
                                output.present();
                            }
                        }
                    } else {
                        // Stop the render loop if simulation is no longer running
                        break;
                    }
                }

                frame_count += 1;

                // Update FPS every second
                if last_fps_update.elapsed() >= Duration::from_secs(1) {
                    let fps = (frame_count as f64 / last_fps_update.elapsed().as_secs_f64()) as u32;

                    // Emit FPS update to frontend
                    if let Err(e) = app_handle.emit("fps-update", fps) {
                        tracing::warn!("Failed to emit FPS update: {}", e);
                    }

                    frame_count = 0;
                    last_fps_update = Instant::now();
                }

                // Handle FPS limiting
                if fps_limit_enabled.load(Ordering::Relaxed) {
                    let target_fps = fps_limit.load(Ordering::Relaxed);
                    if target_fps > 0 {
                        let target_frame_time =
                            Duration::from_nanos(1_000_000_000 / target_fps as u64);
                        let frame_time = frame_start.elapsed();

                        if frame_time < target_frame_time {
                            tokio::time::sleep(target_frame_time - frame_time).await;
                        }
                    }
                }
            }
        });
    }

    pub fn stop_render_loop(&self) {
        self.render_loop_running.store(false, Ordering::Relaxed);
    }

    pub fn set_fps_limit(&self, enabled: bool, limit: u32) {
        self.fps_limit_enabled.store(enabled, Ordering::Relaxed);
        self.fps_limit.store(limit, Ordering::Relaxed);
    }

    // Reset methods
    pub fn reset_trails(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.reset_runtime_state(device, queue)?;
        }
        Ok(())
    }

    pub fn reset_agents(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(sim) => {
                    // For slime mold, call the specific reset_agents method that repositions agents randomly
                    sim.reset_agents(device, queue)
                        .map_err(AppError::Simulation)?;
                }
                _ => {
                    // For other simulations, use the generic reset_runtime_state
                    simulation.reset_runtime_state(device, queue)?;
                }
            }
        }
        Ok(())
    }

    pub fn reset_simulation(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(sim) => {
                    sim.reset();
                }
                SimulationType::ParticleLife(sim) => {
                    sim.reset_particles_gpu(device, queue)?;
                }
                SimulationType::Ecosystem(_sim) => {
                    simulation.reset_runtime_state(device, queue)?;
                }
                _ => {
                    simulation.reset_runtime_state(device, queue)?;
                }
            }
        }
        Ok(())
    }

    pub fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.randomize_settings(device, queue)?;
        }
        Ok(())
    }

    // Note: seed_random_noise is Gray-Scott specific functionality
    pub fn seed_random_noise(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(sim) => {
                    sim.seed_random_noise(device, queue)
                        .map_err(AppError::Simulation)?;
                }
                _ => {
                    // Seed random noise is only supported for Gray-Scott simulation
                    tracing::warn!("Seed random noise is only supported for Gray-Scott simulation");
                }
            }
        }
        Ok(())
    }

    // Camera control methods
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        tracing::debug!(
            "SimulationManager pan_camera: delta=({:.2}, {:.2})",
            delta_x,
            delta_y
        );
        if let Some(simulation) = &mut self.current_simulation {
            simulation.pan_camera(delta_x, delta_y);
        }
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        tracing::debug!("SimulationManager zoom_camera: delta={:.2}", delta);
        if let Some(simulation) = &mut self.current_simulation {
            simulation.zoom_camera(delta);
        }
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        tracing::debug!(
            "SimulationManager zoom_camera_to_cursor: delta={:.2}, cursor=({:.2}, {:.2})",
            delta,
            cursor_x,
            cursor_y
        );
        if let Some(simulation) = &mut self.current_simulation {
            simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y);
        }
    }

    pub fn reset_camera(&mut self) {
        tracing::debug!("SimulationManager reset_camera");
        if let Some(simulation) = &mut self.current_simulation {
            simulation.reset_camera();
        }
    }

    pub fn get_camera_state(&self) -> Option<serde_json::Value> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => Some(simulation.camera.get_state()),
                SimulationType::GrayScott(simulation) => {
                    Some(simulation.renderer.camera.get_state())
                }
                SimulationType::ParticleLife(simulation) => Some(simulation.get_camera_state()),
                SimulationType::Ecosystem(simulation) => Some(simulation.get_camera_state()),
                SimulationType::MainMenu(_) => Some(serde_json::json!({})), // No camera for main menu background
            }
        } else {
            None
        }
    }

    /// Set the camera smoothing factor for the active simulation
    pub fn set_camera_smoothing(&mut self, smoothing_factor: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    simulation.camera.set_smoothing_factor(smoothing_factor)
                }
                SimulationType::GrayScott(simulation) => simulation
                    .renderer
                    .camera
                    .set_smoothing_factor(smoothing_factor),
                SimulationType::ParticleLife(simulation) => {
                    simulation.camera.set_smoothing_factor(smoothing_factor)
                }
                SimulationType::Ecosystem(simulation) => {
                    simulation.camera.set_smoothing_factor(smoothing_factor)
                }
                SimulationType::MainMenu(_) => {} // No camera for main menu background
            }
        }
    }

    /// Set the camera sensitivity for the active simulation
    pub fn set_camera_sensitivity(&mut self, sensitivity: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    simulation.camera.set_sensitivity(sensitivity)
                }
                SimulationType::GrayScott(simulation) => {
                    simulation.renderer.camera.set_sensitivity(sensitivity)
                }
                SimulationType::ParticleLife(simulation) => {
                    simulation.camera.set_sensitivity(sensitivity)
                }
                SimulationType::Ecosystem(simulation) => {
                    simulation.camera.set_sensitivity(sensitivity)
                }
                SimulationType::MainMenu(_) => {} // No camera for main menu background
            }
        }
    }
}
