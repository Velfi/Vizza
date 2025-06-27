use crate::error::{LutError, LutResult};
use dirs::home_dir;
use rand::Rng;
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone)]
pub struct LutData {
    pub name: String,
    pub red: [u8; 256],
    pub green: [u8; 256],
    pub blue: [u8; 256],
}

impl LutData {
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

    /// Convert to u32 buffer for GPU usage
    pub fn to_u32_buffer(&self) -> Vec<u32> {
        let mut lut_data_combined = Vec::with_capacity(768);
        lut_data_combined.extend_from_slice(&self.red);
        lut_data_combined.extend_from_slice(&self.green);
        lut_data_combined.extend_from_slice(&self.blue);
        lut_data_combined.iter().map(|&x| x as u32).collect()
    }
}

macro_rules! include_luts {
    ($($name:expr),*) => {
        {
            let mut map = HashMap::new();
            $(
                map.insert(
                    $name.strip_suffix(".lut").unwrap(),
                    include_bytes!(concat!("LUTs/", $name))
                );
            )*
            map
        }
    };
}

lazy_static::lazy_static! {
    static ref EMBEDDED_LUTS: HashMap<&'static str, &'static [u8; 768]> = include_luts!(
        "KTZ_bt_Brick.lut",
        "KTZ_bt_Teal.lut",
        "KTZ_bw_Avada.lut",
        "KTZ_bw_CityNight.lut",
        "KTZ_bw_Coral.lut",
        "KTZ_bw_DarkGold.lut",
        "KTZ_bw_DeepBlush.lut",
        "KTZ_bw_DeepLime.lut",
        "KTZ_bw_Div_Orange.lut",
        "KTZ_bw_Ember.lut",
        "KTZ_bw_Incendio.lut",
        "KTZ_bw_IndiGlow.lut",
        "KTZ_bw_Iris.lut",
        "KTZ_bw_kawa.lut",
        "KTZ_bw_Lagoon.lut",
        "KTZ_bw_Lavender.lut",
        "KTZ_bw_Moon.lut",
        "KTZ_bw_NavyGold.lut",
        "KTZ_bw_Nebula.lut",
        "KTZ_bw_NightRose.lut",
        "KTZ_bw_PinkShui.lut",
        "KTZ_bw_Sakura.lut",
        "KTZ_bw_Saphira.lut",
        "KTZ_bw_Scarlet.lut",
        "KTZ_bw_SeaWeed.lut",
        "KTZ_bw_Spectral.lut",
        "KTZ_bw_Sunrise.lut",
        "KTZ_bw_TealHot.lut",
        "KTZ_Campfire.lut",
        "KTZ_color_BCO.lut",
        "KTZ_color_BOG.lut",
        "KTZ_color_Gazoil.lut",
        "KTZ_color_POC.lut",
        "KTZ_color_POCY.lut",
        "KTZ_Div_Cyan.lut",
        "KTZ_Div_Red.lut",
        "KTZ_Grey_Div_Green.lut",
        "KTZ_Grey_Div_Orange.lut",
        "KTZ_Grey_To_Black.lut",
        "KTZ_inv_Noice_Blue.lut",
        "KTZ_inv_Noice_Orange.lut",
        "KTZ_inv_Owl_Red.lut",
        "KTZ_inv_Owl_Teal.lut",
        "KTZ_k_Blue.lut",
        "KTZ_k_Green.lut",
        "KTZ_k_Magenta.lut",
        "KTZ_k_Orange.lut",
        "KTZ_Klein_Blue.lut",
        "KTZ_Klein_Gold.lut",
        "KTZ_Klein_Pink.lut",
        "KTZ_Noice_Blue.lut",
        "KTZ_Noice_Cyan.lut",
        "KTZ_Noice_Green.lut",
        "KTZ_Noice_Magenta.lut",
        "KTZ_Noice_Orange.lut",
        "KTZ_Noice_Red.lut",
        "KTZ_poc_Cyan.lut",
        "KTZ_poc_Orange.lut",
        "KTZ_poc_Purple.lut",
        "KTZ_rgb_Blue.lut",
        "KTZ_rgb_Green.lut",
        "KTZ_rgb_Red.lut",
        "MATPLOTLIB_Accent.lut",
        "MATPLOTLIB_afmhot.lut",
        "MATPLOTLIB_autumn.lut",
        "MATPLOTLIB_berlin.lut",
        "MATPLOTLIB_binary.lut",
        "MATPLOTLIB_Blues.lut",
        "MATPLOTLIB_bone.lut",
        "MATPLOTLIB_BrBG.lut",
        "MATPLOTLIB_brg.lut",
        "MATPLOTLIB_BuGn.lut",
        "MATPLOTLIB_BuPu.lut",
        "MATPLOTLIB_bwr.lut",
        "MATPLOTLIB_cividis.lut",
        "MATPLOTLIB_CMRmap.lut",
        "MATPLOTLIB_cool.lut",
        "MATPLOTLIB_coolwarm.lut",
        "MATPLOTLIB_copper.lut",
        "MATPLOTLIB_cubehelix.lut",
        "MATPLOTLIB_Dark2.lut",
        "MATPLOTLIB_flag.lut",
        "MATPLOTLIB_gist_earth.lut",
        "MATPLOTLIB_gist_gray.lut",
        "MATPLOTLIB_gist_heat.lut",
        "MATPLOTLIB_gist_ncar.lut",
        "MATPLOTLIB_gist_rainbow.lut",
        "MATPLOTLIB_gist_stern.lut",
        "MATPLOTLIB_gist_yarg.lut",
        "MATPLOTLIB_gist_yerg.lut",
        "MATPLOTLIB_GnBu.lut",
        "MATPLOTLIB_gnuplot.lut",
        "MATPLOTLIB_gnuplot2.lut",
        "MATPLOTLIB_gray.lut",
        "MATPLOTLIB_Grays.lut",
        "MATPLOTLIB_Greens.lut",
        "MATPLOTLIB_grey.lut",
        "MATPLOTLIB_Greys.lut",
        "MATPLOTLIB_hot.lut",
        "MATPLOTLIB_hsv.lut",
        "MATPLOTLIB_inferno.lut",
        "MATPLOTLIB_jet.lut",
        "MATPLOTLIB_magma.lut",
        "MATPLOTLIB_managua.lut",
        "MATPLOTLIB_nipy_spectral.lut",
        "MATPLOTLIB_ocean.lut",
        "MATPLOTLIB_Oranges.lut",
        "MATPLOTLIB_OrRd.lut",
        "MATPLOTLIB_Paired.lut",
        "MATPLOTLIB_Pastel1.lut",
        "MATPLOTLIB_Pastel2.lut",
        "MATPLOTLIB_pink.lut",
        "MATPLOTLIB_PiYG.lut",
        "MATPLOTLIB_plasma.lut",
        "MATPLOTLIB_PRGn.lut",
        "MATPLOTLIB_prism.lut",
        "MATPLOTLIB_PuBu.lut",
        "MATPLOTLIB_PuBuGn.lut",
        "MATPLOTLIB_PuOr.lut",
        "MATPLOTLIB_PuRd.lut",
        "MATPLOTLIB_Purples.lut",
        "MATPLOTLIB_rainbow.lut",
        "MATPLOTLIB_RdBu.lut",
        "MATPLOTLIB_RdGy.lut",
        "MATPLOTLIB_RdPu.lut",
        "MATPLOTLIB_RdYlBu.lut",
        "MATPLOTLIB_RdYlGn.lut",
        "MATPLOTLIB_Reds.lut",
        "MATPLOTLIB_seismic.lut",
        "MATPLOTLIB_Set1.lut",
        "MATPLOTLIB_Set2.lut",
        "MATPLOTLIB_Set3.lut",
        "MATPLOTLIB_Spectral.lut",
        "MATPLOTLIB_spring.lut",
        "MATPLOTLIB_summer.lut",
        "MATPLOTLIB_tab10.lut",
        "MATPLOTLIB_tab20.lut",
        "MATPLOTLIB_tab20b.lut",
        "MATPLOTLIB_tab20c.lut",
        "MATPLOTLIB_terrain.lut",
        "MATPLOTLIB_turbo.lut",
        "MATPLOTLIB_twilight_shifted.lut",
        "MATPLOTLIB_twilight.lut",
        "MATPLOTLIB_vanimo.lut",
        "MATPLOTLIB_viridis.lut",
        "MATPLOTLIB_winter.lut",
        "MATPLOTLIB_Wistia.lut",
        "MATPLOTLIB_YlGn.lut",
        "MATPLOTLIB_YlGnBu.lut",
        "MATPLOTLIB_YlOrBr.lut",
        "MATPLOTLIB_YlOrRd.lut",
        "ZELDA_Glass.lut",
        "ZELDA_Monochrome.lut",
        "ZELDA_Rainbow.lut",
        "ZELDA_Slava Ukraini.lut",
        "ZELDA_Terrain.lut",
        "ZELDA_Trans Rights.lut"
    );
}

#[derive(Debug, Clone)]
pub struct LutManager;

impl LutManager {
    pub fn new() -> Self {
        Self
    }

    pub fn all_luts(&self) -> Vec<String> {
        let mut luts: Vec<String> = EMBEDDED_LUTS.keys().map(|&name| name.to_string()).collect();

        // Add custom LUTs
        if let Ok(custom_luts) = self.all_custom_luts() {
            luts.extend(custom_luts);
        }

        luts.sort();
        luts
    }

    pub fn get(&self, name: &str) -> LutResult<LutData> {
        // Try to load from embedded LUTs first
        if let Some(&buffer) = EMBEDDED_LUTS.get(name) {
            // Each color component should be 256 bytes
            if buffer.len() != 768 {
                // 256 * 3 (RGB)
                return Err(LutError::DataError(format!(
                    "Invalid LUT file size for {}",
                    name
                )));
            }

            return LutData::from_bytes(name.to_string(), buffer.as_slice())
                .map_err(|e| LutError::DataError(e.to_string()));
        }

        // If not found in embedded LUTs, try to load as a custom LUT
        self.get_custom(name)
    }

    fn lut_dir() -> LutResult<std::path::PathBuf> {
        let home_dir = home_dir()
            .ok_or_else(|| LutError::DataError("Could not find home directory".to_string()))?;

        let lut_dir = home_dir.join("sim-pix").join("LUTs");
        Ok(lut_dir)
    }

    pub fn save_custom(&self, name: &str, lut_data: &LutData) -> LutResult<()> {
        // Create LUTs directory if it doesn't exist
        let lut_dir = Self::lut_dir()?;
        if !lut_dir.exists() {
            std::fs::create_dir_all(&lut_dir).map_err(|e| LutError::DataError(e.to_string()))?;
        }

        // Save the LUT file
        let file_path = lut_dir.join(format!("{}.lut", name));
        std::fs::write(file_path, lut_data.clone().into_bytes())
            .map_err(|e| LutError::DataError(e.to_string()))?;

        Ok(())
    }

    pub fn all_custom_luts(&self) -> LutResult<Vec<String>> {
        let lut_dir = Self::lut_dir()?;
        if !lut_dir.exists() {
            return Ok(Vec::new());
        }

        let mut custom_luts = Vec::new();
        for entry in std::fs::read_dir(lut_dir).map_err(|e| LutError::DataError(e.to_string()))? {
            let entry = entry.map_err(|e| LutError::DataError(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lut") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    custom_luts.push(name.to_string());
                }
            }
        }

        Ok(custom_luts)
    }

    pub fn get_custom(&self, name: &str) -> LutResult<LutData> {
        let file_path = Self::lut_dir()?.join(format!("{}.lut", name));
        let data = std::fs::read(file_path).map_err(|e| LutError::DataError(e.to_string()))?;
        LutData::from_bytes(name.to_string(), &data).map_err(|e| LutError::DataError(e.to_string()))
    }

    pub fn get_default(&self) -> LutData {
        let mut lut_data = self.get("MATPLOTLIB_bone").unwrap();
        lut_data.reverse();
        lut_data
    }

    pub(crate) fn get_random_lut(&self) -> LutResult<LutData> {
        let mut lut_names = EMBEDDED_LUTS.keys();
        let random_index = rand::rng().random_range(0..lut_names.len());
        let lut_name = lut_names.nth(random_index).expect("LUTs exist");
        self.get(lut_name)
    }
}

impl Default for LutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified LUT manager for simulation operations
pub struct SimulationLutManager;

impl SimulationLutManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_luts(&self, lut_manager: &LutManager) -> Vec<String> {
        lut_manager.all_luts()
    }
}

impl Default for SimulationLutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_buffer_sizes() {
        let lut_data = LutData {
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
}
