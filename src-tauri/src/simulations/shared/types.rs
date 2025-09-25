use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageFitMode {
    Stretch,
    Center,
    FitH,
    FitV,
}

impl Default for ImageFitMode {
    fn default() -> Self {
        Self::Stretch
    }
}

impl ImageFitMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageFitMode::Stretch => "Stretch",
            ImageFitMode::Center => "Center",
            ImageFitMode::FitH => "Fit H",
            ImageFitMode::FitV => "Fit V",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Stretch" => Some(ImageFitMode::Stretch),
            "Center" => Some(ImageFitMode::Center),
            "Fit H" => Some(ImageFitMode::FitH),
            "Fit V" => Some(ImageFitMode::FitV),
            _ => None,
        }
    }
}

impl std::str::FromStr for ImageFitMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        // Accept exact display names
        match t {
            "Stretch" => return Ok(ImageFitMode::Stretch),
            "Center" => return Ok(ImageFitMode::Center),
            "Fit H" => return Ok(ImageFitMode::FitH),
            "Fit V" => return Ok(ImageFitMode::FitV),
            _ => {}
        }
        // Accept lowercase and compact variants
        let lc = t.to_lowercase();
        match lc.as_str() {
            "stretch" => Ok(ImageFitMode::Stretch),
            "center" => Ok(ImageFitMode::Center),
            "fit h" | "fith" => Ok(ImageFitMode::FitH),
            "fit v" | "fitv" => Ok(ImageFitMode::FitV),
            other => Err(format!("Invalid ImageFitMode: '{}'", other)),
        }
    }
}

/// Color mode for particle rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    Gray18,
    White,
    Black,
    ColorScheme,
}
