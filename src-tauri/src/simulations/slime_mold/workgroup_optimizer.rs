use tracing::debug;
use wgpu::{AdapterInfo, Device, Limits};

#[derive(Debug, Clone)]
pub struct WorkgroupConfig {
    /// Optimal workgroup size for 1D compute operations (agents, trail processing)
    pub compute_1d: u32,
    /// Optimal workgroup size for 2D compute operations (display updates)
    pub compute_2d: (u32, u32),
}

#[derive(Debug, Clone, Copy)]
enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Unknown,
}

impl GpuVendor {
    fn detect(adapter_info: &AdapterInfo) -> Self {
        let name_lower = adapter_info.name.to_lowercase();
        let driver_lower = adapter_info.driver.to_lowercase();

        // Check by vendor ID (PCI vendor IDs) - but many platforms use 0
        match adapter_info.vendor {
            0x10DE => return GpuVendor::Nvidia, // NVIDIA Corporation
            0x1002 => return GpuVendor::Amd,    // AMD/ATI Technologies Inc.
            0x8086 => return GpuVendor::Intel,  // Intel Corporation
            _ => {}
        }

        // Apple Silicon and macOS detection (vendor ID is typically 0)
        if matches!(adapter_info.backend, wgpu::Backend::Metal)
            || name_lower.contains("apple")
            || name_lower.contains("m1")
            || name_lower.contains("m2")
            || name_lower.contains("m3")
            || name_lower.contains("m4")
        {
            return GpuVendor::Apple;
        }

        // Fallback to string matching on name and driver
        if name_lower.contains("nvidia")
            || name_lower.contains("geforce")
            || name_lower.contains("quadro")
            || name_lower.contains("tesla")
        {
            GpuVendor::Nvidia
        } else if name_lower.contains("amd")
            || name_lower.contains("radeon")
            || name_lower.contains("rx ")
            || driver_lower.contains("amd")
        {
            GpuVendor::Amd
        } else if name_lower.contains("intel")
            || name_lower.contains("iris")
            || name_lower.contains("uhd")
            || name_lower.contains("hd graphics")
        {
            GpuVendor::Intel
        } else {
            GpuVendor::Unknown
        }
    }

    fn preferred_warp_size(&self) -> u32 {
        match self {
            GpuVendor::Nvidia => 32,  // CUDA warp size
            GpuVendor::Amd => 64,     // AMD wavefront size (RDNA/RDNA2/RDNA3)
            GpuVendor::Intel => 32,   // Intel SIMD width
            GpuVendor::Apple => 32,   // Apple GPU SIMD width (similar to mobile GPUs)
            GpuVendor::Unknown => 32, // Safe default
        }
    }

    fn prefers_large_workgroups(&self) -> bool {
        match self {
            GpuVendor::Nvidia => true, // NVIDIA generally benefits from larger workgroups
            GpuVendor::Amd => true,    // AMD also generally benefits from larger workgroups
            GpuVendor::Intel => false, // Intel integrated graphics prefer smaller workgroups
            GpuVendor::Apple => false, // Apple GPUs prefer smaller workgroups due to unified memory architecture
            GpuVendor::Unknown => false, // Conservative default
        }
    }
}

impl WorkgroupConfig {
    /// Create optimized workgroup configuration based on device capabilities and adapter info
    pub fn new(device: &Device, adapter_info: &AdapterInfo) -> Self {
        let limits = device.limits();
        let vendor = GpuVendor::detect(adapter_info);

        debug!("Detected GPU vendor: {:?}", vendor);
        debug!("GPU Name: {}", adapter_info.name);
        debug!("GPU Vendor: {}", adapter_info.vendor);
        debug!("GPU Device: {}", adapter_info.device);
        debug!("GPU Device Type: {:?}", adapter_info.device_type);
        debug!("GPU Backend: {:?}", adapter_info.backend);
        debug!("GPU Driver: {}", adapter_info.driver);
        debug!("GPU Limits:");
        debug!(
            "  Max workgroup size X: {}",
            limits.max_compute_workgroup_size_x
        );
        debug!(
            "  Max workgroup size Y: {}",
            limits.max_compute_workgroup_size_y
        );
        debug!(
            "  Max workgroup size Z: {}",
            limits.max_compute_workgroup_size_z
        );
        debug!(
            "  Max workgroup invocations: {}",
            limits.max_compute_invocations_per_workgroup
        );
        debug!(
            "  Max storage buffer size: {}",
            limits.max_storage_buffer_binding_size
        );
        debug!(
            "  Max uniform buffer size: {}",
            limits.max_uniform_buffer_binding_size
        );

        // Determine optimal 1D workgroup size
        let compute_1d = Self::optimize_1d_workgroup(&limits, vendor);

        // Determine optimal 2D workgroup size
        let compute_2d = Self::optimize_2d_workgroup(&limits, vendor);

        debug!("Optimized workgroup sizes:");
        debug!("  1D compute: {}", compute_1d);
        debug!("  2D compute: {}x{}", compute_2d.0, compute_2d.1);

        Self {
            compute_1d,
            compute_2d,
        }
    }

    /// Optimize workgroup size for 1D operations (agent updates, trail decay/diffusion)
    fn optimize_1d_workgroup(limits: &Limits, vendor: GpuVendor) -> u32 {
        let warp_size = vendor.preferred_warp_size();
        let prefers_large = vendor.prefers_large_workgroups();

        // Generate candidate sizes based on GPU characteristics
        let mut candidate_sizes = Vec::new();

        if prefers_large {
            // For GPUs that prefer larger workgroups (NVIDIA, AMD)
            candidate_sizes.extend([1024, 512, 256, 128, 64]);
        } else {
            // For GPUs that prefer smaller workgroups (Intel, Apple)
            candidate_sizes.extend([256, 128, 64, 32]);
        }

        // Ensure all candidates are multiples of warp size
        candidate_sizes.retain(|&size| size % warp_size == 0);

        // Find the largest size that fits within device limits
        for &size in &candidate_sizes {
            if size <= limits.max_compute_workgroup_size_x
                && size <= limits.max_compute_invocations_per_workgroup
            {
                return size;
            }
        }

        // Fallback: find largest multiple of warp size that fits
        let max_size = limits
            .max_compute_workgroup_size_x
            .min(limits.max_compute_invocations_per_workgroup);
        (max_size / warp_size) * warp_size
    }

    /// Optimize workgroup size for 2D operations (display texture generation)
    fn optimize_2d_workgroup(limits: &Limits, vendor: GpuVendor) -> (u32, u32) {
        let prefers_large = vendor.prefers_large_workgroups();

        // Generate candidate sizes based on GPU characteristics
        let candidates = if prefers_large {
            // For GPUs that prefer larger workgroups
            vec![
                (32, 32),
                (32, 16),
                (16, 32),
                (16, 16),
                (16, 8),
                (8, 16),
                (8, 8),
            ]
        } else {
            // For GPUs that prefer smaller workgroups
            vec![(16, 16), (16, 8), (8, 16), (8, 8), (8, 4), (4, 8)]
        };

        // Find the largest 2D workgroup that fits within device limits
        for &(x, y) in &candidates {
            let total_invocations = x * y;

            if x <= limits.max_compute_workgroup_size_x
                && y <= limits.max_compute_workgroup_size_y
                && total_invocations <= limits.max_compute_invocations_per_workgroup
            {
                return (x, y);
            }
        }

        // Fallback: calculate optimal square workgroup size
        let max_invocations = limits.max_compute_invocations_per_workgroup;
        let max_x = limits.max_compute_workgroup_size_x;
        let max_y = limits.max_compute_workgroup_size_y;

        // Find largest square size that fits
        let max_square = (max_invocations as f32).sqrt() as u32;
        let size = max_square.min(max_x).min(max_y);

        // Ensure it's a power of 2 for better performance
        let size = if size >= 16 {
            16
        } else if size >= 8 {
            8
        } else if size >= 4 {
            4
        } else {
            2
        };

        (size, size)
    }

    /// Calculate number of workgroups needed for 1D dispatch (agents)
    pub fn workgroups_1d(&self, count: u32) -> u32 {
        count.div_ceil(self.compute_1d).min(65535)
    }

    /// Calculate number of workgroups needed for 2D dispatch  
    pub fn workgroups_2d(&self, width: u32, height: u32) -> (u32, u32) {
        let x_groups = width.div_ceil(self.compute_2d.0).min(65535);
        let y_groups = height.div_ceil(self.compute_2d.1).min(65535);
        (x_groups, y_groups)
    }
}
