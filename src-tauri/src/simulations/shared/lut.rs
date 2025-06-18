use dirs::home_dir;
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
    /// Returns a vector of RGBA colors (f32 values in range 0-1)
    pub fn get_colors(&self, num_species: usize) -> Vec<Vec<f32>> {
        let n = num_species + 1; // +1 for background color
        let mut colors = Vec::with_capacity(n);

        // Sample n equidistant points along the LUT (0 to 255)
        for i in 0..n {
            let index = if n == 1 { 0 } else { (i * 255) / (n - 1) };
            let index = index.min(255);

            colors.push(vec![
                self.red[index] as f32 / 255.0,
                self.green[index] as f32 / 255.0,
                self.blue[index] as f32 / 255.0,
                1.0, // Alpha is always 1.0
            ]);
        }

        colors
    }

    /// Get the background color (first color in the LUT)
    pub fn get_background_color(&self) -> [f32; 4] {
        [
            self.red[0] as f32 / 255.0,
            self.green[0] as f32 / 255.0,
            self.blue[0] as f32 / 255.0,
            1.0,
        ]
    }

    /// Get a color at a specific position in the LUT (0.0 to 1.0)
    pub fn get_color_at(&self, position: f32) -> [f32; 4] {
        let index = (position * 255.0) as usize;
        let index = index.min(255);
        [
            self.red[index] as f32 / 255.0,
            self.green[index] as f32 / 255.0,
            self.blue[index] as f32 / 255.0,
            1.0,
        ]
    }

    pub(crate) fn get_particle_color(&self, particle_type: usize, num_species: usize) -> [f32; 4] {
        debug_assert!(
            num_species >= 2,
            "num_species is expected to be between 2 and 8."
        );
        debug_assert!(
            num_species <= 8,
            "num_species is expected to be between 2 and 8."
        );
        debug_assert!(
            particle_type < num_species,
            "particle_type is expected to be less than num_species."
        );

        // 0 is background color, 1 is first species, 2 is second species, etc.
        // We add one because this function isn't meant to be used for the background color.
        let index = (particle_type + 1) * 255 / num_species;

        [
            self.red[index] as f32 / 255.0,
            self.green[index] as f32 / 255.0,
            self.blue[index] as f32 / 255.0,
            1.0,
        ]
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
        "MATPLOTLIB_Accent.lut",
        "MATPLOTLIB_afmhot_r.lut",
        "MATPLOTLIB_afmhot.lut",
        "MATPLOTLIB_autumn_r.lut",
        "MATPLOTLIB_autumn.lut",
        "MATPLOTLIB_berlin_r.lut",
        "MATPLOTLIB_berlin.lut",
        "MATPLOTLIB_binary_r.lut",
        "MATPLOTLIB_binary.lut",
        "MATPLOTLIB_Blues_r.lut",
        "MATPLOTLIB_Blues.lut",
        "MATPLOTLIB_bone_r.lut",
        "MATPLOTLIB_bone.lut",
        "MATPLOTLIB_BrBG_r.lut",
        "MATPLOTLIB_BrBG.lut",
        "MATPLOTLIB_brg_r.lut",
        "MATPLOTLIB_brg.lut",
        "MATPLOTLIB_BuGn_r.lut",
        "MATPLOTLIB_BuGn.lut",
        "MATPLOTLIB_BuPu_r.lut",
        "MATPLOTLIB_BuPu.lut",
        "MATPLOTLIB_bwr_r.lut",
        "MATPLOTLIB_bwr.lut",
        "MATPLOTLIB_cividis_r.lut",
        "MATPLOTLIB_cividis.lut",
        "MATPLOTLIB_CMRmap_r.lut",
        "MATPLOTLIB_CMRmap.lut",
        "MATPLOTLIB_cool_r.lut",
        "MATPLOTLIB_cool.lut",
        "MATPLOTLIB_coolwarm_r.lut",
        "MATPLOTLIB_coolwarm.lut",
        "MATPLOTLIB_copper_r.lut",
        "MATPLOTLIB_copper.lut",
        "MATPLOTLIB_cubehelix_r.lut",
        "MATPLOTLIB_cubehelix.lut",
        "MATPLOTLIB_Dark2_r.lut",
        "MATPLOTLIB_Dark2.lut",
        "MATPLOTLIB_flag_r.lut",
        "MATPLOTLIB_flag.lut",
        "MATPLOTLIB_gist_earth_r.lut",
        "MATPLOTLIB_gist_earth.lut",
        "MATPLOTLIB_gist_gray_r.lut",
        "MATPLOTLIB_gist_gray.lut",
        "MATPLOTLIB_gist_grey_r.lut",
        "MATPLOTLIB_gist_grey.lut",
        "MATPLOTLIB_gist_heat_r.lut",
        "MATPLOTLIB_gist_heat.lut",
        "MATPLOTLIB_gist_ncar_r.lut",
        "MATPLOTLIB_gist_ncar.lut",
        "MATPLOTLIB_gist_rainbow_r.lut",
        "MATPLOTLIB_gist_rainbow.lut",
        "MATPLOTLIB_gist_stern_r.lut",
        "MATPLOTLIB_gist_stern.lut",
        "MATPLOTLIB_gist_yarg_r.lut",
        "MATPLOTLIB_gist_yarg.lut",
        "MATPLOTLIB_gist_yerg_r.lut",
        "MATPLOTLIB_gist_yerg.lut",
        "MATPLOTLIB_GnBu_r.lut",
        "MATPLOTLIB_GnBu.lut",
        "MATPLOTLIB_gnuplot_r.lut",
        "MATPLOTLIB_gnuplot.lut",
        "MATPLOTLIB_gnuplot2_r.lut",
        "MATPLOTLIB_gnuplot2.lut",
        "MATPLOTLIB_gray_r.lut",
        "MATPLOTLIB_gray.lut",
        "MATPLOTLIB_Grays_r.lut",
        "MATPLOTLIB_Grays.lut",
        "MATPLOTLIB_Greens_r.lut",
        "MATPLOTLIB_Greens.lut",
        "MATPLOTLIB_grey_r.lut",
        "MATPLOTLIB_grey.lut",
        "MATPLOTLIB_Greys_r.lut",
        "MATPLOTLIB_Greys.lut",
        "MATPLOTLIB_hot_r.lut",
        "MATPLOTLIB_hot.lut",
        "MATPLOTLIB_hsv_r.lut",
        "MATPLOTLIB_hsv.lut",
        "MATPLOTLIB_inferno_r.lut",
        "MATPLOTLIB_inferno.lut",
        "MATPLOTLIB_jet_r.lut",
        "MATPLOTLIB_jet.lut",
        "MATPLOTLIB_magma_r.lut",
        "MATPLOTLIB_magma.lut",
        "MATPLOTLIB_managua_r.lut",
        "MATPLOTLIB_managua.lut",
        "MATPLOTLIB_nipy_spectral_r.lut",
        "MATPLOTLIB_nipy_spectral.lut",
        "MATPLOTLIB_ocean_r.lut",
        "MATPLOTLIB_ocean.lut",
        "MATPLOTLIB_Oranges_r.lut",
        "MATPLOTLIB_Oranges.lut",
        "MATPLOTLIB_OrRd_r.lut",
        "MATPLOTLIB_OrRd.lut",
        "MATPLOTLIB_Paired_r.lut",
        "MATPLOTLIB_Paired.lut",
        "MATPLOTLIB_Pastel1_r.lut",
        "MATPLOTLIB_Pastel1.lut",
        "MATPLOTLIB_Pastel2_r.lut",
        "MATPLOTLIB_Pastel2.lut",
        "MATPLOTLIB_pink_r.lut",
        "MATPLOTLIB_pink.lut",
        "MATPLOTLIB_PiYG_r.lut",
        "MATPLOTLIB_PiYG.lut",
        "MATPLOTLIB_plasma_r.lut",
        "MATPLOTLIB_plasma.lut",
        "MATPLOTLIB_PRGn_r.lut",
        "MATPLOTLIB_PRGn.lut",
        "MATPLOTLIB_prism_r.lut",
        "MATPLOTLIB_prism.lut",
        "MATPLOTLIB_PuBu_r.lut",
        "MATPLOTLIB_PuBu.lut",
        "MATPLOTLIB_PuBuGn_r.lut",
        "MATPLOTLIB_PuBuGn.lut",
        "MATPLOTLIB_PuOr_r.lut",
        "MATPLOTLIB_PuOr.lut",
        "MATPLOTLIB_PuRd_r.lut",
        "MATPLOTLIB_PuRd.lut",
        "MATPLOTLIB_Purples_r.lut",
        "MATPLOTLIB_Purples.lut",
        "MATPLOTLIB_rainbow_r.lut",
        "MATPLOTLIB_rainbow.lut",
        "MATPLOTLIB_RdBu_r.lut",
        "MATPLOTLIB_RdBu.lut",
        "MATPLOTLIB_RdGy_r.lut",
        "MATPLOTLIB_RdGy.lut",
        "MATPLOTLIB_RdPu_r.lut",
        "MATPLOTLIB_RdPu.lut",
        "MATPLOTLIB_RdYlBu_r.lut",
        "MATPLOTLIB_RdYlBu.lut",
        "MATPLOTLIB_RdYlGn_r.lut",
        "MATPLOTLIB_RdYlGn.lut",
        "MATPLOTLIB_Reds_r.lut",
        "MATPLOTLIB_Reds.lut",
        "MATPLOTLIB_seismic_r.lut",
        "MATPLOTLIB_seismic.lut",
        "MATPLOTLIB_Set1_r.lut",
        "MATPLOTLIB_Set1.lut",
        "MATPLOTLIB_Set2_r.lut",
        "MATPLOTLIB_Set2.lut",
        "MATPLOTLIB_Set3_r.lut",
        "MATPLOTLIB_Set3.lut",
        "MATPLOTLIB_Spectral_r.lut",
        "MATPLOTLIB_Spectral.lut",
        "MATPLOTLIB_spring_r.lut",
        "MATPLOTLIB_spring.lut",
        "MATPLOTLIB_summer_r.lut",
        "MATPLOTLIB_summer.lut",
        "MATPLOTLIB_tab10_r.lut",
        "MATPLOTLIB_tab10.lut",
        "MATPLOTLIB_tab20_r.lut",
        "MATPLOTLIB_tab20.lut",
        "MATPLOTLIB_tab20b_r.lut",
        "MATPLOTLIB_tab20b.lut",
        "MATPLOTLIB_tab20c_r.lut",
        "MATPLOTLIB_tab20c.lut",
        "MATPLOTLIB_terrain_r.lut",
        "MATPLOTLIB_terrain.lut",
        "MATPLOTLIB_turbo_r.lut",
        "MATPLOTLIB_turbo.lut",
        "MATPLOTLIB_twilight_r.lut",
        "MATPLOTLIB_twilight.lut",
        "MATPLOTLIB_twilight_shifted_r.lut",
        "MATPLOTLIB_twilight_shifted.lut",
        "MATPLOTLIB_viridis_r.lut",
        "MATPLOTLIB_viridis.lut",
        "MATPLOTLIB_winter_r.lut",
        "MATPLOTLIB_winter.lut",
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

    pub fn all_luts(&self) -> Vec<String> {
        let mut luts: Vec<String> = EMBEDDED_LUTS.keys().map(|&name| name.to_string()).collect();

        // Add custom LUTs
        if let Ok(custom_luts) = self.all_custom_luts() {
            luts.extend(custom_luts);
        }

        luts.sort();
        luts
    }

    pub fn get(&self, name: &str) -> io::Result<LutData> {
        // Try to load from embedded LUTs first
        if let Some(&buffer) = EMBEDDED_LUTS.get(name) {
            // Each color component should be 256 bytes
            if buffer.len() != 768 {
                // 256 * 3 (RGB)
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid LUT file size",
                ));
            }

            return Ok(LutData::from_bytes(name.to_string(), buffer.as_slice())?);
        }

        // If not found in embedded LUTs, try to load as a custom LUT
        self.get_custom(name)
    }

    fn lut_dir() -> io::Result<std::path::PathBuf> {
        let home_dir = home_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not find home directory")
        })?;

        let lut_dir = home_dir.join("sim-pix").join("LUTs");
        Ok(lut_dir)
    }

    pub fn save_custom(&self, name: &str, lut_data: &LutData) -> io::Result<()> {
        // Create LUTs directory if it doesn't exist
        let lut_dir = Self::lut_dir()?;
        if !lut_dir.exists() {
            std::fs::create_dir_all(&lut_dir)?;
        }

        // Save the LUT file
        let file_path = lut_dir.join(format!("{}.lut", name));
        std::fs::write(file_path, lut_data.clone().into_bytes())?;

        Ok(())
    }

    pub fn all_custom_luts(&self) -> io::Result<Vec<String>> {
        let lut_dir = Self::lut_dir()?;
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

    pub fn get_custom(&self, name: &str) -> io::Result<LutData> {
        let file_path = Self::lut_dir()?.join(format!("{}.lut", name));
        let data = std::fs::read(file_path)?;
        LutData::from_bytes(name.to_string(), &data)
    }

    pub fn get_default(&self) -> LutData {
        let lut_data = self.get("MATPLOTLIB_bone_r").unwrap();
        lut_data
    }
}

impl Default for LutManager {
    fn default() -> Self {
        Self::new()
    }
}
