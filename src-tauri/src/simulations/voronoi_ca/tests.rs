//! Voronoi CA shader validation tests

use super::shaders::{
    GRID_CLEAR_SHADER, GRID_POPULATE_SHADER,
    JFA_INIT_SHADER, JFA_ITERATION_SHADER, VORONOI_RENDER_JFA_SHADER,
};

struct VcaValidator {
    device: wgpu::Device,
}

impl VcaValidator {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("No adapter");

        let (device, _queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                ..Default::default()
            })
            .await
            .expect("device");

        Self { device }
    }

    fn compile_render(&self) {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA JFA Render Shader"),
                source: wgpu::ShaderSource::Wgsl(VORONOI_RENDER_JFA_SHADER.into()),
            });
    }

    fn compile_grid(&self) {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA Grid Clear Shader"),
                source: wgpu::ShaderSource::Wgsl(GRID_CLEAR_SHADER.into()),
            });
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA Grid Populate Shader"),
                source: wgpu::ShaderSource::Wgsl(GRID_POPULATE_SHADER.into()),
            });
    }

    fn compile_jfa(&self) {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA JFA Init Shader"),
                source: wgpu::ShaderSource::Wgsl(JFA_INIT_SHADER.into()),
            });
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA JFA Iteration Shader"),
                source: wgpu::ShaderSource::Wgsl(JFA_ITERATION_SHADER.into()),
            });
    }

    fn compile_jfa_render(&self) {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA JFA Render Shader"),
                source: wgpu::ShaderSource::Wgsl(VORONOI_RENDER_JFA_SHADER.into()),
            });
    }
}

#[tokio::test]
async fn test_vca_shader_compilation() {
    let v = VcaValidator::new().await;
    v.compile_render();
    v.compile_grid();
}

#[tokio::test]
async fn test_vca_jfa_shader_compilation() {
    let v = VcaValidator::new().await;
    v.compile_jfa();
    v.compile_jfa_render();
}

#[tokio::test]
async fn test_all_vca_shaders_compilation() {
    let v = VcaValidator::new().await;
    v.compile_render();
    v.compile_grid();
    v.compile_jfa();
    v.compile_jfa_render();
}
