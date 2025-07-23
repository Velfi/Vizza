//! # Simulation Traits Module
//!
//! Defines the core interfaces that unify all simulations in the Vizzy application.
//! This module establishes the contract that ensures consistent behavior across
//! different simulation types while preserving their unique characteristics.
//!
//! ## Design Philosophy
//!
//! The trait system is designed to provide a unified interface for simulation
//! management while maintaining the flexibility needed for diverse simulation
//! types. This approach enables polymorphic handling of simulations while
//! ensuring each can implement its unique behavior and requirements.
//!
//! ## Core Concepts
//!
//! The trait system emphasizes the distinction between configuration and
//! runtime state, enabling proper preset management and state restoration.
//! It also provides comprehensive user interaction capabilities that work
//! consistently across all simulation types.

use crate::error::SimulationResult;
use serde_json::Value;
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

/// Common interface for all simulation types
///
/// This trait defines the contract that all simulations must implement.
/// It provides a unified way to interact with different simulation types
/// while maintaining clear separation between settings (presettable) and state (runtime).
pub trait Simulation {
    /// Render a single frame of the simulation
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()>;

    /// Render a static frame without updating simulation state (for paused mode)
    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()>;

    /// Handle window resize events
    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()>;

    /// Update a specific setting by name
    ///
    /// This method should only modify user-configurable settings that can be saved in presets.
    /// Runtime state should not be modified through this method.
    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;

    /// Get the current settings as a serializable value
    ///
    /// This should return only user-configurable settings that can be saved in presets.
    /// Runtime state should not be included in the returned value.
    fn get_settings(&self) -> Value;

    /// Get the current runtime state as a serializable value
    ///
    /// This should return runtime state that is not saved in presets.
    /// Examples: current agent positions, simulation time, etc.
    fn get_state(&self) -> Value;

    /// Handle mouse interaction for cursor-based particle attraction/repulsion
    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32, // 0 = left, 1 = middle, 2 = right
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;

    /// Handle mouse release events
    fn handle_mouse_release(
        &mut self,
        mouse_button: u32, // 0 = left, 1 = middle, 2 = right
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;

    /// Pan the camera by the given delta
    fn pan_camera(&mut self, delta_x: f32, delta_y: f32);

    /// Zoom the camera by the given delta
    fn zoom_camera(&mut self, delta: f32);

    /// Zoom the camera to a specific cursor position
    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32);

    /// Reset the camera to default position and zoom
    fn reset_camera(&mut self);

    /// Get the current camera state as a serializable value
    fn get_camera_state(&self) -> Value;

    /// Save the current settings as a preset
    ///
    /// This should only save settings, not runtime state.
    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()>;

    /// Load settings from a preset and reset runtime state
    ///
    /// This should load settings and reset any runtime state to default values.
    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()>;

    /// Update the simulation settings directly
    ///
    /// This should apply new settings to the simulation without resetting runtime state.
    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;

    /// Reset the simulation's runtime state
    ///
    /// This should reset runtime state (like agent positions, trail maps) but preserve settings.
    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;

    /// Toggle GUI visibility
    fn toggle_gui(&mut self) -> bool;

    /// Check if GUI is visible
    fn is_gui_visible(&self) -> bool;

    /// Randomize the current settings
    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;
}

/// Enum wrapper for all simulation types
///
/// This provides a type-safe way to handle different simulation types
/// without using trait objects (Box<dyn Simulation>).
#[derive(Debug)]
pub enum SimulationType {
    SlimeMold(crate::simulations::slime_mold::SlimeMoldModel),
    GrayScott(crate::simulations::gray_scott::GrayScottModel),
    ParticleLife(crate::simulations::particle_life::ParticleLifeModel),
    Ecosystem(crate::simulations::ecosystem::EcosystemModel),
    Flow(crate::simulations::flow::simulation::FlowModel),
    Pellets(Box<crate::simulations::pellets::PelletsModel>),
    MainMenu(crate::simulations::main_menu::MainMenuModel),
    Gradient(crate::simulations::gradient::GradientSimulation),
}

impl SimulationType {
    /// Create a new simulation of the specified type
    pub async fn new(
        simulation_type: &str,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
        lut_manager: &crate::simulations::shared::LutManager,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match simulation_type {
            "slime_mold" => {
                let settings = crate::simulations::slime_mold::settings::Settings::default();
                let simulation = crate::simulations::slime_mold::SlimeMoldModel::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    10_000_000,
                    settings,
                    lut_manager,
                )?;
                Ok(SimulationType::SlimeMold(simulation))
            }
            "gray_scott" => {
                let settings = crate::simulations::gray_scott::settings::Settings::default();
                let simulation = crate::simulations::gray_scott::GrayScottModel::new(
                    device,
                    queue,
                    surface_config,
                    surface_config.width,
                    surface_config.height,
                    settings,
                    lut_manager,
                )?;
                Ok(SimulationType::GrayScott(simulation))
            }
            "particle_life" => {
                let settings = crate::simulations::particle_life::settings::Settings::default();
                let simulation = crate::simulations::particle_life::ParticleLifeModel::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    15000, // Default particle count
                    settings,
                    lut_manager,
                    crate::simulations::particle_life::simulation::ColorMode::Lut,
                )?;
                Ok(SimulationType::ParticleLife(simulation))
            }
            "ecosystem" => {
                let settings = crate::simulations::ecosystem::settings::Settings::default();
                let simulation = crate::simulations::ecosystem::EcosystemModel::new(
                    device,
                    queue,
                    surface_config,
                    1000, // Default agent count
                    settings,
                    lut_manager,
                )?;
                Ok(SimulationType::Ecosystem(simulation))
            }
            "flow" => {
                let settings = crate::simulations::flow::settings::Settings::default();
                let simulation = crate::simulations::flow::simulation::FlowModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    lut_manager,
                )?;
                Ok(SimulationType::Flow(simulation))
            }
            "pellets" => {
                let settings = crate::simulations::pellets::settings::Settings::default();
                let simulation = crate::simulations::pellets::PelletsModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    lut_manager,
                )?;
                Ok(SimulationType::Pellets(Box::new(simulation)))
            }
            "gradient" => {
                let simulation = crate::simulations::gradient::GradientSimulation::new(
                    device,
                    queue,
                    surface_config.format,
                );
                Ok(SimulationType::Gradient(simulation))
            }
            "main_menu" => {
                let simulation = crate::simulations::main_menu::MainMenuModel::new(
                    device,
                    surface_config,
                    lut_manager,
                )?;
                Ok(SimulationType::MainMenu(simulation))
            }
            _ => Err(format!("Unknown simulation type: {}", simulation_type).into()),
        }
    }

    pub fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::GrayScott(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::ParticleLife(simulation) => {
                simulation.reset_runtime_state(device, queue)
            }
            SimulationType::Ecosystem(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::Flow(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::Pellets(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::MainMenu(simulation) => simulation.reset_runtime_state(device, queue),
            _ => Ok(()),
        }
    }
}

impl Simulation for SimulationType {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
            SimulationType::Flow(sim) => sim.render_frame(device, queue, surface_view),
            SimulationType::Pellets(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
            SimulationType::Gradient(simulation) => {
                simulation.render_frame(device, queue, surface_view)
            }
        }
    }

    fn render_frame_static(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
            SimulationType::Flow(sim) => sim.render_frame_static(device, queue, surface_view),
            SimulationType::Pellets(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
            SimulationType::Gradient(simulation) => {
                simulation.render_frame_static(device, queue, surface_view)
            }
        }
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::GrayScott(simulation) => simulation.resize(new_config),
            SimulationType::ParticleLife(simulation) => {
                simulation.resize(device, queue, new_config)
            }
            SimulationType::Ecosystem(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::Flow(sim) => sim.resize(device, queue, new_config),
            SimulationType::Pellets(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::MainMenu(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::Gradient(simulation) => simulation.resize(device, queue, new_config),
        }
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
            SimulationType::Flow(sim) => sim.update_setting(setting_name, value, device, queue),
            SimulationType::Pellets(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
            SimulationType::Gradient(simulation) => {
                simulation.update_setting(setting_name, value, device, queue)
            }
        }
    }

    fn get_settings(&self) -> Value {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_settings(),
            SimulationType::GrayScott(simulation) => simulation.get_settings(),
            SimulationType::ParticleLife(simulation) => simulation.get_settings(),
            SimulationType::Ecosystem(simulation) => simulation.get_settings(),
            SimulationType::Flow(sim) => sim.get_settings(),
            SimulationType::Pellets(simulation) => simulation.get_settings(),
            SimulationType::MainMenu(simulation) => simulation.get_settings(),
            SimulationType::Gradient(simulation) => simulation.get_settings(),
        }
    }

    fn get_state(&self) -> Value {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_state(),
            SimulationType::GrayScott(simulation) => simulation.get_state(),
            SimulationType::ParticleLife(simulation) => simulation.get_state(),
            SimulationType::Ecosystem(simulation) => simulation.get_state(),
            SimulationType::Flow(sim) => sim.get_state(),
            SimulationType::Pellets(simulation) => simulation.get_state(),
            SimulationType::MainMenu(simulation) => simulation.get_state(),
            SimulationType::Gradient(simulation) => simulation.get_state(),
        }
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32, // 0 = left, 1 = middle, 2 = right
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::Flow(sim) => {
                sim.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::Pellets(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
            SimulationType::Gradient(simulation) => {
                simulation.handle_mouse_interaction(world_x, world_y, mouse_button, device, queue)
            }
        }
    }

    fn handle_mouse_release(
        &mut self,
        mouse_button: u32, // 0 = left, 1 = middle, 2 = right
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
            SimulationType::Flow(sim) => sim.handle_mouse_release(mouse_button, queue),
            SimulationType::Pellets(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
            SimulationType::Gradient(simulation) => {
                simulation.handle_mouse_release(mouse_button, queue)
            }
        }
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::GrayScott(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::ParticleLife(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::Ecosystem(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::Flow(sim) => sim.pan_camera(delta_x, delta_y),
            SimulationType::Pellets(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::MainMenu(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::Gradient(simulation) => simulation.pan_camera(delta_x, delta_y),
        }
    }

    fn zoom_camera(&mut self, delta: f32) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.zoom_camera(delta),
            SimulationType::GrayScott(simulation) => simulation.zoom_camera(delta),
            SimulationType::ParticleLife(simulation) => simulation.zoom_camera(delta),
            SimulationType::Ecosystem(simulation) => simulation.zoom_camera(delta),
            SimulationType::Flow(sim) => sim.zoom_camera(delta),
            SimulationType::Pellets(simulation) => simulation.zoom_camera(delta),
            SimulationType::MainMenu(simulation) => simulation.zoom_camera(delta),
            SimulationType::Gradient(simulation) => simulation.zoom_camera(delta),
        }
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
            SimulationType::Flow(sim) => sim.zoom_camera_to_cursor(delta, cursor_x, cursor_y),
            SimulationType::Pellets(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
            SimulationType::Gradient(simulation) => {
                simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y)
            }
        }
    }

    fn reset_camera(&mut self) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.reset_camera(),
            SimulationType::GrayScott(simulation) => simulation.reset_camera(),
            SimulationType::ParticleLife(simulation) => simulation.reset_camera(),
            SimulationType::Ecosystem(simulation) => simulation.reset_camera(),
            SimulationType::Flow(sim) => sim.reset_camera(),
            SimulationType::Pellets(simulation) => simulation.reset_camera(),
            SimulationType::MainMenu(simulation) => simulation.reset_camera(),
            SimulationType::Gradient(simulation) => simulation.reset_camera(),
        }
    }

    fn get_camera_state(&self) -> Value {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_camera_state(),
            SimulationType::GrayScott(simulation) => simulation.get_camera_state(),
            SimulationType::ParticleLife(simulation) => simulation.get_camera_state(),
            SimulationType::Ecosystem(simulation) => simulation.get_camera_state(),
            SimulationType::Flow(sim) => sim.get_camera_state(),
            SimulationType::Pellets(simulation) => simulation.get_camera_state(),
            SimulationType::MainMenu(simulation) => simulation.get_camera_state(),
            SimulationType::Gradient(simulation) => simulation.get_camera_state(),
        }
    }

    fn save_preset(&self, preset_name: &str) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.save_preset(preset_name),
            SimulationType::GrayScott(simulation) => simulation.save_preset(preset_name),
            SimulationType::ParticleLife(simulation) => simulation.save_preset(preset_name),
            SimulationType::Ecosystem(simulation) => simulation.save_preset(preset_name),
            SimulationType::Flow(sim) => sim.save_preset(preset_name),
            SimulationType::Pellets(simulation) => simulation.save_preset(preset_name),
            SimulationType::MainMenu(simulation) => simulation.save_preset(preset_name),
            SimulationType::Gradient(simulation) => simulation.save_preset(preset_name),
        }
    }

    fn load_preset(&mut self, preset_name: &str, queue: &Arc<Queue>) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::GrayScott(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::ParticleLife(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::Ecosystem(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::Flow(sim) => sim.load_preset(preset_name, queue),
            SimulationType::Pellets(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::MainMenu(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::Gradient(simulation) => simulation.load_preset(preset_name, queue),
        }
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
            SimulationType::GrayScott(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
            SimulationType::ParticleLife(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
            SimulationType::Ecosystem(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
            SimulationType::Flow(sim) => sim.apply_settings(settings, device, queue),
            SimulationType::Pellets(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
            SimulationType::MainMenu(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
            SimulationType::Gradient(simulation) => {
                simulation.apply_settings(settings, device, queue)
            }
        }
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::GrayScott(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::ParticleLife(simulation) => {
                simulation.reset_runtime_state(device, queue)
            }
            SimulationType::Ecosystem(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::Flow(sim) => sim.reset_runtime_state(device, queue),
            SimulationType::Pellets(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::MainMenu(simulation) => simulation.reset_runtime_state(device, queue),
            SimulationType::Gradient(simulation) => simulation.reset_runtime_state(device, queue),
        }
    }

    fn toggle_gui(&mut self) -> bool {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.toggle_gui(),
            SimulationType::GrayScott(simulation) => simulation.toggle_gui(),
            SimulationType::ParticleLife(simulation) => simulation.toggle_gui(),
            SimulationType::Ecosystem(simulation) => simulation.toggle_gui(),
            SimulationType::Flow(sim) => sim.toggle_gui(),
            SimulationType::Pellets(simulation) => simulation.toggle_gui(),
            SimulationType::MainMenu(simulation) => simulation.toggle_gui(),
            SimulationType::Gradient(simulation) => simulation.toggle_gui(),
        }
    }

    fn is_gui_visible(&self) -> bool {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.is_gui_visible(),
            SimulationType::GrayScott(simulation) => simulation.is_gui_visible(),
            SimulationType::ParticleLife(simulation) => simulation.is_gui_visible(),
            SimulationType::Ecosystem(simulation) => simulation.is_gui_visible(),
            SimulationType::Flow(sim) => sim.is_gui_visible(),
            SimulationType::Pellets(simulation) => simulation.is_gui_visible(),
            SimulationType::MainMenu(simulation) => simulation.is_gui_visible(),
            SimulationType::Gradient(simulation) => simulation.is_gui_visible(),
        }
    }

    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.randomize_settings(device, queue),
            SimulationType::GrayScott(simulation) => simulation.randomize_settings(device, queue),
            SimulationType::ParticleLife(simulation) => {
                simulation.randomize_settings(device, queue)
            }
            SimulationType::Ecosystem(simulation) => simulation.randomize_settings(device, queue),
            SimulationType::Flow(sim) => sim.randomize_settings(device, queue),
            SimulationType::Pellets(simulation) => simulation.randomize_settings(device, queue),
            SimulationType::MainMenu(simulation) => simulation.randomize_settings(device, queue),
            SimulationType::Gradient(simulation) => simulation.randomize_settings(device, queue),
        }
    }
}
