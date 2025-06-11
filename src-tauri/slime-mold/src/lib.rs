pub mod app;
pub mod buffer_pool;
pub mod egui_tools;
pub mod frame_pacing;
pub mod gradient_editor;
pub mod gpu_state;
pub mod lut_manager;
pub mod presets;
pub mod render;
pub mod settings;
pub mod simulation;
pub mod workgroup_optimizer;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn wasm_main() -> Result<(), JsValue> {
    // Initialize logging for WebAssembly
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Create event loop
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Create app
    let mut app = app::App::new();

    // Run the app
    event_loop
        .run_app(&mut app)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
