use wgpu::{Device, Queue};

use crate::simulations::shared::LutData;

/// Trait for handling LUT operations in simulations
pub trait LutHandler {
    /// Get the name of the currently active LUT
    fn get_name_of_active_lut(&self) -> &str;

    /// Check if the LUT is currently reversed
    fn is_lut_reversed(&self) -> bool;

    /// Set whether the LUT should be reversed
    fn set_lut_reversed(&mut self, reversed: bool);

    /// Set the active LUT with new data and name
    fn set_active_lut(&mut self, lut_data: &LutData, name: String, device: &Device, queue: &Queue);
} 