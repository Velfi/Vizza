use crate::settings::{GradientType, Settings};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SimSizeUniform {
    pub width: u32,
    pub height: u32,
    pub decay_rate: f32,
    pub agent_jitter: f32,
    pub agent_speed_min: f32,
    pub agent_speed_max: f32,
    pub agent_turn_rate: f32,
    pub agent_sensor_angle: f32,
    pub agent_sensor_distance: f32,
    pub diffusion_rate: f32,
    pub pheromone_deposition_rate: f32,
    pub gradient_enabled: u32,
    pub gradient_type: u32,
    pub gradient_strength: f32,
    pub gradient_center_x: f32,
    pub gradient_center_y: f32,
    pub gradient_size: f32,
    pub gradient_angle: f32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl SimSizeUniform {
    pub fn new(width: u32, height: u32, decay_rate: f32, settings: &Settings) -> Self {
        Self {
            width,
            height,
            decay_rate,
            agent_jitter: settings.agent_jitter,
            agent_speed_min: settings.agent_speed_min,
            agent_speed_max: settings.agent_speed_max,
            agent_turn_rate: settings.agent_turn_rate,
            agent_sensor_angle: settings.agent_sensor_angle,
            agent_sensor_distance: settings.agent_sensor_distance,
            diffusion_rate: settings.pheromone_diffusion_rate,
            pheromone_deposition_rate: settings.pheromone_deposition_rate,
            gradient_enabled: if settings.gradient_type != GradientType::Disabled {
                1
            } else {
                0
            },
            gradient_type: match settings.gradient_type {
                GradientType::Disabled => 0,
                GradientType::Linear => 1,
                GradientType::Radial => 2,
                GradientType::Ellipse => 3,
                GradientType::Spiral => 4,
                GradientType::Checkerboard => 5,
            },
            gradient_strength: settings.gradient_strength,
            gradient_center_x: settings.gradient_center_x,
            gradient_center_y: settings.gradient_center_y,
            gradient_size: settings.gradient_size,
            gradient_angle: settings.gradient_angle,
            _pad1: 0,
            _pad2: 0,
        }
    }
}
