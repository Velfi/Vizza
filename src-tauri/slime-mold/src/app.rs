use crate::frame_pacing::FramePacing;
use crate::gpu_state::GpuState;
use crate::lut_manager::LutManager;
use crate::presets::init_preset_manager;
use crate::settings::Settings;
use crate::gradient_editor::GradientEditor;
use std::collections::HashMap;
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

pub struct App {
    // GPU state (None until initialized)
    gpu_state: Option<GpuState>,

    // Window settings
    window_fullscreen: bool,
    window_width: u32,
    window_height: u32,

    // FPS settings
    fps_limit_enabled: bool,
    fps_limit: f32,

    // Simulation settings and state
    settings: Settings,
    settings_changed: bool,
    needs_gpu_update: bool,
    needs_display_update: bool,
    ui_visible: bool,
    paused: bool,
    decay_rate_hi_range: bool,

    // FPS tracking
    frame_times: Vec<Duration>,
    #[cfg(not(target_arch = "wasm32"))]
    last_frame_time: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    last_frame_time: web_time::Instant,

    // LUT management
    current_lut_index: usize,
    previous_lut_index: usize,
    lut_reversed: bool,
    lut_preview_cache: HashMap<(String, bool), Vec<egui::Color32>>,
    available_luts: Vec<String>,
    lut_manager: LutManager,

    // Presets
    preset_manager: crate::presets::PresetManager,
    preset_names: Vec<String>,
    selected_preset: String,
    new_preset_name: String,
    save_preset_dialog_open: bool,
    agent_count: usize,
    previous_agent_count: usize,

    // Frame pacing state
    frame_pacing: FramePacing,

    // Gradient editor
    gradient_editor: GradientEditor,
    custom_lut_name: String,
    show_gradient_editor: bool,

    // Add a field to track previous show_gradient_editor state
    prev_show_gradient_editor: bool,

    // Frame counter for optimization
    frame_counter: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_state.is_none() {
            let gpu_state = pollster::block_on(GpuState::new(
                event_loop,
                self.window_width,
                self.window_height,
                self.window_fullscreen,
                self.agent_count,
                &self.settings,
                &self.lut_manager,
                &self.available_luts,
                self.current_lut_index,
                self.lut_reversed,
            ));

            match gpu_state {
                Ok(state) => {
                    self.gpu_state = Some(state);
                }
                Err(e) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        // Show error message in browser
                        let window = web_sys::window().unwrap();
                        let document = window.document().unwrap();
                        let error_div = document.get_element_by_id("error-message").unwrap();
                        error_div.set_attribute("style", "display: block").unwrap();
                        let error_details =
                            error_div.query_selector(".error-details").unwrap().unwrap();
                        error_details.set_text_content(Some(&e.to_string()));
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        eprintln!("Failed to initialize GPU state: {}", e);
                    }
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(gpu_state) = &mut self.gpu_state {
            // Handle egui input
            gpu_state
                .egui_renderer
                .handle_input(&gpu_state.window, &event);

            // Handle key press for UI toggle
            if let WindowEvent::KeyboardInput {
                event: key_event, ..
            } = &event
            {
                if key_event.state.is_pressed() {
                    // Check if any text input is focused
                    let text_input_focused = if let Some(gpu_state) = &self.gpu_state {
                        gpu_state.egui_renderer.context().memory(|mem| mem.focused().is_some())
                    } else {
                        false
                    };

                    // Only handle hotkeys if no text input is focused
                    if !text_input_focused {
                        if let winit::keyboard::Key::Character(c) = &key_event.logical_key {
                            if c == "/" {
                                self.ui_visible = !self.ui_visible;
                            } else if c == "r" {
                                self.agent_count =
                                    randomize_settings(&mut self.settings, self.agent_count);

                                // Mark settings as changed
                                self.settings_changed = true;
                                self.needs_display_update = true;
                            }
                        }
                    }
                }

                // Handle alt-enter for fullscreen toggle
                #[cfg(target_os = "windows")]
                if key_event.state.is_pressed()
                    && key_event.logical_key == winit::keyboard::Key::Enter
                    && key_event.modifiers.alt_key()
                {
                    if let Some(gpu_state) = &mut self.gpu_state {
                        self.window_fullscreen = !self.window_fullscreen;
                        if self.window_fullscreen {
                            gpu_state
                                .window
                                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        } else {
                            gpu_state.window.set_fullscreen(None);
                        }
                    }
                }
            }
        }

        match event {
            WindowEvent::Resized(physical_size) => {
                if let Some(gpu_state) = &mut self.gpu_state {
                    // Update simulation size and settings
                    self.window_width = physical_size.width;
                    self.window_height = physical_size.height;

                    gpu_state.resize_buffers(self.agent_count, &self.settings);
                    gpu_state.update_settings(&self.settings);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu_state) = &self.gpu_state {
            // If FPS limiting is enabled, calculate sleep time
            if self.fps_limit_enabled {
                let elapsed = self.last_frame_time.elapsed();
                let sleep_time = self.frame_pacing.get_sleep_time(elapsed);
                if !sleep_time.is_zero() {
                    std::thread::sleep(sleep_time);
                }
            }
            gpu_state.window.request_redraw();
        }
    }
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::default();
        let preset_manager = init_preset_manager();
        let preset_names = preset_manager.get_preset_names();
        let selected_preset = "Default".to_string();

        // Initialize LUT manager and get available LUTs
        let lut_manager = LutManager::new();
        let available_luts = lut_manager.get_available_luts();
        let current_lut_index = available_luts
            .iter()
            .position(|name| name == "MATPLOTLIB_bone_r")
            .expect("MATPLOTLIB_bone_r LUT not found");

        Self {
            gpu_state: None,
            agent_count: 1_000_000,
            available_luts,
            current_lut_index,
            decay_rate_hi_range: false,
            frame_times: Vec::with_capacity(60),
            #[cfg(not(target_arch = "wasm32"))]
            last_frame_time: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            last_frame_time: web_time::Instant::now(),
            lut_manager,
            lut_preview_cache: HashMap::new(),
            lut_reversed: false,
            needs_display_update: false,
            needs_gpu_update: false,
            new_preset_name: String::new(),
            paused: false,
            preset_manager,
            preset_names,
            previous_agent_count: 1_000_000,
            previous_lut_index: current_lut_index,
            save_preset_dialog_open: false,
            selected_preset,
            settings_changed: false,
            settings: settings.clone(),
            ui_visible: true,
            // Window settings
            window_fullscreen: false,
            window_width: 1600,
            window_height: 900,
            // FPS settings
            fps_limit_enabled: false,
            fps_limit: 60.0,
            frame_pacing: FramePacing::new(60.0),
            gradient_editor: GradientEditor::new(),
            custom_lut_name: String::new(),
            show_gradient_editor: false,
            // Add a field to track previous show_gradient_editor state
            prev_show_gradient_editor: false,
            // Frame counter for optimization
            frame_counter: 0,
        }
    }

    /// Helper function to handle agent count changes
    fn handle_agent_count_change(&mut self) {
        if self.agent_count != self.previous_agent_count {
            if let Some(gpu_state) = &mut self.gpu_state {
                gpu_state.recreate_agent_buffer(self.agent_count, &self.settings);
                self.settings_changed = true;
                self.previous_agent_count = self.agent_count;
            }
        }
    }

    fn render(&mut self) {
        // Update FPS tracking
        #[cfg(not(target_arch = "wasm32"))]
        let now = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let now = web_time::Instant::now();

        // Calculate frame time including any sleep time
        let frame_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Only update FPS tracking if we're not paused
        if !self.paused {
            self.frame_times.push(frame_time);
            if self.frame_times.len() > 60 {
                self.frame_times.remove(0);
            }

            // Update frame pacing
            if self.fps_limit_enabled {
                self.frame_pacing.update(frame_time);
            }
        }

        // Calculate average FPS over the last 60 frames
        let avg_frame_time: Duration =
            self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;

        // Update window title with FPS
        if let Some(gpu_state) = &self.gpu_state {
            gpu_state.window.set_title(&format!(
                "Physarum Simulation - {:.1} FPS",
                1.0 / avg_frame_time.as_secs_f64()
            ));
        }

        // Handle agent count changes immediately to prevent buffer overruns
        self.handle_agent_count_change();

        // Speed settings changed - first update uniform buffer, then reassign agent speeds
        if let Some(gpu_state) = &self.gpu_state {
            gpu_state.update_settings(&self.settings);
            gpu_state.reassign_agent_speeds(self.agent_count);
        }

        if self.settings_changed {
            // Other settings changed - update uniform buffer
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.update_settings(&self.settings);
            }
            self.needs_display_update = true;
        }

        // Update LUT if it has changed, or if the gradient editor is open and the gradient changes
        if self.show_gradient_editor {
            if let Some(gpu_state) = &mut self.gpu_state {
                // Generate LUT from the gradient editor
                let lut_vec = self.gradient_editor.generate_lut();
                // Convert to LutData
                let lut_data = crate::lut_manager::LutData {
                    name: "gradient_editor_preview".to_string(),
                    red: lut_vec[0..256].to_vec(),
                    green: lut_vec[256..512].to_vec(),
                    blue: lut_vec[512..768].to_vec(),
                };
                gpu_state.update_lut(&lut_data);
                // No need to update previous_lut_index here
            }
        } else if self.current_lut_index != self.previous_lut_index {
            if let Some(gpu_state) = &mut self.gpu_state {
                if let Ok(mut new_lut_data) = self
                    .lut_manager
                    .load_lut(&self.available_luts[self.current_lut_index])
                {
                    if self.lut_reversed {
                        new_lut_data.reverse();
                    }
                    gpu_state.update_lut(&new_lut_data);
                    self.previous_lut_index = self.current_lut_index;
                }
            }
        }

        // Full rendering with simulation and UI
        if let Some(gpu_state) = &mut self.gpu_state {
            if let Ok(frame) = gpu_state.get_current_texture() {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = gpu_state.create_command_encoder();

                // Run compute passes for simulation
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Simulation Compute Pass"),
                        timestamp_writes: None,
                    });

                    // Generate gradient (always update when settings change or at startup)
                    if self.settings.gradient_type != crate::settings::GradientType::Disabled
                        && (self.settings_changed
                            || self.needs_gpu_update
                            || self.needs_display_update)
                    {
                        cpass.set_pipeline(&gpu_state.pipeline_manager().gradient_pipeline);
                        cpass.set_bind_group(
                            0,
                            &gpu_state.bind_group_manager().gradient_bind_group,
                            &[],
                        );
                        cpass.dispatch_workgroups(
                            gpu_state.workgroup_config().workgroups_gradient(
                                gpu_state.config().width * gpu_state.config().height
                            ),
                            1,
                            1,
                        );
                    }

                    // Only run simulation updates when not paused
                    if !self.paused {
                        self.frame_counter += 1;
                        
                        // Update agent positions (always every frame)
                        cpass.set_pipeline(&gpu_state.pipeline_manager().compute_pipeline);
                        cpass.set_bind_group(
                            0,
                            &gpu_state.bind_group_manager().compute_bind_group,
                            &[],
                        );
                        cpass.dispatch_workgroups(
                            gpu_state.workgroup_config().workgroups_1d(
                                self.agent_count as u32
                            ),
                            1,
                            1,
                        );

                        // Decay trail map (respect frequency setting)
                        if self.frame_counter % self.settings.decay_frequency == 0 {
                            cpass.set_pipeline(&gpu_state.pipeline_manager().decay_pipeline);
                            cpass.set_bind_group(
                                0,
                                &gpu_state.bind_group_manager().compute_bind_group,
                                &[],
                            );
                            cpass.dispatch_workgroups(
                                gpu_state.workgroup_config().workgroups_1d(
                                    gpu_state.config().width * gpu_state.config().height
                                ),
                                1,
                                1,
                            );
                        }

                        // Diffuse trail map (respect frequency setting)
                        if self.frame_counter % self.settings.diffusion_frequency == 0 {
                            cpass.set_pipeline(&gpu_state.pipeline_manager().diffuse_pipeline);
                            cpass.set_bind_group(
                                0,
                                &gpu_state.bind_group_manager().compute_bind_group,
                                &[],
                            );
                            cpass.dispatch_workgroups(
                                gpu_state.workgroup_config().workgroups_1d(
                                    gpu_state.config().width * gpu_state.config().height
                                ),
                                1,
                                1,
                            );
                        }
                    }

                    // Always update display (even when paused) to show current state
                    if !self.paused || self.needs_display_update {
                        cpass.set_pipeline(&gpu_state.pipeline_manager().display_pipeline);
                        cpass.set_bind_group(
                            0,
                            &gpu_state.bind_group_manager().display_bind_group,
                            &[],
                        );
                        let (x_groups, y_groups) = gpu_state.workgroup_config().workgroups_2d(
                            gpu_state.config().width, 
                            gpu_state.config().height
                        );
                        cpass.dispatch_workgroups(x_groups, y_groups, 1);
                        self.needs_display_update = false;
                    }
                }

                // Clear the framebuffer first
                {
                    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Clear Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                }

                // Begin egui frame and draw UI
                // Track changes during UI processing
                let mut agent_count_changed = false;
                let mut new_agent_count = self.agent_count;
                let mut preset_to_apply: Option<String> = None;
                let mut reset_trails_flag = false;
                let mut reset_agents_flag = false;

                {
                    gpu_state.egui_renderer.begin_frame(&gpu_state.window);

                    let full_output = gpu_state.egui_renderer.run_ui(&gpu_state.window, |ctx| {
                        if self.ui_visible {
                            egui::SidePanel::left("settings_panel")
                                .resizable(false)
                                .default_width(300.0)
                                .show(ctx, |ui| {
                                    ui.heading("Simulation Settings");
                                    
                                    // Add keyboard shortcut note
                                    ui.label(egui::RichText::new("Press / (forward slash) to show/hide this panel").italics().color(egui::Color32::GRAY));
                                    ui.label(egui::RichText::new("Hover over an option to see a tooltip explaining it.").italics().color(egui::Color32::GRAY));
                                    ui.separator();
                                    
                                    // Add FPS display at the top
                                    ui.horizontal(|ui| {
                                        ui.label("FPS:");
                                        ui.label(format!("{:.1}", 1.0 / avg_frame_time.as_secs_f64()));
                                    });
                                    
                                    // Add FPS limiter controls
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut self.fps_limit_enabled, "Limit FPS").changed() {
                                            self.settings_changed = true;
                                        }
                                        if self.fps_limit_enabled {
                                            if ui.add(egui::DragValue::new(&mut self.fps_limit)
                                                .range(1.0..=1000.0)
                                                .speed(1.0)
                                                .suffix(" FPS")
                                            ).changed() {
                                                // Update frame pacing target when FPS limit changes
                                                self.frame_pacing = FramePacing::new(self.fps_limit);
                                                self.settings_changed = true;
                                            }
                                        }
                                    });
                                    ui.separator();
                                    
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        // Presets
                                        ui.heading("Presets");
                                            ui.horizontal(|ui| {
                                                if ui.button("◀").clicked() {
                                                    let current_index = self.preset_names.iter().position(|name| name == &self.selected_preset).unwrap_or(0);
                                                    let prev_index = if current_index == 0 {
                                                        self.preset_names.len() - 1
                                                    } else {
                                                        current_index - 1
                                                    };
                                                    preset_to_apply = Some(self.preset_names[prev_index].clone());
                                                }
                                                egui::ComboBox::from_id_salt("preset_selector")
                                                    .selected_text(if self.settings_changed { "(unsaved)" } else { &self.selected_preset })
                                                    .show_ui(ui, |ui| {
                                                        for name in &self.preset_names {
                                                            if ui.selectable_label(&self.selected_preset == name, name).clicked() {
                                                                preset_to_apply = Some(name.clone());
                                                            }
                                                        }
                                                    });
                                                if ui.button("▶").clicked() {
                                                    let current_index = self.preset_names.iter().position(|name| name == &self.selected_preset).unwrap_or(0);
                                                    let next_index = (current_index + 1) % self.preset_names.len();
                                                    preset_to_apply = Some(self.preset_names[next_index].clone());
                                                }
                                            });
                                        
                                        // Save and Delete preset buttons
                                        ui.horizontal(|ui| {
                                            if ui.button("💾 Save Current").clicked() {
                                                self.save_preset_dialog_open = true;
                                                self.new_preset_name = String::new();
                                            }
                                            
                                            // Only show delete button for user presets (not built-in ones)
                                            let user_preset_names = self.preset_manager.get_user_preset_names();
                                            if user_preset_names.contains(&self.selected_preset) && ui.button("🗑 Delete").clicked() {
                                                if let Err(e) = self.preset_manager.delete_user_preset(&self.selected_preset) {
                                                    eprintln!("Failed to delete preset: {}", e);
                                                } else {
                                                    // Update the preset list after deletion
                                                    self.preset_names = self.preset_manager.get_preset_names();
                                                    
                                                    // Select default preset if current was deleted
                                                    if !self.preset_names.contains(&self.selected_preset) {
                                                        self.selected_preset = "Default".to_string();
                                                        // Apply the default preset
                                                        preset_to_apply = Some(self.selected_preset.clone());
                                            }
                                                }
                                            }
                                        });
                                        
                                        // Save preset dialog
                                        if self.save_preset_dialog_open {
                                            egui::Window::new("Save Preset")
                                                .collapsible(false)
                                                .resizable(false)
                                                .show(ctx, |ui| {
                                                    ui.label("Enter preset name:");
                                                    ui.text_edit_singleline(&mut self.new_preset_name);
                                                    ui.horizontal(|ui| {
                                                        if ui.button("Save").clicked() && !self.new_preset_name.trim().is_empty() {
                                                            if let Err(e) = self.preset_manager.save_user_preset(&self.new_preset_name, &self.settings) {
                                                                eprintln!("Failed to save preset: {}", e);
                                                            } else {
                                                                // Reload presets to include the new one
                                                                self.preset_manager = crate::presets::init_preset_manager();
                                                                self.preset_names = self.preset_manager.get_preset_names();
                                                                self.selected_preset = self.new_preset_name.clone();
                                                            }
                                                            self.save_preset_dialog_open = false;
                                                        }
                                                        if ui.button("Cancel").clicked() {
                                                            self.save_preset_dialog_open = false;
                                                        }
                                                    });
                                                });
                                        }
                                        
                                        ui.separator();

                                        // Color Scheme
                                        ui.heading("Color Scheme");
                                        ui.horizontal(|ui| {
                                            if ui.button("◀").clicked() {
                                                if self.current_lut_index > 0 {
                                                    self.current_lut_index -= 1;
                                                } else {
                                                    self.current_lut_index = self.available_luts.len() - 1;
                                                }
                                            }
                                            egui::ComboBox::from_id_salt("lut_selector")
                                                .selected_text(format!("{}{}", self.available_luts[self.current_lut_index], if self.lut_reversed { " (Reversed)" } else { "" }))
                                                .show_ui(ui, |ui| {
                                                    for (i, lut_name) in self.available_luts.iter().enumerate() {
                                                        ui.horizontal(|ui| {
                                                            // Use cache for LUT preview
                                                            let cache_key = (lut_name.clone(), self.lut_reversed);
                                                            println!("Generating preview for LUT: {}", lut_name);
                                                            let preview = self.lut_preview_cache.entry(cache_key.clone()).or_insert_with(|| {
                                                                println!("Cache miss for LUT: {}", lut_name);
                                                                if let Ok(mut lut_data) = self.lut_manager.load_lut(lut_name) {
                                                                    println!("Successfully loaded LUT data for: {}", lut_name);
                                                                    if self.lut_reversed {
                                                                        lut_data.reverse();
                                                                    }
                                                                    // Generate a Vec<egui::Color32> for the preview gradient
                                                                    (0..256).map(|idx| {
                                                                        egui::Color32::from_rgb(
                                                                            lut_data.red[idx],
                                                                            lut_data.green[idx],
                                                                            lut_data.blue[idx],
                                                                        )
                                                                    }).collect::<Vec<_>>()
                                                                } else {
                                                                    println!("Failed to load LUT data for: {}", lut_name);
                                                                    // Fallback: gray gradient
                                                                    (0..256).map(|idx| egui::Color32::from_gray(idx as u8)).collect::<Vec<_>>()
                                                                }
                                                            });
                                                            // Draw the gradient preview using the cached Vec<egui::Color32>
                                                            let rect = ui.allocate_rect(
                                                                egui::Rect::from_min_size(
                                                                    ui.min_rect().min,
                                                                    egui::vec2(50.0, ui.spacing().interact_size.y),
                                                                ),
                                                                egui::Sense::hover(),
                                                            );
                                                            let painter = ui.painter();
                                                            let rect = rect.rect;
                                                            let width = rect.width();
                                                            let steps = 50; // Number of gradient steps
                                                            let step_width = width / steps as f32;
                                                            for step in 0..steps {
                                                                let x = rect.min.x + step as f32 * step_width;
                                                                let t = step as f32 / steps as f32;
                                                                let idx = (t * 255.0) as usize;
                                                                let color = preview[idx];
                                                                painter.rect_filled(
                                                                    egui::Rect::from_min_size(
                                                                        egui::pos2(x, rect.min.y),
                                                                        egui::vec2(step_width, rect.height()),
                                                                    ),
                                                                    0.0,
                                                                    color,
                                                                );
                                                            }
                                                            ui.add_space(5.0);
                                                            // Add the LUT name
                                                            if ui.selectable_value(&mut self.current_lut_index, i, lut_name).clicked() {
                                                                ui.close_menu();
                                                            }
                                                        });
                                                    }
                                                });
                                            if ui.button("▶").clicked() {
                                                self.current_lut_index = (self.current_lut_index + 1) % self.available_luts.len();
                                            }
                                        });
                                        if ui.button("Reverse LUT").clicked() {
                                            self.lut_reversed = !self.lut_reversed;
                                            // Force LUT reload
                                            self.previous_lut_index = usize::MAX;
                                        }

                                        // Add gradient editor button
                                        if ui.button("🎨 Create Custom LUT").clicked() {
                                            self.show_gradient_editor = true;
                                        }

                                        // Show gradient editor window
                                        if self.show_gradient_editor {
                                            egui::Window::new("Gradient Editor")
                                                .collapsible(false)
                                                .resizable(true)
                                                .default_size([300.0, 150.0])
                                                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                                                .show(ctx, |ui| {
                                                    if self.gradient_editor.show(ui) {
                                                        // Update preview when gradient changes
                                                        self.needs_display_update = true;
                                                    }

                                                    ui.horizontal(|ui| {
                                                        ui.label("LUT Name:");
                                                        ui.text_edit_singleline(&mut self.custom_lut_name);
                                                    });

                                                    ui.horizontal(|ui| {
                                                        if ui.button("Save LUT").clicked() {
                                                            if !self.custom_lut_name.trim().is_empty() {
                                                                let lut_data = self.gradient_editor.generate_lut();
                                                                if let Err(e) = self.lut_manager.save_custom_lut(&self.custom_lut_name, &lut_data) {
                                                                    eprintln!("Failed to save LUT: {}", e);
                                                                } else {
                                                                    // Reload available LUTs
                                                                    self.available_luts = self.lut_manager.get_available_luts();
                                                                    // Select the new LUT
                                                                    if let Some(idx) = self.available_luts.iter().position(|name| name == &self.custom_lut_name) {
                                                                        self.current_lut_index = idx;
                                                                    }
                                                                    self.show_gradient_editor = false;
                                                                }
                                                            }
                                                        }
                                                        if ui.button("Cancel").clicked() {
                                                            self.show_gradient_editor = false;
                                                        }
                                                    });
                                                });
                                        }

                                        ui.separator();

                                        // Controls
                                        ui.heading("Controls");
                                        ui.horizontal(|ui| {
                                            // Pause/Resume button
                                            let pause_button_text = if self.paused { "▶ Resume" } else { "⏸ Pause" };
                                            if ui.button(pause_button_text).clicked() {
                                                self.paused = !self.paused;
                                            }
                                            
                                            if ui.button("Reset Trails").clicked() {
                                                reset_trails_flag = true;
                                                self.needs_display_update = true;
                                            }
                                            if ui.button("Reset Agents").clicked() {
                                                reset_agents_flag = true;
                                            }
                                        });
                                        if ui.button("🎲 Randomize Settings").clicked() {
                                            self.agent_count = randomize_settings(&mut self.settings, self.agent_count);
                                            
                                            // Mark settings as changed
                                            self.settings_changed = true;
                                            self.needs_display_update = true;
                                        }
                                        ui.separator();

                                        // Pheromone Settings
                                        ui.heading("Pheromone Settings");
                                        
                                        egui::Grid::new("pheromone_grid")
                                            .num_columns(2)
                                            .spacing([40.0, 4.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                // Decay Rate with fine controls
                                                ui.label("Decay Rate").on_hover_text("Controls how fast trails disappear. Increasing this is a great way to lighten a slime mold that's too dense. A normal value is 0.1% (1.0 internally).");
                                                ui.horizontal(|ui| {
                                                    // Convert internal value to percent for display
                                                    let mut decay_percent = self.settings.pheromone_decay_rate * 0.1; // 1.0 = 0.1%
                                                    if ui.checkbox(&mut self.decay_rate_hi_range, "Lo/Hi").changed() {
                                                        // When switching to lo range, cap at 1%
                                                        if !self.decay_rate_hi_range && decay_percent > 1.0 {
                                                            decay_percent = 1.0;
                                                            self.settings.pheromone_decay_rate = decay_percent / 0.1;
                                                        }
                                                    }
                                                    if ui.button("−").clicked() {
                                                        if self.decay_rate_hi_range {
                                                            decay_percent = (decay_percent - 0.1).max(0.0);
                                                        } else {
                                                            decay_percent = (decay_percent - 0.01).max(0.0);
                                                        }
                                                        self.settings.pheromone_decay_rate = decay_percent / 0.1;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut decay_percent)
                                                        .range(0.0..=if self.decay_rate_hi_range { 100.0 } else { 10.0 })
                                                        .speed(if self.decay_rate_hi_range { 0.1 } else { 0.01 })
                                                        .suffix("%")
                                                    ).changed() {
                                                        self.settings.pheromone_decay_rate = decay_percent / 0.1;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        if self.decay_rate_hi_range {
                                                            decay_percent = (decay_percent + 0.1).min(10.0);
                                                        } else {
                                                            decay_percent = (decay_percent + 0.01).min(1.0);
                                                        }
                                                        self.settings.pheromone_decay_rate = decay_percent / 0.1;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Deposition Rate with fine controls
                                                ui.label("Deposition Rate").on_hover_text("At 0%, agents will not deposit any pheromones. At 100%, agents will saturate their location with the maximum amount of pheromones.");
                                                ui.horizontal(|ui| {
                                                    let mut deposition_percent = self.settings.pheromone_deposition_rate * 100.0;
                                                    if ui.button("−").clicked() {
                                                        deposition_percent = (deposition_percent - 1.0).max(0.0);
                                                        self.settings.pheromone_deposition_rate = deposition_percent / 100.0;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut deposition_percent).range(0.0..=100.0).speed(1.0).suffix("%")).changed() {
                                                        self.settings.pheromone_deposition_rate = deposition_percent / 100.0;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        deposition_percent = (deposition_percent + 1.0).min(100.0);
                                                        self.settings.pheromone_deposition_rate = deposition_percent / 100.0;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Diffusion Rate with fine controls
                                                ui.label("Diffusion Rate").on_hover_text("At 0%, pheromones will stay exactly where agents deposit them. At 100%, pheromones will spread to neighboring cells and dissapate.");
                                                ui.horizontal(|ui| {
                                                    let mut diffusion_percent = self.settings.pheromone_diffusion_rate * 100.0;
                                                    if ui.button("−").clicked() {
                                                        diffusion_percent = (diffusion_percent - 1.0).max(0.0);
                                                        self.settings.pheromone_diffusion_rate = diffusion_percent / 100.0;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut diffusion_percent).range(0.0..=100.0).speed(1.0).suffix("%")).changed() {
                                                        self.settings.pheromone_diffusion_rate = diffusion_percent / 100.0;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        diffusion_percent = (diffusion_percent + 1.0).min(100.0);
                                                        self.settings.pheromone_diffusion_rate = diffusion_percent / 100.0;
                                                    }
                                                });
                                                ui.end_row();
                                            });
                                        ui.separator();

                                        // Agent Settings
                                        ui.heading("Agent Settings");
                                        
                                        egui::Grid::new("agent_grid")
                                            .num_columns(2)
                                            .spacing([40.0, 4.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                // Agent Count with buttons and number display
                                                ui.label("Agent Count").on_hover_text("Number of agents in the simulation. More agents create denser patterns but require more processing power.");
                                                ui.horizontal(|ui| {
                                                    let mut agent_count_m = (self.agent_count as f32 / 1_000_000.0).round();
                                                    if ui.button("−").clicked() {
                                                        agent_count_m = (agent_count_m - 1.0).max(0.0);
                                                        new_agent_count = (agent_count_m * 1_000_000.0) as usize;
                                                        agent_count_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut agent_count_m).range(0.0..=100.0).speed(0.1).suffix("M")).changed() {
                                                        new_agent_count = (agent_count_m * 1_000_000.0) as usize;
                                                        agent_count_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        agent_count_m = (agent_count_m + 1.0).min(100.0);
                                                        new_agent_count = (agent_count_m * 1_000_000.0) as usize;
                                                        agent_count_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Min Speed with fine controls
                                                ui.label("Min Speed").on_hover_text("Minimum speed of agents. Lower values create more detailed patterns but slower movement.");
                                                ui.horizontal(|ui| {
                                                    if ui.button("−").clicked() {
                                                        self.settings.agent_speed_min = (self.settings.agent_speed_min - 0.1).max(0.0);
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut self.settings.agent_speed_min).range(0.0..=500.0).speed(1.0)).changed() {
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        self.settings.agent_speed_min = (self.settings.agent_speed_min + 0.1).min(self.settings.agent_speed_max);
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Max Speed with fine controls
                                                ui.label("Max Speed").on_hover_text("Maximum speed of agents. Higher values create more dynamic patterns but may be less stable.");
                                                ui.horizontal(|ui| {
                                                    if ui.button("−").clicked() {
                                                        self.settings.agent_speed_max = (self.settings.agent_speed_max - 0.1).max(self.settings.agent_speed_min);
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut self.settings.agent_speed_max).range(0.0..=500.0).speed(1.0)).changed() {
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        self.settings.agent_speed_max = (self.settings.agent_speed_max + 0.1).min(500.0);
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Turn Rate with fine controls (convert radians to degrees for display)
                                                ui.label("Turn Rate (deg/s)").on_hover_text("How quickly agents can change direction. Higher values create more dynamic, less predictable patterns.");
                                                ui.horizontal(|ui| {
                                                    let mut turn_rate_degrees = self.settings.agent_turn_rate * 180.0 / std::f32::consts::PI;
                                                    if ui.button("−").clicked() {
                                                        turn_rate_degrees = (turn_rate_degrees - 1.0).max(0.0);
                                                        self.settings.agent_turn_rate = turn_rate_degrees * std::f32::consts::PI / 180.0;
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut turn_rate_degrees).range(0.0..=360.0).speed(1.0).suffix(" deg/s")).changed() {
                                                        self.settings.agent_turn_rate = turn_rate_degrees * std::f32::consts::PI / 180.0;
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        turn_rate_degrees = (turn_rate_degrees + 1.0).min(360.0);
                                                        self.settings.agent_turn_rate = turn_rate_degrees * std::f32::consts::PI / 180.0;
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Jitter with fine controls
                                                ui.label("Jitter").on_hover_text("Random movement added to agent direction. Higher values create more chaotic, less organized patterns.");
                                                ui.horizontal(|ui| {
                                                    if ui.button("−").clicked() {
                                                        self.settings.agent_jitter = (self.settings.agent_jitter - 0.01).max(0.0);
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut self.settings.agent_jitter).range(0.0..=5.0).speed(0.01)).changed() {
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        self.settings.agent_jitter = (self.settings.agent_jitter + 0.01).min(5.0);
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();

                                                // Sensor Angle with fine controls (convert radians to degrees for display)
                                                ui.label("Sensor Angle (degrees)").on_hover_text("How wide the agent's sensor field is. Wider angles create more complex, branching patterns.");
                                                ui.horizontal(|ui| {
                                                    let mut sensor_angle_degrees = self.settings.agent_sensor_angle * 180.0 / std::f32::consts::PI;
                                                    if ui.button("−").clicked() {
                                                        sensor_angle_degrees = (sensor_angle_degrees - 0.5).max(0.0);
                                                        self.settings.agent_sensor_angle = sensor_angle_degrees * std::f32::consts::PI / 180.0;
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut sensor_angle_degrees).range(0.0..=180.0).speed(0.5).suffix(" deg")).changed() {
                                                        self.settings.agent_sensor_angle = sensor_angle_degrees * std::f32::consts::PI / 180.0;
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        sensor_angle_degrees = (sensor_angle_degrees + 0.5).min(180.0);
                                                        self.settings.agent_sensor_angle = sensor_angle_degrees * std::f32::consts::PI / 180.0;
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // Sensor Distance with fine controls
                                                ui.label("Sensor Distance").on_hover_text("How far ahead agents can sense pheromones. Longer distances create more organized, network-like patterns.");
                                                ui.horizontal(|ui| {
                                                    if ui.button("−").clicked() {
                                                        self.settings.agent_sensor_distance = (self.settings.agent_sensor_distance - 1.0).max(0.0);
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut self.settings.agent_sensor_distance).range(0.0..=500.0).speed(1.0)).changed() {
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        self.settings.agent_sensor_distance = (self.settings.agent_sensor_distance + 1.0).min(500.0);
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                            });
                                        
                                        // Ensure min speed doesn't exceed max speed
                                        if self.settings.agent_speed_min > self.settings.agent_speed_max {
                                            self.settings.agent_speed_max = self.settings.agent_speed_min;
                                        }
                                        if self.settings.agent_speed_max < self.settings.agent_speed_min {
                                            self.settings.agent_speed_min = self.settings.agent_speed_max;
                                        }

                                        // Starting Direction Range
                                        ui.heading("Starting Direction Range");
                                        let mut start_angle = self.settings.agent_possible_starting_headings.start;
                                        let mut end_angle = self.settings.agent_possible_starting_headings.end;
                                        
                                        egui::Grid::new("direction_grid")
                                            .num_columns(2)
                                            .spacing([40.0, 4.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                // Start Angle with fine controls
                                                ui.label("Min Angle (degrees)");
                                                ui.horizontal(|ui| {
                                                    if ui.button("−").clicked() {
                                                        start_angle = (start_angle - 1.0).max(0.0);
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut start_angle).range(0.0..=360.0).speed(1.0)).changed() {
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        start_angle = (start_angle + 1.0).min(end_angle);
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                                
                                                // End Angle with fine controls
                                                ui.label("Max Angle (degrees)");
                                                ui.horizontal(|ui| {
                                                    if ui.button("−").clicked() {
                                                        end_angle = (end_angle - 1.0).max(start_angle);
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.add(egui::DragValue::new(&mut end_angle).range(0.0..=360.0).speed(1.0)).changed() {
                                                        self.settings_changed = true;
                                                    }
                                                    if ui.button("+").clicked() {
                                                        end_angle = (end_angle + 1.0).min(360.0);
                                                        self.settings_changed = true;
                                                    }
                                                });
                                                ui.end_row();
                                            });
                                        
                                        if start_angle != self.settings.agent_possible_starting_headings.start || end_angle != self.settings.agent_possible_starting_headings.end {
                                            self.settings.agent_possible_starting_headings = start_angle.min(end_angle)..start_angle.max(end_angle);
                                        }
                                        ui.separator();

                                        // Gradient Settings
                                        ui.heading("Gradient Settings");
                                        
                                        egui::Grid::new("gradient_grid")
                                            .num_columns(2)
                                            .spacing([40.0, 4.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                // Gradient Type
                                                ui.label("Gradient Type").on_hover_text("Different gradient patterns that influence agent movement. Each creates unique emergent behaviors.");
                                                egui::ComboBox::from_id_salt("gradient_type")
                                                    .selected_text(self.settings.gradient_type.as_str())
                                                    .show_ui(ui, |ui| {
                                                        if ui.selectable_value(&mut self.settings.gradient_type, crate::settings::GradientType::Disabled, "Disabled").changed() {
                                                            self.settings_changed = true;
                                                        }
                                                        for &gradient_type in crate::settings::GradientType::all() {
                                                            if gradient_type != crate::settings::GradientType::Disabled && ui.selectable_value(&mut self.settings.gradient_type, gradient_type, gradient_type.as_str()).changed() {
                                                                self.settings_changed = true;
                                                            }
                                                        }
                                                    });
                                                ui.end_row();
                                                
                                                if self.settings.gradient_type != crate::settings::GradientType::Disabled {
                                                    // Gradient Strength
                                                    ui.label("Strength").on_hover_text("How strongly the gradient influences agent movement. Higher values create more pronounced directional patterns.");
                                                    ui.horizontal(|ui| {
                                                        if ui.button("−").clicked() {
                                                            self.settings.gradient_strength = (self.settings.gradient_strength - 1.0).max(0.0);
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.add(egui::DragValue::new(&mut self.settings.gradient_strength).range(0.0..=100.0).speed(1.0)).changed() {
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.button("+").clicked() {
                                                            self.settings.gradient_strength = (self.settings.gradient_strength + 1.0).min(100.0);
                                                            self.settings_changed = true;
                                                        }
                                                    });
                                                    ui.end_row();

                                                    // Center X
                                                    ui.label("Center X").on_hover_text("Horizontal position of the gradient center (0-100%). Affects where the gradient pattern is centered.");
                                                    ui.horizontal(|ui| {
                                                        let mut center_x_percent = self.settings.gradient_center_x * 100.0;
                                                        if ui.button("−").clicked() {
                                                            center_x_percent = (center_x_percent - 5.0).max(0.0);
                                                            self.settings.gradient_center_x = center_x_percent / 100.0;
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.add(egui::DragValue::new(&mut center_x_percent).range(0.0..=100.0).speed(1.0).suffix("%")).changed() {
                                                            self.settings.gradient_center_x = center_x_percent / 100.0;
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.button("+").clicked() {
                                                            center_x_percent = (center_x_percent + 5.0).min(100.0);
                                                            self.settings.gradient_center_x = center_x_percent / 100.0;
                                                            self.settings_changed = true;
                                                        }
                                                    });
                                                    ui.end_row();

                                                    // Center Y
                                                    ui.label("Center Y").on_hover_text("Vertical position of the gradient center (0-100%). Affects where the gradient pattern is centered.");
                                                    ui.horizontal(|ui| {
                                                        let mut center_y_percent = self.settings.gradient_center_y * 100.0;
                                                        if ui.button("−").clicked() {
                                                            center_y_percent = (center_y_percent - 5.0).max(0.0);
                                                            self.settings.gradient_center_y = center_y_percent / 100.0;
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.add(egui::DragValue::new(&mut center_y_percent).range(0.0..=100.0).speed(1.0).suffix("%")).changed() {
                                                            self.settings.gradient_center_y = center_y_percent / 100.0;
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.button("+").clicked() {
                                                            center_y_percent = (center_y_percent + 5.0).min(100.0);
                                                            self.settings.gradient_center_y = center_y_percent / 100.0;
                                                            self.settings_changed = true;
                                                        }
                                                    });
                                                    ui.end_row();

                                                    // Size (controls scale for all gradient types)
                                                    ui.label("Size").on_hover_text("Controls the scale of the gradient pattern. Larger values create more spread-out effects.");
                                                    ui.horizontal(|ui| {
                                                        if ui.button("−").clicked() {
                                                            self.settings.gradient_size = (self.settings.gradient_size - 0.05).max(0.1);
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.add(egui::DragValue::new(&mut self.settings.gradient_size).range(0.1..=2.0).speed(0.01)).changed() {
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.button("+").clicked() {
                                                            self.settings.gradient_size = (self.settings.gradient_size + 0.05).min(2.0);
                                                            self.settings_changed = true;
                                                        }
                                                    });
                                                    ui.end_row();

                                                    // Angle (rotates all gradient types)
                                                    ui.label("Angle (degrees)").on_hover_text("Rotates the gradient pattern. Affects the direction of influence on agent movement.");
                                                    ui.horizontal(|ui| {
                                                        if ui.button("−").clicked() {
                                                            self.settings.gradient_angle = (self.settings.gradient_angle - 5.0) % 360.0;
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.add(egui::DragValue::new(&mut self.settings.gradient_angle).range(0.0..=360.0).speed(1.0)).changed() {
                                                            self.settings_changed = true;
                                                        }
                                                        if ui.button("+").clicked() {
                                                            self.settings.gradient_angle = (self.settings.gradient_angle + 5.0) % 360.0;
                                                            self.settings_changed = true;
                                                        }
                                                    });
                                                    ui.end_row();
                                                }
                                            });
                                        ui.separator();
                                    });
                                });
                        }
                    });

                    // Render simulation to screen
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Simulation Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        // Render the simulation texture
                        rpass.set_pipeline(&gpu_state.pipeline_manager().render_pipeline);
                        rpass.set_bind_group(
                            0,
                            &gpu_state.bind_group_manager().render_bind_group,
                            &[],
                        );
                        rpass.draw(0..6, 0..1);
                    }

                    // End egui frame and draw with proper blending
                    use egui_wgpu::ScreenDescriptor;
                    let screen_descriptor = ScreenDescriptor {
                        size_in_pixels: [gpu_state.config().width, gpu_state.config().height],
                        pixels_per_point: gpu_state.window.scale_factor() as f32,
                    };

                    gpu_state.egui_renderer.end_frame_and_draw(
                        &gpu_state.device,
                        &gpu_state.queue,
                        &mut encoder,
                        &gpu_state.window,
                        &view,
                        screen_descriptor,
                        full_output,
                    );
                }

                // Apply preset changes after UI processing
                if let Some(preset_name) = preset_to_apply {
                    if let Some(preset) = self.preset_manager.get_preset(&preset_name) {
                        self.settings = preset.settings.clone();
                        self.needs_gpu_update = true;
                        self.settings_changed = false;

                        // Update the uniform buffer with new settings
                        gpu_state.update_settings(&self.settings);

                        // Reset trails and agents with new settings
                        gpu_state.reset_trails();
                        gpu_state.reset_agents(&self.settings, self.agent_count);
                        self.needs_display_update = true;
                    }
                    self.selected_preset = preset_name;
                }

                // Handle agent count changes after UI processing (outside egui scope)
                if agent_count_changed {
                    self.agent_count = new_agent_count;
                    gpu_state.recreate_agent_buffer(self.agent_count, &self.settings);
                    self.settings_changed = true;
                    self.previous_agent_count = self.agent_count;
                }

                // Handle reset actions
                if reset_trails_flag {
                    gpu_state.reset_trails();
                }
                if reset_agents_flag {
                    gpu_state.reset_agents(&self.settings, self.agent_count);
                }

                // Submit the command buffer
                gpu_state.submit(encoder.finish());
                frame.present();
            }
        }

        // Reset gradient editor state if it was just closed
        if self.prev_show_gradient_editor && !self.show_gradient_editor {
            // Reload the current LUT
            if let Some(gpu_state) = &mut self.gpu_state {
                if let Ok(mut new_lut_data) = self.lut_manager.load_lut(&self.available_luts[self.current_lut_index]) {
                    if self.lut_reversed {
                        new_lut_data.reverse();
                    }
                    gpu_state.update_lut(&new_lut_data);
                }
            }
        }
        self.prev_show_gradient_editor = self.show_gradient_editor;
    }
}

// Helper function to randomize settings while preserving agent count
fn randomize_settings(settings: &mut Settings, agent_count: usize) -> usize {
    // Store current agent count
    let current_agent_count = agent_count;

    // Randomize all settings
    // Use high range for decay rate to allow for more variation
    settings.pheromone_decay_rate = rand::random::<f32>() * 10.0; // 1.0 is normal value
    settings.pheromone_deposition_rate = rand::random::<f32>() * 100.0 / 100.0; // Convert to percentage
    settings.pheromone_diffusion_rate = rand::random::<f32>() * 100.0 / 100.0; // Convert to percentage
    settings.agent_speed_min = rand::random::<f32>() * 500.0;
    settings.agent_speed_max =
        settings.agent_speed_min + rand::random::<f32>() * (500.0 - settings.agent_speed_min);
    settings.agent_turn_rate = (rand::random::<f32>() * 360.0) * std::f32::consts::PI / 180.0; // Convert degrees to radians
    settings.agent_jitter = rand::random::<f32>() * 5.0;
    settings.agent_sensor_angle = (rand::random::<f32>() * 180.0) * std::f32::consts::PI / 180.0; // Convert degrees to radians
    settings.agent_sensor_distance = rand::random::<f32>() * 500.0;

    // Don't randomize gradient settings
    settings.gradient_type = crate::settings::GradientType::Disabled;
    settings.gradient_strength = 0.5;
    settings.gradient_center_x = 0.5;
    settings.gradient_center_y = 0.5;
    settings.gradient_size = 1.0;
    settings.gradient_angle = 0.0;

    // Randomize starting direction range
    let start = rand::random::<f32>() * 360.0;
    let end = start + rand::random::<f32>() * (360.0 - start);
    settings.agent_possible_starting_headings = start..end;

    current_agent_count
}
