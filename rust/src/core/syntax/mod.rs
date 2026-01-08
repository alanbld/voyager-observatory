//! Voyager Observatory - Core Syntax Infrastructure
//!
//! This module provides AST-level parsing capabilities using Tree-sitter,
//! enabling the Fractal Telescope to see beyond raw text into the
//! structural "DNA" of code.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    SyntaxProvider Trait                     │
//! │  ┌─────────────────────────────────────────────────────────┤
//! │  │  parse(source) -> NormalizedAst                         │
//! │  │  language() -> Language                                 │
//! │  │  apply_plugin_hook(hook) -> Result<()>  [Reserved]      │
//! │  └─────────────────────────────────────────────────────────┤
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!          ┌───────────────────┼───────────────────┐
//!          ▼                   ▼                   ▼
//! ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
//! │ TreeSitter      │ │ TreeSitter      │ │ TreeSitter      │
//! │ Rust Adapter    │ │ Python Adapter  │ │ TypeScript...   │
//! └─────────────────┘ └─────────────────┘ └─────────────────┘
//! ```
//!
//! # Supported Languages (25 Core)
//!
//! | Category | Languages |
//! |----------|-----------|
//! | Systems  | Rust, C, C++, Go |
//! | JVM      | Java, Kotlin, Scala |
//! | .NET     | C# |
//! | Scripting| Python, Ruby, PHP, Lua |
//! | Web      | JavaScript, TypeScript, HTML, CSS |
//! | Mobile   | Swift, Kotlin |
//! | Data     | JSON, YAML, TOML, SQL |
//! | DevOps   | Bash, HCL, Dockerfile |
//! | Docs     | Markdown |
//!
//! # Example
//!
//! ```rust,ignore
//! use voyager_observatory::core::syntax::{SyntaxRegistry, Language};
//!
//! let registry = SyntaxRegistry::new();
//! let ast = registry.parse("fn main() { println!(\"Hello\"); }", Language::Rust)?;
//!
//! for symbol in ast.symbols() {
//!     println!("Found: {} at line {}", symbol.name, symbol.location.line);
//! }
//! ```

mod adapter;
mod ast;

pub use adapter::{SyntaxRegistry, TreeSitterAdapter};
pub use ast::{
    DiagnosticSeverity, Import, ImportKind, Location, Module, NormalizedAst, Parameter,
    ParseDiagnostic, Scope, Span, Symbol, SymbolKind, SymbolVisibility,
};

use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during syntax analysis
#[derive(Error, Debug)]
pub enum SyntaxError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Tree-sitter initialization failed: {0}")]
    InitializationError(String),

    #[error("Plugin hook error: {0}")]
    PluginHookError(String),
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    // Systems
    Rust,
    C,
    Cpp,
    Go,

    // JVM
    Java,
    Kotlin,
    Scala,

    // .NET
    CSharp,

    // Scripting
    Python,
    Ruby,
    Php,
    Lua,

    // Web
    JavaScript,
    TypeScript,
    Tsx,
    Html,
    Css,

    // Mobile (Swift uses same as systems, Kotlin above)
    Swift,

    // Data
    Json,
    Yaml,
    Toml,
    Sql,

    // DevOps
    Bash,
    Hcl,
    Dockerfile,

    // Docs
    Markdown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            // Systems
            "rs" => Some(Language::Rust),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
            "go" => Some(Language::Go),

            // JVM
            "java" => Some(Language::Java),
            "kt" | "kts" => Some(Language::Kotlin),
            "scala" | "sc" => Some(Language::Scala),

            // .NET
            "cs" => Some(Language::CSharp),

            // Scripting
            "py" | "pyw" | "pyi" => Some(Language::Python),
            "rb" | "rake" | "gemspec" => Some(Language::Ruby),
            "php" | "phtml" => Some(Language::Php),
            "lua" => Some(Language::Lua),

            // Web
            "js" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "mts" | "cts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
            "html" | "htm" => Some(Language::Html),
            "css" | "scss" | "sass" => Some(Language::Css),

            // Mobile
            "swift" => Some(Language::Swift),

            // Data
            "json" | "jsonc" => Some(Language::Json),
            "yaml" | "yml" => Some(Language::Yaml),
            "toml" => Some(Language::Toml),
            "sql" => Some(Language::Sql),

            // DevOps
            "sh" | "bash" | "zsh" => Some(Language::Bash),
            "tf" | "hcl" => Some(Language::Hcl),
            "dockerfile" => Some(Language::Dockerfile),

            // Docs
            "md" | "markdown" => Some(Language::Markdown),

            _ => None,
        }
    }

    /// Get the canonical file extension for this language
    pub fn extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Go => "go",
            Language::Java => "java",
            Language::Kotlin => "kt",
            Language::Scala => "scala",
            Language::CSharp => "cs",
            Language::Python => "py",
            Language::Ruby => "rb",
            Language::Php => "php",
            Language::Lua => "lua",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Tsx => "tsx",
            Language::Html => "html",
            Language::Css => "css",
            Language::Swift => "swift",
            Language::Json => "json",
            Language::Yaml => "yaml",
            Language::Toml => "toml",
            Language::Sql => "sql",
            Language::Bash => "sh",
            Language::Hcl => "tf",
            Language::Dockerfile => "dockerfile",
            Language::Markdown => "md",
        }
    }

    /// Get human-readable language name
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::Kotlin => "Kotlin",
            Language::Scala => "Scala",
            Language::CSharp => "C#",
            Language::Python => "Python",
            Language::Ruby => "Ruby",
            Language::Php => "PHP",
            Language::Lua => "Lua",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Tsx => "TSX",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::Swift => "Swift",
            Language::Json => "JSON",
            Language::Yaml => "YAML",
            Language::Toml => "TOML",
            Language::Sql => "SQL",
            Language::Bash => "Bash",
            Language::Hcl => "HCL",
            Language::Dockerfile => "Dockerfile",
            Language::Markdown => "Markdown",
        }
    }
}

/// Plugin hook definition (reserved for Phase 2 Lua ecosystem)
///
/// This structure defines the interface for external plugins to extend
/// syntax analysis capabilities. In Phase 2, Lua scripts will be able
/// to register hooks that modify or enhance the AST extraction process.
#[derive(Debug, Clone)]
pub struct PluginHook {
    /// Unique identifier for the hook
    pub id: String,

    /// Human-readable description
    pub description: String,

    /// Hook priority (lower = earlier execution)
    pub priority: i32,

    /// Reserved: Lua script path or inline code
    #[allow(dead_code)]
    lua_source: Option<String>,
}

/// The core trait for syntax analysis providers
///
/// This trait defines the interface that all syntax analyzers must implement.
/// The Tree-sitter adapter is the primary implementation, but the design
/// allows for alternative backends or custom analyzers.
pub trait SyntaxProvider: Send + Sync {
    /// Parse source code and extract a normalized AST
    ///
    /// # Arguments
    /// * `source` - The source code to parse
    /// * `language` - The programming language
    ///
    /// # Returns
    /// A `NormalizedAst` containing all extracted symbols, imports, and structure
    fn parse(&self, source: &str, language: Language) -> Result<NormalizedAst, SyntaxError>;

    /// Get the languages supported by this provider
    fn supported_languages(&self) -> &[Language];

    /// Check if a specific language is supported
    fn supports(&self, language: Language) -> bool {
        self.supported_languages().contains(&language)
    }

    /// Apply a plugin hook to modify parsing behavior
    ///
    /// # Phase 2 Reserved
    ///
    /// This method is reserved for the Phase 2 Lua plugin ecosystem.
    /// Currently returns `Ok(())` for all inputs.
    ///
    /// In Phase 2, hooks will be able to:
    /// - Add custom symbol extractors
    /// - Modify AST traversal order
    /// - Inject metadata into symbols
    /// - Filter or transform extracted data
    fn apply_plugin_hook(&mut self, _hook: PluginHook) -> Result<(), SyntaxError> {
        // Reserved for Phase 2 Lua ecosystem
        Ok(())
    }

    /// Get statistics about parsing performance
    fn stats(&self) -> ProviderStats {
        ProviderStats::default()
    }
}

/// Statistics about syntax provider performance
#[derive(Debug, Clone, Default)]
pub struct ProviderStats {
    /// Total files parsed
    pub files_parsed: usize,

    /// Total symbols extracted
    pub symbols_extracted: usize,

    /// Total parse time in milliseconds
    pub total_parse_time_ms: u64,

    /// Cache hit rate (0.0 - 1.0)
    pub cache_hit_rate: f64,

    /// Per-language statistics
    pub by_language: HashMap<Language, LanguageStats>,
}

/// Per-language parsing statistics
#[derive(Debug, Clone, Default)]
pub struct LanguageStats {
    pub files: usize,
    pub symbols: usize,
    pub avg_parse_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Language Tests
    // =========================================================================

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("tsx"), Some(Language::Tsx));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("java"), Some(Language::Java));
        assert_eq!(Language::from_extension("unknown"), None);
    }

    #[test]
    fn test_language_from_extension_systems() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("c"), Some(Language::C));
        assert_eq!(Language::from_extension("h"), Some(Language::C));
        assert_eq!(Language::from_extension("cpp"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("cc"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("cxx"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("hpp"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("hxx"), Some(Language::Cpp));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
    }

    #[test]
    fn test_language_from_extension_jvm() {
        assert_eq!(Language::from_extension("java"), Some(Language::Java));
        assert_eq!(Language::from_extension("kt"), Some(Language::Kotlin));
        assert_eq!(Language::from_extension("kts"), Some(Language::Kotlin));
        assert_eq!(Language::from_extension("scala"), Some(Language::Scala));
        assert_eq!(Language::from_extension("sc"), Some(Language::Scala));
    }

    #[test]
    fn test_language_from_extension_dotnet() {
        assert_eq!(Language::from_extension("cs"), Some(Language::CSharp));
    }

    #[test]
    fn test_language_from_extension_scripting() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("pyw"), Some(Language::Python));
        assert_eq!(Language::from_extension("pyi"), Some(Language::Python));
        assert_eq!(Language::from_extension("rb"), Some(Language::Ruby));
        assert_eq!(Language::from_extension("rake"), Some(Language::Ruby));
        assert_eq!(Language::from_extension("gemspec"), Some(Language::Ruby));
        assert_eq!(Language::from_extension("php"), Some(Language::Php));
        assert_eq!(Language::from_extension("phtml"), Some(Language::Php));
        assert_eq!(Language::from_extension("lua"), Some(Language::Lua));
    }

    #[test]
    fn test_language_from_extension_web() {
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("mjs"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("cjs"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("mts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("cts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("tsx"), Some(Language::Tsx));
        assert_eq!(Language::from_extension("html"), Some(Language::Html));
        assert_eq!(Language::from_extension("htm"), Some(Language::Html));
        assert_eq!(Language::from_extension("css"), Some(Language::Css));
        assert_eq!(Language::from_extension("scss"), Some(Language::Css));
        assert_eq!(Language::from_extension("sass"), Some(Language::Css));
    }

    #[test]
    fn test_language_from_extension_mobile() {
        assert_eq!(Language::from_extension("swift"), Some(Language::Swift));
    }

    #[test]
    fn test_language_from_extension_data() {
        assert_eq!(Language::from_extension("json"), Some(Language::Json));
        assert_eq!(Language::from_extension("jsonc"), Some(Language::Json));
        assert_eq!(Language::from_extension("yaml"), Some(Language::Yaml));
        assert_eq!(Language::from_extension("yml"), Some(Language::Yaml));
        assert_eq!(Language::from_extension("toml"), Some(Language::Toml));
        assert_eq!(Language::from_extension("sql"), Some(Language::Sql));
    }

    #[test]
    fn test_language_from_extension_devops() {
        assert_eq!(Language::from_extension("sh"), Some(Language::Bash));
        assert_eq!(Language::from_extension("bash"), Some(Language::Bash));
        assert_eq!(Language::from_extension("zsh"), Some(Language::Bash));
        assert_eq!(Language::from_extension("tf"), Some(Language::Hcl));
        assert_eq!(Language::from_extension("hcl"), Some(Language::Hcl));
        assert_eq!(
            Language::from_extension("dockerfile"),
            Some(Language::Dockerfile)
        );
    }

    #[test]
    fn test_language_from_extension_docs() {
        assert_eq!(Language::from_extension("md"), Some(Language::Markdown));
        assert_eq!(
            Language::from_extension("markdown"),
            Some(Language::Markdown)
        );
    }

    #[test]
    fn test_language_from_extension_case_insensitive() {
        assert_eq!(Language::from_extension("RS"), Some(Language::Rust));
        assert_eq!(Language::from_extension("PY"), Some(Language::Python));
        assert_eq!(Language::from_extension("Ts"), Some(Language::TypeScript));
    }

    #[test]
    fn test_language_extension_roundtrip() {
        let languages = [
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::Go,
            Language::Java,
        ];

        for lang in languages {
            let ext = lang.extension();
            let recovered = Language::from_extension(ext);
            assert_eq!(recovered, Some(lang), "Roundtrip failed for {:?}", lang);
        }
    }

    #[test]
    fn test_language_extension_all() {
        assert_eq!(Language::Rust.extension(), "rs");
        assert_eq!(Language::C.extension(), "c");
        assert_eq!(Language::Cpp.extension(), "cpp");
        assert_eq!(Language::Go.extension(), "go");
        assert_eq!(Language::Java.extension(), "java");
        assert_eq!(Language::Kotlin.extension(), "kt");
        assert_eq!(Language::Scala.extension(), "scala");
        assert_eq!(Language::CSharp.extension(), "cs");
        assert_eq!(Language::Python.extension(), "py");
        assert_eq!(Language::Ruby.extension(), "rb");
        assert_eq!(Language::Php.extension(), "php");
        assert_eq!(Language::Lua.extension(), "lua");
        assert_eq!(Language::JavaScript.extension(), "js");
        assert_eq!(Language::TypeScript.extension(), "ts");
        assert_eq!(Language::Tsx.extension(), "tsx");
        assert_eq!(Language::Html.extension(), "html");
        assert_eq!(Language::Css.extension(), "css");
        assert_eq!(Language::Swift.extension(), "swift");
        assert_eq!(Language::Json.extension(), "json");
        assert_eq!(Language::Yaml.extension(), "yaml");
        assert_eq!(Language::Toml.extension(), "toml");
        assert_eq!(Language::Sql.extension(), "sql");
        assert_eq!(Language::Bash.extension(), "sh");
        assert_eq!(Language::Hcl.extension(), "tf");
        assert_eq!(Language::Dockerfile.extension(), "dockerfile");
        assert_eq!(Language::Markdown.extension(), "md");
    }

    #[test]
    fn test_language_names() {
        assert_eq!(Language::Rust.name(), "Rust");
        assert_eq!(Language::CSharp.name(), "C#");
        assert_eq!(Language::Cpp.name(), "C++");
        assert_eq!(Language::TypeScript.name(), "TypeScript");
    }

    #[test]
    fn test_language_names_all() {
        assert_eq!(Language::Rust.name(), "Rust");
        assert_eq!(Language::C.name(), "C");
        assert_eq!(Language::Cpp.name(), "C++");
        assert_eq!(Language::Go.name(), "Go");
        assert_eq!(Language::Java.name(), "Java");
        assert_eq!(Language::Kotlin.name(), "Kotlin");
        assert_eq!(Language::Scala.name(), "Scala");
        assert_eq!(Language::CSharp.name(), "C#");
        assert_eq!(Language::Python.name(), "Python");
        assert_eq!(Language::Ruby.name(), "Ruby");
        assert_eq!(Language::Php.name(), "PHP");
        assert_eq!(Language::Lua.name(), "Lua");
        assert_eq!(Language::JavaScript.name(), "JavaScript");
        assert_eq!(Language::TypeScript.name(), "TypeScript");
        assert_eq!(Language::Tsx.name(), "TSX");
        assert_eq!(Language::Html.name(), "HTML");
        assert_eq!(Language::Css.name(), "CSS");
        assert_eq!(Language::Swift.name(), "Swift");
        assert_eq!(Language::Json.name(), "JSON");
        assert_eq!(Language::Yaml.name(), "YAML");
        assert_eq!(Language::Toml.name(), "TOML");
        assert_eq!(Language::Sql.name(), "SQL");
        assert_eq!(Language::Bash.name(), "Bash");
        assert_eq!(Language::Hcl.name(), "HCL");
        assert_eq!(Language::Dockerfile.name(), "Dockerfile");
        assert_eq!(Language::Markdown.name(), "Markdown");
    }

    #[test]
    fn test_language_clone_copy_eq() {
        let lang = Language::Rust;
        let cloned = lang;
        assert_eq!(lang, cloned);
        assert_eq!(lang, Language::Rust);
        assert_ne!(lang, Language::Python);
    }

    #[test]
    fn test_language_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Language::Rust);
        set.insert(Language::Python);
        set.insert(Language::Rust); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&Language::Rust));
        assert!(set.contains(&Language::Python));
    }

    // =========================================================================
    // SyntaxError Tests
    // =========================================================================

    #[test]
    fn test_syntax_error_unsupported_language() {
        let error = SyntaxError::UnsupportedLanguage("brainfuck".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Unsupported language"));
        assert!(display.contains("brainfuck"));
    }

    #[test]
    fn test_syntax_error_parse_error() {
        let error = SyntaxError::ParseError {
            line: 10,
            column: 5,
            message: "unexpected token".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("line 10"));
        assert!(display.contains("column 5"));
        assert!(display.contains("unexpected token"));
    }

    #[test]
    fn test_syntax_error_initialization() {
        let error = SyntaxError::InitializationError("failed to load grammar".to_string());
        let display = format!("{}", error);
        assert!(display.contains("initialization failed"));
        assert!(display.contains("failed to load grammar"));
    }

    #[test]
    fn test_syntax_error_plugin_hook() {
        let error = SyntaxError::PluginHookError("hook failed".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Plugin hook error"));
        assert!(display.contains("hook failed"));
    }

    #[test]
    fn test_syntax_error_debug() {
        let error = SyntaxError::UnsupportedLanguage("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("UnsupportedLanguage"));
    }

    // =========================================================================
    // PluginHook Tests
    // =========================================================================

    #[test]
    fn test_plugin_hook_creation() {
        let hook = PluginHook {
            id: "test-hook".to_string(),
            description: "A test hook".to_string(),
            priority: 100,
            lua_source: None,
        };

        assert_eq!(hook.id, "test-hook");
        assert_eq!(hook.priority, 100);
    }

    #[test]
    fn test_plugin_hook_with_lua_source() {
        let hook = PluginHook {
            id: "lua-hook".to_string(),
            description: "A Lua hook".to_string(),
            priority: 50,
            lua_source: Some("return true".to_string()),
        };

        assert_eq!(hook.id, "lua-hook");
        assert_eq!(hook.priority, 50);
    }

    #[test]
    fn test_plugin_hook_clone() {
        let hook = PluginHook {
            id: "cloneable".to_string(),
            description: "Test".to_string(),
            priority: 1,
            lua_source: None,
        };
        let cloned = hook.clone();
        assert_eq!(cloned.id, "cloneable");
    }

    // =========================================================================
    // ProviderStats Tests
    // =========================================================================

    #[test]
    fn test_provider_stats_default() {
        let stats = ProviderStats::default();
        assert_eq!(stats.files_parsed, 0);
        assert_eq!(stats.symbols_extracted, 0);
        assert_eq!(stats.total_parse_time_ms, 0);
        assert_eq!(stats.cache_hit_rate, 0.0);
        assert!(stats.by_language.is_empty());
    }

    #[test]
    fn test_provider_stats_with_data() {
        let mut stats = ProviderStats {
            files_parsed: 100,
            symbols_extracted: 500,
            total_parse_time_ms: 1500,
            cache_hit_rate: 0.75,
            by_language: HashMap::new(),
        };

        stats.by_language.insert(
            Language::Rust,
            LanguageStats {
                files: 50,
                symbols: 300,
                avg_parse_time_ms: 10.5,
            },
        );

        assert_eq!(stats.files_parsed, 100);
        assert_eq!(stats.by_language.len(), 1);
        assert_eq!(stats.by_language.get(&Language::Rust).unwrap().files, 50);
    }

    #[test]
    fn test_language_stats_default() {
        let stats = LanguageStats::default();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.symbols, 0);
        assert_eq!(stats.avg_parse_time_ms, 0.0);
    }

    #[test]
    fn test_provider_stats_clone() {
        let stats = ProviderStats {
            files_parsed: 10,
            symbols_extracted: 50,
            total_parse_time_ms: 100,
            cache_hit_rate: 0.5,
            by_language: HashMap::new(),
        };
        let cloned = stats.clone();
        assert_eq!(cloned.files_parsed, 10);
    }
}
