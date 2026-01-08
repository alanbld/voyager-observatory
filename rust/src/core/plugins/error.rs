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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_violation_display() {
        let err = PluginError::SandboxViolation("path escape attempt".to_string());
        assert_eq!(err.to_string(), "Sandbox violation: path escape attempt");
    }

    #[test]
    fn test_timeout_exceeded_display() {
        let err = PluginError::TimeoutExceeded;
        assert_eq!(err.to_string(), "Plugin execution timeout (>100ms)");
    }

    #[test]
    fn test_memory_quota_exceeded_display() {
        let err = PluginError::MemoryQuotaExceeded;
        assert_eq!(err.to_string(), "Memory quota exceeded (>10MB)");
    }

    #[test]
    fn test_api_version_mismatch_display() {
        let err = PluginError::ApiVersionMismatch {
            expected: "1.0".to_string(),
            actual: "2.0".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "API version mismatch: expected 1.0, got 2.0"
        );
    }

    #[test]
    fn test_invalid_manifest_display() {
        let err = PluginError::InvalidManifest("missing name field".to_string());
        assert_eq!(err.to_string(), "Invalid manifest: missing name field");
    }

    #[test]
    fn test_plugin_not_found_display() {
        let err = PluginError::PluginNotFound("my-plugin.lua".to_string());
        assert_eq!(err.to_string(), "Plugin not found: my-plugin.lua");
    }

    #[test]
    fn test_lua_error_display() {
        let err = PluginError::LuaError("syntax error at line 5".to_string());
        assert_eq!(err.to_string(), "Lua runtime error: syntax error at line 5");
    }

    #[test]
    fn test_registration_failed_display() {
        let err = PluginError::RegistrationFailed("duplicate name".to_string());
        assert_eq!(err.to_string(), "Registration failed: duplicate name");
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let plugin_err: PluginError = io_err.into();
        assert!(plugin_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_plugin_result_type() {
        let success: PluginResult<i32> = Ok(42);
        assert_eq!(success.unwrap(), 42);

        let failure: PluginResult<i32> = Err(PluginError::TimeoutExceeded);
        assert!(failure.is_err());
    }

    #[test]
    fn test_error_debug_impl() {
        let err = PluginError::SandboxViolation("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("SandboxViolation"));
        assert!(debug_str.contains("test"));
    }
}
