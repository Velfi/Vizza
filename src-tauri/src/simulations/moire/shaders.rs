//! # Moiré Shaders Module
//!
//! Contains the WGSL shader code for the Moiré simulation.
//! This module provides the compute shader for moiré pattern generation
//! and the render shader for final display.

pub const COMPUTE_SHADER: &str = include_str!("compute.wgsl");
pub const RENDER_INFINITE_SHADER: &str = crate::simulations::shared::INFINITE_RENDER_SHADER;
