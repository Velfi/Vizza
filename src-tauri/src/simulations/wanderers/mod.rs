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

    let pool_balls = Settings {
        gravitational_constant: 0.0, // No gravity for pool ball behavior
        collision_damping: 0.0,      // No damping
        energy_damping: 1.0,         // No energy loss
        initial_velocity_max: 0.4,   // Higher initial velocities
        initial_velocity_min: 0.2,   // Higher minimum velocities
        particle_count: 500,         // Moderate count for pool ball demo
        ..Default::default()
    };
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "Pool Balls".to_string(),
        pool_balls,
    ));

    let clumping = Settings {
        gravitational_constant: 0.008, // Moderate gravity for clumping
        collision_damping: 0.1,        // Light damping
        energy_damping: 0.999,         // Energy loss for settling
        initial_velocity_max: 0.2,     // Lower velocities for stable clumping
        initial_velocity_min: 0.05,    // Lower minimum velocities
        particle_count: 2000,          // More particles for clumping
        ..Default::default()
    };
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "Clumping".to_string(),
        clumping,
    ));

    let high_performance = Settings {
        particle_count: 10000,         // Maximum particle count for GPU testing
        particle_size: 0.008,          // Smaller particles for density
        gravitational_constant: 0.005, // Moderate gravity
        collision_damping: 0.05,       // Very light damping
        energy_damping: 0.9995,        // Minimal energy loss
        initial_velocity_max: 0.15,    // Low velocities for stability
        initial_velocity_min: 0.05,    // Low minimum velocities
        ..Default::default()
    };
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "High Performance".to_string(),
        high_performance,
    ));

    let chaotic = Settings {
        gravitational_constant: 0.012, // Strong gravity
        initial_velocity_max: 0.6,     // High initial velocities
        energy_damping: 0.99,          // Less damping
        particle_count: 3000,          // More particles for chaos
        ..Default::default()
    };
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "Chaotic".to_string(),
        chaotic,
    ));

    let stable = Settings {
        gravitational_constant: 0.003, // Weak gravity
        initial_velocity_max: 0.15,    // Very low velocities
        energy_damping: 0.999,         // Strong damping
        particle_count: 1500,          // Moderate particle count
        ..Default::default()
    };
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "Stable".to_string(),
        stable,
    ));
}
