pub mod settings;
pub mod shaders;
pub mod simulation;
pub mod state;

#[cfg(test)]
mod tests;

pub use settings::Settings;
pub use simulation::WanderersModel;

pub fn init_presets(
    preset_manager: &mut crate::simulation::preset_manager::WanderersPresetManager,
) {
    // Initialize default presets for Wanderers simulation
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "Default".to_string(),
        Settings::default(),
    ));
}
