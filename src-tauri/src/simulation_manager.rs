use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tauri::{State, AppHandle, Emitter};
use wgpu::{Device, Queue, SurfaceConfiguration};
use winit::event_loop::ActiveEventLoop;

use crate::simulations::slime_mold::{
    SlimeMoldSimulation,
    Settings,
    LutManager,
    PresetManager,
    presets::init_preset_manager,
};

#[derive(Debug, Clone)]
pub enum SimulationType {
    SlimeMold,
    ParticleLife,
    ReactionDiffusion,
}

pub struct SimulationManager {
    pub slime_mold_state: Option<SlimeMoldSimulation>,
    pub preset_manager: PresetManager,
    pub lut_manager: LutManager,
    pub render_loop_running: Arc<AtomicBool>,
    pub fps_limit_enabled: Arc<AtomicBool>,
    pub fps_limit: Arc<AtomicU32>,
}

impl SimulationManager {
    pub fn new() -> Self {
        Self {
            slime_mold_state: None,
            preset_manager: init_preset_manager(),
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
                let settings = Settings::default();
                let available_luts = self.lut_manager.get_available_luts();
                let default_lut_name = "MATPLOTLIB_bone_r";
                let current_lut_index = available_luts.iter().position(|name| name == default_lut_name).unwrap_or(0);
                
                let simulation = SlimeMoldSimulation::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    100000, // agent_count
                    settings,
                    &self.lut_manager,
                    &available_luts,
                    current_lut_index, // current_lut_index
                    false, // lut_reversed
                )?;
                
                self.slime_mold_state = Some(simulation);
                Ok(())
            }
            _ => Err("Unknown simulation type".into()),
        }
    }

    pub fn stop_simulation(&mut self) {
        self.slime_mold_state = None;
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
        Ok(())
    }

    pub fn handle_resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.resize(device, queue, new_config, adapter_info)?;
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.slime_mold_state.is_some()
    }

    pub fn get_status(&self) -> String {
        if self.slime_mold_state.is_some() {
            "Slime Mold Simulation Running".to_string()
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
        adapter_info: &wgpu::AdapterInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.update_agent_count(count, device, queue, surface_config, adapter_info).await?;
        }
        Ok(())
    }

    // Preset management methods
    pub fn get_available_presets(&self) -> Vec<String> {
        self.preset_manager.get_preset_names()
    }

    pub fn apply_preset(
        &mut self,
        preset_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(preset) = self.preset_manager.get_preset(preset_name) {
            if let Some(simulation) = &mut self.slime_mold_state {
                simulation.update_settings(preset.settings.clone(), queue);
            }
            Ok(())
        } else {
            Err(format!("Preset '{}' not found", preset_name).into())
        }
    }

    pub fn apply_preset_settings(
        &mut self,
        settings: Settings,
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
        settings: &Settings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.preset_manager.save_user_preset(preset_name, settings)
    }

    pub fn delete_preset(
        &mut self,
        preset_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.preset_manager.delete_user_preset(preset_name)?;
        Ok(())
    }

    pub fn get_current_settings(&self) -> Option<Settings> {
        self.slime_mold_state.as_ref().map(|sim| sim.settings.clone())
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
                    
                    if sim_manager.slime_mold_state.is_some() && sim_manager.render_loop_running.load(Ordering::Relaxed) {
                        if let Ok(output) = gpu_ctx.get_current_texture() {
                            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                            if let Ok(_) = sim_manager.render(&gpu_ctx.device, &gpu_ctx.queue, &view) {
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
                        let target_frame_time = Duration::from_nanos(1_000_000_000 / target_fps as u64);
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

    pub fn reset_trails(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.reset_trails(queue);
        }
        Ok(())
    }

    pub fn reset_agents(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.reset_agents(queue);
        }
        Ok(())
    }
}