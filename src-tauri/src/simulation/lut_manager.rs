use std::sync::Arc;
use wgpu::Queue;

use crate::simulations::shared::LutManager;
use crate::simulations::shared::lut_handler::LutHandler;
use crate::simulations::traits::SimulationType;

pub struct SimulationLutManager {
    // This will be implemented to handle LUT operations
    // Currently the LUT manager is kept in the main SimulationManager
    // This module will provide a unified interface for LUT operations
}

impl SimulationLutManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_available_luts(&self) -> Vec<String> {
        // This would access the main LUT manager
        vec![] // Placeholder
    }

    pub fn apply_lut(
        &self,
        simulation: &mut SimulationType,
        lut_manager: &LutManager,
        lut_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match simulation {
            SimulationType::SlimeMold(simulation) => {
                Self::handle_lut_application(simulation, lut_manager, lut_name, queue)
            }
            SimulationType::GrayScott(simulation) => {
                Self::handle_lut_application(simulation, lut_manager, lut_name, queue)
            }
        }
    }

    pub fn reverse_current_lut(
        &self,
        simulation: &mut SimulationType,
        lut_manager: &LutManager,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match simulation {
            SimulationType::SlimeMold(simulation) => {
                Self::handle_lut_reversal(simulation, lut_manager, queue)
            }
            SimulationType::GrayScott(simulation) => {
                Self::handle_lut_reversal(simulation, lut_manager, queue)
            }
        }
    }

    fn handle_lut_reversal<T: LutHandler>(
        simulation: &mut T,
        lut_manager: &LutManager,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_lut_name = simulation.get_name_of_active_lut();
        let is_reversed = simulation.is_lut_reversed();
        let new_reversed = !is_reversed;

        if let Ok(lut_data) = lut_manager.get(&current_lut_name) {
            simulation.set_lut_reversed(new_reversed);
            simulation.set_active_lut(&lut_data, queue);
        }

        Ok(())
    }

    fn handle_lut_application<T: LutHandler>(
        simulation: &mut T,
        lut_manager: &LutManager,
        lut_name: &str,
        queue: &Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(lut_data) = lut_manager.get(lut_name) {
            simulation.set_active_lut(&lut_data, queue);
        }

        Ok(())
    }
} 