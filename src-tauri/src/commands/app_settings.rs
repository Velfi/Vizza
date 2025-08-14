use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use toml;
use wgpu;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TextureFiltering {
    Linear,
    Nearest,
    Lanczos,
}

impl From<TextureFiltering> for wgpu::FilterMode {
    fn from(filtering: TextureFiltering) -> Self {
        match filtering {
            TextureFiltering::Linear => wgpu::FilterMode::Linear,
            TextureFiltering::Nearest => wgpu::FilterMode::Nearest,
            TextureFiltering::Lanczos => wgpu::FilterMode::Linear, // Lanczos uses Linear as base, custom shader handles the rest
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    // Display Settings
    pub default_fps_limit: u32,
    pub default_fps_limit_enabled: bool,
    pub texture_filtering: TextureFiltering,

    // Window Settings
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,

    // UI Settings
    pub ui_scale: f32,
    pub auto_hide_ui: bool,
    pub auto_hide_delay: u32,
    pub menu_position: String,

    // Camera Settings
    pub default_camera_sensitivity: f32,
}

impl AppSettings {
    pub(crate) fn load_from_file() -> Result<Self, String> {
        let settings_path = get_settings_path();
        if !settings_path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(settings_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse settings file: {}", e))
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // Display Settings
            default_fps_limit: 60,
            default_fps_limit_enabled: false,
            texture_filtering: TextureFiltering::Linear,

            // Window Settings
            window_width: 1200,
            window_height: 800,
            window_maximized: false,

            // UI Settings
            ui_scale: 1.0,
            auto_hide_ui: true,
            auto_hide_delay: 3000,
            menu_position: "middle".to_string(),

            // Camera Settings
            default_camera_sensitivity: 1.0,
        }
    }
}

fn get_settings_path() -> PathBuf {
    let home_dir = home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join(env!("CARGO_PKG_NAME")).join("settings.toml")
}

pub(crate) fn get_settings_dir() -> PathBuf {
    let home_dir = home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join(env!("CARGO_PKG_NAME"))
}

#[tauri::command]
pub async fn get_app_settings() -> Result<AppSettings, String> {
    let settings_path = get_settings_path();

    if !settings_path.exists() {
        // Return default settings if file doesn't exist
        return Ok(AppSettings::default());
    }

    match fs::read_to_string(&settings_path) {
        Ok(content) => match toml::from_str::<AppSettings>(&content) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                tracing::warn!("Failed to parse settings file, using defaults: {}", e);
                Ok(AppSettings::default())
            }
        },
        Err(e) => {
            tracing::warn!("Failed to read settings file, using defaults: {}", e);
            Ok(AppSettings::default())
        }
    }
}

#[tauri::command]
pub async fn save_app_settings(settings: AppSettings) -> Result<String, String> {
    let settings_dir = get_settings_dir();
    let settings_path = get_settings_path();

    // Create settings directory if it doesn't exist
    if !settings_dir.exists() {
        if let Err(e) = fs::create_dir_all(&settings_dir) {
            return Err(format!("Failed to create settings directory: {}", e));
        }
    }

    // Serialize settings to TOML
    let toml_content = match toml::to_string_pretty(&settings) {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to serialize settings: {}", e)),
    };

    // Write to file
    match fs::write(&settings_path, toml_content) {
        Ok(_) => {
            tracing::debug!("App settings saved successfully to {:?}", settings_path);
            Ok("Settings saved successfully".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to save settings: {}", e);
            Err(format!("Failed to save settings: {}", e))
        }
    }
}

#[tauri::command]
pub async fn reset_app_settings() -> Result<String, String> {
    let settings_path = get_settings_path();

    // If settings file exists, delete it
    if settings_path.exists() {
        if let Err(e) = fs::remove_file(&settings_path) {
            return Err(format!("Failed to delete settings file: {}", e));
        }
    }

    tracing::debug!("App settings reset to defaults");
    Ok("Settings reset to defaults".to_string())
}

#[tauri::command]
pub async fn get_settings_file_path() -> Result<String, String> {
    let path = get_settings_path();
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn set_webview_zoom(app: tauri::AppHandle, zoom_factor: f64) -> Result<String, String> {
    // Get the main window
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Set the webview zoom factor
    window
        .set_zoom(zoom_factor)
        .map_err(|e| format!("Failed to set zoom factor: {}", e))?;

    tracing::debug!("Webview zoom factor set to: {}", zoom_factor);
    Ok("Zoom factor set successfully".to_string())
}

#[tauri::command]
pub async fn apply_window_settings(app: tauri::AppHandle) -> Result<String, String> {
    // Load current settings
    let settings = get_app_settings().await?;

    // Get the main window
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Apply window size only (not maximized state)
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: settings.window_width as f64,
            height: settings.window_height as f64,
        }))
        .map_err(|e| format!("Failed to set window size: {}", e))?;

    tracing::debug!(
        "Window size applied: {}x{}",
        settings.window_width,
        settings.window_height
    );
    Ok("Window size applied successfully".to_string())
}

#[tauri::command]
pub async fn apply_window_settings_on_startup(app: tauri::AppHandle) -> Result<String, String> {
    // Load current settings
    let settings = get_app_settings().await?;

    // Get the main window
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Apply maximized state or size (only on startup)
    if settings.window_maximized {
        window
            .maximize()
            .map_err(|e| format!("Failed to maximize window: {}", e))?;
    } else {
        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: settings.window_width as f64,
                height: settings.window_height as f64,
            }))
            .map_err(|e| format!("Failed to set window size: {}", e))?;
    }

    tracing::debug!(
        "Window settings applied on startup: {}x{}, maximized: {}",
        settings.window_width,
        settings.window_height,
        settings.window_maximized
    );
    Ok("Window settings applied successfully".to_string())
}

#[tauri::command]
pub async fn get_current_window_size(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    // Get the main window
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Get current window size in logical pixels
    let size = window
        .inner_size()
        .map_err(|e| format!("Failed to get window size: {}", e))?;

    // Convert physical pixels to logical pixels
    let scale_factor = window
        .scale_factor()
        .map_err(|e| format!("Failed to get scale factor: {}", e))?;

    let logical_width = (size.width as f64 / scale_factor) as u32;
    let logical_height = (size.height as f64 / scale_factor) as u32;

    // Get current maximized state
    let is_maximized = window
        .is_maximized()
        .map_err(|e| format!("Failed to get maximized state: {}", e))?;

    let result = serde_json::json!({
        "width": logical_width,
        "height": logical_height,
        "maximized": is_maximized
    });

    tracing::debug!(
        "Current window size: {}x{} (logical), {}x{} (physical), scale: {}",
        logical_width,
        logical_height,
        size.width,
        size.height,
        scale_factor
    );
    Ok(result)
}
