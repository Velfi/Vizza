use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use wgpu::Device;
use wgpu::Queue;

use crate::commands::get_settings_dir;
use crate::error::PresetError;
use crate::error::PresetResult;
use serde::{Deserialize, Serialize};
use toml;

use crate::simulations::traits::Simulation;
use crate::simulations::traits::SimulationType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset<Settings> {
    pub name: String,
    pub settings: Settings,
}

impl<Settings> Preset<Settings> {
    pub fn new(name: String, settings: Settings) -> Self {
        Self { name, settings }
    }
}

pub struct PresetManager<Settings> {
    presets: Vec<Preset<Settings>>,
    user_presets_dir: PathBuf,
    built_in_preset_names: Vec<String>,
}

impl<Settings> PresetManager<Settings>
where
    Settings: Clone + Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(simulation_name: String) -> Self {
        let user_presets_dir = get_user_presets_dir(&simulation_name);
        let manager = Self {
            presets: vec![],
            user_presets_dir,
            built_in_preset_names: vec![],
        };

        // Create the user presets directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&manager.user_presets_dir) {
            eprintln!("Warning: Could not create user presets directory: {}", e);
        }

        manager
    }

    pub fn add_preset(&mut self, preset: Preset<Settings>) {
        self.presets.push(preset);
    }

    pub fn get_preset(&self, name: &str) -> Option<&Preset<Settings>> {
        self.presets.iter().find(|p| p.name == name)
    }

    pub fn get_preset_names(&self) -> Vec<String> {
        self.presets.iter().map(|p| p.name.clone()).collect()
    }

    /// Capture the current preset names as built-in presets
    pub fn capture_built_in_presets(&mut self) {
        self.built_in_preset_names = self.presets.iter().map(|p| p.name.clone()).collect();
    }

    /// Save a preset to a TOML file in the user's Documents folder
    pub fn save_user_preset(&self, name: &str, settings: &Settings) -> PresetResult<()> {
        let preset = Preset {
            name: name.to_string(),
            settings: settings.clone(),
        };

        let toml_content = toml::to_string_pretty(&preset)
            .map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
        let path = self
            .user_presets_dir
            .join(format!("{}.toml", sanitize_filename(name)));
        fs::write(&path, toml_content).map_err(|e| PresetError::FileError {
            path,
            error: e.to_string(),
        })?;

        Ok(())
    }

    /// Load user presets from TOML files in the user's Documents folder
    pub fn load_user_presets(&mut self) -> PresetResult<()> {
        if !self.user_presets_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.user_presets_dir).map_err(|e| PresetError::FileError {
            path: self.user_presets_dir.clone(),
            error: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| PresetError::FileError {
                path: self.user_presets_dir.clone(),
                error: e.to_string(),
            })?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match self.load_preset_from_file(&path) {
                    Ok(preset) => {
                        // Check if this preset name already exists (avoid duplicates)
                        if !self.presets.iter().any(|p| p.name == preset.name) {
                            self.presets.push(preset);
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not load preset from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single preset from a TOML file
    fn load_preset_from_file(&self, path: &PathBuf) -> PresetResult<Preset<Settings>> {
        let content = fs::read_to_string(path).map_err(|e| PresetError::FileError {
            path: path.clone(),
            error: e.to_string(),
        })?;
        let preset: Preset<Settings> = toml::from_str(&content)
            .map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
        Ok(preset)
    }

    /// Delete a user preset file and remove it from memory
    pub fn delete_user_preset(&mut self, name: &str) -> PresetResult<()> {
        let sanitized_name = sanitize_filename(name);
        let file_path = self
            .user_presets_dir
            .join(format!("{}.toml", sanitized_name));

        // Remove from file system
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| PresetError::FileError {
                path: file_path.clone(),
                error: e.to_string(),
            })?;
        }

        // Also remove from memory immediately
        self.presets.retain(|p| p.name != name);

        Ok(())
    }

    /// Get a preset by name and return its settings
    pub fn get_preset_settings(&self, name: &str) -> Option<&Settings> {
        self.get_preset(name).map(|p| &p.settings)
    }
}

impl<Settings> Default for PresetManager<Settings>
where
    Settings: Clone + Serialize + for<'de> Deserialize<'de>,
{
    fn default() -> Self {
        Self::new("default".to_string())
    }
}

/// Create the Vizzy/simulation-specific presets subdirectory path
fn get_user_presets_dir(simulation_name: &str) -> PathBuf {
    get_settings_dir().join(simulation_name).join("presets")
}

/// Sanitize filename to be safe for filesystem
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            _ => c,
        })
        .collect()
}

// Type aliases for convenience
pub type SlimeMoldPresetManager = PresetManager<crate::simulations::slime_mold::settings::Settings>;
pub type GrayScottPresetManager = PresetManager<crate::simulations::gray_scott::settings::Settings>;
pub type ParticleLifePresetManager =
    PresetManager<crate::simulations::particle_life::settings::Settings>;
pub type WanderersPresetManager = PresetManager<crate::simulations::wanderers::settings::Settings>;

// Trait for unified preset manager operations
pub trait AnyPresetManager {
    fn get_preset_names(&self) -> Vec<String>;
    fn delete_user_preset(&mut self, name: &str) -> PresetResult<()>;
    fn save_user_preset_json(&self, name: &str, settings: &serde_json::Value) -> PresetResult<()>;
}

// Implement the trait for each specific preset manager type
impl AnyPresetManager for SlimeMoldPresetManager {
    fn get_preset_names(&self) -> Vec<String> {
        self.get_preset_names()
    }

    fn delete_user_preset(&mut self, name: &str) -> PresetResult<()> {
        self.delete_user_preset(name)
    }

    fn save_user_preset_json(&self, name: &str, settings: &serde_json::Value) -> PresetResult<()> {
        let typed_settings: crate::simulations::slime_mold::settings::Settings =
            serde_json::from_value(settings.clone())
                .map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
        self.save_user_preset(name, &typed_settings)
    }
}

impl AnyPresetManager for GrayScottPresetManager {
    fn get_preset_names(&self) -> Vec<String> {
        self.get_preset_names()
    }

    fn delete_user_preset(&mut self, name: &str) -> PresetResult<()> {
        self.delete_user_preset(name)
    }

    fn save_user_preset_json(&self, name: &str, settings: &serde_json::Value) -> PresetResult<()> {
        let typed_settings: crate::simulations::gray_scott::settings::Settings =
            serde_json::from_value(settings.clone())
                .map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
        self.save_user_preset(name, &typed_settings)
    }
}

impl AnyPresetManager for ParticleLifePresetManager {
    fn get_preset_names(&self) -> Vec<String> {
        self.get_preset_names()
    }

    fn delete_user_preset(&mut self, name: &str) -> PresetResult<()> {
        self.delete_user_preset(name)
    }

    fn save_user_preset_json(&self, name: &str, settings: &serde_json::Value) -> PresetResult<()> {
        let typed_settings: crate::simulations::particle_life::settings::Settings =
            serde_json::from_value(settings.clone())
                .map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
        self.save_user_preset(name, &typed_settings)
    }
}

impl AnyPresetManager for WanderersPresetManager {
    fn get_preset_names(&self) -> Vec<String> {
        self.get_preset_names()
    }

    fn delete_user_preset(&mut self, name: &str) -> PresetResult<()> {
        self.delete_user_preset(name)
    }

    fn save_user_preset_json(&self, name: &str, settings: &serde_json::Value) -> PresetResult<()> {
        let typed_settings: crate::simulations::wanderers::settings::Settings =
            serde_json::from_value(settings.clone())
                .map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
        self.save_user_preset(name, &typed_settings)
    }
}

// Enum to hold different types of preset managers
pub enum PresetManagerType {
    SlimeMold(SlimeMoldPresetManager),
    GrayScott(GrayScottPresetManager),
    ParticleLife(ParticleLifePresetManager),
    Wanderers(WanderersPresetManager),
}

impl PresetManagerType {
    fn as_any_preset_manager(&self) -> &dyn AnyPresetManager {
        match self {
            PresetManagerType::SlimeMold(manager) => manager,
            PresetManagerType::GrayScott(manager) => manager,
            PresetManagerType::ParticleLife(manager) => manager,
            PresetManagerType::Wanderers(manager) => manager,
        }
    }

    fn as_any_preset_manager_mut(&mut self) -> &mut dyn AnyPresetManager {
        match self {
            PresetManagerType::SlimeMold(manager) => manager,
            PresetManagerType::GrayScott(manager) => manager,
            PresetManagerType::ParticleLife(manager) => manager,
            PresetManagerType::Wanderers(manager) => manager,
        }
    }

    fn get_preset_settings_for_simulation(
        &self,
        preset_name: &str,
        simulation: &mut SimulationType,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> PresetResult<()> {
        match (self, simulation) {
            (PresetManagerType::SlimeMold(manager), SimulationType::SlimeMold(sim)) => {
                if let Some(settings) = manager.get_preset_settings(preset_name) {
                    let settings_json = serde_json::to_value(settings)
                        .map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
                    sim.apply_settings(settings_json, device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    sim.reset_runtime_state(device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    tracing::info!("Applied slime mold preset '{}'", preset_name);
                    Ok(())
                } else {
                    Err(format!("Preset '{}' not found for Slime Mold", preset_name).into())
                }
            }
            (PresetManagerType::GrayScott(manager), SimulationType::GrayScott(sim)) => {
                if let Some(settings) = manager.get_preset_settings(preset_name) {
                    let settings_json = serde_json::to_value(settings)
                        .map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
                    sim.apply_settings(settings_json, device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    sim.reset_runtime_state(device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    tracing::info!("Applied Gray-Scott preset '{}'", preset_name);
                    Ok(())
                } else {
                    Err(format!("Preset '{}' not found for Gray-Scott", preset_name).into())
                }
            }
            (PresetManagerType::ParticleLife(manager), SimulationType::ParticleLife(sim)) => {
                if let Some(settings) = manager.get_preset_settings(preset_name) {
                    let settings_json = serde_json::to_value(settings)
                        .map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
                    sim.apply_settings(settings_json, device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    sim.reset_runtime_state(device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    tracing::info!("Applied Particle Life preset '{}'", preset_name);
                    Ok(())
                } else {
                    Err(format!("Preset '{}' not found for Particle Life", preset_name).into())
                }
            }
            (PresetManagerType::Wanderers(manager), SimulationType::Wanderers(sim)) => {
                if let Some(settings) = manager.get_preset_settings(preset_name) {
                    let settings_json = serde_json::to_value(settings)
                        .map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
                    sim.apply_settings(settings_json, device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    sim.reset_runtime_state(device, queue)
                        .map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    tracing::info!("Applied Wanderers preset '{}'", preset_name);
                    Ok(())
                } else {
                    Err(format!("Preset '{}' not found for Wanderers", preset_name).into())
                }
            }
            (_, SimulationType::Flow(_)) => {
                Err("Flow simulation presets not yet implemented".into())
            }
            (_, SimulationType::Ecosystem(_)) => {
                Err("Ecosystem simulation presets not yet implemented".into())
            }
            (_, SimulationType::MainMenu(_)) => Err("Main menu does not support presets".into()),
            _ => Err("Simulation type does not match preset manager type".into()),
        }
    }
}

// Wrapper struct to hold multiple preset managers using HashMap
pub struct SimulationPresetManager {
    managers: HashMap<String, PresetManagerType>,
}

impl SimulationPresetManager {
    pub fn new() -> Self {
        let mut slime_mold_preset_manager = SlimeMoldPresetManager::new("slime_mold".to_string());
        let mut gray_scott_preset_manager = GrayScottPresetManager::new("gray_scott".to_string());
        let mut particle_life_preset_manager =
            ParticleLifePresetManager::new("particle_life".to_string());
        let mut wanderers_preset_manager = WanderersPresetManager::new("wanderers".to_string());

        crate::simulations::slime_mold::init_presets(&mut slime_mold_preset_manager);
        crate::simulations::gray_scott::init_presets(&mut gray_scott_preset_manager);
        crate::simulations::particle_life::init_presets(&mut particle_life_preset_manager);
        crate::simulations::wanderers::init_presets(&mut wanderers_preset_manager);

        let mut managers = HashMap::new();
        managers.insert(
            "slime_mold".to_string(),
            PresetManagerType::SlimeMold(slime_mold_preset_manager),
        );
        managers.insert(
            "gray_scott".to_string(),
            PresetManagerType::GrayScott(gray_scott_preset_manager),
        );
        managers.insert(
            "particle_life".to_string(),
            PresetManagerType::ParticleLife(particle_life_preset_manager),
        );
        managers.insert(
            "wanderers".to_string(),
            PresetManagerType::Wanderers(wanderers_preset_manager),
        );

        Self { managers }
    }

    fn get_simulation_type_name(simulation_type: &SimulationType) -> &'static str {
        match simulation_type {
            SimulationType::SlimeMold(_) => "slime_mold",
            SimulationType::GrayScott(_) => "gray_scott",
            SimulationType::ParticleLife(_) => "particle_life",
            SimulationType::Wanderers(_) => "wanderers",
            SimulationType::Ecosystem(_) => "ecosystem",
            SimulationType::Flow(_) => "flow",
            SimulationType::MainMenu(_) => "main_menu",
        }
    }

    pub fn get_available_presets(&self, simulation_type: &SimulationType) -> Vec<String> {
        let sim_name = Self::get_simulation_type_name(simulation_type);

        if sim_name == "main_menu" {
            return vec![]; // No presets for main menu background
        }

        let presets = self
            .managers
            .get(sim_name)
            .map(|manager| manager.as_any_preset_manager().get_preset_names())
            .unwrap_or_default();

        tracing::info!("{} presets: {:?}", sim_name, presets);
        presets
    }

    pub fn apply_preset(
        &self,
        simulation: &mut SimulationType,
        preset_name: &str,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> PresetResult<()> {
        let sim_name = Self::get_simulation_type_name(simulation);

        if sim_name == "main_menu" {
            return Err("No presets available for Main Menu Background".into());
        }

        if let Some(manager) = self.managers.get(sim_name) {
            manager.get_preset_settings_for_simulation(preset_name, simulation, device, queue)
        } else {
            Err(format!("No preset manager found for simulation type: {}", sim_name).into())
        }
    }

    pub fn save_preset(
        &self,
        simulation: &SimulationType,
        preset_name: &str,
        settings: &serde_json::Value,
    ) -> PresetResult<()> {
        let sim_name = Self::get_simulation_type_name(simulation);

        if sim_name == "main_menu" {
            return Err("Cannot save presets for Main Menu Background".into());
        }

        if let Some(manager) = self.managers.get(sim_name) {
            manager
                .as_any_preset_manager()
                .save_user_preset_json(preset_name, settings)
        } else {
            Err(format!("No preset manager found for simulation type: {}", sim_name).into())
        }
    }

    pub fn delete_preset(
        &mut self,
        simulation_type: &SimulationType,
        preset_name: &str,
    ) -> PresetResult<()> {
        let sim_name = Self::get_simulation_type_name(simulation_type);

        if sim_name == "main_menu" {
            return Err("Cannot delete presets for Main Menu Background".into());
        }

        if let Some(manager) = self.managers.get_mut(sim_name) {
            manager
                .as_any_preset_manager_mut()
                .delete_user_preset(preset_name)?;
            tracing::info!("Deleted {} preset '{}'", sim_name, preset_name);
            Ok(())
        } else {
            Err(format!("No preset manager found for simulation type: {}", sim_name).into())
        }
    }

    // Getter methods for accessing the specific preset managers
    pub fn get_manager(&self, sim_name: &str) -> Option<&dyn AnyPresetManager> {
        self.managers
            .get(sim_name)
            .map(|m| m.as_any_preset_manager())
    }
}
