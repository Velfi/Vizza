use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use wgpu::{Device, Queue, SurfaceConfiguration};

use crate::simulations::shared::{coordinates::ScreenCoords, LutHandler, LutManager, SimulationLutManager};
use crate::simulations::traits::{Simulation, SimulationType};
use crate::simulation::preset_manager::SimulationPresetManager;
use crate::simulations::slime_mold::{SlimeMoldModel, settings::Settings as SlimeMoldSettings};
use crate::simulations::gray_scott::{GrayScottModel, settings::Settings as GrayScottSettings};

pub struct SimulationManager {
    pub current_simulation: Option<SimulationType>,
    pub preset_manager: SimulationPresetManager,
    pub lut_manager: LutManager,
    pub simulation_lut_manager: SimulationLutManager,
    pub render_loop_running: Arc<AtomicBool>,
    pub fps_limit_enabled: Arc<AtomicBool>,
    pub fps_limit: Arc<AtomicU32>,
    pub start_time: Instant,
}

impl SimulationManager {
    pub fn new() -> Self {
        Self {
            current_simulation: None,
            preset_manager: SimulationPresetManager::new(),
            lut_manager: LutManager::new(),
            simulation_lut_manager: SimulationLutManager::new(),
            render_loop_running: Arc::new(AtomicBool::new(false)),
            fps_limit_enabled: Arc::new(AtomicBool::new(false)),
            fps_limit: Arc::new(AtomicU32::new(60)),
            start_time: Instant::now(),
        }
    }

    pub fn get_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    /// Get immutable reference to current simulation
    pub fn simulation(&self) -> Option<&SimulationType> {
        self.current_simulation.as_ref()
    }

    /// Get mutable reference to current simulation
    pub fn simulation_mut(&mut self) -> Option<&mut SimulationType> {
        self.current_simulation.as_mut()
    }

    pub async fn start_simulation(
        &mut self,
        simulation_type: String,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.render_frame(device, queue, surface_view)?;
        }
        Ok(())
    }

    pub fn handle_resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.resize(device, queue, new_config)?;
        }
        Ok(())
    }

    pub fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        is_seeding: bool,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.handle_mouse_interaction(world_x, world_y, is_seeding, queue)?;
        }
        Ok(())
    }

    /// Handle mouse interaction using screen coordinates (physical pixels)
    pub fn handle_mouse_interaction_screen_coords(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        is_seeding: bool,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(simulation) => {
                    let camera = &simulation.renderer.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);
                    // Convert world to NDC relative to camera
                    let ndc_x = (world.x - camera.position[0]) * camera.zoom;
                    let ndc_y = (world.y - camera.position[1]) * camera.zoom;
                    // Convert NDC [-1,1] to texture coordinates [0,1]
                    let texture_x = (ndc_x + 1.0) * 0.5;
                    let texture_y = (ndc_y + 1.0) * 0.5;
                    simulation.handle_mouse_interaction(texture_x, texture_y, is_seeding, queue)?;
                }
                _ => (),
            }
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.current_simulation.is_some()
    }

    pub fn get_status(&self) -> String {
        if self.current_simulation.is_some() {
            "Simulation Running"
        } else {
            "No Simulation Running"
        }.to_string()
    }

    pub fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.update_setting(setting_name, value.clone(), device, queue)?;
        }
        Ok(())
    }

    pub async fn update_agent_count(
        &mut self,
        count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.update_agent_count(count, device, queue, surface_config).await?;
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
        match simulation_type {
            "slime_mold" => {
                let presets = self.preset_manager.slime_mold_preset_manager().get_preset_names();
                tracing::info!("Slime mold presets: {:?}", presets);
                presets
            }
            "gray_scott" => {
                let presets = self.preset_manager.gray_scott_preset_manager().get_preset_names();
                tracing::info!("Gray-Scott presets: {:?}", presets);
                presets
            }
            _ => {
                tracing::warn!("Unknown simulation type: {}", simulation_type);
                vec![]
            }
        }
    }

    pub fn apply_preset(
        &mut self,
        preset_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            self.preset_manager.apply_preset(simulation, preset_name, queue)?;
        }
        Ok(())
    }

    pub fn save_preset(
        &self,
        preset_name: &str,
        settings: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &self.current_simulation {
            self.preset_manager.save_preset(simulation, preset_name, settings)?;
        }
        Ok(())
    }

    pub fn delete_preset(&mut self, preset_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &self.current_simulation {
            self.preset_manager.delete_preset(simulation, preset_name)?;
        }
        Ok(())
    }

    pub fn get_current_settings(&self) -> Option<serde_json::Value> {
        if let Some(simulation) = &self.current_simulation {
            Some(simulation.get_settings())
        } else {
            None
        }
    }

    pub fn get_current_state(&self) -> Option<serde_json::Value> {
        if let Some(simulation) = &self.current_simulation {
            Some(simulation.get_state())
        } else {
            None
        }
    }

    pub fn get_current_agent_count(&self) -> Option<u32> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.get_agent_count(),
                SimulationType::GrayScott(_) => None, // Gray-Scott doesn't have agents
            }
        } else {
            None
        }
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
        self.simulation_lut_manager.get_available_luts(&self.lut_manager)
    }

    pub fn apply_lut(
        &mut self,
        lut_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            self.simulation_lut_manager.apply_lut(simulation, &self.lut_manager, lut_name, queue)?;
        }
        Ok(())
    }

    pub fn reverse_current_lut(
        &mut self,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            self.simulation_lut_manager.reverse_current_lut(simulation, &self.lut_manager, queue)?;
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

        render_loop_running.store(true, Ordering::Relaxed);

        tokio::spawn(async move {
            let mut frame_count = 0u32;
            let mut last_fps_update = Instant::now();

            while render_loop_running.load(Ordering::Relaxed) {
                let frame_start = Instant::now();

                // Render frame (only if simulation is running)
                {
                    let mut sim_manager = manager.lock().await;
                    let gpu_ctx = gpu_context.lock().await;

                    if sim_manager.is_running() {
                        if let Ok(output) = gpu_ctx.get_current_texture() {
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            if sim_manager
                                .render(&gpu_ctx.device, &gpu_ctx.queue, &view)
                                .is_ok()
                            {
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
    pub fn reset_trails(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(SimulationType::SlimeMold(simulation)) = &mut self.current_simulation {
            simulation.reset_trails(queue);
        }
        Ok(())
    }

    pub fn reset_agents(
        &mut self,
        device: &Arc<wgpu::Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(SimulationType::SlimeMold(simulation)) = &mut self.current_simulation {
            simulation.reset_agents(device, queue)?;
        }
        Ok(())
    }

    pub fn reset_simulation(
        &mut self,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(simulation) => simulation.reset(),
                SimulationType::SlimeMold(simulation) => simulation.reset_trails(queue),
            }
        }
        Ok(())
    }

    pub fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.randomize_settings(device, queue)?;
        }
        Ok(())
    }

    // Note: seed_random_noise is not in the Simulation trait, so we'll implement it per simulation type
    pub fn seed_random_noise(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.seed_random_noise(device, queue)?,
                SimulationType::GrayScott(simulation) => simulation.seed_random_noise(device, queue)?,
            }
        }
        Ok(())
    }

    // Camera control methods
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        tracing::debug!("SimulationManager pan_camera: delta=({:.2}, {:.2})", delta_x, delta_y);
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
        tracing::debug!("SimulationManager zoom_camera_to_cursor: delta={:.2}, cursor=({:.2}, {:.2})", delta, cursor_x, cursor_y);
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
                SimulationType::GrayScott(simulation) => Some(simulation.renderer.camera.get_state()),
            }
        } else {
            None
        }
    }

    /// Convert screen coordinates to world coordinates using the active camera
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> Option<(f32, f32)> {
        let screen = ScreenCoords::new(screen_x, screen_y);
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    let world = simulation.camera.screen_to_world(screen);
                    Some((world.x, world.y))
                }
                SimulationType::GrayScott(simulation) => {
                    let world = simulation.renderer.camera.screen_to_world(screen);
                    Some((world.x, world.y))
                }
            }
        } else {
            None
        }
    }
} 