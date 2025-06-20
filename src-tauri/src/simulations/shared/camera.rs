use super::coordinates::{CoordinateTransform, NdcCoords, ScreenCoords, WorldCoords};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use tracing;
use wgpu::{Device, Queue};
use serde_json;
use crate::error::SimulationResult;

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
    /// Zoom level (scale factor)
    pub zoom: f32,
    /// Viewport dimensions
    pub viewport_width: f32,
    pub viewport_height: f32,
    /// GPU buffer for camera uniform data
    buffer: wgpu::Buffer,
    /// Cached uniform data
    uniform_data: CameraUniform,
}

impl Camera {
    /// Create a new camera with default settings
    pub fn new(
        device: &Arc<Device>,
        viewport_width: f32,
        viewport_height: f32,
    ) -> SimulationResult<Self> {
        let position = [0.5, 0.5]; // Center of [0,1] space
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
            zoom,
            viewport_width,
            viewport_height,
            buffer,
            uniform_data,
        })
    }

    /// Create a simple 2D transformation matrix
    fn create_simple_transform_matrix(position: [f32; 2], zoom: f32) -> [f32; 16] {
        // Create a simple orthographic projection matrix
        // This maps [0,1] x [0,1] world space to [-1,1] x [-1,1] clip space
        let scale_x = zoom;
        let scale_y = zoom;
        
        // For proper center zooming, we want to:
        // 1. Scale around the origin (0,0)
        // 2. Then translate to account for camera position
        // The translation should move the camera center to NDC origin (0,0)
        let translate_x = -(position[0] - 0.5) * 2.0 * zoom;
        let translate_y = -(position[1] - 0.5) * 2.0 * zoom;

        [
            scale_x, 0.0, 0.0, 0.0,
            0.0, scale_y, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            translate_x, translate_y, 0.0, 1.0,
        ]
    }

    /// Update camera state (call this every frame for smooth movement)
    pub fn update(&mut self, _delta_time: f32) -> bool {
        // For now, just update the uniform data
        self.update_uniform();
        true
    }

    /// Update camera position (panning)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let pan_speed = 0.1 / self.zoom; // Pan speed depends on zoom level
        self.position[0] += delta_x * pan_speed;
        self.position[1] += delta_y * pan_speed;
        
        // Clamp position to reasonable bounds
        self.position[0] = self.position[0].clamp(-2.0, 2.0);
        self.position[1] = self.position[1].clamp(-2.0, 2.0);

        // Update uniform data after position change
        self.update_uniform();

        tracing::debug!(
            "Camera pan: delta=({:.2}, {:.2}), pos=({:.2}, {:.2})",
            delta_x, delta_y, self.position[0], self.position[1]
        );
    }

    /// Update zoom level (zooms to center of viewport)
    pub fn zoom(&mut self, delta: f32) {
        let zoom_factor = 1.0 + delta * 0.3;
        let new_zoom = self.zoom * zoom_factor;
        let clamped_zoom = new_zoom.clamp(0.1, 50.0);

        if (clamped_zoom - self.zoom).abs() > 0.001 {
            // Store the center point in world coordinates before zoom
            let center_world_x = self.position[0];
            let center_world_y = self.position[1];
            
            self.zoom = clamped_zoom;
            
            // Keep the same center point after zoom
            self.position[0] = center_world_x;
            self.position[1] = center_world_y;
            
            // Update uniform data after zoom change
            self.update_uniform();
            tracing::debug!("Camera zoom: delta={:.2}, new_zoom={:.2}", delta, clamped_zoom);
        }
    }

    /// Zoom towards a specific screen position (for mouse wheel)
    pub fn zoom_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        let old_zoom = self.zoom;
        self.zoom(delta);

        // Convert cursor position to NDC coordinates
        let mouse_x_norm = (cursor_x / self.viewport_width) * 2.0 - 1.0;
        let mouse_y_norm = -((cursor_y / self.viewport_height) * 2.0 - 1.0);
        
        // Calculate the world point under the cursor before zoom
        let world_x = mouse_x_norm / old_zoom + self.position[0];
        let world_y = mouse_y_norm / old_zoom + self.position[1];
        
        // Calculate where this world point should be in NDC after zoom
        let new_ndc_x = (world_x - self.position[0]) * self.zoom;
        let new_ndc_y = (world_y - self.position[1]) * self.zoom;
        
        // Calculate the offset needed to keep the cursor point stationary
        let offset_x = (mouse_x_norm - new_ndc_x) / self.zoom;
        let offset_y = (mouse_y_norm - new_ndc_y) / self.zoom;
        
        // Apply the offset to keep the cursor point stationary
        self.position[0] += offset_x;
        self.position[1] += offset_y;
        
        // Clamp position to reasonable bounds to prevent going too far out
        self.position[0] = self.position[0].clamp(-2.0, 2.0);
        self.position[1] = self.position[1].clamp(-2.0, 2.0);

        // Update uniform data after position change
        self.update_uniform();

        tracing::debug!(
            "Camera zoom to cursor: cursor=({:.2}, {:.2}), zoom={:.2}, pos=({:.2}, {:.2})",
            cursor_x, cursor_y, self.zoom, self.position[0], self.position[1]
        );
    }

    /// Reset camera to default position and zoom
    pub fn reset(&mut self) {
        self.position = [0.5, 0.5];
        self.zoom = 1.0;
        self.update_uniform();
        tracing::debug!("Camera reset to default position and zoom");
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
        
        let world_x = (ndc_x / self.zoom) + self.position[0];
        let world_y = (ndc_y / self.zoom) + self.position[1];
        
        WorldCoords::new(world_x, world_y)
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world: WorldCoords) -> ScreenCoords {
        let ndc_x = (world.x - self.position[0]) * self.zoom;
        let ndc_y = (world.y - self.position[1]) * self.zoom;
        
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
        let world_x = (ndc.x / self.zoom) + self.position[0];
        let world_y = (ndc.y / self.zoom) + self.position[1];
        WorldCoords::new(world_x, world_y)
    }

    /// Convert world coordinates to NDC
    pub fn world_to_ndc(&self, world: WorldCoords) -> NdcCoords {
        let ndc_x = (world.x - self.position[0]) * self.zoom;
        let ndc_y = (world.y - self.position[1]) * self.zoom;
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
}
