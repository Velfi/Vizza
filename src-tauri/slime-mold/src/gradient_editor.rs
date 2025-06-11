use egui::{Color32, PointerButton, Pos2, Rect, Stroke, StrokeKind, Ui};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ColorSpace {
    LinearRgb,
    Lab,
    Hsv,
    Oklab,
}

impl ColorSpace {
    fn as_str(&self) -> &'static str {
        match self {
            ColorSpace::LinearRgb => "Linear RGB",
            ColorSpace::Lab => "LAB",
            ColorSpace::Hsv => "HSV",
            ColorSpace::Oklab => "Oklab",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GradientStop {
    pub position: f32,  // 0.0 to 1.0
    pub color: Color32,
}

pub struct GradientEditor {
    stops: Vec<GradientStop>,
    selected_stop: Option<usize>,
    dragging: bool,
    pub color_space: ColorSpace,
}

impl GradientEditor {
    pub fn new() -> Self {
        // Initialize with a default black-to-white gradient
        let stops = vec![
            GradientStop {
                position: 0.0,
                color: Color32::BLUE,
            },
            GradientStop {
                position: 1.0,
                color: Color32::YELLOW,
            },
        ];

        Self {
            stops,
            selected_stop: None,
            dragging: false,
            color_space: ColorSpace::Oklab,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        
        // Use a fixed height for the gradient preview
        let preview_height = 40.0;
        let preview_width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(preview_width, preview_height), egui::Sense::drag());

        // Draw the gradient preview
        let painter = ui.painter();
        let steps = 256;
        let step_width = rect.width() / steps as f32;

        for i in 0..steps {
            let t = i as f32 / (steps - 1) as f32;
            let color = self.get_color_at(t);
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x + i as f32 * step_width, rect.min.y),
                    egui::vec2(step_width, rect.height() - 20.0),
                ),
                0.0,
                color,
            );
        }

        // Draw the stops
        for (i, stop) in self.stops.iter().enumerate() {
            let x = rect.min.x + stop.position * rect.width();
            let y = rect.min.y + rect.height() - 10.0;
            
            let stop_rect = Rect::from_center_size(
                Pos2::new(x, y),
                egui::vec2(10.0, 20.0),
            );

            let is_selected = self.selected_stop == Some(i);
            let stroke = if is_selected {
                Stroke::new(2.0, Color32::WHITE)
            } else {
                Stroke::new(1.0, Color32::GRAY)
            };

            painter.rect_stroke(stop_rect, 0.0, stroke, StrokeKind::Middle);
            painter.rect_filled(stop_rect, 0.0, stop.color);

            // Handle stop selection and dragging
            if ui.rect_contains_pointer(stop_rect) {
                if ui.input(|i| i.pointer.primary_down()) {
                    self.selected_stop = Some(i);
                    self.dragging = true;
                    changed = true;
                }
            }
        }

        // Handle dragging
        if self.dragging {
            if let Some(idx) = self.selected_stop {
                if ui.input(|i| i.pointer.primary_down()) {
                    let pointer_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();
                    let new_pos = ((pointer_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                    self.stops[idx].position = new_pos;
                    // Sort stops after dragging
                    let dragged_color = self.stops[idx].color;
                    self.stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
                    // Update selected_stop to new index of the dragged stop
                    self.selected_stop = self.stops.iter().position(|stop| stop.position == new_pos && stop.color == dragged_color);
                    changed = true;
                } else {
                    self.dragging = false;
                }
            }
        }

        // Add new stop on double click
        if ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary)) {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let position = ((pointer_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                let color = self.get_color_at(position);
                self.stops.push(GradientStop { position, color });
                self.stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
                changed = true;
            }
        }

        // Delete selected stop with Delete key
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(idx) = self.selected_stop {
                if self.stops.len() > 2 {
                    self.stops.remove(idx);
                    self.selected_stop = None;
                    changed = true;
                }
            }
        }

        // Color picker for selected stop
        if let Some(idx) = self.selected_stop {
            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut color = self.stops[idx].color;
                if ui.color_edit_button_srgba(&mut color).changed() {
                    self.stops[idx].color = color;
                    changed = true;
                }
            });
        }

        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Color space")
                .selected_text(self.color_space.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.color_space, ColorSpace::LinearRgb, "Linear RGB");
                    ui.selectable_value(&mut self.color_space, ColorSpace::Lab, "LAB");
                    ui.selectable_value(&mut self.color_space, ColorSpace::Hsv, "HSV");
                    ui.selectable_value(&mut self.color_space, ColorSpace::Oklab, "Oklab");
                });
        });

        changed
    }

    // Helper functions for color space conversions
    fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let v = max;
        let delta = max - min;
        let s = if max != 0.0 { delta / max } else { 0.0 };
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };
        let h = if h < 0.0 { h + 360.0 } else { h };
        (h, s, v)
    }

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r1, g1, b1) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        (r1 + m, g1 + m, b1 + m)
    }

    // sRGB <-> XYZ <-> LAB conversion helpers
    fn srgb_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn linear_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    fn rgb_to_xyz(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        // Convert sRGB to linear
        let r = GradientEditor::srgb_to_linear(r);
        let g = GradientEditor::srgb_to_linear(g);
        let b = GradientEditor::srgb_to_linear(b);
        // Observer = 2°, Illuminant = D65
        let x = r * 0.4124 + g * 0.3576 + b * 0.1805;
        let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
        let z = r * 0.0193 + g * 0.1192 + b * 0.9505;
        (x, y, z)
    }

    fn xyz_to_rgb(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        // Observer = 2°, Illuminant = D65
        let r = x * 3.2406 + y * -1.5372 + z * -0.4986;
        let g = x * -0.9689 + y * 1.8758 + z * 0.0415;
        let b = x * 0.0557 + y * -0.2040 + z * 1.0570;
        (GradientEditor::linear_to_srgb(r), GradientEditor::linear_to_srgb(g), GradientEditor::linear_to_srgb(b))
    }

    fn xyz_to_lab(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        // D65 reference white
        let xr = x / 0.95047;
        let yr = y / 1.00000;
        let zr = z / 1.08883;
        let fx = if xr > 0.008856 { xr.powf(1.0 / 3.0) } else { (7.787 * xr) + (16.0 / 116.0) };
        let fy = if yr > 0.008856 { yr.powf(1.0 / 3.0) } else { (7.787 * yr) + (16.0 / 116.0) };
        let fz = if zr > 0.008856 { zr.powf(1.0 / 3.0) } else { (7.787 * zr) + (16.0 / 116.0) };
        let l = (116.0 * fy) - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);
        (l, a, b)
    }

    fn lab_to_xyz(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;
        let fx3 = fx.powi(3);
        let fy3 = fy.powi(3);
        let fz3 = fz.powi(3);
        let xr = if fx3 > 0.008856 { fx3 } else { (fx - 16.0 / 116.0) / 7.787 };
        let yr = if l > (903.3 * 0.008856) { fy3 } else { l / 903.3 };
        let zr = if fz3 > 0.008856 { fz3 } else { (fz - 16.0 / 116.0) / 7.787 };
        let x = xr * 0.95047;
        let y = yr * 1.00000;
        let z = zr * 1.08883;
        (x, y, z)
    }

    fn rgb_to_lab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let (x, y, z) = GradientEditor::rgb_to_xyz(r, g, b);
        GradientEditor::xyz_to_lab(x, y, z)
    }

    fn lab_to_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
        let (x, y, z) = GradientEditor::lab_to_xyz(l, a, b);
        GradientEditor::xyz_to_rgb(x, y, z)
    }

    fn get_color_at(&self, position: f32) -> Color32 {
        if self.stops.is_empty() {
            return Color32::BLACK;
        }

        // If position is before the first stop, return the first stop's color
        if position <= self.stops[0].position {
            return self.stops[0].color;
        }
        // If position is after the last stop, return the last stop's color
        if position >= self.stops[self.stops.len() - 1].position {
            return self.stops[self.stops.len() - 1].color;
        }

        // Find the two stops that bound the position
        let mut left_stop = &self.stops[0];
        let mut right_stop = &self.stops[self.stops.len() - 1];

        for i in 0..self.stops.len() - 1 {
            if self.stops[i].position <= position && self.stops[i + 1].position >= position {
                left_stop = &self.stops[i];
                right_stop = &self.stops[i + 1];
                break;
            }
        }

        // Calculate the interpolation factor
        let t = (position - left_stop.position) / (right_stop.position - left_stop.position);

        // Convert colors to RGB
        let left_rgb = left_stop.color.to_array();
        let right_rgb = right_stop.color.to_array();

        match self.color_space {
            ColorSpace::LinearRgb => {
                let mut interpolated_rgb = [0u8; 3];
                for i in 0..3 {
                    let left = left_rgb[i] as f32 / 255.0;
                    let right = right_rgb[i] as f32 / 255.0;
                    let v = left * (1.0 - t) + right * t;
                    interpolated_rgb[i] = (v * 255.0).round() as u8;
                }
                Color32::from_rgb(interpolated_rgb[0], interpolated_rgb[1], interpolated_rgb[2])
            }
            ColorSpace::Lab => {
                // Convert to LAB
                let l1 = left_rgb[0] as f32 / 255.0;
                let a1 = left_rgb[1] as f32 / 255.0;
                let b1 = left_rgb[2] as f32 / 255.0;
                let (l1, a1, b1) = GradientEditor::rgb_to_lab(l1, a1, b1);
                let l2 = right_rgb[0] as f32 / 255.0;
                let a2 = right_rgb[1] as f32 / 255.0;
                let b2 = right_rgb[2] as f32 / 255.0;
                let (l2, a2, b2) = GradientEditor::rgb_to_lab(l2, a2, b2);
                // Interpolate in LAB
                let l = l1 * (1.0 - t) + l2 * t;
                let a = a1 * (1.0 - t) + a2 * t;
                let b = b1 * (1.0 - t) + b2 * t;
                let (r, g, b) = GradientEditor::lab_to_rgb(l, a, b);
                Color32::from_rgb(
                    (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                )
            }
            ColorSpace::Hsv => {
                // Convert to HSV
                let r1 = left_rgb[0] as f32 / 255.0;
                let g1 = left_rgb[1] as f32 / 255.0;
                let b1 = left_rgb[2] as f32 / 255.0;
                let (h1, s1, v1) = GradientEditor::rgb_to_hsv(r1, g1, b1);
                let r2 = right_rgb[0] as f32 / 255.0;
                let g2 = right_rgb[1] as f32 / 255.0;
                let b2 = right_rgb[2] as f32 / 255.0;
                let (h2, s2, v2) = GradientEditor::rgb_to_hsv(r2, g2, b2);
                // Interpolate hue circularly
                let mut dh = h2 - h1;
                if dh > 180.0 {
                    dh -= 360.0;
                } else if dh < -180.0 {
                    dh += 360.0;
                }
                let h = (h1 + t * dh + 360.0) % 360.0;
                let s = s1 * (1.0 - t) + s2 * t;
                let v = v1 * (1.0 - t) + v2 * t;
                let (r, g, b) = GradientEditor::hsv_to_rgb(h, s, v);
                Color32::from_rgb(
                    (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                )
            }
            ColorSpace::Oklab => {
                // sRGB -> linear RGB
                fn srgb_to_linear(c: f32) -> f32 {
                    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
                }
                // linear RGB -> sRGB
                fn linear_to_srgb(c: f32) -> f32 {
                    if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0/2.4) - 0.055 }
                }
                // linear RGB -> Oklab
                fn linear_rgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
                    let l = 0.4122214708*r + 0.5363325363*g + 0.0514459929*b;
                    let m = 0.2119034982*r + 0.6806995451*g + 0.1073969566*b;
                    let s = 0.0883024619*r + 0.2817188376*g + 0.6299787005*b;
                    let l_ = l.cbrt();
                    let m_ = m.cbrt();
                    let s_ = s.cbrt();
                    let l = 0.2104542553*l_ + 0.7936177850*m_ - 0.0040720468*s_;
                    let a = 1.9779984951*l_ - 2.4285922050*m_ + 0.4505937099*s_;
                    let b = 0.0259040371*l_ + 0.7827717662*m_ - 0.8086757660*s_;
                    (l, a, b)
                }
                // Oklab -> linear RGB
                fn oklab_to_linear_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
                    let l_ = l + 0.3963377774*a + 0.2158037573*b;
                    let m_ = l - 0.1055613458*a - 0.0638541728*b;
                    let s_ = l - 0.0894841775*a - 1.2914855480*b;
                    let l = l_.powi(3);
                    let m = m_.powi(3);
                    let s = s_.powi(3);
                    let r =  4.0767416621*l - 3.3077115913*m + 0.2309699292*s;
                    let g = -1.2684380046*l + 2.6097574011*m - 0.3413193965*s;
                    let b = -0.0041960863*l - 0.7034186147*m + 1.7076147010*s;
                    (r, g, b)
                }
                // Convert both stops to Oklab
                let l_rgb = [left_rgb[0] as f32 / 255.0, left_rgb[1] as f32 / 255.0, left_rgb[2] as f32 / 255.0];
                let r_rgb = [right_rgb[0] as f32 / 255.0, right_rgb[1] as f32 / 255.0, right_rgb[2] as f32 / 255.0];
                let l_lin = [srgb_to_linear(l_rgb[0]), srgb_to_linear(l_rgb[1]), srgb_to_linear(l_rgb[2])];
                let r_lin = [srgb_to_linear(r_rgb[0]), srgb_to_linear(r_rgb[1]), srgb_to_linear(r_rgb[2])];
                let (l_l, la, lb) = linear_rgb_to_oklab(l_lin[0], l_lin[1], l_lin[2]);
                let (r_l, ra, rb) = linear_rgb_to_oklab(r_lin[0], r_lin[1], r_lin[2]);
                // Interpolate in Oklab
                let l = l_l * (1.0 - t) + r_l * t;
                let a = la * (1.0 - t) + ra * t;
                let b = lb * (1.0 - t) + rb * t;
                // Convert back to linear RGB
                let (r, g, b) = oklab_to_linear_rgb(l, a, b);
                // Convert to sRGB
                let r = linear_to_srgb(r).clamp(0.0, 1.0);
                let g = linear_to_srgb(g).clamp(0.0, 1.0);
                let b = linear_to_srgb(b).clamp(0.0, 1.0);
                Color32::from_rgb((r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8)
            }
        }
    }

    pub fn generate_lut(&self) -> Vec<u8> {
        let mut lut = Vec::with_capacity(768); // 256 * 3 for RGB
        
        // First generate all red values
        for i in 0..256 {
            let t = i as f32 / 255.0;
            let color = self.get_color_at(t);
            lut.push(color.r());
        }
        
        // Then all green values
        for i in 0..256 {
            let t = i as f32 / 255.0;
            let color = self.get_color_at(t);
            lut.push(color.g());
        }
        
        // Finally all blue values
        for i in 0..256 {
            let t = i as f32 / 255.0;
            let color = self.get_color_at(t);
            lut.push(color.b());
        }
        
        lut
    }
} 