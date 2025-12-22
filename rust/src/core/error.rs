//! Error types for pm_encoder
//!
//! This module provides structured error handling using thiserror.

use thiserror::Error;
use std::path::PathBuf;

/// Result type alias for encoder operations
pub type Result<T> = std::result::Result<T, EncoderError>;

/// Errors that can occur during context serialization
#[derive(Error, Debug)]
pub enum EncoderError {
    /// IO error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Directory not found
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Invalid configuration
    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Lens not found
    #[error("Lens not found: {name}")]
    LensNotFound { name: String },

    /// Invalid zoom target
    #[error("Invalid zoom target: {target}")]
    InvalidZoomTarget { target: String },

    /// Budget exceeded
    #[error("Token budget exceeded: used {used}, budget {budget}")]
    BudgetExceeded { used: usize, budget: usize },

    /// XML generation error
    #[error("XML generation error: {message}")]
    XmlError { message: String },

    /// UTF-8 encoding error
    #[error("UTF-8 encoding error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<EncoderError>,
    },
}

impl EncoderError {
    /// Wrap an error with additional context
    pub fn with_context(self, context: impl Into<String>) -> Self {
        EncoderError::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Create an invalid config error
    pub fn invalid_config(message: impl Into<String>) -> Self {
        EncoderError::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create an XML error
    pub fn xml_error(message: impl Into<String>) -> Self {
        EncoderError::XmlError {
            message: message.into(),
        }
    }
}

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to an error
    fn context(self, ctx: impl Into<String>) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, ctx: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.with_context(ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = EncoderError::DirectoryNotFound {
            path: PathBuf::from("/tmp/missing"),
        };
        assert!(err.to_string().contains("/tmp/missing"));
    }

    #[test]
    fn test_error_with_context() {
        let err = EncoderError::invalid_config("bad value");
        let wrapped = err.with_context("loading config");
        assert!(wrapped.to_string().contains("loading config"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: EncoderError = io_err.into();
        assert!(matches!(err, EncoderError::Io(_)));
    }

    #[test]
    fn test_budget_exceeded() {
        let err = EncoderError::BudgetExceeded {
            used: 15000,
            budget: 10000,
        };
        assert!(err.to_string().contains("15000"));
        assert!(err.to_string().contains("10000"));
    }

    #[test]
    fn test_invalid_config_helper() {
        let err = EncoderError::invalid_config("missing field");
        assert!(err.to_string().contains("missing field"));
        assert!(matches!(err, EncoderError::InvalidConfig { .. }));
    }

    #[test]
    fn test_xml_error_helper() {
        let err = EncoderError::xml_error("invalid tag");
        assert!(err.to_string().contains("invalid tag"));
        assert!(matches!(err, EncoderError::XmlError { .. }));
    }

    #[test]
    fn test_result_ext_context() {
        let result: Result<()> = Err(EncoderError::invalid_config("test"));
        let with_ctx = result.context("during processing");
        assert!(with_ctx.is_err());
        let err = with_ctx.unwrap_err();
        assert!(err.to_string().contains("during processing"));
    }

    #[test]
    fn test_file_not_found_error() {
        let err = EncoderError::FileNotFound {
            path: PathBuf::from("missing.txt"),
        };
        assert!(err.to_string().contains("missing.txt"));
    }

    #[test]
    fn test_lens_not_found_error() {
        let err = EncoderError::LensNotFound {
            name: "unknown_lens".to_string(),
        };
        assert!(err.to_string().contains("unknown_lens"));
    }

    #[test]
    fn test_invalid_zoom_target_error() {
        let err = EncoderError::InvalidZoomTarget {
            target: "bad=target".to_string(),
        };
        assert!(err.to_string().contains("bad=target"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err: serde_json::Error = serde_json::from_str::<i32>("not json").unwrap_err();
        let err: EncoderError = json_err.into();
        assert!(matches!(err, EncoderError::Json(_)));
    }
}
