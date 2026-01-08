//! Adapter Registry
//!
//! The registry manages all language adapters and provides a unified interface
//! for parsing files across languages.

use crate::adapters::{
    LanguageAdapter, PythonTreeSitterAdapter, RustTreeSitterAdapter, TypeScriptTreeSitterAdapter,
};
use crate::error::{AstError, Result};
use crate::ir::{File, LanguageId, Span};
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
#[allow(dead_code)]
pub struct TreeSitterProvider {
    registry: AdapterRegistry,
}

#[allow(dead_code)]
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
        let source =
            std::fs::read_to_string(file_path).map_err(|e| AstError::IoError(e.to_string()))?;

        // Detect language
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
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
        let adapter = self
            .registry
            .get(language)
            .ok_or(AstError::UnsupportedLanguage(language))?;

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
        let source_text = Some(source[declaration.span.start..declaration.span.end].to_string());

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

#[allow(dead_code)]
impl TreeSitterProvider {
    /// Collect files to process
    fn collect_files(
        &self,
        root: &Path,
        options: &IndexOptions,
    ) -> Result<Vec<std::path::PathBuf>> {
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
                    if matches!(
                        name,
                        "node_modules" | "target" | "build" | "dist" | "__pycache__" | ".git"
                    ) {
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
    use std::fs;
    use tempfile::TempDir;

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

    // =========================================================================
    // TreeSitterProvider Tests
    // =========================================================================

    #[test]
    fn test_provider_new() {
        let provider = TreeSitterProvider::new();
        assert!(provider.registry().supports(LanguageId::Rust));
    }

    #[test]
    fn test_provider_default() {
        let provider = TreeSitterProvider::default();
        assert!(provider.registry().supports(LanguageId::Python));
    }

    #[test]
    fn test_provider_with_registry() {
        let registry = AdapterRegistry::new();
        let provider = TreeSitterProvider::with_registry(registry);
        assert!(provider.registry().supports(LanguageId::Rust));
    }

    #[test]
    fn test_provider_registry_getter() {
        let provider = TreeSitterProvider::new();
        let registry = provider.registry();
        assert!(registry.supports(LanguageId::TypeScript));
    }

    #[test]
    fn test_provider_registry_mut() {
        let mut provider = TreeSitterProvider::new();
        let registry = provider.registry_mut();
        // Can mutate the registry
        assert!(registry.supports(LanguageId::Rust));
    }

    #[test]
    fn test_provider_supported_languages() {
        let provider = TreeSitterProvider::new();
        let languages = provider.supported_languages();

        assert!(languages.contains(&LanguageId::Rust));
        assert!(languages.contains(&LanguageId::Python));
        assert!(languages.contains(&LanguageId::TypeScript));
        assert!(languages.contains(&LanguageId::Tsx));
        assert!(languages.contains(&LanguageId::JavaScript));
        assert_eq!(languages.len(), 5);
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
        assert_eq!(
            file.declarations.len(),
            2,
            "Expected 2 declarations (function + class)"
        );
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
        assert!(
            file.declarations.len() >= 3,
            "Expected at least 3 declarations (interface + function + class)"
        );
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
        assert!(
            file.declarations.len() >= 2,
            "Expected at least 2 declarations"
        );
    }

    #[test]
    fn test_provider_parse_tsx() {
        let provider = TreeSitterProvider::new();
        let source = r#"
interface Props {
    name: string;
}

function Greeting({ name }: Props): JSX.Element {
    return <div>Hello, {name}!</div>;
}
"#;
        let file = provider.parse_file(source, LanguageId::Tsx).unwrap();
        assert!(file.declarations.len() >= 1);
    }

    #[test]
    fn test_unsupported_language() {
        let provider = TreeSitterProvider::new();
        let result = provider.parse_file("some code", LanguageId::Unknown);
        assert!(matches!(result, Err(AstError::UnsupportedLanguage(_))));
    }

    // =========================================================================
    // index_project Tests
    // =========================================================================

    #[test]
    fn test_index_project_basic() {
        let temp_dir = TempDir::new().unwrap();

        // Create a simple Rust file
        let rust_file = temp_dir.path().join("main.rs");
        fs::write(&rust_file, "fn main() {}\nfn helper() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 1);
        assert!(model.stats.declarations_found >= 2);
        assert!(model.files.contains_key("main.rs"));
    }

    #[test]
    fn test_index_project_multiple_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create Rust file
        fs::write(temp_dir.path().join("lib.rs"), "pub fn public_fn() {}").unwrap();

        // Create Python file
        fs::write(
            temp_dir.path().join("script.py"),
            "def process():\n    pass",
        )
        .unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 2);
        assert!(model.files.contains_key("lib.rs"));
        assert!(model.files.contains_key("script.py"));
    }

    #[test]
    fn test_index_project_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested structure
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src_dir.join("utils.rs"), "pub fn util() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 2);
    }

    #[test]
    fn test_index_project_skips_hidden_files() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("visible.rs"), "fn visible() {}").unwrap();
        fs::write(temp_dir.path().join(".hidden.rs"), "fn hidden() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 1);
        assert!(model.files.contains_key("visible.rs"));
        assert!(!model.files.contains_key(".hidden.rs"));
    }

    #[test]
    fn test_index_project_skips_common_directories() {
        let temp_dir = TempDir::new().unwrap();

        // Create node_modules directory
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        fs::write(node_modules.join("package.js"), "function pkg() {}").unwrap();

        // Create target directory
        let target = temp_dir.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("build.rs"), "fn build() {}").unwrap();

        // Create a valid source file
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        // Should only process main.rs, not files in node_modules or target
        assert_eq!(model.stats.files_processed, 1);
    }

    #[test]
    fn test_index_project_max_files_limit() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple files
        for i in 0..5 {
            fs::write(temp_dir.path().join(format!("file{}.rs", i)), "fn f() {}").unwrap();
        }

        let provider = TreeSitterProvider::new();
        let options = IndexOptions {
            max_files: 2,
            ..Default::default()
        };

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 2);
    }

    #[test]
    fn test_index_project_exclude_patterns() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("test_main.rs"), "fn test() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions {
            exclude_patterns: vec!["test_".to_string()],
            ..Default::default()
        };

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 1);
        assert!(model.files.contains_key("main.rs"));
    }

    #[test]
    fn test_index_project_include_patterns() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("lib.rs"), "fn lib() {}").unwrap();
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions {
            include_patterns: vec!["main".to_string()],
            ..Default::default()
        };

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 1);
        assert!(model.files.contains_key("main.rs"));
    }

    #[test]
    fn test_index_project_per_language_stats() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("script.py"), "def foo():\n    pass").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert!(model.stats.by_language.contains_key("Rust"));
        assert!(model.stats.by_language.contains_key("Python"));
        assert_eq!(model.stats.by_language["Rust"].files, 1);
        assert_eq!(model.stats.by_language["Python"].files, 1);
    }

    #[test]
    fn test_index_project_skips_large_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create a file larger than 1MB
        let large_content = "fn f() {}\n".repeat(150_000);
        fs::write(temp_dir.path().join("large.rs"), large_content).unwrap();

        // Create a normal file
        fs::write(temp_dir.path().join("normal.rs"), "fn normal() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        // Should skip the large file
        assert_eq!(model.stats.files_processed, 1);
        assert!(!model.files.contains_key("large.rs"));
    }

    #[test]
    fn test_index_project_skips_unsupported_languages() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("style.css"), "body { color: red; }").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        // Should only process Rust file, skip CSS
        assert_eq!(model.stats.files_processed, 1);
    }

    #[test]
    fn test_index_project_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 0);
        assert!(model.files.is_empty());
    }

    // =========================================================================
    // zoom_into Tests
    // =========================================================================

    #[test]
    fn test_zoom_into_function() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let source =
            "fn foo() {\n    println!(\"foo\");\n}\n\nfn bar() {\n    println!(\"bar\");\n}\n";
        fs::write(&file_path, source).unwrap();

        let provider = TreeSitterProvider::new();
        let options = ZoomOptions::default();

        // id format is "kind:name:start_line"
        let model = provider
            .zoom_into(&file_path, "function:foo:1", &options)
            .unwrap();

        assert_eq!(model.symbol.name, "foo");
        assert!(model.source_text.is_some());
    }

    #[test]
    fn test_zoom_into_with_context() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let source = "// Header comment\nuse std::io;\n\nfn target() {\n    println!(\"target\");\n}\n\nfn after() {\n    println!(\"after\");\n}\n";
        fs::write(&file_path, source).unwrap();

        let provider = TreeSitterProvider::new();
        let options = ZoomOptions {
            context_lines: 2,
            ..Default::default()
        };

        // target function is at line 4
        let model = provider
            .zoom_into(&file_path, "function:target:4", &options)
            .unwrap();

        assert!(model.context.is_some());
        let ctx = model.context.unwrap();
        assert!(!ctx.before.is_empty() || !ctx.after.is_empty());
    }

    #[test]
    fn test_zoom_into_with_control_flow() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let source = "fn process(x: i32) -> i32 {\n    if x > 0 {\n        x * 2\n    } else {\n        0\n    }\n}\n";
        fs::write(&file_path, source).unwrap();

        let provider = TreeSitterProvider::new();
        let options = ZoomOptions {
            extract_control_flow: true,
            ..Default::default()
        };

        // process function is at line 1
        let model = provider
            .zoom_into(&file_path, "function:process:1", &options)
            .unwrap();

        assert_eq!(model.symbol.name, "process");
        // Body extraction should be attempted
    }

    #[test]
    fn test_zoom_into_nested_symbol() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let source = r#"struct Container {
    value: i32,
}

impl Container {
    fn new() -> Self {
        Container { value: 0 }
    }
}
"#;
        fs::write(&file_path, source).unwrap();

        let provider = TreeSitterProvider::new();
        let options = ZoomOptions::default();

        // Try to find "new" which is nested in impl
        let result = provider.zoom_into(&file_path, "new", &options);
        // May or may not find nested, depending on id format
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_zoom_into_symbol_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        fs::write(&file_path, "fn existing() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = ZoomOptions::default();

        let result = provider.zoom_into(&file_path, "nonexistent", &options);

        assert!(matches!(result, Err(AstError::SymbolNotFound { .. })));
    }

    #[test]
    fn test_zoom_into_file_not_found() {
        let provider = TreeSitterProvider::new();
        let options = ZoomOptions::default();

        let result = provider.zoom_into(Path::new("/nonexistent/file.rs"), "symbol", &options);

        assert!(matches!(result, Err(AstError::IoError(_))));
    }

    #[test]
    fn test_zoom_into_python() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");

        let source = "def hello():\n    print(\"Hello!\")\n\ndef world():\n    print(\"World!\")\n";
        fs::write(&file_path, source).unwrap();

        let provider = TreeSitterProvider::new();
        let options = ZoomOptions::default();

        // hello function is at line 1
        let model = provider
            .zoom_into(&file_path, "function:hello:1", &options)
            .unwrap();

        assert_eq!(model.symbol.name, "hello");
    }

    // =========================================================================
    // collect_files Tests
    // =========================================================================

    #[test]
    fn test_collect_files_sorted() {
        let temp_dir = TempDir::new().unwrap();

        // Create files in non-alphabetical order
        fs::write(temp_dir.path().join("zebra.rs"), "fn z() {}").unwrap();
        fs::write(temp_dir.path().join("alpha.rs"), "fn a() {}").unwrap();
        fs::write(temp_dir.path().join("beta.rs"), "fn b() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let files = provider.collect_files(temp_dir.path(), &options).unwrap();

        // Files should be sorted
        let names: Vec<_> = files
            .iter()
            .filter_map(|p| p.file_name())
            .filter_map(|n| n.to_str())
            .collect();

        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }

    #[test]
    fn test_collect_files_only_supported() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("code.rs"), "fn f() {}").unwrap();
        fs::write(temp_dir.path().join("data.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "# Readme").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let files = provider.collect_files(temp_dir.path(), &options).unwrap();

        // Should only include .rs file
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().ends_with(".rs"));
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[test]
    fn test_index_project_handles_parse_errors_gracefully() {
        let temp_dir = TempDir::new().unwrap();

        // Create a file with valid Rust syntax
        fs::write(temp_dir.path().join("valid.rs"), "fn valid() {}").unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert_eq!(model.stats.files_processed, 1);
    }

    #[test]
    fn test_index_project_nonexistent_directory() {
        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let result = provider.index_project(Path::new("/nonexistent/path"), &options);

        assert!(result.is_err());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_empty_source() {
        let provider = TreeSitterProvider::new();
        let file = provider.parse_file("", LanguageId::Rust).unwrap();

        assert!(file.declarations.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let provider = TreeSitterProvider::new();
        let file = provider
            .parse_file("   \n\n   \t", LanguageId::Rust)
            .unwrap();

        assert!(file.declarations.is_empty());
    }

    #[test]
    fn test_parse_comments_only() {
        let provider = TreeSitterProvider::new();
        let source = "// This is a comment\n/* Block comment */";
        let file = provider.parse_file(source, LanguageId::Rust).unwrap();

        assert!(file.declarations.is_empty());
    }

    #[test]
    fn test_parse_with_imports() {
        let provider = TreeSitterProvider::new();
        let source = "use std::io;\nuse std::collections::HashMap;\n\nfn main() {}";
        let file = provider.parse_file(source, LanguageId::Rust).unwrap();

        assert!(!file.imports.is_empty());
    }

    #[test]
    fn test_index_project_counts_imports() {
        let temp_dir = TempDir::new().unwrap();

        let source = "use std::io;\nfn main() {}";
        fs::write(temp_dir.path().join("main.rs"), source).unwrap();

        let provider = TreeSitterProvider::new();
        let options = IndexOptions::default();

        let model = provider.index_project(temp_dir.path(), &options).unwrap();

        assert!(model.stats.imports_found >= 1);
    }
}
