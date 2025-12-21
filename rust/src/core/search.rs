//! Symbol Resolution for Cross-File Navigation (Fractal Protocol v2)
//!
//! This module provides the ability to find symbol definitions (functions, classes, structs)
//! across a codebase without requiring the user to specify the exact file path.
//!
//! # Example
//! ```ignore
//! use pm_encoder::core::SymbolResolver;
//!
//! let resolver = SymbolResolver::new();
//! let location = resolver.find_function("apply_budget", "/path/to/project")?;
//! println!("Found at {}:{}-{}", location.path, location.start_line, location.end_line);
//! ```

use lazy_static::lazy_static;
use regex::Regex;
use std::path::Path;

use super::walker::{SmartWalker, SmartWalkConfig};

/// A resolved symbol location in the codebase
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    /// File path relative to project root
    pub path: String,
    /// Line number where the symbol starts (1-indexed)
    pub start_line: usize,
    /// Line number where the symbol ends (1-indexed, inclusive)
    pub end_line: usize,
    /// The symbol name
    pub name: String,
    /// Symbol type (function, class, struct, etc.)
    pub symbol_type: SymbolType,
    /// The signature or first line of the definition
    pub signature: String,
}

/// Type of symbol being resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Function,
    Class,
    Struct,
    Trait,
    Enum,
    Module,
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolType::Function => write!(f, "function"),
            SymbolType::Class => write!(f, "class"),
            SymbolType::Struct => write!(f, "struct"),
            SymbolType::Trait => write!(f, "trait"),
            SymbolType::Enum => write!(f, "enum"),
            SymbolType::Module => write!(f, "module"),
        }
    }
}

lazy_static! {
    // Rust patterns
    static ref RUST_FN: Regex = Regex::new(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)"
    ).unwrap();
    static ref RUST_STRUCT: Regex = Regex::new(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)"
    ).unwrap();
    static ref RUST_ENUM: Regex = Regex::new(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+(\w+)"
    ).unwrap();
    static ref RUST_TRAIT: Regex = Regex::new(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?trait\s+(\w+)"
    ).unwrap();
    static ref RUST_IMPL: Regex = Regex::new(
        r"^\s*impl(?:\s*<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)"
    ).unwrap();

    // Python patterns
    static ref PYTHON_DEF: Regex = Regex::new(
        r"^\s*(?:async\s+)?def\s+(\w+)"
    ).unwrap();
    static ref PYTHON_CLASS: Regex = Regex::new(
        r"^\s*class\s+(\w+)"
    ).unwrap();

    // JavaScript/TypeScript patterns
    static ref JS_FUNCTION: Regex = Regex::new(
        r"^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)"
    ).unwrap();
    static ref JS_CLASS: Regex = Regex::new(
        r"^\s*(?:export\s+)?class\s+(\w+)"
    ).unwrap();
    static ref JS_CONST_FN: Regex = Regex::new(
        r"^\s*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?(?:\([^)]*\)|[^=])\s*=>"
    ).unwrap();
    static ref JS_METHOD: Regex = Regex::new(
        r"^\s*(?:async\s+)?(\w+)\s*\([^)]*\)\s*\{"
    ).unwrap();

    // Go patterns
    static ref GO_FUNC: Regex = Regex::new(
        r"^\s*func\s+(?:\([^)]+\)\s+)?(\w+)"
    ).unwrap();
    static ref GO_TYPE: Regex = Regex::new(
        r"^\s*type\s+(\w+)\s+(?:struct|interface)"
    ).unwrap();
}

/// Symbol resolver for finding definitions across a codebase
pub struct SymbolResolver {
    ignore_patterns: Vec<String>,
    include_patterns: Vec<String>,
}

impl Default for SymbolResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolResolver {
    /// Create a new symbol resolver with default patterns
    pub fn new() -> Self {
        Self {
            ignore_patterns: vec![
                "*.pyc".to_string(),
                "__pycache__".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "*.min.js".to_string(),
            ],
            include_patterns: Vec::new(),
        }
    }

    /// Create with custom ignore patterns
    pub fn with_ignore(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    /// Create with include patterns
    pub fn with_include(mut self, patterns: Vec<String>) -> Self {
        self.include_patterns = patterns;
        self
    }

    /// Find a function definition by name
    pub fn find_function(&self, name: &str, root: &Path) -> Result<SymbolLocation, String> {
        self.find_symbol(name, SymbolType::Function, root)
    }

    /// Find a class/struct definition by name
    pub fn find_class(&self, name: &str, root: &Path) -> Result<SymbolLocation, String> {
        // Try struct first (Rust), then class (Python/JS)
        self.find_symbol(name, SymbolType::Struct, root)
            .or_else(|_| self.find_symbol(name, SymbolType::Class, root))
    }

    /// Find all matches for a symbol (for disambiguation)
    pub fn find_all(&self, name: &str, symbol_type: SymbolType, root: &Path) -> Vec<SymbolLocation> {
        let mut results = Vec::new();

        // Use SmartWalker to respect hygiene exclusions (.venv, node_modules, etc.)
        let config = SmartWalkConfig {
            max_file_size: 1_048_576, // 1MB
            ..Default::default()
        };

        let walker = SmartWalker::with_config(root, config);
        let entries = match walker.walk_as_file_entries() {
            Ok(e) => e,
            Err(_) => return results,
        };

        for entry in entries {
            if let Some(locations) = self.find_in_file(&entry.path, &entry.content, name, symbol_type) {
                results.extend(locations);
            }
        }

        results
    }

    /// Find a single symbol (returns first match or error)
    pub fn find_symbol(&self, name: &str, symbol_type: SymbolType, root: &Path) -> Result<SymbolLocation, String> {
        // Use SmartWalker to respect hygiene exclusions (.venv, node_modules, etc.)
        let config = SmartWalkConfig {
            max_file_size: 1_048_576,
            ..Default::default()
        };

        let walker = SmartWalker::with_config(root, config);
        let entries = walker.walk_as_file_entries()
            .map_err(|e| format!("Failed to walk directory: {}", e))?;

        for entry in entries {
            if let Some(locations) = self.find_in_file(&entry.path, &entry.content, name, symbol_type) {
                if let Some(loc) = locations.into_iter().next() {
                    return Ok(loc);
                }
            }
        }

        Err(format!(
            "{} '{}' not found in scanned files. Try checking the name or file patterns.",
            symbol_type, name
        ))
    }

    /// Find symbols in a single file
    fn find_in_file(&self, path: &str, content: &str, name: &str, symbol_type: SymbolType) -> Option<Vec<SymbolLocation>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut results = Vec::new();

        let ext = Path::new(path).extension()?.to_str()?;

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            if let Some(loc) = self.match_symbol(path, line, line_num, name, symbol_type, ext, &lines) {
                results.push(loc);
            }
        }

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Match a symbol on a specific line
    #[allow(clippy::too_many_arguments)]
    fn match_symbol(
        &self,
        path: &str,
        line: &str,
        line_num: usize,
        name: &str,
        symbol_type: SymbolType,
        ext: &str,
        all_lines: &[&str],
    ) -> Option<SymbolLocation> {
        let patterns: Vec<&Regex> = match (ext, symbol_type) {
            ("rs", SymbolType::Function) => vec![&RUST_FN],
            ("rs", SymbolType::Struct) => vec![&RUST_STRUCT],
            ("rs", SymbolType::Enum) => vec![&RUST_ENUM],
            ("rs", SymbolType::Trait) => vec![&RUST_TRAIT],
            ("rs", SymbolType::Class) => vec![&RUST_STRUCT, &RUST_ENUM], // Rust doesn't have classes

            ("py" | "pyw", SymbolType::Function) => vec![&PYTHON_DEF],
            ("py" | "pyw", SymbolType::Class) => vec![&PYTHON_CLASS],

            ("js" | "jsx" | "ts" | "tsx" | "mjs", SymbolType::Function) => {
                vec![&JS_FUNCTION, &JS_CONST_FN, &JS_METHOD]
            }
            ("js" | "jsx" | "ts" | "tsx" | "mjs", SymbolType::Class) => vec![&JS_CLASS],

            ("go", SymbolType::Function) => vec![&GO_FUNC],
            ("go", SymbolType::Class | SymbolType::Struct) => vec![&GO_TYPE],

            _ => return None,
        };

        for pattern in patterns {
            if let Some(caps) = pattern.captures(line) {
                // Get the captured name (group 1, or group 2 for some patterns)
                let captured_name = caps.get(1)
                    .or_else(|| caps.get(2))
                    .map(|m| m.as_str())?;

                if captured_name == name {
                    // Find the end of the symbol (simple heuristic: find closing brace at same indent)
                    let end_line = self.find_block_end(all_lines, line_num - 1, ext);

                    return Some(SymbolLocation {
                        path: path.to_string(),
                        start_line: line_num,
                        end_line,
                        name: name.to_string(),
                        symbol_type,
                        signature: line.trim().to_string(),
                    });
                }
            }
        }

        None
    }

    /// Find the end of a code block (heuristic based on brace/indent matching)
    fn find_block_end(&self, lines: &[&str], start_idx: usize, ext: &str) -> usize {
        if start_idx >= lines.len() {
            return start_idx + 1;
        }

        let start_line = lines[start_idx];
        let start_indent = start_line.len() - start_line.trim_start().len();

        match ext {
            // Brace-based languages
            "rs" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "go" | "c" | "cpp" | "java" => {
                let mut brace_count = 0;
                let mut found_open = false;

                for (i, line) in lines.iter().enumerate().skip(start_idx) {
                    for ch in line.chars() {
                        if ch == '{' {
                            brace_count += 1;
                            found_open = true;
                        } else if ch == '}' {
                            brace_count -= 1;
                            if found_open && brace_count == 0 {
                                return i + 1; // 1-indexed
                            }
                        }
                    }
                }
                // If no closing brace found, estimate ~50 lines
                (start_idx + 50).min(lines.len())
            }

            // Indent-based languages (Python)
            "py" | "pyw" => {
                for (i, line) in lines.iter().enumerate().skip(start_idx + 1) {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let indent = line.len() - line.trim_start().len();
                    if indent <= start_indent && !trimmed.starts_with('#') && !trimmed.starts_with('@') {
                        return i; // 1-indexed (previous line is end)
                    }
                }
                lines.len()
            }

            _ => (start_idx + 30).min(lines.len()),
        }
    }
}

// ============================================================================
// Call Graph Analysis (Fractal Protocol v2 - AI-Guided Zoom)
// ============================================================================

lazy_static! {
    // Function call patterns (language-agnostic)

    /// Rust/Go/C++ style: function_name(...) or module::function(...)
    static ref CALL_SIMPLE: Regex = Regex::new(
        r"\b([a-z_][a-z0-9_]*)\s*\("
    ).unwrap();

    /// Method call: object.method(...) or self.method(...)
    static ref CALL_METHOD: Regex = Regex::new(
        r"\.([a-z_][a-z0-9_]*)\s*\("
    ).unwrap();

    /// Rust path call: Module::function(...) or Type::method(...)
    static ref CALL_PATH: Regex = Regex::new(
        r"([A-Z][a-zA-Z0-9_]*)::\s*([a-z_][a-z0-9_]*)\s*\("
    ).unwrap();

    /// Python/JS: Class.method(...) or module.function(...)
    static ref CALL_DOT_PATH: Regex = Regex::new(
        r"([A-Z][a-zA-Z0-9_]*)\.([a-z_][a-z0-9_]*)\s*\("
    ).unwrap();
}

/// Keywords to ignore (not function calls)
const KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "match", "loop", "return", "break", "continue",
    "let", "const", "mut", "ref", "fn", "pub", "use", "mod", "impl", "trait",
    "struct", "enum", "type", "where", "async", "await", "move", "dyn", "box",
    // Python
    "def", "class", "import", "from", "as", "with", "try", "except", "finally",
    "raise", "yield", "lambda", "pass", "assert", "global", "nonlocal", "del",
    // JavaScript/TypeScript
    "function", "var", "new", "delete", "typeof", "instanceof", "void",
    "throw", "catch", "switch", "case", "default", "do", "in", "of",
    // Common stdlib functions to ignore
    "print", "println", "printf", "format", "write", "writeln",
    "len", "str", "int", "float", "bool", "list", "dict", "set", "tuple",
    "Some", "None", "Ok", "Err", "Vec", "Box", "Arc", "Rc", "String",
];

/// A potential function call found in source code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCall {
    /// The function/method name
    pub name: String,
    /// Optional qualifier (module, type, or object)
    pub qualifier: Option<String>,
    /// The full call expression as found
    pub full_expr: String,
}

impl FunctionCall {
    /// Get the simple name for symbol lookup
    pub fn lookup_name(&self) -> &str {
        &self.name
    }

    /// Format as a zoom target
    pub fn as_zoom_target(&self) -> String {
        format!("function={}", self.name)
    }
}

/// Analyzes source code to extract function calls for zoom suggestions
pub struct CallGraphAnalyzer {
    /// Maximum number of calls to return
    max_results: usize,
}

impl Default for CallGraphAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CallGraphAnalyzer {
    /// Create a new analyzer with default settings
    pub fn new() -> Self {
        Self { max_results: 10 }
    }

    /// Set maximum results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Extract function calls from source code
    pub fn extract_calls(&self, source: &str) -> Vec<FunctionCall> {
        let mut calls = std::collections::HashSet::new();

        for line in source.lines() {
            // Skip comments
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("*") {
                continue;
            }

            // Simple function calls: foo(...)
            for caps in CALL_SIMPLE.captures_iter(line) {
                if let Some(name) = caps.get(1) {
                    let name_str = name.as_str();
                    if !self.is_keyword(name_str) && !self.is_builtin(name_str) {
                        calls.insert(FunctionCall {
                            name: name_str.to_string(),
                            qualifier: None,
                            full_expr: name_str.to_string(),
                        });
                    }
                }
            }

            // Method calls: obj.method(...)
            for caps in CALL_METHOD.captures_iter(line) {
                if let Some(method) = caps.get(1) {
                    let method_str = method.as_str();
                    if !self.is_keyword(method_str) {
                        calls.insert(FunctionCall {
                            name: method_str.to_string(),
                            qualifier: Some("self".to_string()),
                            full_expr: format!(".{}", method_str),
                        });
                    }
                }
            }

            // Path calls: Module::function(...)
            for caps in CALL_PATH.captures_iter(line) {
                if let (Some(module), Some(func)) = (caps.get(1), caps.get(2)) {
                    let func_str = func.as_str();
                    let module_str = module.as_str();
                    if !self.is_keyword(func_str) && !self.is_builtin(module_str) {
                        calls.insert(FunctionCall {
                            name: func_str.to_string(),
                            qualifier: Some(module_str.to_string()),
                            full_expr: format!("{}::{}", module_str, func_str),
                        });
                    }
                }
            }
        }

        // Convert to vec and limit results
        let mut result: Vec<_> = calls.into_iter().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result.truncate(self.max_results);
        result
    }

    /// Extract calls and validate against known symbols in the codebase
    pub fn extract_validated_calls(
        &self,
        source: &str,
        resolver: &SymbolResolver,
        root: &Path,
    ) -> Vec<(FunctionCall, Option<SymbolLocation>)> {
        let calls = self.extract_calls(source);

        calls.into_iter()
            .map(|call| {
                // Try to resolve the function in the codebase
                let location = resolver.find_function(&call.name, root).ok();
                (call, location)
            })
            .collect()
    }

    /// Get only the calls that exist in the codebase
    pub fn get_valid_calls(
        &self,
        source: &str,
        resolver: &SymbolResolver,
        root: &Path,
    ) -> Vec<(FunctionCall, SymbolLocation)> {
        self.extract_validated_calls(source, resolver, root)
            .into_iter()
            .filter_map(|(call, loc)| loc.map(|l| (call, l)))
            .collect()
    }

    fn is_keyword(&self, name: &str) -> bool {
        KEYWORDS.contains(&name)
    }

    fn is_builtin(&self, name: &str) -> bool {
        // Check for common type constructors and builtins
        name.chars().next().is_some_and(|c| c.is_uppercase())
            || KEYWORDS.contains(&name)
    }
}

// ============================================================================
// Reverse Call Graph - Find Usages (Phase 2)
// ============================================================================

/// A location where a symbol is used (not defined)
#[derive(Debug, Clone)]
pub struct UsageLocation {
    /// File path relative to project root
    pub path: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// The code snippet containing the usage
    pub snippet: String,
    /// Column offset where the symbol starts (0-indexed)
    pub column: Option<usize>,
}

impl UsageLocation {
    /// Format as XML for rich zoom output
    pub fn to_xml(&self) -> String {
        format!(
            r#"<usage file="{}" line="{}">{}</usage>"#,
            self.path,
            self.line,
            escape_xml(&self.snippet)
        )
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Find usages of a symbol across the codebase (reverse call graph)
pub struct UsageFinder {
    /// Maximum number of usages to return
    max_results: usize,
    /// Ignore patterns for walking (reserved for future use)
    #[allow(dead_code)]
    ignore_patterns: Vec<String>,
}

impl Default for UsageFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageFinder {
    /// Create a new usage finder
    pub fn new() -> Self {
        Self {
            max_results: 10,
            ignore_patterns: vec![
                "*.pyc".to_string(),
                "__pycache__".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "*.min.js".to_string(),
            ],
        }
    }

    /// Set maximum results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Find all usages of a symbol in the codebase
    ///
    /// Excludes the definition itself by checking if the line matches
    /// a definition pattern (fn, def, class, etc.)
    pub fn find_usages(
        &self,
        symbol: &str,
        root: &Path,
        definition_path: Option<&str>,
        definition_line: Option<usize>,
    ) -> Vec<UsageLocation> {
        let config = SmartWalkConfig {
            max_file_size: 1_048_576,
            ..Default::default()
        };

        let walker = SmartWalker::with_config(root, config);
        let entries = match walker.walk_as_file_entries() {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let mut usages = Vec::new();

        // Build regex to find the symbol as a word (not substring)
        let pattern = format!(r"\b{}\b", regex::escape(symbol));
        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        for entry in entries {
            for (line_idx, line) in entry.content.lines().enumerate() {
                let line_num = line_idx + 1;

                // Skip if this is the definition line
                if let (Some(def_path), Some(def_line)) = (definition_path, definition_line) {
                    if entry.path == def_path && line_num == def_line {
                        continue;
                    }
                }

                // Skip if line looks like a definition (not a usage)
                if self.is_definition_line(line, symbol) {
                    continue;
                }

                // Check if symbol appears on this line
                if regex.is_match(line) {
                    // Find column offset
                    let column = regex.find(line).map(|m| m.start());

                    usages.push(UsageLocation {
                        path: entry.path.clone(),
                        line: line_num,
                        snippet: line.trim().to_string(),
                        column,
                    });

                    if usages.len() >= self.max_results {
                        return usages;
                    }
                }
            }
        }

        usages
    }

    /// Check if a line is a definition (not a usage)
    fn is_definition_line(&self, line: &str, symbol: &str) -> bool {
        let trimmed = line.trim();

        // Rust definitions
        if trimmed.contains(&format!("fn {}", symbol))
            || trimmed.contains(&format!("struct {}", symbol))
            || trimmed.contains(&format!("enum {}", symbol))
            || trimmed.contains(&format!("trait {}", symbol))
            || trimmed.contains(&format!("type {}", symbol))
            || trimmed.contains(&format!("mod {}", symbol))
        {
            return true;
        }

        // Python definitions
        if trimmed.contains(&format!("def {}(", symbol))
            || trimmed.contains(&format!("def {}:", symbol))
            || trimmed.contains(&format!("class {}(", symbol))
            || trimmed.contains(&format!("class {}:", symbol))
        {
            return true;
        }

        // JavaScript/TypeScript definitions
        if trimmed.contains(&format!("function {}", symbol))
            || trimmed.contains(&format!("const {} =", symbol))
            || trimmed.contains(&format!("let {} =", symbol))
            || trimmed.contains(&format!("var {} =", symbol))
            || trimmed.contains(&format!("class {} ", symbol))
        {
            return true;
        }

        // Go definitions
        if trimmed.contains(&format!("func {}", symbol))
            || trimmed.contains(&format!("type {} ", symbol))
        {
            return true;
        }

        false
    }
}

/// Related context for a zoomed symbol (callers, callees, etc.)
#[derive(Debug, Clone, Default)]
pub struct RelatedContext {
    /// Functions/methods that call this symbol
    pub callers: Vec<UsageLocation>,
    /// Functions/methods called by this symbol (if available)
    pub callees: Vec<ZoomSuggestion>,
}

impl RelatedContext {
    /// Create empty related context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add callers to the context
    pub fn with_callers(mut self, callers: Vec<UsageLocation>) -> Self {
        self.callers = callers;
        self
    }

    /// Add callees to the context
    pub fn with_callees(mut self, callees: Vec<ZoomSuggestion>) -> Self {
        self.callees = callees;
        self
    }

    /// Check if context is empty
    pub fn is_empty(&self) -> bool {
        self.callers.is_empty() && self.callees.is_empty()
    }

    /// Format as XML for Claude-XML output
    pub fn to_xml(&self) -> String {
        if self.is_empty() {
            return String::new();
        }

        let mut xml = String::from("<related_context>\n");

        if !self.callers.is_empty() {
            xml.push_str("  <callers>\n");
            for caller in &self.callers {
                xml.push_str("    ");
                xml.push_str(&caller.to_xml());
                xml.push('\n');
            }
            xml.push_str("  </callers>\n");
        }

        if !self.callees.is_empty() {
            xml.push_str("  <callees>\n");
            for callee in &self.callees {
                xml.push_str("    ");
                xml.push_str(&callee.to_xml());
                xml.push('\n');
            }
            xml.push_str("  </callees>\n");
        }

        xml.push_str("</related_context>");
        xml
    }
}

/// A zoom suggestion for the user/AI
#[derive(Debug, Clone)]
pub struct ZoomSuggestion {
    /// The target for --zoom
    pub target: String,
    /// Human-readable description
    pub description: String,
    /// File path where the target was found
    pub path: String,
    /// Line range
    pub lines: (usize, usize),
}

impl ZoomSuggestion {
    /// Create from a function call and its resolved location
    pub fn from_call(call: &FunctionCall, location: &SymbolLocation) -> Self {
        Self {
            target: call.as_zoom_target(),
            description: format!("Definition of {}", call.name),
            path: location.path.clone(),
            lines: (location.start_line, location.end_line),
        }
    }

    /// Format as XML for Claude-XML output
    pub fn to_xml(&self) -> String {
        format!(
            r#"<option target="{}" path="{}:{}-{}">{}</option>"#,
            self.target, self.path, self.lines.0, self.lines.1, self.description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_function_pattern() {
        let test_cases = vec![
            ("fn main() {", "main"),
            ("pub fn process() {", "process"),
            ("    pub async fn fetch_data() ->", "fetch_data"),
            ("pub(crate) fn internal() {", "internal"),
        ];

        for (line, expected) in test_cases {
            let caps = RUST_FN.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_python_function_pattern() {
        let test_cases = vec![
            ("def hello():", "hello"),
            ("    def process(self):", "process"),
            ("async def fetch():", "fetch"),
        ];

        for (line, expected) in test_cases {
            let caps = PYTHON_DEF.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_javascript_patterns() {
        let test_cases = vec![
            ("function hello() {", "hello", &*JS_FUNCTION),
            ("export async function fetch() {", "fetch", &*JS_FUNCTION),
            ("const handler = () => {", "handler", &*JS_CONST_FN),
            ("export const process = async () => {", "process", &*JS_CONST_FN),
        ];

        for (line, expected, pattern) in test_cases {
            let caps = pattern.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_find_block_end_rust() {
        let resolver = SymbolResolver::new();
        let lines = vec![
            "fn test() {",
            "    let x = 1;",
            "    if x > 0 {",
            "        println!(\"hi\");",
            "    }",
            "}",
            "",
            "fn other() {",
        ];

        let end = resolver.find_block_end(&lines, 0, "rs");
        assert_eq!(end, 6); // Line 6 (1-indexed)
    }

    #[test]
    fn test_find_block_end_python() {
        let resolver = SymbolResolver::new();
        let lines = vec![
            "def test():",
            "    x = 1",
            "    if x > 0:",
            "        print('hi')",
            "",
            "def other():",
        ];

        let end = resolver.find_block_end(&lines, 0, "py");
        assert_eq!(end, 5); // Ends at line 5 (before def other)
    }

    #[test]
    fn test_symbol_location_display() {
        let loc = SymbolLocation {
            path: "src/main.rs".to_string(),
            start_line: 10,
            end_line: 25,
            name: "main".to_string(),
            symbol_type: SymbolType::Function,
            signature: "fn main() {".to_string(),
        };

        assert_eq!(loc.symbol_type.to_string(), "function");
        assert_eq!(loc.name, "main");
    }

    // ========================================================================
    // Call Graph Analyzer Tests
    // ========================================================================

    #[test]
    fn test_extract_simple_calls() {
        let analyzer = CallGraphAnalyzer::new();
        let source = r#"
            fn main() {
                init_logger();
                let config = parse_args();
                process_data(config);
            }
        "#;

        let calls = analyzer.extract_calls(source);
        let names: Vec<_> = calls.iter().map(|c| c.name.as_str()).collect();

        assert!(names.contains(&"init_logger"));
        assert!(names.contains(&"parse_args"));
        assert!(names.contains(&"process_data"));
    }

    #[test]
    fn test_extract_method_calls() {
        let analyzer = CallGraphAnalyzer::new();
        let source = r#"
            fn process() {
                self.validate();
                data.transform();
                result.save();
            }
        "#;

        let calls = analyzer.extract_calls(source);
        let names: Vec<_> = calls.iter().map(|c| c.name.as_str()).collect();

        assert!(names.contains(&"validate"));
        assert!(names.contains(&"transform"));
        assert!(names.contains(&"save"));
    }

    #[test]
    fn test_extract_path_calls() {
        let analyzer = CallGraphAnalyzer::new();
        let source = r#"
            fn main() {
                Config::load();
                Engine::create();
                Walker::walk();
            }
        "#;

        let calls = analyzer.extract_calls(source);
        let names: Vec<_> = calls.iter().map(|c| c.name.as_str()).collect();

        assert!(names.contains(&"load"));
        assert!(names.contains(&"create")); // "new" is a JS keyword, use "create"
        assert!(names.contains(&"walk"));
    }

    #[test]
    fn test_ignores_keywords() {
        let analyzer = CallGraphAnalyzer::new();
        let source = r#"
            fn test() {
                if (condition) { }
                for item in items { }
                while (running) { }
                match value { }
            }
        "#;

        let calls = analyzer.extract_calls(source);

        // Should not include keywords
        assert!(!calls.iter().any(|c| c.name == "if"));
        assert!(!calls.iter().any(|c| c.name == "for"));
        assert!(!calls.iter().any(|c| c.name == "while"));
        assert!(!calls.iter().any(|c| c.name == "match"));
    }

    #[test]
    fn test_ignores_comments() {
        let analyzer = CallGraphAnalyzer::new();
        let source = r#"
            fn test() {
                // commented_out();
                # python_comment()
                actual_call();
            }
        "#;

        let calls = analyzer.extract_calls(source);
        let names: Vec<_> = calls.iter().map(|c| c.name.as_str()).collect();

        assert!(!names.contains(&"commented_out"));
        assert!(!names.contains(&"python_comment"));
        assert!(names.contains(&"actual_call"));
    }

    #[test]
    fn test_max_results_limit() {
        let analyzer = CallGraphAnalyzer::new().with_max_results(3);
        let source = r#"
            fn test() {
                call_a();
                call_b();
                call_c();
                call_d();
                call_e();
            }
        "#;

        let calls = analyzer.extract_calls(source);
        assert_eq!(calls.len(), 3);
    }

    #[test]
    fn test_zoom_suggestion_xml() {
        let suggestion = ZoomSuggestion {
            target: "function=init_logger".to_string(),
            description: "Definition of init_logger".to_string(),
            path: "src/logging.rs".to_string(),
            lines: (10, 25),
        };

        let xml = suggestion.to_xml();
        assert!(xml.contains("target=\"function=init_logger\""));
        assert!(xml.contains("path=\"src/logging.rs:10-25\""));
        assert!(xml.contains("Definition of init_logger"));
    }

    // ========================================================================
    // Phase 2: Find Usages Tests
    // ========================================================================

    #[test]
    fn test_usage_location_xml() {
        let usage = UsageLocation {
            path: "src/main.rs".to_string(),
            line: 45,
            snippet: "let res = process_request(data);".to_string(),
            column: Some(10),
        };

        let xml = usage.to_xml();
        assert!(xml.contains("file=\"src/main.rs\""));
        assert!(xml.contains("line=\"45\""));
        assert!(xml.contains("let res = process_request(data);"));
    }

    #[test]
    fn test_usage_location_xml_escapes_special_chars() {
        let usage = UsageLocation {
            path: "src/lib.rs".to_string(),
            line: 10,
            snippet: "if x < y && y > 0 { func() }".to_string(),
            column: None,
        };

        let xml = usage.to_xml();
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
        assert!(xml.contains("&amp;"));
    }

    #[test]
    fn test_related_context_empty() {
        let ctx = RelatedContext::new();
        assert!(ctx.is_empty());
        assert_eq!(ctx.to_xml(), "");
    }

    #[test]
    fn test_related_context_with_callers() {
        let callers = vec![
            UsageLocation {
                path: "src/main.rs".to_string(),
                line: 45,
                snippet: "process_request(data)".to_string(),
                column: None,
            },
            UsageLocation {
                path: "src/api.rs".to_string(),
                line: 120,
                snippet: "return process_request(req);".to_string(),
                column: None,
            },
        ];

        let ctx = RelatedContext::new().with_callers(callers);
        assert!(!ctx.is_empty());

        let xml = ctx.to_xml();
        assert!(xml.contains("<related_context>"));
        assert!(xml.contains("<callers>"));
        assert!(xml.contains("<usage file=\"src/main.rs\""));
        assert!(xml.contains("<usage file=\"src/api.rs\""));
        assert!(xml.contains("</callers>"));
        assert!(xml.contains("</related_context>"));
    }

    #[test]
    fn test_related_context_with_callees() {
        let callees = vec![ZoomSuggestion {
            target: "function=helper".to_string(),
            description: "Definition of helper".to_string(),
            path: "src/utils.rs".to_string(),
            lines: (5, 15),
        }];

        let ctx = RelatedContext::new().with_callees(callees);
        let xml = ctx.to_xml();

        assert!(xml.contains("<callees>"));
        assert!(xml.contains("function=helper"));
        assert!(xml.contains("</callees>"));
    }

    #[test]
    fn test_usage_finder_is_definition_line() {
        let finder = UsageFinder::new();

        // Rust definitions
        assert!(finder.is_definition_line("fn process_data() {", "process_data"));
        assert!(finder.is_definition_line("pub fn process_data() {", "process_data"));
        assert!(finder.is_definition_line("struct Config {", "Config"));

        // Python definitions
        assert!(finder.is_definition_line("def process_data():", "process_data"));
        assert!(finder.is_definition_line("class Config:", "Config"));

        // JavaScript definitions
        assert!(finder.is_definition_line("function processData() {", "processData"));
        assert!(finder.is_definition_line("const processData = () => {", "processData"));

        // Not definitions (usages)
        assert!(!finder.is_definition_line("let x = process_data();", "process_data"));
        assert!(!finder.is_definition_line("result = process_data()", "process_data"));
    }

    #[test]
    fn test_usage_finder_default() {
        let finder = UsageFinder::new();
        assert_eq!(finder.max_results, 10);
    }

    #[test]
    fn test_usage_finder_with_max_results() {
        let finder = UsageFinder::new().with_max_results(5);
        assert_eq!(finder.max_results, 5);
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a < b"), "a &lt; b");
        assert_eq!(escape_xml("a > b"), "a &gt; b");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("a \"b\""), "a &quot;b&quot;");
        assert_eq!(escape_xml("hello"), "hello");
    }

    #[test]
    fn test_symbol_resolver_excludes_venv() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a function in src/ (should be found)
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.py"),
            "def target_function():\n    pass\n",
        ).unwrap();

        // Create the same function in .venv/ (should be ignored)
        fs::create_dir_all(root.join(".venv/lib/python3.12/site-packages")).unwrap();
        fs::write(
            root.join(".venv/lib/python3.12/site-packages/lib.py"),
            "def target_function():\n    pass\n",
        ).unwrap();

        // Also create in node_modules/ (should be ignored)
        fs::create_dir_all(root.join("node_modules/some-package")).unwrap();
        fs::write(
            root.join("node_modules/some-package/index.js"),
            "function target_function() {}\n",
        ).unwrap();

        let resolver = SymbolResolver::new();
        let result = resolver.find_function("target_function", root);

        // Should find the function
        assert!(result.is_ok(), "Should find target_function");

        let location = result.unwrap();
        // Should be from src/, not .venv/ or node_modules/
        assert!(
            location.path.contains("src/"),
            "Found function should be in src/, not {:?}",
            location.path
        );
        assert!(
            !location.path.contains(".venv"),
            "Should not find in .venv, got {:?}",
            location.path
        );
        assert!(
            !location.path.contains("node_modules"),
            "Should not find in node_modules, got {:?}",
            location.path
        );
    }

    #[test]
    fn test_usage_finder_excludes_venv() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a function definition in src/
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.py"),
            "def helper():\n    pass\n\ndef caller():\n    helper()\n",
        ).unwrap();

        // Create a usage in .venv/ (should be ignored)
        fs::create_dir_all(root.join(".venv/lib")).unwrap();
        fs::write(
            root.join(".venv/lib/module.py"),
            "from lib import helper\nhelper()\n",
        ).unwrap();

        let finder = UsageFinder::new();
        let usages = finder.find_usages("helper", root, Some("src/lib.py"), Some(1));

        // Should find usage in src/lib.py (caller function)
        assert!(!usages.is_empty(), "Should find at least one usage");

        // None of the usages should be from .venv/
        for usage in &usages {
            assert!(
                !usage.path.contains(".venv"),
                "Should not find usage in .venv, got {:?}",
                usage.path
            );
        }
    }

    // ========================================================================
    // Additional Coverage Tests
    // ========================================================================

    #[test]
    fn test_go_function_pattern() {
        let test_cases = vec![
            ("func main() {", "main"),
            ("func (s *Server) Handle() {", "Handle"),
            ("func processData(x int) int {", "processData"),
        ];

        for (line, expected) in test_cases {
            let caps = GO_FUNC.captures(line);
            assert!(caps.is_some(), "Failed to match Go func: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_go_type_pattern() {
        let test_cases = vec![
            ("type Config struct {", "Config"),
            ("type Handler interface {", "Handler"),
            ("type MyService struct {", "MyService"),
        ];

        for (line, expected) in test_cases {
            let caps = GO_TYPE.captures(line);
            assert!(caps.is_some(), "Failed to match Go type: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_symbol_type_display_all_variants() {
        assert_eq!(SymbolType::Function.to_string(), "function");
        assert_eq!(SymbolType::Class.to_string(), "class");
        assert_eq!(SymbolType::Struct.to_string(), "struct");
        assert_eq!(SymbolType::Trait.to_string(), "trait");
        assert_eq!(SymbolType::Enum.to_string(), "enum");
        assert_eq!(SymbolType::Module.to_string(), "module");
    }

    #[test]
    fn test_symbol_resolver_with_ignore() {
        let resolver = SymbolResolver::new()
            .with_ignore(vec!["*.test".to_string()]);
        assert_eq!(resolver.ignore_patterns, vec!["*.test".to_string()]);
    }

    #[test]
    fn test_symbol_resolver_with_include() {
        let resolver = SymbolResolver::new()
            .with_include(vec!["src/**".to_string()]);
        assert_eq!(resolver.include_patterns, vec!["src/**".to_string()]);
    }

    #[test]
    fn test_symbol_resolver_default() {
        let resolver = SymbolResolver::default();
        assert!(!resolver.ignore_patterns.is_empty());
    }

    #[test]
    fn test_find_symbol_not_found() {
        use tempfile::TempDir;
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a file without the target function
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "fn other() {}").unwrap();

        let resolver = SymbolResolver::new();
        let result = resolver.find_function("nonexistent_function", root);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_find_class_tries_struct_first() {
        use tempfile::TempDir;
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a Rust struct
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub struct Config {\n    field: i32,\n}\n").unwrap();

        let resolver = SymbolResolver::new();
        let result = resolver.find_class("Config", root);

        assert!(result.is_ok());
        let loc = result.unwrap();
        assert_eq!(loc.name, "Config");
        assert_eq!(loc.symbol_type, SymbolType::Struct);
    }

    #[test]
    fn test_find_all_multiple_matches() {
        use tempfile::TempDir;
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create multiple files with the same function name
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.rs"), "fn helper() {}\n").unwrap();
        fs::write(root.join("src/b.rs"), "fn helper() {}\n").unwrap();

        let resolver = SymbolResolver::new();
        let results = resolver.find_all("helper", SymbolType::Function, root);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_find_block_end_unknown_extension() {
        let resolver = SymbolResolver::new();
        let lines = vec![
            "something {",
            "  content",
            "}",
        ];

        // Unknown extension should return start + 30 (capped at lines.len)
        let end = resolver.find_block_end(&lines, 0, "xyz");
        assert_eq!(end, 3); // min(0 + 30, 3) = 3
    }

    #[test]
    fn test_find_block_end_start_beyond_lines() {
        let resolver = SymbolResolver::new();
        let lines = vec!["fn test() {}"];

        let end = resolver.find_block_end(&lines, 10, "rs");
        assert_eq!(end, 11); // start_idx + 1
    }

    #[test]
    fn test_find_block_end_no_closing_brace() {
        let resolver = SymbolResolver::new();
        let lines = vec![
            "fn test() {",
            "    let x = 1;",
            "    // no closing brace",
        ];

        let end = resolver.find_block_end(&lines, 0, "rs");
        // Should return start + 50 capped at lines.len = 3
        assert_eq!(end, 3);
    }

    #[test]
    fn test_is_definition_line_go() {
        let finder = UsageFinder::new();

        // Go definitions
        assert!(finder.is_definition_line("func processData() {", "processData"));
        assert!(finder.is_definition_line("type Config struct {", "Config"));

        // Not definitions
        assert!(!finder.is_definition_line("result := processData()", "processData"));
    }

    #[test]
    fn test_is_definition_line_rust_more() {
        let finder = UsageFinder::new();

        // Additional Rust definitions
        assert!(finder.is_definition_line("enum Status {", "Status"));
        assert!(finder.is_definition_line("trait Handler {", "Handler"));
        assert!(finder.is_definition_line("type Alias = String;", "Alias"));
        assert!(finder.is_definition_line("mod utils {", "utils"));
    }

    #[test]
    fn test_function_call_helpers() {
        let call = FunctionCall {
            name: "process".to_string(),
            qualifier: Some("Config".to_string()),
            full_expr: "Config::process".to_string(),
        };

        assert_eq!(call.lookup_name(), "process");
        assert_eq!(call.as_zoom_target(), "function=process");
    }

    #[test]
    fn test_call_graph_analyzer_default() {
        let analyzer = CallGraphAnalyzer::default();
        assert_eq!(analyzer.max_results, 10);
    }

    #[test]
    fn test_usage_finder_default_impl() {
        let finder = UsageFinder::default();
        assert_eq!(finder.max_results, 10);
    }

    #[test]
    fn test_find_usages_empty_on_walk_error() {
        use std::path::Path;

        let finder = UsageFinder::new();
        // Non-existent path should return empty
        let usages = finder.find_usages("test", Path::new("/nonexistent/path/xyz"), None, None);
        assert!(usages.is_empty());
    }

    #[test]
    fn test_find_usages_skips_definition_line() {
        use tempfile::TempDir;
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "fn target() {\n    println!(\"hi\");\n}\n\nfn caller() {\n    target();\n}\n",
        ).unwrap();

        let finder = UsageFinder::new();
        let usages = finder.find_usages("target", root, Some("src/lib.rs"), Some(1));

        // Should find usage in caller(), but not the definition
        assert!(!usages.is_empty());
        for u in &usages {
            assert!(!u.snippet.contains("fn target"), "Should skip definition line");
        }
    }

    #[test]
    fn test_rust_struct_pattern() {
        let test_cases = vec![
            ("struct Config {", "Config"),
            ("pub struct Handler {", "Handler"),
            ("pub(crate) struct Internal {", "Internal"),
        ];

        for (line, expected) in test_cases {
            let caps = RUST_STRUCT.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_rust_enum_pattern() {
        let test_cases = vec![
            ("enum Status {", "Status"),
            ("pub enum Result {", "Result"),
        ];

        for (line, expected) in test_cases {
            let caps = RUST_ENUM.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_rust_trait_pattern() {
        let test_cases = vec![
            ("trait Handler {", "Handler"),
            ("pub trait Service {", "Service"),
        ];

        for (line, expected) in test_cases {
            let caps = RUST_TRAIT.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_python_class_pattern() {
        let test_cases = vec![
            ("class Config:", "Config"),
            ("class Handler(Base):", "Handler"),
        ];

        for (line, expected) in test_cases {
            let caps = PYTHON_CLASS.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_js_class_pattern() {
        let test_cases = vec![
            ("class Config {", "Config"),
            ("export class Handler {", "Handler"),
        ];

        for (line, expected) in test_cases {
            let caps = JS_CLASS.captures(line);
            assert!(caps.is_some(), "Failed to match: {}", line);
            assert_eq!(caps.unwrap().get(1).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn test_zoom_suggestion_from_call() {
        let call = FunctionCall {
            name: "process".to_string(),
            qualifier: None,
            full_expr: "process".to_string(),
        };
        let location = SymbolLocation {
            path: "src/lib.rs".to_string(),
            start_line: 10,
            end_line: 20,
            name: "process".to_string(),
            symbol_type: SymbolType::Function,
            signature: "fn process()".to_string(),
        };

        let suggestion = ZoomSuggestion::from_call(&call, &location);
        assert_eq!(suggestion.target, "function=process");
        assert_eq!(suggestion.path, "src/lib.rs");
        assert_eq!(suggestion.lines, (10, 20));
    }
}
