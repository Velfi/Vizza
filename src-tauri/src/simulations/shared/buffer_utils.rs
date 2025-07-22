//! # Buffer Utilities Module
//!
//! Provides shared buffer creation utilities to eliminate duplication
//! of buffer creation patterns across simulations.

use bytemuck::Pod;
use std::sync::Arc;
use wgpu::{Buffer, BufferUsages, Device, util::DeviceExt};

/// Common buffer usage patterns
pub struct BufferUsage {
    pub storage: bool,
    pub uniform: bool,
    pub vertex: bool,
    pub index: bool,
    pub copy_src: bool,
    pub copy_dst: bool,
    pub map_read: bool,
    pub map_write: bool,
}

impl BufferUsage {
    /// Create storage buffer usage
    pub fn storage() -> Self {
        Self {
            storage: true,
            uniform: false,
            vertex: false,
            index: false,
            copy_src: false,
            copy_dst: true,
            map_read: false,
            map_write: false,
        }
    }

    /// Create uniform buffer usage
    pub fn uniform() -> Self {
        Self {
            storage: false,
            uniform: true,
            vertex: false,
            index: false,
            copy_src: false,
            copy_dst: true,
            map_read: false,
            map_write: false,
        }
    }

    /// Convert to wgpu BufferUsages
    pub fn to_wgpu_usage(&self) -> BufferUsages {
        let mut usage = BufferUsages::empty();
        if self.storage {
            usage |= BufferUsages::STORAGE;
        }
        if self.uniform {
            usage |= BufferUsages::UNIFORM;
        }
        if self.vertex {
            usage |= BufferUsages::VERTEX;
        }
        if self.index {
            usage |= BufferUsages::INDEX;
        }
        if self.copy_src {
            usage |= BufferUsages::COPY_SRC;
        }
        if self.copy_dst {
            usage |= BufferUsages::COPY_DST;
        }
        if self.map_read {
            usage |= BufferUsages::MAP_READ;
        }
        if self.map_write {
            usage |= BufferUsages::MAP_WRITE;
        }
        usage
    }
}

/// Buffer creation utilities
pub struct BufferUtils;

impl BufferUtils {
    /// Create a buffer initialized with data
    pub fn create_buffer_with_data<T: Pod>(
        device: &Arc<Device>,
        label: &str,
        data: &[T],
        usage: &BufferUsage,
    ) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: usage.to_wgpu_usage(),
        })
    }

    /// Create a buffer for a single struct instance
    pub fn create_buffer_for_struct<T: Pod>(
        device: &Arc<Device>,
        label: &str,
        data: &T,
        usage: &BufferUsage,
    ) -> Buffer {
        Self::create_buffer_with_data(device, label, std::slice::from_ref(data), usage)
    }

    /// Create a uniform buffer for simulation parameters
    pub fn create_uniform_buffer<T: Pod>(device: &Arc<Device>, label: &str, data: &T) -> Buffer {
        Self::create_buffer_for_struct(device, label, data, &BufferUsage::uniform())
    }

    /// Create a storage buffer for data arrays
    pub fn create_storage_buffer<T: Pod>(device: &Arc<Device>, label: &str, data: &[T]) -> Buffer {
        Self::create_buffer_with_data(device, label, data, &BufferUsage::storage())
    }
}
