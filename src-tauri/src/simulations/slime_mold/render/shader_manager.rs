use crate::simulations::shared::AVERAGE_COLOR_SHADER;
use crate::simulations::slime_mold::shaders::{
    BACKGROUND_RENDER_SHADER, COMPUTE_SHADER, DISPLAY_SHADER, GRADIENT_SHADER,
    QUAD_INFINITE_SHADER, QUAD_SHADER,
};
use crate::simulations::slime_mold::workgroup_optimizer::WorkgroupConfig;
use std::borrow::Cow;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

pub struct ShaderManager {
    pub compute_shader: ShaderModule,
    pub display_shader: ShaderModule,
    pub quad_shader: ShaderModule,
    pub quad_infinite_shader: ShaderModule,
    pub gradient_shader: ShaderModule,
    pub background_render_shader: ShaderModule,
    pub average_color_shader: ShaderModule,
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
            quad_shader: Self::create_shader(device, "Quad Shader", QUAD_SHADER),
            quad_infinite_shader: Self::create_shader(
                device,
                "Quad Infinite Shader",
                QUAD_INFINITE_SHADER,
            ),
            gradient_shader: Self::create_compute_shader(
                device,
                "Gradient Shader",
                GRADIENT_SHADER,
                workgroup_config.compute_1d,
                workgroup_config.compute_2d,
            ),
            background_render_shader: Self::create_shader(
                device,
                "Background Render Shader",
                BACKGROUND_RENDER_SHADER,
            ),
            average_color_shader: Self::create_compute_shader(
                device,
                "Average Color Shader",
                AVERAGE_COLOR_SHADER,
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
