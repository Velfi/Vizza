pub mod buffer_pool;
pub mod commands;
pub mod render;
pub mod settings;
pub mod simulation;
pub mod workgroup_optimizer;

pub use simulation::SlimeMoldModel;

use crate::simulation::preset_manager::{Preset, SlimeMoldPresetManager};

/// Initialize slime mold presets with built-in configurations
pub fn init_presets(preset_manager: &mut SlimeMoldPresetManager) {
    use settings::{GradientType, Settings};

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
    preset_manager.add_preset(Preset::new(
        "Venom".to_string(),
        Settings {
            agent_jitter: 2.0,
            agent_sensor_angle: 0.3,
            agent_sensor_distance: 20.0,
            agent_speed_max: 500.0,
            agent_speed_min: 0.0,
            agent_turn_rate: 0.20943951606750488,
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
