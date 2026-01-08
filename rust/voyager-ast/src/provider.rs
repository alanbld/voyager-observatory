//! AST Provider Trait and Core Models
//!
//! This module defines the primary interface for voyager-ast:
//! - `AstProvider` trait for implementations
//! - `PlanetariumModel` for project-wide indexing
//! - `MicroscopeModel` for symbol zoom

use crate::error::Result;
use crate::ir::{Block, Declaration, File, LanguageId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

// ============================================================================
// Options
// ============================================================================

/// Options for project indexing (Planetarium mode)
#[derive(Debug, Clone, Default)]
pub struct IndexOptions {
    /// Maximum files to process (0 = unlimited)
    pub max_files: usize,

    /// File patterns to include (glob syntax)
    pub include_patterns: Vec<String>,

    /// File patterns to exclude (glob syntax)
    pub exclude_patterns: Vec<String>,

    /// Whether to extract doc comments
    pub extract_comments: bool,

    /// Whether to follow symbolic links
    pub follow_symlinks: bool,

    /// Languages to include (empty = all supported)
    pub languages: Vec<LanguageId>,

    /// Whether to extract nested declarations in Index mode
    pub extract_nested: bool,
}

/// Options for symbol zoom (Microscope mode)
#[derive(Debug, Clone)]
pub struct ZoomOptions {
    /// Maximum depth for nested blocks
    pub max_depth: usize,

    /// Whether to extract function/method calls
    pub extract_calls: bool,

    /// Whether to extract control flow structures
    pub extract_control_flow: bool,

    /// Include surrounding context lines (before/after)
    pub context_lines: usize,

    /// Whether to include nested declarations
    pub extract_nested: bool,
}

impl Default for ZoomOptions {
    fn default() -> Self {
        Self {
            max_depth: 10,
            extract_calls: true,
            extract_control_flow: true,
            context_lines: 0,
            extract_nested: true,
        }
    }
}

// ============================================================================
// Planetarium Model (Index Result)
// ============================================================================

/// The result of indexing a project (Planetarium View)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetariumModel {
    /// Root path of the indexed project
    pub root: String,

    /// All indexed files, keyed by relative path (BTreeMap for determinism)
    pub files: BTreeMap<String, File>,

    /// Statistics about the indexing run
    pub stats: IndexStats,

    /// Errors encountered during indexing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<IndexError>,
}

impl PlanetariumModel {
    /// Create a new empty model
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            files: BTreeMap::new(),
            stats: IndexStats::default(),
            errors: Vec::new(),
        }
    }

    /// Get all declarations across all files
    pub fn all_declarations(&self) -> impl Iterator<Item = (&str, &Declaration)> {
        self.files
            .iter()
            .flat_map(|(path, file)| file.declarations.iter().map(move |d| (path.as_str(), d)))
    }

    /// Find declarations by name
    pub fn find_by_name(&self, name: &str) -> Vec<(&str, &Declaration)> {
        self.all_declarations()
            .filter(|(_, d)| d.name == name)
            .collect()
    }

    /// Get total declaration count
    pub fn total_declarations(&self) -> usize {
        self.files.values().map(|f| f.total_declarations()).sum()
    }
}

/// Statistics from an indexing run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of files processed
    pub files_processed: usize,

    /// Number of files skipped (binary, too large, etc.)
    pub files_skipped: usize,

    /// Total declarations found
    pub declarations_found: usize,

    /// Total imports found
    pub imports_found: usize,

    /// Number of unknown/error regions
    pub unknown_regions: usize,

    /// Parse time in milliseconds
    pub parse_time_ms: u64,

    /// Per-language statistics
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub by_language: BTreeMap<String, LanguageStats>,
}

/// Per-language statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageStats {
    pub files: usize,
    pub declarations: usize,
    pub imports: usize,
}

/// An error that occurred during indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexError {
    /// Path to the file that caused the error
    pub path: String,

    /// Error message
    pub message: String,

    /// Whether parsing could partially recover
    pub recoverable: bool,
}

// ============================================================================
// Microscope Model (Zoom Result)
// ============================================================================

/// The result of zooming into a symbol (Microscope View)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroscopeModel {
    /// The file containing the symbol
    pub file_path: String,

    /// The symbol that was zoomed into
    pub symbol: Declaration,

    /// The fully-parsed body block (if the symbol has a body)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Block>,

    /// Surrounding context (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextWindow>,

    /// Source code of the symbol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
}

/// Surrounding context for a zoomed symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Lines before the symbol
    pub before: Vec<String>,

    /// Lines after the symbol
    pub after: Vec<String>,
}

// ============================================================================
// Provider Trait
// ============================================================================

/// The core trait for AST providers
///
/// This trait defines the two primary operations:
/// 1. `index_project` - Planetarium view (project-wide scan)
/// 2. `zoom_into` - Microscope view (symbol deep-dive)
pub trait AstProvider: Send + Sync {
    /// Index an entire project, returning a Planetarium model
    ///
    /// This performs a shallow scan of all files, extracting:
    /// - Top-level declarations
    /// - Import statements
    /// - File-level comments
    ///
    /// # Arguments
    /// * `root` - Root directory to index
    /// * `options` - Indexing options
    fn index_project(&self, root: &Path, options: &IndexOptions) -> Result<PlanetariumModel>;

    /// Zoom into a specific symbol, returning a Microscope model
    ///
    /// This performs a deep parse of the target symbol, extracting:
    /// - Full body with nested blocks
    /// - Control flow structures
    /// - Function/method calls
    /// - Inline comments
    ///
    /// # Arguments
    /// * `file_path` - Path to the file containing the symbol
    /// * `symbol_id` - Identifier for the symbol (from Declaration::id())
    /// * `options` - Zoom options
    fn zoom_into(
        &self,
        file_path: &Path,
        symbol_id: &str,
        options: &ZoomOptions,
    ) -> Result<MicroscopeModel>;

    /// Parse a single file (used internally and for testing)
    ///
    /// # Arguments
    /// * `source` - Source code to parse
    /// * `language` - Language of the source
    fn parse_file(&self, source: &str, language: LanguageId) -> Result<File>;

    /// Get the list of supported languages
    fn supported_languages(&self) -> &[LanguageId];

    /// Check if a language is supported
    fn supports(&self, language: LanguageId) -> bool {
        self.supported_languages().contains(&language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planetarium_model_determinism() {
        let mut model = PlanetariumModel::new("/test");
        model.files.insert(
            "a.rs".to_string(),
            File::new("a.rs".to_string(), LanguageId::Rust),
        );
        model.files.insert(
            "b.py".to_string(),
            File::new("b.py".to_string(), LanguageId::Python),
        );

        let json1 = serde_json::to_string(&model).unwrap();
        let json2 = serde_json::to_string(&model).unwrap();
        assert_eq!(json1, json2, "Model serialization must be deterministic");
    }

    #[test]
    fn test_index_options_default() {
        let opts = IndexOptions::default();
        assert_eq!(opts.max_files, 0);
        assert!(!opts.extract_comments);
    }

    #[test]
    fn test_zoom_options_default() {
        let opts = ZoomOptions::default();
        assert!(opts.extract_calls);
        assert!(opts.extract_control_flow);
        assert_eq!(opts.max_depth, 10);
    }

    // =========================================================================
    // IndexOptions Tests
    // =========================================================================

    #[test]
    fn test_index_options_fields() {
        let opts = IndexOptions {
            max_files: 100,
            include_patterns: vec!["*.rs".to_string()],
            exclude_patterns: vec!["target/**".to_string()],
            extract_comments: true,
            follow_symlinks: true,
            languages: vec![LanguageId::Rust, LanguageId::Python],
            extract_nested: true,
        };

        assert_eq!(opts.max_files, 100);
        assert_eq!(opts.include_patterns.len(), 1);
        assert_eq!(opts.exclude_patterns.len(), 1);
        assert!(opts.extract_comments);
        assert!(opts.follow_symlinks);
        assert_eq!(opts.languages.len(), 2);
        assert!(opts.extract_nested);
    }

    #[test]
    fn test_index_options_clone() {
        let opts = IndexOptions {
            max_files: 50,
            include_patterns: vec!["**/*.py".to_string()],
            exclude_patterns: vec![],
            extract_comments: true,
            follow_symlinks: false,
            languages: vec![LanguageId::Python],
            extract_nested: false,
        };

        let cloned = opts.clone();
        assert_eq!(cloned.max_files, 50);
        assert_eq!(cloned.include_patterns.len(), 1);
    }

    #[test]
    fn test_index_options_debug() {
        let opts = IndexOptions::default();
        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("IndexOptions"));
        assert!(debug_str.contains("max_files"));
    }

    // =========================================================================
    // ZoomOptions Tests
    // =========================================================================

    #[test]
    fn test_zoom_options_fields() {
        let opts = ZoomOptions {
            max_depth: 5,
            extract_calls: false,
            extract_control_flow: true,
            context_lines: 3,
            extract_nested: false,
        };

        assert_eq!(opts.max_depth, 5);
        assert!(!opts.extract_calls);
        assert!(opts.extract_control_flow);
        assert_eq!(opts.context_lines, 3);
        assert!(!opts.extract_nested);
    }

    #[test]
    fn test_zoom_options_clone() {
        let opts = ZoomOptions::default();
        let cloned = opts.clone();
        assert_eq!(cloned.max_depth, opts.max_depth);
        assert_eq!(cloned.extract_calls, opts.extract_calls);
    }

    #[test]
    fn test_zoom_options_debug() {
        let opts = ZoomOptions::default();
        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("ZoomOptions"));
        assert!(debug_str.contains("max_depth"));
    }

    // =========================================================================
    // PlanetariumModel Tests
    // =========================================================================

    #[test]
    fn test_planetarium_model_new() {
        let model = PlanetariumModel::new("/project");
        assert_eq!(model.root, "/project");
        assert!(model.files.is_empty());
        assert!(model.errors.is_empty());
    }

    #[test]
    fn test_planetarium_model_all_declarations() {
        use crate::ir::{Declaration, DeclarationKind, Span};

        let mut model = PlanetariumModel::new("/test");

        let mut file1 = File::new("a.rs".to_string(), LanguageId::Rust);
        file1.declarations.push(Declaration::new(
            "foo".to_string(),
            DeclarationKind::Function,
            Span::new(0, 10, 1, 5),
        ));

        let mut file2 = File::new("b.rs".to_string(), LanguageId::Rust);
        file2.declarations.push(Declaration::new(
            "bar".to_string(),
            DeclarationKind::Function,
            Span::new(0, 10, 1, 5),
        ));

        model.files.insert("a.rs".to_string(), file1);
        model.files.insert("b.rs".to_string(), file2);

        let all_decls: Vec<_> = model.all_declarations().collect();
        assert_eq!(all_decls.len(), 2);
    }

    #[test]
    fn test_planetarium_model_find_by_name() {
        use crate::ir::{Declaration, DeclarationKind, Span};

        let mut model = PlanetariumModel::new("/test");

        let mut file = File::new("test.rs".to_string(), LanguageId::Rust);
        file.declarations.push(Declaration::new(
            "target_fn".to_string(),
            DeclarationKind::Function,
            Span::new(0, 10, 1, 5),
        ));
        file.declarations.push(Declaration::new(
            "other_fn".to_string(),
            DeclarationKind::Function,
            Span::new(20, 30, 10, 15),
        ));

        model.files.insert("test.rs".to_string(), file);

        let found = model.find_by_name("target_fn");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.name, "target_fn");

        let not_found = model.find_by_name("nonexistent");
        assert!(not_found.is_empty());
    }

    #[test]
    fn test_planetarium_model_total_declarations() {
        use crate::ir::{Declaration, DeclarationKind, Span};

        let mut model = PlanetariumModel::new("/test");

        let mut file1 = File::new("a.rs".to_string(), LanguageId::Rust);
        file1.declarations.push(Declaration::new(
            "fn1".to_string(),
            DeclarationKind::Function,
            Span::new(0, 10, 1, 5),
        ));
        file1.declarations.push(Declaration::new(
            "fn2".to_string(),
            DeclarationKind::Function,
            Span::new(20, 30, 10, 15),
        ));

        let file2 = File::new("b.rs".to_string(), LanguageId::Rust);

        model.files.insert("a.rs".to_string(), file1);
        model.files.insert("b.rs".to_string(), file2);

        assert_eq!(model.total_declarations(), 2);
    }

    #[test]
    fn test_planetarium_model_empty() {
        let model = PlanetariumModel::new("/empty");
        assert_eq!(model.total_declarations(), 0);
        assert!(model.find_by_name("anything").is_empty());
        assert_eq!(model.all_declarations().count(), 0);
    }

    // =========================================================================
    // IndexStats Tests
    // =========================================================================

    #[test]
    fn test_index_stats_default() {
        let stats = IndexStats::default();
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.declarations_found, 0);
        assert_eq!(stats.imports_found, 0);
        assert_eq!(stats.unknown_regions, 0);
        assert_eq!(stats.parse_time_ms, 0);
        assert!(stats.by_language.is_empty());
    }

    #[test]
    fn test_index_stats_fields() {
        let mut stats = IndexStats {
            files_processed: 100,
            files_skipped: 5,
            declarations_found: 500,
            imports_found: 200,
            unknown_regions: 10,
            parse_time_ms: 1500,
            by_language: BTreeMap::new(),
        };

        stats.by_language.insert(
            "rust".to_string(),
            LanguageStats {
                files: 50,
                declarations: 250,
                imports: 100,
            },
        );

        assert_eq!(stats.files_processed, 100);
        assert_eq!(stats.by_language.len(), 1);
    }

    #[test]
    fn test_index_stats_serialization() {
        let stats = IndexStats {
            files_processed: 10,
            files_skipped: 2,
            declarations_found: 50,
            imports_found: 20,
            unknown_regions: 1,
            parse_time_ms: 500,
            by_language: BTreeMap::new(),
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: IndexStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.files_processed, 10);
        assert_eq!(deserialized.declarations_found, 50);
    }

    // =========================================================================
    // LanguageStats Tests
    // =========================================================================

    #[test]
    fn test_language_stats_default() {
        let stats = LanguageStats::default();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.declarations, 0);
        assert_eq!(stats.imports, 0);
    }

    #[test]
    fn test_language_stats_fields() {
        let stats = LanguageStats {
            files: 25,
            declarations: 150,
            imports: 75,
        };

        assert_eq!(stats.files, 25);
        assert_eq!(stats.declarations, 150);
        assert_eq!(stats.imports, 75);
    }

    // =========================================================================
    // IndexError Tests
    // =========================================================================

    #[test]
    fn test_index_error_fields() {
        let err = IndexError {
            path: "broken.rs".to_string(),
            message: "syntax error at line 10".to_string(),
            recoverable: true,
        };

        assert_eq!(err.path, "broken.rs");
        assert!(err.message.contains("syntax error"));
        assert!(err.recoverable);
    }

    #[test]
    fn test_index_error_serialization() {
        let err = IndexError {
            path: "test.py".to_string(),
            message: "indentation error".to_string(),
            recoverable: false,
        };

        let json = serde_json::to_string(&err).unwrap();
        let deserialized: IndexError = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.path, "test.py");
        assert!(!deserialized.recoverable);
    }

    // =========================================================================
    // MicroscopeModel Tests
    // =========================================================================

    #[test]
    fn test_microscope_model_fields() {
        use crate::ir::{Declaration, DeclarationKind, Span};

        let model = MicroscopeModel {
            file_path: "src/lib.rs".to_string(),
            symbol: Declaration::new(
                "calculate".to_string(),
                DeclarationKind::Function,
                Span::new(100, 200, 10, 25),
            ),
            body: None,
            context: None,
            source_text: Some("fn calculate() { ... }".to_string()),
        };

        assert_eq!(model.file_path, "src/lib.rs");
        assert_eq!(model.symbol.name, "calculate");
        assert!(model.body.is_none());
        assert!(model.source_text.is_some());
    }

    #[test]
    fn test_microscope_model_with_context() {
        use crate::ir::{Declaration, DeclarationKind, Span};

        let model = MicroscopeModel {
            file_path: "test.rs".to_string(),
            symbol: Declaration::new(
                "test_fn".to_string(),
                DeclarationKind::Function,
                Span::new(0, 50, 5, 10),
            ),
            body: None,
            context: Some(ContextWindow {
                before: vec!["// comment".to_string(), "use std::io;".to_string()],
                after: vec!["".to_string(), "fn next_fn() {}".to_string()],
            }),
            source_text: None,
        };

        assert!(model.context.is_some());
        let ctx = model.context.unwrap();
        assert_eq!(ctx.before.len(), 2);
        assert_eq!(ctx.after.len(), 2);
    }

    #[test]
    fn test_microscope_model_serialization() {
        use crate::ir::{Declaration, DeclarationKind, Span};

        let model = MicroscopeModel {
            file_path: "test.rs".to_string(),
            symbol: Declaration::new(
                "my_func".to_string(),
                DeclarationKind::Function,
                Span::new(0, 10, 1, 5),
            ),
            body: None,
            context: None,
            source_text: None,
        };

        let json = serde_json::to_string(&model).unwrap();
        let deserialized: MicroscopeModel = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.file_path, "test.rs");
        assert_eq!(deserialized.symbol.name, "my_func");
    }

    // =========================================================================
    // ContextWindow Tests
    // =========================================================================

    #[test]
    fn test_context_window_fields() {
        let ctx = ContextWindow {
            before: vec!["line1".to_string(), "line2".to_string()],
            after: vec!["line3".to_string()],
        };

        assert_eq!(ctx.before.len(), 2);
        assert_eq!(ctx.after.len(), 1);
    }

    #[test]
    fn test_context_window_empty() {
        let ctx = ContextWindow {
            before: vec![],
            after: vec![],
        };

        assert!(ctx.before.is_empty());
        assert!(ctx.after.is_empty());
    }

    #[test]
    fn test_context_window_serialization() {
        let ctx = ContextWindow {
            before: vec!["// header".to_string()],
            after: vec!["// footer".to_string()],
        };

        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: ContextWindow = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.before[0], "// header");
        assert_eq!(deserialized.after[0], "// footer");
    }
}
