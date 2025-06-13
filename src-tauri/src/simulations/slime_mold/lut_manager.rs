use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LutData {
    pub name: String,
    pub red: Vec<u8>,
    pub green: Vec<u8>,
    pub blue: Vec<u8>,
}

impl LutData {
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
            red: data[0..256].to_vec(),
            green: data[256..512].to_vec(),
            blue: data[512..768].to_vec(),
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(768);
        bytes.extend_from_slice(&self.red);
        bytes.extend_from_slice(&self.green);
        bytes.extend_from_slice(&self.blue);
        bytes
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
        "MATPLOTLIB_Accent_r.lut",
        "MATPLOTLIB_bone_r.lut",
        "MATPLOTLIB_brg_r.lut",
        "MATPLOTLIB_bwr_r.lut",
        "MATPLOTLIB_cool_r.lut",
        "MATPLOTLIB_coolwarm_r.lut",
        "MATPLOTLIB_copper_r.lut",
        "MATPLOTLIB_cubehelix_r.lut",
        "MATPLOTLIB_Dark2_r.lut",
        "MATPLOTLIB_flag_r.lut",
        "MATPLOTLIB_gist_earth_r.lut",
        "MATPLOTLIB_gist_gray_r.lut",
        "MATPLOTLIB_gist_grey_r.lut",
        "MATPLOTLIB_gist_heat_r.lut",
        "MATPLOTLIB_gist_ncar_r.lut",
        "MATPLOTLIB_gist_rainbow_r.lut",
        "MATPLOTLIB_gist_stern_r.lut",
        "MATPLOTLIB_gist_yarg_r.lut",
        "MATPLOTLIB_gist_yerg_r.lut",
        "MATPLOTLIB_gnuplot_r.lut",
        "MATPLOTLIB_gnuplot2_r.lut",
        "MATPLOTLIB_gray_r.lut",
        "MATPLOTLIB_Grays_r.lut",
        "MATPLOTLIB_grey_r.lut",
        "MATPLOTLIB_hot_r.lut",
        "MATPLOTLIB_hsv_r.lut",
        "MATPLOTLIB_jet_r.lut",
        "MATPLOTLIB_nipy_spectral_r.lut",
        "MATPLOTLIB_ocean_r.lut",
        "MATPLOTLIB_Paired_r.lut",
        "MATPLOTLIB_Pastel1_r.lut",
        "MATPLOTLIB_Pastel2_r.lut",
        "MATPLOTLIB_pink_r.lut",
        "MATPLOTLIB_prism_r.lut",
        "MATPLOTLIB_rainbow_r.lut",
        "MATPLOTLIB_seismic_r.lut",
        "MATPLOTLIB_Set1_r.lut",
        "MATPLOTLIB_Set2_r.lut",
        "MATPLOTLIB_Set3_r.lut",
        "MATPLOTLIB_spring_r.lut",
        "MATPLOTLIB_summer_r.lut",
        "MATPLOTLIB_tab10_r.lut",
        "MATPLOTLIB_tab20_r.lut",
        "MATPLOTLIB_tab20b_r.lut",
        "MATPLOTLIB_tab20c_r.lut",
        "MATPLOTLIB_terrain_r.lut",
        "MATPLOTLIB_winter_r.lut",
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
        "ZELDA_Glass.lut",
        "ZELDA_Monochrome.lut",
        "ZELDA_Rainbow.lut",
        "ZELDA_Slava Ukraini.lut",
        "ZELDA_Terrain.lut",
        "ZELDA_Trans Rights.lut"
    );
}

pub struct LutManager;

impl LutManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_luts(&self) -> Vec<String> {
        let mut luts: Vec<String> = EMBEDDED_LUTS.keys().map(|&name| name.to_string()).collect();

        // Add custom LUTs
        if let Ok(custom_luts) = self.get_custom_luts() {
            luts.extend(custom_luts);
        }

        luts.sort();
        luts
    }

    pub fn load_lut(&self, name: &str) -> io::Result<LutData> {
        // Try to load from embedded LUTs first
        if let Some(buffer) = EMBEDDED_LUTS.get(name) {
            // Each color component should be 256 bytes
            if buffer.len() != 768 {
                // 256 * 3 (RGB)
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid LUT file size",
                ));
            }

            let red = buffer[0..256].to_vec();
            let green = buffer[256..512].to_vec();
            let blue = buffer[512..768].to_vec();

            return Ok(LutData {
                name: name.to_string(),
                red,
                green,
                blue,
            });
        }

        // If not found in embedded LUTs, try to load as a custom LUT
        self.load_custom_lut(name)
    }

    fn get_lut_dir() -> io::Result<std::path::PathBuf> {
        let home_dir = home_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not find home directory")
        })?;

        let lut_dir = home_dir.join("slime-mold").join("LUTs");
        Ok(lut_dir)
    }

    pub fn save_custom_lut(&self, name: &str, lut_data: &LutData) -> io::Result<()> {
        // Create LUTs directory if it doesn't exist
        let lut_dir = Self::get_lut_dir()?;
        if !lut_dir.exists() {
            std::fs::create_dir_all(&lut_dir)?;
        }

        // Save the LUT file
        let file_path = lut_dir.join(format!("{}.lut", name));
        std::fs::write(file_path, lut_data.clone().into_bytes())?;

        Ok(())
    }

    pub fn get_custom_luts(&self) -> io::Result<Vec<String>> {
        let lut_dir = Self::get_lut_dir()?;
        if !lut_dir.exists() {
            return Ok(Vec::new());
        }

        let mut custom_luts = Vec::new();
        for entry in std::fs::read_dir(lut_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lut") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    custom_luts.push(name.to_string());
                }
            }
        }

        Ok(custom_luts)
    }

    pub fn load_custom_lut(&self, name: &str) -> io::Result<LutData> {
        let file_path = Self::get_lut_dir()?.join(format!("{}.lut", name));
        let data = std::fs::read(file_path)?;
        LutData::from_bytes(name.to_string(), &data)
    }
}

impl Default for LutManager {
    fn default() -> Self {
        Self::new()
    }
}
