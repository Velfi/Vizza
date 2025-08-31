use crate::commands::get_settings_dir;
use crate::error::{ColorSchemeError, LutResult};
use include_dir::{Dir, include_dir};
use rand::Rng;
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub red: [u8; 256],
    pub green: [u8; 256],
    pub blue: [u8; 256],
}

impl ColorScheme {
    pub fn reversed(&self) -> Self {
        let mut red = self.red;
        let mut green = self.green;
        let mut blue = self.blue;
        red.reverse();
        green.reverse();
        blue.reverse();
        Self {
            name: format!("{}_reversed", self.name),
            red,
            green,
            blue,
        }
    }

    pub fn reverse(&mut self) {
        self.red.reverse();
        self.green.reverse();
        self.blue.reverse();
    }

    pub fn from_bytes(name: String, data: &[u8]) -> io::Result<Self> {
        if data.len() != 768 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid LUT data size",
            ));
        }

        Ok(Self {
            name,
            red: data[0..256].try_into().expect("invalid LUT data"),
            green: data[256..512].try_into().expect("invalid LUT data"),
            blue: data[512..768].try_into().expect("invalid LUT data"),
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(768);
        bytes.extend_from_slice(&self.red);
        bytes.extend_from_slice(&self.green);
        bytes.extend_from_slice(&self.blue);
        bytes
    }

    /// Get colors for a given number of species
    /// Returns a vector of RGBA colors (f32 values in range 0-1) in linear color space
    pub fn get_colors(&self, n: usize) -> Vec<Vec<f32>> {
        let mut colors = Vec::with_capacity(n);

        // Sample n equidistant points along the LUT (0 to 255)
        for i in 0..n {
            let index = if n == 1 { 0 } else { (i * 255) / (n - 1) };
            let index = index.min(255);

            // Convert from sRGB (gamma-corrected) to linear RGB
            let srgb_to_linear = |srgb: f32| -> f32 {
                if srgb <= 0.04045 {
                    srgb / 12.92
                } else {
                    ((srgb + 0.055) / 1.055).powf(2.4)
                }
            };

            let r_srgb = self.red[index] as f32 / 255.0;
            let g_srgb = self.green[index] as f32 / 255.0;
            let b_srgb = self.blue[index] as f32 / 255.0;

            colors.push(vec![
                srgb_to_linear(r_srgb),
                srgb_to_linear(g_srgb),
                srgb_to_linear(b_srgb),
                1.0, // Alpha is always 1.0
            ]);
        }

        colors
    }

    pub fn get_first_color(&self) -> Option<Vec<f32>> {
        self.get_colors(1).first().cloned()
    }

    pub fn get_last_color(&self) -> Option<Vec<f32>> {
        self.get_colors(2).last().cloned()
    }
    /// Convert to u32 buffer for GPU usage
    pub fn to_u32_buffer(&self) -> Vec<u32> {
        let mut lut_data_combined = Vec::with_capacity(768);
        lut_data_combined.extend_from_slice(&self.red);
        lut_data_combined.extend_from_slice(&self.green);
        lut_data_combined.extend_from_slice(&self.blue);
        lut_data_combined.iter().map(|&x| x as u32).collect()
    }
}

static LUT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/simulations/shared/LUTs");

lazy_static::lazy_static! {
    static ref EMBEDDED_COLOR_SCHEMES: HashMap<&'static str, &'static [u8]> = {
        let mut map = HashMap::new();
        for file in LUT_DIR.files() {
            if let Some(name) = file.path().file_stem().and_then(|s| s.to_str()) {
                if file.path().extension().and_then(|s| s.to_str()) == Some("lut") {
                    map.insert(name, file.contents());
                }
            }
        }
        map
    };
}

#[derive(Debug, Clone)]
pub struct ColorSchemeManager {}

impl ColorSchemeManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn all_color_schemes(&self) -> Vec<String> {
        let mut luts: Vec<String> = EMBEDDED_COLOR_SCHEMES
            .keys()
            .map(|&name| name.to_string())
            .collect();

        // Add custom LUTs
        if let Ok(custom_luts) = self.all_custom_luts() {
            luts.extend(custom_luts);
        }

        luts.sort();
        luts
    }

    pub fn get(&self, name: &str) -> LutResult<ColorScheme> {
        // Try to load from embedded LUTs first
        if let Some(&buffer) = EMBEDDED_COLOR_SCHEMES.get(name) {
            // Each color component should be 256 bytes
            if buffer.len() != 768 {
                // 256 * 3 (RGB)
                return Err(ColorSchemeError::DataError(format!(
                    "Invalid LUT file size for {}",
                    name
                )));
            }

            return ColorScheme::from_bytes(name.to_string(), buffer)
                .map_err(|e| ColorSchemeError::DataError(e.to_string()));
        }

        // If not found in embedded LUTs, try to load as a custom LUT
        self.get_custom(name)
    }

    fn lut_dir() -> LutResult<std::path::PathBuf> {
        let lut_dir = get_settings_dir().join("LUTs");
        Ok(lut_dir)
    }

    pub fn save_custom(&self, name: &str, lut_data: &ColorScheme) -> LutResult<()> {
        // Create LUTs directory if it doesn't exist
        let lut_dir = Self::lut_dir()?;
        if !lut_dir.exists() {
            std::fs::create_dir_all(&lut_dir)
                .map_err(|e| ColorSchemeError::DataError(e.to_string()))?;
        }

        // Save the LUT file
        let file_path = lut_dir.join(format!("{}.lut", name));
        std::fs::write(file_path, lut_data.clone().into_bytes())
            .map_err(|e| ColorSchemeError::DataError(e.to_string()))?;

        Ok(())
    }

    pub fn all_custom_luts(&self) -> LutResult<Vec<String>> {
        let lut_dir = Self::lut_dir()?;
        if !lut_dir.exists() {
            return Ok(Vec::new());
        }

        let mut custom_luts = Vec::new();
        for entry in
            std::fs::read_dir(lut_dir).map_err(|e| ColorSchemeError::DataError(e.to_string()))?
        {
            let entry = entry.map_err(|e| ColorSchemeError::DataError(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lut") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    custom_luts.push(name.to_string());
                }
            }
        }

        Ok(custom_luts)
    }

    pub fn get_custom(&self, name: &str) -> LutResult<ColorScheme> {
        let file_path = Self::lut_dir()?.join(format!("{}.lut", name));
        let data =
            std::fs::read(file_path).map_err(|e| ColorSchemeError::DataError(e.to_string()))?;
        ColorScheme::from_bytes(name.to_string(), &data)
            .map_err(|e| ColorSchemeError::DataError(e.to_string()))
    }

    pub fn get_default(&self) -> ColorScheme {
        let mut lut_data = self.get("MATPLOTLIB_bone").unwrap();
        lut_data.reverse();
        lut_data
    }

    pub(crate) fn get_random_lut(&self) -> LutResult<ColorScheme> {
        let lut_names: Vec<&str> = EMBEDDED_COLOR_SCHEMES.keys().copied().collect();
        let random_index = rand::rng().random_range(0..lut_names.len());
        let lut_name = lut_names[random_index];
        self.get(lut_name)
    }
}

impl Default for ColorSchemeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified LUT manager for simulation operations
pub struct SimulationColorSchemeManager;

impl SimulationColorSchemeManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_color_schemes(
        &self,
        color_scheme_manager: &ColorSchemeManager,
    ) -> Vec<String> {
        color_scheme_manager.all_color_schemes()
    }
}

impl Default for SimulationColorSchemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_buffer_sizes() {
        let lut_data = ColorScheme {
            name: "test".to_string(),
            red: [0; 256],
            green: [0; 256],
            blue: [0; 256],
        };

        // Test u8 buffer size (original format)
        let u8_buffer = lut_data.clone().into_bytes();
        assert_eq!(u8_buffer.len(), 768); // 256 * 3 = 768 bytes

        // Test u32 buffer size (GPU format)
        let u32_buffer = lut_data.to_u32_buffer();
        assert_eq!(u32_buffer.len(), 768); // 768 u32 values
    }

    #[test]
    fn test_automatic_lut_loading() {
        let manager = ColorSchemeManager::new();
        let luts = manager.all_color_schemes();

        // Verify that some known LUTs are included
        assert!(luts.contains(&"MATPLOTLIB_viridis".to_string()));
        assert!(luts.contains(&"ZELDA_Blueprint".to_string()));
        assert!(luts.contains(&"KTZ_bw_Avada".to_string()));

        // Verify that we can load each LUT
        for lut_name in &luts {
            let lut_data = manager.get(lut_name).expect("LUT exists");

            // Verify the LUT data is valid
            assert_eq!(lut_data.name, *lut_name);
            assert_eq!(lut_data.red.len(), 256);
            assert_eq!(lut_data.green.len(), 256);
            assert_eq!(lut_data.blue.len(), 256);
        }

        // Verify that we have a reasonable number of LUTs
        assert!(
            luts.len() > 100,
            "Expected many LUTs but got {}",
            luts.len()
        );
    }
}
