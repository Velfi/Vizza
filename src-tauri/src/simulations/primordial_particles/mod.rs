pub mod settings;
pub mod shaders;
pub mod simulation;
pub mod state;

pub use simulation::PrimordialParticlesModel;

use crate::simulation::preset_manager::{Preset, PrimordialParticlesPresetManager};

/// Initialize Primordial Particles presets with built-in configurations
/// Based on "How a life-like system emerges from a simplistic particle motion law" (Schmickl et al., 2016)
pub fn init_presets(preset_manager: &mut PrimordialParticlesPresetManager) {
    use settings::Settings;

    // Research-backed presets from the Nature paper
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
    tracing::info!("Initialized {} Primordial Particles presets", preset_count);
}
