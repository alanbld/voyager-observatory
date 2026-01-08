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
    AdapterRegistry, AstError, Declaration, DeclarationKind, File as AstFile, LanguageId,
    Visibility,
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
                if let AstError::ParseError {
                    partial: Some(file),
                    ..
                } = e
                {
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

        let doc_summary = decl
            .doc_comment
            .as_ref()
            .map(|c| c.text.lines().next().unwrap_or("").to_string());

        let children = decl
            .children
            .iter()
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
    }
    .to_string()
}

/// Helper to convert visibility to string
fn visibility_to_string(vis: Visibility) -> String {
    match vis {
        Visibility::Public => "public",
        Visibility::Private => "private",
        Visibility::Protected => "protected",
        Visibility::Internal => "internal",
        Visibility::Unknown => "unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Bridge Creation Tests ====================

    #[test]
    fn test_bridge_creation() {
        let bridge = AstBridge::new();
        assert!(bridge.supports(LanguageId::Rust));
    }

    #[test]
    fn test_bridge_default() {
        let bridge = AstBridge::default();
        assert!(bridge.supports(LanguageId::Rust));
        assert!(bridge.supports(LanguageId::Python));
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
        assert!(
            file.declarations.len() >= 2,
            "Expected at least 2 declarations, got {}",
            file.declarations.len()
        );

        let stars = bridge.extract_stars(&file);
        // Stars include both top-level declarations and nested children
        assert!(
            stars.len() >= 2,
            "Expected at least 2 stars, got {}",
            stars.len()
        );

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
        let result = bridge.analyze_file(
            "def greet(name): pass\nclass User: pass",
            LanguageId::Python,
        );
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(
            file.declarations.len() >= 2,
            "Expected at least function and class, got {}",
            file.declarations.len()
        );
    }

    #[test]
    fn test_typescript_analysis() {
        let bridge = AstBridge::new();
        // TypeScript is supported (Phase 1B Core Fleet)
        let result = bridge.analyze_file(
            "function greet(name: string): void {}\ninterface User { name: string; }",
            LanguageId::TypeScript,
        );
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(
            file.declarations.len() >= 2,
            "Expected at least function and interface, got {}",
            file.declarations.len()
        );
    }

    #[test]
    fn test_javascript_analysis() {
        let bridge = AstBridge::new();
        // JavaScript is supported (Phase 1B Core Fleet)
        let result = bridge.analyze_file(
            "function greet(name) { return 'Hello ' + name; }\nclass Calculator {}",
            LanguageId::JavaScript,
        );
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(
            file.declarations.len() >= 2,
            "Expected at least function and class, got {}",
            file.declarations.len()
        );
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(
            AstBridge::detect_language(Path::new("main.rs")),
            LanguageId::Rust
        );
        assert_eq!(
            AstBridge::detect_language(Path::new("app.py")),
            LanguageId::Python
        );
        assert_eq!(
            AstBridge::detect_language(Path::new("index.ts")),
            LanguageId::TypeScript
        );
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

    // ==================== StarKind Tests ====================

    #[test]
    fn test_star_kind_emoji() {
        assert_eq!(StarKind::Function.emoji(), "fn");
        assert_eq!(StarKind::Method.emoji(), "fn");
        assert_eq!(StarKind::Class.emoji(), "class");
        assert_eq!(StarKind::Struct.emoji(), "struct");
        assert_eq!(StarKind::Enum.emoji(), "enum");
        assert_eq!(StarKind::Interface.emoji(), "interface");
        assert_eq!(StarKind::Trait.emoji(), "trait");
        assert_eq!(StarKind::Type.emoji(), "type");
        assert_eq!(StarKind::Constant.emoji(), "const");
        assert_eq!(StarKind::Variable.emoji(), "var");
        assert_eq!(StarKind::Module.emoji(), "mod");
        assert_eq!(StarKind::Namespace.emoji(), "ns");
        assert_eq!(StarKind::Implementation.emoji(), "impl");
        assert_eq!(StarKind::Macro.emoji(), "macro");
        assert_eq!(StarKind::Other.emoji(), "?");
    }

    #[test]
    fn test_star_kind_name() {
        assert_eq!(StarKind::Function.name(), "function");
        assert_eq!(StarKind::Method.name(), "method");
        assert_eq!(StarKind::Class.name(), "class");
        assert_eq!(StarKind::Struct.name(), "struct");
        assert_eq!(StarKind::Enum.name(), "enum");
        assert_eq!(StarKind::Interface.name(), "interface");
        assert_eq!(StarKind::Trait.name(), "trait");
        assert_eq!(StarKind::Type.name(), "type alias");
        assert_eq!(StarKind::Constant.name(), "constant");
        assert_eq!(StarKind::Variable.name(), "variable");
        assert_eq!(StarKind::Module.name(), "module");
        assert_eq!(StarKind::Namespace.name(), "namespace");
        assert_eq!(StarKind::Implementation.name(), "implementation");
        assert_eq!(StarKind::Macro.name(), "macro");
        assert_eq!(StarKind::Other.name(), "other");
    }

    #[test]
    fn test_star_kind_equality() {
        assert_eq!(StarKind::Function, StarKind::Function);
        assert_ne!(StarKind::Function, StarKind::Method);
    }

    // ==================== Star Tests ====================

    #[test]
    fn test_star_label() {
        let star = Star {
            name: "my_function".to_string(),
            kind: StarKind::Function,
            file_path: "test.rs".to_string(),
            line: 1,
            end_line: 5,
            is_public: true,
            doc_summary: None,
            children: vec![],
        };
        assert_eq!(star.label(), "fn my_function");
    }

    #[test]
    fn test_star_label_class() {
        let star = Star {
            name: "MyClass".to_string(),
            kind: StarKind::Class,
            file_path: "test.py".to_string(),
            line: 1,
            end_line: 10,
            is_public: true,
            doc_summary: Some("A test class".to_string()),
            children: vec![],
        };
        assert_eq!(star.label(), "class MyClass");
    }

    // ==================== Declaration Conversion Tests ====================

    #[test]
    fn test_declaration_kind_to_string_all_variants() {
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Function),
            "function"
        );
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Method),
            "method"
        );
        assert_eq!(declaration_kind_to_string(DeclarationKind::Class), "class");
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Struct),
            "struct"
        );
        assert_eq!(declaration_kind_to_string(DeclarationKind::Enum), "enum");
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Interface),
            "interface"
        );
        assert_eq!(declaration_kind_to_string(DeclarationKind::Trait), "trait");
        assert_eq!(declaration_kind_to_string(DeclarationKind::Type), "type");
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Constant),
            "constant"
        );
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Variable),
            "variable"
        );
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Module),
            "module"
        );
        assert_eq!(
            declaration_kind_to_string(DeclarationKind::Namespace),
            "namespace"
        );
        assert_eq!(declaration_kind_to_string(DeclarationKind::Impl), "impl");
        assert_eq!(declaration_kind_to_string(DeclarationKind::Macro), "macro");
        assert_eq!(declaration_kind_to_string(DeclarationKind::Other), "other");
    }

    #[test]
    fn test_visibility_to_string_all_variants() {
        assert_eq!(visibility_to_string(Visibility::Public), "public");
        assert_eq!(visibility_to_string(Visibility::Private), "private");
        assert_eq!(visibility_to_string(Visibility::Protected), "protected");
        assert_eq!(visibility_to_string(Visibility::Internal), "internal");
        assert_eq!(visibility_to_string(Visibility::Unknown), "unknown");
    }

    // ==================== Star from Declaration Tests ====================

    #[test]
    fn test_star_from_rust_enum() {
        let bridge = AstBridge::new();
        let source = r#"
/// Status enumeration
pub enum Status {
    Active,
    Inactive,
}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        let status = stars.iter().find(|s| s.name == "Status");
        assert!(status.is_some(), "Should find 'Status' enum");
        let status = status.unwrap();
        assert_eq!(status.kind, StarKind::Enum);
        assert!(status.is_public);
    }

    #[test]
    fn test_star_from_rust_trait() {
        let bridge = AstBridge::new();
        let source = r#"
pub trait Drawable {
    fn draw(&self);
}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        let drawable = stars.iter().find(|s| s.name == "Drawable");
        assert!(drawable.is_some(), "Should find 'Drawable' trait");
        let drawable = drawable.unwrap();
        assert_eq!(drawable.kind, StarKind::Trait);
    }

    #[test]
    fn test_star_from_rust_impl() {
        let bridge = AstBridge::new();
        let source = r#"
struct Point { x: i32, y: i32 }

impl Point {
    fn new() -> Self {
        Point { x: 0, y: 0 }
    }
}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        // Should have struct and impl
        assert!(stars.len() >= 2, "Should have at least struct and impl");
    }

    #[test]
    fn test_star_from_rust_const() {
        let bridge = AstBridge::new();
        let source = r#"
pub const MAX_SIZE: usize = 100;
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        let max_size = stars.iter().find(|s| s.name == "MAX_SIZE");
        assert!(max_size.is_some(), "Should find 'MAX_SIZE' constant");
        let max_size = max_size.unwrap();
        assert_eq!(max_size.kind, StarKind::Constant);
    }

    #[test]
    fn test_star_from_rust_type_alias() {
        let bridge = AstBridge::new();
        let source = r#"
pub type Result<T> = std::result::Result<T, Error>;
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        let result = stars.iter().find(|s| s.name == "Result");
        assert!(result.is_some(), "Should find 'Result' type alias");
        let result = result.unwrap();
        assert_eq!(result.kind, StarKind::Type);
    }

    #[test]
    fn test_star_from_rust_macro() {
        let bridge = AstBridge::new();
        let source = r#"
macro_rules! say_hello {
    () => {
        println!("Hello!");
    };
}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        let macro_star = stars.iter().find(|s| s.name == "say_hello");
        assert!(macro_star.is_some(), "Should find 'say_hello' macro");
        let macro_star = macro_star.unwrap();
        assert_eq!(macro_star.kind, StarKind::Macro);
    }

    #[test]
    fn test_star_from_rust_module() {
        let bridge = AstBridge::new();
        let source = r#"
mod inner {
    pub fn foo() {}
}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let stars = bridge.extract_stars(&file);

        let module = stars.iter().find(|s| s.name == "inner");
        assert!(module.is_some(), "Should find 'inner' module");
        let module = module.unwrap();
        assert_eq!(module.kind, StarKind::Module);
    }

    #[test]
    fn test_star_from_typescript_interface() {
        let bridge = AstBridge::new();
        let source = r#"
interface User {
    name: string;
    age: number;
}
"#;
        let file = bridge.analyze_file(source, LanguageId::TypeScript).unwrap();
        let stars = bridge.extract_stars(&file);

        let user = stars.iter().find(|s| s.name == "User");
        assert!(user.is_some(), "Should find 'User' interface");
        let user = user.unwrap();
        assert_eq!(user.kind, StarKind::Interface);
    }

    #[test]
    fn test_star_from_python_class_with_method() {
        let bridge = AstBridge::new();
        let source = r#"
class Calculator:
    def add(self, a, b):
        return a + b
"#;
        let file = bridge.analyze_file(source, LanguageId::Python).unwrap();
        let stars = bridge.extract_stars(&file);

        let calc = stars.iter().find(|s| s.name == "Calculator");
        assert!(calc.is_some(), "Should find 'Calculator' class");
        let calc = calc.unwrap();
        assert_eq!(calc.kind, StarKind::Class);

        // Should also have the method
        let add_method = stars.iter().find(|s| s.name == "add");
        assert!(add_method.is_some(), "Should find 'add' method");
    }

    // ==================== Language Detection Tests ====================

    #[test]
    fn test_language_detection_javascript() {
        assert_eq!(
            AstBridge::detect_language(Path::new("app.js")),
            LanguageId::JavaScript
        );
        // JSX is a separate language ID
        assert_eq!(
            AstBridge::detect_language(Path::new("app.jsx")),
            LanguageId::Jsx
        );
    }

    #[test]
    fn test_language_detection_typescript() {
        assert_eq!(
            AstBridge::detect_language(Path::new("app.ts")),
            LanguageId::TypeScript
        );
        // TSX is a separate language ID
        assert_eq!(
            AstBridge::detect_language(Path::new("app.tsx")),
            LanguageId::Tsx
        );
    }

    #[test]
    fn test_language_detection_unknown() {
        assert_eq!(
            AstBridge::detect_language(Path::new("readme")),
            LanguageId::Unknown
        );
        assert_eq!(
            AstBridge::detect_language(Path::new("file.xyz")),
            LanguageId::Unknown
        );
    }

    // ==================== File Summary Tests ====================

    #[test]
    fn test_file_summary_with_imports() {
        let bridge = AstBridge::new();
        let source = r#"
use std::collections::HashMap;
use std::io::Result;

pub fn process() {}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let summary = bridge.get_file_summary(&file);

        assert!(summary.import_count >= 2, "Should have imports");
        assert!(!summary.has_errors);
    }

    #[test]
    fn test_file_summary_language_name() {
        let bridge = AstBridge::new();
        let source = "fn main() {}";
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let summary = bridge.get_file_summary(&file);

        // Language name is capitalized
        assert_eq!(summary.language, "Rust");
    }

    #[test]
    fn test_star_summary_visibility() {
        let bridge = AstBridge::new();
        let source = r#"
pub fn public_fn() {}
fn private_fn() {}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let summary = bridge.get_file_summary(&file);

        let public_star = summary.stars.iter().find(|s| s.name == "public_fn");
        assert!(public_star.is_some());
        assert_eq!(public_star.unwrap().visibility, "public");

        let private_star = summary.stars.iter().find(|s| s.name == "private_fn");
        assert!(private_star.is_some());
        assert_eq!(private_star.unwrap().visibility, "private");
    }

    #[test]
    fn test_star_summary_with_doc() {
        let bridge = AstBridge::new();
        let source = r#"
/// Documented function
pub fn documented() {}

pub fn undocumented() {}
"#;
        let file = bridge.analyze_file(source, LanguageId::Rust).unwrap();
        let summary = bridge.get_file_summary(&file);

        let doc_star = summary.stars.iter().find(|s| s.name == "documented");
        assert!(doc_star.is_some());
        assert!(doc_star.unwrap().has_doc);

        let undoc_star = summary.stars.iter().find(|s| s.name == "undocumented");
        assert!(undoc_star.is_some());
        assert!(!undoc_star.unwrap().has_doc);
    }
}
