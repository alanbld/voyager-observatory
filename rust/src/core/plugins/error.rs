//! Plugin Error Types
//!
//! Defines the error hierarchy for the Plugin Ecosystem.
//! Uses celestial terminology to maintain the Observatory metaphor.

use thiserror::Error;

/// Plugin ecosystem errors
#[derive(Debug, Error)]
pub enum PluginError {
    /// Security breach attempt detected
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),

    /// Plugin execution exceeded time limit
    #[error("Plugin execution timeout (>100ms)")]
    TimeoutExceeded,

    /// Plugin exceeded memory allocation limit
    #[error("Memory quota exceeded (>10MB)")]
    MemoryQuotaExceeded,

    /// Plugin API version doesn't match Observatory version
    #[error("API version mismatch: expected {expected}, got {actual}")]
    ApiVersionMismatch { expected: String, actual: String },

    /// Plugin manifest parsing failed
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    /// Plugin file not found
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    /// Lua runtime error
    #[error("Lua runtime error: {0}")]
    LuaError(String),

    /// Plugin registration failed
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    /// I/O error during plugin loading
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

#[cfg(feature = "plugins")]
impl From<mlua::Error> for PluginError {
    fn from(err: mlua::Error) -> Self {
        let msg = err.to_string();

        // Classify the error based on message content
        if msg.contains("timeout") || msg.contains("Timeout") {
            PluginError::TimeoutExceeded
        } else if msg.contains("memory") || msg.contains("Memory") {
            PluginError::MemoryQuotaExceeded
        } else if msg.contains("nil value")
            && (msg.contains("io") || msg.contains("os") || msg.contains("debug"))
        {
            PluginError::SandboxViolation(format!("Attempted access to disabled library: {}", msg))
        } else {
            PluginError::LuaError(msg)
        }
    }
}
