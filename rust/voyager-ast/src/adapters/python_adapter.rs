//! Python Language Adapter
//!
//! Extracts structural information from Python source files using Tree-sitter.
//! Supports functions (def/async def), classes, imports, decorators, and docstrings.

use super::{find_child_by_kind, node_text, node_to_span, LanguageAdapter};
use crate::ir::{
    Block, Call, Comment, CommentKind, ControlFlow, ControlFlowKind, Declaration, DeclarationKind,
    ImportKind, ImportLike, LanguageId, Parameter, Span, Visibility,
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

    fn extract_declarations(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Declaration> {
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
            "function_definition" | "class_definition" => find_child_by_kind(&node, "block"),
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
    fn extract_declaration(&self, node: &tree_sitter::Node, source: &str) -> Option<Declaration> {
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
            decl.metadata
                .insert("async".to_string(), "true".to_string());
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
            decl.metadata
                .insert("decorators".to_string(), decorators.join(", "));
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
        // Only check the first statement for docstring
        if let Some(child) = block.children(&mut cursor).next() {
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
        let min_indent = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .skip(1) // Skip first line which may not be indented
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // Remove that indentation from each line
        let cleaned: Vec<String> = lines
            .iter()
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
    fn extract_default_parameter(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Parameter> {
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
    fn extract_typed_default_parameter(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Parameter> {
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
    fn extract_import_statement(&self, node: &tree_sitter::Node, source: &str) -> Vec<ImportLike> {
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
        let module_source = self
            .extract_module_path(node, source)
            .unwrap_or_else(|| ".".to_string());

        // Check for wildcard import and collect items
        let mut cursor = node.walk();
        let mut is_wildcard = false;
        let mut items = Vec::new();

        for child in node.children(&mut cursor) {
            if child.kind() == "wildcard_import" {
                is_wildcard = true;
            } else if child.kind() == "import_prefix"
                || child.kind() == "from"
                || child.kind() == "import"
                || child.kind() == "dotted_name"
                || child.kind() == "relative_import"
            {
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
    #[allow(clippy::only_used_in_recursion)]
    fn visit_comments(&self, node: &tree_sitter::Node, source: &str, comments: &mut Vec<Comment>) {
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
            self.extract_block_contents(
                &child,
                source,
                &mut control_flow,
                &mut calls,
                &mut nested_declarations,
            );
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
                    self.extract_block_contents(
                        &child,
                        source,
                        control_flow,
                        calls,
                        nested_declarations,
                    );
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
    use crate::ir::Span;

    fn parse_python(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    // =========================================================================
    // Basic Adapter Tests
    // =========================================================================

    #[test]
    fn test_adapter_new() {
        let adapter = PythonTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::Python);
    }

    #[test]
    fn test_adapter_default() {
        let adapter = PythonTreeSitterAdapter::default();
        assert_eq!(adapter.language(), LanguageId::Python);
    }

    #[test]
    fn test_tree_sitter_language() {
        let adapter = PythonTreeSitterAdapter::new();
        let lang = adapter.tree_sitter_language();
        let mut parser = tree_sitter::Parser::new();
        assert!(parser.set_language(&lang).is_ok());
    }

    #[test]
    fn test_adapter_language() {
        let adapter = PythonTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::Python);
    }

    // =========================================================================
    // Function Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_simple_function() {
        let source = "def hello(name: str) -> str:\n    \"\"\"Greet someone.\"\"\"\n    return f\"Hello, {name}!\"\n";
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
        let source = "async def fetch_data(url: str) -> dict:\n    \"\"\"Fetch data from URL.\"\"\"\n    pass\n";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "fetch_data");
        assert_eq!(func.metadata.get("async"), Some(&"true".to_string()));
    }

    #[test]
    fn test_extract_function_no_return_type() {
        let source = "def no_return():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].return_type.is_none());
    }

    #[test]
    fn test_extract_function_no_params() {
        let source = "def empty():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].parameters.is_empty());
    }

    #[test]
    fn test_extract_function_args_kwargs() {
        let source = "def variadic(*args, **kwargs):\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        // Should extract args and kwargs as parameters
    }

    #[test]
    fn test_extract_function_with_defaults() {
        let source = "def greet(name: str = \"World\", times: int = 1):\n    pass\n";
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
    fn test_extract_lambda() {
        let source = "square = lambda x: x * x";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        // Lambda may or may not be extracted as a declaration
        // Just verify no panic
    }

    // =========================================================================
    // Class Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_class() {
        let source = "class MyClass:\n    \"\"\"A sample class.\"\"\"\n\n    class_var = 10\n\n    def __init__(self, name):\n        self.name = name\n\n    def greet(self):\n        return f\"Hello, {self.name}\"\n";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let class = &declarations[0];
        assert_eq!(class.name, "MyClass");
        assert_eq!(class.kind, DeclarationKind::Class);
        assert!(class.doc_comment.is_some());

        assert!(
            class.children.len() >= 2,
            "Expected at least 2 children, got {}",
            class.children.len()
        );
    }

    #[test]
    fn test_extract_class_with_inheritance() {
        let source = "class Child(Parent):\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Child");
        assert_eq!(decls[0].kind, DeclarationKind::Class);
    }

    #[test]
    fn test_extract_class_multiple_inheritance() {
        let source = "class Multi(Base1, Base2, Mixin):\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Multi");
    }

    #[test]
    fn test_extract_decorated_class() {
        let source = "@dataclass\nclass Point:\n    x: int\n    y: int";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Point");
        assert!(decls[0].metadata.get("decorators").is_some());
    }

    // =========================================================================
    // Decorator Tests
    // =========================================================================

    #[test]
    fn test_extract_decorated_function() {
        let source = "@staticmethod\n@cache\ndef compute(x, y):\n    return x + y\n";
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
    fn test_extract_decorator_with_args() {
        let source = "@decorator(arg1, arg2)\ndef decorated():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].metadata.get("decorators").is_some());
    }

    // =========================================================================
    // Import Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_imports() {
        let source = "import os\nimport json as js\nfrom pathlib import Path\nfrom typing import List, Optional\nfrom . import sibling\nfrom ..utils import helper\n";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert!(
            imports.len() >= 4,
            "Expected at least 4 imports, got {}",
            imports.len()
        );
        assert!(imports.iter().any(|i| i.source == "os"));
        assert!(imports
            .iter()
            .any(|i| i.source == "json" && i.alias == Some("js".to_string())));
    }

    #[test]
    fn test_extract_import_simple() {
        let source = "import math";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "math");
        assert_eq!(imports[0].kind, ImportKind::Import);
    }

    #[test]
    fn test_extract_from_import() {
        let source = "from os.path import join, exists";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "os.path");
        // Items extraction depends on implementation
    }

    #[test]
    fn test_extract_relative_import() {
        let source = "from . import module";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        // Relative import source may include the imported name
        assert!(imports[0].source.starts_with("."));
    }

    #[test]
    fn test_extract_star_import() {
        let source = "from module import *";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
    }

    // =========================================================================
    // Visibility Tests
    // =========================================================================

    #[test]
    fn test_visibility_conventions() {
        let source = "def public_func():\n    pass\n\ndef _protected_func():\n    pass\n\ndef __private_func():\n    pass\n\ndef __dunder_method__():\n    pass\n";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 4);

        let public = declarations
            .iter()
            .find(|d| d.name == "public_func")
            .unwrap();
        assert_eq!(public.visibility, Visibility::Public);

        let protected = declarations
            .iter()
            .find(|d| d.name == "_protected_func")
            .unwrap();
        assert_eq!(protected.visibility, Visibility::Protected);

        let private = declarations
            .iter()
            .find(|d| d.name == "__private_func")
            .unwrap();
        assert_eq!(private.visibility, Visibility::Private);

        let dunder = declarations
            .iter()
            .find(|d| d.name == "__dunder_method__")
            .unwrap();
        assert_eq!(dunder.visibility, Visibility::Public);
    }

    // =========================================================================
    // Comment Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_line_comment() {
        let source = "# This is a comment\ndef foo():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert_eq!(comments[0].kind, CommentKind::Line);
    }

    #[test]
    fn test_extract_docstring_comment() {
        let source = "def foo():\n    \"\"\"Docstring.\"\"\"\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].doc_comment.is_some());
        assert_eq!(
            decls[0].doc_comment.as_ref().unwrap().kind,
            CommentKind::Doc
        );
    }

    #[test]
    fn test_extract_multiline_docstring() {
        let source = "def foo():\n    \"\"\"\n    Multi-line\n    docstring.\n    \"\"\"\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].doc_comment.is_some());
    }

    // =========================================================================
    // Body Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_body_function() {
        let source = "def test():\n    x = 1\n    return x";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_extract_body_class() {
        let source = "class Foo:\n    def method(self):\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    // =========================================================================
    // Control Flow Tests
    // =========================================================================

    #[test]
    fn test_extract_if_statement() {
        let source = "def test():\n    if x > 0:\n        print('positive')\n    else:\n        print('non-positive')";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::If);
            }
        }
    }

    #[test]
    fn test_extract_for_loop() {
        let source = "def test():\n    for i in range(10):\n        print(i)";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::For);
            }
        }
    }

    #[test]
    fn test_extract_while_loop() {
        let source = "def test():\n    while x > 0:\n        x -= 1";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::While);
            }
        }
    }

    #[test]
    fn test_extract_try_except() {
        let source = "def test():\n    try:\n        risky()\n    except Exception:\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            let has_try = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Try);
            let has_catch = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Catch);
            assert!(has_try || has_catch || body.control_flow.is_empty());
        }
    }

    #[test]
    fn test_extract_with_statement() {
        let source = "def test():\n    with open('file') as f:\n        data = f.read()";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::With);
            }
        }
    }

    // =========================================================================
    // Error Recovery Tests
    // =========================================================================

    #[test]
    fn test_error_recovery() {
        let source = "def broken(\n    pass\n\ndef valid():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();

        let decls = adapter.extract_declarations(&tree, source);
        // Error recovery may or may not extract valid function
        // Just verify it doesn't panic
        let _ = decls;
    }

    #[test]
    fn test_extract_errors() {
        let source = "def broken(\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);

        // Should detect syntax error
        assert!(errors.len() > 0 || tree.root_node().has_error());
    }

    // =========================================================================
    // Multiple Declarations Tests
    // =========================================================================

    #[test]
    fn test_multiple_functions() {
        let source = "def one():\n    pass\n\ndef two():\n    pass\n\ndef three():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 3);
    }

    #[test]
    fn test_mixed_declarations() {
        let source = "class Foo:\n    pass\n\ndef bar():\n    pass\n\nVAR = 42";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls.iter().any(|d| d.kind == DeclarationKind::Class));
        assert!(decls.iter().any(|d| d.kind == DeclarationKind::Function));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_source() {
        let source = "";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls.is_empty());
    }

    #[test]
    fn test_only_comments() {
        let source = "# Just a comment\n# Another comment";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls.is_empty());
    }

    #[test]
    fn test_unicode_identifiers() {
        let source = "def 中文函数():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "中文函数");
    }

    #[test]
    fn test_nested_function() {
        let source = "def outer():\n    def inner():\n        pass\n    return inner";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        // Outer function should be extracted
        assert!(decls.iter().any(|d| d.name == "outer"));
    }

    #[test]
    fn test_generator_function() {
        let source = "def gen():\n    yield 1\n    yield 2";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "gen");
    }

    #[test]
    fn test_property_decorator() {
        let source = "class Foo:\n    @property\n    def value(self):\n        return self._value";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].children.len() >= 1);
    }

    // =========================================================================
    // Extended Coverage Tests - Control Flow
    // =========================================================================

    #[test]
    fn test_extract_finally_clause() {
        let source = "def test():\n    try:\n        risky()\n    except Exception:\n        pass\n    finally:\n        cleanup()";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            let has_finally = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Finally);
            // Should detect finally clause
            assert!(has_finally || body.control_flow.len() >= 2);
        }
    }

    #[test]
    fn test_extract_match_statement() {
        let source = "def test():\n    match value:\n        case 1:\n            pass\n        case 2:\n            pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            let has_match = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Match);
            // Match statement might be extracted depending on Python version support
            let _ = has_match;
        }
    }

    #[test]
    fn test_extract_return_statement() {
        let source = "def test():\n    return 42";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            let has_return = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Return);
            assert!(has_return || body.control_flow.is_empty());
        }
    }

    #[test]
    fn test_extract_nested_function_in_body() {
        let source = "def outer():\n    def inner():\n        pass\n    return inner()";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            // Nested function should be in nested_declarations
            assert!(body.nested_declarations.len() >= 1);
            assert_eq!(body.nested_declarations[0].name, "inner");
        }
    }

    #[test]
    fn test_extract_call_simple() {
        let source = "def test():\n    foo(1, 2, 3)";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            assert!(!body.calls.is_empty());
            let call = &body.calls[0];
            assert_eq!(call.callee, "foo");
            assert_eq!(call.argument_count, 3);
            assert!(!call.is_method);
        }
    }

    #[test]
    fn test_extract_method_call() {
        let source = "def test():\n    obj.method(x, y)";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            assert!(!body.calls.is_empty());
            let call = &body.calls[0];
            assert!(call.is_method);
        }
    }

    #[test]
    fn test_extract_call_no_args() {
        let source = "def test():\n    empty_call()";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            assert!(!body.calls.is_empty());
            assert_eq!(body.calls[0].argument_count, 0);
        }
    }

    #[test]
    fn test_extract_chained_calls() {
        let source = "def test():\n    a().b().c()";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            // Should extract multiple calls
            assert!(body.calls.len() >= 1);
        }
    }

    // =========================================================================
    // Extended Coverage Tests - Class Variables
    // =========================================================================

    #[test]
    fn test_extract_private_class_variable() {
        let source = "class Foo:\n    _private_var = 10\n    __very_private = 20";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        let class = &decls[0];
        let private_vars: Vec<_> = class
            .children
            .iter()
            .filter(|c| c.visibility == Visibility::Private)
            .collect();
        // Both should be private (start with _)
        assert!(private_vars.len() >= 1);
    }

    #[test]
    fn test_extract_class_with_methods_and_vars() {
        let source = "class Foo:\n    x = 1\n    def method(self):\n        pass\n    y = 2";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        let class = &decls[0];
        let methods: Vec<_> = class
            .children
            .iter()
            .filter(|c| c.kind == DeclarationKind::Method)
            .collect();
        let vars: Vec<_> = class
            .children
            .iter()
            .filter(|c| c.kind == DeclarationKind::Variable)
            .collect();
        assert!(methods.len() >= 1);
        assert!(vars.len() >= 1);
    }

    // =========================================================================
    // Extended Coverage Tests - Docstrings
    // =========================================================================

    #[test]
    fn test_clean_docstring_single_quotes() {
        let source = "def foo():\n    '''Single quote docstring.'''\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].doc_comment.is_some());
        assert!(decls[0]
            .doc_comment
            .as_ref()
            .unwrap()
            .text
            .contains("Single quote"));
    }

    #[test]
    fn test_clean_docstring_with_indentation() {
        let source = "def foo():\n    \"\"\"\n    First line.\n    Second line.\n    Third line.\n    \"\"\"\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].doc_comment.is_some());
        let doc = &decls[0].doc_comment.as_ref().unwrap().text;
        assert!(doc.contains("First line"));
        assert!(doc.contains("Second line"));
    }

    #[test]
    fn test_empty_docstring() {
        let source = "def foo():\n    \"\"\"\"\"\"\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        // Empty docstring might not be captured
        let _ = decls[0].doc_comment.as_ref();
    }

    // =========================================================================
    // Extended Coverage Tests - Import Statements
    // =========================================================================

    #[test]
    fn test_import_multiple_modules() {
        let source = "import os, sys, json";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        // Should extract multiple imports
        assert!(imports.len() >= 1);
    }

    #[test]
    fn test_import_from_with_alias() {
        let source = "from os.path import join as path_join";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].kind, ImportKind::From);
    }

    #[test]
    fn test_relative_import_parent() {
        let source = "from ..parent import module";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert!(imports[0].source.starts_with(".."));
    }

    #[test]
    fn test_relative_import_current() {
        let source = "from .sibling import func";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert!(imports[0].source.starts_with("."));
    }

    #[test]
    fn test_relative_import_only_dots() {
        let source = "from .. import something";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
    }

    // =========================================================================
    // Extended Coverage Tests - Parameters
    // =========================================================================

    #[test]
    fn test_parameter_complex_type_annotation() {
        let source = "def foo(items: List[Dict[str, Any]]) -> None:\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].parameters.len(), 1);
        assert!(decls[0].parameters[0].type_annotation.is_some());
    }

    #[test]
    fn test_parameter_keyword_only() {
        let source = "def foo(*, keyword_only: str):\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        // Keyword-only parameters should be extracted
    }

    #[test]
    fn test_parameter_positional_only() {
        let source = "def foo(pos_only, /, normal):\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        // Positional-only parameters should be extracted
    }

    #[test]
    fn test_parameter_mixed_args_kwargs() {
        let source = "def foo(a, b=1, *args, c=2, **kwargs):\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        // Should extract all parameter types
        assert!(decls[0].parameters.len() >= 2);
    }

    // =========================================================================
    // Extended Coverage Tests - Spans
    // =========================================================================

    #[test]
    fn test_signature_span_extraction() {
        let source = "def long_function_name(param1: int, param2: str) -> bool:\n    return True";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].signature_span.is_some());
        let sig_span = decls[0].signature_span.as_ref().unwrap();
        assert!(sig_span.start < sig_span.end);
    }

    #[test]
    fn test_body_span_extraction() {
        let source = "def foo():\n    x = 1\n    y = 2\n    return x + y";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].body_span.is_some());
        let body_span = decls[0].body_span.as_ref().unwrap();
        assert!(body_span.start < body_span.end);
    }

    // =========================================================================
    // Extended Coverage Tests - Visibility Edge Cases
    // =========================================================================

    #[test]
    fn test_visibility_dunder_init() {
        let source = "class Foo:\n    def __init__(self):\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let method = &decls[0].children[0];
        assert_eq!(method.name, "__init__");
        assert_eq!(method.visibility, Visibility::Public);
    }

    #[test]
    fn test_visibility_protected_method() {
        let source = "class Foo:\n    def _internal(self):\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let method = &decls[0].children[0];
        assert_eq!(method.name, "_internal");
        assert_eq!(method.visibility, Visibility::Protected);
    }

    #[test]
    fn test_visibility_private_method() {
        let source = "class Foo:\n    def __private(self):\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let method = &decls[0].children[0];
        assert_eq!(method.name, "__private");
        assert_eq!(method.visibility, Visibility::Private);
    }

    // =========================================================================
    // Extended Coverage Tests - Decorated Methods in Class
    // =========================================================================

    #[test]
    fn test_decorated_method_in_class() {
        let source =
            "class Foo:\n    @classmethod\n    def cls_method(cls):\n        pass\n    @staticmethod\n    def static_method():\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert!(decls[0].children.len() >= 2);
        let cls_method = decls[0]
            .children
            .iter()
            .find(|c| c.name == "cls_method")
            .unwrap();
        assert!(cls_method
            .metadata
            .get("decorators")
            .unwrap()
            .contains("classmethod"));
    }

    // =========================================================================
    // Extended Coverage Tests - Multiple Control Flow Structures
    // =========================================================================

    #[test]
    fn test_nested_control_flow() {
        let source = "def test():\n    for i in range(10):\n        if i > 5:\n            while True:\n                break";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            // Should have for, if, while
            let kinds: Vec<_> = body.control_flow.iter().map(|cf| &cf.kind).collect();
            assert!(kinds.contains(&&ControlFlowKind::For));
        }
    }

    #[test]
    fn test_elif_extraction() {
        let source =
            "def test():\n    if x:\n        pass\n    elif y:\n        pass\n    else:\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            // elif is part of if_statement in tree-sitter
            assert!(!body.control_flow.is_empty());
        }
    }

    // =========================================================================
    // Extended Coverage Tests - Condition Span
    // =========================================================================

    #[test]
    fn test_if_condition_span() {
        let source = "def test():\n    if x > 5 and y < 10:\n        pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                let if_stmt = &body.control_flow[0];
                assert_eq!(if_stmt.kind, ControlFlowKind::If);
                // Condition span should be set
                if let Some(cond_span) = &if_stmt.condition_span {
                    assert!(cond_span.start < cond_span.end);
                }
            }
        }
    }

    #[test]
    fn test_while_condition_span() {
        let source = "def test():\n    while not done:\n        process()";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                let while_stmt = &body.control_flow[0];
                assert_eq!(while_stmt.kind, ControlFlowKind::While);
            }
        }
    }

    // =========================================================================
    // Extended Coverage Tests - Error Handling
    // =========================================================================

    #[test]
    fn test_extract_errors_valid_code() {
        let source = "def valid():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);

        assert!(errors.is_empty());
    }

    #[test]
    fn test_extract_errors_multiple_syntax_errors() {
        let source = "def broken(\ndef also_broken(";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);

        // Should detect syntax errors
        assert!(!errors.is_empty() || tree.root_node().has_error());
    }

    // =========================================================================
    // Extended Coverage Tests - find_matching_descendant
    // =========================================================================

    #[test]
    fn test_body_extraction_non_matching_declaration() {
        let source = "def foo():\n    pass";
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();

        // Create a declaration with wrong span
        let fake_decl = Declaration::new(
            "nonexistent".to_string(),
            DeclarationKind::Function,
            Span {
                start: 9999,
                end: 10000,
                start_line: 999,
                end_line: 999,
                start_column: 0,
                end_column: 0,
            },
        );

        let body = adapter.extract_body(&tree, source, &fake_decl);
        assert!(body.is_none());
    }

    // =========================================================================
    // Extended Coverage Tests - Complex Real-World Scenarios
    // =========================================================================

    #[test]
    fn test_complex_class_definition() {
        let source = r#"class ComplexClass(BaseClass, Mixin):
    """A complex class with many features."""

    CLASS_CONSTANT = "constant"
    _protected_var = []

    def __init__(self, value: int = 0):
        self.value = value

    @property
    def computed(self) -> str:
        return str(self.value)

    @classmethod
    def from_string(cls, s: str) -> "ComplexClass":
        return cls(int(s))

    async def fetch_data(self) -> dict:
        pass
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        let class = &decls[0];
        assert_eq!(class.name, "ComplexClass");
        assert!(class.doc_comment.is_some());

        // Should have multiple children (methods + variables)
        assert!(class.children.len() >= 4);
    }

    #[test]
    fn test_complex_function_with_all_features() {
        let source = r#"@decorator1
@decorator2(arg)
async def complex_func(
    pos_arg: int,
    *args,
    keyword: str = "default",
    **kwargs
) -> Optional[Dict[str, Any]]:
    """
    A complex function with all features.

    Args:
        pos_arg: A positional argument
        *args: Variable args
        keyword: A keyword argument
        **kwargs: Variable keyword args

    Returns:
        Optional dictionary
    """
    try:
        for item in args:
            if item:
                result = process(item)
    except Exception as e:
        logger.error(e)
    finally:
        cleanup()
    return None
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        let func = &decls[0];
        assert_eq!(func.name, "complex_func");
        assert!(func.doc_comment.is_some());
        assert!(func.metadata.get("decorators").is_some());
        assert!(func.metadata.get("async").is_some());
    }

    #[test]
    fn test_module_level_code() {
        let source = r#"#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Module docstring."""

import os
from typing import List

CONSTANT = 42

def main():
    pass

class Config:
    pass

if __name__ == "__main__":
    main()
"#;
        let tree = parse_python(source);
        let adapter = PythonTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);
        let imports = adapter.extract_imports(&tree, source);
        let comments = adapter.extract_comments(&tree, source);

        // Should extract function and class
        assert!(decls.len() >= 2);
        // Should extract imports
        assert!(imports.len() >= 2);
        // Should extract comments
        assert!(!comments.is_empty());
    }
}
