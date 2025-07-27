pub mod matrix_operations;
pub mod settings;
pub mod shaders;
pub mod simulation;

#[cfg(test)]
mod tests;

pub use simulation::ParticleLifeModel;

use crate::simulation::preset_manager::{ParticleLifePresetManager, Preset};

/// Initialize Particle Life presets with built-in configurations
pub fn init_presets(preset_manager: &mut ParticleLifePresetManager) {
    use settings::Settings;

    // Add default presets
    let all_presets = vec![("Default", Settings::default())];

    for (preset_name, settings) in all_presets {
        preset_manager.add_preset(Preset::new(preset_name.to_string(), settings));
    }

    // Capture all the built-in preset names we just added
    preset_manager.capture_built_in_presets();

    // Load user presets from TOML files
    if let Err(e) = preset_manager.load_user_presets() {
        eprintln!("Warning: Could not load user presets: {}", e);
    }

    let preset_count = preset_manager.get_preset_names().len();
    tracing::info!("Initialized {} Particle Life presets", preset_count);
}
