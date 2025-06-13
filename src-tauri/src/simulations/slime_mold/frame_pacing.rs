use std::time::Duration;

pub struct FramePacing {
    target_frame_time: Duration,
    frame_time_history: Vec<Duration>,
    sleep_compensation: Duration,
    last_adjustment: std::time::Instant,
    adjustment_interval: Duration,
}

impl FramePacing {
    pub fn new(target_fps: f32) -> Self {
        Self {
            target_frame_time: Duration::from_secs_f32(1.0 / target_fps),
            frame_time_history: Vec::with_capacity(60),
            sleep_compensation: Duration::from_micros(2000), // Start with 2ms compensation
            #[cfg(not(target_arch = "wasm32"))]
            last_adjustment: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            last_adjustment: web_time::Instant::now(),
            adjustment_interval: Duration::from_secs(1), // Adjust compensation every second
        }
    }

    pub fn update(&mut self, frame_time: Duration) {
        // Add frame time to history
        self.frame_time_history.push(frame_time);
        if self.frame_time_history.len() > 60 {
            self.frame_time_history.remove(0);
        }

        // Only adjust compensation periodically
        #[cfg(not(target_arch = "wasm32"))]
        let now = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let now = web_time::Instant::now();
        if now.duration_since(self.last_adjustment) >= self.adjustment_interval {
            self.adjust_compensation();
            self.last_adjustment = now;
        }
    }

    fn adjust_compensation(&mut self) {
        if self.frame_time_history.is_empty() {
            return;
        }

        // Calculate average frame time
        let avg_frame_time: Duration =
            self.frame_time_history.iter().sum::<Duration>() / self.frame_time_history.len() as u32;

        // Calculate how far off we are from target
        let error = avg_frame_time.as_secs_f64() - self.target_frame_time.as_secs_f64();

        // Adjust compensation based on error
        // If we're running too slow (positive error), decrease compensation
        // If we're running too fast (negative error), increase compensation
        let adjustment = Duration::from_micros((error * 1_000_000.0 * 0.1) as u64);
        let new_compensation =
            self.sleep_compensation.as_micros() as i64 + adjustment.as_micros() as i64;
        self.sleep_compensation = Duration::from_micros(
            new_compensation
                .max(500) // Minimum 500μs compensation
                .min(5000) as u64, // Maximum 5ms compensation
        );
    }

    pub fn get_sleep_time(&self, elapsed: Duration) -> Duration {
        if elapsed >= self.target_frame_time {
            return Duration::ZERO;
        }
        (self.target_frame_time - elapsed).saturating_sub(self.sleep_compensation)
    }
}
