//! # Camera State Module
//!
//! Manages shared camera state and bind group creation across all simulations.
//! This eliminates duplication of camera initialization and bind group creation.

use crate::error::SimulationResult;
use crate::simulations::shared::camera::Camera;

use std::sync::Arc;
use wgpu::{Device, Queue};

/// Camera state shared across all simulations
#[derive(Debug)]
pub struct CameraState {
    /// The camera instance
    pub camera: Camera,
    /// Camera bind group for GPU rendering
    pub bind_group: wgpu::BindGroup,
    /// Camera bind group layout
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraState {
    /// Create new camera state with bind group
    pub fn new(device: &Arc<Device>, width: f32, height: f32) -> SimulationResult<Self> {
        // Create camera
        let camera = Camera::new(device, width, height)?;

        // Create camera bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create camera bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.buffer().as_entire_binding(),
            }],
        });

        Ok(Self {
            camera,
            bind_group,
            bind_group_layout,
        })
    }

    /// Update camera state (call this every frame for smooth movement)
    pub fn update(&mut self, delta_time: f32) -> bool {
        self.camera.update(delta_time)
    }

    /// Upload camera data to GPU
    pub fn upload_to_gpu(&self, queue: &Arc<Queue>) {
        self.camera.upload_to_gpu(queue);
    }

    /// Resize camera viewport
    pub fn resize(&mut self, width: f32, height: f32) {
        self.camera.resize(width, height);
    }

    /// Pan camera
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
    }

    /// Zoom camera
    pub fn zoom(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    /// Zoom camera to cursor position
    pub fn zoom_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    /// Reset camera to default position and zoom
    pub fn reset(&mut self) {
        self.camera.reset();
    }

    /// Get camera state for serialization
    pub fn get_state(&self) -> serde_json::Value {
        self.camera.get_state()
    }

    /// Set camera smoothing factor
    pub fn set_smoothing_factor(&mut self, factor: f32) {
        self.camera.set_smoothing_factor(factor);
    }

    /// Get camera smoothing factor
    pub fn get_smoothing_factor(&self) -> f32 {
        self.camera.get_smoothing_factor()
    }

    /// Set camera sensitivity
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.camera.set_sensitivity(sensitivity);
    }

    /// Get camera sensitivity
    pub fn get_sensitivity(&self) -> f32 {
        self.camera.get_sensitivity()
    }

    /// Get camera buffer reference
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.camera.buffer()
    }

    /// Get bind group reference
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get bind group layout reference
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}
