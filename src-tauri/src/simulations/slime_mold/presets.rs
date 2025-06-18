//! Presets for the simulation

use std::fs;
use std::path::PathBuf;

use super::settings::{GradientType, Settings};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub settings: Settings,
}

impl Preset {
    pub fn new(name: String, settings: Settings) -> Self {
        Self { name, settings }
    }
}

pub struct PresetManager {
    presets: Vec<Preset>,
    user_presets_dir: PathBuf,
    built_in_preset_names: Vec<String>,
}

impl PresetManager {
    pub fn new() -> Self {
        let user_presets_dir = get_user_presets_dir();
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

    pub fn add_preset(&mut self, preset: Preset) {
        self.presets.push(preset);
    }

    pub fn get_preset(&self, name: &str) -> Option<&Preset> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let preset = Preset {
            name: name.to_string(),
            settings: settings.clone(),
        };

        let toml_content = toml::to_string_pretty(&preset)?;
        let file_path = self
            .user_presets_dir
            .join(format!("{}.toml", sanitize_filename(name)));
        fs::write(file_path, toml_content)?;

        Ok(())
    }

    /// Load user presets from TOML files in the user's Documents folder
    pub fn load_user_presets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.user_presets_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.user_presets_dir)?;

        for entry in entries {
            let entry = entry?;
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
    fn load_preset_from_file(&self, path: &PathBuf) -> Result<Preset, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let preset: Preset = toml::from_str(&content)?;
        Ok(preset)
    }

    /// Delete a user preset file and remove it from memory
    pub fn delete_user_preset(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sanitized_name = sanitize_filename(name);
        let file_path = self
            .user_presets_dir
            .join(format!("{}.toml", sanitized_name));

        // Remove from file system
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }

        // Also remove from memory immediately
        self.presets.retain(|p| p.name != name);

        Ok(())
    }
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the user's Documents folder path and create the slime-mold presets subdirectory path
fn get_user_presets_dir() -> PathBuf {
    let home_dir = home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("slime-mold").join("presets")
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

pub fn init_preset_manager() -> PresetManager {
    let mut preset_manager = PresetManager::new();

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
            pheromone_diffusion_rate: 47.481441,
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
            pheromone_diffusion_rate: 66.059273,
            ..Settings::default()
        },
    ));

    // Capture all the built-in preset names we just added
    preset_manager.capture_built_in_presets();

    // Load user presets from TOML files
    if let Err(e) = preset_manager.load_user_presets() {
        eprintln!("Warning: Could not load user presets: {}", e);
    }

    preset_manager
}
