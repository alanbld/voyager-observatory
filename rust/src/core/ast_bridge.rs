//! AST Bridge - Integration with voyager-ast
//!
//! This module provides the bridge between Voyager Observatory's context engine
//! and the voyager-ast structural optics layer. It enables AST-based analysis
//! for supported languages while falling back to regex heuristics for others.
//!
//! # Design
//!
//! The bridge follows the "telescope, not compiler" philosophy:
//! - Best-effort parsing with graceful fallback
//! - No failures propagated to the user for unsupported languages
//! - AST results enhance (not replace) existing functionality

use std::path::Path;
use voyager_ast::{
    AdapterRegistry, AstError, Declaration, DeclarationKind, File as AstFile,
    LanguageId, Visibility,
};

/// Bridge for AST-based code analysis
pub struct AstBridge {
    registry: AdapterRegistry,
}

impl AstBridge {
    /// Create a new AST bridge with all available adapters
    pub fn new() -> Self {
        Self {
            registry: AdapterRegistry::new(),
        }
    }

    /// Check if a language is supported for AST analysis
    pub fn supports(&self, language: LanguageId) -> bool {
        self.registry.supports(language)
    }

    /// Detect language from file extension
    pub fn detect_language(path: &Path) -> LanguageId {
        path.extension()
            .and_then(|e| e.to_str())
            .map(LanguageId::from_extension)
            .unwrap_or(LanguageId::Unknown)
    }

    /// Analyze a source file using AST-based parsing
    ///
    /// Returns structured information about the file, or None if:
    /// - The language is not supported
    /// - Parsing completely fails
    ///
    /// Note: Partial results with errors are still returned (with unknown_regions)
    pub fn analyze_file(&self, source: &str, language: LanguageId) -> Option<AstFile> {
        if !self.supports(language) {
            return None;
        }

        match self.registry.parse(source, language) {
            Ok(file) => Some(file),
            Err(e) => {
                // Try to extract partial results
                if let AstError::ParseError { partial: Some(file), .. } = e {
                    Some(*file)
                } else {
                    None
                }
            }
        }
    }

    /// Extract "Stars" (significant symbols) from an AST file
    ///
    /// Stars are the key navigation points in the code that users and LLMs
    /// care about most. This translates AST declarations into the VO metaphor.
    pub fn extract_stars(&self, file: &AstFile) -> Vec<Star> {
        let mut stars = Vec::new();

        for decl in &file.declarations {
            stars.push(Star::from_declaration(decl, &file.path));

            // Also extract nested stars (methods, inner types)
            for child in &decl.children {
                stars.push(Star::from_declaration(child, &file.path));
            }
        }

        stars
    }

    /// Get a summary of the file structure for context generation
    pub fn get_file_summary(&self, file: &AstFile) -> FileSummary {
        let mut summary = FileSummary {
            path: file.path.clone(),
            language: file.language.name().to_string(),
            total_declarations: file.total_declarations(),
            import_count: file.imports.len(),
            has_errors: file.has_errors(),
            stars: Vec::new(),
        };

        // Collect top-level stars
        for decl in &file.declarations {
            summary.stars.push(StarSummary {
                name: decl.name.clone(),
                kind: declaration_kind_to_string(decl.kind),
                visibility: visibility_to_string(decl.visibility),
                line: decl.span.start_line,
                has_doc: decl.doc_comment.is_some(),
            });
        }

        summary
    }
}

impl Default for AstBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// A "Star" - a significant navigational point in the code
///
/// In the Voyager Observatory metaphor, stars are the bright points
/// that help users and LLMs navigate the codebase.
#[derive(Debug, Clone)]
pub struct Star {
    /// Name of the star (function name, class name, etc.)
    pub name: String,

    /// What kind of star this is
    pub kind: StarKind,

    /// File path where this star is located
    pub file_path: String,

    /// Line number (1-indexed)
    pub line: usize,

    /// End line number
    pub end_line: usize,

    /// Whether this star is publicly visible
    pub is_public: bool,

    /// Documentation summary (first line of doc comment)
    pub doc_summary: Option<String>,

    /// Child stars (methods, nested types)
    pub children: Vec<Star>,
}

impl Star {
    /// Create a star from an AST declaration
    pub fn from_declaration(decl: &Declaration, file_path: &str) -> Self {
        let kind = match decl.kind {
            DeclarationKind::Function => StarKind::Function,
            DeclarationKind::Method => StarKind::Method,
            DeclarationKind::Class => StarKind::Class,
            DeclarationKind::Struct => StarKind::Struct,
            DeclarationKind::Enum => StarKind::Enum,
            DeclarationKind::Interface => StarKind::Interface,
            DeclarationKind::Trait => StarKind::Trait,
            DeclarationKind::Type => StarKind::Type,
            DeclarationKind::Constant => StarKind::Constant,
            DeclarationKind::Variable => StarKind::Variable,
            DeclarationKind::Module => StarKind::Module,
            DeclarationKind::Namespace => StarKind::Namespace,
            DeclarationKind::Impl => StarKind::Implementation,
            DeclarationKind::Macro => StarKind::Macro,
            DeclarationKind::Other => StarKind::Other,
        };

        let is_public = matches!(decl.visibility, Visibility::Public);

        let doc_summary = decl.doc_comment.as_ref().map(|c| {
            c.text.lines().next().unwrap_or("").to_string()
        });

        let children = decl.children.iter()
            .map(|c| Star::from_declaration(c, file_path))
            .collect();

        Self {
            name: decl.name.clone(),
            kind,
            file_path: file_path.to_string(),
            line: decl.span.start_line,
            end_line: decl.span.end_line,
            is_public,
            doc_summary,
            children,
        }
    }

    /// Get a human-friendly label for this star
    pub fn label(&self) -> String {
        format!("{} {}", self.kind.emoji(), self.name)
    }
}

/// Kind of star (maps to declaration kinds with VO metaphor)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Type,
    Constant,
    Variable,
    Module,
    Namespace,
    Implementation,
    Macro,
    Other,
}

impl StarKind {
    /// Get the emoji for this star kind (for human-friendly output)
    pub fn emoji(&self) -> &'static str {
        match self {
            StarKind::Function => "fn",
            StarKind::Method => "fn",
            StarKind::Class => "class",
            StarKind::Struct => "struct",
            StarKind::Enum => "enum",
            StarKind::Interface => "interface",
            StarKind::Trait => "trait",
            StarKind::Type => "type",
            StarKind::Constant => "const",
            StarKind::Variable => "var",
            StarKind::Module => "mod",
            StarKind::Namespace => "ns",
            StarKind::Implementation => "impl",
            StarKind::Macro => "macro",
            StarKind::Other => "?",
        }
    }

    /// Get the human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            StarKind::Function => "function",
            StarKind::Method => "method",
            StarKind::Class => "class",
            StarKind::Struct => "struct",
            StarKind::Enum => "enum",
            StarKind::Interface => "interface",
            StarKind::Trait => "trait",
            StarKind::Type => "type alias",
            StarKind::Constant => "constant",
            StarKind::Variable => "variable",
            StarKind::Module => "module",
            StarKind::Namespace => "namespace",
            StarKind::Implementation => "implementation",
            StarKind::Macro => "macro",
            StarKind::Other => "other",
        }
    }
}

/// Summary of a file's structure
#[derive(Debug, Clone)]
pub struct FileSummary {
    pub path: String,
    pub language: String,
    pub total_declarations: usize,
    pub import_count: usize,
    pub has_errors: bool,
    pub stars: Vec<StarSummary>,
}

/// Summary of a single star
#[derive(Debug, Clone)]
pub struct StarSummary {
    pub name: String,
    pub kind: String,
    pub visibility: String,
    pub line: usize,
    pub has_doc: bool,
}

/// Helper to convert declaration kind to string
fn declaration_kind_to_string(kind: DeclarationKind) -> String {
    match kind {
        DeclarationKind::Function => "function",
        DeclarationKind::Method => "method",
        DeclarationKind::Class => "class",
        DeclarationKind::Struct => "struct",
        DeclarationKind::Enum => "enum",
        DeclarationKind::Interface => "interface",
        DeclarationKind::Trait => "trait",
        DeclarationKind::Type => "type",
        DeclarationKind::Constant => "constant",
        DeclarationKind::Variable => "variable",
        DeclarationKind::Module => "module",
        DeclarationKind::Namespace => "namespace",
        DeclarationKind::Impl => "impl",
        DeclarationKind::Macro => "macro",
        DeclarationKind::Other => "other",
    }.to_string()
}

/// Helper to convert visibility to string
fn visibility_to_string(vis: Visibility) -> String {
    match vis {
        Visibility::Public => "public",
        Visibility::Private => "private",
        Visibility::Protected => "protected",
        Visibility::Internal => "internal",
        Visibility::Unknown => "unknown",
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = AstBridge::new();
        assert!(bridge.supports(LanguageId::Rust));
    }

    #[test]
    fn test_rust_analysis() {
        let bridge = AstBridge::new();
        let source = r#"
/// A greeting function
pub fn hello() {
    println!("Hello!");
}

struct Point {
    x: f64,
    y: f64,
}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();

        // Should find at least the function and struct
        assert!(file.declarations.len() >= 2,
            "Expected at least 2 declarations, got {}", file.declarations.len());

        let stars = bridge.extract_stars(&file);
        // Stars include both top-level declarations and nested children
        assert!(stars.len() >= 2,
            "Expected at least 2 stars, got {}", stars.len());

        // Find the hello function
        let hello = stars.iter().find(|s| s.name == "hello");
        assert!(hello.is_some(), "Should find 'hello' function");
        let hello = hello.unwrap();
        assert!(hello.is_public);
        assert!(hello.doc_summary.is_some());
    }

    #[test]
    fn test_unsupported_language() {
        let bridge = AstBridge::new();
        // Go is not yet supported in voyager-ast (Phase 1B Core Fleet: Rust, Python, TypeScript, JavaScript)
        let result = bridge.analyze_file("package main\nfunc main() {}", LanguageId::Go);
        assert!(result.is_none());
    }

    #[test]
    fn test_python_analysis() {
        let bridge = AstBridge::new();
        // Python is supported (Phase 1B Core Fleet)
        let result = bridge.analyze_file("def greet(name): pass\nclass User: pass", LanguageId::Python);
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(file.declarations.len() >= 2, "Expected at least function and class, got {}", file.declarations.len());
    }

    #[test]
    fn test_typescript_analysis() {
        let bridge = AstBridge::new();
        // TypeScript is supported (Phase 1B Core Fleet)
        let result = bridge.analyze_file("function greet(name: string): void {}\ninterface User { name: string; }", LanguageId::TypeScript);
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(file.declarations.len() >= 2, "Expected at least function and interface, got {}", file.declarations.len());
    }

    #[test]
    fn test_javascript_analysis() {
        let bridge = AstBridge::new();
        // JavaScript is supported (Phase 1B Core Fleet)
        let result = bridge.analyze_file("function greet(name) { return 'Hello ' + name; }\nclass Calculator {}", LanguageId::JavaScript);
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(file.declarations.len() >= 2, "Expected at least function and class, got {}", file.declarations.len());
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(AstBridge::detect_language(Path::new("main.rs")), LanguageId::Rust);
        assert_eq!(AstBridge::detect_language(Path::new("app.py")), LanguageId::Python);
        assert_eq!(AstBridge::detect_language(Path::new("index.ts")), LanguageId::TypeScript);
    }

    #[test]
    fn test_file_summary() {
        let bridge = AstBridge::new();
        let source = r#"
pub fn foo() {}
fn bar() {}
struct Baz {}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let summary = bridge.get_file_summary(&file);

        assert_eq!(summary.total_declarations, 3);
        assert_eq!(summary.stars.len(), 3);
    }
}
