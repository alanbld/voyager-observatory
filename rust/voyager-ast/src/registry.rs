//! Adapter Registry
//!
//! The registry manages all language adapters and provides a unified interface
//! for parsing files across languages.

use crate::adapters::{
    LanguageAdapter, PythonTreeSitterAdapter, RustTreeSitterAdapter, TypeScriptTreeSitterAdapter,
};
use crate::error::{AstError, Result};
use crate::ir::{File, LanguageId, Span};
use std::collections::BTreeMap;
use std::path::Path;

/// Registry of language adapters
pub struct AdapterRegistry {
    adapters: BTreeMap<LanguageId, Box<dyn LanguageAdapter>>,
}

impl AdapterRegistry {
    /// Create a new registry with all built-in adapters
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: BTreeMap::new(),
        };

        // Register built-in adapters - Core Fleet (Phase 1B)
        registry.register(Box::new(RustTreeSitterAdapter::new()));
        registry.register(Box::new(PythonTreeSitterAdapter::new()));
        registry.register(Box::new(TypeScriptTreeSitterAdapter::new())); // .ts, .mts, .cts
        registry.register(Box::new(TypeScriptTreeSitterAdapter::tsx())); // .tsx
        registry.register(Box::new(TypeScriptTreeSitterAdapter::javascript())); // .js, .mjs, .cjs
                                                                                // Note: JSX (.jsx) uses same JavaScript grammar but with different LanguageId
                                                                                // For now, JSX files will use JavaScript adapter

        registry
    }

    /// Register a language adapter
    pub fn register(&mut self, adapter: Box<dyn LanguageAdapter>) {
        self.adapters.insert(adapter.language(), adapter);
    }

    /// Get an adapter for a language
    pub fn get(&self, language: LanguageId) -> Option<&dyn LanguageAdapter> {
        self.adapters.get(&language).map(|a| a.as_ref())
    }

    /// Get supported languages
    pub fn supported_languages(&self) -> Vec<LanguageId> {
        self.adapters.keys().copied().collect()
    }

    /// Check if a language is supported
    pub fn supports(&self, language: LanguageId) -> bool {
        self.adapters.contains_key(&language)
    }

    /// Parse a source file
    pub fn parse(&self, source: &str, language: LanguageId) -> Result<File> {
        let adapter = self
            .get(language)
            .ok_or(AstError::UnsupportedLanguage(language))?;

        // Create parser
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .map_err(|e| AstError::TreeSitterError(e.to_string()))?;

        // Parse source
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| AstError::parse_error("Failed to parse source"))?;

        // Extract file structure
        let mut file = File::new(String::new(), language);
        file.span = Span {
            start: 0,
            end: source.len(),
            start_line: 1,
            end_line: source.lines().count(),
            start_column: 0,
            end_column: 0,
        };

        // Extract declarations
        file.declarations = adapter.extract_declarations(&tree, source);

        // Populate each declaration's body (control flow, calls, nested
        // declarations) so consumers like census get real complexity data
        // without having to go through the separate Zoom-mode `zoom_into`
        // path, which most callers (serialize, survey) never invoke.
        for decl in &mut file.declarations {
            populate_bodies(adapter, &tree, source, decl);
        }

        // Extract imports
        file.imports = adapter.extract_imports(&tree, source);

        // Extract comments
        file.comments = adapter.extract_comments(&tree, source);

        // Extract error regions
        file.unknown_regions = adapter.extract_errors(&tree, source);

        Ok(file)
    }
}

/// Recursively populate `body` for `decl` and every nested declaration
/// (methods in a class, inner functions, etc.) via the adapter's Zoom-mode
/// `extract_body`, so a normal parse carries the same control-flow data
/// Zoom mode would compute on demand for a single declaration.
fn populate_bodies(
    adapter: &dyn LanguageAdapter,
    tree: &tree_sitter::Tree,
    source: &str,
    decl: &mut crate::ir::Declaration,
) {
    decl.body = adapter.extract_body(tree, source, decl);
    for child in &mut decl.children {
        populate_bodies(adapter, tree, source, child);
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AdapterRegistry Tests
    // =========================================================================

    #[test]
    fn test_registry_creation() {
        let registry = AdapterRegistry::new();
        // Core Fleet (Phase 1B)
        assert!(registry.supports(LanguageId::Rust));
        assert!(registry.supports(LanguageId::Python));
        assert!(registry.supports(LanguageId::TypeScript));
        assert!(registry.supports(LanguageId::Tsx));
        assert!(registry.supports(LanguageId::JavaScript));
        assert!(!registry.supports(LanguageId::Unknown));
    }

    #[test]
    fn test_registry_default() {
        let registry = AdapterRegistry::default();
        // Should be same as new()
        assert!(registry.supports(LanguageId::Rust));
        assert!(registry.supports(LanguageId::Python));
    }

    #[test]
    fn test_registry_get_adapter() {
        let registry = AdapterRegistry::new();

        // Get existing adapter
        let rust_adapter = registry.get(LanguageId::Rust);
        assert!(rust_adapter.is_some());
        assert_eq!(rust_adapter.unwrap().language(), LanguageId::Rust);

        // Get non-existing adapter
        let unknown_adapter = registry.get(LanguageId::Unknown);
        assert!(unknown_adapter.is_none());
    }

    #[test]
    fn test_registry_supported_languages() {
        let registry = AdapterRegistry::new();
        let languages = registry.supported_languages();

        // Core Fleet (Phase 1B)
        assert!(languages.contains(&LanguageId::Rust));
        assert!(languages.contains(&LanguageId::Python));
        assert!(languages.contains(&LanguageId::TypeScript));
        assert!(languages.contains(&LanguageId::Tsx));
        assert!(languages.contains(&LanguageId::JavaScript));
        assert!(!languages.contains(&LanguageId::Unknown));
    }

    #[test]
    fn test_registry_register_custom_adapter() {
        let mut registry = AdapterRegistry {
            adapters: BTreeMap::new(),
        };

        // Initially empty
        assert!(!registry.supports(LanguageId::Rust));

        // Register Rust adapter
        registry.register(Box::new(RustTreeSitterAdapter::new()));
        assert!(registry.supports(LanguageId::Rust));

        // Can get the adapter
        let adapter = registry.get(LanguageId::Rust);
        assert!(adapter.is_some());
    }

    #[test]
    fn test_registry_parse_rust() {
        let registry = AdapterRegistry::new();
        let source = "fn test_function() {}";

        let file = registry.parse(source, LanguageId::Rust).unwrap();
        assert_eq!(file.language, LanguageId::Rust);
        assert!(file.declarations.len() >= 1);
    }

    #[test]
    fn test_registry_parse_python() {
        let registry = AdapterRegistry::new();
        let source = "def test_function():\n    pass";

        let file = registry.parse(source, LanguageId::Python).unwrap();
        assert_eq!(file.language, LanguageId::Python);
    }

    #[test]
    fn test_registry_parse_unsupported_language() {
        let registry = AdapterRegistry::new();
        let result = registry.parse("code", LanguageId::Unknown);

        assert!(matches!(
            result,
            Err(AstError::UnsupportedLanguage(LanguageId::Unknown))
        ));
    }

    #[test]
    fn test_registry_parse_sets_span() {
        let registry = AdapterRegistry::new();
        let source = "fn foo() {}\nfn bar() {}";

        let file = registry.parse(source, LanguageId::Rust).unwrap();
        assert_eq!(file.span.start, 0);
        assert_eq!(file.span.end, source.len());
        assert_eq!(file.span.start_line, 1);
        assert_eq!(file.span.end_line, 2);
    }
}
