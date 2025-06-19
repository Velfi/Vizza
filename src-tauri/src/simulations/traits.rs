use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};
use serde_json::Value;

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
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Handle window resize events
    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>>;

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
    ) -> Result<(), Box<dyn std::error::Error>>;

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

    /// Handle mouse interaction for seeding or other interactions
    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        is_seeding: bool,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>>;

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
    fn save_preset(&self, _preset_name: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// Load settings from a preset and reset runtime state
    /// 
    /// This should load settings and reset any runtime state to default values.
    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>>;

    /// Update the simulation settings directly
    /// 
    /// This should apply new settings to the simulation without resetting runtime state.
    fn apply_settings(&mut self, settings: serde_json::Value, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>>;

    /// Reset the simulation's runtime state
    /// 
    /// This should reset runtime state (like agent positions, trail maps) but preserve settings.
    fn reset_runtime_state(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>>;

    /// Get the name of the simulation type
    fn get_simulation_type(&self) -> &str;

    /// Check if the simulation is currently running
    fn is_running(&self) -> bool;

    /// Get the current agent count (if applicable)
    fn get_agent_count(&self) -> Option<u32>;

    /// Update the agent count (if applicable)
    async fn update_agent_count(
        &mut self,
        count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Toggle GUI visibility
    fn toggle_gui(&mut self) -> bool;

    /// Check if GUI is visible
    fn is_gui_visible(&self) -> bool;

    /// Randomize the current settings
    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Enum wrapper for all simulation types
/// 
/// This provides a type-safe way to handle different simulation types
/// without using trait objects (Box<dyn Simulation>).
#[derive(Debug)]
pub enum SimulationType {
    SlimeMold(crate::simulations::slime_mold::SlimeMoldModel),
    GrayScott(crate::simulations::gray_scott::GrayScottModel),
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
            _ => Err(format!("Unknown simulation type: {}", simulation_type).into()),
        }
    }
}

impl Simulation for SimulationType {
    fn render_frame(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.render_frame(device, queue, surface_view),
            SimulationType::GrayScott(simulation) => simulation.render_frame(device, queue, surface_view).map_err(|e| e.into()),
        }
    }

    fn resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.resize(device, queue, new_config),
            SimulationType::GrayScott(simulation) => simulation.resize(new_config),
        }
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.update_setting(setting_name, value, device, queue),
            SimulationType::GrayScott(simulation) => simulation.update_setting(setting_name, value, device, queue),
        }
    }

    fn get_settings(&self) -> Value {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_settings(),
            SimulationType::GrayScott(simulation) => simulation.get_settings(),
        }
    }

    fn get_state(&self) -> Value {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_state(),
            SimulationType::GrayScott(simulation) => simulation.get_state(),
        }
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        is_seeding: bool,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.handle_mouse_interaction(world_x, world_y, is_seeding, queue),
            SimulationType::GrayScott(simulation) => simulation.handle_mouse_interaction(world_x, world_y, is_seeding, queue),
        }
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.pan_camera(delta_x, delta_y),
            SimulationType::GrayScott(simulation) => simulation.pan_camera(delta_x, delta_y),
        }
    }

    fn zoom_camera(&mut self, delta: f32) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.zoom_camera(delta),
            SimulationType::GrayScott(simulation) => simulation.zoom_camera(delta),
        }
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y),
            SimulationType::GrayScott(simulation) => simulation.zoom_camera_to_cursor(delta, cursor_x, cursor_y),
        }
    }

    fn reset_camera(&mut self) {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.reset_camera(),
            SimulationType::GrayScott(simulation) => simulation.reset_camera(),
        }
    }

    fn get_camera_state(&self) -> Value {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_camera_state(),
            SimulationType::GrayScott(simulation) => simulation.get_camera_state(),
        }
    }

    fn save_preset(&self, preset_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.save_preset(preset_name),
            SimulationType::GrayScott(simulation) => simulation.save_preset(preset_name),
        }
    }

    fn load_preset(&mut self, preset_name: &str, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.load_preset(preset_name, queue),
            SimulationType::GrayScott(simulation) => simulation.load_preset(preset_name, queue),
        }
    }

    fn apply_settings(&mut self, settings: serde_json::Value, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.apply_settings(settings, queue),
            SimulationType::GrayScott(simulation) => simulation.apply_settings(settings, queue),
        }
    }

    fn reset_runtime_state(&mut self, queue: &Arc<Queue>) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.reset_runtime_state(queue),
            SimulationType::GrayScott(simulation) => simulation.reset_runtime_state(queue),
        }
    }

    fn get_simulation_type(&self) -> &str {
        match self {
            SimulationType::SlimeMold(_) => "slime_mold",
            SimulationType::GrayScott(_) => "gray_scott",
        }
    }

    fn is_running(&self) -> bool {
        match self {
            SimulationType::SlimeMold(_) => true,
            SimulationType::GrayScott(_) => true,
        }
    }

    fn get_agent_count(&self) -> Option<u32> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.get_agent_count(),
            SimulationType::GrayScott(_) => None,
        }
    }

    async fn update_agent_count(
        &mut self,
        count: u32,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.update_agent_count(count, device, queue, surface_config).await,
            SimulationType::GrayScott(_) => Err("Gray-Scott simulation does not support agent count updates".into()),
        }
    }

    fn toggle_gui(&mut self) -> bool {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.toggle_gui(),
            SimulationType::GrayScott(simulation) => simulation.toggle_gui(),
        }
    }

    fn is_gui_visible(&self) -> bool {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.is_gui_visible(),
            SimulationType::GrayScott(simulation) => simulation.is_gui_visible(),
        }
    }

    fn randomize_settings(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            SimulationType::SlimeMold(simulation) => simulation.randomize_settings(device, queue),
            SimulationType::GrayScott(simulation) => simulation.randomize_settings(device, queue),
        }
    }
}

/// Error types for simulation operations
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("GPU operation failed: {0}")]
    GpuError(#[from] wgpu::Error),
    
    #[error("Invalid setting: {setting_name}")]
    InvalidSetting { setting_name: String },
    
    #[error("Preset operation failed: {0}")]
    PresetError(String),
    
    #[error("Simulation not running")]
    NotRunning,
    
    #[error("Unsupported operation for this simulation type")]
    UnsupportedOperation,
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Result type for simulation operations
pub type SimulationResult<T> = Result<T, SimulationError>; 