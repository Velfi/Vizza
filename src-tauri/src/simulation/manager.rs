use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;
use std::time::Instant;
use wgpu::{Device, Queue, SurfaceConfiguration};

use crate::simulations::gray_scott::{
    self, presets::init_preset_manager as init_gray_scott_preset_manager,
};
use crate::simulations::slime_mold::{
    self, presets::init_preset_manager as init_slime_mold_preset_manager,
};
use crate::simulations::shared::coordinates::ScreenCoords;
use crate::simulations::traits::{Simulation, SimulationType};

use super::preset_manager::PresetManager;
use super::lut_manager::LutManager as SimulationLutManager;
use super::render_loop::RenderLoopManager;

pub struct SimulationManager {
    pub current_simulation: Option<SimulationType>,
    pub slime_mold_preset_manager: slime_mold::presets::PresetManager,
    pub gray_scott_preset_manager: gray_scott::presets::PresetManager,
    pub lut_manager: crate::simulations::shared::LutManager,
    pub render_loop_running: Arc<AtomicBool>,
    pub fps_limit_enabled: Arc<AtomicBool>,
    pub fps_limit: Arc<AtomicU32>,
    pub start_time: Instant,
}

impl SimulationManager {
    pub fn new() -> Self {
        Self {
            current_simulation: None,
            slime_mold_preset_manager: init_slime_mold_preset_manager(),
            gray_scott_preset_manager: init_gray_scott_preset_manager(),
            lut_manager: crate::simulations::shared::LutManager::new(),
            render_loop_running: Arc::new(AtomicBool::new(false)),
            fps_limit_enabled: Arc::new(AtomicBool::new(false)),
            fps_limit: Arc::new(AtomicU32::new(60)),
            start_time: Instant::now(),
        }
    }

    pub fn get_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
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
                let simulation = slime_mold::SlimeMoldModel::new(
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
                let settings = crate::simulations::gray_scott::settings::Settings::default();

                let simulation = crate::simulations::gray_scott::GrayScottModel::new(
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
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.render_frame(device, queue, surface_view)?,
                SimulationType::GrayScott(simulation) => simulation.render_frame(device, queue, surface_view)?,
            }
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
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.resize(device, queue, new_config)?,
                SimulationType::GrayScott(simulation) => simulation.resize(new_config)?,
            }
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
            match simulation {
                SimulationType::GrayScott(simulation) => simulation.handle_mouse_interaction(world_x, world_y, is_seeding, queue)?,
                _ => (),
            }
        }
        // TODO: Add slime mold mouse interaction if needed
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
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.update_setting(setting_name, value.clone(), device, queue)?,
                SimulationType::GrayScott(simulation) => simulation.update_setting(setting_name, value.clone(), device, queue)?,
            }
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
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation
                    .update_agent_count(count, device, queue, surface_config)
                    .await?,
                _ => (),
            }
        }
        Ok(())
    }

    pub fn get_current_settings(&self) -> Option<serde_json::Value> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => Some(simulation.get_settings()),
                SimulationType::GrayScott(simulation) => Some(simulation.get_settings()),
            }
        } else {
            None
        }
    }

    pub fn get_current_state(&self) -> Option<serde_json::Value> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => Some(simulation.get_state()),
                SimulationType::GrayScott(simulation) => Some(simulation.get_state()),
            }
        } else {
            None
        }
    }

    pub fn get_current_agent_count(&self) -> Option<u32> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => Some(simulation.get_agent_count()),
                SimulationType::GrayScott(_) => None, // Gray-Scott doesn't have agents
            }
        } else {
            None
        }
    }

    pub fn toggle_gui(&mut self) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {simulation.toggle_gui();},
                SimulationType::GrayScott(simulation) => {simulation.toggle_gui();},
            }
        }
    }

    pub fn is_gui_visible(&self) -> bool {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.is_gui_visible(),
                SimulationType::GrayScott(simulation) => simulation.is_gui_visible(),
            }
        } else {
            false
        }
    }

    pub fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.randomize_settings(device, queue)?,
                SimulationType::GrayScott(simulation) => simulation.randomize_settings(device, queue)?,
            }
        }
        Ok(())
    }

    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.pan_camera(delta_x, delta_y),
                SimulationType::GrayScott(simulation) => simulation.pan_camera(delta_x, delta_y),
            }
        }
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.zoom_camera(delta),
                SimulationType::GrayScott(simulation) => simulation.zoom_camera(delta),
            }
        }
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y),
                SimulationType::GrayScott(simulation) => simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y),
            }
        }
    }

    pub fn reset_camera(&mut self) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.reset_camera(),
                SimulationType::GrayScott(simulation) => simulation.reset_camera(),
            }
        }
    }

    pub fn get_camera_state(&self) -> Option<serde_json::Value> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => Some(simulation.get_camera_state()),
                SimulationType::GrayScott(simulation) => Some(simulation.get_camera_state()),
            }
        } else {
            None
        }
    }

    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> Option<(f32, f32)> {
        if let Some(simulation) = &self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    let camera = &simulation.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);
                    Some((world.x, world.y))
                }
                SimulationType::GrayScott(simulation) => {
                    let camera = &simulation.renderer.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);
                    Some((world.x, world.y))
                }
            }
        } else {
            None
        }
    }
} 