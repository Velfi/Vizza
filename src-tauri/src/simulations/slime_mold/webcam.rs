use crate::error::{SimulationError, SimulationResult};
use nokhwa::{
    Camera,
    buffer::Buffer,
    pixel_format::RgbFormat,
    utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType},
};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread;

/// Webcam capture state
pub struct WebcamCapture {
    pub is_active: bool,
    pub device_index: i32,
    pub current_frame_data: Arc<Mutex<Option<Vec<u8>>>>,
    pub target_width: u32,
    pub target_height: u32,
    pub cached_format: Option<CameraFormat>,
    pub capture_thread_handle: Option<thread::JoinHandle<()>>,
}

impl WebcamCapture {
    pub fn new() -> Self {
        Self {
            is_active: false,
            device_index: 0,
            current_frame_data: Arc::new(Mutex::new(None)),
            target_width: 640,
            target_height: 480,
            cached_format: None,
            capture_thread_handle: None,
        }
    }

    /// Start webcam capture
    pub fn start_capture(&mut self, device_index: i32) -> SimulationResult<()> {
        if self.is_active {
            return Ok(());
        }

        self.device_index = device_index;
        self.is_active = true;

        // Get and cache the best format for this device
        let format = match Self::get_best_camera_format(device_index) {
            Ok(format) => format,
            Err(e) => {
                tracing::warn!("Webcam device {} is not available: {}", device_index, e);
                self.is_active = false;
                return Err(e);
            }
        };

        self.cached_format = Some(format.clone());

        // Clone data for the background thread
        let frame_data = self.current_frame_data.clone();
        let target_width = self.target_width;
        let target_height = self.target_height;
        let device_index = self.device_index;

        // Spawn background thread for continuous frame capture
        let handle = thread::spawn(move || {
            // Create camera in the background thread
            let camera_index = CameraIndex::Index(device_index as u32);
            let requested_format =
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format));

            match Camera::new(camera_index, requested_format) {
                Ok(mut camera) => {
                    // Open the camera stream before attempting to grab frames
                    if let Err(e) = camera.open_stream() {
                        tracing::error!("Failed to open camera stream in background thread: {}", e);
                        return;
                    }

                    // Continuous frame capture loop
                    let mut frame_count = 0;
                    let mut error_count = 0;
                    loop {
                        tracing::trace!(
                            "Attempting to capture frame {} in background thread",
                            frame_count + 1
                        );
                        match camera.frame() {
                            Ok(frame) => {
                                frame_count += 1;
                                error_count = 0; // Reset error count on success
                                tracing::trace!(
                                    "Captured frame {} in background thread",
                                    frame_count
                                );

                                // Convert frame to grayscale and resize
                                if let Ok(gray_data) = Self::convert_to_grayscale(
                                    &frame,
                                    format.width(),
                                    format.height(),
                                    target_width,
                                    target_height,
                                ) {
                                    if let Ok(mut current_data) = frame_data.lock() {
                                        // Only replace if we actually got decoded/resized data
                                        if !gray_data.is_empty() {
                                            *current_data = Some(gray_data);
                                        }
                                    } else {
                                        tracing::warn!("Failed to lock frame data mutex");
                                    }
                                } else {
                                    tracing::warn!("Failed to convert frame to grayscale");
                                }
                            }
                            Err(e) => {
                                error_count += 1;
                                tracing::error!(
                                    "Failed to capture frame {} in background thread: {}",
                                    error_count,
                                    e
                                );

                                // If we get too many consecutive errors, something is seriously wrong
                                if error_count > 10 {
                                    tracing::error!(
                                        "Too many consecutive frame capture errors, stopping background thread"
                                    );
                                    break;
                                }
                            }
                        }

                        // Small delay to prevent excessive CPU usage
                        thread::sleep(std::time::Duration::from_millis(33)); // ~30 FPS
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create camera in background thread: {}", e);
                }
            }
        });

        self.capture_thread_handle = Some(handle);

        tracing::debug!(
            "Webcam capture started successfully for device {} with format {}x{}@{}FPS",
            device_index,
            self.cached_format.as_ref().unwrap().width(),
            self.cached_format.as_ref().unwrap().height(),
            self.cached_format.as_ref().unwrap().frame_rate()
        );
        Ok(())
    }

    /// Stop webcam capture
    pub fn stop_capture(&mut self) {
        if self.is_active {
            self.is_active = false;
            self.cached_format = None; // Clear cached format

            // Clear frame data
            if let Ok(mut current_data) = self.current_frame_data.lock() {
                *current_data = None;
            }

            // Note: We can't easily stop the background thread here since it's in an infinite loop
            // The thread will continue running until the process exits, but it won't affect
            // the main simulation since is_active is now false
        }
    }

    /// Update frame data (call this from the main thread)
    /// Note: This is now a no-op since the background thread handles frame capture
    pub fn update_frame(&mut self) -> SimulationResult<()> {
        // The background thread is now handling frame capture
        // This method is kept for compatibility but doesn't need to do anything
        if self.is_active {}
        Ok(())
    }

    /// Convert frame to grayscale and resize to the target dimensions
    fn convert_to_grayscale(
        frame: &Buffer,
        src_width: u32,
        src_height: u32,
        target_width: u32,
        target_height: u32,
    ) -> SimulationResult<Vec<u8>> {
        // Decode to RGB regardless of camera native format (e.g., MJPEG, YUYV)
        let rgb = frame.decode_image::<RgbFormat>().map_err(|e| {
            SimulationError::InvalidParameter(format!("Failed to decode frame: {}", e))
        })?;

        // Convert to grayscale (luminance)
        let mut gray = vec![0u8; (src_width * src_height) as usize];
        let mut idx = 0usize;
        for px in rgb.chunks_exact(3) {
            let r = px[0] as f32;
            let g = px[1] as f32;
            let b = px[2] as f32;
            gray[idx] = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            idx += 1;
        }

        // If dimensions already match, return grayscale directly
        if src_width == target_width && src_height == target_height {
            return Ok(gray);
        }

        // Nearest-neighbor resize to target size
        let mut resized = vec![0u8; (target_width * target_height) as usize];
        let scale_x = src_width as f32 / target_width as f32;
        let scale_y = src_height as f32 / target_height as f32;
        for ty in 0..target_height {
            let sy = ((ty as f32 + 0.5) * scale_y - 0.5).clamp(0.0, (src_height - 1) as f32) as u32;
            for tx in 0..target_width {
                let sx =
                    ((tx as f32 + 0.5) * scale_x - 0.5).clamp(0.0, (src_width - 1) as f32) as u32;
                let src_idx = (sy * src_width + sx) as usize;
                let dst_idx = (ty * target_width + tx) as usize;
                resized[dst_idx] = gray[src_idx];
            }
        }

        Ok(resized)
    }

    /// Get the latest frame data as Vec<u8>
    pub fn get_latest_frame_data(&self) -> Option<Vec<u8>> {
        if let Ok(current_data) = self.current_frame_data.lock() {
            current_data.clone()
        } else {
            None
        }
    }

    /// Convert frame data to Vec<f32> for gradient buffer
    pub fn frame_data_to_gradient_buffer(
        &self,
        frame_data: &[u8],
        target_width: u32,
        target_height: u32,
    ) -> SimulationResult<Vec<f32>> {
        let expected_size = (target_width * target_height) as usize;

        // Handle cases where frame size doesn't match exactly
        let buffer = if frame_data.len() == expected_size {
            // Perfect match, use as-is
            frame_data
                .iter()
                .map(|&byte| (byte as f32) / 255.0)
                .collect::<Vec<f32>>()
        } else if frame_data.len() > 0 {
            // Resize by sampling or padding
            let mut resized = Vec::new();
            let scale_x = frame_data.len() as f32 / expected_size as f32;

            for i in 0..expected_size {
                let source_index = ((i as f32 * scale_x) as usize).min(frame_data.len() - 1);
                resized.push((frame_data[source_index] as f32) / 255.0);
            }
            resized
        } else {
            return Err(SimulationError::InvalidParameter(
                "Empty frame data".to_string(),
            ));
        };

        Ok(buffer)
    }

    /// Set target dimensions for frame capture
    pub fn set_target_dimensions(&mut self, width: u32, height: u32) {
        let old_width = self.target_width;
        let old_height = self.target_height;
        self.target_width = width;
        self.target_height = height;
        
        // If dimensions changed and capture is active, restart the capture thread
        if self.is_active && (old_width != width || old_height != height) {
            tracing::debug!(
                "Webcam target dimensions changed from {}x{} to {}x{}, restarting capture",
                old_width, old_height, width, height
            );
            
            // Stop current capture
            self.stop_capture();
            
            // Restart with new dimensions
            if let Err(e) = self.start_capture(self.device_index) {
                tracing::error!("Failed to restart webcam capture with new dimensions: {}", e);
            }
        }
    }

    /// Check if webcam is available and get its best format
    pub fn is_webcam_available(device_index: i32) -> bool {
        match Self::get_best_camera_format(device_index) {
            Ok(format) => {
                tracing::debug!(
                    "Webcam device {} is available with format: {}x{}@{}FPS, {:?}",
                    device_index,
                    format.width(),
                    format.height(),
                    format.frame_rate(),
                    format.format()
                );
                true
            }
            Err(e) => {
                tracing::debug!("Webcam device {} is not available: {}", device_index, e);
                false
            }
        }
    }

    /// Get the best available camera format for a device
    pub fn get_best_camera_format(device_index: i32) -> SimulationResult<CameraFormat> {
        let camera_index = CameraIndex::Index(device_index as u32);

        // Try common formats in order of preference
        let fallback_formats = [
            (3840, 2160, 30), // 4K
            (1920, 1080, 30), // HD
            (1280, 720, 30),  // 720p
            (854, 480, 30),   // 480p
            (640, 480, 30),   // VGA
        ];

        for (width, height, fps) in fallback_formats.iter() {
            let format = CameraFormat::new_from(*width, *height, FrameFormat::MJPEG, *fps);
            let requested_format =
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(format.clone()));

            match Camera::new(camera_index.clone(), requested_format) {
                Ok(_) => {
                    tracing::trace!(
                        "Webcam device {} works at {}x{}@{}FPS",
                        device_index,
                        width,
                        height,
                        fps
                    );
                    return Ok(format);
                }
                Err(e) => {
                    tracing::trace!(
                        "Webcam device {} failed at {}x{}@{}FPS: {}",
                        device_index,
                        width,
                        height,
                        fps,
                        e
                    );
                }
            }
        }

        Err(SimulationError::InvalidParameter(format!(
            "No supported format found for webcam device {}",
            device_index
        )))
    }

    /// Get list of available webcam devices
    pub fn get_available_devices() -> Vec<i32> {
        let mut devices = Vec::new();

        // Check devices 0 and 1 (covers most use cases: built-in + external camera)
        for i in 0..2 {
            if Self::is_webcam_available(i) {
                devices.push(i);
            }
        }

        devices
    }
}

impl Default for WebcamCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for WebcamCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebcamCapture")
            .field("is_active", &self.is_active)
            .field("device_index", &self.device_index)
            .field("target_width", &self.target_width)
            .field("target_height", &self.target_height)
            .finish()
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        self.stop_capture();
    }
}
