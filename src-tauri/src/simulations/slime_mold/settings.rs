use crate::simulations::shared::ImageFitMode;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
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
    /// The fit mode for image-based position generation.
    ///
    /// Defaults to ImageFitMode::Stretch.
    pub position_image_fit_mode: ImageFitMode,
    /// The frequency of diffusion updates.
    ///
    /// Defaults to 1.
    pub diffusion_frequency: u32,
    /// The frequency of decay updates.
    ///
    /// Defaults to 1.
    pub decay_frequency: u32,
    /// Random seed for reproducible randomization.
    ///
    /// Defaults to 0.
    pub random_seed: u32,
    /// Background mode for the simulation.
    ///
    /// Defaults to BackgroundMode::Black.
    pub background_mode: BackgroundMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BackgroundMode {
    Black,
    White,
}

impl BackgroundMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackgroundMode::Black => "black",
            BackgroundMode::White => "white",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "black" => Some(BackgroundMode::Black),
            "white" => Some(BackgroundMode::White),
            _ => None,
        }
    }
}

impl From<BackgroundMode> for u32 {
    fn from(mode: BackgroundMode) -> Self {
        match mode {
            BackgroundMode::Black => 0,
            BackgroundMode::White => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrailMapFiltering {
    Nearest,
    Linear,
}

impl TrailMapFiltering {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrailMapFiltering::Nearest => "Nearest",
            TrailMapFiltering::Linear => "Linear",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Nearest" => Some(TrailMapFiltering::Nearest),
            "Linear" => Some(TrailMapFiltering::Linear),
            _ => None,
        }
    }
}

impl Display for TrailMapFiltering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Nearest => "Nearest",
                Self::Linear => "Linear",
            }
        )
    }
}

impl Default for TrailMapFiltering {
    fn default() -> Self {
        Self::Nearest
    }
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
            position_image_fit_mode: ImageFitMode::FitV,
            diffusion_frequency: 1,
            decay_frequency: 1,
            random_seed: 0,
            background_mode: BackgroundMode::Black,
        }
    }
}

impl Settings {
    /// Randomize all settings within reasonable bounds
    pub fn randomize(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();

        self.agent_speed_min = rand::random::<f32>() * 500.0;
        self.agent_speed_max =
            self.agent_speed_min + rand::random::<f32>() * (500.0 - self.agent_speed_min);
        self.agent_turn_rate = (rand::random::<f32>() * 360.0) * std::f32::consts::PI / 180.0; // Convert degrees to radians
        self.agent_jitter = rand::random::<f32>();
        self.agent_sensor_angle = (rand::random::<f32>() * 180.0) * std::f32::consts::PI / 180.0; // Convert degrees to radians
        self.agent_sensor_distance = rand::random::<f32>() * 500.0;
        self.pheromone_decay_rate = 100.0;
        self.pheromone_deposition_rate = 100.0;
        self.pheromone_diffusion_rate = 100.0;

        // Mask settings are runtime state; do not modify here

        // Randomize starting direction range
        let start = rand::random::<f32>() * 360.0;
        let end = start + rand::random::<f32>() * (360.0 - start);
        self.agent_possible_starting_headings = start..end;

        self.diffusion_frequency = 1;
        self.decay_frequency = 1;
        self.random_seed = rng.random();
    }
}
