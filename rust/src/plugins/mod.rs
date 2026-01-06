//! Language Plugins for pm_encoder
//!
//! This module provides language-specific analysis plugins for the fractal context engine.
//! Each plugin implements symbol extraction, relationship detection, and language-specific
//! context enrichment.
//!
//! # Supported Languages
//!
//! - **Shell** (bash, sh, zsh, ksh) - Shell script analysis
//! - **ABL** (OpenEdge Progress 4GL) - Business application language
//! - **Python** - Python source analysis with decorator recognition
//! - **TypeScript** - TypeScript/JavaScript with type-aware semantic mapping
//!
//! # Plugin Architecture
//!
//! Plugins implement the `LanguagePlugin` trait which provides:
//! - Symbol extraction (functions, variables, exports)
//! - File information extraction
//! - Relationship detection (calls, sources)
//! - Documentation extraction
//! - **Semantic concept mapping** for intent-driven exploration
//!
//! # Semantic Substrate
//!
//! Each plugin maps language-specific constructs to our universal semantic space:
//! - ConceptType classification (Calculation, Validation, etc.)
//! - 64-dimension feature vector extraction
//! - Intent-based relevance scoring

pub mod shell;
pub mod abl;
pub mod python;
pub mod typescript;

use std::path::Path;

use thiserror::Error;

use crate::core::fractal::{ExtractedSymbol, Import, ConceptType};

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

    // =========================================================================
    // Semantic Mapping (for Intent-Driven Exploration)
    // =========================================================================

    /// Infer the semantic concept type for a symbol.
    ///
    /// This method provides language-aware concept classification that goes beyond
    /// the generic ConceptType::infer. For example:
    /// - ABL: `PROCEDURE calculate-tax:` → ConceptType::Calculation
    /// - ABL: `FOR EACH customer:` → ConceptType::Transformation
    /// - Shell: `cleanup()` function → ConceptType::Infrastructure
    ///
    /// Default implementation falls back to the generic ConceptType::infer.
    fn infer_concept_type(&self, symbol: &ExtractedSymbol, _content: &str) -> ConceptType {
        // Default: use the generic name-based inference
        // Subclasses can override for language-specific semantics
        infer_concept_from_symbol(symbol)
    }

    /// Calculate semantic relevance boost for a symbol based on language patterns.
    ///
    /// Returns a value between -0.5 and 0.5 to adjust the base relevance score.
    /// For example:
    /// - ABL: `SUPER:` calls might get a boost for debugging intent
    /// - Shell: `set -e` might indicate error handling awareness
    ///
    /// Default implementation returns 0.0 (no adjustment).
    fn semantic_relevance_boost(
        &self,
        _symbol: &ExtractedSymbol,
        _intent: &str,
        _content: &str,
    ) -> f32 {
        0.0
    }

    /// Get language-specific feature dimensions.
    ///
    /// Each language can contribute specific features to the 64D feature vector.
    /// Returns a map of dimension index (0-63) to feature value (0.0-1.0).
    ///
    /// Default implementation returns empty (no language-specific features).
    fn language_features(&self, _symbol: &ExtractedSymbol, _content: &str) -> Vec<(usize, f32)> {
        Vec::new()
    }
}

/// Infer concept type from ExtractedSymbol (helper for default implementation).
fn infer_concept_from_symbol(symbol: &ExtractedSymbol) -> ConceptType {
    use crate::core::fractal::{ContextLayer, LayerContent, Visibility};

    // Build a temporary layer to use the standard ConceptType::infer
    let layer = ContextLayer::new(
        &symbol.name,
        LayerContent::Symbol {
            name: symbol.name.clone(),
            kind: symbol.kind.clone(),
            signature: symbol.signature.clone(),
            return_type: symbol.return_type.clone(),
            parameters: symbol.parameters.clone(),
            documentation: symbol.documentation.clone(),
            visibility: if symbol.signature.contains("pub ") {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: symbol.range.clone(),
        },
    );

    ConceptType::infer(&layer)
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
        registry.register(Box::new(abl::AblPlugin::new()));
        registry.register(Box::new(python::PythonPlugin::new()));
        registry.register(Box::new(typescript::TypeScriptPlugin::new()));
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

    // ==================== Error Tests ====================

    #[test]
    fn test_plugin_error_unsupported_language() {
        let err = PluginError::UnsupportedLanguage("cobol".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Unsupported language"));
        assert!(msg.contains("cobol"));
    }

    #[test]
    fn test_plugin_error_extraction_failed() {
        let err = PluginError::ExtractionFailed("bad syntax".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Extraction failed"));
        assert!(msg.contains("bad syntax"));
    }

    #[test]
    fn test_plugin_error_parse_error() {
        let err = PluginError::ParseError {
            line: 42,
            message: "unexpected token".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("line 42"));
        assert!(msg.contains("unexpected token"));
    }

    #[test]
    fn test_plugin_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = PluginError::Io(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("IO error"));
    }

    #[test]
    fn test_plugin_error_debug() {
        let err = PluginError::UnsupportedLanguage("rust".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("UnsupportedLanguage"));
    }

    // ==================== FileInfo Tests ====================

    #[test]
    fn test_file_info_default() {
        let info = FileInfo::default();
        assert!(info.language.is_empty());
        assert!(info.dialect.is_none());
        assert_eq!(info.symbol_count, 0);
        assert_eq!(info.line_count, 0);
        assert!(!info.is_test);
        assert!(!info.is_executable);
        assert!(info.metadata.is_empty());
    }

    #[test]
    fn test_file_info_creation() {
        let info = FileInfo {
            language: "python".to_string(),
            dialect: Some("python3".to_string()),
            symbol_count: 10,
            line_count: 100,
            is_test: true,
            is_executable: false,
            metadata: std::collections::HashMap::new(),
        };
        assert_eq!(info.language, "python");
        assert_eq!(info.dialect, Some("python3".to_string()));
        assert_eq!(info.symbol_count, 10);
        assert!(info.is_test);
    }

    #[test]
    fn test_file_info_clone() {
        let info = FileInfo {
            language: "shell".to_string(),
            dialect: None,
            symbol_count: 5,
            line_count: 50,
            is_test: false,
            is_executable: true,
            metadata: std::collections::HashMap::new(),
        };
        let cloned = info.clone();
        assert_eq!(info.language, cloned.language);
        assert_eq!(info.is_executable, cloned.is_executable);
    }

    #[test]
    fn test_file_info_debug() {
        let info = FileInfo::default();
        let debug = format!("{:?}", info);
        assert!(debug.contains("FileInfo"));
    }

    // ==================== Registry Tests ====================

    #[test]
    fn test_plugin_registry_new() {
        let registry = PluginRegistry::new();
        assert!(registry.plugins().is_empty());
    }

    #[test]
    fn test_plugin_registry_with_defaults() {
        let registry = PluginRegistry::with_defaults();
        assert!(!registry.plugins().is_empty());
        // Should have at least 4 plugins: shell, abl, python, typescript
        assert!(registry.plugins().len() >= 4);
    }

    #[test]
    fn test_plugin_registry_default_trait() {
        let registry = PluginRegistry::default();
        assert!(!registry.plugins().is_empty());
    }

    #[test]
    fn test_plugin_registry_register() {
        let mut registry = PluginRegistry::new();
        assert!(registry.plugins().is_empty());

        registry.register(Box::new(shell::ShellPlugin::new()));
        assert_eq!(registry.plugins().len(), 1);

        registry.register(Box::new(python::PythonPlugin::new()));
        assert_eq!(registry.plugins().len(), 2);
    }

    // ==================== Plugin Finding Tests ====================

    #[test]
    fn test_find_plugin_for_shell() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("script.sh"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "shell");
    }

    #[test]
    fn test_find_plugin_for_bash() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("script.bash"));
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

    #[test]
    fn test_no_plugin_for_unknown_language() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_by_language("cobol");
        assert!(plugin.is_none());
    }

    #[test]
    fn test_find_plugin_for_python() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("main.py"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "python");
    }

    #[test]
    fn test_find_python_by_language() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_by_language("python");
        assert!(plugin.is_some());

        let plugin = registry.find_by_language("PYTHON");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_for_typescript() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("app.ts"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "typescript");

        let plugin = registry.find_for_file(Path::new("component.tsx"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "typescript");

        let plugin = registry.find_for_file(Path::new("script.js"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "typescript");
    }

    #[test]
    fn test_find_plugin_for_jsx() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("component.jsx"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "typescript");
    }

    #[test]
    fn test_find_typescript_by_language() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_by_language("typescript");
        assert!(plugin.is_some());

        let plugin = registry.find_by_language("TYPESCRIPT");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_for_abl() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_for_file(Path::new("procedure.p"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "abl");

        let plugin = registry.find_for_file(Path::new("class.cls"));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().language_name(), "abl");
    }

    #[test]
    fn test_find_abl_by_language() {
        let registry = PluginRegistry::with_defaults();

        let plugin = registry.find_by_language("abl");
        assert!(plugin.is_some());
    }

    // ==================== Plugin Trait Tests ====================

    #[test]
    fn test_shell_plugin_language_name() {
        let plugin = shell::ShellPlugin::new();
        assert_eq!(plugin.language_name(), "shell");
    }

    #[test]
    fn test_shell_plugin_extensions() {
        let plugin = shell::ShellPlugin::new();
        let exts = plugin.extensions();
        assert!(exts.contains(&"sh"));
        assert!(exts.contains(&"bash"));
    }

    #[test]
    fn test_shell_plugin_supports_file() {
        let plugin = shell::ShellPlugin::new();
        assert!(plugin.supports_file(Path::new("script.sh")));
        assert!(plugin.supports_file(Path::new("script.bash")));
        assert!(!plugin.supports_file(Path::new("script.py")));
    }

    #[test]
    fn test_python_plugin_language_name() {
        let plugin = python::PythonPlugin::new();
        assert_eq!(plugin.language_name(), "python");
    }

    #[test]
    fn test_python_plugin_extensions() {
        let plugin = python::PythonPlugin::new();
        let exts = plugin.extensions();
        assert!(exts.contains(&"py"));
    }

    #[test]
    fn test_typescript_plugin_language_name() {
        let plugin = typescript::TypeScriptPlugin::new();
        assert_eq!(plugin.language_name(), "typescript");
    }

    #[test]
    fn test_typescript_plugin_extensions() {
        let plugin = typescript::TypeScriptPlugin::new();
        let exts = plugin.extensions();
        assert!(exts.contains(&"ts"));
        assert!(exts.contains(&"tsx"));
        assert!(exts.contains(&"js"));
        assert!(exts.contains(&"jsx"));
    }

    #[test]
    fn test_abl_plugin_language_name() {
        let plugin = abl::AblPlugin::new();
        assert_eq!(plugin.language_name(), "abl");
    }

    // ==================== Extraction Tests ====================

    #[test]
    fn test_shell_extract_symbols() {
        let plugin = shell::ShellPlugin::new();
        let content = "#!/bin/bash\nfunction hello() {\n  echo 'Hello'\n}";
        let result = plugin.extract_symbols(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_extract_imports() {
        let plugin = shell::ShellPlugin::new();
        let content = "#!/bin/bash\nsource ./lib.sh\n. /etc/profile";
        let result = plugin.extract_imports(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shell_file_info() {
        let plugin = shell::ShellPlugin::new();
        let content = "#!/bin/bash\necho 'hello'";
        let result = plugin.file_info(content);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.language, "shell");
    }

    #[test]
    fn test_python_extract_symbols() {
        let plugin = python::PythonPlugin::new();
        let content = "def hello():\n    print('Hello')";
        let result = plugin.extract_symbols(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_typescript_extract_symbols() {
        let plugin = typescript::TypeScriptPlugin::new();
        let content = "function hello(): void { console.log('Hello'); }";
        let result = plugin.extract_symbols(content);
        assert!(result.is_ok());
    }

    // ==================== Semantic Method Tests ====================

    fn make_test_symbol(name: &str) -> crate::core::fractal::ExtractedSymbol {
        use crate::core::fractal::{ExtractedSymbol, SymbolKind, Visibility, Range};
        ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: String::new(),
            return_type: None,
            parameters: Vec::new(),
            documentation: None,
            visibility: Visibility::Public,
            range: Range::default(),
            calls: Vec::new(),
        }
    }

    #[test]
    fn test_semantic_relevance_boost_default() {
        let plugin = shell::ShellPlugin::new();
        let symbol = make_test_symbol("test");
        let boost = plugin.semantic_relevance_boost(&symbol, "debugging", "");
        assert_eq!(boost, 0.0);
    }

    #[test]
    fn test_language_features_default() {
        let plugin = shell::ShellPlugin::new();
        let symbol = make_test_symbol("test");
        let features = plugin.language_features(&symbol, "");
        assert!(features.is_empty());
    }

    // ==================== supports_file Edge Cases ====================

    #[test]
    fn test_supports_file_no_extension() {
        let plugin = shell::ShellPlugin::new();
        assert!(!plugin.supports_file(Path::new("Makefile")));
    }

    #[test]
    fn test_supports_file_case_insensitive() {
        let plugin = shell::ShellPlugin::new();
        assert!(plugin.supports_file(Path::new("script.SH")));
        assert!(plugin.supports_file(Path::new("script.Sh")));
    }

    #[test]
    fn test_supports_file_hidden() {
        let plugin = shell::ShellPlugin::new();
        assert!(plugin.supports_file(Path::new(".bashrc.sh")));
    }

    // ==================== infer_concept_from_symbol Tests ====================

    #[test]
    fn test_infer_concept_from_symbol_calculation() {
        let symbol = make_test_symbol("calculate_total");
        let concept = infer_concept_from_symbol(&symbol);
        assert_eq!(concept, ConceptType::Calculation);
    }

    #[test]
    fn test_infer_concept_from_symbol_validation() {
        let symbol = make_test_symbol("validate_input");
        let concept = infer_concept_from_symbol(&symbol);
        assert_eq!(concept, ConceptType::Validation);
    }

    #[test]
    fn test_infer_concept_from_symbol_error_handling() {
        let symbol = make_test_symbol("handle_error");
        let concept = infer_concept_from_symbol(&symbol);
        assert_eq!(concept, ConceptType::ErrorHandling);
    }

    #[test]
    fn test_infer_concept_from_symbol_logging() {
        let symbol = make_test_symbol("log_event");
        let concept = infer_concept_from_symbol(&symbol);
        assert_eq!(concept, ConceptType::Logging);
    }

    #[test]
    fn test_infer_concept_from_symbol_config() {
        let symbol = make_test_symbol("init_config");
        let concept = infer_concept_from_symbol(&symbol);
        assert_eq!(concept, ConceptType::Configuration);
    }

    #[test]
    fn test_infer_concept_from_symbol_transformation() {
        let symbol = make_test_symbol("convert_data");
        let concept = infer_concept_from_symbol(&symbol);
        assert_eq!(concept, ConceptType::Transformation);
    }

    #[test]
    fn test_infer_concept_from_symbol_public() {
        use crate::core::fractal::{ExtractedSymbol, SymbolKind, Visibility, Range};
        let symbol = ExtractedSymbol {
            name: "public_function".to_string(),
            kind: SymbolKind::Function,
            signature: "pub fn public_function()".to_string(),
            return_type: None,
            parameters: Vec::new(),
            documentation: None,
            visibility: Visibility::Public,
            range: Range::default(),
            calls: Vec::new(),
        };
        let concept = infer_concept_from_symbol(&symbol);
        // Just verifies no panic and returns a concept
        let _ = concept;
    }

    #[test]
    fn test_infer_concept_type_default_impl() {
        let plugin = shell::ShellPlugin::new();
        let symbol = make_test_symbol("calculate_total");
        let concept = plugin.infer_concept_type(&symbol, "");
        assert_eq!(concept, ConceptType::Calculation);
    }

    // ==================== Additional Registry Tests ====================

    #[test]
    fn test_registry_plugins_getter() {
        let registry = PluginRegistry::with_defaults();
        let plugins = registry.plugins();
        assert!(plugins.len() >= 4);
    }

    #[test]
    fn test_registry_find_for_file_with_path() {
        let registry = PluginRegistry::with_defaults();
        let plugin = registry.find_for_file(Path::new("/some/path/script.sh"));
        assert!(plugin.is_some());
    }

    #[test]
    fn test_registry_find_for_file_empty_extension() {
        let registry = PluginRegistry::with_defaults();
        let plugin = registry.find_for_file(Path::new("script."));
        assert!(plugin.is_none());
    }
}
