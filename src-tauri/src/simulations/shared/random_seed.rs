//! # Random Seed State Module
//!
//! Manages shared random seed state across all simulations.
//! This standardizes random seed handling and provides consistent
//! random seed management behavior.

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

/// Random seed state shared across all simulations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RandomSeedState {
    /// Current random seed value
    pub seed: u32,
}

impl RandomSeedState {
    /// Create new random seed state with specified seed
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Get current seed value
    pub fn get_seed(&self) -> u32 {
        self.seed
    }

    /// Set seed value
    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }

    /// Generate a new random seed
    pub fn randomize(&mut self) {
        self.seed = rand::random::<u32>();
    }

    /// Create a seeded RNG from current seed
    pub fn create_rng(&self) -> StdRng {
        StdRng::seed_from_u64(self.seed as u64)
    }

    /// Create a seeded RNG from a specific seed
    pub fn create_rng_from_seed(seed: u32) -> StdRng {
        StdRng::seed_from_u64(seed as u64)
    }

    /// Generate a new seed and create RNG
    pub fn create_random_rng() -> (StdRng, u32) {
        let seed = rand::random::<u32>();
        let rng = StdRng::seed_from_u64(seed as u64);
        (rng, seed)
    }
}
