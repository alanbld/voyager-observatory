//! Core IR (Intermediate Representation) Types
//!
//! This module defines the language-agnostic structural model used by
//! voyager-ast. All types are designed for:
//!
//! 1. **Determinism**: Using BTreeMap/BTreeSet for ordered iteration
//! 2. **Serialization**: Full serde support for caching and export
//! 3. **Error Tolerance**: UnknownNode/UnparsedBlock for graceful degradation

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================================
// Language Identification
// ============================================================================

/// Language identifier for source files
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LanguageId {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Tsx,
    Jsx,
    Abl,
    C,
    Cpp,
    Java,
    Go,
    Ruby,
    Php,
    CSharp,
    Swift,
    Kotlin,
    Scala,
    Html,
    Css,
    Json,
    Yaml,
    Toml,
    Markdown,
    Bash,
    Sql,
    Unknown,
}

impl LanguageId {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" | "pyw" | "pyi" => Self::Python,
            "ts" | "mts" | "cts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "jsx" => Self::Jsx,
            "p" | "i" | "w" | "cls" => Self::Abl,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Self::Cpp,
            "java" => Self::Java,
            "go" => Self::Go,
            "rb" | "rake" | "gemspec" => Self::Ruby,
            "php" | "phtml" => Self::Php,
            "cs" => Self::CSharp,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "sass" => Self::Css,
            "json" | "jsonc" => Self::Json,
            "yaml" | "yml" => Self::Yaml,
            "toml" => Self::Toml,
            "md" | "markdown" => Self::Markdown,
            "sh" | "bash" | "zsh" | "ksh" => Self::Bash,
            "sql" => Self::Sql,
            _ => Self::Unknown,
        }
    }

    /// Get canonical file extension
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Rust => "rs",
            Self::Python => "py",
            Self::TypeScript => "ts",
            Self::Tsx => "tsx",
            Self::JavaScript => "js",
            Self::Jsx => "jsx",
            Self::Abl => "p",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Go => "go",
            Self::Ruby => "rb",
            Self::Php => "php",
            Self::CSharp => "cs",
            Self::Swift => "swift",
            Self::Kotlin => "kt",
            Self::Scala => "scala",
            Self::Html => "html",
            Self::Css => "css",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Markdown => "md",
            Self::Bash => "sh",
            Self::Sql => "sql",
            Self::Unknown => "",
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
            Self::JavaScript => "JavaScript",
            Self::Jsx => "JSX",
            Self::Abl => "ABL",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Java => "Java",
            Self::Go => "Go",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::CSharp => "C#",
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::Scala => "Scala",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
            Self::Markdown => "Markdown",
            Self::Bash => "Bash",
            Self::Sql => "SQL",
            Self::Unknown => "Unknown",
        }
    }
}

// ============================================================================
// Span and Region
// ============================================================================

/// A contiguous region in source code
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Span {
    /// Start byte offset (inclusive)
    pub start: usize,

    /// End byte offset (exclusive)
    pub end: usize,

    /// Start line (1-indexed)
    pub start_line: usize,

    /// End line (1-indexed)
    pub end_line: usize,

    /// Start column (0-indexed, in bytes)
    pub start_column: usize,

    /// End column (0-indexed, in bytes)
    pub end_column: usize,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize, start_line: usize, end_line: usize) -> Self {
        Self {
            start,
            end,
            start_line,
            end_line,
            start_column: 0,
            end_column: 0,
        }
    }

    /// Check if this span contains a byte offset
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Check if this span contains a line number
    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// A source region with optional language override (for embedded languages)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Region {
    pub span: Span,
    pub language: Option<LanguageId>,
}

// ============================================================================
// File
// ============================================================================

/// A parsed source file with its structural elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// Canonical path to the file
    pub path: String,

    /// Detected language identifier
    pub language: LanguageId,

    /// Top-level declarations (functions, classes, structs, etc.)
    pub declarations: Vec<Declaration>,

    /// Import statements
    pub imports: Vec<ImportLike>,

    /// File-level and attached comments
    pub comments: Vec<Comment>,

    /// Regions that couldn't be parsed
    pub unknown_regions: Vec<UnknownNode>,

    /// Byte range of the entire file
    pub span: Span,

    /// Additional metadata (BTreeMap for determinism)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl File {
    /// Create a new empty file
    pub fn new(path: String, language: LanguageId) -> Self {
        Self {
            path,
            language,
            declarations: Vec::new(),
            imports: Vec::new(),
            comments: Vec::new(),
            unknown_regions: Vec::new(),
            span: Span::default(),
            metadata: BTreeMap::new(),
        }
    }

    /// Check if the file has any parse errors
    pub fn has_errors(&self) -> bool {
        !self.unknown_regions.is_empty()
    }

    /// Get the total number of declarations (including nested)
    pub fn total_declarations(&self) -> usize {
        fn count_nested(decls: &[Declaration]) -> usize {
            decls.iter().map(|d| 1 + count_nested(&d.children)).sum()
        }
        count_nested(&self.declarations)
    }
}

// ============================================================================
// Declaration
// ============================================================================

/// A named declaration (function, class, struct, type, constant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Declaration {
    /// The declaration's name
    pub name: String,

    /// What kind of declaration this is
    pub kind: DeclarationKind,

    /// Visibility (public, private, etc.)
    pub visibility: Visibility,

    /// The full span of the declaration
    pub span: Span,

    /// Span of just the signature/header (for display)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_span: Option<Span>,

    /// Span of the body (for Zoom mode extraction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_span: Option<Span>,

    /// Attached documentation comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_comment: Option<Comment>,

    /// Nested declarations (methods in class, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Declaration>,

    /// Parameters (for functions/methods)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,

    /// Return type annotation (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl Declaration {
    /// Create a new declaration
    pub fn new(name: String, kind: DeclarationKind, span: Span) -> Self {
        Self {
            name,
            kind,
            visibility: Visibility::Unknown,
            span,
            signature_span: None,
            body_span: None,
            doc_comment: None,
            children: Vec::new(),
            parameters: Vec::new(),
            return_type: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Create a unique identifier for this declaration
    pub fn id(&self) -> String {
        format!("{}:{}:{}", self.kind.as_str(), self.name, self.span.start_line)
    }
}

/// Kind of declaration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeclarationKind {
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
    Impl,
    Macro,
    Other,
}

impl DeclarationKind {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Type => "type",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Impl => "impl",
            Self::Macro => "macro",
            Self::Other => "other",
        }
    }
}

/// Visibility of a declaration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    #[default]
    Unknown,
}

/// A function/method parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_annotation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    pub span: Span,
}

// ============================================================================
// Block and Control Flow
// ============================================================================

/// A code block (function body, if body, loop body, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    /// The block's span
    pub span: Span,

    /// Nested control flow structures
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_flow: Vec<ControlFlow>,

    /// Function/method calls within this block
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<Call>,

    /// Comments within this block
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,

    /// Unknown/unparsed regions within the block
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_regions: Vec<UnknownNode>,

    /// Nested declarations (lambdas, inner functions, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested_declarations: Vec<Declaration>,
}

/// Control flow constructs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlow {
    pub kind: ControlFlowKind,
    pub span: Span,
    /// The condition expression span (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_span: Option<Span>,
    /// Child blocks (then/else branches, loop body, match arms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<Block>,
}

/// Kind of control flow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ControlFlowKind {
    If,
    Else,
    ElseIf,
    Match,
    Switch,
    For,
    While,
    Loop,
    Try,
    Catch,
    Finally,
    With,
    Return,
    Break,
    Continue,
    Other,
}

/// A function or method call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    /// The callee expression (function name, method chain, etc.)
    pub callee: String,

    /// Span of the entire call expression
    pub span: Span,

    /// Number of arguments
    pub argument_count: usize,

    /// Whether this is a method call
    #[serde(default)]
    pub is_method: bool,
}

// ============================================================================
// Import
// ============================================================================

/// Import, require, include, using, or module reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportLike {
    /// What is being imported (module path, file, etc.)
    pub source: String,

    /// Kind of import
    pub kind: ImportKind,

    /// Specific items imported (for selective imports)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,

    /// Alias if renamed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Whether this is a type-only import
    #[serde(default)]
    pub type_only: bool,

    pub span: Span,
}

/// Kind of import statement
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportKind {
    /// import x from 'y' or import 'y'
    Import,
    /// require('x')
    Require,
    /// #include <x> or #include "x"
    Include,
    /// using namespace x
    Using,
    /// mod x; or mod x { }
    Module,
    /// from x import y
    From,
    /// use x::y
    Use,
    Other,
}

// ============================================================================
// Comment
// ============================================================================

/// A comment in source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// The comment text (without delimiters)
    pub text: String,

    /// Kind of comment
    pub kind: CommentKind,

    pub span: Span,

    /// The declaration this comment is attached to (if any)
    /// Uses "Nearest Preceding Node" heuristic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_to: Option<String>,
}

/// Kind of comment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommentKind {
    /// Single-line comment (// or #)
    Line,
    /// Multi-line block comment (/* */)
    Block,
    /// Documentation comment (/// or /** */)
    Doc,
}

// ============================================================================
// Error Recovery Types
// ============================================================================

/// A region that couldn't be parsed or is syntactically invalid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownNode {
    pub span: Span,
    /// Optional description of why this region is unknown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The raw text of the region (for debugging, may be truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_text: Option<String>,
}

/// An unparsed block (larger region with syntax errors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnparsedBlock {
    pub span: Span,
    pub reason: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(LanguageId::from_extension("rs"), LanguageId::Rust);
        assert_eq!(LanguageId::from_extension("py"), LanguageId::Python);
        assert_eq!(LanguageId::from_extension("ts"), LanguageId::TypeScript);
        assert_eq!(LanguageId::from_extension("tsx"), LanguageId::Tsx);
        assert_eq!(LanguageId::from_extension("p"), LanguageId::Abl);
        assert_eq!(LanguageId::from_extension("xyz"), LanguageId::Unknown);
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(10, 20, 1, 2);
        assert!(span.contains(10));
        assert!(span.contains(15));
        assert!(!span.contains(20)); // exclusive end
        assert!(!span.contains(5));
    }

    #[test]
    fn test_declaration_id() {
        let decl = Declaration::new(
            "my_function".to_string(),
            DeclarationKind::Function,
            Span::new(0, 100, 5, 10),
        );
        assert_eq!(decl.id(), "function:my_function:5");
    }

    #[test]
    fn test_file_serialization_deterministic() {
        let file = File::new("test.rs".to_string(), LanguageId::Rust);
        let json1 = serde_json::to_string(&file).unwrap();
        let json2 = serde_json::to_string(&file).unwrap();
        assert_eq!(json1, json2, "Serialization must be deterministic");
    }

    // =========================================================================
    // LanguageId Tests
    // =========================================================================

    #[test]
    fn test_language_from_extension_all_variants() {
        // Rust
        assert_eq!(LanguageId::from_extension("rs"), LanguageId::Rust);

        // Python
        assert_eq!(LanguageId::from_extension("py"), LanguageId::Python);
        assert_eq!(LanguageId::from_extension("pyw"), LanguageId::Python);
        assert_eq!(LanguageId::from_extension("pyi"), LanguageId::Python);

        // TypeScript variants
        assert_eq!(LanguageId::from_extension("ts"), LanguageId::TypeScript);
        assert_eq!(LanguageId::from_extension("mts"), LanguageId::TypeScript);
        assert_eq!(LanguageId::from_extension("cts"), LanguageId::TypeScript);
        assert_eq!(LanguageId::from_extension("tsx"), LanguageId::Tsx);

        // JavaScript variants
        assert_eq!(LanguageId::from_extension("js"), LanguageId::JavaScript);
        assert_eq!(LanguageId::from_extension("mjs"), LanguageId::JavaScript);
        assert_eq!(LanguageId::from_extension("cjs"), LanguageId::JavaScript);
        assert_eq!(LanguageId::from_extension("jsx"), LanguageId::Jsx);

        // ABL
        assert_eq!(LanguageId::from_extension("p"), LanguageId::Abl);
        assert_eq!(LanguageId::from_extension("i"), LanguageId::Abl);
        assert_eq!(LanguageId::from_extension("w"), LanguageId::Abl);
        assert_eq!(LanguageId::from_extension("cls"), LanguageId::Abl);

        // C/C++
        assert_eq!(LanguageId::from_extension("c"), LanguageId::C);
        assert_eq!(LanguageId::from_extension("h"), LanguageId::C);
        assert_eq!(LanguageId::from_extension("cpp"), LanguageId::Cpp);
        assert_eq!(LanguageId::from_extension("cc"), LanguageId::Cpp);
        assert_eq!(LanguageId::from_extension("cxx"), LanguageId::Cpp);
        assert_eq!(LanguageId::from_extension("hpp"), LanguageId::Cpp);
        assert_eq!(LanguageId::from_extension("hxx"), LanguageId::Cpp);
        assert_eq!(LanguageId::from_extension("hh"), LanguageId::Cpp);

        // Other languages
        assert_eq!(LanguageId::from_extension("java"), LanguageId::Java);
        assert_eq!(LanguageId::from_extension("go"), LanguageId::Go);
        assert_eq!(LanguageId::from_extension("rb"), LanguageId::Ruby);
        assert_eq!(LanguageId::from_extension("rake"), LanguageId::Ruby);
        assert_eq!(LanguageId::from_extension("gemspec"), LanguageId::Ruby);
        assert_eq!(LanguageId::from_extension("php"), LanguageId::Php);
        assert_eq!(LanguageId::from_extension("phtml"), LanguageId::Php);
        assert_eq!(LanguageId::from_extension("cs"), LanguageId::CSharp);
        assert_eq!(LanguageId::from_extension("swift"), LanguageId::Swift);
        assert_eq!(LanguageId::from_extension("kt"), LanguageId::Kotlin);
        assert_eq!(LanguageId::from_extension("kts"), LanguageId::Kotlin);
        assert_eq!(LanguageId::from_extension("scala"), LanguageId::Scala);
        assert_eq!(LanguageId::from_extension("sc"), LanguageId::Scala);

        // Markup/Config
        assert_eq!(LanguageId::from_extension("html"), LanguageId::Html);
        assert_eq!(LanguageId::from_extension("htm"), LanguageId::Html);
        assert_eq!(LanguageId::from_extension("css"), LanguageId::Css);
        assert_eq!(LanguageId::from_extension("scss"), LanguageId::Css);
        assert_eq!(LanguageId::from_extension("sass"), LanguageId::Css);
        assert_eq!(LanguageId::from_extension("json"), LanguageId::Json);
        assert_eq!(LanguageId::from_extension("jsonc"), LanguageId::Json);
        assert_eq!(LanguageId::from_extension("yaml"), LanguageId::Yaml);
        assert_eq!(LanguageId::from_extension("yml"), LanguageId::Yaml);
        assert_eq!(LanguageId::from_extension("toml"), LanguageId::Toml);
        assert_eq!(LanguageId::from_extension("md"), LanguageId::Markdown);
        assert_eq!(LanguageId::from_extension("markdown"), LanguageId::Markdown);

        // Shell/SQL
        assert_eq!(LanguageId::from_extension("sh"), LanguageId::Bash);
        assert_eq!(LanguageId::from_extension("bash"), LanguageId::Bash);
        assert_eq!(LanguageId::from_extension("zsh"), LanguageId::Bash);
        assert_eq!(LanguageId::from_extension("ksh"), LanguageId::Bash);
        assert_eq!(LanguageId::from_extension("sql"), LanguageId::Sql);

        // Unknown
        assert_eq!(LanguageId::from_extension("xyz"), LanguageId::Unknown);
        assert_eq!(LanguageId::from_extension(""), LanguageId::Unknown);
    }

    #[test]
    fn test_language_from_extension_case_insensitive() {
        assert_eq!(LanguageId::from_extension("RS"), LanguageId::Rust);
        assert_eq!(LanguageId::from_extension("Py"), LanguageId::Python);
        assert_eq!(LanguageId::from_extension("TS"), LanguageId::TypeScript);
    }

    #[test]
    fn test_language_extension_all_variants() {
        assert_eq!(LanguageId::Rust.extension(), "rs");
        assert_eq!(LanguageId::Python.extension(), "py");
        assert_eq!(LanguageId::TypeScript.extension(), "ts");
        assert_eq!(LanguageId::Tsx.extension(), "tsx");
        assert_eq!(LanguageId::JavaScript.extension(), "js");
        assert_eq!(LanguageId::Jsx.extension(), "jsx");
        assert_eq!(LanguageId::Abl.extension(), "p");
        assert_eq!(LanguageId::C.extension(), "c");
        assert_eq!(LanguageId::Cpp.extension(), "cpp");
        assert_eq!(LanguageId::Java.extension(), "java");
        assert_eq!(LanguageId::Go.extension(), "go");
        assert_eq!(LanguageId::Ruby.extension(), "rb");
        assert_eq!(LanguageId::Php.extension(), "php");
        assert_eq!(LanguageId::CSharp.extension(), "cs");
        assert_eq!(LanguageId::Swift.extension(), "swift");
        assert_eq!(LanguageId::Kotlin.extension(), "kt");
        assert_eq!(LanguageId::Scala.extension(), "scala");
        assert_eq!(LanguageId::Html.extension(), "html");
        assert_eq!(LanguageId::Css.extension(), "css");
        assert_eq!(LanguageId::Json.extension(), "json");
        assert_eq!(LanguageId::Yaml.extension(), "yaml");
        assert_eq!(LanguageId::Toml.extension(), "toml");
        assert_eq!(LanguageId::Markdown.extension(), "md");
        assert_eq!(LanguageId::Bash.extension(), "sh");
        assert_eq!(LanguageId::Sql.extension(), "sql");
        assert_eq!(LanguageId::Unknown.extension(), "");
    }

    #[test]
    fn test_language_name_all_variants() {
        assert_eq!(LanguageId::Rust.name(), "Rust");
        assert_eq!(LanguageId::Python.name(), "Python");
        assert_eq!(LanguageId::TypeScript.name(), "TypeScript");
        assert_eq!(LanguageId::Tsx.name(), "TSX");
        assert_eq!(LanguageId::JavaScript.name(), "JavaScript");
        assert_eq!(LanguageId::Jsx.name(), "JSX");
        assert_eq!(LanguageId::Abl.name(), "ABL");
        assert_eq!(LanguageId::C.name(), "C");
        assert_eq!(LanguageId::Cpp.name(), "C++");
        assert_eq!(LanguageId::Java.name(), "Java");
        assert_eq!(LanguageId::Go.name(), "Go");
        assert_eq!(LanguageId::Ruby.name(), "Ruby");
        assert_eq!(LanguageId::Php.name(), "PHP");
        assert_eq!(LanguageId::CSharp.name(), "C#");
        assert_eq!(LanguageId::Swift.name(), "Swift");
        assert_eq!(LanguageId::Kotlin.name(), "Kotlin");
        assert_eq!(LanguageId::Scala.name(), "Scala");
        assert_eq!(LanguageId::Html.name(), "HTML");
        assert_eq!(LanguageId::Css.name(), "CSS");
        assert_eq!(LanguageId::Json.name(), "JSON");
        assert_eq!(LanguageId::Yaml.name(), "YAML");
        assert_eq!(LanguageId::Toml.name(), "TOML");
        assert_eq!(LanguageId::Markdown.name(), "Markdown");
        assert_eq!(LanguageId::Bash.name(), "Bash");
        assert_eq!(LanguageId::Sql.name(), "SQL");
        assert_eq!(LanguageId::Unknown.name(), "Unknown");
    }

    #[test]
    fn test_language_id_traits() {
        // Clone
        let lang = LanguageId::Rust;
        let cloned = lang.clone();
        assert_eq!(lang, cloned);

        // Copy
        let copied = lang;
        assert_eq!(lang, copied);

        // Hash (via BTreeMap key)
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(LanguageId::Rust, "rust");
        assert_eq!(map.get(&LanguageId::Rust), Some(&"rust"));

        // Ord
        assert!(LanguageId::Rust < LanguageId::Unknown);

        // Debug
        let debug_str = format!("{:?}", LanguageId::Python);
        assert!(debug_str.contains("Python"));
    }

    #[test]
    fn test_language_id_serialization() {
        let lang = LanguageId::TypeScript;
        let json = serde_json::to_string(&lang).unwrap();
        assert_eq!(json, "\"typescript\"");

        let deserialized: LanguageId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LanguageId::TypeScript);
    }

    // =========================================================================
    // Span Tests
    // =========================================================================

    #[test]
    fn test_span_default() {
        let span = Span::default();
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 0);
        assert_eq!(span.start_line, 0);
        assert_eq!(span.end_line, 0);
        assert_eq!(span.start_column, 0);
        assert_eq!(span.end_column, 0);
    }

    #[test]
    fn test_span_new() {
        let span = Span::new(10, 50, 5, 10);
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 50);
        assert_eq!(span.start_line, 5);
        assert_eq!(span.end_line, 10);
        assert_eq!(span.start_column, 0);  // Default
        assert_eq!(span.end_column, 0);    // Default
    }

    #[test]
    fn test_span_contains_line() {
        let span = Span::new(0, 100, 5, 15);
        assert!(!span.contains_line(4));  // Before
        assert!(span.contains_line(5));   // Start
        assert!(span.contains_line(10));  // Middle
        assert!(span.contains_line(15));  // End (inclusive)
        assert!(!span.contains_line(16)); // After
    }

    #[test]
    fn test_span_len() {
        let span = Span::new(10, 50, 1, 5);
        assert_eq!(span.len(), 40);

        let empty_span = Span::new(10, 10, 1, 1);
        assert_eq!(empty_span.len(), 0);

        // Saturating sub for inverted spans
        let inverted = Span { start: 50, end: 10, ..Default::default() };
        assert_eq!(inverted.len(), 0);
    }

    #[test]
    fn test_span_is_empty() {
        let empty = Span::new(10, 10, 1, 1);
        assert!(empty.is_empty());

        let non_empty = Span::new(10, 20, 1, 2);
        assert!(!non_empty.is_empty());

        // Inverted span is also empty
        let inverted = Span { start: 20, end: 10, ..Default::default() };
        assert!(inverted.is_empty());
    }

    #[test]
    fn test_span_equality() {
        let span1 = Span::new(0, 10, 1, 2);
        let span2 = Span::new(0, 10, 1, 2);
        assert_eq!(span1, span2);
    }

    #[test]
    fn test_span_serialization() {
        let span = Span::new(100, 200, 10, 20);
        let json = serde_json::to_string(&span).unwrap();
        let deserialized: Span = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.start, 100);
        assert_eq!(deserialized.end, 200);
    }

    // =========================================================================
    // Region Tests
    // =========================================================================

    #[test]
    fn test_region_fields() {
        let region = Region {
            span: Span::new(0, 100, 1, 10),
            language: Some(LanguageId::Python),
        };
        assert_eq!(region.span.start, 0);
        assert_eq!(region.language, Some(LanguageId::Python));
    }

    #[test]
    fn test_region_no_language() {
        let region = Region {
            span: Span::default(),
            language: None,
        };
        assert!(region.language.is_none());
    }

    // =========================================================================
    // File Tests
    // =========================================================================

    #[test]
    fn test_file_new() {
        let file = File::new("src/main.rs".to_string(), LanguageId::Rust);
        assert_eq!(file.path, "src/main.rs");
        assert_eq!(file.language, LanguageId::Rust);
        assert!(file.declarations.is_empty());
        assert!(file.imports.is_empty());
        assert!(file.comments.is_empty());
        assert!(file.unknown_regions.is_empty());
        assert!(file.metadata.is_empty());
    }

    #[test]
    fn test_file_has_errors() {
        let mut file = File::new("test.rs".to_string(), LanguageId::Rust);
        assert!(!file.has_errors());

        file.unknown_regions.push(UnknownNode {
            span: Span::default(),
            reason: Some("syntax error".to_string()),
            raw_text: None,
        });
        assert!(file.has_errors());
    }

    #[test]
    fn test_file_total_declarations_flat() {
        let mut file = File::new("test.rs".to_string(), LanguageId::Rust);
        file.declarations.push(Declaration::new(
            "fn1".to_string(),
            DeclarationKind::Function,
            Span::default(),
        ));
        file.declarations.push(Declaration::new(
            "fn2".to_string(),
            DeclarationKind::Function,
            Span::default(),
        ));
        assert_eq!(file.total_declarations(), 2);
    }

    #[test]
    fn test_file_total_declarations_nested() {
        let mut file = File::new("test.rs".to_string(), LanguageId::Rust);

        let mut class = Declaration::new(
            "MyClass".to_string(),
            DeclarationKind::Class,
            Span::default(),
        );
        class.children.push(Declaration::new(
            "method1".to_string(),
            DeclarationKind::Method,
            Span::default(),
        ));
        class.children.push(Declaration::new(
            "method2".to_string(),
            DeclarationKind::Method,
            Span::default(),
        ));

        file.declarations.push(class);
        file.declarations.push(Declaration::new(
            "standalone".to_string(),
            DeclarationKind::Function,
            Span::default(),
        ));

        // 1 class + 2 methods + 1 function = 4
        assert_eq!(file.total_declarations(), 4);
    }

    // =========================================================================
    // Declaration Tests
    // =========================================================================

    #[test]
    fn test_declaration_new() {
        let decl = Declaration::new(
            "test_fn".to_string(),
            DeclarationKind::Function,
            Span::new(0, 100, 1, 10),
        );
        assert_eq!(decl.name, "test_fn");
        assert_eq!(decl.kind, DeclarationKind::Function);
        assert_eq!(decl.visibility, Visibility::Unknown);
        assert!(decl.signature_span.is_none());
        assert!(decl.body_span.is_none());
        assert!(decl.doc_comment.is_none());
        assert!(decl.children.is_empty());
        assert!(decl.parameters.is_empty());
        assert!(decl.return_type.is_none());
        assert!(decl.metadata.is_empty());
    }

    #[test]
    fn test_declaration_id_various_kinds() {
        let fn_decl = Declaration::new(
            "foo".to_string(),
            DeclarationKind::Function,
            Span::new(0, 10, 5, 10),
        );
        assert_eq!(fn_decl.id(), "function:foo:5");

        let class_decl = Declaration::new(
            "MyClass".to_string(),
            DeclarationKind::Class,
            Span::new(0, 100, 1, 20),
        );
        assert_eq!(class_decl.id(), "class:MyClass:1");

        let method_decl = Declaration::new(
            "process".to_string(),
            DeclarationKind::Method,
            Span::new(50, 100, 10, 15),
        );
        assert_eq!(method_decl.id(), "method:process:10");
    }

    // =========================================================================
    // DeclarationKind Tests
    // =========================================================================

    #[test]
    fn test_declaration_kind_as_str_all() {
        assert_eq!(DeclarationKind::Function.as_str(), "function");
        assert_eq!(DeclarationKind::Method.as_str(), "method");
        assert_eq!(DeclarationKind::Class.as_str(), "class");
        assert_eq!(DeclarationKind::Struct.as_str(), "struct");
        assert_eq!(DeclarationKind::Enum.as_str(), "enum");
        assert_eq!(DeclarationKind::Interface.as_str(), "interface");
        assert_eq!(DeclarationKind::Trait.as_str(), "trait");
        assert_eq!(DeclarationKind::Type.as_str(), "type");
        assert_eq!(DeclarationKind::Constant.as_str(), "constant");
        assert_eq!(DeclarationKind::Variable.as_str(), "variable");
        assert_eq!(DeclarationKind::Module.as_str(), "module");
        assert_eq!(DeclarationKind::Namespace.as_str(), "namespace");
        assert_eq!(DeclarationKind::Impl.as_str(), "impl");
        assert_eq!(DeclarationKind::Macro.as_str(), "macro");
        assert_eq!(DeclarationKind::Other.as_str(), "other");
    }

    #[test]
    fn test_declaration_kind_equality() {
        assert_eq!(DeclarationKind::Function, DeclarationKind::Function);
        assert_ne!(DeclarationKind::Function, DeclarationKind::Method);
    }

    // =========================================================================
    // Visibility Tests
    // =========================================================================

    #[test]
    fn test_visibility_default() {
        let vis = Visibility::default();
        assert_eq!(vis, Visibility::Unknown);
    }

    #[test]
    fn test_visibility_all_variants() {
        assert_eq!(Visibility::Public, Visibility::Public);
        assert_eq!(Visibility::Private, Visibility::Private);
        assert_eq!(Visibility::Protected, Visibility::Protected);
        assert_eq!(Visibility::Internal, Visibility::Internal);
        assert_eq!(Visibility::Unknown, Visibility::Unknown);
    }

    // =========================================================================
    // Parameter Tests
    // =========================================================================

    #[test]
    fn test_parameter_fields() {
        let param = Parameter {
            name: "x".to_string(),
            type_annotation: Some("i32".to_string()),
            default_value: Some("0".to_string()),
            span: Span::new(10, 20, 1, 1),
        };

        assert_eq!(param.name, "x");
        assert_eq!(param.type_annotation, Some("i32".to_string()));
        assert_eq!(param.default_value, Some("0".to_string()));
    }

    #[test]
    fn test_parameter_minimal() {
        let param = Parameter {
            name: "arg".to_string(),
            type_annotation: None,
            default_value: None,
            span: Span::default(),
        };

        assert_eq!(param.name, "arg");
        assert!(param.type_annotation.is_none());
        assert!(param.default_value.is_none());
    }

    // =========================================================================
    // Block Tests
    // =========================================================================

    #[test]
    fn test_block_default() {
        let block = Block::default();
        assert!(block.control_flow.is_empty());
        assert!(block.calls.is_empty());
        assert!(block.comments.is_empty());
        assert!(block.unknown_regions.is_empty());
        assert!(block.nested_declarations.is_empty());
    }

    #[test]
    fn test_block_with_content() {
        let block = Block {
            span: Span::new(0, 100, 1, 10),
            control_flow: vec![ControlFlow {
                kind: ControlFlowKind::If,
                span: Span::new(10, 50, 2, 5),
                condition_span: Some(Span::new(13, 20, 2, 2)),
                branches: vec![],
            }],
            calls: vec![Call {
                callee: "print".to_string(),
                span: Span::new(60, 80, 7, 7),
                argument_count: 1,
                is_method: false,
            }],
            comments: vec![],
            unknown_regions: vec![],
            nested_declarations: vec![],
        };

        assert_eq!(block.control_flow.len(), 1);
        assert_eq!(block.calls.len(), 1);
    }

    // =========================================================================
    // ControlFlow Tests
    // =========================================================================

    #[test]
    fn test_control_flow_kind_all() {
        let kinds = [
            ControlFlowKind::If,
            ControlFlowKind::Else,
            ControlFlowKind::ElseIf,
            ControlFlowKind::Match,
            ControlFlowKind::Switch,
            ControlFlowKind::For,
            ControlFlowKind::While,
            ControlFlowKind::Loop,
            ControlFlowKind::Try,
            ControlFlowKind::Catch,
            ControlFlowKind::Finally,
            ControlFlowKind::With,
            ControlFlowKind::Return,
            ControlFlowKind::Break,
            ControlFlowKind::Continue,
            ControlFlowKind::Other,
        ];

        for kind in kinds {
            let cf = ControlFlow {
                kind,
                span: Span::default(),
                condition_span: None,
                branches: vec![],
            };
            assert_eq!(cf.kind, kind);
        }
    }

    // =========================================================================
    // Call Tests
    // =========================================================================

    #[test]
    fn test_call_function() {
        let call = Call {
            callee: "println".to_string(),
            span: Span::new(10, 30, 5, 5),
            argument_count: 2,
            is_method: false,
        };

        assert_eq!(call.callee, "println");
        assert_eq!(call.argument_count, 2);
        assert!(!call.is_method);
    }

    #[test]
    fn test_call_method() {
        let call = Call {
            callee: "self.process".to_string(),
            span: Span::new(10, 40, 5, 5),
            argument_count: 0,
            is_method: true,
        };

        assert!(call.is_method);
    }

    // =========================================================================
    // ImportLike Tests
    // =========================================================================

    #[test]
    fn test_import_like_simple() {
        let import = ImportLike {
            source: "std::collections".to_string(),
            kind: ImportKind::Use,
            items: vec!["HashMap".to_string()],
            alias: None,
            type_only: false,
            span: Span::default(),
        };

        assert_eq!(import.source, "std::collections");
        assert_eq!(import.kind, ImportKind::Use);
        assert_eq!(import.items.len(), 1);
    }

    #[test]
    fn test_import_like_with_alias() {
        let import = ImportLike {
            source: "numpy".to_string(),
            kind: ImportKind::Import,
            items: vec![],
            alias: Some("np".to_string()),
            type_only: false,
            span: Span::default(),
        };

        assert_eq!(import.alias, Some("np".to_string()));
    }

    #[test]
    fn test_import_kind_all() {
        let kinds = [
            ImportKind::Import,
            ImportKind::Require,
            ImportKind::Include,
            ImportKind::Using,
            ImportKind::Module,
            ImportKind::From,
            ImportKind::Use,
            ImportKind::Other,
        ];

        for kind in kinds {
            assert_eq!(kind, kind);
        }
    }

    // =========================================================================
    // Comment Tests
    // =========================================================================

    #[test]
    fn test_comment_line() {
        let comment = Comment {
            text: "This is a comment".to_string(),
            kind: CommentKind::Line,
            span: Span::new(0, 20, 1, 1),
            attached_to: None,
        };

        assert_eq!(comment.kind, CommentKind::Line);
        assert!(comment.attached_to.is_none());
    }

    #[test]
    fn test_comment_doc() {
        let comment = Comment {
            text: "Documentation for function".to_string(),
            kind: CommentKind::Doc,
            span: Span::new(0, 30, 1, 1),
            attached_to: Some("my_function".to_string()),
        };

        assert_eq!(comment.kind, CommentKind::Doc);
        assert_eq!(comment.attached_to, Some("my_function".to_string()));
    }

    #[test]
    fn test_comment_kind_all() {
        assert_eq!(CommentKind::Line, CommentKind::Line);
        assert_eq!(CommentKind::Block, CommentKind::Block);
        assert_eq!(CommentKind::Doc, CommentKind::Doc);
    }

    // =========================================================================
    // UnknownNode Tests
    // =========================================================================

    #[test]
    fn test_unknown_node_minimal() {
        let node = UnknownNode {
            span: Span::new(100, 150, 10, 12),
            reason: None,
            raw_text: None,
        };

        assert_eq!(node.span.start, 100);
        assert!(node.reason.is_none());
    }

    #[test]
    fn test_unknown_node_with_details() {
        let node = UnknownNode {
            span: Span::new(0, 50, 1, 3),
            reason: Some("unexpected token '}'".to_string()),
            raw_text: Some("} extra brace".to_string()),
        };

        assert!(node.reason.is_some());
        assert!(node.raw_text.is_some());
    }

    // =========================================================================
    // UnparsedBlock Tests
    // =========================================================================

    #[test]
    fn test_unparsed_block() {
        let block = UnparsedBlock {
            span: Span::new(200, 500, 20, 50),
            reason: "Complex macro expansion".to_string(),
        };

        assert_eq!(block.span.start, 200);
        assert!(block.reason.contains("macro"));
    }
}
