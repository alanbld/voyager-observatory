//! Python Language Adapter
//!
//! Extracts structural information from Python source files using Tree-sitter.
//! Supports functions (def/async def), classes, imports, decorators, and docstrings.

use super::{find_child_by_kind, node_text, node_to_span, LanguageAdapter};
use crate::ir::{
    Block, Call, Comment, CommentKind, ControlFlow, ControlFlowKind, Declaration,
    DeclarationKind, ImportKind, ImportLike, LanguageId, Parameter, Span, Visibility,
};

/// Python language adapter using Tree-sitter
pub struct PythonTreeSitterAdapter {
    language: tree_sitter::Language,
}

impl PythonTreeSitterAdapter {
    /// Create a new Python adapter
    pub fn new() -> Self {
        Self {
            language: tree_sitter_python::LANGUAGE.into(),
        }
    }
}

impl Default for PythonTreeSitterAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAdapter for PythonTreeSitterAdapter {
    fn language(&self) -> LanguageId {
        LanguageId::Python
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        self.language.clone()
    }

    fn extract_declarations(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            // Handle decorated definitions
            if child.kind() == "decorated_definition" {
                if let Some(decl) = self.extract_decorated_definition(&child, source) {
                    declarations.push(decl);
                }
            } else if let Some(decl) = self.extract_declaration(&child, source) {
                declarations.push(decl);
            }
        }

        declarations
    }

    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<ImportLike> {
        let mut imports = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "import_statement" => {
                    imports.extend(self.extract_import_statement(&child, source));
                }
                "import_from_statement" => {
                    imports.extend(self.extract_import_from_statement(&child, source));
                }
                _ => {}
            }
        }

        imports
    }

    fn extract_comments(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Comment> {
        let mut comments = Vec::new();
        self.visit_comments(&tree.root_node(), source, &mut comments);
        comments
    }

    fn extract_body(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        declaration: &Declaration,
    ) -> Option<Block> {
        let target_start = declaration.span.start;
        let target_end = declaration.span.end;

        let root = tree.root_node();
        let node = self.find_matching_descendant(&root, target_start, target_end)?;

        // Extract the body block
        let body_node = match node.kind() {
            "function_definition" | "class_definition" => {
                find_child_by_kind(&node, "block")
            }
            _ => None,
        }?;

        Some(self.extract_block(&body_node, source))
    }

    fn extract_visibility(&self, node: &tree_sitter::Node, source: &str) -> Visibility {
        // Python uses naming conventions for visibility
        // _name = protected/internal
        // __name = private (name mangling)
        // __name__ = dunder/magic (public)
        // name = public

        if let Some(name) = self.extract_name(node, source) {
            if name.starts_with("__") && name.ends_with("__") {
                // Dunder methods are public
                Visibility::Public
            } else if name.starts_with("__") {
                // Double underscore = private
                Visibility::Private
            } else if name.starts_with('_') {
                // Single underscore = protected/internal
                Visibility::Protected
            } else {
                Visibility::Public
            }
        } else {
            Visibility::Public
        }
    }
}

impl PythonTreeSitterAdapter {
    /// Extract a declaration from a node
    fn extract_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let kind = node.kind();
        let decl_kind = match kind {
            "function_definition" => DeclarationKind::Function,
            "class_definition" => DeclarationKind::Class,
            _ => return None,
        };

        let name = self.extract_name(node, source)?;
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, decl_kind, span);
        decl.visibility = visibility;

        // Extract docstring as doc comment
        decl.doc_comment = self.extract_docstring(node, source);

        // Extract signature span
        if let Some(sig_span) = self.extract_signature_span(node) {
            decl.signature_span = Some(sig_span);
        }

        // Extract body span
        if let Some(body_span) = self.extract_body_span(node) {
            decl.body_span = Some(body_span);
        }

        // Extract parameters for functions
        if decl_kind == DeclarationKind::Function {
            decl.parameters = self.extract_parameters(node, source);
            decl.return_type = self.extract_return_type(node, source);
        }

        // Check for async and store in metadata
        if self.is_async_function(node) {
            decl.metadata.insert("async".to_string(), "true".to_string());
        }

        // Extract children for classes
        if decl_kind == DeclarationKind::Class {
            decl.children = self.extract_children(node, source);
        }

        Some(decl)
    }

    /// Extract a decorated definition (function or class with decorators)
    fn extract_decorated_definition(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        // Find the actual definition inside
        let mut cursor = node.walk();
        let mut decorators = Vec::new();
        let mut definition_node = None;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "decorator" => {
                    decorators.push(node_text(&child, source).to_string());
                }
                "function_definition" | "class_definition" => {
                    definition_node = Some(child);
                }
                _ => {}
            }
        }

        let def_node = definition_node?;
        let mut decl = self.extract_declaration(&def_node, source)?;

        // Add decorators to metadata
        if !decorators.is_empty() {
            decl.metadata.insert("decorators".to_string(), decorators.join(", "));
        }

        // Update span to include decorators
        decl.span = node_to_span(node);

        Some(decl)
    }

    /// Extract the name of a declaration
    fn extract_name(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        match node.kind() {
            "function_definition" | "class_definition" => {
                // Look for the 'name' child (identifier)
                if let Some(name_node) = find_child_by_kind(node, "identifier") {
                    return Some(node_text(&name_node, source).to_string());
                }
                None
            }
            _ => None,
        }
    }

    /// Extract docstring from a function or class
    fn extract_docstring(&self, node: &tree_sitter::Node, source: &str) -> Option<Comment> {
        // In Python, docstrings are the first statement in the block
        // They are expression_statement containing a string
        let block = find_child_by_kind(node, "block")?;

        let mut cursor = block.walk();
        for child in block.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                let mut expr_cursor = child.walk();
                for expr_child in child.children(&mut expr_cursor) {
                    if expr_child.kind() == "string" {
                        let text = node_text(&expr_child, source);
                        // Clean up the docstring (remove quotes)
                        let cleaned = self.clean_docstring(text);
                        if !cleaned.is_empty() {
                            return Some(Comment {
                                text: cleaned,
                                kind: CommentKind::Doc,
                                span: node_to_span(&expr_child),
                                attached_to: None,
                            });
                        }
                    }
                }
            }
            // Only check the first statement
            break;
        }

        None
    }

    /// Clean up a Python docstring (remove quotes and common indentation)
    fn clean_docstring(&self, text: &str) -> String {
        let text = text
            .trim_start_matches("\"\"\"")
            .trim_start_matches("'''")
            .trim_end_matches("\"\"\"")
            .trim_end_matches("'''")
            .trim_start_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('"')
            .trim_end_matches('\'');

        // Remove common leading whitespace
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return text.trim().to_string();
        }

        // Find minimum indentation (ignoring empty lines)
        let min_indent = lines.iter()
            .filter(|l| !l.trim().is_empty())
            .skip(1) // Skip first line which may not be indented
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // Remove that indentation from each line
        let cleaned: Vec<String> = lines.iter()
            .enumerate()
            .map(|(i, l)| {
                if i == 0 {
                    l.trim().to_string()
                } else if l.len() >= min_indent {
                    l[min_indent..].to_string()
                } else {
                    l.to_string()
                }
            })
            .collect();

        cleaned.join("\n").trim().to_string()
    }

    /// Check if a function is async
    fn is_async_function(&self, node: &tree_sitter::Node) -> bool {
        if node.kind() != "function_definition" {
            return false;
        }

        // Check if there's an 'async' keyword before 'def'
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "async" {
                return true;
            }
        }

        // Also check the parent for decorated async functions
        if let Some(parent) = node.parent() {
            if parent.kind() == "decorated_definition" {
                let mut pcursor = parent.walk();
                for pchild in parent.children(&mut pcursor) {
                    if pchild.kind() == "async" {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extract function parameters
    fn extract_parameters(&self, node: &tree_sitter::Node, source: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = find_child_by_kind(node, "parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        // Simple parameter
                        params.push(Parameter {
                            name: node_text(&child, source).to_string(),
                            type_annotation: None,
                            default_value: None,
                            span: node_to_span(&child),
                        });
                    }
                    "typed_parameter" => {
                        if let Some(param) = self.extract_typed_parameter(&child, source) {
                            params.push(param);
                        }
                    }
                    "default_parameter" => {
                        if let Some(param) = self.extract_default_parameter(&child, source) {
                            params.push(param);
                        }
                    }
                    "typed_default_parameter" => {
                        if let Some(param) = self.extract_typed_default_parameter(&child, source) {
                            params.push(param);
                        }
                    }
                    "list_splat_pattern" | "dictionary_splat_pattern" => {
                        // *args or **kwargs
                        let text = node_text(&child, source);
                        params.push(Parameter {
                            name: text.to_string(),
                            type_annotation: None,
                            default_value: None,
                            span: node_to_span(&child),
                        });
                    }
                    _ => {}
                }
            }
        }

        params
    }

    /// Extract a typed parameter (name: type)
    fn extract_typed_parameter(&self, node: &tree_sitter::Node, source: &str) -> Option<Parameter> {
        let mut name = None;
        let mut type_annotation = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    name = Some(node_text(&child, source).to_string());
                }
                "type" => {
                    type_annotation = Some(node_text(&child, source).to_string());
                }
                _ => {}
            }
        }

        name.map(|n| Parameter {
            name: n,
            type_annotation,
            default_value: None,
            span: node_to_span(node),
        })
    }

    /// Extract a default parameter (name=value)
    fn extract_default_parameter(&self, node: &tree_sitter::Node, source: &str) -> Option<Parameter> {
        let mut name = None;
        let mut default_value = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" if name.is_none() => {
                    name = Some(node_text(&child, source).to_string());
                }
                _ if name.is_some() && default_value.is_none() && child.kind() != "=" => {
                    default_value = Some(node_text(&child, source).to_string());
                }
                _ => {}
            }
        }

        name.map(|n| Parameter {
            name: n,
            type_annotation: None,
            default_value,
            span: node_to_span(node),
        })
    }

    /// Extract a typed default parameter (name: type = value)
    fn extract_typed_default_parameter(&self, node: &tree_sitter::Node, source: &str) -> Option<Parameter> {
        let mut name = None;
        let mut type_annotation = None;
        let mut default_value = None;
        let mut found_equals = false;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" if name.is_none() => {
                    name = Some(node_text(&child, source).to_string());
                }
                "type" => {
                    type_annotation = Some(node_text(&child, source).to_string());
                }
                "=" => {
                    found_equals = true;
                }
                _ if found_equals && default_value.is_none() => {
                    default_value = Some(node_text(&child, source).to_string());
                }
                _ => {}
            }
        }

        name.map(|n| Parameter {
            name: n,
            type_annotation,
            default_value,
            span: node_to_span(node),
        })
    }

    /// Extract return type annotation
    fn extract_return_type(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        // Look for return_type child (-> type)
        if let Some(return_type) = find_child_by_kind(node, "type") {
            return Some(node_text(&return_type, source).to_string());
        }
        None
    }

    /// Extract signature span (up to the colon before the body)
    fn extract_signature_span(&self, node: &tree_sitter::Node) -> Option<Span> {
        let block = find_child_by_kind(node, "block")?;

        Some(Span {
            start: node.start_byte(),
            end: block.start_byte(),
            start_line: node.start_position().row + 1,
            end_line: block.start_position().row + 1,
            start_column: node.start_position().column,
            end_column: block.start_position().column,
        })
    }

    /// Extract body span
    fn extract_body_span(&self, node: &tree_sitter::Node) -> Option<Span> {
        let block = find_child_by_kind(node, "block")?;
        Some(node_to_span(&block))
    }

    /// Extract children (methods in class)
    fn extract_children(&self, node: &tree_sitter::Node, source: &str) -> Vec<Declaration> {
        let mut children = Vec::new();

        if let Some(block) = find_child_by_kind(node, "block") {
            let mut cursor = block.walk();
            for child in block.children(&mut cursor) {
                match child.kind() {
                    "function_definition" => {
                        if let Some(mut decl) = self.extract_declaration(&child, source) {
                            decl.kind = DeclarationKind::Method;
                            children.push(decl);
                        }
                    }
                    "decorated_definition" => {
                        if let Some(mut decl) = self.extract_decorated_definition(&child, source) {
                            // Check if it's a method
                            if decl.kind == DeclarationKind::Function {
                                decl.kind = DeclarationKind::Method;
                            }
                            children.push(decl);
                        }
                    }
                    "expression_statement" => {
                        // Check for class-level assignments (class variables)
                        if let Some(decl) = self.extract_class_variable(&child, source) {
                            children.push(decl);
                        }
                    }
                    _ => {}
                }
            }
        }

        children
    }

    /// Extract a class variable from an expression statement
    fn extract_class_variable(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "assignment" {
                // Get the left-hand side (variable name)
                if let Some(left) = find_child_by_kind(&child, "identifier") {
                    let name = node_text(&left, source).to_string();
                    let span = node_to_span(&child);
                    let visibility = if name.starts_with('_') {
                        Visibility::Private
                    } else {
                        Visibility::Public
                    };

                    let mut decl = Declaration::new(name, DeclarationKind::Variable, span);
                    decl.visibility = visibility;
                    return Some(decl);
                }
            }
        }
        None
    }

    /// Extract an import statement (import x, y, z)
    fn extract_import_statement(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Vec<ImportLike> {
        let mut imports = Vec::new();
        let span = node_to_span(node);

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "dotted_name" => {
                    let name = node_text(&child, source).to_string();
                    imports.push(ImportLike {
                        source: name.clone(),
                        kind: ImportKind::Import,
                        items: vec![name],
                        alias: None,
                        type_only: false,
                        span,
                    });
                }
                "aliased_import" => {
                    let (name, alias) = self.extract_aliased_import(&child, source);
                    if let Some(name) = name {
                        imports.push(ImportLike {
                            source: name.clone(),
                            kind: ImportKind::Import,
                            items: vec![name],
                            alias,
                            type_only: false,
                            span,
                        });
                    }
                }
                _ => {}
            }
        }

        imports
    }

    /// Extract an aliased import (x as y)
    fn extract_aliased_import(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> (Option<String>, Option<String>) {
        let mut name = None;
        let mut alias = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "dotted_name" if name.is_none() => {
                    name = Some(node_text(&child, source).to_string());
                }
                "identifier" if name.is_some() && alias.is_none() => {
                    alias = Some(node_text(&child, source).to_string());
                }
                _ => {}
            }
        }

        (name, alias)
    }

    /// Extract a from...import statement (from x import a, b, c)
    fn extract_import_from_statement(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Vec<ImportLike> {
        let span = node_to_span(node);

        // Get the module path (after 'from')
        let module_source = self.extract_module_path(node, source).unwrap_or_else(|| ".".to_string());

        // Check for wildcard import and collect items
        let mut cursor = node.walk();
        let mut is_wildcard = false;
        let mut items = Vec::new();

        for child in node.children(&mut cursor) {
            if child.kind() == "wildcard_import" {
                is_wildcard = true;
            } else if child.kind() == "import_prefix" || child.kind() == "from" || child.kind() == "import" || child.kind() == "dotted_name" || child.kind() == "relative_import" {
                continue;
            } else if child.kind() == "identifier" {
                items.push(node_text(&child, source).to_string());
            } else if child.kind() == "aliased_import" {
                let (name, _alias) = self.extract_aliased_import(&child, source);
                if let Some(n) = name {
                    items.push(n);
                }
            }
        }

        if is_wildcard {
            items.push("*".to_string());
        }

        vec![ImportLike {
            source: module_source,
            kind: ImportKind::From,
            items,
            alias: None,
            type_only: false,
            span,
        }]
    }

    /// Extract the module path from a from...import statement
    fn extract_module_path(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        let mut cursor = node.walk();
        let mut relative_dots = String::new();
        let mut module_name = None;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "relative_import" => {
                    // Handle relative imports like "from . import x" or "from .. import x"
                    let text = node_text(&child, source);
                    relative_dots = text.chars().take_while(|&c| c == '.').collect();
                    // The rest might be a module name
                    let after_dots: String = text.chars().skip_while(|&c| c == '.').collect();
                    if !after_dots.is_empty() {
                        module_name = Some(after_dots);
                    }
                }
                "import_prefix" => {
                    // Relative import prefix (dots only)
                    relative_dots = node_text(&child, source).to_string();
                }
                "dotted_name" if module_name.is_none() => {
                    module_name = Some(node_text(&child, source).to_string());
                }
                "from" | "import" => continue,
                _ => {}
            }
        }

        match (relative_dots.is_empty(), module_name) {
            (true, Some(name)) => Some(name),
            (false, Some(name)) => Some(format!("{}{}", relative_dots, name)),
            (false, None) => Some(relative_dots),
            (true, None) => None,
        }
    }

    /// Visit comments in the tree
    fn visit_comments(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        comments: &mut Vec<Comment>,
    ) {
        if node.kind() == "comment" {
            let text = node_text(node, source);
            let cleaned = text.trim_start_matches('#').trim();

            comments.push(Comment {
                text: cleaned.to_string(),
                kind: CommentKind::Line,
                span: node_to_span(node),
                attached_to: None,
            });
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_comments(&child, source, comments);
        }
    }

    /// Find a node matching the given byte range
    fn find_matching_descendant<'a>(
        &self,
        root: &tree_sitter::Node<'a>,
        start: usize,
        end: usize,
    ) -> Option<tree_sitter::Node<'a>> {
        let mut cursor = root.walk();

        loop {
            let node = cursor.node();

            if node.start_byte() == start && node.end_byte() == end {
                return Some(node);
            }

            // Try to go deeper
            if cursor.goto_first_child() {
                continue;
            }

            // Try next sibling
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return None;
                }
            }
        }
    }

    /// Extract a body block
    fn extract_block(&self, node: &tree_sitter::Node, source: &str) -> Block {
        let mut control_flow = Vec::new();
        let mut calls = Vec::new();
        let mut nested_declarations = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_block_contents(&child, source, &mut control_flow, &mut calls, &mut nested_declarations);
        }

        Block {
            span: node_to_span(node),
            control_flow,
            calls,
            comments: vec![],
            unknown_regions: vec![],
            nested_declarations,
        }
    }

    /// Recursively extract block contents
    fn extract_block_contents(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        control_flow: &mut Vec<ControlFlow>,
        calls: &mut Vec<Call>,
        nested_declarations: &mut Vec<Declaration>,
    ) {
        match node.kind() {
            // Control flow statements
            "if_statement" => {
                let condition_span = self.extract_condition_span(node);
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::If,
                    span: node_to_span(node),
                    condition_span,
                    branches: vec![],
                });
            }
            "for_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::For,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "while_statement" => {
                let condition_span = self.extract_condition_span(node);
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::While,
                    span: node_to_span(node),
                    condition_span,
                    branches: vec![],
                });
            }
            "try_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Try,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "except_clause" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Catch,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "finally_clause" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Finally,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "with_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::With,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "match_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Match,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "return_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Return,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }

            // Function calls
            "call" => {
                if let Some(call) = self.extract_call(node, source) {
                    calls.push(call);
                }
            }

            // Nested function definitions (lambdas, inner functions)
            "function_definition" => {
                if let Some(decl) = self.extract_declaration(node, source) {
                    nested_declarations.push(decl);
                }
            }

            _ => {
                // Recurse into children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_block_contents(&child, source, control_flow, calls, nested_declarations);
                }
            }
        }
    }

    /// Extract condition span from if/while
    fn extract_condition_span(&self, node: &tree_sitter::Node) -> Option<Span> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Skip keywords
            if child.kind() == "if" || child.kind() == "elif" || child.kind() == "while" {
                continue;
            }
            if child.kind() == "block" || child.kind() == ":" {
                break;
            }
            // This should be the condition expression
            if !child.kind().is_empty() {
                return Some(node_to_span(&child));
            }
        }
        None
    }

    /// Extract a function call
    fn extract_call(&self, node: &tree_sitter::Node, source: &str) -> Option<Call> {
        // call -> function (arguments)
        let function = find_child_by_kind(node, "attribute")
            .or_else(|| find_child_by_kind(node, "identifier"))?;

        let callee = node_text(&function, source).to_string();
        let is_method = function.kind() == "attribute";

        // Count arguments
        let argument_count = if let Some(args) = find_child_by_kind(node, "argument_list") {
            let mut count = 0;
            let mut cursor = args.walk();
            for child in args.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    count += 1;
                }
            }
            count
        } else {
            0
        };

        Some(Call {
            callee,
            span: node_to_span(node),
            argument_count,
            is_method,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_python(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_extract_simple_function() {
        let source = r#"
def hello(name: str) -> str:
    """Greet someone."""
    return f"Hello, {name}!"
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "hello");
        assert_eq!(func.kind, DeclarationKind::Function);
        assert!(func.doc_comment.is_some());
        assert_eq!(func.doc_comment.as_ref().unwrap().text, "Greet someone.");
        assert_eq!(func.parameters.len(), 1);
        assert_eq!(func.parameters[0].name, "name");
        assert_eq!(func.parameters[0].type_annotation, Some("str".to_string()));
        assert_eq!(func.return_type, Some("str".to_string()));
    }

    #[test]
    fn test_extract_async_function() {
        let source = r#"
async def fetch_data(url: str) -> dict:
    """Fetch data from URL."""
    pass
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "fetch_data");
        assert_eq!(func.metadata.get("async"), Some(&"true".to_string()));
    }

    #[test]
    fn test_extract_class() {
        let source = r#"
class MyClass:
    """A sample class."""

    class_var = 10

    def __init__(self, name):
        self.name = name

    def greet(self):
        return f"Hello, {self.name}"
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let class = &declarations[0];
        assert_eq!(class.name, "MyClass");
        assert_eq!(class.kind, DeclarationKind::Class);
        assert!(class.doc_comment.is_some());

        // Should have methods and class variable as children
        assert!(class.children.len() >= 2, "Expected at least 2 children, got {}", class.children.len());
    }

    #[test]
    fn test_extract_decorated_function() {
        let source = r#"
@staticmethod
@cache
def compute(x, y):
    return x + y
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "compute");
        assert!(func.metadata.get("decorators").is_some());
        let decorators = func.metadata.get("decorators").unwrap();
        assert!(decorators.contains("@staticmethod"));
        assert!(decorators.contains("@cache"));
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
import os
import json as js
from pathlib import Path
from typing import List, Optional
from . import sibling
from ..utils import helper
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert!(imports.len() >= 4, "Expected at least 4 imports, got {}", imports.len());

        // Check for 'os' import
        assert!(imports.iter().any(|i| i.source == "os"));

        // Check for 'json as js' import
        assert!(imports.iter().any(|i| i.source == "json" && i.alias == Some("js".to_string())));
    }

    #[test]
    fn test_visibility_conventions() {
        let source = r#"
def public_func():
    pass

def _protected_func():
    pass

def __private_func():
    pass

def __dunder_method__():
    pass
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 4);

        let public = declarations.iter().find(|d| d.name == "public_func").unwrap();
        assert_eq!(public.visibility, Visibility::Public);

        let protected = declarations.iter().find(|d| d.name == "_protected_func").unwrap();
        assert_eq!(protected.visibility, Visibility::Protected);

        let private = declarations.iter().find(|d| d.name == "__private_func").unwrap();
        assert_eq!(private.visibility, Visibility::Private);

        let dunder = declarations.iter().find(|d| d.name == "__dunder_method__").unwrap();
        assert_eq!(dunder.visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_function_with_defaults() {
        let source = r#"
def greet(name: str = "World", times: int = 1):
    pass
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.parameters.len(), 2);

        let name_param = &func.parameters[0];
        assert_eq!(name_param.name, "name");
        assert!(name_param.type_annotation.is_some());
        assert!(name_param.default_value.is_some());
    }

    #[test]
    fn test_adapter_language() {
        let adapter = PythonTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::Python);
    }
}
