pub mod renderer;
pub mod settings;
pub mod shaders;
pub mod simulation;

pub use simulation::GrayScottModel;

use crate::simulation::preset_manager::{GrayScottPresetManager, Preset};

/// Initialize Gray-Scott presets with built-in configurations
pub fn init_presets(preset_manager: &mut GrayScottPresetManager) {
    use settings::{NutrientPattern, Settings};

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
            cursor_size: 10.0,
            cursor_strength: 0.5,
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
