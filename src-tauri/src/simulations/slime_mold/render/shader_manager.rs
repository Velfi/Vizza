use crate::simulations::slime_mold::workgroup_optimizer::WorkgroupConfig;
use std::borrow::Cow;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};
use crate::simulations::slime_mold::shaders::{COMPUTE_SHADER, DISPLAY_SHADER, QUAD_SHADER, QUAD_3X3_SHADER, GRADIENT_SHADER};

pub struct ShaderManager {
    pub compute_shader: ShaderModule,
    pub display_shader: ShaderModule,
    pub quad_shader: ShaderModule,
    pub quad_3x3_shader: ShaderModule,
    pub gradient_shader: ShaderModule,
}

impl ShaderManager {
    pub fn new(device: &Device, workgroup_config: &WorkgroupConfig) -> Self {
        Self {
            compute_shader: Self::create_compute_shader(
                device,
                "Compute Shader",
                COMPUTE_SHADER,
                workgroup_config.compute_1d,
                workgroup_config.compute_2d,
            ),
            display_shader: Self::create_display_shader(
                device,
                "Display Compute Shader",
                DISPLAY_SHADER,
                workgroup_config.compute_2d,
            ),
            quad_shader: Self::create_shader(
                device,
                "Quad Shader",
                QUAD_SHADER,
            ),
            quad_3x3_shader: Self::create_shader(
                device,
                "Quad 3x3 Shader",
                QUAD_3X3_SHADER,
            ),
            gradient_shader: Self::create_compute_shader(
                device,
                "Gradient Shader",
                GRADIENT_SHADER,
                workgroup_config.compute_1d,
                workgroup_config.compute_2d,
            ),
        }
    }

    fn create_shader(device: &Device, label: &str, source: &str) -> ShaderModule {
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Borrowed(source)),
        })
    }

    fn create_compute_shader(
        device: &Device,
        label: &str,
        source: &str,
        workgroup_size_1d: u32,
        workgroup_size_2d: (u32, u32),
    ) -> ShaderModule {
        let mut modified_source = source.replace(
            "@workgroup_size(256)",
            &format!("@workgroup_size({})", workgroup_size_1d),
        );

        // Also replace 2D workgroup sizes for functions that need them
        modified_source = modified_source.replace(
            "@workgroup_size(16, 16, 1)",
            &format!(
                "@workgroup_size({}, {}, 1)",
                workgroup_size_2d.0, workgroup_size_2d.1
            ),
        );

        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Owned(modified_source)),
        })
    }

    fn create_display_shader(
        device: &Device,
        label: &str,
        source: &str,
        workgroup_size: (u32, u32),
    ) -> ShaderModule {
        let modified_source = source.replace(
            "@workgroup_size(16, 16)",
            &format!(
                "@workgroup_size({}, {})",
                workgroup_size.0, workgroup_size.1
            ),
        );

        device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: ShaderSource::Wgsl(Cow::Owned(modified_source)),
        })
    }
}
