use std::sync::Arc;

use serde_json::Value;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

use crate::error::SimulationResult;
use crate::simulations::shared::camera::Camera;
use crate::simulations::shared::LutManager;
use crate::simulations::traits::Simulation;

use super::settings::Settings;

pub struct VoronoiCAModel {
    pub settings: Settings,
    pub width: u32,
    pub height: u32,
    pub camera: Camera,
    pub show_gui: bool,
    // GPU resources would go here (grids, pipelines, etc.)
}

impl VoronoiCAModel {
    pub fn new(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        surface_config: &SurfaceConfiguration,
        settings: Settings,
        _lut_manager: &LutManager,
        _app_settings: &crate::commands::AppSettings,
    ) -> SimulationResult<Self> {
        let camera = Camera::new(device, surface_config.width as f32, surface_config.height as f32)?;
        let mut model = Self {
            settings,
            width: surface_config.width,
            height: surface_config.height,
            camera,
            show_gui: false,
        };

        // TODO: build pipelines and buffers
        let _ = (device, queue);

        Ok(model)
    }

    fn write_settings_to_gpu(&self, _queue: &Arc<Queue>) {
        // TODO: upload uniforms/buffers
    }
}

impl Simulation for VoronoiCAModel {
    fn render_frame(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        _surface_view: &TextureView,
        _delta_time: f32,
    ) -> SimulationResult<()> {
        // TODO: dispatch compute for steps_per_frame and render
        Ok(())
    }

    fn render_frame_static(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        _surface_view: &TextureView,
    ) -> SimulationResult<()> {
        // Render without stepping
        Ok(())
    }

    fn resize(
        &mut self,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
        new_config: &SurfaceConfiguration,
    ) -> SimulationResult<()> {
        self.width = new_config.width;
        self.height = new_config.height;
        self.camera.resize(new_config.width as f32, new_config.height as f32);
        Ok(())
    }

    fn update_setting(
        &mut self,
        setting_name: &str,
        value: Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        match setting_name {
            "rulestring" => {
                if let Some(s) = value.as_str() {
                    self.settings.rulestring = s.to_string();
                }
            }
            "timestep" => {
                if let Some(v) = value.as_f64() {
                    self.settings.timestep = v as f32;
                }
            }
            "steps_per_frame" => {
                if let Some(v) = value.as_u64() {
                    self.settings.steps_per_frame = v as u32;
                }
            }
            "brush_radius" | "cursor_size" => {
                if let Some(v) = value.as_f64() {
                    self.settings.brush_radius = v as f32;
                }
            }
            "brush_strength" | "cursor_strength" => {
                if let Some(v) = value.as_f64() {
                    self.settings.brush_strength = v as f32;
                }
            }
            "auto_reseed_enabled" => {
                if let Some(v) = value.as_bool() {
                    self.settings.auto_reseed_enabled = v;
                }
            }
            "auto_reseed_interval_secs" => {
                if let Some(v) = value.as_f64() {
                    self.settings.auto_reseed_interval_secs = v as f32;
                }
            }
            "random_seed" => {
                if let Some(v) = value.as_u64() {
                    self.settings.random_seed = v as u32;
                }
            }
            "lut" | "currentLut" => {
                if let Some(s) = value.as_str() {
                    self.settings.lut_name = s.to_string();
                }
            }
            "lut_reversed" | "lutReversed" => {
                if let Some(b) = value.as_bool() {
                    self.settings.lut_reversed = b;
                }
            }
            _ => {}
        }
        self.write_settings_to_gpu(queue);
        Ok(())
    }

    fn get_settings(&self) -> Value {
        serde_json::to_value(&self.settings).unwrap_or(Value::Null)
    }

    fn get_state(&self) -> Value {
        serde_json::json!({
            "width": self.width,
            "height": self.height,
        })
    }

    fn handle_mouse_interaction(
        &mut self,
        world_x: f32,
        world_y: f32,
        mouse_button: u32,
        _device: &Arc<Device>,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        // Interpret left click as paint alive, right as erase
        let _is_paint = mouse_button == 0;
        let _is_erase = mouse_button == 2;
        let _x = world_x;
        let _y = world_y;
        // TODO: write into a brush buffer for compute shader to apply
        Ok(())
    }

    fn handle_mouse_release(
        &mut self,
        _mouse_button: u32,
        _queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        Ok(())
    }

    fn pan_camera(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.pan(delta_x, delta_y);
    }

    fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    fn zoom_camera_to_cursor(&mut self, delta: f32, cursor_x: f32, cursor_y: f32) {
        self.camera.zoom_to_cursor(delta, cursor_x, cursor_y);
    }

    fn reset_camera(&mut self) {
        self.camera.reset();
    }

    fn get_camera_state(&self) -> Value {
        self.camera.get_state()
    }

    fn save_preset(&self, _preset_name: &str) -> SimulationResult<()> {
        Ok(())
    }

    fn load_preset(&mut self, _preset_name: &str, _queue: &Arc<Queue>) -> SimulationResult<()> {
        Ok(())
    }

    fn apply_settings(
        &mut self,
        settings: serde_json::Value,
        _device: &Arc<Device>,
        queue: &Arc<Queue>,
    ) -> SimulationResult<()> {
        if let Ok(new_settings) = serde_json::from_value::<Settings>(settings) {
            self.settings = new_settings;
            self.write_settings_to_gpu(queue);
        }
        Ok(())
    }

    fn reset_runtime_state(&mut self, _device: &Arc<Device>, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // TODO: clear grids and re-seed if needed
        Ok(())
    }

    fn toggle_gui(&mut self) -> bool {
        self.show_gui = !self.show_gui;
        self.show_gui
    }

    fn is_gui_visible(&self) -> bool {
        self.show_gui
    }

    fn randomize_settings(&mut self, _device: &Arc<Device>, _queue: &Arc<Queue>) -> SimulationResult<()> {
        // Minimal randomization
        self.settings.random_seed = rand::random::<u32>();
        Ok(())
    }
}