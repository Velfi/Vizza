use std::collections::HashMap;
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ColorTargetState,
    ComputePipeline, ComputePipelineDescriptor, Device, FragmentState, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, TextureView, VertexState,
};

/// Manages shader modules with caching to avoid duplicate compilation
#[derive(Debug)]
pub struct ShaderManager {
    shaders: HashMap<String, Arc<ShaderModule>>,
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderManager {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    /// Load a shader module, reusing cached version if available
    pub fn load_shader(&mut self, device: &Device, name: &str, source: &str) -> Arc<ShaderModule> {
        self.shaders
            .entry(name.to_string())
            .or_insert_with(|| {
                Arc::new(device.create_shader_module(ShaderModuleDescriptor {
                    label: Some(name),
                    source: ShaderSource::Wgsl(source.into()),
                }))
            })
            .clone()
    }

    /// Get a cached shader module
    pub fn get_shader(&self, name: &str) -> Option<Arc<ShaderModule>> {
        self.shaders.get(name).cloned()
    }
}

/// Builder for creating render pipelines with common configurations
pub struct RenderPipelineBuilder {
    device: Arc<Device>,
    shader: Option<Arc<ShaderModule>>,
    bind_group_layouts: Vec<BindGroupLayout>,
    vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    primitive: PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    fragment_targets: Vec<Option<ColorTargetState>>,
    label: Option<String>,
}

impl RenderPipelineBuilder {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            shader: None,
            bind_group_layouts: Vec::new(),
            vertex_buffer_layouts: Vec::new(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment_targets: Vec::new(),
            label: None,
        }
    }

    pub fn with_shader(mut self, shader: Arc<ShaderModule>) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn with_bind_group_layouts(mut self, layouts: Vec<BindGroupLayout>) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    pub fn with_vertex_buffer_layouts(
        mut self,
        layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    ) -> Self {
        self.vertex_buffer_layouts = layouts;
        self
    }

    pub fn with_primitive(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }

    pub fn with_fragment_targets(mut self, targets: Vec<Option<ColorTargetState>>) -> Self {
        self.fragment_targets = targets;
        self
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn build(self) -> RenderPipeline {
        let layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: self.label.as_deref(),
                bind_group_layouts: &self.bind_group_layouts.iter().collect::<Vec<_>>(),
                push_constant_ranges: &[],
            });

        let shader = self.shader.expect("Shader not set");

        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: self.label.as_deref(),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &self.vertex_buffer_layouts,
                    compilation_options: Default::default(),
                },
                fragment: if self.fragment_targets.is_empty() {
                    None
                } else {
                    Some(FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &self.fragment_targets,
                        compilation_options: Default::default(),
                    })
                },
                primitive: self.primitive,
                depth_stencil: self.depth_stencil,
                multisample: self.multisample,
                multiview: None,
                cache: None,
            })
    }
}

/// Builder for creating compute pipelines
pub struct ComputePipelineBuilder {
    device: Arc<Device>,
    shader: Option<Arc<ShaderModule>>,
    bind_group_layouts: Vec<BindGroupLayout>,
    label: Option<String>,
}

impl ComputePipelineBuilder {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            shader: None,
            bind_group_layouts: Vec::new(),
            label: None,
        }
    }

    pub fn with_shader(mut self, shader: Arc<ShaderModule>) -> Self {
        self.shader = Some(shader);
        self
    }

    pub fn with_bind_group_layouts(mut self, layouts: Vec<BindGroupLayout>) -> Self {
        self.bind_group_layouts = layouts;
        self
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn build(self) -> ComputePipeline {
        let layout = self
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: self.label.as_deref(),
                bind_group_layouts: &self.bind_group_layouts.iter().collect::<Vec<_>>(),
                push_constant_ranges: &[],
            });

        let shader = self.shader.expect("Shader not set");

        self.device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: self.label.as_deref(),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
    }
}

/// Builder for creating bind groups
pub struct BindGroupBuilder<'a> {
    device: &'a Device,
    layout: &'a BindGroupLayout,
    entries: Vec<BindGroupEntry<'a>>,
    label: Option<String>,
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new(device: &'a Device, layout: &'a BindGroupLayout) -> Self {
        Self {
            device,
            layout,
            entries: Vec::new(),
            label: None,
        }
    }

    pub fn add_buffer(mut self, binding: u32, buffer: &'a Buffer) -> Self {
        self.entries.push(BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: None,
            }),
        });
        self
    }

    pub fn add_texture_view(mut self, binding: u32, view: &'a TextureView) -> Self {
        self.entries.push(BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
        self
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn build(self) -> BindGroup {
        self.device.create_bind_group(&BindGroupDescriptor {
            label: self.label.as_deref(),
            layout: self.layout,
            entries: &self.entries,
        })
    }
}

/// Common bind group layout templates
#[derive(Debug)]
pub struct CommonBindGroupLayouts {
    pub camera: BindGroupLayout,
    pub lut: BindGroupLayout,
    pub texture_sampler: BindGroupLayout,
    pub uniform_buffer: BindGroupLayout,
}

impl CommonBindGroupLayouts {
    pub fn new(device: &Device) -> Self {
        Self {
            camera: Self::create_camera_layout(device),
            lut: Self::create_lut_layout(device),
            texture_sampler: Self::create_texture_sampler_layout(device),
            uniform_buffer: Self::create_uniform_buffer_layout(device),
        }
    }

    fn create_camera_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        })
    }

    fn create_lut_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("lut_bind_group_layout"),
        })
    }

    fn create_texture_sampler_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_sampler_bind_group_layout"),
        })
    }

    fn create_uniform_buffer_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_buffer_bind_group_layout"),
        })
    }
}

/// Helper functions for creating common WGPU resources
pub mod resource_helpers {
    use super::*;
    use wgpu::util::DeviceExt;
    use wgpu::{
        AddressMode, BufferUsages, Extent3d, FilterMode, Sampler, SamplerDescriptor, Texture,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    };

    /// Create a storage buffer with initial data
    pub fn create_storage_buffer_with_data<T: bytemuck::Pod>(
        device: &Device,
        label: &str,
        data: &[T],
    ) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        })
    }

    /// Create an empty storage buffer
    pub fn create_storage_buffer(
        device: &Device,
        label: &str,
        size: u64,
        mapped_at_creation: bool,
    ) -> Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation,
        })
    }

    /// Create a uniform buffer with initial data
    pub fn create_uniform_buffer_with_data<T: bytemuck::Pod>(
        device: &Device,
        label: &str,
        data: &[T],
    ) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }

    /// Create a 2D storage texture
    pub fn create_storage_texture(
        device: &Device,
        label: &str,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    /// Create a linear sampler with filtering
    pub fn create_linear_sampler(device: &Device, label: &str, filter_mode: FilterMode) -> Sampler {
        device.create_sampler(&SamplerDescriptor {
            label: Some(label),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: filter_mode,
            ..Default::default()
        })
    }

    /// Create a bind group with buffer entries
    pub fn create_buffer_bind_group(
        device: &Device,
        layout: &BindGroupLayout,
        label: &str,
        buffers: &[&Buffer],
    ) -> BindGroup {
        let entries: Vec<BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| BindGroupEntry {
                binding: i as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();

        device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &entries,
        })
    }

    /// Create a storage buffer layout entry
    pub fn storage_buffer_entry(
        binding: u32,
        visibility: ShaderStages,
        read_only: bool,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// Create a uniform buffer layout entry
    pub fn uniform_buffer_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// Create a storage texture layout entry
    pub fn storage_texture_entry(
        binding: u32,
        visibility: ShaderStages,
        access: wgpu::StorageTextureAccess,
        format: TextureFormat,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::StorageTexture {
                access,
                format,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }
    }

    /// Create a texture layout entry
    pub fn texture_entry(
        binding: u32,
        visibility: ShaderStages,
        sample_type: wgpu::TextureSampleType,
        view_dimension: wgpu::TextureViewDimension,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled: false,
            },
            count: None,
        }
    }

    /// Create a sampler layout entry
    pub fn sampler_entry(
        binding: u32,
        visibility: ShaderStages,
        binding_type: wgpu::SamplerBindingType,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Sampler(binding_type),
            count: None,
        }
    }

    /// Create a buffer bind group entry
    pub fn buffer_entry(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        }
    }

    /// Create a texture view bind group entry
    pub fn texture_view_entry(binding: u32, view: &TextureView) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        }
    }

    /// Create a sampler bind group entry
    pub fn sampler_bind_entry(binding: u32, sampler: &Sampler) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        }
    }
}
