use wgpu::{BindGroup, Device, Texture, TextureView};

/// Manages a pair of textures for ping-pong operations
///
/// This struct eliminates the possibility of texture mixups by providing
/// a clean interface for alternating between two textures during compute operations.
/// It automatically tracks which texture is currently active and provides
/// methods to swap them safely.
#[derive(Debug)]
pub struct PingPongTextures {
    /// The two textures for ping-pong operations
    textures: [Texture; 2],
    /// The two texture views for ping-pong operations
    views: [TextureView; 2],
    /// Index of the currently active texture (0 or 1)
    current: usize,
}

impl PingPongTextures {
    /// Create a new ping-pong texture pair
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        label: &str,
    ) -> Self {
        let create_texture = |index: usize| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("{} {}", label, if index == 0 { "A" } else { "B" })),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };

        let texture_a = create_texture(0);
        let texture_b = create_texture(1);

        let view_a = texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = texture_b.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            textures: [texture_a, texture_b],
            views: [view_a, view_b],
            current: 0,
        }
    }

    /// Get the currently active texture view
    pub fn current_view(&self) -> &TextureView {
        &self.views[self.current]
    }

    /// Get the currently inactive texture view (for writing)
    pub fn inactive_view(&self) -> &TextureView {
        &self.views[1 - self.current]
    }

    /// Swap the active and inactive textures
    pub fn swap(&mut self) {
        self.current = 1 - self.current;
    }

    /// Get the current texture index (0 or 1)
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Get the appropriate bind group based on current texture state
    /// This eliminates the need for manual texture index checking in the calling code
    pub fn get_bind_group<'a>(&self, bg_a: &'a BindGroup, bg_b: &'a BindGroup) -> &'a BindGroup {
        if self.current_index() == 0 {
            bg_a
        } else {
            bg_b
        }
    }

    /// Get the currently active texture
    pub fn current_texture(&self) -> &Texture {
        &self.textures[self.current]
    }

    /// Get both textures (useful for initialization)
    pub fn textures(&self) -> &[Texture; 2] {
        &self.textures
    }

    /// Get both texture views (useful for bind group creation)
    pub fn views(&self) -> &[TextureView; 2] {
        &self.views
    }
}
