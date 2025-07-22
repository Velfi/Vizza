//! # Timing Module
//!
//! Manages shared frame timing state across simulations.
//! This eliminates duplication of timing fields and provides consistent
//! frame timing behavior.

use std::time::Instant;

/// Frame timing state shared across simulations
#[derive(Debug, Clone)]
pub struct TimingState {
    /// Time of the last frame
    pub last_frame_time: Instant,
}

impl Default for TimingState {
    fn default() -> Self {
        Self {
            last_frame_time: Instant::now(),
        }
    }
}

impl TimingState {
    /// Create new timing state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get time since last frame in seconds
    pub fn delta_time(&mut self) -> f32 {
        let current_time = Instant::now();
        let delta = current_time
            .duration_since(self.last_frame_time)
            .as_secs_f32();
        self.last_frame_time = current_time;
        delta
    }

    /// Update frame time without returning delta
    pub fn update_frame_time(&mut self) {
        self.last_frame_time = Instant::now();
    }

    /// Reset timing state
    pub fn reset(&mut self) {
        self.last_frame_time = Instant::now();
    }
}
