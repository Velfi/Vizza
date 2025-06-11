use std::borrow::Cow;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};
use crate::workgroup_optimizer::WorkgroupConfig;

pub struct ShaderManager {
    pub compute_shader: ShaderModule,
    pub display_shader: ShaderModule,
    pub gradient_shader: ShaderModule,
    pub quad_shader: ShaderModule,
}

impl ShaderManager {
    pub fn new(device: &Device, workgroup_config: &WorkgroupConfig) -> Self {
        Self {
            compute_shader: Self::create_compute_shader(
                device,
                "Compute Shader",
                include_str!("../shaders/compute.wgsl"),
                workgroup_config.compute_1d,
            ),
            display_shader: Self::create_display_shader(
                device,
                "Display Compute Shader",
                include_str!("../shaders/display.wgsl"),
                workgroup_config.compute_2d,
            ),
            gradient_shader: Self::create_gradient_shader(
                device,
                "Gradient Compute Shader",
                include_str!("../shaders/gradient.wgsl"),
                workgroup_config.gradient,
            ),
            quad_shader: Self::create_shader(
                device,
                "Quad Shader",
                include_str!("../shaders/quad.wgsl"),
            ),
        }
    }

    fn create_shader(device: &Device, label: &str, source: &str) -> ShaderModule {
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Borrowed(source)),
        })
    }

    fn create_compute_shader(device: &Device, label: &str, source: &str, workgroup_size: u32) -> ShaderModule {
        let modified_source = source.replace("@workgroup_size(256)", &format!("@workgroup_size({})", workgroup_size));
        
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Owned(modified_source)),
        })
    }

    fn create_display_shader(device: &Device, label: &str, source: &str, workgroup_size: (u32, u32)) -> ShaderModule {
        let modified_source = source.replace(
            "@workgroup_size(16, 16)", 
            &format!("@workgroup_size({}, {})", workgroup_size.0, workgroup_size.1)
        );
        
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Owned(modified_source)),
        })
    }

    fn create_gradient_shader(device: &Device, label: &str, source: &str, workgroup_size: u32) -> ShaderModule {
        let modified_source = source.replace("@workgroup_size(256)", &format!("@workgroup_size({})", workgroup_size));
        
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Owned(modified_source)),
        })
    }
}
