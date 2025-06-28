use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml;

use super::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelPreset {
    BrainCoral,
    Fingerprint,
    Mitosis,
    Ripples,
    SolitonCollapse,
    USkateWorld,
    Undulating,
    Worms,
    Custom,
}

impl ModelPreset {
    pub fn name(&self) -> &'static str {
        match self {
            ModelPreset::BrainCoral => "Brain Coral",
            ModelPreset::Fingerprint => "Fingerprint",
            ModelPreset::Mitosis => "Mitosis",
            ModelPreset::Ripples => "Ripples",
            ModelPreset::SolitonCollapse => "Soliton Collapse",
            ModelPreset::USkateWorld => "U-Skate World",
            ModelPreset::Undulating => "Undulating",
            ModelPreset::Worms => "Worms",
            ModelPreset::Custom => "Custom",
        }
    }

    pub fn get_rates(&self) -> (f32, f32) {
        match self {
            ModelPreset::BrainCoral => (0.0545, 0.062),
            ModelPreset::Fingerprint => (0.0545, 0.062),
            ModelPreset::Mitosis => (0.0367, 0.0649),
            ModelPreset::Ripples => (0.018, 0.051),
            ModelPreset::SolitonCollapse => (0.022, 0.06),
            ModelPreset::USkateWorld => (0.062, 0.061),
            ModelPreset::Undulating => (0.026, 0.051),
            ModelPreset::Worms => (0.078, 0.061),
            ModelPreset::Custom => (0.035, 0.058),
        }
    }

    pub fn all() -> Vec<ModelPreset> {
        vec![
            ModelPreset::BrainCoral,
            ModelPreset::Fingerprint,
            ModelPreset::Mitosis,
            ModelPreset::Ripples,
            ModelPreset::SolitonCollapse,
            ModelPreset::USkateWorld,
            ModelPreset::Undulating,
            ModelPreset::Worms,
            ModelPreset::Custom,
        ]
    }
}

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

/// Get the user's Documents folder path and create the gray-scott presets subdirectory path
fn get_user_presets_dir() -> PathBuf {
    let home_dir = home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("gray-scott").join("presets")
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

    // Add default presets
    let all_presets = ModelPreset::all();

    for preset_type in all_presets {
        let (feed_rate, kill_rate) = preset_type.get_rates();
        let settings = Settings {
            feed_rate,
            kill_rate,
            diffusion_rate_u: 0.2097,
            diffusion_rate_v: 0.105,
            timestep: 1.0,
            nutrient_pattern: super::settings::NutrientPattern::Uniform,
            nutrient_pattern_reversed: false,
            cursor_size: 10.0,
            cursor_strength: 0.5,
        };

        let preset_name = preset_type.name().to_string();
        preset_manager.add_preset(Preset::new(preset_name, settings));
    }

    // Capture all the built-in preset names we just added
    preset_manager.capture_built_in_presets();

    // Load user presets from TOML files
    if let Err(e) = preset_manager.load_user_presets() {
        eprintln!("Warning: Could not load user presets: {}", e);
    }

    preset_manager
}
