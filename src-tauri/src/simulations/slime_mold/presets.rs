//! Presets for the simulation

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use toml;
use super::settings::{GradientType, Settings};
use dirs::home_dir;

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

    /// Get list of user preset files (without built-in presets)
    pub fn get_user_preset_names(&self) -> Vec<String> {
        self.presets
            .iter()
            .filter(|p| !self.built_in_preset_names.contains(&p.name))
            .map(|p| p.name.clone())
            .collect()
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
            pheromone_decay_rate: 3.5,
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
            pheromone_decay_rate: 10.0,

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
            pheromone_decay_rate: 5.0,
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
            pheromone_decay_rate: 3.0,
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
            pheromone_decay_rate: 40.0,
            ..Settings::default()
        },
    ));
    preset_manager.add_preset(Preset::new(
        "Bars".to_string(),
        Settings {
            agent_jitter: 3.949_936_4,
            agent_sensor_angle: 2.193_287_4,
            agent_sensor_distance: 443.473_57,
            agent_speed_max: 482.086_7,
            agent_speed_min: 426.720_86,
            agent_turn_rate: 4.969_109_5,
            pheromone_decay_rate: 10.0,
            pheromone_deposition_rate: 0.435_905_75,
            pheromone_diffusion_rate: 0.474_814_41,
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
            agent_jitter: 3.164_667_1,
            agent_sensor_angle: 1.250_608_9,
            agent_sensor_distance: 8.729_994,
            agent_speed_max: 479.033_1,
            agent_speed_min: 294.058_1,
            agent_turn_rate: 0.887_346_15,
            pheromone_decay_rate: 15.0,
            pheromone_deposition_rate: 0.525_721_9,
            pheromone_diffusion_rate: 0.243_336_98,
            ..Settings::default()
        },
    ));
    preset_manager.add_preset(Preset::new(
        "Sand On A Speaker".to_string(),
        Settings {
            agent_jitter: 2.991_177,
            agent_sensor_angle: 0.642_961_9,
            agent_sensor_distance: 144.372_2,
            agent_speed_max: 447.087_68,
            agent_speed_min: 416.390_87,
            agent_turn_rate: 2.136_445_8,
            pheromone_decay_rate: 50.0,
            pheromone_deposition_rate: 0.633_740_1,
            pheromone_diffusion_rate: 0.079_050_72,
            ..Settings::default()
        },
    ));
    preset_manager.add_preset(Preset::new(
        "Spots".to_string(),
        Settings {
            agent_jitter: 0.254_688_26,
            agent_sensor_angle: 1.547_680_5,
            agent_sensor_distance: 31.146_05,
            agent_speed_max: 350.695_13,
            agent_speed_min: 300.851_14,
            agent_turn_rate: 4.500_079_6,
            pheromone_decay_rate: 15.0,
            pheromone_deposition_rate: 0.228_417_04,
            pheromone_diffusion_rate: 0.062_788_37,
            ..Settings::default()
        },
    ));
    preset_manager.add_preset(Preset::new(
        "Cascades".to_string(),
        Settings {
            agent_jitter: 4.625_645_6,
            agent_sensor_angle: 0.897_250_9,
            agent_sensor_distance: 239.661_82,
            agent_speed_max: 381.274_63,
            agent_speed_min: 276.855_5,
            agent_turn_rate: 0.733_131_2,
            pheromone_decay_rate: 3.1,
            pheromone_deposition_rate: 0.277_263_16,
            pheromone_diffusion_rate: 0.660_592_73,
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
