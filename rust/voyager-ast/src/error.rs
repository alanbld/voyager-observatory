//! Error types for voyager-ast
//!
//! Following the "Telescope, Not Compiler" philosophy, many errors are
//! recoverable and result in partial output rather than total failure.

use crate::ir::{File, LanguageId};
use thiserror::Error;

/// Errors from AST operations
#[derive(Error, Debug, Clone)]
pub enum AstError {
    /// File not found or couldn't be read
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Symbol not found in file
    #[error("Symbol '{symbol}' not found in file '{file}'")]
    SymbolNotFound { file: String, symbol: String },

    /// Language not supported by any adapter
    #[error("Unsupported language: {0:?}")]
    UnsupportedLanguage(LanguageId),

    /// Parse error occurred, but partial results may be available
    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        /// Partial parse result (if any structure could be recovered)
        partial: Option<Box<File>>,
    },

    /// I/O error during file operations
    #[error("I/O error: {0}")]
    IoError(String),

    /// Invalid configuration or options
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Tree-sitter specific error
    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),

    /// Internal error (should not happen in normal operation)
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl AstError {
    /// Check if this error has partial results available
    pub fn has_partial(&self) -> bool {
        matches!(self, AstError::ParseError { partial: Some(_), .. })
    }

    /// Extract partial results if available
    pub fn take_partial(self) -> Option<File> {
        match self {
            AstError::ParseError { partial: Some(file), .. } => Some(*file),
            _ => None,
        }
    }

    /// Create a parse error with partial results
    pub fn parse_error_with_partial(message: impl Into<String>, file: File) -> Self {
        AstError::ParseError {
            message: message.into(),
            partial: Some(Box::new(file)),
        }
    }

    /// Create a simple parse error without partial results
    pub fn parse_error(message: impl Into<String>) -> Self {
        AstError::ParseError {
            message: message.into(),
            partial: None,
        }
    }
}

/// Result type alias for AstError
pub type Result<T> = std::result::Result<T, AstError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Span;

    #[test]
    fn test_error_display() {
        let err = AstError::FileNotFound("test.rs".to_string());
        assert!(err.to_string().contains("test.rs"));

        let err = AstError::SymbolNotFound {
            file: "main.rs".to_string(),
            symbol: "foo".to_string(),
        };
        assert!(err.to_string().contains("foo"));
        assert!(err.to_string().contains("main.rs"));
    }

    #[test]
    fn test_partial_results() {
        let file = File::new("test.rs".to_string(), LanguageId::Rust);
        let err = AstError::parse_error_with_partial("syntax error", file);

        assert!(err.has_partial());
        let partial = err.take_partial().unwrap();
        assert_eq!(partial.path, "test.rs");
    }

    #[test]
    fn test_no_partial() {
        let err = AstError::parse_error("syntax error");
        assert!(!err.has_partial());
        assert!(err.take_partial().is_none());
    }

    // =========================================================================
    // AstError Variant Tests
    // =========================================================================

    #[test]
    fn test_file_not_found_error() {
        let err = AstError::FileNotFound("/path/to/missing.rs".to_string());
        assert!(err.to_string().contains("File not found"));
        assert!(err.to_string().contains("/path/to/missing.rs"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_symbol_not_found_error() {
        let err = AstError::SymbolNotFound {
            file: "src/lib.rs".to_string(),
            symbol: "calculate_total".to_string(),
        };
        assert!(err.to_string().contains("Symbol"));
        assert!(err.to_string().contains("calculate_total"));
        assert!(err.to_string().contains("src/lib.rs"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_unsupported_language_error() {
        let err = AstError::UnsupportedLanguage(LanguageId::Unknown);
        assert!(err.to_string().contains("Unsupported language"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_parse_error_simple() {
        let err = AstError::ParseError {
            message: "unexpected token".to_string(),
            partial: None,
        };
        assert!(err.to_string().contains("Parse error"));
        assert!(err.to_string().contains("unexpected token"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_io_error() {
        let err = AstError::IoError("permission denied".to_string());
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("permission denied"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_invalid_config_error() {
        let err = AstError::InvalidConfig("max_depth must be positive".to_string());
        assert!(err.to_string().contains("Invalid configuration"));
        assert!(err.to_string().contains("max_depth"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_tree_sitter_error() {
        let err = AstError::TreeSitterError("failed to parse".to_string());
        assert!(err.to_string().contains("Tree-sitter error"));
        assert!(err.to_string().contains("failed to parse"));
        assert!(!err.has_partial());
    }

    #[test]
    fn test_internal_error() {
        let err = AstError::InternalError("assertion failed".to_string());
        assert!(err.to_string().contains("Internal error"));
        assert!(err.to_string().contains("assertion failed"));
        assert!(!err.has_partial());
    }

    // =========================================================================
    // Helper Method Tests
    // =========================================================================

    #[test]
    fn test_parse_error_with_partial_helper() {
        let file = File::new("test.py".to_string(), LanguageId::Python);
        let err = AstError::parse_error_with_partial("incomplete parse", file);

        assert!(err.has_partial());
        if let AstError::ParseError { message, partial } = &err {
            assert_eq!(message, "incomplete parse");
            assert!(partial.is_some());
        }
    }

    #[test]
    fn test_parse_error_helper() {
        let err = AstError::parse_error("simple error");

        assert!(!err.has_partial());
        if let AstError::ParseError { message, partial } = &err {
            assert_eq!(message, "simple error");
            assert!(partial.is_none());
        }
    }

    #[test]
    fn test_take_partial_consumes_error() {
        let file = File::new("consumed.rs".to_string(), LanguageId::Rust);
        let err = AstError::parse_error_with_partial("will be consumed", file);

        // Take partial consumes the error
        let partial = err.take_partial();
        assert!(partial.is_some());
        assert_eq!(partial.unwrap().path, "consumed.rs");
    }

    #[test]
    fn test_take_partial_on_non_parse_errors() {
        // FileNotFound
        let err = AstError::FileNotFound("test.rs".to_string());
        assert!(err.take_partial().is_none());

        // SymbolNotFound
        let err = AstError::SymbolNotFound {
            file: "test.rs".to_string(),
            symbol: "foo".to_string(),
        };
        assert!(err.take_partial().is_none());

        // IoError
        let err = AstError::IoError("error".to_string());
        assert!(err.take_partial().is_none());

        // InvalidConfig
        let err = AstError::InvalidConfig("error".to_string());
        assert!(err.take_partial().is_none());

        // TreeSitterError
        let err = AstError::TreeSitterError("error".to_string());
        assert!(err.take_partial().is_none());

        // InternalError
        let err = AstError::InternalError("error".to_string());
        assert!(err.take_partial().is_none());

        // UnsupportedLanguage
        let err = AstError::UnsupportedLanguage(LanguageId::Rust);
        assert!(err.take_partial().is_none());
    }

    // =========================================================================
    // Clone and Debug Tests
    // =========================================================================

    #[test]
    fn test_error_clone() {
        let err = AstError::FileNotFound("clone_test.rs".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    #[test]
    fn test_error_debug() {
        let err = AstError::IoError("debug test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("IoError"));
        assert!(debug_str.contains("debug test"));
    }

    #[test]
    fn test_parse_error_clone_with_partial() {
        let file = File::new("clone.rs".to_string(), LanguageId::Rust);
        let err = AstError::parse_error_with_partial("clone test", file);
        let cloned = err.clone();

        assert!(cloned.has_partial());
    }

    // =========================================================================
    // Result Type Tests
    // =========================================================================

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(AstError::FileNotFound("test.rs".to_string()));
        assert!(result.is_err());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_strings() {
        let err = AstError::FileNotFound("".to_string());
        assert!(err.to_string().contains("File not found"));

        let err = AstError::parse_error("");
        assert!(err.to_string().contains("Parse error"));
    }

    #[test]
    fn test_unicode_in_errors() {
        let err = AstError::FileNotFound("文件.rs".to_string());
        assert!(err.to_string().contains("文件.rs"));

        let err = AstError::SymbolNotFound {
            file: "モジュール.rs".to_string(),
            symbol: "関数".to_string(),
        };
        assert!(err.to_string().contains("関数"));
    }

    #[test]
    fn test_long_error_message() {
        let long_msg = "a".repeat(1000);
        let err = AstError::parse_error(&long_msg);
        assert!(err.to_string().len() > 1000);
    }
}
