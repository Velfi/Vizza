use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use wgpu::{Device, Queue, SurfaceConfiguration};

use crate::simulations::shared::{LutManager, ScreenCoords};
use crate::simulations::slime_mold::{
    self,
    presets::init_preset_manager as init_slime_mold_preset_manager,
};
use crate::simulations::gray_scott::{
    self,
    presets::init_preset_manager as init_gray_scott_preset_manager,
    GrayScottModel,
};

pub struct SimulationManager {
    pub slime_mold_state: Option<slime_mold::SlimeMoldSimulation>,
    pub gray_scott_state: Option<GrayScottModel>,
    pub slime_mold_preset_manager: slime_mold::presets::PresetManager,
    pub gray_scott_preset_manager: gray_scott::presets::PresetManager,
    pub lut_manager: LutManager,
    pub render_loop_running: Arc<AtomicBool>,
    pub fps_limit_enabled: Arc<AtomicBool>,
    pub fps_limit: Arc<AtomicU32>,
}

impl SimulationManager {
    pub fn new() -> Self {
        Self {
            slime_mold_state: None,
            gray_scott_state: None,
            slime_mold_preset_manager: init_slime_mold_preset_manager(),
            gray_scott_preset_manager: init_gray_scott_preset_manager(),
            lut_manager: LutManager::new(),
            render_loop_running: Arc::new(AtomicBool::new(false)),
            fps_limit_enabled: Arc::new(AtomicBool::new(false)),
            fps_limit: Arc::new(AtomicU32::new(60)),
        }
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
                let settings = slime_mold::settings::Settings::default();
                let available_luts = self.lut_manager.get_available_luts();
                let default_lut_name = "MATPLOTLIB_bone_r";
                let current_lut_index = available_luts
                    .iter()
                    .position(|name| name == default_lut_name)
                    .unwrap_or(0);

                let simulation = slime_mold::SlimeMoldSimulation::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    10_000_000,
                    settings,
                    &self.lut_manager,
                    &available_luts,
                    current_lut_index, // current_lut_index
                    false, // lut_reversed
                )?;

                self.slime_mold_state = Some(simulation);
                Ok(())
            }
            "gray_scott" => {
                // Initialize Gray-Scott simulation
                let settings = crate::simulations::gray_scott::settings::Settings::default();
                let available_luts = self.lut_manager.get_available_luts();
                let default_lut_name = "MATPLOTLIB_bone_r";
                let current_lut_index = available_luts
                    .iter()
                    .position(|name| name == default_lut_name)
                    .unwrap_or(0);

                let simulation = GrayScottModel::new(
                    device,
                    queue,
                    surface_config,
                    surface_config.width,
                    surface_config.height,
                    settings,
                    &self.lut_manager,
                    &available_luts,
                    current_lut_index,
                    false, // lut_reversed
                )?;

                self.gray_scott_state = Some(simulation);
                Ok(())
            }
            _ => Err("Unknown simulation type".into()),
        }
    }

    pub fn stop_simulation(&mut self) {
        self.slime_mold_state = None;
        self.gray_scott_state = None;
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.render_frame(device, queue, surface_view)?;
        }
        if let Some(simulation) = &mut self.gray_scott_state {
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
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.resize(device, queue, new_config)?;
        }
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.resize(device, queue, new_config)?;
        }
        Ok(())
    }

    pub fn handle_mouse_interaction(
        &mut self,
        x: f32,
        y: f32,
        is_seeding: bool,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.handle_mouse_interaction(x, y, is_seeding, queue)?;
        }
        Ok(())
    }

    pub fn update_cursor_position(
        &mut self,
        x: f32,
        y: f32,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.update_cursor_position(x, y, queue)?;
        }
        Ok(())
    }

    pub fn update_cursor_position_screen(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            // Convert screen coordinates to world coordinates using the backend camera
            let screen_coords = ScreenCoords::new(screen_x, screen_y);
            let world_coords = simulation.renderer.camera.screen_to_world_typed(screen_coords);
            
            // Debug logging
            tracing::debug!(
                "Cursor transform: screen=({:.2}, {:.2}) -> world=({:.4}, {:.4}), camera_pos=({:.4}, {:.4}), zoom={:.4}, aspect={:.4}",
                screen_x, screen_y, world_coords.x, world_coords.y,
                simulation.renderer.camera.position[0], simulation.renderer.camera.position[1],
                simulation.renderer.camera.zoom, simulation.renderer.camera.uniform_data().aspect_ratio
            );
            
            simulation.update_cursor_position(world_coords.x, world_coords.y, queue)?;
        }
        Ok(())
    }

    pub fn seed_random_noise(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.seed_random_noise(device, queue);
        }
        Ok(())
    }

    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.pan_camera(delta_x, delta_y);
        }
        Ok(())
    }

    pub fn zoom_camera(&mut self, delta: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.zoom_camera(delta);
        }
        Ok(())
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y);
        }
        Ok(())
    }

    pub fn stop_camera_pan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.stop_camera_pan();
        }
        Ok(())
    }

    pub fn reset_camera(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.reset_camera();
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.slime_mold_state.is_some() || self.gray_scott_state.is_some()
    }

    pub fn get_status(&self) -> String {
        if self.slime_mold_state.is_some() {
            "Slime Mold Simulation Running".to_string()
        } else if self.gray_scott_state.is_some() {
            "Gray-Scott Simulation Running".to_string()
        } else {
            "No Simulation Running".to_string()
        }
    }

    pub fn update_setting(
        &mut self,
        setting_name: &str,
        value: serde_json::Value,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.update_setting(setting_name, value.clone(), queue)?;
        }
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.update_setting(setting_name, value, queue)?;
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
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation
                .update_agent_count(count, device, queue, surface_config)
                .await?;
        }
        Ok(())
    }

    // Preset management methods
    pub fn get_available_presets(&self) -> Vec<String> {
        if self.slime_mold_state.is_some() {
            let presets = self.slime_mold_preset_manager.get_preset_names();
            tracing::info!("Slime mold presets available: {:?}", presets);
            presets
        } else if self.gray_scott_state.is_some() {
            let presets = self.gray_scott_preset_manager.get_preset_names();
            tracing::info!("Gray-Scott presets available: {:?}", presets);
            presets
        } else {
            tracing::info!("No simulation active, returning empty presets");
            vec![]
        }
    }

    pub fn apply_preset(
        &mut self,
        preset_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            if let Some(preset) = self.slime_mold_preset_manager.get_preset(preset_name) {
                simulation.update_settings(preset.settings.clone(), queue);
                Ok(())
            } else {
                Err(format!("Slime mold preset '{}' not found", preset_name).into())
            }
        } else if let Some(simulation) = &mut self.gray_scott_state {
            tracing::info!("Trying to apply Gray-Scott preset: {}", preset_name);
            if let Some(preset) = self.gray_scott_preset_manager.get_preset(preset_name) {
                tracing::info!("Found preset, applying settings: {:?}", preset.settings);
                simulation.update_settings(preset.settings.clone(), queue);
                Ok(())
            } else {
                tracing::error!("Gray-Scott preset '{}' not found", preset_name);
                Err(format!("Gray-Scott preset '{}' not found", preset_name).into())
            }
        } else {
            Err("No active simulation to apply preset to".into())
        }
    }

    pub fn apply_preset_settings(
        &mut self,
        settings: slime_mold::settings::Settings,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.update_settings(settings, queue);
        }
        Ok(())
    }

    pub fn save_preset(
        &self,
        preset_name: &str,
        settings: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.slime_mold_state.is_some() {
            let slime_settings: slime_mold::settings::Settings = serde_json::from_value(settings.clone())?;
            self.slime_mold_preset_manager.save_user_preset(preset_name, &slime_settings)
        } else if self.gray_scott_state.is_some() {
            let gray_scott_settings: gray_scott::settings::Settings = serde_json::from_value(settings.clone())?;
            self.gray_scott_preset_manager.save_user_preset(preset_name, &gray_scott_settings)
        } else {
            Err("No active simulation to save preset from".into())
        }
    }

    pub fn delete_preset(&mut self, preset_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.slime_mold_state.is_some() {
            self.slime_mold_preset_manager.delete_user_preset(preset_name)?;
        } else if self.gray_scott_state.is_some() {
            self.gray_scott_preset_manager.delete_user_preset(preset_name)?;
        }
        Ok(())
    }

    pub fn get_current_settings(&self) -> Option<serde_json::Value> {
        if let Some(sim) = &self.slime_mold_state {
            serde_json::to_value(&sim.settings).ok()
        } else if let Some(sim) = &self.gray_scott_state {
            serde_json::to_value(&sim.settings).ok()
        } else {
            None
        }
    }

    pub fn get_current_agent_count(&self) -> Option<u32> {
        self.slime_mold_state
            .as_ref()
            .map(|sim| sim.agent_count as u32)
    }

    // LUT management methods
    pub fn get_available_luts(&self) -> Vec<String> {
        self.lut_manager.get_available_luts()
    }

    pub fn apply_lut(
        &mut self,
        lut_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let lut_data = self.lut_manager.load_lut(lut_name)?;
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.update_lut(&lut_data, queue);
        } else if let Some(simulation) = &mut self.gray_scott_state {
            simulation.update_lut(&lut_data, queue);
        }
        Ok(())
    }

    pub fn apply_lut_by_index(
        &mut self,
        lut_index: usize,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let available_luts = self.get_available_luts();
        if let Some(lut_name) = available_luts.get(lut_index) {
            // Update the simulation's current LUT index before applying
            if let Some(simulation) = &mut self.slime_mold_state {
                simulation.current_lut_index = lut_index;
                // Reset reversed state when changing LUT
                simulation.lut_reversed = false;
            } else if let Some(simulation) = &mut self.gray_scott_state {
                simulation.current_lut_index = lut_index;
                // Reset reversed state when changing LUT
                simulation.lut_reversed = false;
            }
            self.apply_lut(lut_name, queue)
        } else {
            Err(format!("LUT index {} out of range", lut_index).into())
        }
    }

    pub fn reverse_current_lut(
        &mut self,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            // Toggle the reversed flag
            simulation.lut_reversed = !simulation.lut_reversed;

            // Get the current LUT and apply it (now with reversed flag)
            let available_luts = self.lut_manager.get_available_luts();
            if let Some(lut_name) = available_luts.get(simulation.current_lut_index) {
                let mut lut_data = self.lut_manager.load_lut(lut_name)?;

                // Reverse the LUT data if the flag is set
                if simulation.lut_reversed {
                    lut_data.reverse();
                }

                // Update the GPU with the reversed LUT
                simulation.update_lut(&lut_data, queue);
            }
        } else if let Some(simulation) = &mut self.gray_scott_state {
            // Toggle the reversed flag
            simulation.lut_reversed = !simulation.lut_reversed;

            // Get the current LUT and apply it (now with reversed flag)
            let available_luts = self.lut_manager.get_available_luts();
            if let Some(lut_name) = available_luts.get(simulation.current_lut_index) {
                let mut lut_data = self.lut_manager.load_lut(lut_name)?;

                // Reverse the LUT data if the flag is set
                if simulation.lut_reversed {
                    lut_data.reverse();
                }

                // Update the GPU with the reversed LUT
                simulation.update_lut(&lut_data, queue);
            }
        }
        Ok(())
    }

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

                    if sim_manager.slime_mold_state.is_some()
                        || sim_manager.gray_scott_state.is_some()
                    {
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

    // TODO These should be methods on the slime mold simulation state.
    pub fn reset_trails(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.reset_trails(queue);
        }
        Ok(())
    }

    // TODO These should be methods on the slime mold simulation state.
    pub fn reset_agents(&mut self, device: &Arc<wgpu::Device>, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.reset_agents(device, queue);
        }
        Ok(())
    }

    pub fn reset_simulation(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.gray_scott_state {
            simulation.reset();
        }
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.reset_trails(queue);
        }
        Ok(())
    }

    pub fn randomize_settings(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            // Randomize slime mold settings
            let mut settings = simulation.settings.clone();
            settings.pheromone_decay_rate = rand::random::<f32>() * 10.0;
            settings.pheromone_deposition_rate = rand::random::<f32>() * 100.0 / 100.0;
            settings.pheromone_diffusion_rate = rand::random::<f32>() * 100.0 / 100.0;
            settings.agent_speed_min = rand::random::<f32>() * 500.0;
            settings.agent_speed_max = settings.agent_speed_min + rand::random::<f32>() * (500.0 - settings.agent_speed_min);
            settings.agent_turn_rate = (rand::random::<f32>() * 360.0) * std::f32::consts::PI / 180.0;
            settings.agent_jitter = rand::random::<f32>() * 5.0;
            settings.agent_sensor_angle = (rand::random::<f32>() * 180.0) * std::f32::consts::PI / 180.0;
            settings.agent_sensor_distance = rand::random::<f32>() * 500.0;
            settings.gradient_type = crate::simulations::slime_mold::settings::GradientType::Disabled;
            settings.gradient_strength = 0.5;
            settings.gradient_center_x = 0.5;
            settings.gradient_center_y = 0.5;
            settings.gradient_size = 1.0;
            settings.gradient_angle = 0.0;
            let start = rand::random::<f32>() * 360.0;
            let end = start + rand::random::<f32>() * (360.0 - start);
            settings.agent_possible_starting_headings = start..end;
            
            simulation.update_settings(settings, queue);
        } else if let Some(simulation) = &mut self.gray_scott_state {
            // Randomize Gray-Scott settings
            let mut settings = simulation.settings.clone();
            settings.feed_rate = rand::random::<f32>() * 0.1;
            settings.kill_rate = rand::random::<f32>() * 0.1;
            settings.diffusion_rate_u = rand::random::<f32>() * 0.5;
            settings.diffusion_rate_v = rand::random::<f32>() * 0.3;
            settings.timestep = 0.5 + rand::random::<f32>() * 2.0;
            
            simulation.update_settings(settings, queue);
        }
        Ok(())
    }
}
