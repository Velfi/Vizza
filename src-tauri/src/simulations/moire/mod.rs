pub mod settings;
pub mod shaders;
pub mod simulation;

pub use simulation::MoireModel;

use crate::simulation::preset_manager::{MoirePresetManager, Preset};

/// Initialize Moiré presets with built-in configurations
pub fn init_presets(preset_manager: &mut MoirePresetManager) {
    use settings::Settings;

    // Add built-in presets
    preset_manager.add_preset(Preset::new("Default".to_string(), Settings::default()));

    preset_manager.add_preset(Preset::new(
        "Classic Moiré".to_string(),
        Settings {
            base_freq: 30.0,
            moire_amount: 0.8,
            moire_rotation: 0.1,
            moire_scale: 1.02,
            moire_interference: 0.7,
            advect_strength: 0.1,
            color_scheme_name: "viridis".to_string(),
            color_scheme_reversed: false,
            ..Settings::default()
        },
    ));

    preset_manager.add_preset(Preset::new(
        "Psychedelic".to_string(),
        Settings {
            base_freq: 20.0,
            moire_amount: 0.5,
            moire_rotation: 0.3,
            moire_scale: 1.1,
            moire_interference: 0.5,
            advect_strength: 0.4,
            color_scheme_name: "plasma".to_string(),
            color_scheme_reversed: false,
            ..Settings::default()
        },
    ));

    preset_manager.add_preset(Preset::new(
        "Subtle".to_string(),
        Settings {
            base_freq: 40.0,
            moire_amount: 0.3,
            moire_rotation: 0.05,
            moire_scale: 1.01,
            moire_interference: 0.3,
            advect_strength: 0.2,
            color_scheme_name: "bone".to_string(),
            color_scheme_reversed: true,
            ..Settings::default()
        },
    ));
}
