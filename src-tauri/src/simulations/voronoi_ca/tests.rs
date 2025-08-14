//! Voronoi CA shader validation tests

use super::shaders::{
    COMPUTE_SHADER, COMPUTE_UPDATE_SHADER, GRID_CLEAR_SHADER, GRID_POPULATE_SHADER,
    JFA_SEED_CLEAR_SHADER, JFA_SEED_POPULATE_SHADER, JFA_STEP_SHADER, VORONOI_RENDER_SHADER,
};

struct VcaValidator {
    device: wgpu::Device,
}

impl VcaValidator {
    async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: Default::default(),
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("device");

        Self { device }
    }

    fn compile_render(&self) {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA Render Shader"),
                source: wgpu::ShaderSource::Wgsl(VORONOI_RENDER_SHADER.into()),
            });
    }

    fn compile_compute(&self) {
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
            });
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA Compute Update Shader"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_UPDATE_SHADER.into()),
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
                label: Some("VCA JFA Seed Clear Shader"),
                source: wgpu::ShaderSource::Wgsl(JFA_SEED_CLEAR_SHADER.into()),
            });
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA JFA Seed Populate Shader"),
                source: wgpu::ShaderSource::Wgsl(JFA_SEED_POPULATE_SHADER.into()),
            });
        let _ = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("VCA JFA Step Shader"),
                source: wgpu::ShaderSource::Wgsl(JFA_STEP_SHADER.into()),
            });
    }
}

#[tokio::test]
async fn test_vca_shader_compilation() {
    let v = VcaValidator::new().await;
    v.compile_render();
    v.compile_compute();
    v.compile_grid();
    v.compile_jfa();
}
