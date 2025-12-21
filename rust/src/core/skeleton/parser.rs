//! Skeletonizer - Regex-based code skeleton extraction
//!
//! Extracts signatures, imports, and type definitions while stripping
//! function/method bodies.

use lazy_static::lazy_static;
use regex::Regex;

use super::types::{Language, SkeletonResult};

lazy_static! {
    // Rust patterns
    static ref RUST_FN: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?(async\s+)?fn\s+(\w+)"
    ).unwrap();
    static ref RUST_STRUCT: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?struct\s+(\w+)"
    ).unwrap();
    static ref RUST_ENUM: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?enum\s+(\w+)"
    ).unwrap();
    static ref RUST_TRAIT: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?trait\s+(\w+)"
    ).unwrap();
    static ref RUST_IMPL: Regex = Regex::new(
        r"^\s*impl\s*(?:<[^>]*>)?\s*(?:(\w+)\s+for\s+)?(\w+)"
    ).unwrap();
    static ref RUST_TYPE: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?type\s+(\w+)"
    ).unwrap();
    static ref RUST_CONST: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?(const|static)\s+(\w+)"
    ).unwrap();
    static ref RUST_MOD: Regex = Regex::new(
        r"^\s*(pub(?:\([^)]*\))?\s+)?mod\s+(\w+)"
    ).unwrap();
    static ref RUST_USE: Regex = Regex::new(r"^\s*use\s+").unwrap();
    static ref RUST_DERIVE: Regex = Regex::new(r"^\s*#\[derive").unwrap();
    static ref RUST_ATTRIBUTE: Regex = Regex::new(r"^\s*#\[").unwrap();
    static ref RUST_DOC_COMMENT: Regex = Regex::new(r"^\s*(///|//!)").unwrap();

    // Python patterns
    static ref PYTHON_DEF: Regex = Regex::new(
        r"^\s*(async\s+)?def\s+(\w+)"
    ).unwrap();
    static ref PYTHON_CLASS: Regex = Regex::new(
        r"^\s*class\s+(\w+)"
    ).unwrap();
    static ref PYTHON_IMPORT: Regex = Regex::new(
        r"^\s*(import\s+|from\s+\S+\s+import)"
    ).unwrap();
    static ref PYTHON_DOCSTRING_START: Regex = Regex::new(
        r#"^\s*("""|''')"#
    ).unwrap();

    // TypeScript/JavaScript patterns
    static ref JS_FUNCTION: Regex = Regex::new(
        r"^\s*(export\s+)?(async\s+)?function\s+(\w+)"
    ).unwrap();
    static ref JS_CLASS: Regex = Regex::new(
        r"^\s*(export\s+)?class\s+(\w+)"
    ).unwrap();
    static ref JS_CONST_FN: Regex = Regex::new(
        r"^\s*(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?(\([^)]*\)|[^=])\s*=>"
    ).unwrap();
    static ref JS_IMPORT: Regex = Regex::new(
        r"^\s*import\s+"
    ).unwrap();
    static ref JS_INTERFACE: Regex = Regex::new(
        r"^\s*(export\s+)?interface\s+(\w+)"
    ).unwrap();
    static ref JS_TYPE: Regex = Regex::new(
        r"^\s*(export\s+)?type\s+(\w+)"
    ).unwrap();

    // Go patterns
    static ref GO_FUNC: Regex = Regex::new(
        r"^\s*func\s+(?:\([^)]+\)\s+)?(\w+)"
    ).unwrap();
    static ref GO_TYPE: Regex = Regex::new(
        r"^\s*type\s+(\w+)\s+(struct|interface)"
    ).unwrap();
    static ref GO_IMPORT: Regex = Regex::new(
        r"^\s*import\s+"
    ).unwrap();
    static ref GO_PACKAGE: Regex = Regex::new(
        r"^\s*package\s+(\w+)"
    ).unwrap();
    static ref GO_CONST: Regex = Regex::new(
        r"^\s*(const|var)\s+"
    ).unwrap();
}

/// Skeletonizer extracts code signatures while stripping implementation bodies
pub struct Skeletonizer {
    /// Whether to preserve docstrings (L1 mode)
    preserve_docstrings: bool,
    /// Fallback line count when parsing fails
    fallback_lines: usize,
}

impl Default for Skeletonizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Skeletonizer {
    /// Create a new Skeletonizer with default settings
    pub fn new() -> Self {
        Self {
            preserve_docstrings: true,
            fallback_lines: 50,
        }
    }

    /// Set whether to preserve docstrings
    pub fn with_docstrings(mut self, preserve: bool) -> Self {
        self.preserve_docstrings = preserve;
        self
    }

    /// Skeletonize content for a given language
    pub fn skeletonize(&self, content: &str, lang: Language) -> SkeletonResult {
        if content.is_empty() {
            return SkeletonResult::default();
        }

        let original_tokens = estimate_tokens(content);

        let (skeleton_content, symbols) = match lang {
            Language::Rust => self.skeletonize_rust(content),
            Language::Python => self.skeletonize_python(content),
            Language::TypeScript | Language::JavaScript => self.skeletonize_js(content),
            Language::Go => self.skeletonize_go(content),
        };

        let skeleton_tokens = estimate_tokens(&skeleton_content);

        SkeletonResult::new(skeleton_content, original_tokens, skeleton_tokens, symbols)
    }

    /// Skeletonize Rust code
    fn skeletonize_rust(&self, content: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut result: Vec<String> = Vec::new();
        let mut symbols = Vec::new();
        let mut brace_depth: i32 = 0;
        let mut in_struct_body = false;
        let mut pending_attrs: Vec<String> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Count braces on this line
            let open_braces = line.matches('{').count() as i32;
            let close_braces = line.matches('}').count() as i32;

            // Handle doc comments
            if RUST_DOC_COMMENT.is_match(trimmed) && self.preserve_docstrings {
                if brace_depth == 0 || in_struct_body {
                    result.push(line.to_string());
                }
                i += 1;
                continue;
            }

            // Handle attributes
            if RUST_ATTRIBUTE.is_match(trimmed) {
                if brace_depth == 0 {
                    pending_attrs.push(line.to_string());
                }
                i += 1;
                continue;
            }

            // At top level (depth 0)
            if brace_depth == 0 {
                // Use statements
                if RUST_USE.is_match(trimmed) {
                    result.push(line.to_string());
                    i += 1;
                    continue;
                }

                // Module declarations
                if let Some(caps) = RUST_MOD.captures(trimmed) {
                    result.append(&mut pending_attrs);
                    result.push(line.to_string());
                    if let Some(name) = caps.get(2) {
                        symbols.push(name.as_str().to_string());
                    }
                    i += 1;
                    continue;
                }

                // Constants and statics
                if RUST_CONST.is_match(trimmed) {
                    result.append(&mut pending_attrs);
                    result.push(line.to_string());
                    if let Some(caps) = RUST_CONST.captures(trimmed) {
                        if let Some(name) = caps.get(3) {
                            symbols.push(name.as_str().to_string());
                        }
                    }
                    i += 1;
                    continue;
                }

                // Type aliases
                if RUST_TYPE.is_match(trimmed) {
                    result.append(&mut pending_attrs);
                    result.push(line.to_string());
                    if let Some(caps) = RUST_TYPE.captures(trimmed) {
                        if let Some(name) = caps.get(2) {
                            symbols.push(name.as_str().to_string());
                        }
                    }
                    i += 1;
                    continue;
                }

                // Struct/Enum/Trait definitions
                if RUST_STRUCT.is_match(trimmed) || RUST_ENUM.is_match(trimmed) || RUST_TRAIT.is_match(trimmed) {
                    result.append(&mut pending_attrs);

                    // Extract symbol name
                    if let Some(caps) = RUST_STRUCT.captures(trimmed) {
                        if let Some(name) = caps.get(2) {
                            symbols.push(name.as_str().to_string());
                        }
                    } else if let Some(caps) = RUST_ENUM.captures(trimmed) {
                        if let Some(name) = caps.get(2) {
                            symbols.push(name.as_str().to_string());
                        }
                    } else if let Some(caps) = RUST_TRAIT.captures(trimmed) {
                        if let Some(name) = caps.get(2) {
                            symbols.push(name.as_str().to_string());
                        }
                    }

                    // Include struct body (fields are part of signature)
                    in_struct_body = true;
                    result.push(line.to_string());
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Impl blocks
                if RUST_IMPL.is_match(trimmed) {
                    result.append(&mut pending_attrs);
                    result.push(line.to_string());
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Function definitions
                if let Some(caps) = RUST_FN.captures(trimmed) {
                    result.append(&mut pending_attrs);

                    if let Some(name) = caps.get(3) {
                        symbols.push(name.as_str().to_string());
                    }

                    // Find the complete signature (may span multiple lines)
                    let sig_line = self.extract_rust_signature(&lines, i);
                    result.push(sig_line);

                    // Skip the body
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Pending attrs didn't match anything, discard
                pending_attrs.clear();
            }

            // Inside a block
            if brace_depth > 0 {
                if in_struct_body {
                    // Keep struct field definitions
                    result.push(line.to_string());
                } else {
                    // In impl block - check for method definitions
                    if let Some(caps) = RUST_FN.captures(trimmed) {
                        if let Some(name) = caps.get(3) {
                            symbols.push(name.as_str().to_string());
                        }

                        // Extract just the signature
                        let sig_line = self.extract_rust_signature(&lines, i);
                        result.push(sig_line);
                    }
                }
            }

            // Update brace depth
            brace_depth += open_braces - close_braces;

            // Check if we exited struct body
            if brace_depth == 0 && in_struct_body {
                in_struct_body = false;
            }

            // Fallback: negative brace depth means parsing error
            if brace_depth < 0 {
                return self.fallback(content);
            }

            i += 1;
        }

        (result.join("\n"), symbols)
    }

    /// Extract a complete Rust function signature (handles multi-line)
    fn extract_rust_signature(&self, lines: &[&str], start: usize) -> String {
        let mut sig = String::new();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i];
            sig.push_str(line);

            if line.contains('{') {
                // Truncate at the brace and add placeholder
                if let Some(pos) = sig.rfind('{') {
                    sig.truncate(pos);
                    sig.push_str("{ /* ... */ }");
                }
                break;
            }

            sig.push('\n');
            i += 1;
        }

        sig
    }

    /// Skeletonize Python code
    fn skeletonize_python(&self, content: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut result: Vec<String> = Vec::new();
        let mut symbols = Vec::new();
        // Stack of class indent levels to handle nested classes
        let mut class_indent_stack: Vec<usize> = Vec::new();
        let mut in_docstring = false;
        let mut pending_docstring: Vec<String> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            let indent = line.len() - line.trim_start().len();

            // Handle docstrings
            if in_docstring {
                if self.preserve_docstrings {
                    pending_docstring.push(line.to_string());
                }
                if PYTHON_DOCSTRING_START.is_match(trimmed) && trimmed.len() > 3 {
                    // Single-line docstring or end of multi-line
                    in_docstring = false;
                    if self.preserve_docstrings {
                        result.append(&mut pending_docstring);
                    }
                } else if trimmed.ends_with("\"\"\"") || trimmed.ends_with("'''") {
                    in_docstring = false;
                    if self.preserve_docstrings {
                        result.append(&mut pending_docstring);
                    }
                }
                i += 1;
                continue;
            }

            // Check for docstring start
            if PYTHON_DOCSTRING_START.is_match(trimmed) {
                in_docstring = true;
                pending_docstring.clear();
                pending_docstring.push(line.to_string());

                // Check if it's a single-line docstring
                let quote = if trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
                if trimmed.len() > 6 && trimmed[3..].contains(quote) {
                    in_docstring = false;
                    if self.preserve_docstrings {
                        result.push(line.to_string());
                    }
                }
                i += 1;
                continue;
            }

            // Pop class stack when we return to a lower indent level
            if !trimmed.is_empty() {
                while let Some(&ci) = class_indent_stack.last() {
                    if indent <= ci {
                        class_indent_stack.pop();
                    } else {
                        break;
                    }
                }
            }

            // Import statements (always top-level relevant)
            if PYTHON_IMPORT.is_match(trimmed) {
                result.push(line.to_string());
                i += 1;
                continue;
            }

            // Class definition
            if let Some(caps) = PYTHON_CLASS.captures(trimmed) {
                class_indent_stack.push(indent);
                result.push(line.to_string());
                if let Some(name) = caps.get(1) {
                    symbols.push(name.as_str().to_string());
                }
                i += 1;
                continue;
            }

            // Function/method definition
            if let Some(caps) = PYTHON_DEF.captures(trimmed) {
                let def_indent = indent;

                // Check if we're inside a class (method) - def indent must be greater than class indent
                let is_method = class_indent_stack.last().is_some_and(|&ci| def_indent > ci);

                if class_indent_stack.is_empty() || is_method || def_indent == 0 {
                    result.push(line.to_string());
                    result.push(format!("{}    ...", " ".repeat(def_indent)));

                    if let Some(name) = caps.get(2) {
                        symbols.push(name.as_str().to_string());
                    }

                    // Skip the body (lines with greater indentation)
                    i += 1;
                    while i < lines.len() {
                        let next_line = lines[i];
                        let next_trimmed = next_line.trim();
                        let next_indent = next_line.len() - next_line.trim_start().len();

                        // Empty lines or comments might be part of body
                        if next_trimmed.is_empty() {
                            i += 1;
                            continue;
                        }

                        // If we're back to same or lower indent, body is done
                        if next_indent <= def_indent {
                            break;
                        }

                        // Check for nested docstring
                        if PYTHON_DOCSTRING_START.is_match(next_trimmed) && self.preserve_docstrings {
                            result.push(next_line.to_string());
                            // Handle multi-line docstring
                            let quote = if next_trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
                            if !(next_trimmed.len() > 6 && next_trimmed[3..].contains(quote)) {
                                i += 1;
                                while i < lines.len() {
                                    let ds_line = lines[i];
                                    result.push(ds_line.to_string());
                                    if ds_line.trim().ends_with(quote) {
                                        break;
                                    }
                                    i += 1;
                                }
                            }
                        }

                        i += 1;
                    }
                    continue;
                }
            }

            i += 1;
        }

        (result.join("\n"), symbols)
    }

    /// Skeletonize TypeScript/JavaScript code
    fn skeletonize_js(&self, content: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut result: Vec<String> = Vec::new();
        let mut symbols = Vec::new();
        let mut brace_depth: i32 = 0;
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            let open_braces = line.matches('{').count() as i32;
            let close_braces = line.matches('}').count() as i32;

            // At top level
            if brace_depth == 0 {
                // Imports
                if JS_IMPORT.is_match(trimmed) {
                    result.push(line.to_string());
                    i += 1;
                    continue;
                }

                // Interface/Type definitions (TypeScript)
                if JS_INTERFACE.is_match(trimmed) || JS_TYPE.is_match(trimmed) {
                    result.push(line.to_string());
                    if let Some(caps) = JS_INTERFACE.captures(trimmed) {
                        if let Some(name) = caps.get(2) {
                            symbols.push(name.as_str().to_string());
                        }
                    } else if let Some(caps) = JS_TYPE.captures(trimmed) {
                        if let Some(name) = caps.get(2) {
                            symbols.push(name.as_str().to_string());
                        }
                    }
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Class definitions
                if let Some(caps) = JS_CLASS.captures(trimmed) {
                    result.push(line.to_string());
                    if let Some(name) = caps.get(2) {
                        symbols.push(name.as_str().to_string());
                    }
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Function definitions
                if let Some(caps) = JS_FUNCTION.captures(trimmed) {
                    if let Some(name) = caps.get(3) {
                        symbols.push(name.as_str().to_string());
                    }
                    result.push(format!("{} {{ /* ... */ }}", trimmed.trim_end_matches('{')));
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Arrow functions
                if let Some(caps) = JS_CONST_FN.captures(trimmed) {
                    if let Some(name) = caps.get(3) {
                        symbols.push(name.as_str().to_string());
                    }
                    result.push(line.to_string());
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }
            } else {
                // Inside a block - check for method definitions in classes
                if let Some(caps) = JS_FUNCTION.captures(trimmed) {
                    if let Some(name) = caps.get(3) {
                        symbols.push(name.as_str().to_string());
                    }
                }
            }

            brace_depth += open_braces - close_braces;

            if brace_depth < 0 {
                return self.fallback(content);
            }

            i += 1;
        }

        (result.join("\n"), symbols)
    }

    /// Skeletonize Go code
    fn skeletonize_go(&self, content: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut result: Vec<String> = Vec::new();
        let mut symbols = Vec::new();
        let mut brace_depth: i32 = 0;
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            let open_braces = line.matches('{').count() as i32;
            let close_braces = line.matches('}').count() as i32;

            // At top level
            if brace_depth == 0 {
                // Package declaration
                if GO_PACKAGE.is_match(trimmed) {
                    result.push(line.to_string());
                    i += 1;
                    continue;
                }

                // Imports
                if GO_IMPORT.is_match(trimmed) {
                    result.push(line.to_string());
                    // Handle multi-line imports
                    if trimmed.contains('(') && !trimmed.contains(')') {
                        i += 1;
                        while i < lines.len() {
                            let import_line = lines[i];
                            result.push(import_line.to_string());
                            if import_line.contains(')') {
                                break;
                            }
                            i += 1;
                        }
                    }
                    i += 1;
                    continue;
                }

                // Constants/Variables
                if GO_CONST.is_match(trimmed) {
                    result.push(line.to_string());
                    if trimmed.contains('(') && !trimmed.contains(')') {
                        i += 1;
                        while i < lines.len() {
                            let const_line = lines[i];
                            result.push(const_line.to_string());
                            if const_line.contains(')') {
                                break;
                            }
                            i += 1;
                        }
                    }
                    i += 1;
                    continue;
                }

                // Type definitions
                if let Some(caps) = GO_TYPE.captures(trimmed) {
                    result.push(line.to_string());
                    if let Some(name) = caps.get(1) {
                        symbols.push(name.as_str().to_string());
                    }
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }

                // Function definitions
                if let Some(caps) = GO_FUNC.captures(trimmed) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(name.as_str().to_string());
                    }
                    // Just the signature
                    result.push(format!("{} {{ /* ... */ }}", trimmed.trim_end_matches('{')));
                    brace_depth += open_braces - close_braces;
                    i += 1;
                    continue;
                }
            }

            brace_depth += open_braces - close_braces;

            if brace_depth < 0 {
                return self.fallback(content);
            }

            i += 1;
        }

        (result.join("\n"), symbols)
    }

    /// Fallback: return first N lines when parsing fails
    fn fallback(&self, content: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = content.lines().take(self.fallback_lines).collect();
        (lines.join("\n"), Vec::new())
    }
}

/// Estimate token count (rough approximation: ~4 chars per token)
fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeletonize_simple_rust_fn() {
        let input = r#"
fn hello() {
    println!("Hello, world!");
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("fn hello()"));
        assert!(!result.content.contains("println!"));
        assert!(result.preserved_symbols.contains(&"hello".to_string()));
    }

    #[test]
    fn test_skeletonize_python_class() {
        let input = r#"
class Foo:
    """A class."""

    def bar(self):
        return 42
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result.content.contains("class Foo:"));
        assert!(result.content.contains("def bar(self):"));
        assert!(result.content.contains("A class."));
        assert!(!result.content.contains("return 42"));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("1234"), 1);
        assert_eq!(estimate_tokens("12345678"), 2);
    }
}
