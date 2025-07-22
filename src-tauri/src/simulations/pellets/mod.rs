//! # Pellets Simulation Module
//! 
//! A GPU-accelerated particle physics simulation that creates dynamic environments
//! where particles interact through gravity, collisions, and user-controlled forces.
//! The simulation explores emergent behaviors through simple physical rules.
//! 
//! ## Concept
//! 
//! Pellets simulates a universe of particles that respond to gravitational forces,
//! creating complex orbital patterns, clumping behaviors, and chaotic motion. Users
//! can interact with the system through mouse controls, adding attraction or repulsion
//! forces to influence particle behavior in real-time.
//! 
//! ## Key Features
//! 
//! - **Emergent Complexity**: Simple gravitational rules create complex behaviors
//! - **Real-time Interaction**: Direct manipulation of particle forces
//! - **Visual Exploration**: Multiple coloring modes reveal different aspects of the system
//! - **Performance Optimized**: GPU acceleration enables thousands of particles
//! 
//! ## Architecture
//! 
//! The simulation uses a hybrid approach where the CPU manages configuration and
//! user interaction while the GPU handles all physics calculations and rendering.
//! This separation allows for both responsive user controls and high-performance
//! computation of particle interactions.

pub mod settings;
pub mod shaders;
pub mod simulation;
pub mod state;

#[cfg(test)]
mod tests;

pub use settings::Settings;
pub use simulation::PelletsModel;

/// Initialize default presets for the Pellets simulation.
/// 
/// Creates a set of predefined configurations that users can quickly
/// load to explore different simulation behaviors.
pub fn init_presets(
    preset_manager: &mut crate::simulation::preset_manager::PelletsPresetManager,
) {
    // Initialize default presets for Pellets simulation
    preset_manager.add_preset(crate::simulation::preset_manager::Preset::new(
        "Default".to_string(),
        Settings::default(),
    ));
}
