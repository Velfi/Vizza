use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// The amount of jitter to add to the agent's starting heading.
    ///
    /// Defaults to 0.04.
    pub agent_jitter: f32,
    /// The range of possible starting headings for the agent.
    ///
    /// Defaults to 0.0..360.0.
    #[serde(
        serialize_with = "serialize_range",
        deserialize_with = "deserialize_range"
    )]
    pub agent_possible_starting_headings: Range<f32>,
    /// The angle of the agent's sensor.
    ///
    /// Defaults to 0.3 radians.
    pub agent_sensor_angle: f32,
    /// The distance of the agent's sensor.
    ///
    /// Defaults to 20.0.
    pub agent_sensor_distance: f32,
    /// The maximum speed of the agent.
    ///
    /// Defaults to 60.0.
    pub agent_speed_max: f32,
    /// The minimum speed of the agent.
    ///
    /// Defaults to 30.0.
    pub agent_speed_min: f32,
    /// The rate at which agents can turn.
    ///
    /// Defaults to 0.43 rad/s.
    pub agent_turn_rate: f32,
    /// The decay rate of the pheromone.
    ///
    /// Defaults to 1.0
    pub pheromone_decay_rate: f32,
    /// The rate at which agents deposit pheromones.
    ///
    /// Defaults to 1.0
    pub pheromone_deposition_rate: f32,
    /// The rate at which pheromone diffuses.
    ///
    /// Defaults to 1.0
    pub pheromone_diffusion_rate: f32,
    /// The type of gradient.
    ///
    /// Defaults to GradientType::Disabled.
    pub gradient_type: GradientType,
    /// The strength of the gradient.
    ///
    /// Defaults to 0.5.
    pub gradient_strength: f32,
    /// The x-coordinate of the center of the gradient.
    ///
    /// Defaults to 0.5.
    pub gradient_center_x: f32,
    /// The y-coordinate of the center of the gradient.
    ///
    /// Defaults to 0.5.
    pub gradient_center_y: f32,
    /// The size of the gradient.
    ///
    /// Defaults to 0.3.
    pub gradient_size: f32,
    /// The angle of the gradient.
    ///
    /// Defaults to 0.0.
    pub gradient_angle: f32,
    /// How often to update the diffusion (every N frames).
    ///
    /// Defaults to 1 (every frame). Higher values improve performance.
    pub diffusion_frequency: u32,
    /// How often to update the decay (every N frames).
    ///
    /// Defaults to 1 (every frame). Higher values improve performance.
    pub decay_frequency: u32,
    /// Random seed for agent reset operations.
    ///
    /// This is updated whenever agents are reset to ensure different randomization.
    pub random_seed: u32,
}

// Custom serialization for Range<f32>
fn serialize_range<S>(range: &Range<f32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    (range.start, range.end).serialize(serializer)
}

fn deserialize_range<'de, D>(deserializer: D) -> Result<Range<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let (start, end) = <(f32, f32)>::deserialize(deserializer)?;
    Ok(start..end)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GradientType {
    Disabled,
    Linear,
    Radial,
    Ellipse,
    Spiral,
    Checkerboard,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            agent_jitter: 0.04,
            agent_possible_starting_headings: 0.0..360.0,
            agent_sensor_angle: 0.3,
            agent_sensor_distance: 20.0,
            agent_speed_max: 60.0,
            agent_speed_min: 30.0,
            agent_turn_rate: 0.43, // ~25 degrees
            pheromone_decay_rate: 10.0,
            pheromone_deposition_rate: 100.0,
            pheromone_diffusion_rate: 100.0,
            gradient_type: GradientType::Disabled,
            gradient_strength: 0.5,
            gradient_center_x: 0.5,
            gradient_center_y: 0.5,
            gradient_size: 0.3,
            gradient_angle: 0.0,
            diffusion_frequency: 1,
            decay_frequency: 1,
            random_seed: 0,
        }
    }
}
