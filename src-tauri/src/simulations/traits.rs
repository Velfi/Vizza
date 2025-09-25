//! # Simulation Traits Module
//!
//! Defines the core interfaces that unify all simulations in the Vizza application.
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
use crate::simulations::shared::ColorMode;
use serde_json::Value;
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

/// Helper macro to reduce boilerplate in enum implementations
macro_rules! delegate_to_simulation {
    ($self:expr, $method:ident) => {
        match $self {
            SimulationType::SlimeMold(simulation) => simulation.$method(),
            SimulationType::GrayScott(simulation) => simulation.$method(),
            SimulationType::ParticleLife(simulation) => simulation.$method(),
            SimulationType::Flow(simulation) => simulation.$method(),
            SimulationType::Pellets(simulation) => simulation.$method(),
            SimulationType::MainMenu(simulation) => simulation.$method(),
            SimulationType::Gradient(simulation) => simulation.$method(),
            SimulationType::VoronoiCA(simulation) => simulation.$method(),
            SimulationType::Moire(simulation) => simulation.$method(),
            SimulationType::PrimordialParticles(simulation) => simulation.$method(),
        }
    };
    ($self:expr, $method:ident, $($arg:expr),+) => {
        match $self {
            SimulationType::SlimeMold(simulation) => simulation.$method($($arg),+),
            SimulationType::GrayScott(simulation) => simulation.$method($($arg),+),
            SimulationType::ParticleLife(simulation) => simulation.$method($($arg),+),
            SimulationType::Flow(simulation) => simulation.$method($($arg),+),
            SimulationType::Pellets(simulation) => simulation.$method($($arg),+),
            SimulationType::MainMenu(simulation) => simulation.$method($($arg),+),
            SimulationType::Gradient(simulation) => simulation.$method($($arg),+),
            SimulationType::VoronoiCA(simulation) => simulation.$method($($arg),+),
            SimulationType::Moire(simulation) => simulation.$method($($arg),+),
            SimulationType::PrimordialParticles(simulation) => simulation.$method($($arg),+),
        }
    };
}

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
        delta_time: f32,
    ) -> SimulationResult<()>;

    /// Render a static frame without updating simulation state (for paused mode)
    fn render_frame_paused(
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

    /// Update a specific state parameter by name
    ///
    /// This method should only modify runtime state that is not saved in presets.
    /// Examples: LUT selection, color modes, cursor size, etc.
    fn update_state(
        &mut self,
        state_name: &str,
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
    fn pan_camera(&mut self, _delta_x: f32, _delta_y: f32) {
        // Default implementation: no-op for simulations without cameras
    }

    /// Zoom the camera by the given delta
    fn zoom_camera(&mut self, _delta: f32) {
        // Default implementation: no-op for simulations without cameras
    }

    /// Zoom the camera to a specific cursor position
    fn zoom_camera_to_cursor(&mut self, _delta: f32, _cursor_x: f32, _cursor_y: f32) {
        // Default implementation: no-op for simulations without cameras
    }

    /// Reset the camera to default position and zoom
    fn reset_camera(&mut self) {
        // Default implementation: no-op for simulations without cameras
    }

    /// Get the current camera state as a serializable value
    fn get_camera_state(&self) -> Value {
        // Default implementation: empty object for simulations without cameras
        serde_json::json!({})
    }

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
    fn toggle_gui(&mut self) -> bool {
        // Default implementation: no-op, returns false
        false
    }

    /// Check if GUI is visible
    fn is_gui_visible(&self) -> bool {
        // Default implementation: always visible
        true
    }

    /// Randomize the current settings
    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()>;

    /// Update the color scheme with new data
    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
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
    SlimeMold(Box<crate::simulations::slime_mold::SlimeMoldModel>),
    GrayScott(Box<crate::simulations::gray_scott::GrayScottModel>),
    ParticleLife(Box<crate::simulations::particle_life::ParticleLifeModel>),
    Flow(Box<crate::simulations::flow::simulation::FlowModel>),
    Pellets(Box<crate::simulations::pellets::PelletsModel>),
    MainMenu(Box<crate::simulations::main_menu::MainMenuModel>),
    Gradient(Box<crate::simulations::gradient::GradientSimulation>),
    VoronoiCA(Box<crate::simulations::voronoi_ca::simulation::VoronoiCASimulation>),
    Moire(Box<crate::simulations::moire::MoireModel>),
    PrimordialParticles(Box<crate::simulations::primordial_particles::PrimordialParticlesModel>),
}

impl SimulationType {
    /// Create a new simulation of the specified type
    pub async fn new(
        simulation_type: &str,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
        color_scheme_manager: &crate::simulations::shared::ColorSchemeManager,
        app_settings: &crate::commands::AppSettings,
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
                    app_settings,
                    color_scheme_manager,
                )?;
                Ok(SimulationType::SlimeMold(Box::new(simulation)))
            }
            "gray_scott" => {
                let settings = crate::simulations::gray_scott::settings::Settings::default();

                // Use full surface resolution for simulation
                let sim_width = surface_config.width.max(256);
                let sim_height = surface_config.height.max(256);

                let simulation = crate::simulations::gray_scott::GrayScottModel::new(
                    device,
                    queue,
                    surface_config,
                    sim_width,
                    sim_height,
                    settings,
                    crate::simulations::gray_scott::state::State::default(),
                    color_scheme_manager,
                    app_settings,
                )?;
                Ok(SimulationType::GrayScott(Box::new(simulation)))
            }
            "particle_life" => {
                let settings = crate::simulations::particle_life::settings::Settings::default();
                let simulation = crate::simulations::particle_life::ParticleLifeModel::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    15000,
                    settings,
                    app_settings,
                    color_scheme_manager,
                    ColorMode::ColorScheme,
                )?;
                Ok(SimulationType::ParticleLife(Box::new(simulation)))
            }
            "flow" => {
                let settings = crate::simulations::flow::settings::Settings::default();
                let simulation = crate::simulations::flow::simulation::FlowModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    app_settings,
                    color_scheme_manager,
                )?;
                Ok(SimulationType::Flow(Box::new(simulation)))
            }
            "pellets" => {
                let settings = crate::simulations::pellets::settings::Settings::default();
                let simulation = crate::simulations::pellets::simulation::PelletsModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    app_settings,
                    color_scheme_manager,
                )?;
                Ok(SimulationType::Pellets(Box::new(simulation)))
            }
            "gradient" => {
                let simulation = crate::simulations::gradient::GradientSimulation::new(
                    device,
                    queue,
                    surface_config.format,
                    app_settings,
                );
                Ok(SimulationType::Gradient(Box::new(simulation)))
            }
            "main_menu" => {
                let simulation = crate::simulations::main_menu::MainMenuModel::new(
                    device,
                    surface_config,
                    color_scheme_manager,
                    app_settings,
                )?;
                Ok(SimulationType::MainMenu(Box::new(simulation)))
            }
            "voronoi_ca" => {
                let simulation =
                    crate::simulations::voronoi_ca::simulation::VoronoiCASimulation::new(
                        device,
                        queue,
                        surface_config,
                        app_settings,
                    )?;
                Ok(SimulationType::VoronoiCA(Box::new(simulation)))
            }
            "moire" => {
                let settings = crate::simulations::moire::settings::Settings::default();

                let simulation = crate::simulations::moire::MoireModel::new(
                    device,
                    queue,
                    surface_config,
                    settings,
                    app_settings,
                    color_scheme_manager,
                )?;
                Ok(SimulationType::Moire(Box::new(simulation)))
            }
            "primordial_particles" => {
                use crate::simulations::primordial_particles::{
                    PrimordialParticlesModel, settings::Settings, state::State,
                };

                let settings = Settings::default();
                let state = State::new(surface_config.width, surface_config.height);

                let simulation = PrimordialParticlesModel::new(
                    device,
                    queue,
                    surface_config,
                    &settings,
                    &state,
                )?;
                Ok(SimulationType::PrimordialParticles(Box::new(simulation)))
            }
            _ => Err(format!("Unknown simulation type: {}", simulation_type).into()),
        }
    }

    pub fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, reset_runtime_state, device, queue)
    }
}

impl Simulation for SimulationType {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
        delta_time: f32,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, render_frame, device, queue, surface_view, delta_time)
    }

    fn render_frame_paused(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, render_frame_paused, device, queue, surface_view)
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        match self {
            SimulationType::GrayScott(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::SlimeMold(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::ParticleLife(simulation) => {
                simulation.resize(device, queue, new_config)
            }
            SimulationType::Flow(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::Pellets(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::MainMenu(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::Gradient(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::VoronoiCA(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::Moire(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::PrimordialParticles(simulation) => {
                simulation.resize(device, queue, new_config)
            }
        }
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, update_setting, setting_name, value, device, queue)
    }

    fn update_state(
        &mut self,
        state_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, update_state, state_name, value, device, queue)
    }

    fn get_settings(&self) -> Value {
        delegate_to_simulation!(self, get_settings)
    }

    fn get_state(&self) -> Value {
        delegate_to_simulation!(self, get_state)
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32, // 0 = left, 1 = middle, 2 = right
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(
            self,
            handle_mouse_interaction,
            world_x,
            world_y,
            mouse_button,
            device,
            queue
        )
    }

    fn handle_mouse_release(
        &mut self,
        mouse_button: u32, // 0 = left, 1 = middle, 2 = right
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, handle_mouse_release, mouse_button, queue)
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        delegate_to_simulation!(self, pan_camera, delta_x, delta_y)
    }

    fn zoom_camera(&mut self, delta: f32) {
        delegate_to_simulation!(self, zoom_camera, delta)
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        delegate_to_simulation!(self, zoom_camera_to_cursor, delta, cursor_x, cursor_y)
    }

    fn reset_camera(&mut self) {
        delegate_to_simulation!(self, reset_camera)
    }

    fn get_camera_state(&self) -> Value {
        delegate_to_simulation!(self, get_camera_state)
    }

    fn save_preset(&self, preset_name: &str) -> SimulationResult<()> {
        delegate_to_simulation!(self, save_preset, preset_name)
    }

    fn load_preset(&mut self, preset_name: &str, queue: &Arc<Queue>) -> SimulationResult<()> {
        delegate_to_simulation!(self, load_preset, preset_name, queue)
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, apply_settings, settings, device, queue)
    }

    fn reset_runtime_state(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, reset_runtime_state, device, queue)
    }

    fn toggle_gui(&mut self) -> bool {
        delegate_to_simulation!(self, toggle_gui)
    }

    fn is_gui_visible(&self) -> bool {
        delegate_to_simulation!(self, is_gui_visible)
    }

    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, randomize_settings, device, queue)
    }

    fn update_color_scheme(
        &mut self,
        color_scheme: &crate::simulations::shared::ColorScheme,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        delegate_to_simulation!(self, update_color_scheme, color_scheme, device, queue)
    }
}
