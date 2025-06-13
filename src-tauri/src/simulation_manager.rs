use std::sync::Arc;
use tauri::State;
use wgpu::{Device, Queue, SurfaceConfiguration};
use winit::event_loop::ActiveEventLoop;

use crate::simulations::slime_mold::{
    SlimeMoldSimulation,
    Settings,
    LutManager,
};

#[derive(Debug, Clone)]
pub enum SimulationType {
    SlimeMold,
    ParticleLife,
    ReactionDiffusion,
}

pub struct SimulationManager {
    pub slime_mold_state: Option<SlimeMoldSimulation>,
}

impl SimulationManager {
    pub fn new() -> Self {
        Self {
            slime_mold_state: None,
        }
    }

    pub async fn start_simulation(
        &mut self,
        simulation_type: String,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match simulation_type.as_str() {
            "slime_mold" => {
                // Initialize slime mold simulation
                let settings = Settings::default();
                let lut_manager = LutManager::new();
                let available_luts = lut_manager.get_available_luts();
                
                let simulation = SlimeMoldSimulation::new(
                    device,
                    queue,
                    surface_config,
                    adapter_info,
                    100000, // agent_count
                    settings,
                    &lut_manager,
                    &available_luts,
                    0, // current_lut_index
                    false, // lut_reversed
                )?;
                
                self.slime_mold_state = Some(simulation);
                Ok(())
            }
            _ => Err("Unknown simulation type".into()),
        }
    }

    pub fn stop_simulation(&mut self) {
        self.slime_mold_state = None;
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.render_frame(device, queue, surface_view)?;
        }
        Ok(())
    }

    pub fn handle_resize(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
        adapter_info: &wgpu::AdapterInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(simulation) = &mut self.slime_mold_state {
            simulation.resize(device, queue, new_config, adapter_info)?;
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.slime_mold_state.is_some()
    }

    pub fn get_status(&self) -> String {
        if self.slime_mold_state.is_some() {
            "Slime Mold Simulation Running".to_string()
        } else {
            "No Simulation Running".to_string()
        }
    }
}