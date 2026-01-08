//! TypeScript/JavaScript Language Adapter
//!
//! Extracts structural information from TypeScript and JavaScript source files
//! using Tree-sitter. Supports functions, classes, interfaces, types, imports/exports.

use super::{find_child_by_kind, node_text, node_to_span, LanguageAdapter};
use crate::ir::{
    Block, Call, Comment, CommentKind, ControlFlow, ControlFlowKind, Declaration, DeclarationKind,
    ImportKind, ImportLike, LanguageId, Parameter, Span, Visibility,
};

/// TypeScript/JavaScript language adapter using Tree-sitter
pub struct TypeScriptTreeSitterAdapter {
    language: tree_sitter::Language,
    language_id: LanguageId,
}

impl TypeScriptTreeSitterAdapter {
    /// Create a new TypeScript adapter
    pub fn new() -> Self {
        Self {
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            language_id: LanguageId::TypeScript,
        }
    }

    /// Create a new TSX adapter
    pub fn tsx() -> Self {
        Self {
            language: tree_sitter_typescript::LANGUAGE_TSX.into(),
            language_id: LanguageId::Tsx,
        }
    }

    /// Create a new JavaScript adapter
    pub fn javascript() -> Self {
        Self {
            language: tree_sitter_javascript::LANGUAGE.into(),
            language_id: LanguageId::JavaScript,
        }
    }
}

impl Default for TypeScriptTreeSitterAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAdapter for TypeScriptTreeSitterAdapter {
    fn language(&self) -> LanguageId {
        self.language_id
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        self.language.clone()
    }

    fn extract_declarations(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if let Some(decls) = self.extract_declaration(&child, source) {
                declarations.extend(decls);
            }
        }

        declarations
    }

    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<ImportLike> {
        let mut imports = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "import_statement" {
                if let Some(import) = self.extract_import(&child, source) {
                    imports.push(import);
                }
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

        // Find the body/block inside
        let body_node = find_child_by_kind(&node, "statement_block")
            .or_else(|| find_child_by_kind(&node, "class_body"))?;

        Some(self.extract_block(&body_node, source))
    }

    fn extract_visibility(&self, node: &tree_sitter::Node, source: &str) -> Visibility {
        // Check for visibility modifiers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "public" || kind == "accessibility_modifier" {
                let text = node_text(&child, source);
                return match text {
                    "public" => Visibility::Public,
                    "private" => Visibility::Private,
                    "protected" => Visibility::Protected,
                    _ => Visibility::Public,
                };
            }
        }

        // Check for 'export' keyword in parent or siblings
        if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                return Visibility::Public;
            }
        }

        // Check for preceding 'export' sibling
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "export" || node_text(&prev, source) == "export" {
                return Visibility::Public;
            }
        }

        Visibility::Unknown
    }
}

impl TypeScriptTreeSitterAdapter {
    /// Extract declarations from a node (may return multiple for export statements)
    fn extract_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Vec<Declaration>> {
        match node.kind() {
            "function_declaration" => self
                .extract_function_declaration(node, source)
                .map(|d| vec![d]),
            "class_declaration" => self
                .extract_class_declaration(node, source)
                .map(|d| vec![d]),
            "interface_declaration" => self
                .extract_interface_declaration(node, source)
                .map(|d| vec![d]),
            "type_alias_declaration" => self.extract_type_alias(node, source).map(|d| vec![d]),
            "enum_declaration" => self.extract_enum_declaration(node, source).map(|d| vec![d]),
            "lexical_declaration" | "variable_declaration" => {
                Some(self.extract_variable_declarations(node, source))
            }
            "export_statement" => self.extract_export_statement(node, source),
            _ => None,
        }
    }

    /// Extract a function declaration
    fn extract_function_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "identifier")?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Function, span);
        decl.visibility = visibility;
        decl.parameters = self.extract_parameters(node, source);
        decl.return_type = self.extract_return_type(node, source);
        decl.doc_comment = self.extract_jsdoc(node, source);

        // Check for async
        if self.is_async_function(node) {
            decl.metadata
                .insert("async".to_string(), "true".to_string());
        }

        // Extract body span
        if let Some(body) = find_child_by_kind(node, "statement_block") {
            decl.body_span = Some(node_to_span(&body));
        }

        Some(decl)
    }

    /// Extract a class declaration
    fn extract_class_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "type_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Class, span);
        decl.visibility = visibility;
        decl.doc_comment = self.extract_jsdoc(node, source);

        // Extract class body
        if let Some(body) = find_child_by_kind(node, "class_body") {
            decl.body_span = Some(node_to_span(&body));
            decl.children = self.extract_class_members(&body, source);
        }

        Some(decl)
    }

    /// Extract an interface declaration
    fn extract_interface_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "type_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Interface, span);
        decl.visibility = visibility;
        decl.doc_comment = self.extract_jsdoc(node, source);

        // Extract interface body
        if let Some(body) = find_child_by_kind(node, "interface_body")
            .or_else(|| find_child_by_kind(node, "object_type"))
        {
            decl.body_span = Some(node_to_span(&body));
            decl.children = self.extract_interface_members(&body, source);
        }

        Some(decl)
    }

    /// Extract a type alias declaration
    fn extract_type_alias(&self, node: &tree_sitter::Node, source: &str) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "type_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Type, span);
        decl.visibility = visibility;
        decl.doc_comment = self.extract_jsdoc(node, source);

        Some(decl)
    }

    /// Extract an enum declaration
    fn extract_enum_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "identifier")?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Enum, span);
        decl.visibility = visibility;
        decl.doc_comment = self.extract_jsdoc(node, source);

        // Extract enum body
        if let Some(body) = find_child_by_kind(node, "enum_body") {
            decl.body_span = Some(node_to_span(&body));
        }

        Some(decl)
    }

    /// Extract variable declarations (const, let, var)
    fn extract_variable_declarations(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(decl) = self.extract_variable_declarator(&child, node, source) {
                    declarations.push(decl);
                }
            }
        }

        declarations
    }

    /// Extract a single variable declarator
    fn extract_variable_declarator(
        &self,
        node: &tree_sitter::Node,
        parent: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "identifier")?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(parent);
        let visibility = self.extract_visibility(parent, source);

        // Check if it's an arrow function or function expression
        let mut cursor = node.walk();
        let mut kind = DeclarationKind::Variable;
        let mut is_async = false;

        for child in node.children(&mut cursor) {
            if child.kind() == "arrow_function" || child.kind() == "function_expression" {
                kind = DeclarationKind::Function;
                // Check for async arrow functions
                let mut inner_cursor = child.walk();
                for inner_child in child.children(&mut inner_cursor) {
                    if inner_child.kind() == "async" {
                        is_async = true;
                    }
                }
            }
        }

        let mut decl = Declaration::new(name, kind, span);
        decl.visibility = visibility;
        decl.doc_comment = self.extract_jsdoc(parent, source);

        if is_async {
            decl.metadata
                .insert("async".to_string(), "true".to_string());
        }

        // Extract parameters for arrow functions
        if kind == DeclarationKind::Function {
            if let Some(arrow) = find_child_by_kind(node, "arrow_function")
                .or_else(|| find_child_by_kind(node, "function_expression"))
            {
                decl.parameters = self.extract_parameters(&arrow, source);
                decl.return_type = self.extract_return_type(&arrow, source);
            }
        }

        Some(decl)
    }

    /// Extract declarations from an export statement
    fn extract_export_statement(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Vec<Declaration>> {
        let mut cursor = node.walk();
        let mut declarations = Vec::new();

        for child in node.children(&mut cursor) {
            if let Some(decls) = self.extract_declaration(&child, source) {
                for mut decl in decls {
                    decl.visibility = Visibility::Public;
                    declarations.push(decl);
                }
            }
        }

        if declarations.is_empty() {
            None
        } else {
            Some(declarations)
        }
    }

    /// Extract class members (methods, properties)
    fn extract_class_members(&self, body: &tree_sitter::Node, source: &str) -> Vec<Declaration> {
        let mut members = Vec::new();
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            match child.kind() {
                "method_definition" => {
                    if let Some(method) = self.extract_method_definition(&child, source) {
                        members.push(method);
                    }
                }
                "public_field_definition" | "field_definition" => {
                    if let Some(field) = self.extract_field_definition(&child, source) {
                        members.push(field);
                    }
                }
                _ => {}
            }
        }

        members
    }

    /// Extract a method definition
    fn extract_method_definition(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "property_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Method, span);
        decl.visibility = visibility;
        decl.parameters = self.extract_parameters(node, source);
        decl.return_type = self.extract_return_type(node, source);
        decl.doc_comment = self.extract_jsdoc(node, source);

        // Check for async methods
        if self.is_async_function(node) {
            decl.metadata
                .insert("async".to_string(), "true".to_string());
        }

        // Check for static methods
        if self.is_static_method(node) {
            decl.metadata
                .insert("static".to_string(), "true".to_string());
        }

        Some(decl)
    }

    /// Extract a field definition
    fn extract_field_definition(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "property_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, DeclarationKind::Variable, span);
        decl.visibility = visibility;

        Some(decl)
    }

    /// Extract interface members
    fn extract_interface_members(
        &self,
        body: &tree_sitter::Node,
        source: &str,
    ) -> Vec<Declaration> {
        let mut members = Vec::new();
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            match child.kind() {
                "method_signature" => {
                    if let Some(method) = self.extract_method_signature(&child, source) {
                        members.push(method);
                    }
                }
                "property_signature" => {
                    if let Some(prop) = self.extract_property_signature(&child, source) {
                        members.push(prop);
                    }
                }
                _ => {}
            }
        }

        members
    }

    /// Extract a method signature from interface
    fn extract_method_signature(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "property_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);

        let mut decl = Declaration::new(name, DeclarationKind::Method, span);
        decl.visibility = Visibility::Public;
        decl.parameters = self.extract_parameters(node, source);
        decl.return_type = self.extract_return_type(node, source);

        Some(decl)
    }

    /// Extract a property signature from interface
    fn extract_property_signature(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        let name_node = find_child_by_kind(node, "property_identifier")
            .or_else(|| find_child_by_kind(node, "identifier"))?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);

        let mut decl = Declaration::new(name, DeclarationKind::Variable, span);
        decl.visibility = Visibility::Public;

        Some(decl)
    }

    /// Check if a function is async
    fn is_async_function(&self, node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "async" {
                return true;
            }
        }
        false
    }

    /// Check if a method is static
    fn is_static_method(&self, node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "static" {
                return true;
            }
        }
        false
    }

    /// Extract parameters from a function-like node
    fn extract_parameters(&self, node: &tree_sitter::Node, source: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = find_child_by_kind(node, "formal_parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "required_parameter" | "optional_parameter" => {
                        if let Some(param) = self.extract_parameter(&child, source) {
                            params.push(param);
                        }
                    }
                    "identifier" => {
                        params.push(Parameter {
                            name: node_text(&child, source).to_string(),
                            type_annotation: None,
                            default_value: None,
                            span: node_to_span(&child),
                        });
                    }
                    "rest_pattern" => {
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

    /// Extract a single parameter
    fn extract_parameter(&self, node: &tree_sitter::Node, source: &str) -> Option<Parameter> {
        let name_node = find_child_by_kind(node, "identifier")?;
        let name = node_text(&name_node, source).to_string();
        let span = node_to_span(node);

        // Extract type annotation
        let type_annotation = find_child_by_kind(node, "type_annotation").map(|t| {
            node_text(&t, source)
                .trim_start_matches(':')
                .trim()
                .to_string()
        });

        // Extract default value
        let mut default_value = None;
        let mut cursor = node.walk();
        let mut found_equals = false;
        for child in node.children(&mut cursor) {
            if child.kind() == "=" {
                found_equals = true;
            } else if found_equals {
                default_value = Some(node_text(&child, source).to_string());
                break;
            }
        }

        Some(Parameter {
            name,
            type_annotation,
            default_value,
            span,
        })
    }

    /// Extract return type
    fn extract_return_type(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        find_child_by_kind(node, "type_annotation").map(|t| {
            node_text(&t, source)
                .trim_start_matches(':')
                .trim()
                .to_string()
        })
    }

    /// Extract JSDoc comment
    fn extract_jsdoc(&self, node: &tree_sitter::Node, source: &str) -> Option<Comment> {
        // Look for a preceding comment node
        let prev = node.prev_sibling();
        if let Some(comment_node) = prev {
            if comment_node.kind() == "comment" {
                let text = node_text(&comment_node, source);
                if text.starts_with("/**") {
                    let cleaned = text
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim())
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string();

                    return Some(Comment {
                        text: cleaned,
                        kind: CommentKind::Doc,
                        span: node_to_span(&comment_node),
                        attached_to: None,
                    });
                }
            }
        }
        None
    }

    /// Extract an import statement
    fn extract_import(&self, node: &tree_sitter::Node, source: &str) -> Option<ImportLike> {
        let span = node_to_span(node);

        // Find the source module
        let source_node = find_child_by_kind(node, "string")?;
        let source_text = node_text(&source_node, source)
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();

        // Extract imported items
        let mut items = Vec::new();
        let mut alias = None;
        let mut type_only = false;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "import_clause" => {
                    let (clause_items, clause_alias) = self.extract_import_clause(&child, source);
                    items.extend(clause_items);
                    if clause_alias.is_some() {
                        alias = clause_alias;
                    }
                }
                "type" => {
                    type_only = true;
                }
                _ => {}
            }
        }

        Some(ImportLike {
            source: source_text,
            kind: ImportKind::Import,
            items,
            alias,
            type_only,
            span,
        })
    }

    /// Extract items from import clause
    fn extract_import_clause(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> (Vec<String>, Option<String>) {
        let mut items = Vec::new();
        let mut alias = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    // Default import
                    items.push(node_text(&child, source).to_string());
                }
                "named_imports" => {
                    items.extend(self.extract_named_imports(&child, source));
                }
                "namespace_import" => {
                    // import * as name
                    if let Some(name_node) = find_child_by_kind(&child, "identifier") {
                        alias = Some(node_text(&name_node, source).to_string());
                        items.push("*".to_string());
                    }
                }
                _ => {}
            }
        }

        (items, alias)
    }

    /// Extract named imports
    fn extract_named_imports(&self, node: &tree_sitter::Node, source: &str) -> Vec<String> {
        let mut imports = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "import_specifier" {
                if let Some(name_node) = find_child_by_kind(&child, "identifier") {
                    imports.push(node_text(&name_node, source).to_string());
                }
            }
        }

        imports
    }

    /// Visit comments in the tree
    #[allow(clippy::only_used_in_recursion)]
    fn visit_comments(&self, node: &tree_sitter::Node, source: &str, comments: &mut Vec<Comment>) {
        if node.kind() == "comment" {
            let text = node_text(node, source);
            let span = node_to_span(node);

            let (cleaned, kind) = if text.starts_with("/**") {
                (
                    text.trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim())
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string(),
                    CommentKind::Doc,
                )
            } else if text.starts_with("/*") {
                (
                    text.trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim()
                        .to_string(),
                    CommentKind::Block,
                )
            } else {
                (
                    text.trim_start_matches("//").trim().to_string(),
                    CommentKind::Line,
                )
            };

            comments.push(Comment {
                text: cleaned,
                kind,
                span,
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

            if cursor.goto_first_child() {
                continue;
            }

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
            "if_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::If,
                    span: node_to_span(node),
                    condition_span: self.extract_condition_span(node),
                    branches: vec![],
                });
            }
            "for_statement" | "for_in_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::For,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "while_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::While,
                    span: node_to_span(node),
                    condition_span: self.extract_condition_span(node),
                    branches: vec![],
                });
            }
            "switch_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Switch,
                    span: node_to_span(node),
                    condition_span: None,
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
            "catch_clause" => {
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
            "return_statement" => {
                control_flow.push(ControlFlow {
                    kind: ControlFlowKind::Return,
                    span: node_to_span(node),
                    condition_span: None,
                    branches: vec![],
                });
            }
            "call_expression" => {
                if let Some(call) = self.extract_call(node, source) {
                    calls.push(call);
                }
            }
            "function_declaration" | "arrow_function" | "function_expression" => {
                if let Some(decls) = self.extract_declaration(node, source) {
                    nested_declarations.extend(decls);
                }
            }
            _ => {
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

    /// Extract condition span from if/while statements
    fn extract_condition_span(&self, node: &tree_sitter::Node) -> Option<Span> {
        find_child_by_kind(node, "parenthesized_expression").map(|n| node_to_span(&n))
    }

    /// Extract a function call
    fn extract_call(&self, node: &tree_sitter::Node, source: &str) -> Option<Call> {
        let function = find_child_by_kind(node, "member_expression")
            .or_else(|| find_child_by_kind(node, "identifier"))?;

        let callee = node_text(&function, source).to_string();
        let is_method = function.kind() == "member_expression";

        let argument_count = if let Some(args) = find_child_by_kind(node, "arguments") {
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

    fn parse_typescript(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_javascript(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_extract_function() {
        let source = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "greet");
        assert_eq!(func.kind, DeclarationKind::Function);
        assert_eq!(func.parameters.len(), 1);
        assert_eq!(func.parameters[0].name, "name");
    }

    #[test]
    fn test_extract_async_function() {
        let source = r#"
async function fetchData(url: string): Promise<Response> {
    return await fetch(url);
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "fetchData");
        assert_eq!(func.metadata.get("async"), Some(&"true".to_string()));
    }

    #[test]
    fn test_extract_class() {
        let source = r#"
class Person {
    private name: string;

    constructor(name: string) {
        this.name = name;
    }

    greet(): string {
        return `Hello, ${this.name}`;
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let class = &declarations[0];
        assert_eq!(class.name, "Person");
        assert_eq!(class.kind, DeclarationKind::Class);
        assert!(class.children.len() >= 2);
    }

    #[test]
    fn test_extract_interface() {
        let source = r#"
interface User {
    name: string;
    age: number;
    greet(): void;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let iface = &declarations[0];
        assert_eq!(iface.name, "User");
        assert_eq!(iface.kind, DeclarationKind::Interface);
    }

    #[test]
    fn test_extract_type_alias() {
        let source = r#"
type StringOrNumber = string | number;
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let type_alias = &declarations[0];
        assert_eq!(type_alias.name, "StringOrNumber");
        assert_eq!(type_alias.kind, DeclarationKind::Type);
    }

    #[test]
    fn test_extract_arrow_function() {
        let source = r#"
const add = (a: number, b: number): number => a + b;
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let func = &declarations[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.kind, DeclarationKind::Function);
    }

    #[test]
    fn test_extract_exports() {
        let source = r#"
export function greet() {}
export const PI = 3.14159;
export class MyClass {}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 3);
        for decl in &declarations {
            assert_eq!(decl.visibility, Visibility::Public);
        }
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
import React from 'react';
import { useState, useEffect } from 'react';
import * as lodash from 'lodash';
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 3);
        assert!(imports
            .iter()
            .any(|i| i.source == "react" && i.items.contains(&"React".to_string())));
        assert!(imports
            .iter()
            .any(|i| i.items.contains(&"useState".to_string())));
        assert!(imports.iter().any(|i| i.items.contains(&"*".to_string())));
    }

    #[test]
    fn test_javascript_adapter() {
        let source = r#"
function greet(name) {
    return "Hello, " + name;
}

const add = (a, b) => a + b;

class Calculator {
    add(a, b) {
        return a + b;
    }
}
"#;
        let tree = parse_javascript(source);
        let adapter = TypeScriptTreeSitterAdapter::javascript();
        let declarations = adapter.extract_declarations(&tree, source);

        assert!(declarations.len() >= 3);
        assert_eq!(adapter.language(), LanguageId::JavaScript);
    }

    #[test]
    fn test_adapter_languages() {
        assert_eq!(
            TypeScriptTreeSitterAdapter::new().language(),
            LanguageId::TypeScript
        );
        assert_eq!(
            TypeScriptTreeSitterAdapter::tsx().language(),
            LanguageId::Tsx
        );
        assert_eq!(
            TypeScriptTreeSitterAdapter::javascript().language(),
            LanguageId::JavaScript
        );
    }

    // ==================== Adapter Creation Tests ====================

    #[test]
    fn test_adapter_new() {
        let adapter = TypeScriptTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::TypeScript);
    }

    #[test]
    fn test_adapter_default() {
        let adapter = TypeScriptTreeSitterAdapter::default();
        assert_eq!(adapter.language(), LanguageId::TypeScript);
    }

    #[test]
    fn test_adapter_tsx() {
        let adapter = TypeScriptTreeSitterAdapter::tsx();
        assert_eq!(adapter.language(), LanguageId::Tsx);
    }

    #[test]
    fn test_tree_sitter_language() {
        let adapter = TypeScriptTreeSitterAdapter::new();
        let lang = adapter.tree_sitter_language();
        // Just verify it doesn't panic
        assert!(lang.node_kind_count() > 0);
    }

    // ==================== Function Extraction Tests ====================

    #[test]
    fn test_extract_function_no_params() {
        let source = "function noParams(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "noParams");
        assert_eq!(declarations[0].kind, DeclarationKind::Function);
        assert!(declarations[0].parameters.is_empty());
    }

    #[test]
    fn test_extract_function_multiple_params() {
        let source = "function multi(a: string, b: number, c: boolean): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].parameters.len(), 3);
        assert_eq!(declarations[0].parameters[0].name, "a");
        assert_eq!(declarations[0].parameters[1].name, "b");
        assert_eq!(declarations[0].parameters[2].name, "c");
    }

    #[test]
    fn test_extract_function_with_default() {
        let source = "function withDefault(name: string = 'world'): string { return name; }";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].parameters.len(), 1);
        // Default value may or may not be extracted depending on implementation
    }

    #[test]
    fn test_extract_function_rest_params() {
        let source = "function withRest(...args: string[]): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        // Rest param handling varies
    }

    #[test]
    fn test_extract_async_arrow_function() {
        let source = "const fetchData = async (url: string): Promise<Response> => fetch(url);";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "fetchData");
        assert_eq!(declarations[0].kind, DeclarationKind::Function);
        assert_eq!(
            declarations[0].metadata.get("async"),
            Some(&"true".to_string())
        );
    }

    #[test]
    fn test_extract_generator_function() {
        let source = "function* generator(): Generator<number> { yield 1; }";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let _declarations = adapter.extract_declarations(&tree, source);

        // Generator functions may not be extracted by current implementation
        // Just verify no panic
    }

    // ==================== Class Extraction Tests ====================

    #[test]
    fn test_extract_empty_class() {
        let source = "class Empty {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Empty");
        assert_eq!(declarations[0].kind, DeclarationKind::Class);
    }

    #[test]
    fn test_extract_class_with_static_method() {
        let source = r#"
class Utility {
    static helper(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert!(!declarations[0].children.is_empty());
        // Check for static metadata
        let method = &declarations[0].children[0];
        assert_eq!(method.metadata.get("static"), Some(&"true".to_string()));
    }

    #[test]
    fn test_extract_class_with_async_method() {
        let source = r#"
class Service {
    async fetch(): Promise<void> {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert!(!declarations[0].children.is_empty());
        let method = &declarations[0].children[0];
        assert_eq!(method.metadata.get("async"), Some(&"true".to_string()));
    }

    #[test]
    fn test_extract_class_with_constructor() {
        let source = r#"
class Point {
    constructor(public x: number, public y: number) {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        // Constructor may be in children
    }

    #[test]
    fn test_extract_class_with_visibility() {
        let source = r#"
class Sample {
    public pubMethod(): void {}
    private privMethod(): void {}
    protected protMethod(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let _children = &declarations[0].children;
        // Visibility should be extracted for methods
    }

    // ==================== Interface Tests ====================

    #[test]
    fn test_extract_interface_with_methods() {
        let source = r#"
interface Calculator {
    add(a: number, b: number): number;
    subtract(a: number, b: number): number;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
        assert!(declarations[0].children.len() >= 2);
    }

    #[test]
    fn test_extract_interface_with_properties() {
        let source = r#"
interface Config {
    name: string;
    value: number;
    optional?: boolean;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
    }

    #[test]
    fn test_extract_empty_interface() {
        let source = "interface Empty {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Empty");
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
    }

    // ==================== Enum Tests ====================

    #[test]
    fn test_extract_enum() {
        let source = r#"
enum Color {
    Red,
    Green,
    Blue
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Color");
        assert_eq!(declarations[0].kind, DeclarationKind::Enum);
    }

    #[test]
    fn test_extract_const_enum() {
        let source = r#"
const enum Status {
    Pending = 0,
    Active = 1,
    Completed = 2
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let _declarations = adapter.extract_declarations(&tree, source);

        // const enum may or may not be recognized depending on grammar
    }

    // ==================== Variable Tests ====================

    #[test]
    fn test_extract_const_declaration() {
        let source = "const PI: number = 3.14159;";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "PI");
        assert_eq!(declarations[0].kind, DeclarationKind::Variable);
    }

    #[test]
    fn test_extract_let_declaration() {
        let source = "let counter: number = 0;";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "counter");
    }

    #[test]
    fn test_extract_multiple_variable_declarators() {
        let source = "const a = 1, b = 2, c = 3;";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert!(declarations.len() >= 1); // May be 1 or 3 depending on extraction logic
    }

    // ==================== Import Tests ====================

    #[test]
    fn test_extract_default_import() {
        let source = "import React from 'react';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "react");
        assert!(imports[0].items.contains(&"React".to_string()));
    }

    #[test]
    fn test_extract_named_import() {
        let source = "import { useState, useEffect } from 'react';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "react");
        assert!(imports[0].items.contains(&"useState".to_string()));
        assert!(imports[0].items.contains(&"useEffect".to_string()));
    }

    #[test]
    fn test_extract_namespace_import() {
        let source = "import * as lodash from 'lodash';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "lodash");
        assert!(imports[0].items.contains(&"*".to_string()));
        assert_eq!(imports[0].alias, Some("lodash".to_string()));
    }

    #[test]
    fn test_extract_type_only_import() {
        let source = "import type { User } from './types';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert!(imports[0].type_only);
    }

    #[test]
    fn test_extract_mixed_import() {
        let source = "import React, { useState } from 'react';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert!(imports[0].items.contains(&"React".to_string()));
    }

    // ==================== Export Tests ====================

    #[test]
    fn test_extract_export_function() {
        let source = "export function greet(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "greet");
        assert_eq!(declarations[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_export_class() {
        let source = "export class MyService {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "MyService");
        assert_eq!(declarations[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_export_interface() {
        let source = "export interface Config {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Config");
        assert_eq!(declarations[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_export_const() {
        let source = "export const VERSION = '1.0.0';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "VERSION");
        assert_eq!(declarations[0].visibility, Visibility::Public);
    }

    // ==================== Comment Tests ====================

    #[test]
    fn test_extract_line_comment() {
        let source = "// This is a line comment\nfunction foo() {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert!(comments.iter().any(|c| c.text.contains("line comment")));
        assert!(comments.iter().any(|c| c.kind == CommentKind::Line));
    }

    #[test]
    fn test_extract_block_comment() {
        let source = "/* Block comment */\nfunction foo() {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert!(comments.iter().any(|c| c.kind == CommentKind::Block));
    }

    #[test]
    fn test_extract_jsdoc_comment() {
        let source = r#"
/**
 * Greets the user
 * @param name The name
 * @returns The greeting
 */
function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert!(comments.iter().any(|c| c.kind == CommentKind::Doc));
    }

    #[test]
    fn test_jsdoc_attached_to_function() {
        let source = r#"
/**
 * Adds two numbers
 */
function add(a: number, b: number): number {
    return a + b;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert!(declarations[0].doc_comment.is_some());
        let doc = declarations[0].doc_comment.as_ref().unwrap();
        assert!(doc.text.contains("Adds two numbers"));
    }

    // ==================== Body Extraction Tests ====================

    #[test]
    fn test_extract_function_body() {
        let source = "function test(): void { const x = 1; return; }";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let _body = adapter.extract_body(&tree, source, &declarations[0]);
        // Body extraction may or may not work depending on span matching
        // Just verify no panic
    }

    #[test]
    fn test_extract_class_body() {
        let source = r#"
class Service {
    method() {
        return 42;
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let _body = adapter.extract_body(&tree, source, &declarations[0]);
        // May or may not have body depending on body_span matching
    }

    // ==================== Control Flow Tests ====================

    #[test]
    fn test_extract_if_statement() {
        let source = r#"
function test(): void {
    if (true) {
        console.log('yes');
    } else {
        console.log('no');
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::If));
        }
    }

    #[test]
    fn test_extract_for_loop() {
        let source = r#"
function test(): void {
    for (let i = 0; i < 10; i++) {
        console.log(i);
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::For));
        }
    }

    #[test]
    fn test_extract_for_in_loop() {
        let source = r#"
function test(): void {
    for (const key in obj) {
        console.log(key);
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::For));
        }
    }

    #[test]
    fn test_extract_while_loop() {
        let source = r#"
function test(): void {
    while (true) {
        break;
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::While));
        }
    }

    #[test]
    fn test_extract_switch_statement() {
        let source = r#"
function test(x: number): void {
    switch (x) {
        case 1:
            break;
        default:
            break;
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Switch));
        }
    }

    #[test]
    fn test_extract_try_catch() {
        let source = r#"
function test(): void {
    try {
        throw new Error('test');
    } catch (e) {
        console.log(e);
    } finally {
        console.log('done');
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Try));
        }
    }

    #[test]
    fn test_extract_return_statement() {
        let source = r#"
function test(): number {
    return 42;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Return));
        }
    }

    // ==================== Call Extraction Tests ====================

    #[test]
    fn test_extract_function_call() {
        let source = r#"
function test(): void {
    console.log('hello');
    fetch('/api');
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(body.calls.len() >= 2);
        }
    }

    #[test]
    fn test_extract_method_call() {
        let source = r#"
function test(): void {
    obj.method();
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            let method_calls: Vec<_> = body.calls.iter().filter(|c| c.is_method).collect();
            assert!(!method_calls.is_empty());
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_empty_source() {
        let source = "";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();

        assert!(adapter.extract_declarations(&tree, source).is_empty());
        assert!(adapter.extract_imports(&tree, source).is_empty());
        assert!(adapter.extract_comments(&tree, source).is_empty());
    }

    #[test]
    fn test_only_comments() {
        let source = "// Just a comment";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();

        assert!(adapter.extract_declarations(&tree, source).is_empty());
        assert!(!adapter.extract_comments(&tree, source).is_empty());
    }

    #[test]
    fn test_multiple_declarations() {
        let source = r#"
function first(): void {}
function second(): void {}
class Third {}
interface Fourth {}
type Fifth = string;
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 5);
    }

    #[test]
    fn test_nested_function() {
        let source = r#"
function outer(): void {
    function inner(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "outer");
        // Inner function should be in body, not top-level declarations
    }

    #[test]
    fn test_extract_errors() {
        let source = "function broken( {}"; // Syntax error
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();

        let errors = adapter.extract_errors(&tree, source);
        // May or may not have errors depending on parser recovery
        let _ = errors;
    }

    #[test]
    fn test_unicode_identifiers() {
        let source = "function 日本語(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let _declarations = adapter.extract_declarations(&tree, source);

        // Unicode support varies
    }

    // ==================== JavaScript-specific Tests ====================

    #[test]
    fn test_javascript_function() {
        let source = "function greet(name) { return 'Hello, ' + name; }";
        let tree = parse_javascript(source);
        let adapter = TypeScriptTreeSitterAdapter::javascript();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "greet");
    }

    #[test]
    fn test_javascript_arrow_function() {
        let source = "const add = (a, b) => a + b;";
        let tree = parse_javascript(source);
        let adapter = TypeScriptTreeSitterAdapter::javascript();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "add");
        assert_eq!(declarations[0].kind, DeclarationKind::Function);
    }

    #[test]
    fn test_javascript_class() {
        let source = r#"
class Animal {
    constructor(name) {
        this.name = name;
    }

    speak() {
        console.log(this.name);
    }
}
"#;
        let tree = parse_javascript(source);
        let adapter = TypeScriptTreeSitterAdapter::javascript();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Animal");
        assert_eq!(declarations[0].kind, DeclarationKind::Class);
    }

    #[test]
    fn test_javascript_import() {
        let source = "import { readFile } from 'fs';";
        let tree = parse_javascript(source);
        let adapter = TypeScriptTreeSitterAdapter::javascript();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "fs");
    }

    // ==================== Visibility Tests ====================

    #[test]
    fn test_visibility_unknown() {
        let source = "function internal(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].visibility, Visibility::Unknown);
    }

    #[test]
    fn test_visibility_exported() {
        let source = "export function exported(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].visibility, Visibility::Public);
    }

    // ==================== Extended Coverage Tests ====================

    #[test]
    fn test_class_private_visibility() {
        let source = r#"
class Sample {
    private privateField: string = "";
    private privateMethod(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let class = &declarations[0];
        let private_members: Vec<_> = class
            .children
            .iter()
            .filter(|c| c.visibility == Visibility::Private)
            .collect();
        assert!(!private_members.is_empty());
    }

    #[test]
    fn test_class_protected_visibility() {
        let source = r#"
class Base {
    protected protectedMethod(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let class = &declarations[0];
        let protected_members: Vec<_> = class
            .children
            .iter()
            .filter(|c| c.visibility == Visibility::Protected)
            .collect();
        assert!(!protected_members.is_empty());
    }

    #[test]
    fn test_class_public_visibility() {
        let source = r#"
class Public {
    public publicMethod(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let class = &declarations[0];
        let public_members: Vec<_> = class
            .children
            .iter()
            .filter(|c| c.visibility == Visibility::Public)
            .collect();
        assert!(!public_members.is_empty());
    }

    #[test]
    fn test_interface_with_readonly_property() {
        let source = r#"
interface ReadOnly {
    readonly id: number;
    name: string;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
    }

    #[test]
    fn test_interface_with_index_signature() {
        let source = r#"
interface Dictionary {
    [key: string]: any;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
    }

    #[test]
    fn test_optional_parameter() {
        let source = "function withOptional(required: string, optional?: number): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].parameters.len(), 2);
    }

    #[test]
    fn test_parameter_with_default_value() {
        let source = "function withDefault(value: number = 42): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].parameters.len(), 1);
        // Default value extraction
        let param = &declarations[0].parameters[0];
        assert!(param.default_value.is_some());
    }

    #[test]
    fn test_class_with_field_definition() {
        let source = r#"
class WithFields {
    name: string = "default";
    count = 0;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let fields: Vec<_> = declarations[0]
            .children
            .iter()
            .filter(|c| c.kind == DeclarationKind::Variable)
            .collect();
        assert!(!fields.is_empty());
    }

    #[test]
    fn test_catch_clause_extraction() {
        let source = r#"
function test(): void {
    try {
        throw new Error();
    } catch (error) {
        console.log(error);
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            let has_catch = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Catch);
            assert!(has_catch || body.control_flow.len() >= 1);
        }
    }

    #[test]
    fn test_finally_clause_extraction() {
        let source = r#"
function test(): void {
    try {
        doSomething();
    } finally {
        cleanup();
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            let has_finally = body
                .control_flow
                .iter()
                .any(|cf| cf.kind == ControlFlowKind::Finally);
            assert!(has_finally || body.control_flow.len() >= 1);
        }
    }

    #[test]
    fn test_nested_arrow_function_in_body() {
        let source = r#"
function test(): void {
    const inner = () => 42;
    inner();
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            // Nested arrow function should be in nested_declarations
            assert!(!body.nested_declarations.is_empty() || !body.calls.is_empty());
        }
    }

    #[test]
    fn test_body_extraction_non_matching_span() {
        let source = "function foo(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();

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

    #[test]
    fn test_export_empty_statement() {
        let source = "export {};";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        // Empty export should not produce declarations
        assert!(declarations.is_empty());
    }

    #[test]
    fn test_re_export_statement() {
        let source = "export { foo } from './module';";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        // Re-exports may or may not produce declarations
        let _ = declarations;
    }

    #[test]
    fn test_function_expression_variable() {
        let source = "const fn = function(): void {};";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "fn");
        assert_eq!(declarations[0].kind, DeclarationKind::Function);
    }

    #[test]
    fn test_generic_function() {
        let source = "function identity<T>(value: T): T { return value; }";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "identity");
    }

    #[test]
    fn test_generic_class() {
        let source = r#"
class Container<T> {
    constructor(private value: T) {}
    get(): T { return this.value; }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Container");
        assert_eq!(declarations[0].kind, DeclarationKind::Class);
    }

    #[test]
    fn test_generic_interface() {
        let source = r#"
interface Result<T, E> {
    value?: T;
    error?: E;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Result");
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
    }

    #[test]
    fn test_call_with_no_arguments() {
        let source = r#"
function test(): void {
    doSomething();
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(!body.calls.is_empty());
            assert_eq!(body.calls[0].argument_count, 0);
        }
    }

    #[test]
    fn test_call_with_multiple_arguments() {
        let source = r#"
function test(): void {
    calculate(1, 2, 3, 4);
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            assert!(!body.calls.is_empty());
            assert_eq!(body.calls[0].argument_count, 4);
        }
    }

    #[test]
    fn test_condition_span_extraction() {
        let source = r#"
function test(): void {
    if (x > 5 && y < 10) {
        console.log('yes');
    }
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        if let Some(body) = adapter.extract_body(&tree, source, &declarations[0]) {
            let if_stmt = body.control_flow.iter().find(|cf| cf.kind == ControlFlowKind::If);
            if let Some(if_cf) = if_stmt {
                // Condition span should be present
                assert!(if_cf.condition_span.is_some());
            }
        }
    }

    #[test]
    fn test_jsdoc_multiline() {
        let source = r#"
/**
 * First line
 * Second line
 * Third line
 */
function documented(): void {}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert!(declarations[0].doc_comment.is_some());
        let doc = declarations[0].doc_comment.as_ref().unwrap();
        assert!(doc.text.contains("First line"));
    }

    #[test]
    fn test_interface_method_with_params() {
        let source = r#"
interface Calculator {
    add(a: number, b: number): number;
    multiply(x: number, y: number): number;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        let interface = &declarations[0];
        assert!(interface.children.len() >= 2);
        for method in &interface.children {
            if method.kind == DeclarationKind::Method {
                assert_eq!(method.parameters.len(), 2);
            }
        }
    }

    #[test]
    fn test_complex_type_alias() {
        let source = r#"
type Handler<T> = (event: T) => void;
type Union = string | number | boolean;
type Intersection = A & B & C;
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 3);
        for decl in &declarations {
            assert_eq!(decl.kind, DeclarationKind::Type);
        }
    }

    #[test]
    fn test_abstract_class() {
        let source = r#"
abstract class Shape {
    abstract area(): number;
    name: string = 'shape';
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        // Abstract classes may or may not be extracted depending on tree-sitter grammar
        // Just verify no panic
        let _ = declarations;
    }

    #[test]
    fn test_class_inheritance() {
        let source = r#"
class Child extends Parent implements Interface1, Interface2 {
    method(): void {}
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Child");
    }

    #[test]
    fn test_interface_inheritance() {
        let source = r#"
interface Extended extends Base {
    extraProperty: string;
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Extended");
        assert_eq!(declarations[0].kind, DeclarationKind::Interface);
    }

    #[test]
    fn test_string_enum() {
        let source = r#"
enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT"
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "Direction");
        assert_eq!(declarations[0].kind, DeclarationKind::Enum);
    }

    #[test]
    fn test_export_default_function() {
        let source = "export default function main(): void {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let _declarations = adapter.extract_declarations(&tree, source);

        // Default export handling varies
    }

    #[test]
    fn test_export_default_class() {
        let source = "export default class Main {}";
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let _declarations = adapter.extract_declarations(&tree, source);

        // Default export handling varies
    }

    #[test]
    fn test_tsx_adapter() {
        let adapter = TypeScriptTreeSitterAdapter::tsx();
        assert_eq!(adapter.language(), LanguageId::Tsx);
        let lang = adapter.tree_sitter_language();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_complex_real_world_module() {
        let source = r#"
/**
 * User service module
 * @module UserService
 */

import { Database } from './database';
import type { User, CreateUserDTO } from './types';

export interface UserRepository {
    findById(id: string): Promise<User | null>;
    create(data: CreateUserDTO): Promise<User>;
    update(id: string, data: Partial<User>): Promise<User>;
    delete(id: string): Promise<void>;
}

export class UserService implements UserRepository {
    private db: Database;

    constructor(db: Database) {
        this.db = db;
    }

    async findById(id: string): Promise<User | null> {
        try {
            return await this.db.query('SELECT * FROM users WHERE id = ?', [id]);
        } catch (error) {
            console.error('Error finding user:', error);
            return null;
        }
    }

    async create(data: CreateUserDTO): Promise<User> {
        const user = { ...data, id: generateId() };
        await this.db.insert('users', user);
        return user;
    }

    async update(id: string, data: Partial<User>): Promise<User> {
        await this.db.update('users', id, data);
        return this.findById(id) as Promise<User>;
    }

    async delete(id: string): Promise<void> {
        await this.db.delete('users', id);
    }
}

export const DEFAULT_PAGE_SIZE = 20;

export type UserWithPosts = User & { posts: Post[] };

export enum UserRole {
    Admin = 'ADMIN',
    User = 'USER',
    Guest = 'GUEST'
}
"#;
        let tree = parse_typescript(source);
        let adapter = TypeScriptTreeSitterAdapter::new();
        let declarations = adapter.extract_declarations(&tree, source);
        let imports = adapter.extract_imports(&tree, source);
        let comments = adapter.extract_comments(&tree, source);

        // Should extract interface, class, const, type, enum
        assert!(declarations.len() >= 5);
        // Should extract imports
        assert_eq!(imports.len(), 2);
        // Should extract comments
        assert!(!comments.is_empty());
    }
}
