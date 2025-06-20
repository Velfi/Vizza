use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use wgpu::Queue;

use serde::{Deserialize, Serialize};
use toml;
use dirs::home_dir;
use crate::error::PresetError;
use crate::error::PresetResult;

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
    simulation_name: String,
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
            simulation_name,
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
    pub fn save_user_preset(
        &self,
        name: &str,
        settings: &Settings,
    ) -> PresetResult<()> {
        let preset = Preset {
            name: name.to_string(),
            settings: settings.clone(),
        };

        let toml_content = toml::to_string_pretty(&preset).map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
        let path = self
            .user_presets_dir
            .join(format!("{}.toml", sanitize_filename(name)));
        fs::write(&path, toml_content).map_err(|e| PresetError::FileError { path, error: e.to_string() })?;

        Ok(())
    }

    /// Load user presets from TOML files in the user's Documents folder
    pub fn load_user_presets(&mut self) -> PresetResult<()> {
        if !self.user_presets_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.user_presets_dir).map_err(|e| PresetError::FileError { path: self.user_presets_dir.clone(), error: e.to_string() })?;

        for entry in entries {
            let entry = entry.map_err(|e| PresetError::FileError { path: self.user_presets_dir.clone(), error: e.to_string() })?;
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
        let content = fs::read_to_string(path).map_err(|e| PresetError::FileError { path: path.clone(), error: e.to_string() })?;
        let preset: Preset<Settings> = toml::from_str(&content).map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
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
            fs::remove_file(&file_path).map_err(|e| PresetError::FileError { path: file_path.clone(), error: e.to_string() })?;
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

/// Get the user's Documents folder path and create the simulation-specific presets subdirectory path
fn get_user_presets_dir(simulation_name: &str) -> PathBuf {
    let home_dir = home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join(simulation_name).join("presets")
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

// Wrapper struct to hold multiple preset managers
pub struct SimulationPresetManager {
    slime_mold_preset_manager: SlimeMoldPresetManager,
    gray_scott_preset_manager: GrayScottPresetManager,
}

impl SimulationPresetManager {
    pub fn new() -> Self {
        let mut slime_mold_preset_manager = PresetManager::new("slime-mold".to_string());
        let mut gray_scott_preset_manager = PresetManager::new("gray-scott".to_string());

        // Initialize slime mold presets
        Self::init_slime_mold_presets(&mut slime_mold_preset_manager);
        
        // Initialize gray scott presets
        Self::init_gray_scott_presets(&mut gray_scott_preset_manager);

        Self {
            slime_mold_preset_manager,
            gray_scott_preset_manager,
        }
    }

    fn init_slime_mold_presets(preset_manager: &mut SlimeMoldPresetManager) {
        use crate::simulations::slime_mold::settings::{GradientType, Settings};

        tracing::info!("Initializing slime mold presets...");

        // Add built-in presets
        preset_manager.add_preset(Preset::new("Default".to_string(), Settings::default()));
        preset_manager.add_preset(Preset::new(
            "Gloop Loops".to_string(),
            Settings {
                agent_jitter: 0.1,
                agent_turn_rate: 0.43,
                agent_speed_max: 300.0,
                agent_sensor_angle: 0.7,
                agent_sensor_distance: 5.0,
                pheromone_decay_rate: 100.0,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Firecracker Trees".to_string(),
            Settings {
                agent_jitter: 0.1,
                agent_turn_rate: 0.93,
                agent_speed_min: 200.0,
                agent_speed_max: 300.0,
                agent_sensor_angle: 0.3,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Threads".to_string(),
            Settings {
                agent_jitter: 0.0,
                agent_turn_rate: 0.02,
                agent_sensor_angle: 0.3,
                agent_speed_min: 50.0,
                agent_speed_max: 150.0,
                pheromone_decay_rate: 100.0,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Snake".to_string(),
            Settings {
                agent_turn_rate: 0.37,
                agent_sensor_angle: 1.34,
                agent_sensor_distance: 225.0,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Cells".to_string(),
            Settings {
                agent_jitter: 0.2,
                agent_turn_rate: 3.27,
                agent_speed_min: 200.0,
                agent_speed_max: 300.0,
                agent_sensor_angle: 1.95,
                agent_sensor_distance: 60.0,
                pheromone_decay_rate: 30.0,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Net".to_string(),
            Settings {
                agent_jitter: 3.0,
                agent_turn_rate: 6.0,
                agent_speed_min: 99.0,
                agent_speed_max: 100.0,
                agent_sensor_angle: 1.57,
                agent_sensor_distance: 225.0,
                pheromone_decay_rate: 400.0,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Bars".to_string(),
            Settings {
                agent_jitter: 3.9499364,
                agent_sensor_angle: 2.1932874,
                agent_sensor_distance: 443.47357,
                agent_speed_max: 482.0867,
                agent_speed_min: 426.72086,
                agent_turn_rate: 4.9691095,
                pheromone_decay_rate: 100.0,
                pheromone_deposition_rate: 43.590575,
                pheromone_diffusion_rate: 47.481_44,
                gradient_type: GradientType::Disabled,
                gradient_strength: 0.5,
                gradient_center_x: 0.5,
                gradient_center_y: 0.5,
                gradient_size: 0.3,
                gradient_angle: 0.0,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Healthy Fungus".to_string(),
            Settings {
                agent_jitter: 3.1646671,
                agent_sensor_angle: 1.2506089,
                agent_sensor_distance: 8.729994,
                agent_speed_max: 479.0331,
                agent_speed_min: 294.0581,
                agent_turn_rate: 0.88734615,
                pheromone_decay_rate: 100.0,
                pheromone_deposition_rate: 52.57219,
                pheromone_diffusion_rate: 24.33,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Sand On A Speaker".to_string(),
            Settings {
                agent_jitter: 2.991177,
                agent_sensor_angle: 0.6429619,
                agent_sensor_distance: 144.3722,
                agent_speed_max: 447.08768,
                agent_speed_min: 416.39087,
                agent_turn_rate: 2.1364458,
                pheromone_decay_rate: 100.0,
                pheromone_deposition_rate: 63.37401,
                pheromone_diffusion_rate: 7.905072,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Spots".to_string(),
            Settings {
                agent_jitter: 0.25468826,
                agent_sensor_angle: 1.5476805,
                agent_sensor_distance: 31.14605,
                agent_speed_max: 350.69513,
                agent_speed_min: 300.85114,
                agent_turn_rate: 4.5000796,
                pheromone_decay_rate: 100.0,
                pheromone_deposition_rate: 22.841704,
                pheromone_diffusion_rate: 6.278837,
                ..Settings::default()
            },
        ));
        preset_manager.add_preset(Preset::new(
            "Cascades".to_string(),
            Settings {
                agent_jitter: 4.6256456,
                agent_sensor_angle: 0.8972509,
                agent_sensor_distance: 239.66182,
                agent_speed_max: 381.27463,
                agent_speed_min: 276.855_5,
                agent_turn_rate: 0.733_131_2,
                pheromone_decay_rate: 100.0,
                pheromone_deposition_rate: 27.726316,
                pheromone_diffusion_rate: 66.059_27,
                ..Settings::default()
            },
        ));

        // Capture all the built-in preset names we just added
        preset_manager.capture_built_in_presets();

        // Load user presets from TOML files
        if let Err(e) = preset_manager.load_user_presets() {
            eprintln!("Warning: Could not load user presets: {}", e);
        }

        let preset_count = preset_manager.get_preset_names().len();
        tracing::info!("Initialized {} slime mold presets", preset_count);
    }

    fn init_gray_scott_presets(preset_manager: &mut GrayScottPresetManager) {
        use crate::simulations::gray_scott::settings::{Settings, NutrientPattern};

        tracing::info!("Initializing Gray-Scott presets...");

        // Add default presets
        let all_presets = [
            ("Brain Coral", (0.0545, 0.062)),
            ("Fingerprint", (0.0545, 0.062)),
            ("Mitosis", (0.0367, 0.0649)),
            ("Ripples", (0.018, 0.051)),
            ("Soliton Collapse", (0.022, 0.06)),
            ("U-Skate World", (0.062, 0.061)),
            ("Undulating", (0.026, 0.051)),
            ("Worms", (0.078, 0.061)),
            ("Custom", (0.035, 0.058)),
        ];

        for (preset_name, (feed_rate, kill_rate)) in all_presets {
            let settings = Settings {
                feed_rate,
                kill_rate,
                diffusion_rate_u: 0.2097,
                diffusion_rate_v: 0.105,
                timestep: 1.0,
                nutrient_pattern: NutrientPattern::Uniform,
                nutrient_pattern_reversed: false,
            };

            preset_manager.add_preset(Preset::new(preset_name.to_string(), settings));
        }

        // Capture all the built-in preset names we just added
        preset_manager.capture_built_in_presets();

        // Load user presets from TOML files
        if let Err(e) = preset_manager.load_user_presets() {
            eprintln!("Warning: Could not load user presets: {}", e);
        }

        let preset_count = preset_manager.get_preset_names().len();
        tracing::info!("Initialized {} Gray-Scott presets", preset_count);
    }

    pub fn get_available_presets(&self, simulation_type: &SimulationType) -> Vec<String> {
        let presets = match simulation_type {
            SimulationType::SlimeMold(_) => {
                let slime_presets = self.slime_mold_preset_manager.get_preset_names();
                tracing::info!("Slime mold presets: {:?}", slime_presets);
                slime_presets
            }
            SimulationType::GrayScott(_) => {
                let gray_scott_presets = self.gray_scott_preset_manager.get_preset_names();
                tracing::info!("Gray-Scott presets: {:?}", gray_scott_presets);
                gray_scott_presets
            }
            SimulationType::ParticleLife(_) => {
                vec![] // Presets not yet implemented for Particle Life
            }
            SimulationType::MainMenu(_) => {
                vec![] // No presets for main menu background
            }
        };
        tracing::info!("Total available presets: {:?}", presets);
        presets
    }

    pub fn apply_preset(
        &self,
        simulation: &mut SimulationType,
        preset_name: &str,
        queue: &Arc<Queue>,
    ) -> PresetResult<()> {
        match simulation {
            SimulationType::SlimeMold(simulation) => {
                if let Some(settings) = self.slime_mold_preset_manager.get_preset_settings(preset_name) {
                    // Apply the settings to the simulation
                    let settings_json = serde_json::to_value(settings).map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
                    simulation.apply_settings(settings_json, queue).map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    // Reset runtime state (trails, agents)
                    simulation.reset_runtime_state(queue).map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    tracing::info!("Applied slime mold preset '{}'", preset_name);
                } else {
                    return Err(format!("Preset '{}' not found for Slime Mold", preset_name).into());
                }
                Ok(())
            }
            SimulationType::GrayScott(simulation) => {
                if let Some(settings) = self.gray_scott_preset_manager.get_preset_settings(preset_name) {
                    // Apply the settings to the simulation
                    let settings_json = serde_json::to_value(settings).map_err(|e| PresetError::SerializationFailed(e.to_string()))?;
                    simulation.apply_settings(settings_json, queue).map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    // Reset runtime state
                    simulation.reset_runtime_state(queue).map_err(|e| PresetError::SimulationError(e.to_string()))?;
                    tracing::info!("Applied Gray-Scott preset '{}'", preset_name);
                } else {
                    return Err(format!("Preset '{}' not found for Gray-Scott", preset_name).into());
                }
                Ok(())
            }
            SimulationType::ParticleLife(_) => {
                Err("Presets not yet implemented for Particle Life".into())
            }
            SimulationType::MainMenu(_) => {
                Err("No presets available for Main Menu Background".into())
            }
        }
    }

    pub fn save_preset(
        &self,
        simulation: &SimulationType,
        preset_name: &str,
        settings: &serde_json::Value,
    ) -> PresetResult<()> {
        match simulation {
            SimulationType::SlimeMold(_) => {
                let slime_settings: crate::simulations::slime_mold::settings::Settings =
                    serde_json::from_value(settings.clone()).map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
                self.slime_mold_preset_manager.save_user_preset(preset_name, &slime_settings)?;
            }
            SimulationType::GrayScott(_) => {
                let gray_scott_settings: crate::simulations::gray_scott::settings::Settings =
                    serde_json::from_value(settings.clone()).map_err(|e| PresetError::DeserializationFailed(e.to_string()))?;
                self.gray_scott_preset_manager.save_user_preset(preset_name, &gray_scott_settings)?;
            }
            SimulationType::ParticleLife(_) => {
                return Err("Presets not yet implemented for Particle Life".into());
            }
            SimulationType::MainMenu(_) => {
                return Err("Cannot save presets for Main Menu Background".into());
            }
        }

        Ok(())
    }

    pub fn delete_preset(
        &self,
        simulation_type: &SimulationType,
        _preset_name: &str,
    ) -> PresetResult<()> {
        match simulation_type {
            SimulationType::SlimeMold(_) => {
                // We need a mutable reference for deletion, so we'll need to restructure this
                // For now, return an error indicating this needs to be handled differently
                Err("Delete preset functionality needs to be implemented with mutable access".into())
            }
            SimulationType::GrayScott(_) => {
                // Same issue here
                Err("Delete preset functionality needs to be implemented with mutable access".into())
            }
            SimulationType::ParticleLife(_) => {
                Err("Presets not yet implemented for Particle Life".into())
            }
            SimulationType::MainMenu(_) => {
                Err("No presets available for Main Menu Background".into())
            }
        }
    }

    // Getter methods for accessing the specific preset managers
    pub fn slime_mold_preset_manager(&self) -> &SlimeMoldPresetManager {
        &self.slime_mold_preset_manager
    }

    pub fn gray_scott_preset_manager(&self) -> &GrayScottPresetManager {
        &self.gray_scott_preset_manager
    }

    pub fn slime_mold_preset_manager_mut(&mut self) -> &mut SlimeMoldPresetManager {
        &mut self.slime_mold_preset_manager
    }

    pub fn gray_scott_preset_manager_mut(&mut self) -> &mut GrayScottPresetManager {
        &mut self.gray_scott_preset_manager
    }
} 