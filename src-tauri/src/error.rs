use serde_json;
use std::io;
use std::path::PathBuf;
use wgpu;

/// Main error type for the application
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Simulation error: {0}")]
    Simulation(#[from] SimulationError),

    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),

    #[error("Command error: {0}")]
    Command(#[from] CommandError),

    #[error("Preset error: {0}")]
    Preset(#[from] PresetError),

    #[error("LUT error: {0}")]
    Lut(#[from] LutError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Window error: {0}")]
    Window(String),
}

/// Simulation-specific errors
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("GPU operation failed: {0}")]
    Gpu(Box<dyn std::error::Error>),

    #[error("Invalid setting '{setting_name}': {message}")]
    InvalidSetting {
        setting_name: String,
        message: String,
    },

    #[error("Simulation not running")]
    NotRunning,

    #[error("Unsupported operation for this simulation type")]
    UnsupportedOperation,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Simulation initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Unknown simulation type: {0}")]
    UnknownType(String),

    #[error("Agent count update failed: {0}")]
    AgentCountUpdateFailed(String),

    #[error("Camera operation failed: {0}")]
    CameraOperationFailed(String),

    #[error("Mouse interaction failed: {0}")]
    MouseInteractionFailed(String),

    #[error("Settings application failed: {0}")]
    SettingsApplicationFailed(String),

    #[error("State reset failed: {0}")]
    StateResetFailed(String),

    #[error("{0}")]
    LutError(#[from] LutError),

    #[error("Window error: {0}")]
    Window(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(
        "Buffer too large: requested {requested} bytes, maximum available {max_available} bytes"
    )]
    BufferTooLarge { requested: u64, max_available: u64 },
}

/// GPU-related errors
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("WGPU error: {0}")]
    Wgpu(#[from] wgpu::Error),

    #[error("Device creation failed: {0}")]
    DeviceCreationFailed(String),

    #[error("Surface creation failed: {0}")]
    SurfaceCreationFailed(String),

    #[error("Adapter not found")]
    AdapterNotFound,

    #[error("Surface configuration failed: {0}")]
    SurfaceConfigurationFailed(String),

    #[error("Buffer creation failed: {0}")]
    BufferCreationFailed(String),

    #[error("Texture creation failed: {0}")]
    TextureCreationFailed(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilationFailed(String),

    #[error("Pipeline creation failed: {0}")]
    PipelineCreationFailed(String),

    #[error("Bind group creation failed: {0}")]
    BindGroupCreationFailed(String),

    #[error("Render pass creation failed: {0}")]
    RenderPassCreationFailed(String),

    #[error("Command encoding failed: {0}")]
    CommandEncodingFailed(String),

    #[error("Queue submission failed: {0}")]
    QueueSubmissionFailed(String),

    #[error("Surface presentation failed: {0}")]
    SurfacePresentationFailed(String),
}

/// Command-related errors
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Invalid command parameters: {0}")]
    InvalidParameters(String),

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Command not supported: {0}")]
    NotSupported(String),

    #[error("Command timeout: {0}")]
    Timeout(String),

    #[error("Command cancelled: {0}")]
    Cancelled(String),

    #[error("Command validation failed: {0}")]
    ValidationFailed(String),

    #[error("Command state error: {0}")]
    StateError(String),
}

/// Preset-related errors
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error("Preset not found: {0}")]
    NotFound(String),

    #[error("Preset already exists: {0}")]
    AlreadyExists(String),

    #[error("Preset loading failed: {0}")]
    LoadingFailed(String),

    #[error("Preset saving failed: {0}")]
    SavingFailed(String),

    #[error("Preset deletion failed: {0}")]
    DeletionFailed(String),

    #[error("Preset validation failed: {0}")]
    ValidationFailed(String),

    #[error("Preset serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Preset deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Preset file error: {path}: {error}")]
    FileError { path: PathBuf, error: String },

    #[error("Preset format error: {0}")]
    FormatError(String),

    #[error("Preset compatibility error: {0}")]
    CompatibilityError(String),

    #[error("Preset directory error: {0}")]
    DirectoryError(String),

    #[error("Simulation error: {0}")]
    SimulationError(String),
}

/// LUT-related errors
#[derive(Debug, thiserror::Error)]
pub enum LutError {
    #[error("LUT not found: {0}")]
    NotFound(String),

    #[error("LUT loading failed: {0}")]
    LoadingFailed(String),

    #[error("LUT saving failed: {0}")]
    SavingFailed(String),

    #[error("LUT format error: {0}")]
    FormatError(String),

    #[error("LUT validation failed: {0}")]
    ValidationFailed(String),

    #[error("LUT file error: {path}: {error}")]
    FileError { path: PathBuf, error: String },

    #[error("LUT data error: {0}")]
    DataError(String),

    #[error("LUT size error: expected {expected}, got {actual}")]
    SizeError { expected: usize, actual: usize },

    #[error("LUT color space error: {0}")]
    ColorSpaceError(String),

    #[error("LUT application failed: {0}")]
    ApplicationFailed(String),

    #[error("Custom LUT creation failed: {0}")]
    CustomCreationFailed(String),

    #[error("Gradient LUT generation failed: {0}")]
    GradientGenerationFailed(String),
}

// Type aliases for convenience
pub type AppResult<T> = Result<T, AppError>;
pub type SimulationResult<T> = Result<T, SimulationError>;
pub type PresetResult<T> = Result<T, PresetError>;
pub type LutResult<T> = Result<T, LutError>;

// Conversion traits for easy error conversion
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Unknown(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Unknown(s.to_string())
    }
}

impl From<String> for SimulationError {
    fn from(s: String) -> Self {
        SimulationError::InvalidParameter(s)
    }
}

impl From<&str> for SimulationError {
    fn from(s: &str) -> Self {
        SimulationError::InvalidParameter(s.to_string())
    }
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        CommandError::ExecutionFailed(s)
    }
}

impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        CommandError::ExecutionFailed(s.to_string())
    }
}

impl From<String> for PresetError {
    fn from(s: String) -> Self {
        PresetError::LoadingFailed(s)
    }
}

impl From<&str> for PresetError {
    fn from(s: &str) -> Self {
        PresetError::LoadingFailed(s.to_string())
    }
}

impl From<String> for LutError {
    fn from(s: String) -> Self {
        LutError::LoadingFailed(s)
    }
}

impl From<&str> for LutError {
    fn from(s: &str) -> Self {
        LutError::LoadingFailed(s.to_string())
    }
}

// Helper functions for common error patterns
impl AppError {
    /// Create a simulation error with context
    pub fn simulation_error<T: Into<SimulationError>>(error: T) -> Self {
        AppError::Simulation(error.into())
    }

    /// Create a GPU error with context
    pub fn gpu_error<T: Into<GpuError>>(error: T) -> Self {
        AppError::Gpu(error.into())
    }

    /// Create a command error with context
    pub fn command_error<T: Into<CommandError>>(error: T) -> Self {
        AppError::Command(error.into())
    }

    /// Create a preset error with context
    pub fn preset_error<T: Into<PresetError>>(error: T) -> Self {
        AppError::Preset(error.into())
    }

    /// Create a LUT error with context
    pub fn lut_error<T: Into<LutError>>(error: T) -> Self {
        AppError::Lut(error.into())
    }
}

impl SimulationError {
    /// Create an invalid setting error
    pub fn invalid_setting(setting_name: &str, message: &str) -> Self {
        SimulationError::InvalidSetting {
            setting_name: setting_name.to_string(),
            message: message.to_string(),
        }
    }

    /// Create an unknown simulation type error
    pub fn unknown_type(simulation_type: &str) -> Self {
        SimulationError::UnknownType(simulation_type.to_string())
    }
}

impl PresetError {
    /// Create a file error with path and error message
    pub fn file_error(path: PathBuf, error: &str) -> Self {
        PresetError::FileError {
            path,
            error: error.to_string(),
        }
    }
}

impl LutError {
    /// Create a file error with path and error message
    pub fn file_error(path: PathBuf, error: &str) -> Self {
        LutError::FileError {
            path,
            error: error.to_string(),
        }
    }

    /// Create a size error
    pub fn size_error(expected: usize, actual: usize) -> Self {
        LutError::SizeError { expected, actual }
    }
}
