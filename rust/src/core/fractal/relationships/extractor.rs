//! Call Extractor - Language-specific call relationship extraction
//!
//! Extracts function calls from source code using regex patterns.
//! Supports Rust, Python, JavaScript, Go, and Shell.

use std::collections::HashSet;

use regex::Regex;

use super::call_graph::{CallEdge, CallGraph, CallKind, CallNode, CallableKind};

// =============================================================================
// Extraction Result
// =============================================================================

/// Result of call extraction for a single function.
#[derive(Debug, Clone)]
pub struct ExtractedCalls {
    /// The function making the calls
    pub caller: CallNode,
    /// Functions being called (name + call info)
    pub callees: Vec<(String, CallEdge)>,
}

/// Result of extracting all calls from a file.
#[derive(Debug, Clone)]
pub struct FileCallExtraction {
    /// File path
    pub file_path: String,
    /// Detected language
    pub language: String,
    /// Extracted functions and their calls
    pub extractions: Vec<ExtractedCalls>,
    /// External/unresolved calls (not defined in this file)
    pub external_calls: HashSet<String>,
}

// =============================================================================
// Call Extractor
// =============================================================================

/// Extracts call relationships from source code.
pub struct CallExtractor {
    // Rust patterns
    rust_fn_pattern: Regex,
    rust_call_pattern: Regex,
    rust_method_call_pattern: Regex,
    rust_macro_pattern: Regex,

    // Python patterns
    python_fn_pattern: Regex,
    python_call_pattern: Regex,
    #[allow(dead_code)]
    python_method_pattern: Regex,

    // JavaScript/TypeScript patterns
    js_fn_pattern: Regex,
    js_arrow_pattern: Regex,
    js_call_pattern: Regex,
    #[allow(dead_code)]
    js_method_pattern: Regex,

    // Shell patterns
    shell_fn_pattern: Regex,
    shell_call_pattern: Regex,
    shell_command_pattern: Regex,

    // Go patterns
    go_fn_pattern: Regex,
    go_call_pattern: Regex,
}

impl Default for CallExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl CallExtractor {
    pub fn new() -> Self {
        Self {
            // Rust patterns
            rust_fn_pattern: Regex::new(
                r"(?m)^[ \t]*(pub(?:\s*\([^)]*\))?\s+)?(async\s+)?fn\s+(\w+)\s*(<[^>]*>)?\s*\(([^)]*)\)(\s*->\s*[^{]+)?"
            ).unwrap(),
            rust_call_pattern: Regex::new(
                r"(?m)\b([a-z_][a-z0-9_]*)\s*(?::<[^>]*>)?\s*\("
            ).unwrap(),
            rust_method_call_pattern: Regex::new(
                r"(?m)\.([a-z_][a-z0-9_]*)\s*(?::<[^>]*>)?\s*\("
            ).unwrap(),
            rust_macro_pattern: Regex::new(
                r"(?m)\b([a-z_][a-z0-9_]*)\s*!"
            ).unwrap(),

            // Python patterns
            python_fn_pattern: Regex::new(
                r"(?m)^[ \t]*(async\s+)?def\s+(\w+)\s*\(([^)]*)\)"
            ).unwrap(),
            python_call_pattern: Regex::new(
                r"(?m)\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\("
            ).unwrap(),
            python_method_pattern: Regex::new(
                r"(?m)\.([a-zA-Z_][a-zA-Z0-9_]*)\s*\("
            ).unwrap(),

            // JavaScript/TypeScript patterns
            js_fn_pattern: Regex::new(
                r"(?m)(?:^|[^\w])(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)"
            ).unwrap(),
            js_arrow_pattern: Regex::new(
                r"(?m)(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\([^)]*\)\s*=>"
            ).unwrap(),
            js_call_pattern: Regex::new(
                r"(?m)\b([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\("
            ).unwrap(),
            js_method_pattern: Regex::new(
                r"(?m)\.([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\("
            ).unwrap(),

            // Shell patterns
            shell_fn_pattern: Regex::new(
                r"(?m)^[ \t]*(?:function\s+)?(\w+)\s*\(\s*\)\s*\{"
            ).unwrap(),
            shell_call_pattern: Regex::new(
                r"(?m)(?:^|[\n;&|])[ \t]*(\w+)(?:\s|$|;)"
            ).unwrap(),
            shell_command_pattern: Regex::new(
                r"(?m)\$\((\w+)"
            ).unwrap(),

            // Go patterns
            go_fn_pattern: Regex::new(
                r"(?m)^func\s+(?:\([^)]+\)\s+)?(\w+)\s*\(([^)]*)\)"
            ).unwrap(),
            go_call_pattern: Regex::new(
                r"(?m)\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\("
            ).unwrap(),
        }
    }

    /// Extract calls from source code, detecting language from extension.
    pub fn extract_from_file(&self, content: &str, file_path: &str) -> FileCallExtraction {
        let language = detect_language(file_path);

        let extractions = match language.as_str() {
            "rust" => self.extract_rust(content, file_path),
            "python" => self.extract_python(content, file_path),
            "javascript" | "typescript" => self.extract_javascript(content, file_path),
            "shell" => self.extract_shell(content, file_path),
            "go" => self.extract_go(content, file_path),
            _ => Vec::new(),
        };

        // Collect all defined functions
        let defined: HashSet<String> = extractions.iter().map(|e| e.caller.name.clone()).collect();

        // Find external calls
        let mut external_calls = HashSet::new();
        for extraction in &extractions {
            for (callee, _) in &extraction.callees {
                if !defined.contains(callee) {
                    external_calls.insert(callee.clone());
                }
            }
        }

        FileCallExtraction {
            file_path: file_path.to_string(),
            language,
            extractions,
            external_calls,
        }
    }

    /// Build a CallGraph from multiple file extractions.
    pub fn build_graph(&self, extractions: Vec<FileCallExtraction>) -> CallGraph {
        let mut graph = CallGraph::new();

        // First pass: add all nodes
        for file_ext in &extractions {
            for ext in &file_ext.extractions {
                graph.add_node(ext.caller.clone());
            }
        }

        // Second pass: add edges
        for file_ext in &extractions {
            for ext in &file_ext.extractions {
                for (callee_name, edge) in &ext.callees {
                    // Try to resolve the callee
                    if graph.has_node(callee_name) {
                        graph.add_edge(&ext.caller.id, callee_name, edge.clone());
                    } else {
                        // Add as external node
                        let external_node = CallNode::new(
                            callee_name.clone(),
                            callee_name.clone(),
                            CallableKind::External,
                        );
                        graph.add_node(external_node);
                        graph.add_edge(&ext.caller.id, callee_name, edge.clone());
                    }
                }
            }
        }

        graph.detect_roots();
        graph.update_metadata();
        graph.metadata.file_count = extractions.len();

        graph
    }

    // -------------------------------------------------------------------------
    // Language-specific extractors
    // -------------------------------------------------------------------------

    fn extract_rust(&self, content: &str, file_path: &str) -> Vec<ExtractedCalls> {
        let mut results = Vec::new();

        // Find all function definitions
        for cap in self.rust_fn_pattern.captures_iter(content) {
            let name = cap.get(3).unwrap().as_str();
            let is_pub = cap.get(1).is_some();
            let _is_async = cap.get(2).is_some();
            let params = cap.get(5).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap
                .get(6)
                .map(|m| m.as_str().trim_start_matches("->").trim());

            let line = content[..cap.get(0).unwrap().start()]
                .lines()
                .count()
                .saturating_add(1);

            let mut node = CallNode::new(name, name, CallableKind::Function)
                .with_location(file_path, line)
                .with_visibility(is_pub);

            let param_count = if params.is_empty() {
                0
            } else {
                params.split(',').count()
            };
            node = node.with_signature(param_count, return_type.map(String::from));

            // Find function body and extract calls
            let callees = self.extract_rust_calls_in_function(content, &cap);

            results.push(ExtractedCalls {
                caller: node,
                callees,
            });
        }

        results
    }

    fn extract_rust_calls_in_function(
        &self,
        content: &str,
        fn_cap: &regex::Captures,
    ) -> Vec<(String, CallEdge)> {
        let mut callees = Vec::new();

        // Find function body (simplified - find matching braces)
        let start = fn_cap.get(0).unwrap().end();
        if let Some(body_start) = content[start..].find('{') {
            let body_start = start + body_start;
            if let Some(body) = find_matching_brace(&content[body_start..]) {
                let fn_body = &content[body_start..body_start + body.len()];

                // Extract function calls
                for call in self.rust_call_pattern.captures_iter(fn_body) {
                    let call_name = call.get(1).unwrap().as_str();
                    // Skip common keywords and control flow
                    if !is_rust_keyword(call_name) {
                        let line_in_body = fn_body[..call.get(0).unwrap().start()].lines().count();
                        let edge =
                            CallEdge::new(CallKind::Direct).with_location(line_in_body, None);
                        callees.push((call_name.to_string(), edge));
                    }
                }

                // Extract method calls
                for call in self.rust_method_call_pattern.captures_iter(fn_body) {
                    let call_name = call.get(1).unwrap().as_str();
                    if !is_rust_keyword(call_name) {
                        let line_in_body = fn_body[..call.get(0).unwrap().start()].lines().count();
                        let edge =
                            CallEdge::new(CallKind::Method).with_location(line_in_body, None);
                        callees.push((call_name.to_string(), edge));
                    }
                }

                // Extract macro calls
                for call in self.rust_macro_pattern.captures_iter(fn_body) {
                    let call_name = call.get(1).unwrap().as_str();
                    let line_in_body = fn_body[..call.get(0).unwrap().start()].lines().count();
                    let edge = CallEdge::new(CallKind::Macro).with_location(line_in_body, None);
                    callees.push((format!("{}!", call_name), edge));
                }
            }
        }

        // Deduplicate
        let mut seen = HashSet::new();
        callees.retain(|(name, _)| seen.insert(name.clone()));

        callees
    }

    fn extract_python(&self, content: &str, file_path: &str) -> Vec<ExtractedCalls> {
        let mut results = Vec::new();

        for cap in self.python_fn_pattern.captures_iter(content) {
            let _is_async = cap.get(1).is_some();
            let name = cap.get(2).unwrap().as_str();
            let params = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            let line = content[..cap.get(0).unwrap().start()]
                .lines()
                .count()
                .saturating_add(1);

            let param_count = if params.is_empty() {
                0
            } else {
                params.split(',').count()
            };

            let node = CallNode::new(name, name, CallableKind::Function)
                .with_location(file_path, line)
                .with_signature(param_count, None);

            // Extract calls from function body (simplified - use indentation)
            let callees = self.extract_python_calls_in_function(content, &cap);

            results.push(ExtractedCalls {
                caller: node,
                callees,
            });
        }

        results
    }

    fn extract_python_calls_in_function(
        &self,
        content: &str,
        fn_cap: &regex::Captures,
    ) -> Vec<(String, CallEdge)> {
        let mut callees = Vec::new();

        let fn_end = fn_cap.get(0).unwrap().end();
        let remaining = &content[fn_end..];

        // Find body by looking at indented lines
        let body = extract_python_body(remaining);

        // Extract calls
        for call in self.python_call_pattern.captures_iter(&body) {
            let call_name = call.get(1).unwrap().as_str();
            if !is_python_keyword(call_name) && !is_python_builtin(call_name) {
                let edge = CallEdge::new(CallKind::Direct);
                callees.push((call_name.to_string(), edge));
            }
        }

        // Deduplicate
        let mut seen = HashSet::new();
        callees.retain(|(name, _)| seen.insert(name.clone()));

        callees
    }

    fn extract_javascript(&self, content: &str, file_path: &str) -> Vec<ExtractedCalls> {
        let mut results = Vec::new();

        // Regular functions
        for cap in self.js_fn_pattern.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();
            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let line = content[..cap.get(0).unwrap().start()]
                .lines()
                .count()
                .saturating_add(1);

            let param_count = if params.is_empty() {
                0
            } else {
                params.split(',').count()
            };

            let node = CallNode::new(name, name, CallableKind::Function)
                .with_location(file_path, line)
                .with_signature(param_count, None);

            let callees = self.extract_js_calls_in_function(content, &cap);

            results.push(ExtractedCalls {
                caller: node,
                callees,
            });
        }

        // Arrow functions
        for cap in self.js_arrow_pattern.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();

            let line = content[..cap.get(0).unwrap().start()]
                .lines()
                .count()
                .saturating_add(1);

            let node =
                CallNode::new(name, name, CallableKind::Function).with_location(file_path, line);

            let callees = self.extract_js_calls_in_arrow(content, &cap);

            results.push(ExtractedCalls {
                caller: node,
                callees,
            });
        }

        results
    }

    fn extract_js_calls_in_function(
        &self,
        content: &str,
        fn_cap: &regex::Captures,
    ) -> Vec<(String, CallEdge)> {
        let mut callees = Vec::new();

        let start = fn_cap.get(0).unwrap().end();
        if let Some(body_start) = content[start..].find('{') {
            let body_start = start + body_start;
            if let Some(body) = find_matching_brace(&content[body_start..]) {
                let fn_body = &content[body_start..body_start + body.len()];

                for call in self.js_call_pattern.captures_iter(fn_body) {
                    let call_name = call.get(1).unwrap().as_str();
                    if !is_js_keyword(call_name) {
                        let edge = CallEdge::new(CallKind::Direct);
                        callees.push((call_name.to_string(), edge));
                    }
                }
            }
        }

        let mut seen = HashSet::new();
        callees.retain(|(name, _)| seen.insert(name.clone()));

        callees
    }

    fn extract_js_calls_in_arrow(
        &self,
        content: &str,
        fn_cap: &regex::Captures,
    ) -> Vec<(String, CallEdge)> {
        let mut callees = Vec::new();

        let start = fn_cap.get(0).unwrap().end();
        let remaining = &content[start..];

        // Check if arrow body uses braces or is expression
        if let Some(brace_pos) = remaining.find('{') {
            if brace_pos < 20 {
                // Braces nearby
                if let Some(body) = find_matching_brace(&remaining[brace_pos..]) {
                    for call in self.js_call_pattern.captures_iter(&body) {
                        let call_name = call.get(1).unwrap().as_str();
                        if !is_js_keyword(call_name) {
                            let edge = CallEdge::new(CallKind::Direct);
                            callees.push((call_name.to_string(), edge));
                        }
                    }
                }
            }
        }

        let mut seen = HashSet::new();
        callees.retain(|(name, _)| seen.insert(name.clone()));

        callees
    }

    fn extract_shell(&self, content: &str, file_path: &str) -> Vec<ExtractedCalls> {
        let mut results = Vec::new();

        for cap in self.shell_fn_pattern.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();

            let line = content[..cap.get(0).unwrap().start()]
                .lines()
                .count()
                .saturating_add(1);

            let node = CallNode::new(name, name, CallableKind::ShellFunction)
                .with_location(file_path, line);

            let callees = self.extract_shell_calls_in_function(content, &cap);

            results.push(ExtractedCalls {
                caller: node,
                callees,
            });
        }

        results
    }

    fn extract_shell_calls_in_function(
        &self,
        content: &str,
        fn_cap: &regex::Captures,
    ) -> Vec<(String, CallEdge)> {
        let mut callees = Vec::new();

        // The pattern includes the {, so we need to back up to include it
        let end = fn_cap.get(0).unwrap().end();
        let start = end.saturating_sub(1); // Back up to include the {
        if let Some(body) = find_matching_brace(&content[start..]) {
            // Extract function calls
            for call in self.shell_call_pattern.captures_iter(&body) {
                let call_name = call.get(1).unwrap().as_str();
                if !is_shell_keyword(call_name) {
                    let edge = CallEdge::new(CallKind::Shell);
                    callees.push((call_name.to_string(), edge));
                }
            }

            // Extract command substitutions
            for call in self.shell_command_pattern.captures_iter(&body) {
                let call_name = call.get(1).unwrap().as_str();
                let edge = CallEdge::new(CallKind::Shell);
                callees.push((call_name.to_string(), edge));
            }
        }

        let mut seen = HashSet::new();
        callees.retain(|(name, _)| seen.insert(name.clone()));

        callees
    }

    fn extract_go(&self, content: &str, file_path: &str) -> Vec<ExtractedCalls> {
        let mut results = Vec::new();

        for cap in self.go_fn_pattern.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();
            let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let line = content[..cap.get(0).unwrap().start()]
                .lines()
                .count()
                .saturating_add(1);

            let param_count = if params.is_empty() {
                0
            } else {
                params.split(',').count()
            };

            let is_pub = name
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            let node = CallNode::new(name, name, CallableKind::Function)
                .with_location(file_path, line)
                .with_visibility(is_pub)
                .with_signature(param_count, None);

            let callees = self.extract_go_calls_in_function(content, &cap);

            results.push(ExtractedCalls {
                caller: node,
                callees,
            });
        }

        results
    }

    fn extract_go_calls_in_function(
        &self,
        content: &str,
        fn_cap: &regex::Captures,
    ) -> Vec<(String, CallEdge)> {
        let mut callees = Vec::new();

        let start = fn_cap.get(0).unwrap().end();
        if let Some(body_start) = content[start..].find('{') {
            let body_start = start + body_start;
            if let Some(body) = find_matching_brace(&content[body_start..]) {
                let fn_body = &content[body_start..body_start + body.len()];

                for call in self.go_call_pattern.captures_iter(fn_body) {
                    let call_name = call.get(1).unwrap().as_str();
                    if !is_go_keyword(call_name) {
                        let edge = CallEdge::new(CallKind::Direct);
                        callees.push((call_name.to_string(), edge));
                    }
                }
            }
        }

        let mut seen = HashSet::new();
        callees.retain(|(name, _)| seen.insert(name.clone()));

        callees
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn detect_language(file_path: &str) -> String {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "sh" | "bash" | "zsh" | "ksh" => "shell",
        "rb" => "ruby",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "hpp" => "cpp",
        _ => "unknown",
    }
    .to_string()
}

fn find_matching_brace(content: &str) -> Option<String> {
    let mut depth = 0;
    let mut start = None;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut escape_next = false;

    for (i, c) in content.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if c == '\\' {
            escape_next = true;
            continue;
        }

        if in_string {
            if c == string_char {
                in_string = false;
            }
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = true;
            string_char = c;
            continue;
        }

        match c {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        return Some(content[s..=i].to_string());
                    }
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_python_body(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut body_lines = Vec::new();

    // Skip the colon line and get indented content
    let mut base_indent: Option<usize> = None;

    for line in lines {
        if line.trim().is_empty() {
            if base_indent.is_some() {
                body_lines.push(line);
            }
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        if base_indent.is_none() {
            if indent > 0 {
                base_indent = Some(indent);
                body_lines.push(line);
            }
        } else if indent >= base_indent.unwrap() {
            body_lines.push(line);
        } else {
            break;
        }
    }

    body_lines.join("\n")
}

fn is_rust_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else"
            | "match"
            | "while"
            | "for"
            | "loop"
            | "return"
            | "break"
            | "continue"
            | "let"
            | "mut"
            | "const"
            | "static"
            | "fn"
            | "pub"
            | "impl"
            | "struct"
            | "enum"
            | "trait"
            | "type"
            | "mod"
            | "use"
            | "crate"
            | "self"
            | "super"
            | "as"
            | "where"
            | "async"
            | "await"
            | "move"
            | "unsafe"
            | "dyn"
            | "ref"
            | "true"
            | "false"
    )
}

fn is_python_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else"
            | "elif"
            | "while"
            | "for"
            | "return"
            | "break"
            | "continue"
            | "def"
            | "class"
            | "import"
            | "from"
            | "as"
            | "try"
            | "except"
            | "finally"
            | "raise"
            | "with"
            | "assert"
            | "yield"
            | "lambda"
            | "pass"
            | "global"
            | "nonlocal"
            | "True"
            | "False"
            | "None"
            | "and"
            | "or"
            | "not"
            | "in"
            | "is"
            | "async"
            | "await"
    )
}

fn is_python_builtin(word: &str) -> bool {
    matches!(
        word,
        "print"
            | "len"
            | "range"
            | "str"
            | "int"
            | "float"
            | "list"
            | "dict"
            | "set"
            | "tuple"
            | "bool"
            | "type"
            | "isinstance"
            | "hasattr"
            | "getattr"
            | "setattr"
            | "open"
            | "input"
            | "sorted"
            | "reversed"
            | "enumerate"
            | "zip"
            | "map"
            | "filter"
            | "sum"
            | "min"
            | "max"
            | "abs"
            | "all"
            | "any"
            | "iter"
            | "next"
            | "super"
    )
}

fn is_js_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else"
            | "switch"
            | "case"
            | "while"
            | "for"
            | "do"
            | "return"
            | "break"
            | "continue"
            | "function"
            | "class"
            | "const"
            | "let"
            | "var"
            | "import"
            | "export"
            | "default"
            | "try"
            | "catch"
            | "finally"
            | "throw"
            | "new"
            | "this"
            | "super"
            | "typeof"
            | "instanceof"
            | "true"
            | "false"
            | "null"
            | "undefined"
            | "async"
            | "await"
            | "yield"
            | "from"
    )
}

fn is_shell_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "then"
            | "else"
            | "elif"
            | "fi"
            | "case"
            | "esac"
            | "for"
            | "while"
            | "until"
            | "do"
            | "done"
            | "in"
            | "function"
            | "return"
            | "exit"
            | "break"
            | "continue"
            | "local"
            | "export"
            | "readonly"
            | "declare"
            | "typeset"
            | "source"
            | "echo"
            | "printf"
            | "read"
            | "set"
            | "unset"
            | "shift"
            | "test"
            | "true"
            | "false"
    )
}

fn is_go_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else"
            | "switch"
            | "case"
            | "for"
            | "range"
            | "return"
            | "break"
            | "continue"
            | "goto"
            | "fallthrough"
            | "func"
            | "type"
            | "struct"
            | "interface"
            | "map"
            | "chan"
            | "const"
            | "var"
            | "import"
            | "package"
            | "defer"
            | "go"
            | "select"
            | "default"
            | "true"
            | "false"
            | "nil"
            | "make"
            | "new"
            | "append"
            | "len"
            | "cap"
            | "copy"
            | "delete"
            | "panic"
            | "recover"
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn extractor() -> CallExtractor {
        CallExtractor::new()
    }

    // -------------------------------------------------------------------------
    // Rust Extraction
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_rust_simple() {
        let code = r#"
fn main() {
    foo();
    bar();
}

fn foo() {
    helper();
}

fn bar() {
    println!("hello");
}

fn helper() {}
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.rs");

        assert_eq!(result.language, "rust");
        assert_eq!(result.extractions.len(), 4); // main, foo, bar, helper

        // Find main's calls
        let main_calls = result.extractions.iter().find(|e| e.caller.name == "main");
        assert!(main_calls.is_some());
        let callees: Vec<_> = main_calls
            .unwrap()
            .callees
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        assert!(callees.contains(&"foo"));
        assert!(callees.contains(&"bar"));
    }

    #[test]
    fn test_extract_rust_with_generics() {
        let code = r#"
pub fn process<T: Clone>(items: Vec<T>) -> Result<()> {
    transform(items);
    Ok(())
}

fn transform<T>(data: T) {}
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "lib.rs");

        assert_eq!(result.extractions.len(), 2);

        let process = result
            .extractions
            .iter()
            .find(|e| e.caller.name == "process");
        assert!(process.is_some());
        assert!(process.unwrap().caller.is_public);
    }

    #[test]
    fn test_extract_rust_methods() {
        let code = r#"
fn do_work() {
    let v = vec![1, 2, 3];
    v.iter().map(|x| x * 2).collect();
    helper();
}

fn helper() {}
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.rs");

        let do_work = result
            .extractions
            .iter()
            .find(|e| e.caller.name == "do_work")
            .unwrap();

        let call_names: Vec<_> = do_work.callees.iter().map(|(n, _)| n.as_str()).collect();
        assert!(call_names.contains(&"iter"));
        assert!(call_names.contains(&"map"));
        assert!(call_names.contains(&"collect"));
        assert!(call_names.contains(&"helper"));
    }

    #[test]
    fn test_extract_rust_macros() {
        let code = r#"
fn log_stuff() {
    println!("hello");
    dbg!(value);
    vec![1, 2, 3];
}
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.rs");

        let log_stuff = result
            .extractions
            .iter()
            .find(|e| e.caller.name == "log_stuff")
            .unwrap();

        let call_names: Vec<_> = log_stuff.callees.iter().map(|(n, _)| n.as_str()).collect();
        assert!(call_names.contains(&"println!"));
        assert!(call_names.contains(&"dbg!"));
        assert!(call_names.contains(&"vec!"));
    }

    // -------------------------------------------------------------------------
    // Python Extraction
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_python_simple() {
        let code = r#"
def main():
    foo()
    bar()

def foo():
    helper()

def bar():
    pass

def helper():
    pass
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.py");

        assert_eq!(result.language, "python");
        assert_eq!(result.extractions.len(), 4);

        let main_calls = result.extractions.iter().find(|e| e.caller.name == "main");
        assert!(main_calls.is_some());
    }

    #[test]
    fn test_extract_python_async() {
        let code = r#"
async def fetch_data():
    result = await get_url()
    process_data(result)

async def get_url():
    pass

def process_data(data):
    pass
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.py");

        assert_eq!(result.extractions.len(), 3);
    }

    // -------------------------------------------------------------------------
    // JavaScript Extraction
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_js_functions() {
        let code = r#"
function main() {
    foo();
    bar();
}

function foo() {
    helper();
}

const bar = () => {
    doSomething();
};
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.js");

        assert_eq!(result.language, "javascript");
        assert!(result.extractions.len() >= 2); // main, foo, bar

        let main_calls = result.extractions.iter().find(|e| e.caller.name == "main");
        assert!(main_calls.is_some());
    }

    #[test]
    fn test_extract_js_arrow() {
        let code = r#"
const processData = (data) => {
    transform(data);
    validate(data);
};

const transform = (x) => x;
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "test.js");

        let process = result
            .extractions
            .iter()
            .find(|e| e.caller.name == "processData");
        assert!(process.is_some());
    }

    // -------------------------------------------------------------------------
    // Shell Extraction
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_shell_functions() {
        let code = r#"
#!/bin/bash

main() {
    setup
    process
    cleanup
}

setup() {
    echo "Setting up"
}

process() {
    helper
}

helper() {
    echo "Helping"
}

cleanup() {
    echo "Done"
}
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "script.sh");

        assert_eq!(result.language, "shell");
        assert_eq!(result.extractions.len(), 5);

        let main_calls = result.extractions.iter().find(|e| e.caller.name == "main");
        assert!(main_calls.is_some());
        let callees: Vec<_> = main_calls
            .unwrap()
            .callees
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        assert!(callees.contains(&"setup"));
        assert!(callees.contains(&"process"));
        assert!(callees.contains(&"cleanup"));
    }

    // -------------------------------------------------------------------------
    // Go Extraction
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_go_functions() {
        let code = r#"
package main

func main() {
    Setup()
    process()
}

func Setup() {
    helper()
}

func process() {
    doWork()
}

func helper() {}
func doWork() {}
"#;

        let ext = extractor();
        let result = ext.extract_from_file(code, "main.go");

        assert_eq!(result.language, "go");
        assert_eq!(result.extractions.len(), 5);

        // Setup should be public (uppercase)
        let setup = result
            .extractions
            .iter()
            .find(|e| e.caller.name == "Setup")
            .unwrap();
        assert!(setup.caller.is_public);

        // process should be private (lowercase)
        let process = result
            .extractions
            .iter()
            .find(|e| e.caller.name == "process")
            .unwrap();
        assert!(!process.caller.is_public);
    }

    // -------------------------------------------------------------------------
    // Graph Building
    // -------------------------------------------------------------------------

    #[test]
    fn test_build_graph_simple() {
        let code = r#"
fn main() {
    foo();
    bar();
}

fn foo() {
    baz();
}

fn bar() {}
fn baz() {}
"#;

        let ext = extractor();
        let file_ext = ext.extract_from_file(code, "test.rs");
        let graph = ext.build_graph(vec![file_ext]);

        assert_eq!(graph.node_count(), 4);
        assert!(!graph.has_cycles());

        // main should call foo and bar
        let main_calls = graph.calls_from("main");
        let call_names: Vec<_> = main_calls.iter().map(|(n, _)| n.name.as_str()).collect();
        assert!(call_names.contains(&"foo"));
        assert!(call_names.contains(&"bar"));
    }

    #[test]
    fn test_build_graph_with_external() {
        let code = r#"
fn process() {
    external_lib_call();
    helper();
}

fn helper() {}
"#;

        let ext = extractor();
        let file_ext = ext.extract_from_file(code, "test.rs");
        let graph = ext.build_graph(vec![file_ext]);

        // Should have 3 nodes: process, helper, external_lib_call (as external)
        assert_eq!(graph.node_count(), 3);

        let external = graph.get_node("external_lib_call").unwrap();
        assert_eq!(external.kind, CallableKind::External);
    }

    #[test]
    fn test_build_graph_multi_file() {
        let ext = extractor();

        let file1 = ext.extract_from_file(
            r#"
fn main() {
    lib_func();
}
"#,
            "main.rs",
        );

        let file2 = ext.extract_from_file(
            r#"
fn lib_func() {
    helper();
}

fn helper() {}
"#,
            "lib.rs",
        );

        let graph = ext.build_graph(vec![file1, file2]);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.metadata.file_count, 2);

        // main -> lib_func should be connected
        let path = graph.shortest_path("main", "helper");
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3); // main -> lib_func -> helper
    }

    // -------------------------------------------------------------------------
    // Helper Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("foo.rs"), "rust");
        assert_eq!(detect_language("foo.py"), "python");
        assert_eq!(detect_language("foo.js"), "javascript");
        assert_eq!(detect_language("foo.ts"), "typescript");
        assert_eq!(detect_language("foo.go"), "go");
        assert_eq!(detect_language("foo.sh"), "shell");
        assert_eq!(detect_language("foo.xyz"), "unknown");
    }

    #[test]
    fn test_find_matching_brace() {
        let code = "{ foo(); { bar(); } }";
        let result = find_matching_brace(code);
        assert_eq!(result, Some(code.to_string()));
    }

    #[test]
    fn test_find_matching_brace_with_strings() {
        let code = r#"{ let s = "hello { world }"; }"#;
        let result = find_matching_brace(code);
        assert_eq!(result, Some(code.to_string()));
    }
}
