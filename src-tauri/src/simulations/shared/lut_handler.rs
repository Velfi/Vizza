use wgpu::{Device, Queue};

use crate::simulations::shared::LutData;

/// Trait for handling LUT operations in simulations
pub trait LutHandler {
    /// Get the name of the current LUT
    fn get_lut_name(&self) -> &str;

    /// Check if the LUT is currently reversed
    fn is_lut_reversed(&self) -> bool;

    /// Set whether the LUT should be reversed
    fn set_lut_reversed(&mut self, reversed: bool);

    /// Update the LUT with new data
    fn update_lut(&mut self, lut_data: &LutData, device: &Device, queue: &Queue);
} 