//! TypeScript/JavaScript Language Plugin
//!
//! Provides analysis for TypeScript and JavaScript source files including:
//! - Function and arrow function extraction
//! - Class and method extraction
//! - Interface and type alias extraction
//! - Decorator recognition (@Component, @Injectable, etc.)
//! - Import/export statement tracking
//! - Type annotation analysis
//! - JSX/TSX component detection
//! - Framework detection (React, Angular, Vue, etc.)
//!
//! # Semantic Mapping
//!
//! TypeScript's rich type system enables precise concept classification:
//! - `(x: number) => number` → Calculation (number in/out)
//! - `interface User { ... }` → DataStructure
//! - `(x: T) => boolean` → Validation (generic + boolean)
//! - `React.FC<Props>` → UIComponent
//! - `async function fetch(): Promise<T>` → Infrastructure (async)
//! - `@Component` decorator → UIComponent (Angular)

use std::collections::HashMap;

use regex::Regex;

use crate::core::fractal::{
    ConceptType, ExtractedSymbol, Import, Parameter, Range, SymbolKind, Visibility,
};

use super::{FileInfo, LanguagePlugin, PluginResult};

// =============================================================================
// Framework Detection
// =============================================================================

/// Detected JavaScript/TypeScript framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Framework {
    /// React (JSX, hooks, components)
    React,
    /// Angular (decorators, DI)
    Angular,
    /// Vue.js
    Vue,
    /// Node.js backend
    Node,
    /// NestJS (Node + decorators)
    NestJS,
    /// Express.js
    Express,
    /// Vanilla TypeScript/JavaScript
    Vanilla,
}

impl Framework {
    /// Detect framework from source content
    pub fn detect(content: &str) -> Self {
        let mut scores: HashMap<Framework, i32> = HashMap::new();

        // React indicators
        let react_patterns = [
            "import React",
            "from 'react'",
            "from \"react\"",
            "useState",
            "useEffect",
            "useCallback",
            "useMemo",
            "React.FC",
            "React.Component",
            "JSX.Element",
            "ReactElement",
            "createElement",
        ];
        let react_score: i32 = react_patterns
            .iter()
            .filter(|p| content.contains(*p))
            .count() as i32;
        scores.insert(Framework::React, react_score);

        // Angular indicators
        let angular_patterns = [
            "@Component",
            "@Injectable",
            "@Directive",
            "@Pipe",
            "@NgModule",
            "@Input",
            "@Output",
            "Observable<",
            "Subject<",
            "BehaviorSubject<",
            "from '@angular",
            "from \"@angular",
        ];
        let angular_score: i32 = angular_patterns
            .iter()
            .filter(|p| content.contains(*p))
            .count() as i32;
        scores.insert(Framework::Angular, angular_score);

        // Vue indicators
        let vue_patterns = [
            "defineComponent",
            "ref(",
            "reactive(",
            "from 'vue'",
            "from \"vue\"",
            "@vue/",
            "Vue.component",
        ];
        let vue_score: i32 = vue_patterns.iter().filter(|p| content.contains(*p)).count() as i32;
        scores.insert(Framework::Vue, vue_score);

        // NestJS indicators
        let nestjs_patterns = [
            "@Controller",
            "@Get",
            "@Post",
            "@Put",
            "@Delete",
            "@Module",
            "@Inject",
            "from '@nestjs",
            "from \"@nestjs",
        ];
        let nestjs_score: i32 = nestjs_patterns
            .iter()
            .filter(|p| content.contains(*p))
            .count() as i32;
        scores.insert(Framework::NestJS, nestjs_score);

        // Express indicators
        let express_patterns = [
            "express()",
            "app.get(",
            "app.post(",
            "req, res",
            "req: Request",
            "res: Response",
            "from 'express'",
            "from \"express\"",
        ];
        let express_score: i32 = express_patterns
            .iter()
            .filter(|p| content.contains(*p))
            .count() as i32;
        scores.insert(Framework::Express, express_score);

        // Node.js indicators
        let node_patterns = [
            "require(",
            "module.exports",
            "exports.",
            "process.env",
            "__dirname",
            "__filename",
            "fs.",
            "path.",
            "http.",
            "https.",
        ];
        let node_score: i32 = node_patterns
            .iter()
            .filter(|p| content.contains(*p))
            .count() as i32;
        scores.insert(Framework::Node, node_score);

        // Return highest scoring framework
        scores
            .iter()
            .filter(|(_, &score)| score > 0)
            .max_by_key(|(_, &score)| score)
            .map(|(&framework, _)| framework)
            .unwrap_or(Framework::Vanilla)
    }
}

// =============================================================================
// Decorator Categories
// =============================================================================

/// Known decorator categories for semantic classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoratorCategory {
    /// Component decorators: @Component, @Directive
    Component,
    /// Service/Injectable: @Injectable, @Service
    Service,
    /// Controller/Endpoint: @Controller, @Get, @Post
    Controller,
    /// Validation decorators: @IsEmail, @IsNotEmpty
    Validation,
    /// Middleware: @Middleware, @UseGuards
    Middleware,
    /// Property decorators: @Input, @Output, @Prop
    Property,
    /// Testing decorators: @Test, @Describe
    Testing,
    /// Unknown decorator
    Unknown,
}

impl DecoratorCategory {
    pub fn from_decorator(name: &str) -> Self {
        let name_lower = name.to_lowercase();

        // Component decorators
        if name_lower == "component"
            || name_lower == "directive"
            || name_lower == "pipe"
            || name_lower == "view"
        {
            return DecoratorCategory::Component;
        }

        // Service decorators
        if name_lower == "injectable"
            || name_lower == "service"
            || name_lower == "repository"
            || name_lower == "provider"
        {
            return DecoratorCategory::Service;
        }

        // Controller/endpoint decorators
        if name_lower == "controller"
            || name_lower == "get"
            || name_lower == "post"
            || name_lower == "put"
            || name_lower == "delete"
            || name_lower == "patch"
            || name_lower == "route"
            || name_lower == "api"
        {
            return DecoratorCategory::Controller;
        }

        // Validation decorators
        if name_lower.starts_with("is")
            || name_lower.contains("valid")
            || name_lower == "validate"
            || name_lower == "required"
        {
            return DecoratorCategory::Validation;
        }

        // Middleware decorators
        if name_lower == "middleware"
            || name_lower == "useguards"
            || name_lower == "useinterceptors"
            || name_lower == "usepipes"
        {
            return DecoratorCategory::Middleware;
        }

        // Property decorators
        if name_lower == "input"
            || name_lower == "output"
            || name_lower == "prop"
            || name_lower == "column"
            || name_lower == "viewchild"
            || name_lower == "contentchild"
        {
            return DecoratorCategory::Property;
        }

        // Testing decorators
        if name_lower == "test"
            || name_lower == "it"
            || name_lower == "describe"
            || name_lower == "beforeeach"
        {
            return DecoratorCategory::Testing;
        }

        DecoratorCategory::Unknown
    }

    /// Convert to ConceptType for semantic mapping
    pub fn to_concept_type(&self) -> Option<ConceptType> {
        match self {
            DecoratorCategory::Component => Some(ConceptType::Infrastructure),
            DecoratorCategory::Service => Some(ConceptType::Infrastructure),
            DecoratorCategory::Controller => Some(ConceptType::Infrastructure),
            DecoratorCategory::Validation => Some(ConceptType::Validation),
            DecoratorCategory::Middleware => Some(ConceptType::Infrastructure),
            DecoratorCategory::Property => None,
            DecoratorCategory::Testing => Some(ConceptType::Testing),
            DecoratorCategory::Unknown => None,
        }
    }
}

// =============================================================================
// TypeScript Type Analysis
// =============================================================================

/// TypeScript type information
#[derive(Debug, Clone, Default)]
pub struct TsType {
    /// Raw type string
    pub raw: String,
    /// Whether it's a primitive type
    pub is_primitive: bool,
    /// Whether it contains generics
    pub has_generics: bool,
    /// Whether it's a union type
    pub is_union: bool,
    /// Whether it's a Promise
    pub is_promise: bool,
    /// Whether it's an array
    pub is_array: bool,
    /// Whether it's a function type
    pub is_function: bool,
}

impl TsType {
    pub fn from_string(type_str: &str) -> Self {
        let trimmed = type_str.trim();
        let lower = trimmed.to_lowercase();

        let is_primitive = matches!(
            lower.as_str(),
            "string"
                | "number"
                | "boolean"
                | "void"
                | "null"
                | "undefined"
                | "any"
                | "unknown"
                | "never"
                | "object"
                | "symbol"
                | "bigint"
        );

        Self {
            raw: trimmed.to_string(),
            is_primitive,
            has_generics: trimmed.contains('<') && trimmed.contains('>'),
            is_union: trimmed.contains('|'),
            is_promise: trimmed.starts_with("Promise<") || trimmed.contains("Promise<"),
            is_array: trimmed.ends_with("[]") || trimmed.starts_with("Array<"),
            is_function: trimmed.contains("=>") || trimmed.contains("Function"),
        }
    }

    /// Infer concept type from type annotation
    pub fn suggests_concept_type(&self) -> Option<ConceptType> {
        let lower = self.raw.to_lowercase();

        // Boolean return suggests validation
        if lower == "boolean" || lower == "bool" {
            return Some(ConceptType::Validation);
        }

        // Number return suggests calculation
        if lower == "number" || lower == "int" || lower == "float" || lower == "bigint" {
            return Some(ConceptType::Calculation);
        }

        // Promise suggests async/infrastructure
        if self.is_promise {
            return Some(ConceptType::Infrastructure);
        }

        // Array with transformation
        if self.is_array {
            return Some(ConceptType::Transformation);
        }

        // JSX/React element
        if lower.contains("jsx.element")
            || lower.contains("reactelement")
            || lower.contains("react.fc")
            || lower.contains("react.component")
        {
            return Some(ConceptType::Infrastructure);
        }

        // Error types
        if lower.contains("error") || lower == "never" {
            return Some(ConceptType::ErrorHandling);
        }

        None
    }
}

// =============================================================================
// TypeScript Plugin
// =============================================================================

/// Plugin for analyzing TypeScript and JavaScript source files.
#[allow(dead_code)]
pub struct TypeScriptPlugin {
    /// Pattern for function declarations: `function name(params): ReturnType`
    function_pattern: Regex,
    /// Pattern for async function declarations
    async_function_pattern: Regex,
    /// Pattern for arrow functions: `const name = (params): ReturnType =>`
    arrow_function_pattern: Regex,
    /// Pattern for async arrow functions
    async_arrow_pattern: Regex,
    /// Pattern for class declarations
    class_pattern: Regex,
    /// Pattern for method definitions
    method_pattern: Regex,
    /// Pattern for interface declarations
    interface_pattern: Regex,
    /// Pattern for type alias declarations
    type_alias_pattern: Regex,
    /// Pattern for decorators: `@Decorator` or `@Decorator(...)`
    decorator_pattern: Regex,
    /// Pattern for imports: `import { x } from 'y'`
    import_pattern: Regex,
    /// Pattern for require: `const x = require('y')`
    require_pattern: Regex,
    /// Pattern for exports: `export { x }` or `export default`
    export_pattern: Regex,
    /// Pattern for React hooks
    hook_pattern: Regex,
    /// Pattern for JSX elements
    jsx_pattern: Regex,
}

impl TypeScriptPlugin {
    pub fn new() -> Self {
        Self {
            // function name(params): ReturnType { ... }
            function_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?function\s+(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*:\s*([^\s{]+))?\s*\{"
            ).unwrap(),

            // async function name(params): Promise<T> { ... }
            async_function_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?async\s+function\s+(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*:\s*([^\s{]+))?\s*\{"
            ).unwrap(),

            // const name = (params): ReturnType => { ... } or const name: Type = (params) =>
            arrow_function_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*(?::\s*[^=]+)?\s*=\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*:\s*([^\s=]+))?\s*=>"
            ).unwrap(),

            // const name = async (params) => { ... }
            async_arrow_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?(?:const|let|var)\s+(\w+)\s*(?::\s*[^=]+)?\s*=\s*async\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*:\s*([^\s=]+))?\s*=>"
            ).unwrap(),

            // class Name extends Base implements Interface { ... }
            class_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?(?:abstract\s+)?class\s+(\w+)(?:\s*<[^>]+>)?(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?\s*\{"
            ).unwrap(),

            // Method: name(params): ReturnType { or async name(params)
            method_pattern: Regex::new(
                r"(?m)^[ \t]+(?:public|private|protected|static|async|readonly|\s)*\s*(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)(?:\s*:\s*([^\s{]+))?\s*\{"
            ).unwrap(),

            // interface Name extends Base { ... }
            interface_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?interface\s+(\w+)(?:\s*<[^>]+>)?(?:\s+extends\s+([^{]+))?\s*\{"
            ).unwrap(),

            // type Name = ... or type Name<T> = ...
            type_alias_pattern: Regex::new(
                r"(?m)^[ \t]*(?:export\s+)?type\s+(\w+)\s*(?:<[^>]+>)?\s*=\s*([^;]+)"
            ).unwrap(),

            // @Decorator or @Decorator(...)
            decorator_pattern: Regex::new(
                r"(?m)^[ \t]*@(\w+)(?:\s*\([^)]*\))?"
            ).unwrap(),

            // import { x, y } from 'module' or import x from 'module'
            import_pattern: Regex::new(
                r#"(?m)^[ \t]*import\s+(?:\{([^}]+)\}|(\w+))\s+from\s+['"]([^'"]+)['"]"#
            ).unwrap(),

            // const x = require('module')
            require_pattern: Regex::new(
                r#"(?m)(?:const|let|var)\s+(?:\{([^}]+)\}|(\w+))\s*=\s*require\s*\(\s*['"]([^'"]+)['"]\s*\)"#
            ).unwrap(),

            // export { x } or export default or export const
            export_pattern: Regex::new(
                r"(?m)^[ \t]*export\s+(?:default\s+)?(?:\{([^}]+)\}|(?:const|let|var|function|class|interface|type)\s+(\w+))"
            ).unwrap(),

            // React hooks: useState, useEffect, etc.
            hook_pattern: Regex::new(
                r"\b(use[A-Z]\w*)\s*(?:<[^>]+>)?\s*\("
            ).unwrap(),

            // JSX elements: <Component ... />
            jsx_pattern: Regex::new(
                r"<([A-Z]\w*)(?:\s[^>]*)?>|<([a-z]\w*)\s"
            ).unwrap(),
        }
    }

    /// Extract decorators for a symbol at given line
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
            } else if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
                idx = idx.saturating_sub(1);
            } else {
                break;
            }
        }

        decorators.reverse();
        decorators
    }

    /// Extract JSDoc comment above a definition
    fn extract_jsdoc(&self, lines: &[&str], def_line: usize) -> Option<String> {
        let mut doc_lines = Vec::new();
        let mut idx = def_line.saturating_sub(1);
        let mut in_jsdoc = false;

        while idx > 0 {
            let line = lines.get(idx).unwrap_or(&"");
            let trimmed = line.trim();

            if trimmed.ends_with("*/") {
                in_jsdoc = true;
                let content = trimmed.trim_end_matches("*/").trim();
                if !content.is_empty() && content != "/**" {
                    doc_lines.push(content.trim_start_matches("*").trim().to_string());
                }
            } else if in_jsdoc {
                if trimmed.starts_with("/**") {
                    let content = trimmed
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                    break;
                } else if trimmed.starts_with("*") {
                    let content = trimmed.trim_start_matches("*").trim();
                    // Skip JSDoc tags like @param, @returns
                    if !content.starts_with('@') && !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                }
            } else if trimmed.starts_with("//") {
                // Single-line comment
                let content = trimmed.trim_start_matches("//").trim();
                if !content.is_empty() {
                    doc_lines.push(content.to_string());
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('@') {
                break;
            }

            idx = idx.saturating_sub(1);
        }

        if doc_lines.is_empty() {
            None
        } else {
            doc_lines.reverse();
            Some(doc_lines.join(" "))
        }
    }

    /// Parse parameters from a parameter string, extracting type annotations
    fn parse_parameters(&self, param_str: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        for param in param_str.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }

            // Handle destructuring: { a, b }: Type
            if param.starts_with('{') || param.starts_with('[') {
                let type_hint = if param.contains(':') {
                    param.split(':').nth(1).map(|s| s.trim().to_string())
                } else {
                    None
                };

                params.push(Parameter {
                    name: "destructured".to_string(),
                    type_hint,
                    default_value: None,
                });
                continue;
            }

            // Parse: name: Type = default or name?: Type
            let (name_part, rest) = if param.contains(':') {
                let parts: Vec<&str> = param.splitn(2, ':').collect();
                (parts[0].trim(), Some(parts[1].trim()))
            } else if param.contains('=') {
                let parts: Vec<&str> = param.splitn(2, '=').collect();
                (parts[0].trim(), None)
            } else {
                (param, None)
            };

            let name = name_part.trim_end_matches('?').to_string();
            let is_optional = name_part.ends_with('?');

            let (type_hint, default_value) = if let Some(rest) = rest {
                if rest.contains('=') {
                    let parts: Vec<&str> = rest.splitn(2, '=').collect();
                    (
                        Some(parts[0].trim().to_string()),
                        Some(parts[1].trim().to_string()),
                    )
                } else {
                    (Some(rest.to_string()), None)
                }
            } else {
                (None, None)
            };

            params.push(Parameter {
                name,
                type_hint,
                default_value: if is_optional && default_value.is_none() {
                    Some("undefined".to_string())
                } else {
                    default_value
                },
            });
        }

        params
    }

    /// Detect if content is TSX/JSX
    fn is_tsx(&self, content: &str) -> bool {
        self.jsx_pattern.is_match(content)
            || content.contains("React.createElement")
            || content.contains("JSX.Element")
    }

    /// Detect React hooks usage
    fn count_hooks(&self, content: &str) -> usize {
        self.hook_pattern.find_iter(content).count()
    }

    /// Infer concept type from TypeScript-specific patterns
    fn infer_typescript_concept(
        &self,
        symbol: &ExtractedSymbol,
        decorators: &[String],
        content_context: &str,
    ) -> ConceptType {
        // 1. Check decorators first (highest priority for Angular/NestJS)
        for dec in decorators {
            let category = DecoratorCategory::from_decorator(dec);
            if let Some(concept) = category.to_concept_type() {
                return concept;
            }
        }

        // 2. Check return type for semantic hints
        if let Some(ref return_type) = symbol.return_type {
            let ts_type = TsType::from_string(return_type);
            if let Some(concept) = ts_type.suggests_concept_type() {
                return concept;
            }
        }

        // 3. Check name patterns
        let name_lower = symbol.name.to_lowercase();

        // React hooks
        if name_lower.starts_with("use") && symbol.kind == SymbolKind::Function {
            return ConceptType::Infrastructure;
        }

        // Validation patterns
        if name_lower.contains("validate")
            || name_lower.contains("isvalid")
            || name_lower.starts_with("is")
            || name_lower.starts_with("has")
            || name_lower.starts_with("can")
            || name_lower.starts_with("should")
        {
            return ConceptType::Validation;
        }

        // Calculation patterns
        if name_lower.contains("calculate")
            || name_lower.contains("compute")
            || name_lower.contains("sum")
            || name_lower.contains("total")
            || name_lower.contains("average")
            || name_lower.contains("count")
        {
            return ConceptType::Calculation;
        }

        // Error handling patterns
        if name_lower.contains("error")
            || name_lower.contains("handle")
            || name_lower.contains("catch")
            || name_lower.contains("throw")
        {
            return ConceptType::ErrorHandling;
        }

        // Logging patterns
        if name_lower.contains("log")
            || name_lower.contains("trace")
            || name_lower.contains("debug")
            || name_lower.contains("info")
        {
            return ConceptType::Logging;
        }

        // Configuration patterns
        if name_lower.contains("config")
            || name_lower.contains("settings")
            || name_lower.contains("options")
            || name_lower.contains("setup")
        {
            return ConceptType::Configuration;
        }

        // Transform/process patterns
        if name_lower.contains("transform")
            || name_lower.contains("convert")
            || name_lower.contains("map")
            || name_lower.contains("filter")
            || name_lower.contains("reduce")
            || name_lower.contains("parse")
        {
            return ConceptType::Transformation;
        }

        // Fetch/load patterns
        if name_lower.contains("fetch")
            || name_lower.contains("load")
            || name_lower.contains("get")
            || name_lower.contains("find")
        {
            return ConceptType::Infrastructure;
        }

        // 4. Check if it's async (infrastructure/IO)
        if symbol.signature.contains("async ")
            || symbol
                .return_type
                .as_ref()
                .map_or(false, |t| t.contains("Promise"))
        {
            return ConceptType::Infrastructure;
        }

        // 5. Check content context
        if content_context.contains("throw new") || content_context.contains("catch (") {
            return ConceptType::ErrorHandling;
        }

        if content_context.contains("return <") || content_context.contains("React.createElement") {
            return ConceptType::Infrastructure;
        }

        // 6. Interface/type is data structure
        if symbol.kind == SymbolKind::Interface || symbol.kind == SymbolKind::Type {
            return ConceptType::Configuration;
        }

        ConceptType::Unknown
    }
}

impl Default for TypeScriptPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePlugin for TypeScriptPlugin {
    fn language_name(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &[&'static str] {
        &["ts", "tsx", "js", "jsx", "mjs", "mts", "cts"]
    }

    fn extract_symbols(&self, content: &str) -> PluginResult<Vec<ExtractedSymbol>> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Extract classes first
        for cap in self.class_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let extends = cap.get(2).map(|m| m.as_str());
            let implements = cap.get(3).map(|m| m.as_str());

            if name.is_empty() {
                continue;
            }

            let decorators = self.extract_decorators(&lines, start_line);
            let documentation = self.extract_jsdoc(&lines, start_line);

            // Build signature
            let mut signature = format!("class {}", name);
            if let Some(ext) = extends {
                signature.push_str(&format!(" extends {}", ext));
            }
            if let Some(imp) = implements {
                signature.push_str(&format!(" implements {}", imp.trim()));
            }

            // Add decorators to signature
            if !decorators.is_empty() {
                let dec_str = decorators
                    .iter()
                    .map(|d| format!("@{}", d))
                    .collect::<Vec<_>>()
                    .join(" ");
                signature = format!("{} {}", dec_str, signature);
            }

            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Class,
                signature,
                return_type: None,
                parameters: Vec::new(),
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract interfaces
        for cap in self.interface_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let extends = cap.get(2).map(|m| m.as_str());

            if name.is_empty() {
                continue;
            }

            let documentation = self.extract_jsdoc(&lines, start_line);

            let mut signature = format!("interface {}", name);
            if let Some(ext) = extends {
                signature.push_str(&format!(" extends {}", ext.trim()));
            }

            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Interface,
                signature,
                return_type: None,
                parameters: Vec::new(),
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract type aliases
        for cap in self.type_alias_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let type_def = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if name.is_empty() {
                continue;
            }

            let documentation = self.extract_jsdoc(&lines, start_line);
            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Type,
                signature: format!("type {} = {}", name, type_def.trim()),
                return_type: Some(type_def.trim().to_string()),
                parameters: Vec::new(),
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract regular functions
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
            let documentation = self.extract_jsdoc(&lines, start_line);
            let parameters = self.parse_parameters(params_str);

            let base_sig = if let Some(ref ret) = return_type {
                format!("function {}({}): {}", name, params_str, ret)
            } else {
                format!("function {}({})", name, params_str)
            };

            let signature = if decorators.is_empty() {
                base_sig
            } else {
                let dec_str = decorators
                    .iter()
                    .map(|d| format!("@{}", d))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} {}", dec_str, base_sig)
            };

            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature,
                return_type,
                parameters,
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract async functions
        for cap in self.async_function_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().to_string());

            if name.is_empty() {
                continue;
            }

            // Skip if already extracted
            if symbols
                .iter()
                .any(|s| s.name == name && s.range.start_line == start_line + 1)
            {
                continue;
            }

            let decorators = self.extract_decorators(&lines, start_line);
            let documentation = self.extract_jsdoc(&lines, start_line);
            let parameters = self.parse_parameters(params_str);

            let base_sig = if let Some(ref ret) = return_type {
                format!("async function {}({}): {}", name, params_str, ret)
            } else {
                format!("async function {}({})", name, params_str)
            };

            let signature = if decorators.is_empty() {
                base_sig
            } else {
                let dec_str = decorators
                    .iter()
                    .map(|d| format!("@{}", d))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} {}", dec_str, base_sig)
            };

            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature,
                return_type,
                parameters,
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract arrow functions
        for cap in self.arrow_function_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().to_string());

            if name.is_empty() {
                continue;
            }

            // Skip if already extracted
            if symbols
                .iter()
                .any(|s| s.name == name && s.range.start_line == start_line + 1)
            {
                continue;
            }

            let decorators = self.extract_decorators(&lines, start_line);
            let documentation = self.extract_jsdoc(&lines, start_line);
            let parameters = self.parse_parameters(params_str);

            let base_sig = if let Some(ref ret) = return_type {
                format!("const {} = ({}) => {}", name, params_str, ret)
            } else {
                format!("const {} = ({}) =>", name, params_str)
            };

            let signature = if decorators.is_empty() {
                base_sig
            } else {
                let dec_str = decorators
                    .iter()
                    .map(|d| format!("@{}", d))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} {}", dec_str, base_sig)
            };

            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature,
                return_type,
                parameters,
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract async arrow functions
        for cap in self.async_arrow_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().to_string());

            if name.is_empty() {
                continue;
            }

            // Skip if already extracted
            if symbols
                .iter()
                .any(|s| s.name == name && s.range.start_line == start_line + 1)
            {
                continue;
            }

            let decorators = self.extract_decorators(&lines, start_line);
            let documentation = self.extract_jsdoc(&lines, start_line);
            let parameters = self.parse_parameters(params_str);

            let base_sig = if let Some(ref ret) = return_type {
                format!("const {} = async ({}) => {}", name, params_str, ret)
            } else {
                format!("const {} = async ({}) =>", name, params_str)
            };

            let signature = if decorators.is_empty() {
                base_sig
            } else {
                let dec_str = decorators
                    .iter()
                    .map(|d| format!("@{}", d))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} {}", dec_str, base_sig)
            };

            let is_exported = full_match.as_str().contains("export ");

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature,
                return_type,
                parameters,
                documentation,
                visibility: if is_exported {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract class methods
        for cap in self.method_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().to_string());

            if name.is_empty() {
                continue;
            }

            // Skip keywords and already extracted symbols
            let skip_names = [
                "if",
                "for",
                "while",
                "switch",
                "catch",
                "function",
                "class",
                "constructor",
            ];
            if skip_names.contains(&name) {
                continue;
            }

            // Skip if already extracted (same name at same line)
            if symbols
                .iter()
                .any(|s| s.name == name && s.range.start_line == start_line + 1)
            {
                continue;
            }

            let documentation = self.extract_jsdoc(&lines, start_line);
            let parameters = self.parse_parameters(params_str);

            // Detect visibility from modifiers in the match
            let match_str = full_match.as_str();
            let visibility = if match_str.contains("private") {
                Visibility::Private
            } else {
                Visibility::Public
            };

            let is_async = match_str.contains("async");
            let is_static = match_str.contains("static");

            let base_sig = if let Some(ref ret) = return_type {
                format!("{}({}): {}", name, params_str, ret.trim())
            } else {
                format!("{}({})", name, params_str)
            };

            let signature = match (is_async, is_static) {
                (true, true) => format!("static async {}", base_sig),
                (true, false) => format!("async {}", base_sig),
                (false, true) => format!("static {}", base_sig),
                (false, false) => base_sig,
            };

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Method,
                signature,
                return_type,
                parameters,
                documentation,
                visibility,
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        Ok(symbols)
    }

    fn extract_imports(&self, content: &str) -> PluginResult<Vec<Import>> {
        let mut imports = Vec::new();

        // Extract ES6 imports
        for cap in self.import_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let line_num = content[..full_match.start()].lines().count() + 1;

            let module = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            if module.is_empty() {
                continue;
            }

            // Named imports: { x, y, z }
            let items = if let Some(named) = cap.get(1) {
                named
                    .as_str()
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
                    .collect()
            } else {
                Vec::new()
            };

            // Default import: import x from '...'
            let alias = cap.get(2).map(|m| m.as_str().to_string());

            imports.push(Import {
                module: module.to_string(),
                items,
                alias,
                line: line_num,
            });
        }

        // Extract require() calls
        for cap in self.require_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let line_num = content[..full_match.start()].lines().count() + 1;

            let module = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            if module.is_empty() {
                continue;
            }

            let items = if let Some(named) = cap.get(1) {
                named
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                Vec::new()
            };

            let alias = cap.get(2).map(|m| m.as_str().to_string());

            imports.push(Import {
                module: module.to_string(),
                items,
                alias,
                line: line_num,
            });
        }

        Ok(imports)
    }

    fn file_info(&self, content: &str) -> PluginResult<FileInfo> {
        let symbols = self.extract_symbols(content)?;
        let line_count = content.lines().count();

        // Detect if it's a test file
        let is_test = content.contains("describe(")
            || content.contains("it(")
            || content.contains("test(")
            || content.contains("expect(")
            || content.contains("@Test")
            || content.contains("jest")
            || content.contains("mocha")
            || content.contains("vitest");

        // Detect if it's executable
        let is_executable = content.contains("if (require.main === module)")
            || content.contains("process.argv")
            || content.contains("#!/usr/bin/env node")
            || content.contains("#!/usr/bin/node");

        let mut metadata = HashMap::new();

        // Detect framework
        let framework = Framework::detect(content);
        if framework != Framework::Vanilla {
            metadata.insert("framework".to_string(), format!("{:?}", framework));
        }

        // Detect TSX/JSX
        let is_tsx = self.is_tsx(content);
        if is_tsx {
            metadata.insert("jsx".to_string(), "true".to_string());
        }

        // Count hooks
        let hook_count = self.count_hooks(content);
        if hook_count > 0 {
            metadata.insert("react_hooks".to_string(), hook_count.to_string());
        }

        // Count async functions
        let async_count = symbols
            .iter()
            .filter(|s| s.signature.contains("async "))
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

        // Count interfaces
        let interface_count = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .count();
        if interface_count > 0 {
            metadata.insert("interfaces".to_string(), interface_count.to_string());
        }

        // Detect type annotation usage
        let has_types = symbols.iter().any(|s| s.return_type.is_some())
            || symbols
                .iter()
                .any(|s| s.parameters.iter().any(|p| p.type_hint.is_some()));
        if has_types {
            metadata.insert("typed".to_string(), "true".to_string());
        }

        Ok(FileInfo {
            language: "typescript".to_string(),
            dialect: if is_tsx {
                Some("tsx".to_string())
            } else {
                Some("ts".to_string())
            },
            symbol_count: symbols.len(),
            line_count,
            is_test,
            is_executable,
            metadata,
        })
    }

    // =========================================================================
    // Semantic Mapping (TypeScript-Specific)
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

        // Get context around the symbol
        let lines: Vec<&str> = content.lines().collect();
        let start = symbol.range.start_line.saturating_sub(1);
        let end = (start + 30).min(lines.len());
        let context: String = lines[start..end].join("\n");

        self.infer_typescript_concept(symbol, &decorators, &context)
    }

    fn semantic_relevance_boost(
        &self,
        symbol: &ExtractedSymbol,
        intent: &str,
        content: &str,
    ) -> f32 {
        let intent_lower = intent.to_lowercase();
        let mut boost: f32 = 0.0;

        // Type-annotated functions get boost for onboarding
        if symbol.return_type.is_some() && intent_lower.contains("onboard") {
            boost += 0.15;
        }

        // Functions with JSDoc get boost for onboarding
        if symbol.documentation.is_some() && intent_lower.contains("onboard") {
            boost += 0.15;
        }

        // Async functions get boost for debugging
        if symbol.signature.contains("async ") && intent_lower.contains("debug") {
            boost += 0.1;
        }

        // Interface/type definitions get boost for architecture review
        if (symbol.kind == SymbolKind::Interface || symbol.kind == SymbolKind::Type)
            && intent_lower.contains("architecture")
        {
            boost += 0.2;
        }

        // Validation functions get boost for security review
        if intent_lower.contains("security") {
            let name_lower = symbol.name.to_lowercase();
            if name_lower.contains("validate")
                || name_lower.contains("sanitize")
                || name_lower.contains("escape")
                || name_lower.starts_with("is")
            {
                boost += 0.2;
            }
        }

        // React components get boost for UI-related intents
        if intent_lower.contains("ui") || intent_lower.contains("component") {
            let lines: Vec<&str> = content.lines().collect();
            let start = symbol.range.start_line.saturating_sub(1);
            let end = (start + 30).min(lines.len());
            let context: String = lines[start..end].join("\n");

            if context.contains("return <") || context.contains("React.FC") {
                boost += 0.2;
            }
        }

        boost.clamp(-0.5_f32, 0.5_f32)
    }

    fn language_features(&self, symbol: &ExtractedSymbol, content: &str) -> Vec<(usize, f32)> {
        let mut features = Vec::new();

        // TypeScript-specific features use indices 60-63 in the 64D vector
        // (Python uses 55-59, ABL uses 50-54)

        // Index 60: Type annotation completeness (0.0 - 1.0)
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
            features.push((60, type_completeness));
        }

        // Index 61: Async/Promise usage (0.0 - 1.0)
        let is_async = symbol.signature.contains("async ");
        let has_promise = symbol
            .return_type
            .as_ref()
            .map_or(false, |t| t.contains("Promise"));
        let async_score = if is_async && has_promise {
            1.0
        } else if is_async || has_promise {
            0.5
        } else {
            0.0
        };
        if async_score > 0.0 {
            features.push((61, async_score));
        }

        // Index 62: Generic type usage (0.0 - 1.0)
        let has_generics = symbol.signature.contains('<') && symbol.signature.contains('>');
        if has_generics {
            let generic_count = symbol.signature.matches('<').count();
            let generic_score = (generic_count as f32 / 3.0).min(1.0);
            features.push((62, generic_score));
        }

        // Index 63: Framework pattern indicator (0.0 - 1.0)
        let lines: Vec<&str> = content.lines().collect();
        let start = symbol.range.start_line.saturating_sub(1);
        let end = (start + 30).min(lines.len());
        let context: String = lines[start..end].join("\n");

        let mut framework_score: f32 = 0.0;

        // React patterns
        if context.contains("useState")
            || context.contains("useEffect")
            || context.contains("return <")
            || context.contains("React.FC")
        {
            framework_score += 0.5;
        }

        // Angular patterns
        if symbol.signature.contains("@Component") || symbol.signature.contains("@Injectable") {
            framework_score += 0.5;
        }

        // Express/NestJS patterns
        if symbol.signature.contains("@Controller")
            || symbol.signature.contains("@Get")
            || context.contains("req, res")
        {
            framework_score += 0.5;
        }

        if framework_score > 0.0 {
            features.push((63, framework_score.min(1.0)));
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

    fn plugin() -> TypeScriptPlugin {
        TypeScriptPlugin::new()
    }

    // =========================================================================
    // Basic Tests
    // =========================================================================

    #[test]
    fn test_language_name() {
        assert_eq!(plugin().language_name(), "typescript");
    }

    #[test]
    fn test_extensions() {
        let p = plugin();
        let exts = p.extensions();
        assert!(exts.contains(&"ts"));
        assert!(exts.contains(&"tsx"));
        assert!(exts.contains(&"js"));
        assert!(exts.contains(&"jsx"));
    }

    #[test]
    fn test_supports_file() {
        let p = plugin();
        assert!(p.supports_file(Path::new("main.ts")));
        assert!(p.supports_file(Path::new("app.tsx")));
        assert!(p.supports_file(Path::new("script.js")));
        assert!(p.supports_file(Path::new("component.jsx")));
        assert!(!p.supports_file(Path::new("main.rs")));
        assert!(!p.supports_file(Path::new("app.py")));
    }

    // =========================================================================
    // Function Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_simple_function() {
        let content = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].return_type, Some("string".to_string()));
    }

    #[test]
    fn test_extract_async_function() {
        let content = r#"
async function fetchData(url: string): Promise<Response> {
    const response = await fetch(url);
    return response;
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fetchData");
        assert!(symbols[0].signature.contains("async"));
        assert_eq!(
            symbols[0].return_type,
            Some("Promise<Response>".to_string())
        );
    }

    #[test]
    fn test_extract_arrow_function() {
        let content = r#"
const add = (a: number, b: number): number => {
    return a + b;
};
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "add");
        assert!(symbols[0].signature.contains("=>"));
    }

    #[test]
    fn test_extract_async_arrow_function() {
        let content = r#"
const fetchUser = async (id: string): Promise<User> => {
    const response = await fetch(`/api/users/${id}`);
    return response.json();
};
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fetchUser");
        assert!(symbols[0].signature.contains("async"));
    }

    #[test]
    fn test_extract_exported_function() {
        let content = r#"
export function calculateTotal(items: Item[]): number {
    return items.reduce((sum, item) => sum + item.price, 0);
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].visibility, Visibility::Public);
    }

    // =========================================================================
    // Class Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_class() {
        let content = r#"
class UserService {
    private users: User[] = [];

    getUser(id: string): User | undefined {
        return this.users.find(u => u.id === id);
    }
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        let class = symbols.iter().find(|s| s.name == "UserService").unwrap();
        assert_eq!(class.kind, SymbolKind::Class);
    }

    #[test]
    fn test_extract_class_with_inheritance() {
        let content = r#"
class AdminUser extends User implements Serializable {
    constructor() {
        super();
    }
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        let class = symbols.iter().find(|s| s.name == "AdminUser").unwrap();
        assert!(class.signature.contains("extends User"));
        assert!(class.signature.contains("implements Serializable"));
    }

    // =========================================================================
    // Interface Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_interface() {
        let content = r#"
interface User {
    id: string;
    name: string;
    email: string;
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "User");
        assert_eq!(symbols[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn test_extract_interface_with_extends() {
        let content = r#"
export interface AdminUser extends User, Serializable {
    permissions: string[];
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        let interface = symbols.iter().find(|s| s.name == "AdminUser").unwrap();
        assert!(interface.signature.contains("extends User, Serializable"));
        assert_eq!(interface.visibility, Visibility::Public);
    }

    // =========================================================================
    // Type Alias Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_type_alias() {
        let content = r#"
type UserId = string;
type Callback<T> = (value: T) => void;
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 2);

        let user_id = symbols.iter().find(|s| s.name == "UserId").unwrap();
        assert_eq!(user_id.kind, SymbolKind::Type);

        let callback = symbols.iter().find(|s| s.name == "Callback").unwrap();
        assert!(callback.signature.contains("(value: T) => void"));
    }

    // =========================================================================
    // Decorator Tests
    // =========================================================================

    #[test]
    fn test_extract_decorated_class() {
        // Note: Single-line decorator format for regex parsing
        let content = r#"
@Component
export class AppComponent {
    title = 'My App';
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        let class = symbols.iter().find(|s| s.name == "AppComponent").unwrap();
        assert!(class.signature.contains("@Component"));
    }

    #[test]
    fn test_decorator_category_classification() {
        assert_eq!(
            DecoratorCategory::from_decorator("Component"),
            DecoratorCategory::Component
        );
        assert_eq!(
            DecoratorCategory::from_decorator("Injectable"),
            DecoratorCategory::Service
        );
        assert_eq!(
            DecoratorCategory::from_decorator("Controller"),
            DecoratorCategory::Controller
        );
        assert_eq!(
            DecoratorCategory::from_decorator("Get"),
            DecoratorCategory::Controller
        );
    }

    // =========================================================================
    // Import Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_named_imports() {
        let content = r#"
import { useState, useEffect, useCallback } from 'react';
import { User, UserService } from './user';
"#;
        let imports = plugin().extract_imports(content).unwrap();

        let react_import = imports.iter().find(|i| i.module == "react").unwrap();
        assert!(react_import.items.contains(&"useState".to_string()));
        assert!(react_import.items.contains(&"useEffect".to_string()));
    }

    #[test]
    fn test_extract_default_import() {
        let content = r#"
import React from 'react';
import express from 'express';
"#;
        let imports = plugin().extract_imports(content).unwrap();

        let react_import = imports.iter().find(|i| i.module == "react").unwrap();
        assert_eq!(react_import.alias, Some("React".to_string()));
    }

    #[test]
    fn test_extract_require() {
        let content = r#"
const express = require('express');
const { Router } = require('express');
"#;
        let imports = plugin().extract_imports(content).unwrap();
        assert_eq!(imports.len(), 2);
        assert!(imports.iter().all(|i| i.module == "express"));
    }

    // =========================================================================
    // Framework Detection Tests
    // =========================================================================

    #[test]
    fn test_detect_react_framework() {
        let content = r#"
import React, { useState, useEffect } from 'react';

const Counter: React.FC = () => {
    const [count, setCount] = useState(0);

    return <div>{count}</div>;
};
"#;
        assert_eq!(Framework::detect(content), Framework::React);
    }

    #[test]
    fn test_detect_angular_framework() {
        let content = r#"
import { Component, Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Component({
    selector: 'app-root'
})
export class AppComponent {}

@Injectable()
export class UserService {}
"#;
        assert_eq!(Framework::detect(content), Framework::Angular);
    }

    #[test]
    fn test_detect_nestjs_framework() {
        let content = r#"
import { Controller, Get, Post } from '@nestjs/common';

@Controller('users')
export class UsersController {
    @Get()
    findAll() {
        return [];
    }
}
"#;
        assert_eq!(Framework::detect(content), Framework::NestJS);
    }

    // =========================================================================
    // File Info Tests
    // =========================================================================

    #[test]
    fn test_file_info_test_file() {
        let content = r#"
import { describe, it, expect } from 'vitest';

describe('Calculator', () => {
    it('should add numbers', () => {
        expect(1 + 1).toBe(2);
    });
});
"#;
        let info = plugin().file_info(content).unwrap();
        assert!(info.is_test);
    }

    #[test]
    fn test_file_info_tsx() {
        let content = r#"
import React from 'react';

const Button = () => {
    return <Button onClick={handleClick}>Click me</Button>;
};
"#;
        let info = plugin().file_info(content).unwrap();
        assert_eq!(info.metadata.get("jsx"), Some(&"true".to_string()));
    }

    #[test]
    fn test_file_info_framework_detection() {
        let content = r#"
import { Component } from '@angular/core';

@Component({ selector: 'app' })
export class AppComponent {}
"#;
        let info = plugin().file_info(content).unwrap();
        assert_eq!(info.metadata.get("framework"), Some(&"Angular".to_string()));
    }

    // =========================================================================
    // Semantic Concept Tests
    // =========================================================================

    #[test]
    fn test_concept_from_return_type() {
        let p = plugin();

        // Boolean return → Validation
        let validate_sym = ExtractedSymbol {
            name: "isValid".to_string(),
            kind: SymbolKind::Function,
            signature: "function isValid(x: unknown): boolean".to_string(),
            return_type: Some("boolean".to_string()),
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };
        assert_eq!(
            p.infer_concept_type(&validate_sym, ""),
            ConceptType::Validation
        );

        // Number return → Calculation
        let calc_sym = ExtractedSymbol {
            name: "sum".to_string(),
            kind: SymbolKind::Function,
            signature: "function sum(a: number, b: number): number".to_string(),
            return_type: Some("number".to_string()),
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };
        assert_eq!(
            p.infer_concept_type(&calc_sym, ""),
            ConceptType::Calculation
        );
    }

    #[test]
    fn test_concept_from_decorator() {
        let p = plugin();

        let component_sym = ExtractedSymbol {
            name: "AppComponent".to_string(),
            kind: SymbolKind::Class,
            signature: "@Component class AppComponent".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };
        assert_eq!(
            p.infer_concept_type(&component_sym, ""),
            ConceptType::Infrastructure
        );
    }

    #[test]
    fn test_react_hook_concept() {
        let p = plugin();

        let hook_sym = ExtractedSymbol {
            name: "useAuth".to_string(),
            kind: SymbolKind::Function,
            signature: "function useAuth()".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };
        assert_eq!(
            p.infer_concept_type(&hook_sym, ""),
            ConceptType::Infrastructure
        );
    }

    // =========================================================================
    // Type Analysis Tests
    // =========================================================================

    #[test]
    fn test_type_parsing() {
        let bool_type = TsType::from_string("boolean");
        assert!(bool_type.is_primitive);
        assert_eq!(
            bool_type.suggests_concept_type(),
            Some(ConceptType::Validation)
        );

        let promise_type = TsType::from_string("Promise<User>");
        assert!(promise_type.is_promise);
        assert!(promise_type.has_generics);
        assert_eq!(
            promise_type.suggests_concept_type(),
            Some(ConceptType::Infrastructure)
        );

        let array_type = TsType::from_string("User[]");
        assert!(array_type.is_array);
        assert_eq!(
            array_type.suggests_concept_type(),
            Some(ConceptType::Transformation)
        );
    }

    // =========================================================================
    // Feature Extraction Tests
    // =========================================================================

    #[test]
    fn test_language_features() {
        let p = plugin();

        let typed_async_sym = ExtractedSymbol {
            name: "fetchUsers".to_string(),
            kind: SymbolKind::Function,
            signature: "async function fetchUsers<T>(id: string): Promise<T[]>".to_string(),
            return_type: Some("Promise<T[]>".to_string()),
            parameters: vec![Parameter {
                name: "id".to_string(),
                type_hint: Some("string".to_string()),
                default_value: None,
            }],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::single_line(1),
            calls: vec![],
        };

        let features = p.language_features(&typed_async_sym, "");

        // Should have type completeness (index 60)
        assert!(features.iter().any(|(idx, val)| *idx == 60 && *val > 0.5));

        // Should have async score (index 61)
        assert!(features.iter().any(|(idx, val)| *idx == 61 && *val > 0.0));

        // Should have generics (index 62)
        assert!(features.iter().any(|(idx, val)| *idx == 62 && *val > 0.0));
    }

    // =========================================================================
    // Integration Test
    // =========================================================================

    #[test]
    fn test_real_world_typescript_extraction() {
        let content = r#"
/**
 * User management service.
 */
import { Injectable } from '@angular/core';
import { Observable, BehaviorSubject } from 'rxjs';

export interface User {
    id: string;
    name: string;
    email: string;
}

export type UserRole = 'admin' | 'user' | 'guest';

@Injectable
export class UserService {
    private users$ = new BehaviorSubject<User[]>([]);

    async fetchUsers(): Promise<User[]> {
        const response = await fetch('/api/users');
        return response.json();
    }

    isValidEmail(email: string): boolean {
        return /^[\w.-]+@[\w.-]+\.\w+$/.test(email);
    }

    calculateAge(birthDate: Date): number {
        const today = new Date();
        return today.getFullYear() - birthDate.getFullYear();
    }
}

const formatUserName = (user: User): string => {
    return `${user.name} <${user.email}>`;
};

export const useUserData = () => {
    const [users, setUsers] = useState<User[]>([]);

    useEffect(() => {
        fetchUsers().then(setUsers);
    }, []);

    return users;
};
"#;

        let p = plugin();
        let symbols = p.extract_symbols(content).unwrap();
        let imports = p.extract_imports(content).unwrap();
        let info = p.file_info(content).unwrap();

        // Verify interface
        let user_interface = symbols.iter().find(|s| s.name == "User").unwrap();
        assert_eq!(user_interface.kind, SymbolKind::Interface);

        // Verify type alias
        let user_role = symbols.iter().find(|s| s.name == "UserRole").unwrap();
        assert_eq!(user_role.kind, SymbolKind::Type);

        // Verify class with decorator
        let user_service = symbols.iter().find(|s| s.name == "UserService").unwrap();
        assert_eq!(user_service.kind, SymbolKind::Class);
        assert!(user_service.signature.contains("@Injectable"));

        // Verify imports
        assert!(imports.iter().any(|i| i.module == "@angular/core"));
        assert!(imports.iter().any(|i| i.module == "rxjs"));

        // Verify file info
        assert_eq!(info.metadata.get("framework"), Some(&"Angular".to_string()));
        assert!(info.metadata.get("interfaces").is_some());

        // Verify concept types
        let is_valid = symbols.iter().find(|s| s.name == "isValidEmail").unwrap();
        assert_eq!(
            p.infer_concept_type(is_valid, content),
            ConceptType::Validation
        );

        let calc_age = symbols.iter().find(|s| s.name == "calculateAge").unwrap();
        assert_eq!(
            p.infer_concept_type(calc_age, content),
            ConceptType::Calculation
        );

        // Verify React hook detection
        let use_user_data = symbols.iter().find(|s| s.name == "useUserData").unwrap();
        assert_eq!(
            p.infer_concept_type(use_user_data, content),
            ConceptType::Infrastructure
        );
    }
}
