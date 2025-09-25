use wgpu::{BindGroup, Buffer, Device};

/// Manages a pair of buffers for ping-pong operations
///
/// This struct eliminates the possibility of buffer mixups by providing
/// a clean interface for alternating between two buffers during compute operations.
/// It automatically tracks which buffer is currently active and provides
/// methods to swap them safely.
#[derive(Debug)]
pub struct PingPongBuffers {
    /// The two buffers for ping-pong operations
    buffers: [Buffer; 2],
    /// Index of the currently active buffer (0 or 1)
    current: usize,
}

impl PingPongBuffers {
    /// Create a new ping-pong buffer pair
    pub fn new(device: &Device, size: u64, usage: wgpu::BufferUsages, label: &str) -> Self {
        let create_buffer = |index: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{} {}", label, if index == 0 { "A" } else { "B" })),
                size,
                usage,
                mapped_at_creation: false,
            })
        };

        let buffer_a = create_buffer(0);
        let buffer_b = create_buffer(1);

        Self {
            buffers: [buffer_a, buffer_b],
            current: 0,
        }
    }

    /// Get the currently active buffer
    pub fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.current]
    }

    /// Get the currently inactive buffer (for writing)
    pub fn inactive_buffer(&self) -> &Buffer {
        &self.buffers[1 - self.current]
    }

    /// Swap the active and inactive buffers
    pub fn swap(&mut self) {
        self.current = 1 - self.current;
    }

    /// Get the current buffer index (0 or 1)
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Get the appropriate bind group based on current buffer state
    /// This eliminates the need for manual buffer index checking in the calling code
    pub fn get_bind_group<'a>(&self, bg_a: &'a BindGroup, bg_b: &'a BindGroup) -> &'a BindGroup {
        if self.current_index() == 0 {
            bg_a
        } else {
            bg_b
        }
    }
}
