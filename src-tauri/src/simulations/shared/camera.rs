use super::coordinates::{CoordinateTransform, NdcCoords, ScreenCoords, WorldCoords};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Vector3};
use std::sync::Arc;
use tracing;
use wgpu::{Device, Queue};

/// GPU-compatible camera uniform data
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    /// View-projection matrix (4x4 matrix stored as 16 floats)
    pub view_proj_matrix: [f32; 16],
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

/// Generic 2D camera system for GPU rendering
/// Based on particle life camera with GPU buffer compatibility
pub struct Camera {
    /// Camera position (using nalgebra for smooth math)
    pub position: Vector3<f64>,
    /// Camera size (zoom level - smaller = more zoomed in)
    pub size: f64,
    /// Target position for smooth movement
    pub target_position: Vector3<f64>,
    /// Target size for smooth zooming
    pub target_size: f64,
    /// Viewport dimensions
    pub viewport_width: f32,
    pub viewport_height: f32,
    /// Camera movement speed
    pub pan_speed: f32,
    /// Zoom speed multiplier
    pub zoom_speed: f32,
    /// Minimum and maximum size levels (inverse of zoom)
    pub min_size: f64,
    pub max_size: f64,
    /// Smoothness factor for movement (lower = smoother)
    pub smoothness: f64,
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
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let position = Vector3::new(0.0, 0.0, 0.0);
        let size = 8.0; // Start with a good overview of the world
        let aspect_ratio = viewport_width / viewport_height;

        let uniform_data = CameraUniform {
            view_proj_matrix: Self::calculate_view_proj_matrix(position, size, aspect_ratio),
            position: [position.x as f32, position.y as f32],
            zoom: 1.0 / size as f32,
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
            size,
            target_position: position,
            target_size: size,
            viewport_width,
            viewport_height,
            pan_speed: 0.2,
            zoom_speed: 0.1,
            min_size: 0.01, // Very zoomed in
            max_size: 20.0, // Very zoomed out
            smoothness: 0.1,
            buffer,
            uniform_data,
        })
    }

    /// Calculate the view-projection matrix using nalgebra
    fn calculate_view_proj_matrix(
        position: Vector3<f64>,
        size: f64,
        aspect_ratio: f32,
    ) -> [f32; 16] {
        let left = (position.x - size * 0.5) as f32;
        let right = (position.x + size * 0.5) as f32;
        let bottom = (position.y + size * 0.5 / aspect_ratio as f64) as f32;
        let top = (position.y - size * 0.5 / aspect_ratio as f64) as f32;

        let matrix = Matrix4::new_orthographic(left, right, bottom, top, -1.0, 1.0);

        // Convert nalgebra matrix to array format expected by wgpu
        [
            matrix.m11, matrix.m21, matrix.m31, matrix.m41, matrix.m12, matrix.m22, matrix.m32,
            matrix.m42, matrix.m13, matrix.m23, matrix.m33, matrix.m43, matrix.m14, matrix.m24,
            matrix.m34, matrix.m44,
        ]
    }

    /// Update camera state (call this every frame for smooth movement)
    pub fn update(&mut self, delta_time: f32) -> bool {
        let old_position = self.position;
        let old_size = self.size;

        // Smooth interpolation towards targets
        self.position = self.position.lerp(&self.target_position, self.smoothness);
        self.size = self.size + (self.target_size - self.size) * self.smoothness;

        // Check if camera actually moved
        let position_delta = (self.position - old_position).magnitude();
        let size_delta = (self.size - old_size).abs();

        let changed = position_delta > 0.001 || size_delta > 0.001;

        if changed {
            self.update_uniform();
            tracing::debug!(
                "Camera updated: pos=({:.2}, {:.2}), size={:.2}",
                self.position.x,
                self.position.y,
                self.size
            );
        }

        changed
    }

    /// Update camera position (panning) with smooth movement
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Scale movement by current size (zoom level)
        let new_x =
            self.target_position.x + delta_x as f64 * self.pan_speed as f64 * self.target_size;
        let new_y =
            self.target_position.y + delta_y as f64 * self.pan_speed as f64 * self.target_size;

        self.apply_position_bounds(new_x, new_y);

        tracing::debug!(
            "Camera pan: delta=({:.2}, {:.2}), target_pos=({:.2}, {:.2}), size={:.2}",
            delta_x,
            delta_y,
            self.target_position.x,
            self.target_position.y,
            self.target_size
        );
    }

    /// Update zoom level
    pub fn zoom(&mut self, delta: f32) {
        let zoom_factor = 1.0 + delta * self.zoom_speed;
        let new_size = self.target_size * zoom_factor as f64;
        let clamped_size = new_size.clamp(self.min_size, self.max_size);

        if (clamped_size - self.target_size).abs() > 0.001 {
            self.target_size = clamped_size;
            // Re-apply position bounds in case zoom changed what's valid
            self.apply_position_bounds(self.target_position.x, self.target_position.y);

            tracing::debug!(
                "Camera zoom: delta={:.2}, new_size={:.2}",
                delta,
                clamped_size
            );
        }
    }

    /// Zoom towards a specific screen position (for mouse wheel)
    pub fn zoom_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        let old_size = self.target_size;

        // Calculate zoom factor and new size
        let zoom_factor = 1.0 + delta * self.zoom_speed;
        let new_size = self.target_size * zoom_factor as f64;
        let clamped_size = new_size.clamp(self.min_size, self.max_size);

        if (clamped_size - old_size).abs() < 0.001 {
            return;
        }

        // Convert cursor position to normalized coordinates
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let mouse_x_norm = (cursor_x / self.viewport_width) - 0.5;
        let mouse_y_norm = (cursor_y / self.viewport_height) - 0.5;

        // Calculate size difference and adjust position to zoom towards cursor
        let size_diff = clamped_size - old_size;
        let new_x = self.target_position.x - mouse_x_norm as f64 * size_diff;
        let new_y = self.target_position.y - mouse_y_norm as f64 * size_diff / aspect_ratio as f64;

        self.target_size = clamped_size;
        self.apply_position_bounds(new_x, new_y);

        tracing::debug!(
            "Camera zoom to cursor: cursor=({:.2}, {:.2}), size={:.2}->{:.2}",
            cursor_x,
            cursor_y,
            old_size,
            clamped_size
        );
    }

    /// Apply position bounds (particle life camera logic)
    fn apply_position_bounds(&mut self, new_x: f64, new_y: f64) {
        // World bounds are -2.0 to 2.0 (particle life world)
        let world_min = -2.0;
        let world_max = 2.0;
        let world_size = world_max - world_min; // = 4.0

        // Camera should never be more than half the world width from the simulation
        let max_distance = world_size * 0.5; // = 2.0

        // Calculate the maximum allowed camera bounds
        let min_camera_x = world_min - max_distance; // = -4.0
        let max_camera_x = world_max + max_distance; // = 4.0
        let min_camera_y = world_min - max_distance; // = -4.0
        let max_camera_y = world_max + max_distance; // = 4.0

        // Clamp camera position to strict bounds
        self.target_position.x = new_x.clamp(min_camera_x, max_camera_x);
        self.target_position.y = new_y.clamp(min_camera_y, max_camera_y);
    }

    /// Reset camera to default position and zoom
    pub fn reset(&mut self) {
        self.target_position = Vector3::new(0.0, 0.0, 0.0);
        self.target_size = 8.0;
        self.position = self.target_position;
        self.size = self.target_size;
        self.update_uniform();
        tracing::debug!("Camera reset to default position and size");
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
            view_proj_matrix: Self::calculate_view_proj_matrix(
                self.position,
                self.size,
                aspect_ratio,
            ),
            position: [self.position.x as f32, self.position.y as f32],
            zoom: 1.0 / self.size as f32,
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

    /// Get current uniform data (for debugging)
    pub fn uniform_data(&self) -> &CameraUniform {
        &self.uniform_data
    }

    /// Typed version of screen_to_world conversion
    pub fn screen_to_world(&self, screen: ScreenCoords) -> WorldCoords {
        let ndc = self.screen_to_ndc(screen);
        self.ndc_to_world(ndc)
    }

    /// Typed version of world_to_screen conversion
    pub fn world_to_screen(&self, world: WorldCoords) -> ScreenCoords {
        let ndc = self.world_to_ndc(world);
        self.ndc_to_screen(ndc)
    }

    /// Convert screen coordinates to NDC
    pub fn screen_to_ndc(&self, screen: ScreenCoords) -> NdcCoords {
        // Convert screen coordinates (0..viewport_size) to NDC (-1..1)
        // Screen coordinates increase right and down, NDC increases right and up
        let ndc_x = (screen.x / self.viewport_width) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen.y / self.viewport_height) * 2.0;
        NdcCoords::new(ndc_x, ndc_y)
    }

    /// Convert NDC to screen coordinates
    pub fn ndc_to_screen(&self, ndc: NdcCoords) -> ScreenCoords {
        // Convert NDC (-1..1) to screen coordinates (0..viewport_size)
        // NDC increases right and up, screen coordinates increase right and down
        let screen_x = (ndc.x + 1.0) * self.viewport_width * 0.5;
        let screen_y = (1.0 - ndc.y) * self.viewport_height * 0.5;
        ScreenCoords::new(screen_x, screen_y)
    }

    /// Convert NDC to world coordinates
    pub fn ndc_to_world(&self, ndc: NdcCoords) -> WorldCoords {
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let world_x = (ndc.x as f64 * self.size * 0.5) + self.position.x;
        let world_y = (ndc.y as f64 * self.size * 0.5 / aspect_ratio as f64) + self.position.y;
        WorldCoords::new(world_x as f32, world_y as f32)
    }

    /// Convert world coordinates to NDC
    pub fn world_to_ndc(&self, world: WorldCoords) -> NdcCoords {
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let ndc_x = ((world.x as f64 - self.position.x) / (self.size * 0.5)) as f32;
        let ndc_y =
            ((world.y as f64 - self.position.y) / (self.size * 0.5 / aspect_ratio as f64)) as f32;
        NdcCoords::new(ndc_x, ndc_y)
    }

    /// Get view projection matrix (particle life compatibility)
    pub fn get_view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        let left = (self.position.x - self.size * 0.5) as f32;
        let right = (self.position.x + self.size * 0.5) as f32;
        let bottom = (self.position.y + self.size * 0.5 / aspect_ratio as f64) as f32;
        let top = (self.position.y - self.size * 0.5 / aspect_ratio as f64) as f32;

        Matrix4::new_orthographic(left, right, bottom, top, -1.0, 1.0)
    }

    pub fn reset_fit_to_window(&mut self, width: f32, height: f32) {
        todo!()
    }
}
