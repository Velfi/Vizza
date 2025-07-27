use super::coordinates::{CoordinateTransform, NdcCoords, ScreenCoords, WorldCoords};
use crate::error::SimulationResult;
use bytemuck::{Pod, Zeroable};
use serde_json;
use std::sync::Arc;
use wgpu::{Device, Queue};

/// GPU-compatible camera uniform data
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct CameraUniform {
    /// Simple 2D transformation matrix (4x4 matrix stored as 16 floats)
    pub transform_matrix: [f32; 16],
    /// Camera position in world space
    pub position: [f32; 2],
    /// Zoom level (scale factor)
    pub zoom: f32,
    /// Aspect ratio (width/height)
    pub aspect_ratio: f32,
}

impl CoordinateTransform for Camera {
    fn screen_to_world(&self, screen: ScreenCoords) -> WorldCoords {
        self.screen_to_world(screen)
    }

    fn world_to_screen(&self, world: WorldCoords) -> ScreenCoords {
        self.world_to_screen(world)
    }

    fn screen_to_ndc(&self, screen: ScreenCoords) -> NdcCoords {
        self.screen_to_ndc(screen)
    }

    fn ndc_to_world(&self, ndc: NdcCoords) -> WorldCoords {
        self.ndc_to_world(ndc)
    }

    fn world_to_ndc(&self, world: WorldCoords) -> NdcCoords {
        self.world_to_ndc(world)
    }
}

/// Simple 2D camera system for GPU rendering
#[derive(Debug)]
pub struct Camera {
    /// Camera position in world space
    pub position: [f32; 2],
    /// Target camera position for smooth movement
    target_position: [f32; 2],
    /// Zoom level (scale factor)
    pub zoom: f32,
    /// Target zoom level for smooth zooming
    target_zoom: f32,
    /// Viewport dimensions
    pub viewport_width: f32,
    pub viewport_height: f32,
    /// GPU buffer for camera uniform data
    buffer: wgpu::Buffer,
    /// Cached uniform data
    uniform_data: CameraUniform,
    /// Smoothing factor for camera movement (0.0 = no smoothing, 1.0 = instant)
    smoothing_factor: f32,
    /// Camera sensitivity multiplier for pan and zoom operations
    sensitivity: f32,
}

impl Camera {
    /// Create a new camera with default settings
    pub fn new(
        device: &Arc<Device>,
        viewport_width: f32,
        viewport_height: f32,
    ) -> SimulationResult<Self> {
        let position = [0.0, 0.0]; // Center of [-1,1] space
        let zoom = 1.0; // No zoom
        let aspect_ratio = viewport_width / viewport_height;

        let uniform_data = CameraUniform {
            transform_matrix: Self::create_simple_transform_matrix(position, zoom),
            position,
            zoom,
            aspect_ratio,
        };

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            position,
            target_position: position,
            zoom,
            target_zoom: zoom,
            viewport_width,
            viewport_height,
            buffer,
            uniform_data,
            smoothing_factor: 0.15, // Smooth camera movement
            sensitivity: 1.0,       // Default sensitivity
        })
    }

    /// Create a simple 2D transformation matrix
    fn create_simple_transform_matrix(position: [f32; 2], zoom: f32) -> [f32; 16] {
        // Create a simple orthographic projection matrix
        // This maps [-1,1] x [-1,1] world space to [-1,1] x [-1,1] clip space
        let scale_x = zoom;
        let scale_y = zoom;

        // For proper center zooming, we want to:
        // 1. Scale around the origin (0,0)
        // 2. Then translate to account for camera position
        // The translation should move the camera center to NDC origin (0,0)
        let translate_x = -position[0] * zoom;
        let translate_y = -position[1] * zoom;

        [
            scale_x,
            0.0,
            0.0,
            0.0,
            0.0,
            scale_y,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            translate_x,
            translate_y,
            0.0,
            1.0,
        ]
    }

    /// Update camera state (call this every frame for smooth movement)
    pub fn update(&mut self, delta_time: f32) -> bool {
        // Apply smoothing to position
        let smoothing = self.smoothing_factor * delta_time * 60.0; // Adjust for frame rate
        let smoothing = smoothing.min(1.0); // Clamp to prevent overshooting

        self.position[0] += (self.target_position[0] - self.position[0]) * smoothing;
        self.position[1] += (self.target_position[1] - self.position[1]) * smoothing;

        // Apply smoothing to zoom
        self.zoom += (self.target_zoom - self.zoom) * smoothing;

        // Update uniform data after smoothing
        self.update_uniform();
        true
    }

    /// Update camera position (panning)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let pan_speed = 0.1 / self.zoom; // Pan speed depends on zoom level

        // Apply sensitivity to the pan movement
        let adjusted_delta_x = delta_x * self.sensitivity;
        let adjusted_delta_y = delta_y * self.sensitivity;

        // Update target position instead of current position
        self.target_position[0] += adjusted_delta_x * pan_speed;
        self.target_position[1] += adjusted_delta_y * pan_speed;

        // Clamp target position to reasonable bounds for [-1,1] world space
        // Allow movement within [-2.0, 2.0] to provide some margin around the [-1,1] space
        self.target_position[0] = self.target_position[0].clamp(-2.0, 2.0);
        self.target_position[1] = self.target_position[1].clamp(-2.0, 2.0);
    }

    /// Update zoom level (zooms to center of viewport)
    pub fn zoom(&mut self, delta: f32) {
        // Apply sensitivity to the zoom movement
        let adjusted_delta = delta * self.sensitivity;
        let zoom_factor = 1.0 + adjusted_delta * 0.3;
        let new_zoom = self.target_zoom * zoom_factor;
        let clamped_zoom = new_zoom.clamp(0.005, 50.0); // Set reasonable minimum zoom level

        // Use relative threshold to handle extreme zoom levels properly
        let relative_threshold = self.target_zoom * 0.001; // 0.1% change threshold
        let absolute_threshold = 0.000001; // Much smaller minimum absolute change for extreme zoom levels
        let threshold = relative_threshold.max(absolute_threshold);

        if (clamped_zoom - self.target_zoom).abs() > threshold {
            // Store the center point in world coordinates before zoom
            let center_world_x = self.target_position[0];
            let center_world_y = self.target_position[1];

            self.target_zoom = clamped_zoom;

            // Keep the same center point after zoom
            self.target_position[0] = center_world_x;
            self.target_position[1] = center_world_y;
        }
    }

    /// Zoom towards a specific screen position (for mouse wheel)
    pub fn zoom_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        let old_zoom = self.target_zoom;
        self.zoom(delta);

        // Convert cursor position to NDC coordinates
        let mouse_x_norm = (cursor_x / self.viewport_width) * 2.0 - 1.0;
        let mouse_y_norm = -((cursor_y / self.viewport_height) * 2.0 - 1.0);

        // Calculate the world point under the cursor before zoom (use target position for consistency)
        let world_x = mouse_x_norm / old_zoom + self.target_position[0];
        let world_y = mouse_y_norm / old_zoom + self.target_position[1];

        // Calculate where this world point should be in NDC after zoom
        let new_ndc_x = (world_x - self.target_position[0]) * self.target_zoom;
        let new_ndc_y = (world_y - self.target_position[1]) * self.target_zoom;

        // Calculate the offset needed to keep the cursor point stationary
        let offset_x = (mouse_x_norm - new_ndc_x) / self.target_zoom;
        let offset_y = (mouse_y_norm - new_ndc_y) / self.target_zoom;

        // Apply the offset to keep the cursor point stationary
        self.target_position[0] += offset_x;
        self.target_position[1] += offset_y;

        // Clamp target position to reasonable bounds to prevent going too far out
        self.target_position[0] = self.target_position[0].clamp(-2.0, 2.0);
        self.target_position[1] = self.target_position[1].clamp(-2.0, 2.0);
    }

    /// Reset camera to default position and zoom
    pub fn reset(&mut self) {
        self.position = [0.0, 0.0];
        self.target_position = [0.0, 0.0];
        self.zoom = 1.0;
        self.target_zoom = 1.0;
        self.update_uniform();
    }

    /// Update viewport dimensions (call when window is resized)
    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.update_uniform();
    }

    /// Update the uniform data after camera changes
    fn update_uniform(&mut self) {
        let aspect_ratio = self.viewport_width / self.viewport_height;
        self.uniform_data = CameraUniform {
            transform_matrix: Self::create_simple_transform_matrix(self.position, self.zoom),
            position: self.position,
            zoom: self.zoom,
            aspect_ratio,
        };
    }

    /// Upload camera data to GPU buffer
    pub fn upload_to_gpu(&self, queue: &Arc<Queue>) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform_data]));
    }

    /// Get the GPU buffer for binding to render passes
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen: ScreenCoords) -> WorldCoords {
        let ndc_x = (screen.x / self.viewport_width) * 2.0 - 1.0;
        let ndc_y = -((screen.y / self.viewport_height) * 2.0 - 1.0); // Flip Y axis correctly

        // Use target position and zoom for immediate response (no smoothing lag)
        let world_x = (ndc_x / self.target_zoom) + self.target_position[0];
        let world_y = (ndc_y / self.target_zoom) + self.target_position[1];

        WorldCoords::new(world_x, world_y)
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world: WorldCoords) -> ScreenCoords {
        let ndc_x = (world.x - self.target_position[0]) * self.target_zoom;
        let ndc_y = (world.y - self.target_position[1]) * self.target_zoom;

        let screen_x = (ndc_x + 1.0) * self.viewport_width * 0.5;
        let screen_y = (-ndc_y + 1.0) * self.viewport_height * 0.5; // Flip Y axis correctly

        ScreenCoords::new(screen_x, screen_y)
    }

    /// Convert screen coordinates to NDC
    pub fn screen_to_ndc(&self, screen: ScreenCoords) -> NdcCoords {
        let ndc_x = (screen.x / self.viewport_width) * 2.0 - 1.0;
        let ndc_y = -((screen.y / self.viewport_height) * 2.0 - 1.0); // Flip Y axis correctly
        NdcCoords::new(ndc_x, ndc_y)
    }

    /// Convert NDC to screen coordinates
    pub fn ndc_to_screen(&self, ndc: NdcCoords) -> ScreenCoords {
        let screen_x = (ndc.x + 1.0) * self.viewport_width * 0.5;
        let screen_y = (-ndc.y + 1.0) * self.viewport_height * 0.5; // Flip Y axis correctly
        ScreenCoords::new(screen_x, screen_y)
    }

    /// Convert NDC to world coordinates
    pub fn ndc_to_world(&self, ndc: NdcCoords) -> WorldCoords {
        let world_x = (ndc.x / self.target_zoom) + self.target_position[0];
        let world_y = (ndc.y / self.target_zoom) + self.target_position[1];
        WorldCoords::new(world_x, world_y)
    }

    /// Convert world coordinates to NDC
    pub fn world_to_ndc(&self, world: WorldCoords) -> NdcCoords {
        let ndc_x = (world.x - self.target_position[0]) * self.target_zoom;
        let ndc_y = (world.y - self.target_position[1]) * self.target_zoom;
        NdcCoords::new(ndc_x, ndc_y)
    }

    /// Get camera state for frontend serialization
    pub fn get_state(&self) -> serde_json::Value {
        serde_json::json!({
            "position": [self.position[0], self.position[1], 0.0],
            "zoom": self.zoom,
            "viewport_width": self.viewport_width,
            "viewport_height": self.viewport_height,
            "aspect_ratio": self.viewport_width / self.viewport_height
        })
    }

    /// Set the camera smoothing factor
    pub fn set_smoothing_factor(&mut self, factor: f32) {
        self.smoothing_factor = factor.clamp(0.0, 1.0);
    }

    /// Get the current camera smoothing factor
    pub fn get_smoothing_factor(&self) -> f32 {
        self.smoothing_factor
    }

    /// Get the target camera position
    pub fn get_target_position(&self) -> [f32; 2] {
        self.target_position
    }

    /// Get the target zoom level
    pub fn get_target_zoom(&self) -> f32 {
        self.target_zoom
    }

    /// Set the camera sensitivity
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.clamp(0.1, 5.0);
    }

    /// Get the current camera sensitivity
    pub fn get_sensitivity(&self) -> f32 {
        self.sensitivity
    }
}
