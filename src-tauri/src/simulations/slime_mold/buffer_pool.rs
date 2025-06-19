use std::collections::HashMap;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, Device};

/// A pool of buffers organized by size and usage flags
#[derive(Default, Debug)]
pub struct BufferPool {
    // Map from (size, usage_flags) to a vector of available buffers
    buffers: HashMap<(u64, u32), Vec<Buffer>>,
    // Track total memory usage for debugging
    total_memory_bytes: u64,
    // Maximum number of buffers to keep per size/usage combination
    max_buffers_per_key: usize,
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            total_memory_bytes: 0,
            max_buffers_per_key: 3, // Keep up to 3 buffers of each size/usage
        }
    }

    /// Get a buffer from the pool, or create a new one if none available
    pub fn get_buffer(
        &mut self,
        device: &Device,
        label: Option<&str>,
        size: u64,
        usage: BufferUsages,
    ) -> Buffer {
        let key = (size, usage.bits());

        // Try to reuse an existing buffer
        if let Some(buffer_vec) = self.buffers.get_mut(&key) {
            if let Some(buffer) = buffer_vec.pop() {
                tracing::debug!("Reusing buffer from pool: size={}, usage={:?}", size, usage);
                return buffer;
            }
        }

        // Create a new buffer if none available
        tracing::debug!("Creating new buffer: size={}, usage={:?}", size, usage);
        self.total_memory_bytes += size;

        device.create_buffer(&BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    /// Return a buffer to the pool for reuse
    pub fn return_buffer(&mut self, buffer: Buffer, size: u64, usage: BufferUsages) {
        let key = (size, usage.bits());

        let buffer_vec = self.buffers.entry(key).or_default();

        // Only keep up to max_buffers_per_key buffers
        if buffer_vec.len() < self.max_buffers_per_key {
            tracing::debug!("Returning buffer to pool: size={}, usage={:?}", size, usage);
            buffer_vec.push(buffer);
        } else {
            tracing::debug!(
                "Buffer pool full, dropping buffer: size={}, usage={:?}",
                size,
                usage
            );
            // Buffer will be dropped automatically
            self.total_memory_bytes = self.total_memory_bytes.saturating_sub(size);
        }
    }

    /// Clear all buffers from the pool
    pub fn clear(&mut self) {
        tracing::debug!(
            "Clearing buffer pool, releasing {} bytes",
            self.total_memory_bytes
        );
        self.buffers.clear();
        self.total_memory_bytes = 0;
    }
}

impl Drop for BufferPool {
    fn drop(&mut self) {
        if !self.buffers.is_empty() {
            tracing::debug!(
                "Dropping BufferPool with {} buffer types",
                self.buffers.len()
            );
        }
    }
}
