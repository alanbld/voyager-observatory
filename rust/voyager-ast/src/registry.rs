//! Adapter Registry
//!
//! The registry manages all language adapters and provides a unified interface
//! for parsing files across languages.

use crate::adapters::{
    LanguageAdapter, PythonTreeSitterAdapter, RustTreeSitterAdapter, TypeScriptTreeSitterAdapter,
};
use crate::error::{AstError, Result};
use crate::ir::{File, LanguageId, Span, UnknownNode};
use crate::provider::{
    AstProvider, IndexError, IndexOptions, IndexStats, LanguageStats, MicroscopeModel,
    PlanetariumModel, ZoomOptions,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

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
        registry.register(Box::new(TypeScriptTreeSitterAdapter::new()));      // .ts, .mts, .cts
        registry.register(Box::new(TypeScriptTreeSitterAdapter::tsx()));       // .tsx
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
            .ok_or_else(|| AstError::UnsupportedLanguage(language))?;

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

        // Extract imports
        file.imports = adapter.extract_imports(&tree, source);

        // Extract comments
        file.comments = adapter.extract_comments(&tree, source);

        // Extract error regions
        file.unknown_regions = adapter.extract_errors(&tree, source);

        Ok(file)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tree-sitter based AST provider
pub struct TreeSitterProvider {
    registry: AdapterRegistry,
}

impl TreeSitterProvider {
    /// Create a new provider with all built-in adapters
    pub fn new() -> Self {
        Self {
            registry: AdapterRegistry::new(),
        }
    }

    /// Create a provider with a custom registry
    pub fn with_registry(registry: AdapterRegistry) -> Self {
        Self { registry }
    }

    /// Get the adapter registry
    pub fn registry(&self) -> &AdapterRegistry {
        &self.registry
    }

    /// Get a mutable reference to the registry
    pub fn registry_mut(&mut self) -> &mut AdapterRegistry {
        &mut self.registry
    }
}

impl Default for TreeSitterProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AstProvider for TreeSitterProvider {
    fn index_project(&self, root: &Path, options: &IndexOptions) -> Result<PlanetariumModel> {
        let start = Instant::now();
        let mut model = PlanetariumModel::new(root.display().to_string());

        // Collect files to process
        let files = self.collect_files(root, options)?;

        let mut stats = IndexStats::default();

        for file_path in files {
            if options.max_files > 0 && stats.files_processed >= options.max_files {
                break;
            }

            match self.process_file(&file_path, root, options) {
                Ok(Some(file)) => {
                    // Update stats
                    stats.files_processed += 1;
                    stats.declarations_found += file.total_declarations();
                    stats.imports_found += file.imports.len();
                    stats.unknown_regions += file.unknown_regions.len();

                    // Update per-language stats
                    let lang_stats = stats
                        .by_language
                        .entry(file.language.name().to_string())
                        .or_insert_with(LanguageStats::default);
                    lang_stats.files += 1;
                    lang_stats.declarations += file.total_declarations();
                    lang_stats.imports += file.imports.len();

                    // Store file
                    let relative_path = file_path
                        .strip_prefix(root)
                        .unwrap_or(&file_path)
                        .to_string_lossy()
                        .to_string();
                    model.files.insert(relative_path, file);
                }
                Ok(None) => {
                    stats.files_skipped += 1;
                }
                Err(e) => {
                    model.errors.push(IndexError {
                        path: file_path.display().to_string(),
                        message: e.to_string(),
                        recoverable: e.has_partial(),
                    });

                    // Still add partial results if available
                    if let Some(partial) = e.take_partial() {
                        let relative_path = file_path
                            .strip_prefix(root)
                            .unwrap_or(&file_path)
                            .to_string_lossy()
                            .to_string();
                        model.files.insert(relative_path, partial);
                    }
                }
            }
        }

        stats.parse_time_ms = start.elapsed().as_millis() as u64;
        model.stats = stats;

        Ok(model)
    }

    fn zoom_into(
        &self,
        file_path: &Path,
        symbol_id: &str,
        options: &ZoomOptions,
    ) -> Result<MicroscopeModel> {
        // Read the file
        let source = std::fs::read_to_string(file_path)
            .map_err(|e| AstError::IoError(e.to_string()))?;

        // Detect language
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = LanguageId::from_extension(ext);

        // Parse the file
        let file = self.parse_file(&source, language)?;

        // Find the symbol
        let declaration = file
            .declarations
            .iter()
            .find(|d| d.id() == symbol_id)
            .or_else(|| {
                // Search in nested declarations
                file.declarations
                    .iter()
                    .flat_map(|d| d.children.iter())
                    .find(|d| d.id() == symbol_id)
            })
            .cloned()
            .ok_or_else(|| AstError::SymbolNotFound {
                file: file_path.display().to_string(),
                symbol: symbol_id.to_string(),
            })?;

        // Get adapter and extract body
        let adapter = self.registry.get(language).ok_or_else(|| {
            AstError::UnsupportedLanguage(language)
        })?;

        // Re-parse to get tree for body extraction
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .map_err(|e| AstError::TreeSitterError(e.to_string()))?;
        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| AstError::parse_error("Failed to parse for zoom"))?;

        let body = if options.extract_control_flow || options.extract_calls {
            adapter.extract_body(&tree, &source, &declaration)
        } else {
            None
        };

        // Extract context if requested
        let context = if options.context_lines > 0 {
            let lines: Vec<&str> = source.lines().collect();
            let start_line = declaration.span.start_line.saturating_sub(1);
            let end_line = declaration.span.end_line;

            let before_start = start_line.saturating_sub(options.context_lines);
            let after_end = (end_line + options.context_lines).min(lines.len());

            Some(crate::provider::ContextWindow {
                before: lines[before_start..start_line]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                after: lines[end_line..after_end]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            })
        } else {
            None
        };

        // Extract source text
        let source_text = Some(
            source[declaration.span.start..declaration.span.end].to_string(),
        );

        Ok(MicroscopeModel {
            file_path: file_path.display().to_string(),
            symbol: declaration,
            body,
            context,
            source_text,
        })
    }

    fn parse_file(&self, source: &str, language: LanguageId) -> Result<File> {
        self.registry.parse(source, language)
    }

    fn supported_languages(&self) -> &[LanguageId] {
        // Core Fleet (Phase 1B): Rust, Python, TypeScript, TSX, JavaScript
        static LANGUAGES: &[LanguageId] = &[
            LanguageId::Rust,
            LanguageId::Python,
            LanguageId::TypeScript,
            LanguageId::Tsx,
            LanguageId::JavaScript,
        ];
        LANGUAGES
    }
}

impl TreeSitterProvider {
    /// Collect files to process
    fn collect_files(&self, root: &Path, options: &IndexOptions) -> Result<Vec<std::path::PathBuf>> {
        use std::fs;

        let mut files = Vec::new();

        fn visit_dir(
            dir: &Path,
            files: &mut Vec<std::path::PathBuf>,
            options: &IndexOptions,
            registry: &AdapterRegistry,
        ) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                // Skip hidden files and directories
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }

                // Skip common non-source directories
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if matches!(name, "node_modules" | "target" | "build" | "dist" | "__pycache__" | ".git") {
                        continue;
                    }

                    if options.follow_symlinks || !path.is_symlink() {
                        visit_dir(&path, files, options, registry)?;
                    }
                } else if path.is_file() {
                    // Check if we support this file type
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let language = LanguageId::from_extension(ext);

                    if registry.supports(language) {
                        // Check include patterns
                        if !options.include_patterns.is_empty() {
                            // Simple glob matching (could use globset for full support)
                            let path_str = path.to_string_lossy();
                            let matches = options.include_patterns.iter().any(|p| {
                                path_str.contains(p.trim_start_matches('*').trim_end_matches('*'))
                            });
                            if !matches {
                                continue;
                            }
                        }

                        // Check exclude patterns
                        if !options.exclude_patterns.is_empty() {
                            let path_str = path.to_string_lossy();
                            let excluded = options.exclude_patterns.iter().any(|p| {
                                path_str.contains(p.trim_start_matches('*').trim_end_matches('*'))
                            });
                            if excluded {
                                continue;
                            }
                        }

                        files.push(path);
                    }
                }
            }
            Ok(())
        }

        visit_dir(root, &mut files, options, &self.registry)
            .map_err(|e| AstError::IoError(e.to_string()))?;

        // Sort for determinism
        files.sort();

        Ok(files)
    }

    /// Process a single file
    fn process_file(
        &self,
        path: &Path,
        _root: &Path,
        _options: &IndexOptions,
    ) -> Result<Option<File>> {
        // Read file
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                // Skip binary files silently
                if e.kind() == std::io::ErrorKind::InvalidData {
                    return Ok(None);
                }
                return Err(AstError::IoError(e.to_string()));
            }
        };

        // Skip very large files
        if source.len() > 1_000_000 {
            return Ok(None);
        }

        // Detect language
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let language = LanguageId::from_extension(ext);

        if !self.registry.supports(language) {
            return Ok(None);
        }

        // Parse
        let mut file = self.registry.parse(&source, language)?;
        file.path = path.display().to_string();

        Ok(Some(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_provider_parse_rust() {
        let provider = TreeSitterProvider::new();
        let source = r#"
pub fn hello() {
    println!("Hello!");
}
"#;
        let file = provider.parse_file(source, LanguageId::Rust).unwrap();
        assert_eq!(file.declarations.len(), 1);
        assert_eq!(file.declarations[0].name, "hello");
    }

    #[test]
    fn test_provider_parse_python() {
        let provider = TreeSitterProvider::new();
        let source = r#"
def greet(name):
    """Say hello to someone."""
    print(f"Hello, {name}!")

class Greeter:
    def __init__(self, default_name):
        self.default_name = default_name
"#;
        let file = provider.parse_file(source, LanguageId::Python).unwrap();
        assert_eq!(file.declarations.len(), 2, "Expected 2 declarations (function + class)");
        assert_eq!(file.declarations[0].name, "greet");
        assert_eq!(file.declarations[1].name, "Greeter");
    }

    #[test]
    fn test_provider_parse_typescript() {
        let provider = TreeSitterProvider::new();
        let source = r#"
interface User {
    name: string;
    age: number;
}

function createUser(name: string, age: number): User {
    return { name, age };
}

class UserManager {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }
}
"#;
        let file = provider.parse_file(source, LanguageId::TypeScript).unwrap();
        assert!(file.declarations.len() >= 3, "Expected at least 3 declarations (interface + function + class)");
    }

    #[test]
    fn test_provider_parse_javascript() {
        let provider = TreeSitterProvider::new();
        let source = r#"
function add(a, b) {
    return a + b;
}

const multiply = (a, b) => a * b;

class Calculator {
    constructor() {
        this.result = 0;
    }
}
"#;
        let file = provider.parse_file(source, LanguageId::JavaScript).unwrap();
        assert!(file.declarations.len() >= 2, "Expected at least 2 declarations");
    }

    #[test]
    fn test_unsupported_language() {
        let provider = TreeSitterProvider::new();
        let result = provider.parse_file("some code", LanguageId::Unknown);
        assert!(matches!(result, Err(AstError::UnsupportedLanguage(_))));
    }
}
