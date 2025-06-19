use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::GpuContext;

pub struct RenderLoopManager {
    render_loop_running: Arc<AtomicBool>,
    fps_limit_enabled: Arc<AtomicBool>,
    fps_limit: Arc<AtomicU32>,
}

impl RenderLoopManager {
    pub fn new() -> Self {
        Self {
            render_loop_running: Arc::new(AtomicBool::new(false)),
            fps_limit_enabled: Arc::new(AtomicBool::new(false)),
            fps_limit: Arc::new(AtomicU32::new(60)),
        }
    }

    pub fn start_render_loop(
        &self,
        app_handle: AppHandle,
        gpu_context: Arc<tokio::sync::Mutex<GpuContext>>,
        manager: Arc<tokio::sync::Mutex<crate::simulation::SimulationManager>>,
    ) {
        if self.render_loop_running.load(Ordering::Relaxed) {
            tracing::warn!("Render loop is already running");
            return;
        }

        self.render_loop_running.store(true, Ordering::Relaxed);
        let render_loop_running = self.render_loop_running.clone();
        let fps_limit_enabled = self.fps_limit_enabled.clone();
        let fps_limit = self.fps_limit.clone();

        tauri::async_runtime::spawn(async move {
            let mut last_frame_time = Instant::now();
            let mut frame_count = 0;
            let mut fps_start_time = Instant::now();

            while render_loop_running.load(Ordering::Relaxed) {
                let frame_start = Instant::now();

                // Check if we should limit FPS
                if fps_limit_enabled.load(Ordering::Relaxed) {
                    let target_frame_time = Duration::from_secs_f32(1.0 / fps_limit.load(Ordering::Relaxed) as f32);
                    let elapsed = frame_start.duration_since(last_frame_time);
                    
                    if elapsed < target_frame_time {
                        let sleep_duration = target_frame_time - elapsed;
                        tokio::time::sleep(sleep_duration).await;
                        continue;
                    }
                }

                // Get GPU context and manager
                let gpu_context_guard = gpu_context.lock().await;
                let mut manager_guard = manager.lock().await;

                // Get current surface texture
                match gpu_context_guard.get_current_texture() {
                    Ok(surface_texture) => {
                        let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

                        // Render the current simulation or main menu
                        if let Err(e) = manager_guard.render(&gpu_context_guard.device, &gpu_context_guard.queue, &surface_view) {
                            tracing::error!("Render error: {}", e);
                        }

                        // Present the frame
                        surface_texture.present();
                    }
                    Err(e) => {
                        tracing::error!("Failed to get surface texture: {}", e);
                        tokio::time::sleep(Duration::from_millis(16)).await; // ~60 FPS fallback
                        continue;
                    }
                }

                // Update frame timing
                last_frame_time = frame_start;
                frame_count += 1;

                // Calculate and emit FPS every second
                let fps_elapsed = frame_start.duration_since(fps_start_time);
                if fps_elapsed >= Duration::from_secs(1) {
                    let current_fps = frame_count as f32 / fps_elapsed.as_secs_f32();
                    
                    // Emit FPS to frontend
                    if let Err(e) = app_handle.emit_all("fps-update", current_fps) {
                        tracing::error!("Failed to emit FPS update: {}", e);
                    }

                    frame_count = 0;
                    fps_start_time = frame_start;
                }

                // Small delay to prevent busy waiting
                tokio::time::sleep(Duration::from_micros(100)).await;
            }

            tracing::info!("Render loop stopped");
        });
    }

    pub fn stop_render_loop(&self) {
        self.render_loop_running.store(false, Ordering::Relaxed);
    }

    pub fn set_fps_limit(&self, enabled: bool, limit: u32) {
        self.fps_limit_enabled.store(enabled, Ordering::Relaxed);
        self.fps_limit.store(limit, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.render_loop_running.load(Ordering::Relaxed)
    }
} 