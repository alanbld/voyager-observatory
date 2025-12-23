//! Language Plugins for pm_encoder
//!
//! This module provides language-specific analysis plugins for the fractal context engine.
//! Each plugin implements symbol extraction, relationship detection, and language-specific
//! context enrichment.
//!
//! # Supported Languages
//!
//! - **Shell** (bash, sh, zsh, ksh) - Shell script analysis
//! - More plugins to come: ABL, C#, etc.
//!
//! # Plugin Architecture
//!
//! Plugins implement the `LanguagePlugin` trait which provides:
//! - Symbol extraction (functions, variables, exports)
//! - File information extraction
//! - Relationship detection (calls, sources)
//! - Documentation extraction

pub mod shell;

use std::path::Path;

use thiserror::Error;

use crate::core::fractal::{ExtractedSymbol, Import};

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PluginResult<T> = Result<T, PluginError>;

// =============================================================================
// Plugin Trait
// =============================================================================

/// Trait for language-specific analysis plugins.
pub trait LanguagePlugin: Send + Sync {
    /// Get the language name (e.g., "shell", "rust", "python").
    fn language_name(&self) -> &'static str;

    /// Get supported file extensions.
    fn extensions(&self) -> &[&'static str];

    /// Check if this plugin supports a file by extension.
    fn supports_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.extensions().iter().any(|e| e.eq_ignore_ascii_case(ext)))
            .unwrap_or(false)
    }

    /// Extract symbols from source content.
    fn extract_symbols(&self, content: &str) -> PluginResult<Vec<ExtractedSymbol>>;

    /// Extract imports/sources from source content.
    fn extract_imports(&self, content: &str) -> PluginResult<Vec<Import>>;

    /// Get file metadata.
    fn file_info(&self, content: &str) -> PluginResult<FileInfo>;
}

/// Information about a source file.
#[derive(Debug, Clone, Default)]
pub struct FileInfo {
    /// Detected language
    pub language: String,
    /// Language dialect (e.g., "bash", "zsh" for shell)
    pub dialect: Option<String>,
    /// Number of symbols extracted
    pub symbol_count: usize,
    /// Number of lines
    pub line_count: usize,
    /// Whether file appears to be a test file
    pub is_test: bool,
    /// Whether file is executable
    pub is_executable: bool,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

// =============================================================================
// Plugin Registry
// =============================================================================

/// Registry for language plugins.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn LanguagePlugin>>,
}

impl PluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Create a registry with default plugins.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(shell::ShellPlugin::new()));
        registry
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn LanguagePlugin>) {
        self.plugins.push(plugin);
    }

    /// Find a plugin for a file.
    pub fn find_for_file(&self, path: &Path) -> Option<&dyn LanguagePlugin> {
        self.plugins
            .iter()
            .find(|p| p.supports_file(path))
            .map(|p| p.as_ref())
    }

    /// Find a plugin by language name.
    pub fn find_by_language(&self, language: &str) -> Option<&dyn LanguagePlugin> {
        self.plugins
            .iter()
            .find(|p| p.language_name().eq_ignore_ascii_case(language))
            .map(|p| p.as_ref())
    }

    /// Get all registered plugins.
    pub fn plugins(&self) -> &[Box<dyn LanguagePlugin>] {
        &self.plugins
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_plugin_registry_new() {
        let registry = PluginRegistry::new();
        assert!(registry.plugins().is_empty());
    }

    #[test]
    fn test_plugin_registry_with_defaults() {
        let registry = PluginRegistry::with_defaults();
        assert!(!registry.plugins().is_empty());
    }

    #[test]
    fn test_find_plugin_for_shell() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("script.sh"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "shell");
    }

    #[test]
    fn test_find_plugin_by_language() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_by_language("shell");
        assert!(plugin.is_some());

        let plugin = registry.find_by_language("SHELL");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_no_plugin_for_unknown_file() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("file.xyz"));
        assert!(plugin.is_none());
    }
}
