use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use wgpu::{Device, Queue, SurfaceConfiguration};

use crate::commands::AppSettings;
use crate::error::{AppError, AppResult};
use crate::simulation::preset_manager::SimulationPresetManager;
use crate::simulations::gray_scott::{GrayScottModel, settings::Settings as GrayScottSettings};
use crate::simulations::particle_life::{
    ParticleLifeModel, settings::Settings as ParticleLifeSettings, simulation::ColorMode,
};
use crate::simulations::shared::ColorScheme;
use crate::simulations::shared::{
    ColorSchemeManager, SimulationColorSchemeManager, coordinates::ScreenCoords,
};
use crate::simulations::slime_mold::{SlimeMoldModel, settings::Settings as SlimeMoldSettings};
use crate::simulations::traits::{Simulation, SimulationType};
use crate::simulations::voronoi_ca::simulation::VoronoiCASimulation;

pub struct SimulationManager {
    pub current_simulation: Option<SimulationType>,
    pub preset_manager: SimulationPresetManager,
    pub lut_manager: ColorSchemeManager,
    pub simulation_lut_manager: SimulationColorSchemeManager,
    pub render_loop_running: Arc<AtomicBool>,
    pub fps_limit_enabled: Arc<AtomicBool>,
    pub fps_limit: Arc<AtomicU32>,
    pub is_paused: Arc<AtomicBool>,
    // When paused, render-loop will update the simulation for this many frames then return to paused rendering
    pub step_frames_pending: Arc<AtomicU32>,
    pub app_settings: Arc<AppSettings>,
}

impl SimulationManager {
    pub fn new(app_settings: Arc<AppSettings>) -> Self {
        // Simulations start paused to prevent race conditions between initialization
        // and render loop startup. They are automatically unpaused after successful
        // initialization to ensure all GPU resources and state are ready.
        Self {
            current_simulation: None,
            preset_manager: SimulationPresetManager::new(),
            lut_manager: ColorSchemeManager::new(),
            simulation_lut_manager: SimulationColorSchemeManager::new(),
            render_loop_running: Arc::new(AtomicBool::new(false)),
            fps_limit_enabled: Arc::new(AtomicBool::new(false)),
            fps_limit: Arc::new(AtomicU32::new(60)),
            is_paused: Arc::new(AtomicBool::new(true)), // Start paused to avoid race condition
            step_frames_pending: Arc::new(AtomicU32::new(0)),
            app_settings,
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
                    &self.app_settings,
                    &self.lut_manager,
                )?;

                self.current_simulation = Some(SimulationType::SlimeMold(Box::new(simulation)));

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
                    &self.app_settings,
                )?;

                self.current_simulation = Some(SimulationType::GrayScott(Box::new(simulation)));

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
                    &self.app_settings,
                    &self.lut_manager,
                    ColorMode::ColorScheme,
                )?;

                self.current_simulation = Some(SimulationType::ParticleLife(Box::new(simulation)));

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
            "flow" => {
                // Initialize Flow simulation
                let settings = crate::simulations::flow::settings::Settings::default();
                let simulation = crate::simulations::flow::simulation::FlowModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    &self.app_settings,
                    &self.lut_manager,
                )
                .map_err(|e| format!("Failed to initialize Flow simulation: {}", e))?;

                self.current_simulation = Some(SimulationType::Flow(Box::new(simulation)));
                self.resume();
                Ok(())
            }
            "pellets" => {
                // Initialize Pellets simulation
                let settings = crate::simulations::pellets::settings::Settings::default();
                let simulation = crate::simulations::pellets::simulation::PelletsModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    &self.app_settings,
                    &self.lut_manager,
                )
                .map_err(|e| format!("Failed to initialize Pellets simulation: {}", e))?;

                self.current_simulation = Some(SimulationType::Pellets(Box::new(simulation)));
                self.resume();
                Ok(())
            }
            "gradient" => {
                // Initialize Gradient simulation
                let simulation = crate::simulations::gradient::GradientSimulation::new(
                    device,
                    queue,
                    surface_config.format,
                    &self.app_settings,
                );

                self.current_simulation = Some(SimulationType::Gradient(Box::new(simulation)));
                self.resume();
                Ok(())
            }
            "voronoi_ca" => {
                // Initialize Voronoi CA simulation
                let simulation =
                    VoronoiCASimulation::new(device, queue, surface_config, &self.app_settings)
                        .map_err(|e| {
                            format!("Failed to initialize Voronoi CA simulation: {}", e)
                        })?;

                self.current_simulation = Some(SimulationType::VoronoiCA(Box::new(simulation)));
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
        delta_time: f32,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.render_frame(device, queue, surface_view, delta_time)?;
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
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)?;
        }
        Ok(())
    }

    /// Handle mouse interaction using screen coordinates (physical pixels)
    pub fn handle_mouse_interaction_screen_coords(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        mouse_button: u32,
        device: &Arc<Device>,
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
                    // World space is [-1, 1] where (-1, -1) is bottom-left and (1, 1) is top-right
                    // Texture space is [0, 1] where (0, 0) is top-left and (1, 1) is bottom-right
                    let texture_x = (world.x + 1.0) * 0.5;
                    let texture_y = (1.0 - world.y) * 0.5; // Flip Y axis to match texture coordinates

                    simulation.handle_mouse_interaction(
                        texture_x,
                        texture_y,
                        mouse_button,
                        device,
                        queue,
                    )?;
                }
                SimulationType::ParticleLife(simulation) => {
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
                        device,
                        queue,
                    )?;
                }
                SimulationType::Pellets(simulation) => {
                    let camera = &simulation.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);

                    // Pellets particles also live in [-1,1] world space
                    simulation.handle_mouse_interaction(
                        world.x,
                        world.y,
                        mouse_button,
                        device,
                        queue,
                    )?;
                }
                SimulationType::SlimeMold(simulation) => {
                    let camera = &simulation.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);
                    let world_x = world.x;
                    let world_y = world.y;
                    simulation.handle_mouse_interaction(
                        world_x,
                        world_y,
                        mouse_button,
                        device,
                        queue,
                    )?;
                }
                SimulationType::Flow(simulation) => {
                    let camera = &simulation.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);
                    let world_x = world.x;
                    let world_y = world.y;
                    simulation.handle_mouse_interaction(
                        world_x,
                        world_y,
                        mouse_button,
                        device,
                        queue,
                    )?;
                }
                SimulationType::VoronoiCA(simulation) => {
                    let camera = &simulation.camera;
                    let screen = ScreenCoords::new(screen_x, screen_y);
                    let world = camera.screen_to_world(screen);
                    simulation.handle_mouse_interaction(
                        world.x,
                        world.y,
                        mouse_button,
                        device,
                        queue,
                    )?;
                }

                _ => (),
            }
        }
        Ok(())
    }

    /// Handle mouse release events
    pub fn handle_mouse_release(&mut self, mouse_button: u32, queue: &Arc<Queue>) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(simulation) => {
                    simulation.handle_mouse_release(mouse_button, queue)?;
                }
                SimulationType::ParticleLife(simulation) => {
                    simulation.handle_mouse_release(mouse_button, queue)?;
                }
                SimulationType::SlimeMold(simulation) => {
                    simulation.handle_mouse_release(mouse_button, queue)?;
                }
                SimulationType::Flow(simulation) => {
                    simulation.handle_mouse_release(mouse_button, queue)?;
                }
                SimulationType::Pellets(simulation) => {
                    simulation.handle_mouse_release(mouse_button, queue)?;
                }
                SimulationType::VoronoiCA(simulation) => {
                    simulation.handle_mouse_release(mouse_button, queue)?;
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

    /// Request a single simulation update while remaining in paused mode
    pub fn step_once(&self) {
        self.step_frames_pending.fetch_add(1, Ordering::Relaxed);
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
        tracing::debug!(
            "SimulationManager::update_setting called with setting_name: '{}', value: {:?}",
            setting_name,
            value
        );

        if let Some(simulation) = &mut self.current_simulation {
            tracing::debug!("Calling simulation.update_setting for current simulation");
            simulation.update_setting(setting_name, value.clone(), device, queue)?;
            tracing::debug!("Simulation update_setting completed successfully");
        } else {
            tracing::warn!("No simulation running, cannot update setting");
        }
        Ok(())
    }

    pub fn update_state(
        &mut self,
        state_name: &str,
        value: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        tracing::debug!(
            "SimulationManager::update_state called with state_name: '{}', value: {:?}",
            state_name,
            value
        );

        if let Some(simulation) = &mut self.current_simulation {
            tracing::debug!("Calling simulation.update_state for current simulation");
            simulation.update_state(state_name, value.clone(), device, queue)?;
            tracing::debug!("Simulation update_state completed successfully");
        } else {
            tracing::warn!("No simulation running, cannot update state");
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
            tracing::warn!(
                "No preset manager was created for simulation type: {}",
                simulation_type
            );
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

    pub fn save_preset(
        &mut self,
        preset_name: &str,
        settings: &serde_json::Value,
    ) -> AppResult<()> {
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

    // Color scheme management methods
    pub fn get_available_color_schemes(&self) -> Vec<String> {
        self.simulation_lut_manager
            .get_available_color_schemes(&self.lut_manager)
    }

    pub fn apply_color_scheme(
        &mut self,
        color_scheme_name: &str,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    // For slime mold, load the color scheme data and apply it directly
                    let mut color_scheme_data =
                        self.lut_manager.get(color_scheme_name).map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    color_scheme_name, e
                                )
                                .into(),
                            )
                        })?;

                    if simulation.lut_reversed {
                        color_scheme_data.reverse();
                    }

                    simulation.update_color_scheme(&color_scheme_data, device, queue)?;
                    simulation.current_lut_name = color_scheme_name.to_string();

                    tracing::info!(
                        "Color scheme '{}' applied to slime mold simulation",
                        color_scheme_name
                    );
                }
                SimulationType::GrayScott(simulation) => {
                    // For Gray-Scott, load the color scheme data and apply it to the renderer
                    let mut color_scheme_data =
                        self.lut_manager.get(color_scheme_name).map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    color_scheme_name, e
                                )
                                .into(),
                            )
                        })?;

                    if simulation.color_scheme_reversed {
                        color_scheme_data.reverse();
                    }

                    simulation.update_color_scheme(&color_scheme_data, device, queue)?;
                    simulation.current_color_scheme_name = color_scheme_name.to_string();

                    tracing::info!(
                        "Color scheme '{}' applied to Gray-Scott simulation",
                        color_scheme_name
                    );
                }
                SimulationType::ParticleLife(simulation) => {
                    // For particle life, use the existing update_state method
                    simulation.update_state(
                        "color_scheme",
                        serde_json::json!(color_scheme_name),
                        device,
                        queue,
                    )?;
                }
                SimulationType::Flow(simulation) => {
                    // For Flow, use the existing update_state method
                    simulation.update_state(
                        "currentLut",
                        serde_json::json!(color_scheme_name),
                        device,
                        queue,
                    )?;
                    tracing::info!(
                        "Color scheme '{}' applied to Flow simulation",
                        color_scheme_name
                    );
                }
                SimulationType::Pellets(simulation) => {
                    // For Pellets, use the existing update_state method
                    simulation.update_state(
                        "currentLut",
                        serde_json::json!(color_scheme_name),
                        device,
                        queue,
                    )?;
                    tracing::info!(
                        "Color scheme '{}' applied to Pellets simulation",
                        color_scheme_name
                    );
                }
                SimulationType::MainMenu(_) => {
                    // Main menu doesn't support color scheme changes
                    tracing::warn!("Color scheme changes not supported for main menu simulation");
                }
                SimulationType::Gradient(simulation) => {
                    // For gradient simulation, load the color scheme data and apply it directly
                    let color_scheme_data =
                        self.lut_manager.get(color_scheme_name).map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    color_scheme_name, e
                                )
                                .into(),
                            )
                        })?;

                    simulation.update_color_scheme(&color_scheme_data, device, queue)?;

                    tracing::info!(
                        "Color scheme '{}' applied to gradient simulation",
                        color_scheme_name
                    );
                }
                SimulationType::VoronoiCA(simulation) => {
                    // Load color scheme data and write directly into the VCA LUT buffer
                    let mut color_scheme_data =
                        self.lut_manager.get(color_scheme_name).map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    color_scheme_name, e
                                )
                                .into(),
                            )
                        })?;

                    if simulation.lut_reversed {
                        color_scheme_data.reverse();
                    }

                    let data_u32 = color_scheme_data.to_u32_buffer();
                    queue.write_buffer(&simulation.lut_buffer, 0, bytemuck::cast_slice(&data_u32));
                    simulation.current_lut_name = color_scheme_name.to_string();
                    tracing::info!(
                        "Color scheme '{}' applied to Voronoi CA simulation (direct write)",
                        color_scheme_name
                    );
                }
            }
        }
        Ok(())
    }

    pub fn reverse_current_color_scheme(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    // Toggle the reversed flag and reload the color scheme
                    simulation.lut_reversed = !simulation.lut_reversed;
                    let mut color_scheme_data = self
                        .lut_manager
                        .get(&simulation.current_lut_name)
                        .map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    simulation.current_lut_name, e
                                )
                                .into(),
                            )
                        })?;

                    if simulation.lut_reversed {
                        color_scheme_data.reverse();
                    }

                    simulation.update_color_scheme(&color_scheme_data, device, queue)?;
                    tracing::info!("LUT reversed for slime mold simulation");
                }
                SimulationType::GrayScott(simulation) => {
                    // Toggle the reversed flag and reload the LUT
                    simulation.color_scheme_reversed = !simulation.color_scheme_reversed;
                    let mut color_scheme_data = self
                        .lut_manager
                        .get(&simulation.current_color_scheme_name)
                        .map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    simulation.current_color_scheme_name, e
                                )
                                .into(),
                            )
                        })?;

                    if simulation.color_scheme_reversed {
                        color_scheme_data.reverse();
                    }

                    simulation.update_color_scheme(&color_scheme_data, device, queue)?;
                    tracing::info!("LUT reversed for Gray-Scott simulation");
                }
                SimulationType::ParticleLife(simulation) => {
                    // For particle life, we need to update the LUT with reversed flag
                    let current_reversed = simulation.state.color_scheme_reversed;
                    simulation.state.color_scheme_reversed = !current_reversed;

                    // Get the current color scheme and apply it with the new reversal state
                    let mut color_scheme_data = self
                        .lut_manager
                        .get(&simulation.state.current_color_scheme_name)
                        .map_err(|e| {
                            AppError::Simulation(
                                format!(
                                    "Failed to load color scheme '{}': {}",
                                    simulation.state.current_color_scheme_name, e
                                )
                                .into(),
                            )
                        })?;

                    if simulation.state.color_scheme_reversed {
                        color_scheme_data.reverse();
                    }

                    simulation.update_color_scheme(&color_scheme_data, device, queue)?;
                }
                SimulationType::Flow(simulation) => {
                    // For Flow, use the built-in LUT reversal mechanism
                    let current_reversed = simulation.lut_reversed;
                    simulation.update_setting(
                        "lutReversed",
                        serde_json::json!(!current_reversed),
                        device,
                        queue,
                    )?;
                    tracing::info!("LUT reversed for Flow simulation");
                }
                SimulationType::Pellets(simulation) => {
                    // For Pellets, use the built-in LUT reversal mechanism
                    let current_reversed = simulation.state.lut_reversed;
                    simulation.update_setting(
                        "lut_reversed",
                        serde_json::json!(!current_reversed),
                        device,
                        queue,
                    )?;
                    tracing::info!("LUT reversed for Pellets simulation");
                }
                SimulationType::MainMenu(_) => {
                    // Main menu doesn't support LUT changes
                    tracing::warn!("LUT reversal not supported for main menu simulation");
                }
                SimulationType::VoronoiCA(simulation) => {
                    let current_reversed = simulation.lut_reversed;
                    simulation.lut_reversed = !current_reversed;
                    // Update GPU buffer to reflect reversal using the app's LUT manager
                    if let Ok(lut) = self.lut_manager.get(&simulation.current_lut_name) {
                        let mut data = lut.to_u32_buffer();
                        if simulation.lut_reversed {
                            // reverse planar LUT in-place
                            data[0..256].reverse();
                            data[256..512].reverse();
                            data[512..768].reverse();
                        }
                        queue.write_buffer(&simulation.lut_buffer, 0, bytemuck::cast_slice(&data));
                    }
                    tracing::info!("LUT reversed for Voronoi CA simulation");
                }
                SimulationType::Gradient(_simulation) => {
                    tracing::warn!("LUT reversal not supported for Gradient simulation");
                }
            }
        }
        Ok(())
    }

    /// Apply a custom color scheme to any running simulation
    pub fn apply_custom_color_scheme(
        &mut self,
        color_scheme: &ColorScheme,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            simulation.update_color_scheme(color_scheme, device, queue)?;
            tracing::info!("Custom color scheme applied to simulation");
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
        let step_frames_pending = self.step_frames_pending.clone();

        render_loop_running.store(true, Ordering::Relaxed);

        tokio::spawn(async move {
            let mut frame_count = 0u32;
            let mut last_fps_update = Instant::now();
            let mut last_frame_time = Instant::now();

            while render_loop_running.load(Ordering::Relaxed) {
                let frame_start = Instant::now();

                // Render frame (continue rendering even when paused to show camera changes)
                {
                    let mut sim_manager = manager.lock().await;
                    let gpu_ctx = gpu_context.lock().await;

                    if sim_manager.is_running() {
                        match gpu_ctx.get_current_texture() {
                            Ok(output) => {
                                let view = output
                                    .texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());

                                // Calculate delta time
                                let delta_time =
                                    frame_start.duration_since(last_frame_time).as_secs_f32();

                                let paused = is_paused.load(Ordering::Relaxed);
                                let mut do_update = !paused;
                                if paused {
                                    // If paused, allow a single-frame update when requested
                                    let pending = step_frames_pending.load(Ordering::Relaxed);
                                    if pending > 0 {
                                        step_frames_pending.fetch_sub(1, Ordering::Relaxed);
                                        do_update = true;
                                    }
                                }

                                let render_result = if do_update {
                                    sim_manager.render(
                                        &gpu_ctx.device,
                                        &gpu_ctx.queue,
                                        &view,
                                        delta_time,
                                    )
                                } else {
                                    sim_manager.render_paused(
                                        &gpu_ctx.device,
                                        &gpu_ctx.queue,
                                        &view,
                                    )
                                };

                                if render_result.is_ok() {
                                    output.present();
                                }
                            }
                            Err(e) => {
                                // Attempt to recover from surface errors (e.g., after fullscreen)
                                tracing::warn!(
                                    "Failed to acquire surface texture ({}). Attempting recovery...",
                                    e
                                );

                                if let Some(window) = app_handle.get_webview_window("main") {
                                    match window.inner_size() {
                                        Ok(size) => {
                                            let width = size.width;
                                            let height = size.height;

                                            // Reconfigure the surface to current window size
                                            if let Err(recfg_err) =
                                                gpu_ctx.resize_surface(width, height).await
                                            {
                                                tracing::error!(
                                                    "Surface reconfigure failed during recovery: {}",
                                                    recfg_err
                                                );
                                            } else {
                                                // Propagate resize to current simulation so it can rebuild resources
                                                let new_config =
                                                    gpu_ctx.surface_config.lock().await.clone();
                                                if let Err(resize_err) = sim_manager.handle_resize(
                                                    &gpu_ctx.device,
                                                    &gpu_ctx.queue,
                                                    &new_config,
                                                ) {
                                                    tracing::error!(
                                                        "Simulation resize failed during recovery: {}",
                                                        resize_err
                                                    );
                                                } else {
                                                    tracing::info!(
                                                        "Surface and simulation successfully recovered after resize to {}x{}",
                                                        width,
                                                        height
                                                    );
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            tracing::error!(
                                                "Failed to query window size during recovery: {}",
                                                err
                                            );
                                        }
                                    }
                                } else {
                                    tracing::error!(
                                        "Main window not found during surface recovery"
                                    );
                                }
                            }
                        }
                    } else {
                        // Stop the render loop if simulation is no longer running
                        break;
                    }
                }

                // Update last frame time for next iteration
                last_frame_time = frame_start;
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

    // Note: seed_random_noise is Gray-Scott and CSA specific functionality
    pub fn seed_random_noise(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::GrayScott(sim) => {
                    sim.seed_random_noise(device, queue)
                        .map_err(AppError::Simulation)?;
                }

                _ => {
                    // Seed random noise is only supported for Gray-Scott and CSA simulations
                    tracing::warn!(
                        "Seed random noise is only supported for Gray-Scott and CSA simulations"
                    );
                }
            }
        }
        Ok(())
    }

    // Camera control methods
    pub fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.pan_camera(delta_x, delta_y),
                SimulationType::GrayScott(simulation) => {
                    simulation.renderer.camera.pan(delta_x, delta_y)
                }
                SimulationType::ParticleLife(simulation) => simulation.camera.pan(delta_x, delta_y),
                SimulationType::Flow(simulation) => simulation.pan_camera(delta_x, delta_y),
                SimulationType::Pellets(simulation) => simulation.pan_camera(delta_x, delta_y),
                SimulationType::MainMenu(_) => {}
                SimulationType::VoronoiCA(simulation) => simulation.pan_camera(delta_x, delta_y),
                _ => {}
            }
        }
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.zoom_camera(delta),
                SimulationType::GrayScott(simulation) => simulation.renderer.camera.zoom(delta),
                SimulationType::ParticleLife(simulation) => simulation.camera.zoom(delta),
                SimulationType::Flow(simulation) => simulation.camera.zoom(delta),
                SimulationType::Pellets(simulation) => simulation.camera.zoom(delta),
                SimulationType::MainMenu(_) => {}
                SimulationType::VoronoiCA(simulation) => simulation.camera.zoom(delta),
                _ => {}
            }
        }
    }

    pub fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
                }
                SimulationType::GrayScott(simulation) => simulation
                    .renderer
                    .camera
                    .zoom_to_cursor(delta, cursor_x, cursor_y),
                SimulationType::ParticleLife(simulation) => {
                    simulation.camera.zoom_to_cursor(delta, cursor_x, cursor_y)
                }
                SimulationType::Flow(simulation) => {
                    simulation.camera.zoom_to_cursor(delta, cursor_x, cursor_y)
                }
                SimulationType::Pellets(simulation) => {
                    simulation.camera.zoom_to_cursor(delta, cursor_x, cursor_y)
                }
                SimulationType::MainMenu(_) => {}
                SimulationType::VoronoiCA(simulation) => {
                    simulation.camera.zoom_to_cursor(delta, cursor_x, cursor_y)
                }
                _ => {}
            }
        }
    }

    pub fn reset_camera(&mut self) {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => simulation.reset_camera(),
                SimulationType::GrayScott(simulation) => simulation.renderer.camera.reset(),
                SimulationType::ParticleLife(simulation) => simulation.camera.reset(),
                SimulationType::Flow(simulation) => simulation.camera.reset(),
                SimulationType::Pellets(simulation) => simulation.camera.reset(),
                SimulationType::MainMenu(_) => {}
                SimulationType::VoronoiCA(simulation) => simulation.camera.reset(),
                _ => {}
            }
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
                SimulationType::Flow(simulation) => Some(simulation.get_camera_state()),
                SimulationType::Pellets(simulation) => Some(simulation.get_camera_state()),
                SimulationType::MainMenu(_) => Some(serde_json::json!({})), // No camera for main menu background
                SimulationType::VoronoiCA(simulation) => Some(simulation.get_camera_state()),
                _ => Some(serde_json::json!({})), // No camera for other simulations
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
                SimulationType::Flow(simulation) => {
                    simulation.camera.set_smoothing_factor(smoothing_factor)
                }
                SimulationType::Pellets(simulation) => {
                    simulation.camera.set_smoothing_factor(smoothing_factor)
                }
                SimulationType::MainMenu(_) => {} // No camera for main menu background
                SimulationType::VoronoiCA(simulation) => {
                    simulation.camera.set_smoothing_factor(smoothing_factor)
                }
                _ => {} // No camera for other simulations
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
                SimulationType::Flow(simulation) => simulation.camera.set_sensitivity(sensitivity),
                SimulationType::Pellets(simulation) => {
                    simulation.camera.set_sensitivity(sensitivity)
                }
                SimulationType::MainMenu(_) => {} // No camera for main menu background
                SimulationType::VoronoiCA(simulation) => {
                    simulation.camera.set_sensitivity(sensitivity)
                }
                _ => {} // No camera for other simulations
            }
        }
    }

    /// Update cursor size for the active simulation
    pub fn update_cursor_size(
        &mut self,
        size: f32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_size",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(size as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::ParticleLife(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_size",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(size as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::Pellets(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_size",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(size as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::Flow(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_size",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(size as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::VoronoiCA(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_size",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(size as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                _ => {
                    return Err(AppError::Simulation(
                        crate::error::SimulationError::InvalidParameter(
                            "Cursor size not supported for this simulation type".to_string(),
                        ),
                    ));
                }
            }
        } else {
            return Err(AppError::Simulation(
                crate::error::SimulationError::InvalidParameter(
                    "No simulation running".to_string(),
                ),
            ));
        }
        Ok(())
    }

    /// Update cursor strength for the active simulation
    pub fn update_cursor_strength(
        &mut self,
        strength: f32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> AppResult<()> {
        if let Some(simulation) = &mut self.current_simulation {
            match simulation {
                SimulationType::SlimeMold(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_strength",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(strength as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::ParticleLife(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_strength",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(strength as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::Pellets(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_strength",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(strength as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::Flow(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_strength",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(strength as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                SimulationType::VoronoiCA(simulation) => {
                    simulation
                        .update_setting(
                            "cursor_strength",
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(strength as f64).unwrap(),
                            ),
                            device,
                            queue,
                        )
                        .map_err(AppError::Simulation)?;
                }
                _ => {
                    return Err(AppError::Simulation(
                        crate::error::SimulationError::InvalidParameter(
                            "Cursor strength not supported for this simulation type".to_string(),
                        ),
                    ));
                }
            }
        } else {
            return Err(AppError::Simulation(
                crate::error::SimulationError::InvalidParameter(
                    "No simulation running".to_string(),
                ),
            ));
        }
        Ok(())
    }
}
