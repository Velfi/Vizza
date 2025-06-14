use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{Device, Queue};
use tracing;
use super::coordinates::{ScreenCoords, WorldCoords, NdcCoords, CoordinateTransform};

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
        self.screen_to_world_typed(screen)
    }

    fn world_to_screen(&self, world: WorldCoords) -> ScreenCoords {
        self.world_to_screen_typed(world)
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
pub struct Camera {
    /// Camera position in world space
    pub position: [f32; 2],
    /// Zoom level (1.0 = normal, >1.0 = zoomed in, <1.0 = zoomed out)
    pub zoom: f32,
    /// Viewport dimensions
    pub viewport_width: f32,
    pub viewport_height: f32,
    /// Camera movement speed
    pub pan_speed: f32,
    /// Zoom speed multiplier
    pub zoom_speed: f32,
    /// Minimum and maximum zoom levels
    pub min_zoom: f32,
    pub max_zoom: f32,
    /// Smooth panning state
    velocity: [f32; 2],
    target_velocity: [f32; 2],
    momentum_decay: f32,
    /// Simulation bounds (in world space)
    bounds: [f32; 4], // [min_x, min_y, max_x, max_y]
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
        let position = [0.0, 0.0];
        let zoom = 1.0;
        let aspect_ratio = viewport_width / viewport_height;

        let uniform_data = CameraUniform {
            view_proj_matrix: Self::calculate_view_proj_matrix(position, zoom, aspect_ratio),
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
            pan_speed: 10.0,
            zoom_speed: 0.1,
            min_zoom: 0.1,
            max_zoom: 10.0,
            velocity: [0.0, 0.0],
            target_velocity: [0.0, 0.0],
            momentum_decay: 0.95,
            bounds: [-1.0, -1.0, 1.0, 1.0],
            buffer,
            uniform_data,
        })
    }

    /// Calculate the view-projection matrix for 2D rendering
    fn calculate_view_proj_matrix(position: [f32; 2], zoom: f32, aspect_ratio: f32) -> [f32; 16] {
        // Create orthographic projection matrix
        let left = -1.0 / zoom;
        let right = 1.0 / zoom;
        let bottom = -1.0 / (zoom * aspect_ratio);
        let top = 1.0 / (zoom * aspect_ratio);
        let near = -1.0;
        let far = 1.0;

        // Orthographic projection matrix
        let ortho = [
            2.0 / (right - left), 0.0, 0.0, 0.0,
            0.0, 2.0 / (top - bottom), 0.0, 0.0,
            0.0, 0.0, -2.0 / (far - near), 0.0,
            -(right + left) / (right - left), -(top + bottom) / (top - bottom), -(far + near) / (far - near), 1.0,
        ];

        // Translation matrix for camera position
        let view = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            -position[0], -position[1], 0.0, 1.0,
        ];

        // Combine projection and view matrices (proj * view)
        Self::multiply_matrices_4x4(&ortho, &view)
    }

    /// Multiply two 4x4 matrices
    fn multiply_matrices_4x4(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
        let mut result = [0.0; 16];
        
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i * 4 + j] += a[i * 4 + k] * b[k * 4 + j];
                }
            }
        }
        
        result
    }

    /// Update camera state (call this every frame for smooth movement)
    pub fn update(&mut self, delta_time: f32) -> bool {
        let mut changed = false;
        
        // Calculate view bounds at current zoom level
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let view_width = 2.0 / self.zoom;
        let view_height = 2.0 / (self.zoom * aspect_ratio);
        
        // Calculate bounds that keep the view within simulation bounds
        let min_x = self.bounds[0] + view_width * 0.5;
        let max_x = self.bounds[2] - view_width * 0.5;
        let min_y = self.bounds[1] + view_height * 0.5;
        let max_y = self.bounds[3] - view_height * 0.5;
        
        // Smooth velocity interpolation
        let lerp_factor = 1.0 - self.momentum_decay.powf(delta_time * 60.0); // 60fps reference
        self.velocity[0] = self.velocity[0] + (self.target_velocity[0] - self.velocity[0]) * lerp_factor;
        self.velocity[1] = self.velocity[1] + (self.target_velocity[1] - self.velocity[1]) * lerp_factor;
        
        // Apply velocity to position
        if self.velocity[0].abs() > 0.001 || self.velocity[1].abs() > 0.001 {
            let new_x = self.position[0] + self.velocity[0] * delta_time;
            let new_y = self.position[1] + self.velocity[1] * delta_time;
            
            // Clamp position to bounds
            self.position[0] = new_x.clamp(min_x, max_x);
            self.position[1] = new_y.clamp(min_y, max_y);
            
            // Stop velocity if we hit bounds
            if self.position[0] == min_x || self.position[0] == max_x {
                self.velocity[0] = 0.0;
                self.target_velocity[0] = 0.0;
            }
            if self.position[1] == min_y || self.position[1] == max_y {
                self.velocity[1] = 0.0;
                self.target_velocity[1] = 0.0;
            }
            
            changed = true;
            tracing::debug!(
                "Camera position updated: pos=({:.2}, {:.2}), velocity=({:.2}, {:.2}), bounds=({:.2}, {:.2}, {:.2}, {:.2})",
                self.position[0], self.position[1], self.velocity[0], self.velocity[1],
                min_x, min_y, max_x, max_y
            );
        }
        
        // Apply momentum decay when no input
        if self.target_velocity[0].abs() < 0.001 && self.target_velocity[1].abs() < 0.001 {
            self.velocity[0] *= self.momentum_decay.powf(delta_time * 60.0);
            self.velocity[1] *= self.momentum_decay.powf(delta_time * 60.0);
        }
        
        // Stop very small movements
        if self.velocity[0].abs() < 0.001 {
            self.velocity[0] = 0.0;
        }
        if self.velocity[1].abs() < 0.001 {
            self.velocity[1] = 0.0;
        }
        
        if changed {
            self.update_uniform();
        }
        
        changed
    }

    /// Update camera position (panning) with smooth movement
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Calculate view bounds at current zoom level
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let view_width = 2.0 / self.zoom;
        let view_height = 2.0 / (self.zoom * aspect_ratio);
        
        // Calculate bounds that keep the view within simulation bounds
        // The bounds should be the simulation bounds minus half the view size
        let min_x = self.bounds[0] + view_width * 0.5;
        let max_x = self.bounds[2] - view_width * 0.5;
        let min_y = self.bounds[1] + view_height * 0.5;
        let max_y = self.bounds[3] - view_height * 0.5;
        
        // Scale movement by zoom (more zoom = slower movement)
        let scale = 1.0 / self.zoom;
        
        // Calculate target velocity, but don't exceed bounds
        let target_dx = delta_x * self.pan_speed * scale;
        let target_dy = delta_y * self.pan_speed * scale;
        
        // Only set target velocity if we won't exceed bounds
        let new_x = self.position[0] + target_dx;
        let new_y = self.position[1] + target_dy;
        
        if new_x >= min_x && new_x <= max_x {
            self.target_velocity[0] = target_dx;
        } else {
            self.target_velocity[0] = 0.0;
        }
        
        if new_y >= min_y && new_y <= max_y {
            self.target_velocity[1] = target_dy;
        } else {
            self.target_velocity[1] = 0.0;
        }
        
        tracing::debug!(
            "Camera pan: delta=({:.2}, {:.2}), target_velocity=({:.2}, {:.2}), zoom={:.2}, bounds=({:.2}, {:.2}, {:.2}, {:.2})",
            delta_x, delta_y, self.target_velocity[0], self.target_velocity[1], self.zoom,
            min_x, min_y, max_x, max_y
        );
    }

    /// Stop camera movement (call when input stops)
    pub fn stop_pan(&mut self) {
        // Only log and update if we're actually moving
        if self.target_velocity[0].abs() > 0.001 || self.target_velocity[1].abs() > 0.001 {
            self.target_velocity = [0.0, 0.0];
            tracing::debug!(
                "Camera pan stopped: velocity=({:.2}, {:.2})",
                self.velocity[0], self.velocity[1]
            );
        }
    }

    /// Update zoom level (center-based zoom for keyboard)
    pub fn zoom(&mut self, delta: f32) {
        let old_zoom = self.zoom;
        let new_zoom = self.zoom * (1.0 + delta * self.zoom_speed);
        let clamped_zoom = new_zoom.clamp(self.min_zoom, self.max_zoom);
        
        // Only proceed if zoom actually changed
        if (clamped_zoom - old_zoom).abs() < 0.001 {
            return;
        }

        // Calculate view bounds at new zoom level
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let view_width = 2.0 / clamped_zoom;
        let view_height = 2.0 / (clamped_zoom * aspect_ratio);
        
        // Calculate new position that keeps the view within bounds
        let min_x = self.bounds[0] + view_width * 0.5;
        let max_x = self.bounds[2] - view_width * 0.5;
        let min_y = self.bounds[1] + view_height * 0.5;
        let max_y = self.bounds[3] - view_height * 0.5;
        
        // If the view would be too large for the bounds, don't zoom
        if min_x > max_x || min_y > max_y {
            return;
        }
        
        // Apply the new zoom
        self.zoom = clamped_zoom;
        
        // Clamp position to keep view within bounds
        self.position[0] = self.position[0].clamp(min_x, max_x);
        self.position[1] = self.position[1].clamp(min_y, max_y);
        
        self.update_uniform();
    }

    /// Zoom towards a specific screen position (for mouse wheel)
    pub fn zoom_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        // Store old zoom for calculation
        let old_zoom = self.zoom;
        
        // Calculate new zoom level
        let new_zoom = self.zoom * (1.0 + delta * self.zoom_speed);
        let clamped_zoom = new_zoom.clamp(self.min_zoom, self.max_zoom);
        
        // Only proceed if zoom actually changed
        if (clamped_zoom - old_zoom).abs() < 0.001 {
            return;
        }
        
        // Get world position at cursor before zoom change
        let screen_coords = ScreenCoords::new(cursor_x, cursor_y);
        let world_before = self.screen_to_world_typed(screen_coords);
        
        // Calculate view bounds at new zoom level
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let view_width = 2.0 / clamped_zoom;
        let view_height = 2.0 / (clamped_zoom * aspect_ratio);
        
        // Calculate new position that keeps the view within bounds
        let min_x = self.bounds[0] + view_width * 0.5;
        let max_x = self.bounds[2] - view_width * 0.5;
        let min_y = self.bounds[1] + view_height * 0.5;
        let max_y = self.bounds[3] - view_height * 0.5;
        
        // If the view would be too large for the bounds, don't zoom
        if min_x > max_x || min_y > max_y {
            return;
        }
        
        // Apply the new zoom
        self.zoom = clamped_zoom;
        
        // Calculate the scale factor for the zoom change
        let scale = clamped_zoom / old_zoom;
        
        // Calculate the offset from cursor to camera center in world space
        let dx = world_before.x - self.position[0];
        let dy = world_before.y - self.position[1];
        
        // Scale the offset by the zoom change
        let new_dx = dx * scale;
        let new_dy = dy * scale;
        
        // Calculate new camera position to keep cursor point fixed
        // When zooming in (scale > 1), we want to move camera towards cursor
        // When zooming out (scale < 1), we want to move camera away from cursor
        let new_x = world_before.x - new_dx;
        let new_y = world_before.y - new_dy;
        
        // Clamp position to keep view within bounds
        self.position[0] = new_x.clamp(min_x, max_x);
        self.position[1] = new_y.clamp(min_y, max_y);
        
        // Final update with corrected position
        self.update_uniform();
        
        // Debug logging
        tracing::debug!(
            "Camera zoom to cursor: cursor=({:.2}, {:.2}), world_before=({:.4}, {:.4}), world_after=({:.4}, {:.4}), zoom={:.4}->{:.4}",
            cursor_x, cursor_y, world_before.x, world_before.y, 
            self.screen_to_world_typed(screen_coords).x, self.screen_to_world_typed(screen_coords).y,
            old_zoom, self.zoom
        );
    }

    /// Set absolute zoom level
    pub fn set_zoom(&mut self, zoom: f32) {
        let new_zoom = zoom.clamp(self.min_zoom, self.max_zoom);
        
        // Calculate view bounds at new zoom level
        let aspect_ratio = self.viewport_width / self.viewport_height;
        let view_width = 2.0 / new_zoom;
        let view_height = 2.0 / (new_zoom * aspect_ratio);
        
        // Calculate new position that keeps the view within bounds
        let min_x = self.bounds[0] + view_width * 0.5;
        let max_x = self.bounds[2] - view_width * 0.5;
        let min_y = self.bounds[1] + view_height * 0.5;
        let max_y = self.bounds[3] - view_height * 0.5;
        
        // If the view would be too large for the bounds, don't change zoom
        if min_x > max_x || min_y > max_y {
            return;
        }
        
        self.zoom = new_zoom;
        
        // Clamp position to keep view within bounds
        self.position[0] = self.position[0].clamp(min_x, max_x);
        self.position[1] = self.position[1].clamp(min_y, max_y);
        
        self.update_uniform();
    }

    /// Set absolute position
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = [x, y];
        self.update_uniform();
    }

    /// Reset camera to default position and zoom
    pub fn reset(&mut self) {
        self.position = [0.0, 0.0];
        self.zoom = 1.0;
        self.velocity = [0.0, 0.0];
        self.target_velocity = [0.0, 0.0];
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
            view_proj_matrix: Self::calculate_view_proj_matrix(self.position, self.zoom, aspect_ratio),
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

    /// Get current uniform data (for debugging)
    pub fn uniform_data(&self) -> &CameraUniform {
        &self.uniform_data
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> [f32; 2] {
        self.screen_to_world_typed(ScreenCoords::new(screen_x, screen_y)).to_array()
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world_x: f32, world_y: f32) -> [f32; 2] {
        self.world_to_screen_typed(WorldCoords::new(world_x, world_y)).to_array()
    }

    /// Typed version of screen_to_world conversion
    pub fn screen_to_world_typed(&self, screen: ScreenCoords) -> WorldCoords {
        let ndc = self.screen_to_ndc(screen);
        self.ndc_to_world(ndc)
    }

    /// Typed version of world_to_screen conversion
    pub fn world_to_screen_typed(&self, world: WorldCoords) -> ScreenCoords {
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
        let world_x = (ndc.x / self.zoom) + self.position[0];
        let world_y = (ndc.y / (self.zoom * self.uniform_data.aspect_ratio)) + self.position[1];
        WorldCoords::new(world_x, world_y)
    }

    /// Convert world coordinates to NDC
    pub fn world_to_ndc(&self, world: WorldCoords) -> NdcCoords {
        let ndc_x = (world.x - self.position[0]) * self.zoom;
        let ndc_y = (world.y - self.position[1]) * self.zoom * self.uniform_data.aspect_ratio;
        NdcCoords::new(ndc_x, ndc_y)
    }

    /// Get current zoom level
    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    /// Get current position
    pub fn get_position(&self) -> [f32; 2] {
        self.position
    }

    /// Set simulation bounds
    /// Test coordinate transformation round-trip for debugging
    pub fn test_coordinate_transform(&self, test_name: &str) {
        tracing::info!("=== Camera Coordinate Transform Test: {} ===", test_name);
        tracing::info!("Camera state: pos=({:.4}, {:.4}), zoom={:.4}, aspect={:.4}, viewport=({:.1}, {:.1})", 
            self.position[0], self.position[1], self.zoom, self.uniform_data.aspect_ratio,
            self.viewport_width, self.viewport_height);
        
        // Test center of screen
        let center_screen = ScreenCoords::new(self.viewport_width / 2.0, self.viewport_height / 2.0);
        let center_world = self.screen_to_world_typed(center_screen);
        let center_back = self.world_to_screen_typed(center_world);
        
        tracing::info!("Center test: screen=({:.1}, {:.1}) -> world=({:.4}, {:.4}) -> screen=({:.1}, {:.1})",
            center_screen.x, center_screen.y, center_world.x, center_world.y, center_back.x, center_back.y);
        
        // Test corner
        let corner_screen = ScreenCoords::new(0.0, 0.0);
        let corner_world = self.screen_to_world_typed(corner_screen);
        let corner_back = self.world_to_screen_typed(corner_world);
        
        tracing::info!("Corner test: screen=({:.1}, {:.1}) -> world=({:.4}, {:.4}) -> screen=({:.1}, {:.1})",
            corner_screen.x, corner_screen.y, corner_world.x, corner_world.y, corner_back.x, corner_back.y);
        
        // Test world origin
        let origin_world = WorldCoords::new(0.0, 0.0);
        let origin_screen = self.world_to_screen_typed(origin_world);
        let origin_back = self.screen_to_world_typed(origin_screen);
        
        tracing::info!("Origin test: world=({:.4}, {:.4}) -> screen=({:.1}, {:.1}) -> world=({:.4}, {:.4})",
            origin_world.x, origin_world.y, origin_screen.x, origin_screen.y, origin_back.x, origin_back.y);
        
        tracing::info!("=== End Test ===");
    }

    pub fn set_bounds(&mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) {
        self.bounds = [min_x, min_y, max_x, max_y];
        // Clamp current position to new bounds
        self.position[0] = self.position[0].clamp(min_x, max_x);
        self.position[1] = self.position[1].clamp(min_y, max_y);
        self.update_uniform();
    }
}