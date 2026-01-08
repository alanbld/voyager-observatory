//! Python Language Plugin
//!
//! Provides analysis for Python source files including:
//! - Function and async function extraction
//! - Class and method extraction
//! - Decorator recognition (@pytest.fixture, @dataclass, etc.)
//! - Import statement tracking
//! - Type hint analysis
//! - Docstring extraction
//!
//! # Semantic Mapping
//!
//! Python-specific constructs are mapped to our universal semantic substrate:
//! - `@pytest.fixture` → Testing concept
//! - `@dataclass` → Configuration/Infrastructure
//! - `async def` → Infrastructure (async patterns)
//! - `try/except` → ErrorHandling
//! - Type hints contribute to feature vectors

use std::collections::HashMap;

use regex::Regex;

use crate::core::fractal::{
    ConceptType, ExtractedSymbol, Import, Parameter, Range, SymbolKind, Visibility,
};

use super::{FileInfo, LanguagePlugin, PluginResult};

// =============================================================================
// Decorator Types
// =============================================================================

/// Known decorator categories for semantic classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoratorCategory {
    /// Testing decorators: @pytest.fixture, @pytest.mark.*, @unittest.*
    Testing,
    /// Dataclass/config decorators: @dataclass, @attr.s, @pydantic.validator
    Configuration,
    /// Validation decorators: @validator, @validates, @classmethod
    Validation,
    /// Infrastructure decorators: @abstractmethod, @staticmethod, @property
    Infrastructure,
    /// Caching/memoization: @lru_cache, @cache, @cached_property
    Caching,
    /// Logging/tracing: @log, @trace, @timer
    Logging,
    /// API/routing: @app.route, @get, @post, @api_view
    ApiRouting,
    /// Unknown decorator
    Unknown,
}

impl DecoratorCategory {
    pub fn from_decorator(name: &str) -> Self {
        let name_lower = name.to_lowercase();

        // Testing patterns
        if name_lower.contains("pytest")
            || name_lower.contains("fixture")
            || name_lower.contains("unittest")
            || name_lower.contains("mock")
            || name_lower.contains("patch")
            || name_lower.starts_with("test")
        {
            return DecoratorCategory::Testing;
        }

        // Configuration/dataclass patterns
        if name_lower.contains("dataclass")
            || name_lower.contains("attr")
            || name_lower.contains("pydantic")
            || name_lower.contains("config")
            || name_lower.contains("settings")
        {
            return DecoratorCategory::Configuration;
        }

        // Validation patterns
        if name_lower.contains("validator")
            || name_lower.contains("validates")
            || name_lower.contains("validate")
            || name_lower == "classmethod"
        {
            return DecoratorCategory::Validation;
        }

        // Infrastructure patterns
        if name_lower == "abstractmethod"
            || name_lower == "staticmethod"
            || name_lower == "property"
            || name_lower.contains("override")
            || name_lower.contains("singleton")
        {
            return DecoratorCategory::Infrastructure;
        }

        // Caching patterns
        if name_lower.contains("cache")
            || name_lower.contains("memoize")
            || name_lower.contains("lru_cache")
        {
            return DecoratorCategory::Caching;
        }

        // Logging patterns
        if name_lower.contains("log")
            || name_lower.contains("trace")
            || name_lower.contains("timer")
            || name_lower.contains("profile")
        {
            return DecoratorCategory::Logging;
        }

        // API routing patterns
        if name_lower.contains("route")
            || name_lower.contains("api")
            || name_lower.contains("endpoint")
            || name_lower == "get"
            || name_lower == "post"
            || name_lower == "put"
            || name_lower == "delete"
            || name_lower == "patch"
        {
            return DecoratorCategory::ApiRouting;
        }

        DecoratorCategory::Unknown
    }

    /// Convert to ConceptType for semantic mapping
    pub fn to_concept_type(&self) -> Option<ConceptType> {
        match self {
            DecoratorCategory::Testing => Some(ConceptType::Testing),
            DecoratorCategory::Configuration => Some(ConceptType::Configuration),
            DecoratorCategory::Validation => Some(ConceptType::Validation),
            DecoratorCategory::Infrastructure => Some(ConceptType::Infrastructure),
            DecoratorCategory::Caching => Some(ConceptType::Infrastructure),
            DecoratorCategory::Logging => Some(ConceptType::Logging),
            DecoratorCategory::ApiRouting => Some(ConceptType::Infrastructure),
            DecoratorCategory::Unknown => None,
        }
    }
}

// =============================================================================
// Python Plugin
// =============================================================================

/// Plugin for analyzing Python source files.
#[allow(dead_code)]
pub struct PythonPlugin {
    /// Pattern for function definitions: `def name(args):`
    function_pattern: Regex,
    /// Pattern for async function definitions: `async def name(args):`
    async_function_pattern: Regex,
    /// Pattern for class definitions: `class Name(bases):`
    class_pattern: Regex,
    /// Pattern for method definitions inside classes
    method_pattern: Regex,
    /// Pattern for decorators: `@decorator` or `@decorator(...)`
    decorator_pattern: Regex,
    /// Pattern for imports: `import x` or `from x import y`
    import_pattern: Regex,
    /// Pattern for from imports: `from x import y, z`
    from_import_pattern: Regex,
    /// Pattern for type hints in function signatures
    type_hint_pattern: Regex,
    /// Pattern for docstrings (triple-quoted strings)
    docstring_pattern: Regex,
    /// Pattern for context managers: `with x as y:`
    context_manager_pattern: Regex,
    /// Pattern for exception handling
    try_except_pattern: Regex,
}

impl PythonPlugin {
    pub fn new() -> Self {
        Self {
            // Function: def name(args) -> ReturnType:
            function_pattern: Regex::new(
                r"(?m)^[ \t]*def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^\s:]+))?\s*:",
            )
            .unwrap(),

            // Async function: async def name(args) -> ReturnType:
            async_function_pattern: Regex::new(
                r"(?m)^[ \t]*async\s+def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^\s:]+))?\s*:",
            )
            .unwrap(),

            // Class: class Name(Base1, Base2):
            class_pattern: Regex::new(r"(?m)^[ \t]*class\s+(\w+)(?:\s*\(([^)]*)\))?\s*:").unwrap(),

            // Method (indented def): def name(self, ...):
            method_pattern: Regex::new(
                r"(?m)^[ \t]+def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^\s:]+))?\s*:",
            )
            .unwrap(),

            // Decorator: @name or @name(...) or @name.attr(...)
            decorator_pattern: Regex::new(r"(?m)^[ \t]*@([\w.]+)(?:\s*\([^)]*\))?").unwrap(),

            // Simple import: import x, y, z
            import_pattern: Regex::new(r"(?m)^[ \t]*import\s+([\w., ]+)").unwrap(),

            // From import: from x import y, z
            from_import_pattern: Regex::new(
                r"(?m)^[ \t]*from\s+([\w.]+)\s+import\s+(.+?)(?:\s*#|$)",
            )
            .unwrap(),

            // Type hint in parameter: name: Type or name: Type = default
            type_hint_pattern: Regex::new(r"(\w+)\s*:\s*([\w\[\], |]+)(?:\s*=\s*[^,)]+)?").unwrap(),

            // Docstring: """...""" or '''...'''
            docstring_pattern: Regex::new(r#"(?s)^[ \t]*(""".*?"""|'''.*?''')"#).unwrap(),

            // Context manager: with x as y:
            context_manager_pattern: Regex::new(r"(?m)^[ \t]*(?:async\s+)?with\s+.+:").unwrap(),

            // Try/except block
            try_except_pattern: Regex::new(r"(?m)^[ \t]*try\s*:").unwrap(),
        }
    }

    /// Extract decorators for a function/class at given line
    fn extract_decorators(&self, lines: &[&str], def_line: usize) -> Vec<String> {
        let mut decorators = Vec::new();
        let mut idx = def_line.saturating_sub(1);

        while idx > 0 {
            let line = lines.get(idx).unwrap_or(&"").trim();

            if let Some(cap) = self.decorator_pattern.captures(line) {
                if let Some(name) = cap.get(1) {
                    decorators.push(name.as_str().to_string());
                }
                idx = idx.saturating_sub(1);
            } else if line.is_empty() || line.starts_with('#') {
                idx = idx.saturating_sub(1);
            } else {
                break;
            }
        }

        decorators.reverse();
        decorators
    }

    /// Extract docstring from content following a definition
    fn extract_docstring(&self, content: &str, def_end_offset: usize) -> Option<String> {
        // Look for docstring immediately after the definition line
        let remaining = &content[def_end_offset..];

        // Skip to next line
        let next_line_start = remaining.find('\n').map(|i| i + 1)?;
        let after_def = &remaining[next_line_start..];

        // Look for triple-quoted string at start of next lines (with leading whitespace)
        let trimmed = after_def.trim_start();

        if trimmed.starts_with(r#"""""#) || trimmed.starts_with("'''") {
            let quote_style = if trimmed.starts_with(r#"""""#) {
                r#"""""#
            } else {
                "'''"
            };
            let start = trimmed.find(quote_style)? + 3;
            let remaining_after_open = &trimmed[start..];

            if let Some(end) = remaining_after_open.find(quote_style) {
                let doc = remaining_after_open[..end].trim();
                if !doc.is_empty() {
                    return Some(doc.to_string());
                }
            }
        }

        None
    }

    /// Parse parameters from a parameter string, extracting type hints
    fn parse_parameters(&self, param_str: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        for param in param_str.split(',') {
            let param = param.trim();
            if param.is_empty() || param == "self" || param == "cls" {
                continue;
            }

            // Check for type hint: name: Type or name: Type = default
            if let Some(cap) = self.type_hint_pattern.captures(param) {
                let name = cap.get(1).map(|m| m.as_str()).unwrap_or(param);
                let type_hint = cap.get(2).map(|m| m.as_str().to_string());

                // Check for default value
                let default_value = if param.contains('=') {
                    param.split('=').nth(1).map(|s| s.trim().to_string())
                } else {
                    None
                };

                params.push(Parameter {
                    name: name.to_string(),
                    type_hint,
                    default_value,
                });
            } else {
                // No type hint, just parameter name (maybe with default)
                let (name, default) = if param.contains('=') {
                    let parts: Vec<&str> = param.split('=').collect();
                    (parts[0].trim(), Some(parts[1].trim().to_string()))
                } else {
                    (param, None)
                };

                params.push(Parameter {
                    name: name.to_string(),
                    type_hint: None,
                    default_value: default,
                });
            }
        }

        params
    }

    /// Determine if a function is a method (has self/cls as first param)
    fn is_method(&self, param_str: &str) -> bool {
        let first_param = param_str.split(',').next().map(|s| s.trim());
        matches!(first_param, Some("self") | Some("cls"))
    }

    /// Check if symbol has async patterns in its body
    #[allow(dead_code)]
    fn has_async_patterns(&self, _content: &str, start_line: usize, lines: &[&str]) -> bool {
        // Look at next 20 lines for async patterns
        let end_line = (start_line + 20).min(lines.len());
        for line in &lines[start_line..end_line] {
            if line.contains("await ") || line.contains("async for") || line.contains("async with")
            {
                return true;
            }
        }
        false
    }

    /// Detect calls within a function body
    fn extract_calls(&self, _content: &str, start_line: usize, lines: &[&str]) -> Vec<String> {
        let mut calls = Vec::new();
        let call_pattern = Regex::new(r"(\w+)\s*\(").unwrap();

        // Simple heuristic: look at the function body (next 50 lines or until less indentation)
        let base_indent = lines
            .get(start_line)
            .map(|l| l.len() - l.trim_start().len())
            .unwrap_or(0);

        let end_line = (start_line + 50).min(lines.len());

        for i in (start_line + 1)..end_line {
            let line = lines.get(i).unwrap_or(&"");
            let current_indent = line.len() - line.trim_start().len();

            // Stop if we hit a line with less/equal indentation (end of function)
            if !line.trim().is_empty() && current_indent <= base_indent && i > start_line + 1 {
                break;
            }

            // Find function calls
            for cap in call_pattern.captures_iter(line) {
                if let Some(name) = cap.get(1) {
                    let call_name = name.as_str();
                    // Skip common built-ins and keywords
                    if ![
                        "if",
                        "for",
                        "while",
                        "with",
                        "except",
                        "print",
                        "len",
                        "str",
                        "int",
                        "float",
                        "list",
                        "dict",
                        "set",
                        "tuple",
                        "range",
                        "type",
                        "isinstance",
                        "hasattr",
                        "getattr",
                        "setattr",
                    ]
                    .contains(&call_name)
                    {
                        if !calls.contains(&call_name.to_string()) {
                            calls.push(call_name.to_string());
                        }
                    }
                }
            }
        }

        calls
    }

    /// Infer concept type from Python-specific patterns
    fn infer_python_concept(
        &self,
        symbol: &ExtractedSymbol,
        decorators: &[String],
        content_context: &str,
    ) -> ConceptType {
        // 1. Check decorators first (highest priority)
        for dec in decorators {
            let category = DecoratorCategory::from_decorator(dec);
            if let Some(concept) = category.to_concept_type() {
                return concept;
            }
        }

        // 2. Check name patterns
        let name_lower = symbol.name.to_lowercase();

        // Testing patterns
        if name_lower.starts_with("test_")
            || name_lower.starts_with("test") && symbol.kind == SymbolKind::Function
        {
            return ConceptType::Testing;
        }

        // Validation patterns
        if name_lower.contains("validate")
            || name_lower.contains("check_")
            || name_lower.starts_with("is_")
        {
            return ConceptType::Validation;
        }

        // Calculation patterns
        if name_lower.contains("calculate")
            || name_lower.contains("compute")
            || name_lower.contains("_total")
            || name_lower.contains("_sum")
        {
            return ConceptType::Calculation;
        }

        // Error handling patterns
        if name_lower.contains("error")
            || name_lower.contains("exception")
            || name_lower.contains("handle_")
        {
            return ConceptType::ErrorHandling;
        }

        // Logging patterns
        if name_lower.contains("log")
            || name_lower.contains("debug")
            || name_lower.contains("trace")
        {
            return ConceptType::Logging;
        }

        // Configuration patterns
        if name_lower.contains("config")
            || name_lower.contains("settings")
            || name_lower.contains("setup")
        {
            return ConceptType::Configuration;
        }

        // Transform/process patterns
        if name_lower.contains("transform")
            || name_lower.contains("convert")
            || name_lower.contains("process")
            || name_lower.contains("parse")
        {
            return ConceptType::Transformation;
        }

        // Decision patterns
        if name_lower.contains("decide")
            || name_lower.contains("choose")
            || name_lower.contains("select")
            || name_lower.starts_with("should_")
        {
            return ConceptType::Decision;
        }

        // 3. Check content context
        if content_context.contains("raise ") || content_context.contains("except ") {
            return ConceptType::ErrorHandling;
        }

        // 4. Check if it's an async function (infrastructure/IO)
        if symbol.signature.starts_with("async ") {
            return ConceptType::Infrastructure;
        }

        // 5. Public functions with return type hints are likely calculations or transformations
        if symbol.return_type.is_some() && symbol.visibility == Visibility::Public {
            if let Some(ret) = &symbol.return_type {
                let ret_lower = ret.to_lowercase();
                if ret_lower.contains("bool") {
                    return ConceptType::Validation;
                }
                if ret_lower.contains("int")
                    || ret_lower.contains("float")
                    || ret_lower.contains("decimal")
                {
                    return ConceptType::Calculation;
                }
            }
            return ConceptType::Transformation;
        }

        ConceptType::Unknown
    }
}

impl Default for PythonPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePlugin for PythonPlugin {
    fn language_name(&self) -> &'static str {
        "python"
    }

    fn extensions(&self) -> &[&'static str] {
        &["py", "pyw", "pyi"]
    }

    fn extract_symbols(&self, content: &str) -> PluginResult<Vec<ExtractedSymbol>> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Track class context for method extraction
        let mut _current_class: Option<(String, usize)> = None;

        // First pass: extract classes
        for cap in self.class_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let bases = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if name.is_empty() {
                continue;
            }

            let _decorators = self.extract_decorators(&lines, start_line);
            let docstring = self.extract_docstring(content, full_match.end());

            // Build signature
            let signature = if bases.is_empty() {
                format!("class {}", name)
            } else {
                format!("class {}({})", name, bases)
            };

            // Determine visibility (leading underscore = private)
            let visibility = if name.starts_with('_') && !name.starts_with("__") {
                Visibility::Private
            } else if name.starts_with("__") && !name.ends_with("__") {
                Visibility::Private
            } else {
                Visibility::Public
            };

            _current_class = Some((name.to_string(), start_line));

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Class,
                signature,
                return_type: None,
                parameters: Vec::new(),
                documentation: docstring,
                visibility,
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Second pass: extract functions (both top-level and methods)
        for cap in self.function_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().to_string());

            if name.is_empty() {
                continue;
            }

            let decorators = self.extract_decorators(&lines, start_line);
            let docstring = self.extract_docstring(content, full_match.end());
            let parameters = self.parse_parameters(params_str);
            let calls = self.extract_calls(content, start_line, &lines);

            let is_method = self.is_method(params_str);
            let kind = if is_method {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };

            // Build signature with decorators
            let decorator_str = decorators
                .iter()
                .map(|d| format!("@{}", d))
                .collect::<Vec<_>>()
                .join(" ");

            let base_sig = if let Some(ref ret) = return_type {
                format!("def {}({}) -> {}", name, params_str, ret)
            } else {
                format!("def {}({})", name, params_str)
            };

            let signature = if decorator_str.is_empty() {
                base_sig
            } else {
                format!("{} {}", decorator_str, base_sig)
            };

            // Determine visibility
            let visibility = if name.starts_with('_') && !name.starts_with("__") {
                Visibility::Private
            } else if name.starts_with("__") && name.ends_with("__") {
                Visibility::Public // Dunder methods are public
            } else if name.starts_with("__") {
                Visibility::Private
            } else {
                Visibility::Public
            };

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind,
                signature,
                return_type,
                parameters,
                documentation: docstring,
                visibility,
                range: Range::single_line(start_line + 1),
                calls,
            });
        }

        // Third pass: extract async functions
        for cap in self.async_function_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().to_string());

            if name.is_empty() {
                continue;
            }

            // Skip if already extracted as regular function (shouldn't happen but be safe)
            if symbols
                .iter()
                .any(|s| s.name == name && s.range.start_line == start_line + 1)
            {
                continue;
            }

            let decorators = self.extract_decorators(&lines, start_line);
            let docstring = self.extract_docstring(content, full_match.end());
            let parameters = self.parse_parameters(params_str);
            let calls = self.extract_calls(content, start_line, &lines);

            let is_method = self.is_method(params_str);
            let kind = if is_method {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };

            // Build async signature
            let decorator_str = decorators
                .iter()
                .map(|d| format!("@{}", d))
                .collect::<Vec<_>>()
                .join(" ");

            let base_sig = if let Some(ref ret) = return_type {
                format!("async def {}({}) -> {}", name, params_str, ret)
            } else {
                format!("async def {}({})", name, params_str)
            };

            let signature = if decorator_str.is_empty() {
                base_sig
            } else {
                format!("{} {}", decorator_str, base_sig)
            };

            let visibility = if name.starts_with('_') && !name.starts_with("__") {
                Visibility::Private
            } else {
                Visibility::Public
            };

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind,
                signature,
                return_type,
                parameters,
                documentation: docstring,
                visibility,
                range: Range::single_line(start_line + 1),
                calls,
            });
        }

        Ok(symbols)
    }

    fn extract_imports(&self, content: &str) -> PluginResult<Vec<Import>> {
        let mut imports = Vec::new();

        // Extract `import x, y, z`
        for cap in self.import_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let line_num = content[..full_match.start()].lines().count() + 1;

            let modules = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            for module in modules.split(',') {
                let module = module.trim();
                if module.is_empty() {
                    continue;
                }

                // Handle `import x as y`
                let (module_name, alias) = if module.contains(" as ") {
                    let parts: Vec<&str> = module.split(" as ").collect();
                    (parts[0].trim(), Some(parts[1].trim().to_string()))
                } else {
                    (module, None)
                };

                imports.push(Import {
                    module: module_name.to_string(),
                    items: Vec::new(),
                    alias,
                    line: line_num,
                });
            }
        }

        // Extract `from x import y, z`
        for cap in self.from_import_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let line_num = content[..full_match.start()].lines().count() + 1;

            let module = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let items_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if module.is_empty() {
                continue;
            }

            // Parse items (handling parentheses for multi-line imports)
            let items_str = items_str.trim_start_matches('(').trim_end_matches(')');
            let items: Vec<String> = items_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    // Handle `x as y`
                    if s.contains(" as ") {
                        s.split(" as ").next().unwrap_or(s).trim().to_string()
                    } else {
                        s.to_string()
                    }
                })
                .collect();

            imports.push(Import {
                module: module.to_string(),
                items,
                alias: None,
                line: line_num,
            });
        }

        Ok(imports)
    }

    fn file_info(&self, content: &str) -> PluginResult<FileInfo> {
        let symbols = self.extract_symbols(content)?;
        let line_count = content.lines().count();

        // Detect if it's a test file
        let is_test = content.contains("import pytest")
            || content.contains("import unittest")
            || content.contains("from pytest")
            || content.contains("from unittest")
            || content.contains("def test_")
            || content.contains("class Test");

        // Detect if it's executable
        let is_executable = content.contains("if __name__")
            || content.contains("#!/usr/bin/env python")
            || content.contains("#!/usr/bin/python");

        let mut metadata = HashMap::new();

        // Count async functions
        let async_count = symbols
            .iter()
            .filter(|s| s.signature.starts_with("async "))
            .count();
        if async_count > 0 {
            metadata.insert("async_functions".to_string(), async_count.to_string());
        }

        // Count classes
        let class_count = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .count();
        if class_count > 0 {
            metadata.insert("classes".to_string(), class_count.to_string());
        }

        // Detect type hint usage
        let has_type_hints = symbols.iter().any(|s| s.return_type.is_some())
            || symbols
                .iter()
                .any(|s| s.parameters.iter().any(|p| p.type_hint.is_some()));
        if has_type_hints {
            metadata.insert("type_hints".to_string(), "true".to_string());
        }

        // Detect framework usage
        if content.contains("from flask") || content.contains("import flask") {
            metadata.insert("framework".to_string(), "flask".to_string());
        } else if content.contains("from django") || content.contains("import django") {
            metadata.insert("framework".to_string(), "django".to_string());
        } else if content.contains("from fastapi") || content.contains("import fastapi") {
            metadata.insert("framework".to_string(), "fastapi".to_string());
        }

        Ok(FileInfo {
            language: "python".to_string(),
            dialect: Some("python3".to_string()),
            symbol_count: symbols.len(),
            line_count,
            is_test,
            is_executable,
            metadata,
        })
    }

    // =========================================================================
    // Semantic Mapping (Python-Specific)
    // =========================================================================

    fn infer_concept_type(&self, symbol: &ExtractedSymbol, content: &str) -> ConceptType {
        // Extract decorators from signature
        let decorators: Vec<String> = symbol
            .signature
            .split('@')
            .skip(1)
            .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Get context around the symbol for additional analysis
        let lines: Vec<&str> = content.lines().collect();
        let start = symbol.range.start_line.saturating_sub(1);
        let end = (start + 30).min(lines.len());
        let context: String = lines[start..end].join("\n");

        self.infer_python_concept(symbol, &decorators, &context)
    }

    fn semantic_relevance_boost(
        &self,
        symbol: &ExtractedSymbol,
        intent: &str,
        content: &str,
    ) -> f32 {
        let intent_lower = intent.to_lowercase();
        let mut boost: f32 = 0.0;

        // Async functions get boost for infrastructure/debugging intents
        if symbol.signature.starts_with("async ") {
            if intent_lower.contains("debug") || intent_lower.contains("infrastructure") {
                boost += 0.15;
            }
        }

        // Type-hinted functions get boost for onboarding
        if symbol.return_type.is_some() && intent_lower.contains("onboard") {
            boost += 0.1;
        }

        // Functions with docstrings get boost for onboarding
        if symbol.documentation.is_some() && intent_lower.contains("onboard") {
            boost += 0.15;
        }

        // Validation functions get boost for security review
        if intent_lower.contains("security") {
            if symbol.name.contains("validate")
                || symbol.name.contains("sanitize")
                || symbol.name.contains("escape")
            {
                boost += 0.2;
            }
        }

        // Exception handlers get boost for debugging
        if intent_lower.contains("debug") {
            let lines: Vec<&str> = content.lines().collect();
            let start = symbol.range.start_line.saturating_sub(1);
            let end = (start + 30).min(lines.len());
            let context: String = lines[start..end].join("\n");

            if context.contains("try:") || context.contains("except ") || context.contains("raise ")
            {
                boost += 0.15;
            }
        }

        boost.clamp(-0.5_f32, 0.5_f32)
    }

    fn language_features(&self, symbol: &ExtractedSymbol, content: &str) -> Vec<(usize, f32)> {
        let mut features = Vec::new();

        // Python-specific features use indices 55-59 in the 64D vector
        // Index 55: Async pattern strength (0.0 - 1.0)
        let is_async = symbol.signature.starts_with("async ");
        if is_async {
            features.push((55, 1.0));
        }

        // Index 56: Type hint completeness (0.0 - 1.0)
        let has_return_type = symbol.return_type.is_some();
        let typed_params = symbol
            .parameters
            .iter()
            .filter(|p| p.type_hint.is_some())
            .count();
        let total_params = symbol.parameters.len().max(1);
        let type_completeness = if has_return_type {
            0.5 + (0.5 * typed_params as f32 / total_params as f32)
        } else {
            0.5 * typed_params as f32 / total_params as f32
        };
        if type_completeness > 0.0 {
            features.push((56, type_completeness));
        }

        // Index 57: Decorator complexity (0.0 - 1.0)
        let decorator_count = symbol.signature.matches('@').count();
        let decorator_score = (decorator_count as f32 / 5.0).min(1.0);
        if decorator_score > 0.0 {
            features.push((57, decorator_score));
        }

        // Index 58: Documentation quality (0.0 - 1.0)
        let doc_score = if let Some(ref doc) = symbol.documentation {
            let doc_len = doc.len();
            (doc_len as f32 / 200.0).min(1.0) // Cap at 200 chars
        } else {
            0.0
        };
        if doc_score > 0.0 {
            features.push((58, doc_score));
        }

        // Index 59: Exception handling density (0.0 - 1.0)
        let lines: Vec<&str> = content.lines().collect();
        let start = symbol.range.start_line.saturating_sub(1);
        let end = (start + 50).min(lines.len());
        let context: String = lines[start..end].join("\n");

        let try_count = context.matches("try:").count();
        let except_count = context.matches("except ").count();
        let raise_count = context.matches("raise ").count();
        let exception_density = ((try_count + except_count + raise_count) as f32 / 10.0).min(1.0);
        if exception_density > 0.0 {
            features.push((59, exception_density));
        }

        features
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn plugin() -> PythonPlugin {
        PythonPlugin::new()
    }

    // =========================================================================
    // Basic Extraction Tests
    // =========================================================================

    #[test]
    fn test_language_name() {
        assert_eq!(plugin().language_name(), "python");
    }

    #[test]
    fn test_extensions() {
        let p = plugin();
        let exts = p.extensions();
        assert!(exts.contains(&"py"));
        assert!(exts.contains(&"pyw"));
        assert!(exts.contains(&"pyi"));
    }

    #[test]
    fn test_supports_file() {
        let p = plugin();
        assert!(p.supports_file(Path::new("main.py")));
        assert!(p.supports_file(Path::new("script.pyw")));
        assert!(p.supports_file(Path::new("types.pyi")));
        assert!(!p.supports_file(Path::new("main.rs")));
        assert!(!p.supports_file(Path::new("script.sh")));
    }

    // =========================================================================
    // Function Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_simple_function() {
        let content = r#"
def hello():
    print("Hello, World!")
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_function_with_params() {
        let content = r#"
def greet(name, greeting="Hello"):
    return f"{greeting}, {name}!"
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].parameters.len(), 2);
        assert_eq!(symbols[0].parameters[0].name, "name");
        assert_eq!(symbols[0].parameters[1].name, "greeting");
        assert!(symbols[0].parameters[1].default_value.is_some());
    }

    #[test]
    fn test_extract_function_with_type_hints() {
        let content = r#"
def calculate_total(items: list[int], tax_rate: float = 0.1) -> float:
    subtotal = sum(items)
    return subtotal * (1 + tax_rate)
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "calculate_total");
        assert_eq!(symbols[0].return_type, Some("float".to_string()));
        assert!(symbols[0].parameters[0].type_hint.is_some());
    }

    #[test]
    fn test_extract_async_function() {
        let content = r#"
async def fetch_data(url: str) -> dict:
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.json()
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fetch_data");
        assert!(symbols[0].signature.starts_with("async "));
    }

    #[test]
    fn test_extract_multiple_functions() {
        let content = r#"
def add(a, b):
    return a + b

def subtract(a, b):
    return a - b

def multiply(a, b):
    return a * b
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 3);
        assert!(symbols.iter().any(|s| s.name == "add"));
        assert!(symbols.iter().any(|s| s.name == "subtract"));
        assert!(symbols.iter().any(|s| s.name == "multiply"));
    }

    // =========================================================================
    // Class Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_simple_class() {
        let content = r#"
class Calculator:
    def add(self, a, b):
        return a + b
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert!(symbols
            .iter()
            .any(|s| s.name == "Calculator" && s.kind == SymbolKind::Class));
        assert!(symbols
            .iter()
            .any(|s| s.name == "add" && s.kind == SymbolKind::Method));
    }

    #[test]
    fn test_extract_class_with_inheritance() {
        let content = r#"
class Animal:
    pass

class Dog(Animal):
    def bark(self):
        print("Woof!")
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        let dog = symbols.iter().find(|s| s.name == "Dog").unwrap();
        assert!(dog.signature.contains("Animal"));
    }

    // =========================================================================
    // Decorator Tests
    // =========================================================================

    #[test]
    fn test_extract_decorated_function() {
        let content = r#"
@pytest.fixture
def database():
    return Database()

@dataclass
class Config:
    host: str
    port: int
"#;
        let symbols = plugin().extract_symbols(content).unwrap();

        let fixture = symbols.iter().find(|s| s.name == "database").unwrap();
        assert!(fixture.signature.contains("@pytest.fixture"));

        let config = symbols.iter().find(|s| s.name == "Config").unwrap();
        // Note: class decorators would be in its signature too if we tracked them
    }

    #[test]
    fn test_decorator_category_classification() {
        assert_eq!(
            DecoratorCategory::from_decorator("pytest.fixture"),
            DecoratorCategory::Testing
        );
        assert_eq!(
            DecoratorCategory::from_decorator("dataclass"),
            DecoratorCategory::Configuration
        );
        assert_eq!(
            DecoratorCategory::from_decorator("lru_cache"),
            DecoratorCategory::Caching
        );
        assert_eq!(
            DecoratorCategory::from_decorator("app.route"),
            DecoratorCategory::ApiRouting
        );
    }

    // =========================================================================
    // Import Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_simple_import() {
        let content = r#"
import os
import sys
import json
"#;
        let imports = plugin().extract_imports(content).unwrap();
        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|i| i.module == "os"));
        assert!(imports.iter().any(|i| i.module == "sys"));
    }

    #[test]
    fn test_extract_from_import() {
        let content = r#"
from typing import List, Dict, Optional
from pathlib import Path
"#;
        let imports = plugin().extract_imports(content).unwrap();
        assert!(imports
            .iter()
            .any(|i| i.module == "typing" && i.items.contains(&"List".to_string())));
        assert!(imports.iter().any(|i| i.module == "pathlib"));
    }

    #[test]
    fn test_extract_import_with_alias() {
        let content = r#"
import numpy as np
import pandas as pd
"#;
        let imports = plugin().extract_imports(content).unwrap();
        let np_import = imports.iter().find(|i| i.module == "numpy").unwrap();
        assert_eq!(np_import.alias, Some("np".to_string()));
    }

    // =========================================================================
    // File Info Tests
    // =========================================================================

    #[test]
    fn test_file_info_test_file() {
        let content = r#"
import pytest

def test_addition():
    assert 1 + 1 == 2

class TestCalculator:
    def test_add(self):
        pass
"#;
        let info = plugin().file_info(content).unwrap();
        assert!(info.is_test);
        assert_eq!(info.language, "python");
    }

    #[test]
    fn test_file_info_executable() {
        let content = r#"
#!/usr/bin/env python3

def main():
    print("Hello")

if __name__ == "__main__":
    main()
"#;
        let info = plugin().file_info(content).unwrap();
        assert!(info.is_executable);
    }

    #[test]
    fn test_file_info_async_detection() {
        let content = r#"
async def fetch():
    pass

async def process():
    await fetch()
"#;
        let info = plugin().file_info(content).unwrap();
        assert_eq!(info.metadata.get("async_functions"), Some(&"2".to_string()));
    }

    // =========================================================================
    // Semantic Concept Tests
    // =========================================================================

    #[test]
    fn test_concept_type_from_name() {
        let p = plugin();

        // Validation pattern
        let validate_sym = ExtractedSymbol {
            name: "validate_email".to_string(),
            kind: SymbolKind::Function,
            signature: "def validate_email(email: str) -> bool".to_string(),
            return_type: Some("bool".to_string()),
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };

        let concept = p.infer_concept_type(&validate_sym, "");
        assert_eq!(concept, ConceptType::Validation);
    }

    #[test]
    fn test_concept_type_from_decorator() {
        let p = plugin();

        // Test function with pytest fixture decorator
        let fixture_sym = ExtractedSymbol {
            name: "database".to_string(),
            kind: SymbolKind::Function,
            signature: "@pytest.fixture def database()".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };

        let concept = p.infer_concept_type(&fixture_sym, "");
        assert_eq!(concept, ConceptType::Testing);
    }

    // =========================================================================
    // Visibility Tests
    // =========================================================================

    #[test]
    fn test_private_function_detection() {
        let content = r#"
def public_func():
    pass

def _private_func():
    pass

def __very_private():
    pass

def __dunder__():
    pass
"#;
        let symbols = plugin().extract_symbols(content).unwrap();

        let public = symbols.iter().find(|s| s.name == "public_func").unwrap();
        assert_eq!(public.visibility, Visibility::Public);

        let private = symbols.iter().find(|s| s.name == "_private_func").unwrap();
        assert_eq!(private.visibility, Visibility::Private);

        let very_private = symbols.iter().find(|s| s.name == "__very_private").unwrap();
        assert_eq!(very_private.visibility, Visibility::Private);

        let dunder = symbols.iter().find(|s| s.name == "__dunder__").unwrap();
        assert_eq!(dunder.visibility, Visibility::Public);
    }

    // =========================================================================
    // Feature Extraction Tests
    // =========================================================================

    #[test]
    fn test_language_features_async() {
        let p = plugin();

        let async_sym = ExtractedSymbol {
            name: "fetch".to_string(),
            kind: SymbolKind::Function,
            signature: "async def fetch(url: str) -> dict".to_string(),
            return_type: Some("dict".to_string()),
            parameters: vec![Parameter {
                name: "url".to_string(),
                type_hint: Some("str".to_string()),
                default_value: None,
            }],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };

        let features = p.language_features(&async_sym, "");

        // Should have async feature (index 55)
        assert!(features.iter().any(|(idx, val)| *idx == 55 && *val > 0.0));

        // Should have type hint completeness (index 56)
        assert!(features.iter().any(|(idx, val)| *idx == 56 && *val > 0.0));
    }

    // =========================================================================
    // Integration Test: Real-World Python Code
    // =========================================================================

    #[test]
    fn test_real_world_python_extraction() {
        // Simulate a real-world Python module
        let content = r#"
#!/usr/bin/env python3
"""
User management module.
Handles user authentication and authorization.
"""

import os
import sys
from typing import Optional, List, Dict
from dataclasses import dataclass
from pathlib import Path

@dataclass
class UserConfig:
    """User configuration settings."""
    username: str
    email: str
    is_admin: bool = False

class UserManager:
    """Manages user accounts and authentication."""

    def __init__(self, db_connection):
        self.db = db_connection
        self._cache = {}

    def authenticate(self, username: str, password: str) -> bool:
        """Authenticate a user."""
        user = self._find_user(username)
        return self._verify_password(user, password)

    def _find_user(self, username: str) -> Optional[Dict]:
        """Find user in database (private)."""
        return self.db.query(f"SELECT * FROM users WHERE username = '{username}'")

    def _verify_password(self, user: Dict, password: str) -> bool:
        """Verify password hash (private)."""
        import hashlib
        return hashlib.sha256(password.encode()).hexdigest() == user.get("password_hash")

    def validate_email(self, email: str) -> bool:
        """Validate email format."""
        import re
        return bool(re.match(r'^[\w\.-]+@[\w\.-]+\.\w+$', email))

    def calculate_permissions(self, user_id: int) -> List[str]:
        """Calculate user permissions."""
        base_perms = ["read", "write"]
        if self._is_admin(user_id):
            base_perms.extend(["delete", "admin"])
        return base_perms

async def fetch_user_data(user_id: int) -> Dict:
    """Fetch user data from external API."""
    async with aiohttp.ClientSession() as session:
        async with session.get(f"/api/users/{user_id}") as response:
            return await response.json()

def process_user_import(data: List[Dict]) -> List[UserConfig]:
    """Transform imported user data into UserConfig objects."""
    return [UserConfig(**d) for d in data]

if __name__ == "__main__":
    manager = UserManager(None)
    print(manager.validate_email("test@example.com"))
"#;

        let p = plugin();
        let symbols = p.extract_symbols(content).unwrap();
        let imports = p.extract_imports(content).unwrap();
        let info = p.file_info(content).unwrap();

        // Verify classes
        let user_config = symbols.iter().find(|s| s.name == "UserConfig").unwrap();
        assert_eq!(user_config.kind, SymbolKind::Class);
        // Note: class decorators are tracked separately (not in signature yet)

        let user_manager = symbols.iter().find(|s| s.name == "UserManager").unwrap();
        assert_eq!(user_manager.kind, SymbolKind::Class);

        // Verify methods
        let authenticate = symbols.iter().find(|s| s.name == "authenticate").unwrap();
        assert_eq!(authenticate.kind, SymbolKind::Method);
        assert_eq!(authenticate.return_type, Some("bool".to_string()));

        let find_user = symbols.iter().find(|s| s.name == "_find_user").unwrap();
        assert_eq!(find_user.visibility, Visibility::Private);

        // Verify async function
        let fetch = symbols
            .iter()
            .find(|s| s.name == "fetch_user_data")
            .unwrap();
        assert!(fetch.signature.starts_with("async "));

        // Verify imports
        assert!(imports.iter().any(|i| i.module == "os"));
        assert!(imports
            .iter()
            .any(|i| i.module == "typing" && i.items.contains(&"Optional".to_string())));
        assert!(imports
            .iter()
            .any(|i| i.module == "dataclasses" && i.items.contains(&"dataclass".to_string())));

        // Verify file info
        assert!(info.is_executable);
        assert_eq!(info.language, "python");
        assert!(info.metadata.get("async_functions").is_some());
        assert!(info.metadata.get("classes").is_some());

        // Verify concept type inference
        let validate = symbols.iter().find(|s| s.name == "validate_email").unwrap();
        let concept = p.infer_concept_type(validate, content);
        assert_eq!(concept, ConceptType::Validation);

        let calculate = symbols
            .iter()
            .find(|s| s.name == "calculate_permissions")
            .unwrap();
        let concept = p.infer_concept_type(calculate, content);
        assert_eq!(concept, ConceptType::Calculation);

        let process = symbols
            .iter()
            .find(|s| s.name == "process_user_import")
            .unwrap();
        let concept = p.infer_concept_type(process, content);
        assert_eq!(concept, ConceptType::Transformation);
    }
}
