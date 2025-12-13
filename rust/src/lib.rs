//! pm_encoder - High-performance context serializer (Rust Engine)
//!
//! This library provides the core logic for serializing project files into
//! the Plus/Minus format. It is designed to be consumed by:
//! - The CLI binary (src/bin/main.rs)
//! - WASM bindings (future)
//! - Python bindings via PyO3 (future)
//!
//! # Architecture
//!
//! This crate follows the "Library-First" pattern:
//! - **lib.rs** (this file): Pure logic, no CLI concerns
//! - **bin/main.rs**: Thin wrapper that calls the library
//!
//! This separation allows the core logic to be reusable across different
//! interfaces without coupling to any specific runtime environment.

/// Configuration for the encoder
pub struct EncoderConfig {
    /// Enable truncation of large files
    pub truncate: bool,
    // Future fields:
    // pub max_lines: usize,
    // pub truncate_mode: TruncateMode,
    // pub sort_by: SortBy,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self { truncate: false }
    }
}

/// Returns the version of the pm_encoder library
pub fn version() -> &'static str {
    "0.1.0"
}

/// Serialize a project directory into the Plus/Minus format
///
/// # Arguments
///
/// * `root` - Path to the project root directory
///
/// # Returns
///
/// * `Ok(String)` - The serialized output
/// * `Err(String)` - Error message if serialization fails
///
/// # Example
///
/// ```
/// use pm_encoder::serialize_project;
///
/// let result = serialize_project(".");
/// assert!(result.is_ok());
/// ```
pub fn serialize_project(root: &str) -> Result<String, String> {
    // Placeholder implementation
    // Future: Walk directory tree, apply filters, generate Plus/Minus format
    Ok(format!("Scanning {}... (Rust Engine v{})", root, version()))
}

/// Serialize a project with custom configuration
///
/// # Arguments
///
/// * `root` - Path to the project root directory
/// * `config` - Encoder configuration
///
/// # Returns
///
/// * `Ok(String)` - The serialized output
/// * `Err(String)` - Error message if serialization fails
pub fn serialize_project_with_config(
    root: &str,
    config: &EncoderConfig,
) -> Result<String, String> {
    let truncate_msg = if config.truncate {
        " (truncation enabled)"
    } else {
        ""
    };

    Ok(format!(
        "Scanning {}... (Rust Engine v{}){}",
        root,
        version(),
        truncate_msg
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }

    #[test]
    fn test_serialize_project() {
        let result = serialize_project(".");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Rust Engine"));
    }

    #[test]
    fn test_serialize_with_config() {
        let config = EncoderConfig { truncate: true };
        let result = serialize_project_with_config(".", &config);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("truncation enabled"));
    }

    #[test]
    fn test_default_config() {
        let config = EncoderConfig::default();
        assert_eq!(config.truncate, false);
    }
}
