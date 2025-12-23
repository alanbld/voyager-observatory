//! Fractal Context Builder
//!
//! This module provides a builder pattern for constructing `FractalContext`
//! instances from source files and directories.
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::builder::{FractalContextBuilder, ExtractionDepth};
//!
//! let context = FractalContextBuilder::for_file("src/main.rs")
//!     .with_depth(ExtractionDepth::Standard)
//!     .with_relationships(true)
//!     .build()?;
//! ```

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use regex::Regex;
use thiserror::Error;

use super::context::{FractalContext, GraphEdge, GraphNode, NodeType, RelationshipType};
use super::layers::{
    ContextLayer, Import, LayerContent, Parameter, Range, SymbolKind, Visibility, ZoomLevel,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during context building.
#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

pub type BuilderResult<T> = Result<T, BuilderError>;

// =============================================================================
// Configuration Types
// =============================================================================

/// Depth of extraction - controls how much detail to extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractionDepth {
    /// Only file-level information (fast)
    Minimal,
    /// Files + symbols (functions, classes, structs)
    #[default]
    Standard,
    /// Full extraction including blocks and expressions
    Full,
    /// Custom depth with specific levels enabled
    Custom {
        include_symbols: bool,
        include_blocks: bool,
        include_lines: bool,
    },
}

impl ExtractionDepth {
    pub fn include_symbols(&self) -> bool {
        match self {
            ExtractionDepth::Minimal => false,
            ExtractionDepth::Standard => true,
            ExtractionDepth::Full => true,
            ExtractionDepth::Custom { include_symbols, .. } => *include_symbols,
        }
    }

    pub fn include_blocks(&self) -> bool {
        match self {
            ExtractionDepth::Minimal => false,
            ExtractionDepth::Standard => false,
            ExtractionDepth::Full => true,
            ExtractionDepth::Custom { include_blocks, .. } => *include_blocks,
        }
    }

    pub fn include_lines(&self) -> bool {
        match self {
            ExtractionDepth::Minimal => false,
            ExtractionDepth::Standard => false,
            ExtractionDepth::Full => true,
            ExtractionDepth::Custom { include_lines, .. } => *include_lines,
        }
    }
}

/// Configuration for the builder.
#[derive(Debug, Clone)]
pub struct BuilderConfig {
    /// How deep to extract
    pub depth: ExtractionDepth,
    /// Whether to extract relationships (calls, imports)
    pub extract_relationships: bool,
    /// Whether to cluster similar elements
    pub cluster_similar: bool,
    /// Minimum confidence score for extracted elements
    pub min_confidence: f32,
    /// Maximum file size to process (in bytes)
    pub max_file_size: u64,
    /// File extensions to process (empty = all)
    pub extensions: Vec<String>,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            depth: ExtractionDepth::Standard,
            extract_relationships: true,
            cluster_similar: false,
            min_confidence: 0.5,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            extensions: vec![],
        }
    }
}

// =============================================================================
// Extracted Symbol
// =============================================================================

/// A symbol extracted from source code.
#[derive(Debug, Clone)]
pub struct ExtractedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    pub return_type: Option<String>,
    pub parameters: Vec<Parameter>,
    pub documentation: Option<String>,
    pub visibility: Visibility,
    pub range: Range,
    pub calls: Vec<String>,
}

// =============================================================================
// Language Detection
// =============================================================================

/// Detect language from file extension.
pub fn detect_language(path: &Path) -> Option<&'static str> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "rs" => Some("rust"),
            "py" => Some("python"),
            "js" | "mjs" => Some("javascript"),
            "ts" | "mts" => Some("typescript"),
            "go" => Some("go"),
            "java" => Some("java"),
            "c" | "h" => Some("c"),
            "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
            "cs" => Some("csharp"),
            "rb" => Some("ruby"),
            "sh" | "bash" | "ksh" | "zsh" => Some("shell"),
            "p" | "i" | "w" | "cls" => Some("abl"),
            _ => None,
        })
}

// =============================================================================
// Regex-Based Symbol Extraction
// =============================================================================

/// Extract symbols from source code using regex patterns.
pub fn extract_symbols_regex(content: &str, language: &str) -> Vec<ExtractedSymbol> {
    match language {
        "rust" => extract_rust_symbols(content),
        "python" => extract_python_symbols(content),
        "javascript" | "typescript" => extract_js_ts_symbols(content),
        "go" => extract_go_symbols(content),
        "shell" => extract_shell_symbols(content),
        _ => Vec::new(),
    }
}

fn extract_rust_symbols(content: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();

    // Rust function pattern
    let fn_pattern = Regex::new(
        r"(?m)^[ \t]*(pub(?:\([^)]+\))?\s+)?(async\s+)?(unsafe\s+)?(fn)\s+(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*->\s*([^\s{]+))?"
    ).unwrap();

    // Rust struct pattern
    let struct_pattern = Regex::new(
        r"(?m)^[ \t]*(pub(?:\([^)]+\))?\s+)?struct\s+(\w+)(?:<[^>]+>)?"
    ).unwrap();

    // Rust enum pattern
    let enum_pattern = Regex::new(
        r"(?m)^[ \t]*(pub(?:\([^)]+\))?\s+)?enum\s+(\w+)(?:<[^>]+>)?"
    ).unwrap();

    // Rust trait pattern
    let trait_pattern = Regex::new(
        r"(?m)^[ \t]*(pub(?:\([^)]+\))?\s+)?trait\s+(\w+)(?:<[^>]+>)?"
    ).unwrap();

    // Rust impl pattern
    let impl_pattern = Regex::new(
        r"(?m)^[ \t]*impl(?:<[^>]+>)?\s+(?:(\w+)\s+for\s+)?(\w+)"
    ).unwrap();

    // Rust const/static pattern
    let const_pattern = Regex::new(
        r"(?m)^[ \t]*(pub(?:\([^)]+\))?\s+)?(const|static)\s+(\w+)\s*:\s*([^=]+)"
    ).unwrap();

    // Rust macro pattern
    let macro_pattern = Regex::new(
        r"(?m)^[ \t]*(pub(?:\([^)]+\))?\s+)?macro_rules!\s+(\w+)"
    ).unwrap();

    let lines: Vec<&str> = content.lines().collect();

    // Extract functions
    for cap in fn_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_pub = cap.get(1).is_some();
        let is_async = cap.get(2).is_some();
        let is_unsafe = cap.get(3).is_some();
        let name = cap.get(5).map(|m| m.as_str()).unwrap_or("");
        let params = cap.get(6).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap.get(7).map(|m| m.as_str().to_string());

        let mut sig = String::new();
        if is_pub {
            sig.push_str("pub ");
        }
        if is_async {
            sig.push_str("async ");
        }
        if is_unsafe {
            sig.push_str("unsafe ");
        }
        sig.push_str(&format!("fn {}({})", name, params));
        if let Some(ref ret) = return_type {
            sig.push_str(&format!(" -> {}", ret));
        }

        // Check if this is a test function
        let is_test = start_line > 0
            && lines
                .get(start_line.saturating_sub(1))
                .map(|l| l.contains("#[test]") || l.contains("#[tokio::test]"))
                .unwrap_or(false);

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: if is_test {
                SymbolKind::Test
            } else {
                SymbolKind::Function
            },
            signature: sig,
            return_type,
            parameters: parse_rust_params(params),
            documentation: extract_doc_comment(&lines, start_line),
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract structs
    for cap in struct_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_pub = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Struct,
            signature: format!(
                "{}struct {}",
                if is_pub { "pub " } else { "" },
                name
            ),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_doc_comment(&lines, start_line),
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract enums
    for cap in enum_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_pub = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Enum,
            signature: format!("{}enum {}", if is_pub { "pub " } else { "" }, name),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_doc_comment(&lines, start_line),
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract traits
    for cap in trait_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_pub = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Trait,
            signature: format!(
                "{}trait {}",
                if is_pub { "pub " } else { "" },
                name
            ),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_doc_comment(&lines, start_line),
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract impl blocks (not as symbols, but note them for methods)
    for cap in impl_pattern.captures_iter(content) {
        let _trait_name = cap.get(1).map(|m| m.as_str());
        let _type_name = cap.get(2).map(|m| m.as_str());
        // Impl blocks are structural, methods inside are already captured
    }

    // Extract constants
    for cap in const_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_pub = cap.get(1).is_some();
        let const_type = cap.get(2).map(|m| m.as_str()).unwrap_or("const");
        let name = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        let type_hint = cap.get(4).map(|m| m.as_str().trim().to_string());

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Constant,
            signature: format!(
                "{}{} {}: {}",
                if is_pub { "pub " } else { "" },
                const_type,
                name,
                type_hint.as_deref().unwrap_or("?")
            ),
            return_type: type_hint,
            parameters: Vec::new(),
            documentation: extract_doc_comment(&lines, start_line),
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract macros
    for cap in macro_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_pub = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Macro,
            signature: format!(
                "{}macro_rules! {}",
                if is_pub { "pub " } else { "" },
                name
            ),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_doc_comment(&lines, start_line),
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    symbols
}

fn extract_python_symbols(content: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();

    // Python function pattern
    let fn_pattern = Regex::new(
        r"(?m)^[ \t]*(async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^\s:]+))?\s*:"
    ).unwrap();

    // Python class pattern
    let class_pattern = Regex::new(r"(?m)^[ \t]*class\s+(\w+)(?:\([^)]*\))?\s*:").unwrap();

    let lines: Vec<&str> = content.lines().collect();

    // Extract functions
    for cap in fn_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_async = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let params = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap.get(4).map(|m| m.as_str().to_string());

        // Check indentation to determine if it's a method
        let line_text = lines.get(start_line).unwrap_or(&"");
        let indent = line_text.len() - line_text.trim_start().len();
        let kind = if indent > 0 {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };

        let mut sig = String::new();
        if is_async {
            sig.push_str("async ");
        }
        sig.push_str(&format!("def {}({})", name, params));
        if let Some(ref ret) = return_type {
            sig.push_str(&format!(" -> {}", ret));
        }

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind,
            signature: sig,
            return_type,
            parameters: parse_python_params(params),
            documentation: extract_python_docstring(&lines, start_line),
            visibility: if name.starts_with('_') {
                Visibility::Private
            } else {
                Visibility::Public
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract classes
    for cap in class_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Class,
            signature: format!("class {}", name),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_python_docstring(&lines, start_line),
            visibility: if name.starts_with('_') {
                Visibility::Private
            } else {
                Visibility::Public
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    symbols
}

fn extract_js_ts_symbols(content: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();

    // Function declaration pattern
    let fn_pattern = Regex::new(
        r"(?m)^[ \t]*(export\s+)?(async\s+)?function\s+(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*:\s*([^\s{]+))?"
    ).unwrap();

    // Arrow function pattern
    let arrow_pattern = Regex::new(
        r"(?m)^[ \t]*(export\s+)?(const|let|var)\s+(\w+)\s*(?::\s*[^=]+)?\s*=\s*(async\s+)?\([^)]*\)\s*(?::\s*([^\s=]+))?\s*=>"
    ).unwrap();

    // Class pattern
    let class_pattern =
        Regex::new(r"(?m)^[ \t]*(export\s+)?class\s+(\w+)(?:\s+extends\s+\w+)?").unwrap();

    // Interface pattern (TypeScript)
    let interface_pattern =
        Regex::new(r"(?m)^[ \t]*(export\s+)?interface\s+(\w+)(?:<[^>]+>)?").unwrap();

    let lines: Vec<&str> = content.lines().collect();

    // Extract functions
    for cap in fn_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_export = cap.get(1).is_some();
        let is_async = cap.get(2).is_some();
        let name = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        let params = cap.get(4).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap.get(5).map(|m| m.as_str().to_string());

        let mut sig = String::new();
        if is_export {
            sig.push_str("export ");
        }
        if is_async {
            sig.push_str("async ");
        }
        sig.push_str(&format!("function {}({})", name, params));
        if let Some(ref ret) = return_type {
            sig.push_str(&format!(": {}", ret));
        }

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: sig,
            return_type,
            parameters: Vec::new(),
            documentation: extract_jsdoc(&lines, start_line),
            visibility: if is_export {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract arrow functions
    for cap in arrow_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_export = cap.get(1).is_some();
        let name = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        let is_async = cap.get(4).is_some();
        let return_type = cap.get(5).map(|m| m.as_str().to_string());

        let mut sig = String::new();
        if is_export {
            sig.push_str("export ");
        }
        sig.push_str("const ");
        sig.push_str(name);
        sig.push_str(" = ");
        if is_async {
            sig.push_str("async ");
        }
        sig.push_str("() =>");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: sig,
            return_type,
            parameters: Vec::new(),
            documentation: extract_jsdoc(&lines, start_line),
            visibility: if is_export {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract classes
    for cap in class_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_export = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Class,
            signature: format!(
                "{}class {}",
                if is_export { "export " } else { "" },
                name
            ),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_jsdoc(&lines, start_line),
            visibility: if is_export {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract interfaces (TypeScript)
    for cap in interface_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let is_export = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Interface,
            signature: format!(
                "{}interface {}",
                if is_export { "export " } else { "" },
                name
            ),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_jsdoc(&lines, start_line),
            visibility: if is_export {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    symbols
}

fn extract_go_symbols(content: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();

    // Go function pattern
    let fn_pattern = Regex::new(
        r"(?m)^func\s+(?:\([^)]+\)\s+)?(\w+)\s*\(([^)]*)\)(?:\s*\(([^)]+)\)|\s+([^\s{]+))?"
    ).unwrap();

    // Go type pattern (struct/interface)
    let type_pattern = Regex::new(r"(?m)^type\s+(\w+)\s+(struct|interface)").unwrap();

    let lines: Vec<&str> = content.lines().collect();

    // Extract functions
    for cap in fn_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let return_type = cap
            .get(3)
            .or(cap.get(4))
            .map(|m| m.as_str().to_string());

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: format!("func {}({})", name, params),
            return_type,
            parameters: Vec::new(),
            documentation: extract_go_comment(&lines, start_line),
            visibility: if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    // Extract types
    for cap in type_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let type_kind = cap.get(2).map(|m| m.as_str()).unwrap_or("struct");

        let kind = if type_kind == "interface" {
            SymbolKind::Interface
        } else {
            SymbolKind::Struct
        };

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind,
            signature: format!("type {} {}", name, type_kind),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_go_comment(&lines, start_line),
            visibility: if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                Visibility::Public
            } else {
                Visibility::Private
            },
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    symbols
}

fn extract_shell_symbols(content: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = Vec::new();

    // Shell function pattern
    let fn_pattern = Regex::new(r"(?m)^(\w+)\s*\(\)\s*\{|^function\s+(\w+)").unwrap();

    let lines: Vec<&str> = content.lines().collect();

    for cap in fn_pattern.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let start_line = content[..full_match.start()].lines().count();
        let name = cap
            .get(1)
            .or(cap.get(2))
            .map(|m| m.as_str())
            .unwrap_or("");

        symbols.push(ExtractedSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            signature: format!("{}()", name),
            return_type: None,
            parameters: Vec::new(),
            documentation: extract_shell_comment(&lines, start_line),
            visibility: Visibility::Public,
            range: Range::single_line(start_line + 1),
            calls: Vec::new(),
        });
    }

    symbols
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_rust_params(params: &str) -> Vec<Parameter> {
    if params.trim().is_empty() {
        return Vec::new();
    }

    params
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() || p == "self" || p == "&self" || p == "&mut self" {
                return None;
            }

            let parts: Vec<&str> = p.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some(Parameter {
                    name: parts[0].trim().to_string(),
                    type_hint: Some(parts[1].trim().to_string()),
                    default_value: None,
                })
            } else {
                Some(Parameter {
                    name: p.to_string(),
                    type_hint: None,
                    default_value: None,
                })
            }
        })
        .collect()
}

fn parse_python_params(params: &str) -> Vec<Parameter> {
    if params.trim().is_empty() {
        return Vec::new();
    }

    params
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() || p == "self" || p == "cls" {
                return None;
            }

            // Handle default values
            let (param, default) = if let Some(eq_pos) = p.find('=') {
                (
                    &p[..eq_pos],
                    Some(p[eq_pos + 1..].trim().to_string()),
                )
            } else {
                (p, None)
            };

            // Handle type hints
            let parts: Vec<&str> = param.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some(Parameter {
                    name: parts[0].trim().to_string(),
                    type_hint: Some(parts[1].trim().to_string()),
                    default_value: default,
                })
            } else {
                Some(Parameter {
                    name: parts[0].trim().to_string(),
                    type_hint: None,
                    default_value: default,
                })
            }
        })
        .collect()
}

fn extract_doc_comment(lines: &[&str], start_line: usize) -> Option<String> {
    let mut docs = Vec::new();
    let mut line_idx = start_line.saturating_sub(1);

    while line_idx > 0 {
        let line = lines.get(line_idx)?;
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            docs.push(trimmed.trim_start_matches("///").trim());
            line_idx = line_idx.saturating_sub(1);
        } else if trimmed.starts_with("//!") {
            // Skip module-level docs
            break;
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            // Skip empty lines and attributes
            line_idx = line_idx.saturating_sub(1);
        } else {
            break;
        }
    }

    if docs.is_empty() {
        None
    } else {
        docs.reverse();
        Some(docs.join(" "))
    }
}

fn extract_python_docstring(lines: &[&str], start_line: usize) -> Option<String> {
    // Look for docstring on the line after the def/class
    let next_line = lines.get(start_line + 1)?;
    let trimmed = next_line.trim();

    if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        let quote = if trimmed.starts_with("\"\"\"") {
            "\"\"\""
        } else {
            "'''"
        };
        let content = trimmed.trim_start_matches(quote).trim_end_matches(quote);
        if !content.is_empty() {
            return Some(content.to_string());
        }
    }

    None
}

fn extract_jsdoc(lines: &[&str], start_line: usize) -> Option<String> {
    let mut docs = Vec::new();
    let mut line_idx = start_line.saturating_sub(1);
    let mut in_jsdoc = false;

    while line_idx > 0 {
        let line = lines.get(line_idx)?;
        let trimmed = line.trim();

        if trimmed.ends_with("*/") {
            in_jsdoc = true;
            let content = trimmed.trim_end_matches("*/").trim_start_matches("/**").trim();
            if !content.is_empty() {
                docs.push(content);
            }
        } else if in_jsdoc {
            if trimmed.starts_with("/**") {
                break;
            } else if trimmed.starts_with('*') {
                let content = trimmed.trim_start_matches('*').trim();
                if !content.starts_with('@') && !content.is_empty() {
                    docs.push(content);
                }
            }
        } else if trimmed.is_empty() {
            line_idx = line_idx.saturating_sub(1);
            continue;
        } else {
            break;
        }

        line_idx = line_idx.saturating_sub(1);
    }

    if docs.is_empty() {
        None
    } else {
        docs.reverse();
        Some(docs.join(" "))
    }
}

fn extract_go_comment(lines: &[&str], start_line: usize) -> Option<String> {
    let mut docs = Vec::new();
    let mut line_idx = start_line.saturating_sub(1);

    while line_idx > 0 {
        let line = lines.get(line_idx)?;
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            docs.push(trimmed.trim_start_matches("//").trim());
            line_idx = line_idx.saturating_sub(1);
        } else if trimmed.is_empty() {
            line_idx = line_idx.saturating_sub(1);
        } else {
            break;
        }
    }

    if docs.is_empty() {
        None
    } else {
        docs.reverse();
        Some(docs.join(" "))
    }
}

fn extract_shell_comment(lines: &[&str], start_line: usize) -> Option<String> {
    let mut docs = Vec::new();
    let mut line_idx = start_line.saturating_sub(1);

    while line_idx > 0 {
        let line = lines.get(line_idx)?;
        let trimmed = line.trim();

        if trimmed.starts_with('#') && !trimmed.starts_with("#!") {
            docs.push(trimmed.trim_start_matches('#').trim());
            line_idx = line_idx.saturating_sub(1);
        } else if trimmed.is_empty() {
            line_idx = line_idx.saturating_sub(1);
        } else {
            break;
        }
    }

    if docs.is_empty() {
        None
    } else {
        docs.reverse();
        Some(docs.join(" "))
    }
}

/// Extract imports from source code.
fn extract_imports(content: &str, language: &str) -> Vec<Import> {
    match language {
        "rust" => extract_rust_imports(content),
        "python" => extract_python_imports(content),
        "javascript" | "typescript" => extract_js_imports(content),
        _ => Vec::new(),
    }
}

fn extract_rust_imports(content: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    let use_pattern = Regex::new(r"(?m)^use\s+([^;]+);").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(cap) = use_pattern.captures(line) {
            let path = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            imports.push(Import {
                module: path.to_string(),
                items: Vec::new(),
                alias: None,
                line: line_num + 1,
            });
        }
    }

    imports
}

fn extract_python_imports(content: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    let import_pattern = Regex::new(r"(?m)^(?:from\s+(\S+)\s+)?import\s+(.+)$").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(cap) = import_pattern.captures(line) {
            let from_module = cap.get(1).map(|m| m.as_str());
            let items = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if let Some(module) = from_module {
                imports.push(Import {
                    module: module.to_string(),
                    items: items.split(',').map(|s| s.trim().to_string()).collect(),
                    alias: None,
                    line: line_num + 1,
                });
            } else {
                imports.push(Import {
                    module: items.to_string(),
                    items: Vec::new(),
                    alias: None,
                    line: line_num + 1,
                });
            }
        }
    }

    imports
}

fn extract_js_imports(content: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    let import_pattern = Regex::new(r#"(?m)^import\s+(?:\{([^}]+)\}|(\w+))\s+from\s+['"]([^'"]+)['"]"#).unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(cap) = import_pattern.captures(line) {
            let named_imports = cap.get(1).map(|m| m.as_str());
            let default_import = cap.get(2).map(|m| m.as_str());
            let module = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            let items = if let Some(named) = named_imports {
                named.split(',').map(|s| s.trim().to_string()).collect()
            } else if let Some(default) = default_import {
                vec![default.to_string()]
            } else {
                Vec::new()
            };

            imports.push(Import {
                module: module.to_string(),
                items,
                alias: None,
                line: line_num + 1,
            });
        }
    }

    imports
}

// =============================================================================
// FractalContextBuilder
// =============================================================================

/// Builder for constructing `FractalContext` instances.
#[derive(Debug)]
pub struct FractalContextBuilder {
    source: BuilderSource,
    config: BuilderConfig,
    id_counter: usize,
}

#[derive(Debug, Clone)]
enum BuilderSource {
    File(PathBuf),
    Directory(PathBuf),
    Content { content: String, language: String },
}

impl FractalContextBuilder {
    /// Create a builder for a single file.
    pub fn for_file(path: impl AsRef<Path>) -> Self {
        Self {
            source: BuilderSource::File(path.as_ref().to_path_buf()),
            config: BuilderConfig::default(),
            id_counter: 0,
        }
    }

    /// Create a builder for a directory (project).
    pub fn for_directory(path: impl AsRef<Path>) -> Self {
        Self {
            source: BuilderSource::Directory(path.as_ref().to_path_buf()),
            config: BuilderConfig::default(),
            id_counter: 0,
        }
    }

    /// Create a builder for raw content.
    pub fn for_content(content: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            source: BuilderSource::Content {
                content: content.into(),
                language: language.into(),
            },
            config: BuilderConfig::default(),
            id_counter: 0,
        }
    }

    /// Set extraction depth.
    pub fn with_depth(mut self, depth: ExtractionDepth) -> Self {
        self.config.depth = depth;
        self
    }

    /// Enable/disable relationship extraction.
    pub fn with_relationships(mut self, extract: bool) -> Self {
        self.config.extract_relationships = extract;
        self
    }

    /// Enable/disable semantic clustering.
    pub fn with_clustering(mut self, cluster: bool) -> Self {
        self.config.cluster_similar = cluster;
        self
    }

    /// Set minimum confidence threshold.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.config.min_confidence = confidence;
        self
    }

    /// Set maximum file size.
    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.config.max_file_size = size;
        self
    }

    /// Set file extensions to process.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.config.extensions = extensions;
        self
    }

    /// Build the fractal context.
    pub fn build(mut self) -> BuilderResult<FractalContext> {
        let start_time = Instant::now();

        let context = match &self.source {
            BuilderSource::File(path) => self.build_for_file(path.clone())?,
            BuilderSource::Directory(path) => self.build_for_directory(path.clone())?,
            BuilderSource::Content { content, language } => {
                self.build_for_content(content.clone(), language.clone())?
            }
        };

        // Update extraction time
        let mut context = context;
        context.metadata.extraction_time = start_time.elapsed();

        Ok(context)
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.id_counter += 1;
        format!("{}_{:04}", prefix, self.id_counter)
    }

    fn build_for_file(&mut self, path: PathBuf) -> BuilderResult<FractalContext> {
        if !path.exists() {
            return Err(BuilderError::FileNotFound(path));
        }

        let content = fs::read_to_string(&path)?;
        let language = detect_language(&path)
            .ok_or_else(|| BuilderError::UnsupportedLanguage(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            ))?;

        let metadata = fs::metadata(&path)?;
        let line_count = content.lines().count();

        // Extract symbols if configured
        let symbols = if self.config.depth.include_symbols() {
            extract_symbols_regex(&content, language)
        } else {
            Vec::new()
        };

        // Extract imports
        let imports = extract_imports(&content, language);

        // Create file layer
        let file_id = self.next_id("file");
        let file_layer = ContextLayer::new(
            &file_id,
            LayerContent::File {
                path: path.clone(),
                language: language.to_string(),
                size_bytes: metadata.len(),
                line_count,
                symbol_count: symbols.len(),
                imports,
            },
        );

        let context_id = self.next_id("ctx");
        let mut context = FractalContext::new(&context_id, file_layer);

        // Add symbol layers
        let symbol_ids: Vec<String> = symbols
            .iter()
            .map(|sym| {
                let sym_id = self.next_id("sym");
                let sym_layer = ContextLayer::new(
                    &sym_id,
                    LayerContent::Symbol {
                        name: sym.name.clone(),
                        kind: sym.kind.clone(),
                        signature: sym.signature.clone(),
                        return_type: sym.return_type.clone(),
                        parameters: sym.parameters.clone(),
                        documentation: sym.documentation.clone(),
                        visibility: sym.visibility,
                        range: sym.range,
                    },
                )
                .with_parent(&file_id);

                context.add_layer(sym_layer);
                sym_id
            })
            .collect();

        // Link file to symbols
        if let Some(file_layer) = context.get_layer_mut(&file_id) {
            for sym_id in &symbol_ids {
                file_layer.add_child(sym_id);
            }
        }

        // Link symbols as siblings
        for (i, sym_id) in symbol_ids.iter().enumerate() {
            if let Some(sym_layer) = context.get_layer_mut(sym_id) {
                for (j, other_id) in symbol_ids.iter().enumerate() {
                    if i != j {
                        sym_layer.add_sibling(other_id);
                    }
                }
            }
        }

        // Build relationships if configured
        if self.config.extract_relationships {
            self.build_relationships(&mut context, &symbols)?;
        }

        // Update metadata
        context.metadata.source_path = Some(path);
        context.metadata.language = Some(language.to_string());
        context.metadata.extractor_version = env!("CARGO_PKG_VERSION").to_string();

        Ok(context)
    }

    fn build_for_directory(&mut self, path: PathBuf) -> BuilderResult<FractalContext> {
        if !path.exists() {
            return Err(BuilderError::FileNotFound(path.clone()));
        }

        // Create project layer
        let project_id = self.next_id("proj");
        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        let project_layer = ContextLayer::new(
            &project_id,
            LayerContent::Project {
                name: project_name.clone(),
                description: None,
                root_path: Some(path.clone()),
                file_count: 0,
                dependencies: Vec::new(),
            },
        );

        let context_id = self.next_id("ctx");
        let mut context = FractalContext::new(&context_id, project_layer);

        // Walk directory and process files
        let mut file_count = 0;
        let mut file_ids = Vec::new();

        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                // Check extension filter
                if !self.config.extensions.is_empty() {
                    if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                        if !self.config.extensions.iter().any(|e| e == ext) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                // Check if we can detect the language
                if detect_language(&entry_path).is_none() {
                    continue;
                }

                // Check file size
                if let Ok(metadata) = fs::metadata(&entry_path) {
                    if metadata.len() > self.config.max_file_size {
                        continue;
                    }
                }

                // Build context for this file
                if let Ok(file_context) = self.build_for_file(entry_path) {
                    // Merge file context into project context
                    for (layer_id, layer) in file_context.layers {
                        if layer.level == ZoomLevel::File {
                            let mut layer = layer;
                            layer.parent_id = Some(project_id.clone());
                            file_ids.push(layer_id.clone());
                            context.layers.insert(layer_id, layer);
                        } else {
                            context.layers.insert(layer_id, layer);
                        }
                    }
                    file_count += 1;
                }
            }
        }

        // Update project layer with file count and children
        if let Some(proj_layer) = context.get_layer_mut(&project_id) {
            if let LayerContent::Project {
                file_count: ref mut fc,
                ..
            } = proj_layer.content
            {
                *fc = file_count;
            }
            for file_id in file_ids {
                proj_layer.add_child(file_id);
            }
        }

        // Update metadata
        context.metadata.source_path = Some(path);
        context.metadata.extractor_version = env!("CARGO_PKG_VERSION").to_string();

        Ok(context)
    }

    fn build_for_content(
        &mut self,
        content: String,
        language: String,
    ) -> BuilderResult<FractalContext> {
        let line_count = content.lines().count();

        // Extract symbols if configured
        let symbols = if self.config.depth.include_symbols() {
            extract_symbols_regex(&content, &language)
        } else {
            Vec::new()
        };

        // Extract imports
        let imports = extract_imports(&content, &language);

        // Create file layer (virtual file)
        let file_id = self.next_id("file");
        let file_layer = ContextLayer::new(
            &file_id,
            LayerContent::File {
                path: PathBuf::from("<content>"),
                language: language.clone(),
                size_bytes: content.len() as u64,
                line_count,
                symbol_count: symbols.len(),
                imports,
            },
        );

        let context_id = self.next_id("ctx");
        let mut context = FractalContext::new(&context_id, file_layer);

        // Add symbol layers
        let symbol_ids: Vec<String> = symbols
            .iter()
            .map(|sym| {
                let sym_id = self.next_id("sym");
                let sym_layer = ContextLayer::new(
                    &sym_id,
                    LayerContent::Symbol {
                        name: sym.name.clone(),
                        kind: sym.kind.clone(),
                        signature: sym.signature.clone(),
                        return_type: sym.return_type.clone(),
                        parameters: sym.parameters.clone(),
                        documentation: sym.documentation.clone(),
                        visibility: sym.visibility,
                        range: sym.range,
                    },
                )
                .with_parent(&file_id);

                context.add_layer(sym_layer);
                sym_id
            })
            .collect();

        // Link file to symbols
        if let Some(file_layer) = context.get_layer_mut(&file_id) {
            for sym_id in &symbol_ids {
                file_layer.add_child(sym_id);
            }
        }

        // Build relationships if configured
        if self.config.extract_relationships {
            self.build_relationships(&mut context, &symbols)?;
        }

        // Update metadata
        context.metadata.language = Some(language);
        context.metadata.extractor_version = env!("CARGO_PKG_VERSION").to_string();

        Ok(context)
    }

    fn build_relationships(
        &mut self,
        context: &mut FractalContext,
        symbols: &[ExtractedSymbol],
    ) -> BuilderResult<()> {
        // Add nodes for each symbol
        for sym in symbols {
            context.relationships.add_node(GraphNode {
                id: sym.name.clone(),
                label: sym.name.clone(),
                node_type: NodeType::Symbol,
                properties: HashMap::new(),
            });
        }

        // Add edges for calls (if we had call extraction - placeholder for now)
        // This would require more sophisticated parsing
        for sym in symbols {
            for called in &sym.calls {
                context.relationships.add_edge(GraphEdge {
                    source: sym.name.clone(),
                    target: called.clone(),
                    relationship: RelationshipType::Calls,
                    weight: 1.0,
                    properties: HashMap::new(),
                });
            }
        }

        Ok(())
    }
}

// =============================================================================
// Tests (TDD)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // =========================================================================
    // ExtractionDepth Tests
    // =========================================================================

    #[test]
    fn test_extraction_depth_minimal() {
        let depth = ExtractionDepth::Minimal;
        assert!(!depth.include_symbols());
        assert!(!depth.include_blocks());
        assert!(!depth.include_lines());
    }

    #[test]
    fn test_extraction_depth_standard() {
        let depth = ExtractionDepth::Standard;
        assert!(depth.include_symbols());
        assert!(!depth.include_blocks());
        assert!(!depth.include_lines());
    }

    #[test]
    fn test_extraction_depth_full() {
        let depth = ExtractionDepth::Full;
        assert!(depth.include_symbols());
        assert!(depth.include_blocks());
        assert!(depth.include_lines());
    }

    #[test]
    fn test_extraction_depth_custom() {
        let depth = ExtractionDepth::Custom {
            include_symbols: true,
            include_blocks: false,
            include_lines: true,
        };
        assert!(depth.include_symbols());
        assert!(!depth.include_blocks());
        assert!(depth.include_lines());
    }

    #[test]
    fn test_extraction_depth_default() {
        let depth = ExtractionDepth::default();
        assert_eq!(depth, ExtractionDepth::Standard);
    }

    // =========================================================================
    // BuilderConfig Tests
    // =========================================================================

    #[test]
    fn test_builder_config_default() {
        let config = BuilderConfig::default();
        assert_eq!(config.depth, ExtractionDepth::Standard);
        assert!(config.extract_relationships);
        assert!(!config.cluster_similar);
        assert_eq!(config.min_confidence, 0.5);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(config.extensions.is_empty());
    }

    // =========================================================================
    // Language Detection Tests
    // =========================================================================

    #[test]
    fn test_detect_language_rust() {
        assert_eq!(detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("lib.RS")), Some("rust"));
    }

    #[test]
    fn test_detect_language_python() {
        assert_eq!(detect_language(Path::new("script.py")), Some("python"));
    }

    #[test]
    fn test_detect_language_javascript() {
        assert_eq!(detect_language(Path::new("app.js")), Some("javascript"));
        assert_eq!(detect_language(Path::new("module.mjs")), Some("javascript"));
    }

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(detect_language(Path::new("app.ts")), Some("typescript"));
    }

    #[test]
    fn test_detect_language_shell() {
        assert_eq!(detect_language(Path::new("script.sh")), Some("shell"));
        assert_eq!(detect_language(Path::new("script.bash")), Some("shell"));
    }

    #[test]
    fn test_detect_language_unknown() {
        assert_eq!(detect_language(Path::new("file.xyz")), None);
        assert_eq!(detect_language(Path::new("noextension")), None);
    }

    // =========================================================================
    // Rust Symbol Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_rust_function() {
        let code = r#"
fn simple_function() {
    println!("Hello");
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "simple_function");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_extract_rust_pub_function() {
        let code = r#"
pub fn public_function(x: i32) -> bool {
    x > 0
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "public_function");
        assert_eq!(symbols[0].visibility, Visibility::Public);
        assert_eq!(symbols[0].return_type, Some("bool".to_string()));
        assert_eq!(symbols[0].parameters.len(), 1);
        assert_eq!(symbols[0].parameters[0].name, "x");
    }

    #[test]
    fn test_extract_rust_async_function() {
        let code = r#"
pub async fn async_handler() -> Result<()> {
    Ok(())
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].signature.contains("async"));
    }

    #[test]
    fn test_extract_rust_struct() {
        let code = r#"
pub struct MyStruct {
    field: String,
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyStruct");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn test_extract_rust_enum() {
        let code = r#"
pub enum Color {
    Red,
    Green,
    Blue,
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn test_extract_rust_trait() {
        let code = r#"
pub trait Drawable {
    fn draw(&self);
}
"#;
        let symbols = extract_rust_symbols(code);
        assert!(symbols.iter().any(|s| s.name == "Drawable" && s.kind == SymbolKind::Trait));
    }

    #[test]
    fn test_extract_rust_const() {
        let code = r#"
pub const MAX_SIZE: usize = 100;
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MAX_SIZE");
        assert_eq!(symbols[0].kind, SymbolKind::Constant);
    }

    #[test]
    fn test_extract_rust_test_function() {
        let code = r#"
#[test]
fn test_something() {
    assert!(true);
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "test_something");
        assert_eq!(symbols[0].kind, SymbolKind::Test);
    }

    #[test]
    fn test_extract_rust_with_doc_comment() {
        let code = r#"
/// This is a documented function.
/// It does something important.
pub fn documented_fn() {}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].documentation.is_some());
        assert!(symbols[0].documentation.as_ref().unwrap().contains("documented function"));
    }

    #[test]
    fn test_extract_rust_macro() {
        let code = r#"
macro_rules! my_macro {
    () => {};
}
"#;
        let symbols = extract_rust_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "my_macro");
        assert_eq!(symbols[0].kind, SymbolKind::Macro);
    }

    // =========================================================================
    // Python Symbol Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_python_function() {
        let code = r#"
def hello():
    print("Hello")
"#;
        let symbols = extract_python_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_python_async_function() {
        let code = r#"
async def fetch_data(url: str) -> dict:
    pass
"#;
        let symbols = extract_python_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].signature.contains("async"));
        assert_eq!(symbols[0].return_type, Some("dict".to_string()));
    }

    #[test]
    fn test_extract_python_class() {
        let code = r#"
class MyClass:
    pass
"#;
        let symbols = extract_python_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[0].kind, SymbolKind::Class);
    }

    #[test]
    fn test_extract_python_method() {
        let code = r#"
class MyClass:
    def method(self, x: int) -> str:
        return str(x)
"#;
        let symbols = extract_python_symbols(code);
        // Class + method
        assert!(symbols.iter().any(|s| s.name == "method" && s.kind == SymbolKind::Method));
    }

    #[test]
    fn test_extract_python_private() {
        let code = r#"
def _private_func():
    pass
"#;
        let symbols = extract_python_symbols(code);
        assert_eq!(symbols[0].visibility, Visibility::Private);
    }

    // =========================================================================
    // JavaScript/TypeScript Symbol Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_js_function() {
        let code = r#"
function hello() {
    console.log("Hello");
}
"#;
        let symbols = extract_js_ts_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_js_export_function() {
        let code = r#"
export function publicFunc(): void {
    return;
}
"#;
        let symbols = extract_js_ts_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_js_arrow_function() {
        let code = r#"
const myArrow = () => {
    return 42;
};
"#;
        let symbols = extract_js_ts_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "myArrow");
    }

    #[test]
    fn test_extract_js_class() {
        let code = r#"
class MyComponent {
    render() {}
}
"#;
        let symbols = extract_js_ts_symbols(code);
        assert!(symbols.iter().any(|s| s.name == "MyComponent" && s.kind == SymbolKind::Class));
    }

    #[test]
    fn test_extract_ts_interface() {
        let code = r#"
export interface Config {
    name: string;
}
"#;
        let symbols = extract_js_ts_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Config");
        assert_eq!(symbols[0].kind, SymbolKind::Interface);
    }

    // =========================================================================
    // Go Symbol Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_go_function() {
        let code = r#"
func main() {
    fmt.Println("Hello")
}
"#;
        let symbols = extract_go_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "main");
        assert_eq!(symbols[0].visibility, Visibility::Private); // lowercase = private
    }

    #[test]
    fn test_extract_go_exported_function() {
        let code = r#"
func PublicFunc(x int) string {
    return ""
}
"#;
        let symbols = extract_go_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].visibility, Visibility::Public); // uppercase = public
    }

    #[test]
    fn test_extract_go_struct() {
        let code = r#"
type Config struct {
    Name string
}
"#;
        let symbols = extract_go_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Config");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn test_extract_go_interface() {
        let code = r#"
type Reader interface {
    Read(p []byte) (n int, err error)
}
"#;
        let symbols = extract_go_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Reader");
        assert_eq!(symbols[0].kind, SymbolKind::Interface);
    }

    // =========================================================================
    // Shell Symbol Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_shell_function_parens() {
        let code = r#"
hello() {
    echo "Hello"
}
"#;
        let symbols = extract_shell_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
    }

    #[test]
    fn test_extract_shell_function_keyword() {
        let code = r#"
function greet {
    echo "Hi"
}
"#;
        let symbols = extract_shell_symbols(code);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
    }

    // =========================================================================
    // Import Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_rust_imports() {
        let code = r#"
use std::io;
use std::collections::HashMap;
"#;
        let imports = extract_rust_imports(code);
        assert_eq!(imports.len(), 2);
        assert!(imports.iter().any(|i| i.module == "std::io"));
        assert!(imports.iter().any(|i| i.module == "std::collections::HashMap"));
    }

    #[test]
    fn test_extract_python_imports() {
        let code = r#"
import os
from typing import List, Dict
"#;
        let imports = extract_python_imports(code);
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_extract_js_imports() {
        let code = r#"
import React from 'react';
import { useState, useEffect } from 'react';
"#;
        let imports = extract_js_imports(code);
        assert_eq!(imports.len(), 2);
    }

    // =========================================================================
    // FractalContextBuilder Tests
    // =========================================================================

    #[test]
    fn test_builder_for_content() {
        let code = r#"
fn main() {
    println!("Hello");
}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .with_depth(ExtractionDepth::Standard)
            .build()
            .unwrap();

        assert!(context.layer_count() >= 2); // file + function
        assert!(context.metadata.language.as_deref() == Some("rust"));
    }

    #[test]
    fn test_builder_minimal_depth() {
        let code = r#"
fn main() {}
fn helper() {}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .with_depth(ExtractionDepth::Minimal)
            .build()
            .unwrap();

        // Only file layer, no symbols
        assert_eq!(context.layer_count(), 1);
    }

    #[test]
    fn test_builder_standard_depth() {
        let code = r#"
pub fn main() {}
pub fn helper() {}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .with_depth(ExtractionDepth::Standard)
            .build()
            .unwrap();

        // File + 2 functions
        assert_eq!(context.layer_count(), 3);
    }

    #[test]
    fn test_builder_with_relationships() {
        let code = r#"
pub fn caller() {}
pub fn callee() {}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .with_relationships(true)
            .build()
            .unwrap();

        // Should have nodes for both functions
        assert_eq!(context.relationships.nodes.len(), 2);
    }

    #[test]
    fn test_builder_without_relationships() {
        let code = r#"
pub fn func() {}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .with_relationships(false)
            .build()
            .unwrap();

        assert!(context.relationships.nodes.is_empty());
    }

    #[test]
    fn test_builder_for_file() {
        // Create temp file
        let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(temp_file, "pub fn test_func() {{}}").unwrap();

        let context = FractalContextBuilder::for_file(temp_file.path())
            .build()
            .unwrap();

        assert!(context.layer_count() >= 2);
        assert!(context.metadata.source_path.is_some());
    }

    #[test]
    fn test_builder_file_not_found() {
        let result = FractalContextBuilder::for_file("/nonexistent/path.rs").build();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuilderError::FileNotFound(_)));
    }

    #[test]
    fn test_builder_hierarchy() {
        let code = r#"
pub fn func_a() {}
pub fn func_b() {}
pub struct MyStruct {}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .build()
            .unwrap();

        // Get file layer
        let file_layer = context.root().unwrap();
        assert_eq!(file_layer.level, ZoomLevel::File);

        // Check children are linked
        assert_eq!(file_layer.child_ids.len(), 3);

        // Get symbol layers
        let symbols = context.layers_at_level(ZoomLevel::Symbol);
        assert_eq!(symbols.len(), 3);

        // Check parent links
        for sym in symbols {
            assert!(sym.parent_id.is_some());
        }
    }

    #[test]
    fn test_builder_navigation() {
        let code = r#"
pub fn main() {}
"#;
        let mut context = FractalContextBuilder::for_content(code, "rust")
            .build()
            .unwrap();

        // Start at file level
        assert_eq!(context.current_view.level, ZoomLevel::File);

        // Zoom in to symbol
        assert!(context.zoom_in());
        assert_eq!(context.current_view.level, ZoomLevel::Symbol);

        // Zoom out back to file
        assert!(context.zoom_out());
        assert_eq!(context.current_view.level, ZoomLevel::File);
    }

    #[test]
    fn test_builder_extraction_time() {
        let code = "fn test() {}";
        let context = FractalContextBuilder::for_content(code, "rust")
            .build()
            .unwrap();

        // Extraction time should be recorded
        assert!(context.metadata.extraction_time.as_nanos() > 0);
    }

    #[test]
    fn test_builder_json_output() {
        let code = r#"
pub fn hello(name: &str) -> String {
    format!("Hello, {}", name)
}
"#;
        let context = FractalContextBuilder::for_content(code, "rust")
            .build()
            .unwrap();

        let json = serde_json::to_string_pretty(&context).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"level\": \"file\""));
        assert!(json.contains("\"name\": \"hello\""));
        assert!(json.contains("\"type\": \"symbol\""));

        // Verify roundtrip
        let deserialized: FractalContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.layer_count(), context.layer_count());
    }
}
