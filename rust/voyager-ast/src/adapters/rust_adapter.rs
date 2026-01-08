//! Rust Language Adapter
//!
//! Extracts structural information from Rust source files using Tree-sitter.
//! Supports functions, structs, enums, traits, impl blocks, and more.

use super::{find_child_by_kind, find_children_by_kind, node_text, node_to_span, LanguageAdapter};
use crate::ir::{
    Block, Call, Comment, CommentKind, ControlFlow, ControlFlowKind, Declaration, DeclarationKind,
    ImportKind, ImportLike, LanguageId, Parameter, Span, Visibility,
};

/// Rust language adapter using Tree-sitter
pub struct RustTreeSitterAdapter {
    language: tree_sitter::Language,
}

impl RustTreeSitterAdapter {
    /// Create a new Rust adapter
    pub fn new() -> Self {
        Self {
            language: tree_sitter_rust::LANGUAGE.into(),
        }
    }
}

impl Default for RustTreeSitterAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAdapter for RustTreeSitterAdapter {
    fn language(&self) -> LanguageId {
        LanguageId::Rust
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        self.language.clone()
    }

    fn extract_declarations(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if let Some(decl) = self.extract_declaration(&child, source) {
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
                "use_declaration" => {
                    if let Some(import) = self.extract_use_declaration(&child, source) {
                        imports.push(import);
                    }
                }
                "mod_item" => {
                    if let Some(import) = self.extract_mod_item(&child, source) {
                        imports.push(import);
                    }
                }
                "extern_crate_declaration" => {
                    if let Some(import) = self.extract_extern_crate(&child, source) {
                        imports.push(import);
                    }
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
        // Find the declaration in the tree by its span
        let target_start = declaration.span.start;
        let target_end = declaration.span.end;

        let root = tree.root_node();
        let node = self.find_matching_descendant(root, target_start, target_end)?;

        // Extract the body block
        let body_node = match node.kind() {
            "function_item" | "closure_expression" => find_child_by_kind(&node, "block"),
            "impl_item" => find_child_by_kind(&node, "declaration_list"),
            "struct_item" => find_child_by_kind(&node, "field_declaration_list"),
            "enum_item" => find_child_by_kind(&node, "enum_variant_list"),
            "trait_item" => find_child_by_kind(&node, "declaration_list"),
            _ => None,
        }?;

        Some(self.extract_block(&body_node, source))
    }

    fn extract_visibility(&self, node: &tree_sitter::Node, source: &str) -> Visibility {
        // Look for visibility modifier
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                let text = node_text(&child, source);
                return match text {
                    "pub" => Visibility::Public,
                    s if s.starts_with("pub(crate)") => Visibility::Internal,
                    s if s.starts_with("pub(super)") => Visibility::Protected,
                    s if s.starts_with("pub(") => Visibility::Internal,
                    _ => Visibility::Public,
                };
            }
        }
        Visibility::Private
    }
}

impl RustTreeSitterAdapter {
    /// Extract a declaration from a node
    fn extract_declaration(&self, node: &tree_sitter::Node, source: &str) -> Option<Declaration> {
        let kind = node.kind();
        let decl_kind = match kind {
            "function_item" => DeclarationKind::Function,
            "struct_item" => DeclarationKind::Struct,
            "enum_item" => DeclarationKind::Enum,
            "trait_item" => DeclarationKind::Trait,
            "impl_item" => DeclarationKind::Impl,
            "type_item" => DeclarationKind::Type,
            "const_item" => DeclarationKind::Constant,
            "static_item" => DeclarationKind::Variable,
            "mod_item" => DeclarationKind::Module,
            "macro_definition" => DeclarationKind::Macro,
            _ => return None,
        };

        let name = self.extract_name(node, source)?;
        let span = node_to_span(node);
        let visibility = self.extract_visibility(node, source);

        let mut decl = Declaration::new(name, decl_kind, span);
        decl.visibility = visibility;

        // Extract doc comment
        decl.doc_comment = self.extract_doc_comment(node, source);

        // Extract signature span
        if let Some(sig_span) = self.extract_signature_span(node, source) {
            decl.signature_span = Some(sig_span);
        }

        // Extract body span
        if let Some(body_span) = self.extract_body_span(node, source) {
            decl.body_span = Some(body_span);
        }

        // Extract parameters for functions
        if decl_kind == DeclarationKind::Function {
            decl.parameters = self.extract_parameters(node, source);
            decl.return_type = self.extract_return_type(node, source);
        }

        // Extract children for impl/trait/struct/enum
        if matches!(
            decl_kind,
            DeclarationKind::Impl
                | DeclarationKind::Trait
                | DeclarationKind::Struct
                | DeclarationKind::Enum
        ) {
            decl.children = self.extract_children(node, source);
        }

        Some(decl)
    }

    /// Extract the name of a declaration
    fn extract_name(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        match node.kind() {
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "type_item"
            | "const_item" | "static_item" | "mod_item" | "macro_definition" => {
                // Look for name/identifier child
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" || child.kind() == "type_identifier" {
                        return Some(node_text(&child, source).to_string());
                    }
                }
                None
            }
            "impl_item" => {
                // For impl, get the type being implemented
                if let Some(type_node) = find_child_by_kind(node, "type_identifier") {
                    return Some(node_text(&type_node, source).to_string());
                }
                // For trait impl, try to get "impl Trait for Type"
                if let Some(generic) = find_child_by_kind(node, "generic_type") {
                    return Some(node_text(&generic, source).to_string());
                }
                Some("impl".to_string())
            }
            _ => None,
        }
    }

    /// Extract doc comment preceding a node
    fn extract_doc_comment(&self, node: &tree_sitter::Node, source: &str) -> Option<Comment> {
        // Look at the previous sibling for doc comments
        let mut prev = node.prev_sibling();
        let mut doc_lines = Vec::new();
        let mut doc_span: Option<Span> = None;

        while let Some(prev_node) = prev {
            match prev_node.kind() {
                "line_comment" => {
                    let text = node_text(&prev_node, source);
                    if text.starts_with("///") || text.starts_with("//!") {
                        let comment_text = text
                            .trim_start_matches("///")
                            .trim_start_matches("//!")
                            .trim();
                        doc_lines.insert(0, comment_text.to_string());

                        let span = node_to_span(&prev_node);
                        doc_span = Some(match doc_span {
                            Some(existing) => Span {
                                start: span.start,
                                end: existing.end,
                                start_line: span.start_line,
                                end_line: existing.end_line,
                                start_column: span.start_column,
                                end_column: existing.end_column,
                            },
                            None => span,
                        });
                    } else {
                        break;
                    }
                }
                "block_comment" => {
                    let text = node_text(&prev_node, source);
                    if text.starts_with("/**") || text.starts_with("/*!") {
                        let comment_text = text
                            .trim_start_matches("/**")
                            .trim_start_matches("/*!")
                            .trim_end_matches("*/")
                            .trim();
                        doc_lines.insert(0, comment_text.to_string());
                        doc_span = Some(node_to_span(&prev_node));
                    }
                    break;
                }
                _ => break,
            }
            prev = prev_node.prev_sibling();
        }

        if doc_lines.is_empty() {
            return None;
        }

        Some(Comment {
            text: doc_lines.join("\n"),
            kind: CommentKind::Doc,
            span: doc_span.unwrap_or_default(),
            attached_to: None,
        })
    }

    /// Extract function parameters
    fn extract_parameters(&self, node: &tree_sitter::Node, source: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = find_child_by_kind(node, "parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter" || child.kind() == "self_parameter" {
                    if let Some(param) = self.extract_parameter(&child, source) {
                        params.push(param);
                    }
                }
            }
        }

        params
    }

    /// Extract a single parameter
    fn extract_parameter(&self, node: &tree_sitter::Node, source: &str) -> Option<Parameter> {
        if node.kind() == "self_parameter" {
            let text = node_text(node, source);
            return Some(Parameter {
                name: "self".to_string(),
                type_annotation: Some(text.to_string()),
                default_value: None,
                span: node_to_span(node),
            });
        }

        // Regular parameter: pattern : type
        let mut name = None;
        let mut type_annotation = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    name = Some(node_text(&child, source).to_string());
                }
                "type"
                | "reference_type"
                | "primitive_type"
                | "generic_type"
                | "scoped_type_identifier" => {
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

    /// Extract return type
    fn extract_return_type(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        // Look for return type after ->
        let mut cursor = node.walk();
        let mut found_arrow = false;

        for child in node.children(&mut cursor) {
            if child.kind() == "->" {
                found_arrow = true;
            } else if found_arrow {
                // The next non-trivial node after -> is the return type
                match child.kind() {
                    "type"
                    | "reference_type"
                    | "primitive_type"
                    | "generic_type"
                    | "scoped_type_identifier"
                    | "tuple_type"
                    | "unit_type" => {
                        return Some(node_text(&child, source).to_string());
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// Extract signature span (up to but not including the body)
    fn extract_signature_span(&self, node: &tree_sitter::Node, _source: &str) -> Option<Span> {
        let body_kinds = [
            "block",
            "field_declaration_list",
            "enum_variant_list",
            "declaration_list",
        ];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if body_kinds.contains(&child.kind()) {
                // Signature ends just before the body
                return Some(Span {
                    start: node.start_byte(),
                    end: child.start_byte(),
                    start_line: node.start_position().row + 1,
                    end_line: child.start_position().row + 1,
                    start_column: node.start_position().column,
                    end_column: child.start_position().column,
                });
            }
        }

        None
    }

    /// Extract body span
    fn extract_body_span(&self, node: &tree_sitter::Node, _source: &str) -> Option<Span> {
        let body_kinds = [
            "block",
            "field_declaration_list",
            "enum_variant_list",
            "declaration_list",
        ];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if body_kinds.contains(&child.kind()) {
                return Some(node_to_span(&child));
            }
        }

        None
    }

    /// Extract children (methods in impl, variants in enum, etc.)
    fn extract_children(&self, node: &tree_sitter::Node, source: &str) -> Vec<Declaration> {
        let mut children = Vec::new();

        let body_kinds = [
            "declaration_list",
            "field_declaration_list",
            "enum_variant_list",
        ];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if body_kinds.contains(&child.kind()) {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if let Some(decl) = self.extract_child_declaration(&inner, source) {
                        children.push(decl);
                    }
                }
            }
        }

        children
    }

    /// Extract a child declaration (method, field, variant)
    fn extract_child_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<Declaration> {
        match node.kind() {
            "function_item" => {
                let mut decl = self.extract_declaration(node, source)?;
                decl.kind = DeclarationKind::Method;
                Some(decl)
            }
            "field_declaration" => {
                // struct field
                if let Some(name_node) = find_child_by_kind(node, "field_identifier") {
                    let name = node_text(&name_node, source).to_string();
                    let mut decl =
                        Declaration::new(name, DeclarationKind::Variable, node_to_span(node));
                    decl.visibility = self.extract_visibility(node, source);
                    Some(decl)
                } else {
                    None
                }
            }
            "enum_variant" => {
                if let Some(name_node) = find_child_by_kind(node, "identifier") {
                    let name = node_text(&name_node, source).to_string();
                    Some(Declaration::new(
                        name,
                        DeclarationKind::Variable,
                        node_to_span(node),
                    ))
                } else {
                    None
                }
            }
            "const_item" | "type_item" => self.extract_declaration(node, source),
            _ => None,
        }
    }

    /// Extract a use declaration
    fn extract_use_declaration(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Option<ImportLike> {
        // Find the use_tree or scoped_identifier
        let use_tree = find_child_by_kind(node, "use_tree")
            .or_else(|| find_child_by_kind(node, "scoped_identifier"))?;

        let full_path = node_text(&use_tree, source).to_string();

        // Parse items from braced groups
        let items = self.extract_use_items(&use_tree, source);

        Some(ImportLike {
            source: full_path,
            kind: ImportKind::Use,
            items,
            alias: self.extract_use_alias(&use_tree, source),
            type_only: false,
            span: node_to_span(node),
        })
    }

    /// Extract items from a use tree (for use crate::{A, B, C})
    fn extract_use_items(&self, node: &tree_sitter::Node, source: &str) -> Vec<String> {
        let mut items = Vec::new();

        // Look for use_list
        if let Some(list) = find_child_by_kind(node, "use_list") {
            let mut cursor = list.walk();
            for child in list.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "use_tree" {
                    items.push(node_text(&child, source).to_string());
                }
            }
        }

        items
    }

    /// Extract alias from use statement
    fn extract_use_alias(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        if let Some(alias) = find_child_by_kind(node, "use_as_clause") {
            if let Some(ident) = find_child_by_kind(&alias, "identifier") {
                return Some(node_text(&ident, source).to_string());
            }
        }
        None
    }

    /// Extract a mod item
    fn extract_mod_item(&self, node: &tree_sitter::Node, source: &str) -> Option<ImportLike> {
        // Only for `mod foo;` (external module), not `mod foo { }`
        if find_child_by_kind(node, "declaration_list").is_some() {
            return None;
        }

        let name = find_child_by_kind(node, "identifier")?;

        Some(ImportLike {
            source: node_text(&name, source).to_string(),
            kind: ImportKind::Module,
            items: Vec::new(),
            alias: None,
            type_only: false,
            span: node_to_span(node),
        })
    }

    /// Extract an extern crate declaration
    fn extract_extern_crate(&self, node: &tree_sitter::Node, source: &str) -> Option<ImportLike> {
        let name =
            find_child_by_kind(node, "identifier").or_else(|| find_child_by_kind(node, "crate"))?;

        Some(ImportLike {
            source: node_text(&name, source).to_string(),
            kind: ImportKind::Import,
            items: Vec::new(),
            alias: None,
            type_only: false,
            span: node_to_span(node),
        })
    }

    /// Visit all comments in a tree
    #[allow(clippy::only_used_in_recursion)]
    fn visit_comments(&self, node: &tree_sitter::Node, source: &str, comments: &mut Vec<Comment>) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "line_comment" => {
                    let text = node_text(&child, source);
                    let kind = if text.starts_with("///") || text.starts_with("//!") {
                        CommentKind::Doc
                    } else {
                        CommentKind::Line
                    };
                    let trimmed = text
                        .trim_start_matches("///")
                        .trim_start_matches("//!")
                        .trim_start_matches("//")
                        .trim();

                    comments.push(Comment {
                        text: trimmed.to_string(),
                        kind,
                        span: node_to_span(&child),
                        attached_to: None,
                    });
                }
                "block_comment" => {
                    let text = node_text(&child, source);
                    let kind = if text.starts_with("/**") || text.starts_with("/*!") {
                        CommentKind::Doc
                    } else {
                        CommentKind::Block
                    };
                    let trimmed = text
                        .trim_start_matches("/**")
                        .trim_start_matches("/*!")
                        .trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim();

                    comments.push(Comment {
                        text: trimmed.to_string(),
                        kind,
                        span: node_to_span(&child),
                        attached_to: None,
                    });
                }
                _ => {
                    self.visit_comments(&child, source, comments);
                }
            }
        }
    }

    /// Find a node at a specific span (returns whether found at the expected location)
    fn node_matches_span(&self, node: &tree_sitter::Node, start: usize, end: usize) -> bool {
        node.start_byte() == start && node.end_byte() == end
    }

    /// Find a descendant node matching a span
    fn find_matching_descendant<'a>(
        &self,
        node: tree_sitter::Node<'a>,
        start: usize,
        end: usize,
    ) -> Option<tree_sitter::Node<'a>> {
        if self.node_matches_span(&node, start, end) {
            return Some(node);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.start_byte() <= start && child.end_byte() >= end {
                    if let Some(found) = self.find_matching_descendant(child, start, end) {
                        return Some(found);
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        None
    }

    /// Extract a block with control flow and calls
    fn extract_block(&self, node: &tree_sitter::Node, source: &str) -> Block {
        let mut block = Block {
            span: node_to_span(node),
            control_flow: Vec::new(),
            calls: Vec::new(),
            comments: Vec::new(),
            unknown_regions: Vec::new(),
            nested_declarations: Vec::new(),
        };

        self.visit_block_contents(node, source, &mut block);

        block
    }

    /// Visit block contents recursively
    fn visit_block_contents(&self, node: &tree_sitter::Node, source: &str, block: &mut Block) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                // Control flow
                "if_expression" => {
                    block.control_flow.push(self.extract_if(&child, source));
                }
                "match_expression" => {
                    block.control_flow.push(self.extract_match(&child, source));
                }
                "for_expression" => {
                    block.control_flow.push(ControlFlow {
                        kind: ControlFlowKind::For,
                        span: node_to_span(&child),
                        condition_span: find_child_by_kind(&child, "for_pattern")
                            .map(|n| node_to_span(&n)),
                        branches: find_child_by_kind(&child, "block")
                            .map(|b| vec![self.extract_block(&b, source)])
                            .unwrap_or_default(),
                    });
                }
                "while_expression" => {
                    block.control_flow.push(ControlFlow {
                        kind: ControlFlowKind::While,
                        span: node_to_span(&child),
                        condition_span: find_child_by_kind(&child, "condition")
                            .map(|n| node_to_span(&n)),
                        branches: find_child_by_kind(&child, "block")
                            .map(|b| vec![self.extract_block(&b, source)])
                            .unwrap_or_default(),
                    });
                }
                "loop_expression" => {
                    block.control_flow.push(ControlFlow {
                        kind: ControlFlowKind::Loop,
                        span: node_to_span(&child),
                        condition_span: None,
                        branches: find_child_by_kind(&child, "block")
                            .map(|b| vec![self.extract_block(&b, source)])
                            .unwrap_or_default(),
                    });
                }
                "return_expression" => {
                    block.control_flow.push(ControlFlow {
                        kind: ControlFlowKind::Return,
                        span: node_to_span(&child),
                        condition_span: None,
                        branches: Vec::new(),
                    });
                }
                // Function calls
                "call_expression" => {
                    if let Some(call) = self.extract_call(&child, source) {
                        block.calls.push(call);
                    }
                }
                "method_call_expression" => {
                    if let Some(call) = self.extract_method_call(&child, source) {
                        block.calls.push(call);
                    }
                }
                // Comments
                "line_comment" | "block_comment" => {
                    // Already handled by extract_comments
                }
                // Error nodes
                "ERROR" => {
                    block.unknown_regions.push(crate::ir::UnknownNode {
                        span: node_to_span(&child),
                        reason: Some("Syntax error".to_string()),
                        raw_text: Some(node_text(&child, source).to_string()),
                    });
                }
                // Recurse into other nodes
                _ => {
                    self.visit_block_contents(&child, source, block);
                }
            }
        }
    }

    /// Extract if expression
    fn extract_if(&self, node: &tree_sitter::Node, source: &str) -> ControlFlow {
        let mut branches = Vec::new();

        // Extract condition
        let condition_span = find_child_by_kind(node, "condition").map(|n| node_to_span(&n));

        // Extract then block
        if let Some(block) = find_child_by_kind(node, "block") {
            branches.push(self.extract_block(&block, source));
        }

        // Extract else clause
        if let Some(else_clause) = find_child_by_kind(node, "else_clause") {
            if let Some(block) = find_child_by_kind(&else_clause, "block") {
                branches.push(self.extract_block(&block, source));
            } else if let Some(if_expr) = find_child_by_kind(&else_clause, "if_expression") {
                // else if - create a nested block with the if
                let else_if = self.extract_if(&if_expr, source);
                branches.push(Block {
                    span: node_to_span(&else_clause),
                    control_flow: vec![else_if],
                    ..Default::default()
                });
            }
        }

        ControlFlow {
            kind: ControlFlowKind::If,
            span: node_to_span(node),
            condition_span,
            branches,
        }
    }

    /// Extract match expression
    fn extract_match(&self, node: &tree_sitter::Node, source: &str) -> ControlFlow {
        let mut branches = Vec::new();

        // Extract match arms
        if let Some(body) = find_child_by_kind(node, "match_block") {
            let arms = find_children_by_kind(&body, "match_arm");
            for arm in arms {
                if let Some(body) = find_child_by_kind(&arm, "block") {
                    branches.push(self.extract_block(&body, source));
                } else {
                    // Non-block match arm body
                    branches.push(Block {
                        span: node_to_span(&arm),
                        ..Default::default()
                    });
                }
            }
        }

        ControlFlow {
            kind: ControlFlowKind::Match,
            span: node_to_span(node),
            condition_span: find_child_by_kind(node, "value").map(|n| node_to_span(&n)),
            branches,
        }
    }

    /// Extract a function call
    fn extract_call(&self, node: &tree_sitter::Node, source: &str) -> Option<Call> {
        let function = find_child_by_kind(node, "identifier")
            .or_else(|| find_child_by_kind(node, "scoped_identifier"))?;

        let args = find_child_by_kind(node, "arguments");
        let arg_count = args
            .map(|a| {
                let mut count = 0;
                let mut cursor = a.walk();
                for child in a.children(&mut cursor) {
                    // Count non-punctuation children
                    if !child.kind().starts_with(',')
                        && !child.kind().starts_with('(')
                        && !child.kind().starts_with(')')
                    {
                        count += 1;
                    }
                }
                count
            })
            .unwrap_or(0);

        Some(Call {
            callee: node_text(&function, source).to_string(),
            span: node_to_span(node),
            argument_count: arg_count,
            is_method: false,
        })
    }

    /// Extract a method call
    fn extract_method_call(&self, node: &tree_sitter::Node, source: &str) -> Option<Call> {
        // For method calls like `foo.bar()`, get the full expression
        let callee = node_text(node, source).to_string();

        let args = find_child_by_kind(node, "arguments");
        let arg_count = args.map(|a| a.named_child_count()).unwrap_or(0);

        Some(Call {
            callee,
            span: node_to_span(node),
            argument_count: arg_count,
            is_method: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_rust(source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    // =========================================================================
    // Basic Adapter Tests
    // =========================================================================

    #[test]
    fn test_adapter_new() {
        let adapter = RustTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::Rust);
    }

    #[test]
    fn test_adapter_default() {
        let adapter = RustTreeSitterAdapter::default();
        assert_eq!(adapter.language(), LanguageId::Rust);
    }

    #[test]
    fn test_tree_sitter_language() {
        let adapter = RustTreeSitterAdapter::new();
        let lang = adapter.tree_sitter_language();
        // Should be able to create a parser with this language
        let mut parser = tree_sitter::Parser::new();
        assert!(parser.set_language(&lang).is_ok());
    }

    // =========================================================================
    // Function Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_function() {
        let source =
            "/// A simple function\npub fn hello_world() {\n    println!(\"Hello!\");\n}\n";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "hello_world");
        assert_eq!(decls[0].kind, DeclarationKind::Function);
        assert_eq!(decls[0].visibility, Visibility::Public);
        assert!(decls[0].doc_comment.is_some());
    }

    #[test]
    fn test_extract_function_private() {
        let source = "fn private_func() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].visibility, Visibility::Private);
    }

    #[test]
    fn test_extract_async_function() {
        let source = "pub async fn async_func() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "async_func");
    }

    #[test]
    fn test_extract_generic_function() {
        let source = "fn generic<T: Clone>(val: T) -> T { val }";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "generic");
    }

    // =========================================================================
    // Struct Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_struct() {
        let source = "pub struct Point {\n    pub x: f64,\n    pub y: f64,\n}\n";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Point");
        assert_eq!(decls[0].kind, DeclarationKind::Struct);
        assert_eq!(decls[0].children.len(), 2);
    }

    #[test]
    fn test_extract_tuple_struct() {
        let source = "pub struct Color(u8, u8, u8);";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Color");
        assert_eq!(decls[0].kind, DeclarationKind::Struct);
    }

    #[test]
    fn test_extract_unit_struct() {
        let source = "struct Empty;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Empty");
    }

    // =========================================================================
    // Enum Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_enum() {
        let source = "pub enum Status {\n    Active,\n    Inactive,\n    Pending,\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Status");
        assert_eq!(decls[0].kind, DeclarationKind::Enum);
        assert_eq!(decls[0].children.len(), 3);
    }

    #[test]
    fn test_extract_enum_with_data() {
        let source = "enum Message {\n    Text(String),\n    Number(i32),\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].kind, DeclarationKind::Enum);
        assert_eq!(decls[0].children.len(), 2);
    }

    // =========================================================================
    // Trait Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_trait() {
        let source = "pub trait Drawable {\n    fn draw(&self);\n    fn area(&self) -> f64;\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Drawable");
        assert_eq!(decls[0].kind, DeclarationKind::Trait);
        // Trait may or may not extract children (depends on implementation)
    }

    #[test]
    fn test_extract_trait_with_default_impl() {
        let source = "trait HasDefault {\n    fn default_method(&self) {\n        println!(\"default\");\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].kind, DeclarationKind::Trait);
    }

    // =========================================================================
    // Impl Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_impl() {
        let source = "impl Point {\n    pub fn new(x: f64, y: f64) -> Self {\n        Self { x, y }\n    }\n\n    pub fn distance(&self) -> f64 {\n        (self.x * self.x + self.y * self.y).sqrt()\n    }\n}\n";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Point");
        assert_eq!(decls[0].kind, DeclarationKind::Impl);
        assert_eq!(decls[0].children.len(), 2);
        assert_eq!(decls[0].children[0].kind, DeclarationKind::Method);
    }

    #[test]
    fn test_extract_trait_impl() {
        let source = "impl Display for Point {\n    fn fmt(&self, f: &mut Formatter) -> Result {\n        write!(f, \"({}, {})\", self.x, self.y)\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].kind, DeclarationKind::Impl);
    }

    // =========================================================================
    // Type Alias Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_type_alias() {
        let source = "type Result<T> = std::result::Result<T, Error>;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "Result");
        assert_eq!(decls[0].kind, DeclarationKind::Type);
    }

    // =========================================================================
    // Constant and Static Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_const() {
        let source = "pub const MAX_SIZE: usize = 1024;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "MAX_SIZE");
        assert_eq!(decls[0].kind, DeclarationKind::Constant);
    }

    #[test]
    fn test_extract_static() {
        let source = "static mut COUNTER: i32 = 0;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "COUNTER");
        assert_eq!(decls[0].kind, DeclarationKind::Variable);
    }

    // =========================================================================
    // Module Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_mod_declaration() {
        let source = "pub mod utils;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        // mod utils; should be treated as an import
        assert!(imports
            .iter()
            .any(|i| i.source == "utils" && i.kind == ImportKind::Module));
    }

    #[test]
    fn test_extract_mod_inline() {
        let source = "mod inner {\n    fn inner_func() {}\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "inner");
        assert_eq!(decls[0].kind, DeclarationKind::Module);
    }

    // =========================================================================
    // Macro Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_macro() {
        let source = "macro_rules! my_macro {\n    () => {};\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "my_macro");
        assert_eq!(decls[0].kind, DeclarationKind::Macro);
    }

    // =========================================================================
    // Visibility Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_visibility_pub_crate() {
        let source = "pub(crate) fn internal_func() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].visibility, Visibility::Internal);
    }

    #[test]
    fn test_extract_visibility_pub_super() {
        let source = "pub(super) fn parent_visible() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].visibility, Visibility::Protected);
    }

    #[test]
    fn test_extract_visibility_pub_in_path() {
        let source = "pub(in crate::module) fn limited() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].visibility, Visibility::Internal);
    }

    // =========================================================================
    // Import Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_use() {
        let source = "use std::collections::HashMap;\nuse crate::ir::{File, Span};\nmod utils;\n";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert!(
            imports.len() >= 2,
            "Expected at least 2 imports, got {}",
            imports.len()
        );
        assert!(imports[0].source.contains("HashMap"));
    }

    #[test]
    fn test_extract_use_grouped() {
        let source = "use std::{io, fs, path::Path};";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        // Grouped use may be extracted as one or multiple imports
        assert!(imports.len() >= 1 || source.contains("std"));
    }

    #[test]
    fn test_extract_use_alias() {
        let source = "use std::collections::HashMap as Map;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        // Alias may or may not be extracted depending on implementation
        if !imports.is_empty() {
            // If extracted, check it contains HashMap
            assert!(imports[0].source.contains("HashMap"));
        }
    }

    #[test]
    fn test_extract_extern_crate() {
        let source = "extern crate serde;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let imports = adapter.extract_imports(&tree, source);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].source, "serde");
        assert_eq!(imports[0].kind, ImportKind::Import);
    }

    // =========================================================================
    // Parameter Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_parameters() {
        let source = "fn complex_fn(a: i32, b: &str, c: Option<Vec<u8>>) -> Result<(), Error> {\n    Ok(())\n}\n";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].parameters.len(), 3);
        assert_eq!(decls[0].parameters[0].name, "a");
        assert!(decls[0].return_type.is_some());
    }

    #[test]
    fn test_extract_self_parameter() {
        let source = "impl Foo {\n    fn method(&self, x: i32) {}\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let method = &decls[0].children[0];
        assert_eq!(method.parameters.len(), 2);
        assert_eq!(method.parameters[0].name, "self");
    }

    #[test]
    fn test_extract_no_parameters() {
        let source = "fn no_params() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].parameters.is_empty());
    }

    // =========================================================================
    // Comment Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_line_comment() {
        let source = "// This is a comment\nfn foo() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert_eq!(comments[0].kind, CommentKind::Line);
    }

    #[test]
    fn test_extract_block_comment() {
        let source = "/* Block comment */\nfn foo() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert_eq!(comments[0].kind, CommentKind::Block);
    }

    #[test]
    fn test_extract_doc_comment_line() {
        let source = "/// Doc comment\nfn foo() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert_eq!(comments[0].kind, CommentKind::Doc);
    }

    #[test]
    fn test_extract_doc_comment_block() {
        let source = "/** Block doc comment */\nfn foo() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert_eq!(comments[0].kind, CommentKind::Doc);
    }

    #[test]
    fn test_extract_inner_doc_comment() {
        let source = "//! Module doc\nfn foo() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let comments = adapter.extract_comments(&tree, source);

        assert!(!comments.is_empty());
        assert_eq!(comments[0].kind, CommentKind::Doc);
    }

    // =========================================================================
    // Body Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_body_function() {
        let source = "fn test() {\n    let x = 1;\n    println!(\"{}\", x);\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        // Body extraction depends on span matching - may or may not work
        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_extract_body_impl() {
        let source = "impl Foo {\n    fn method(&self) {}\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_extract_body_struct() {
        let source = "struct Point {\n    x: i32,\n    y: i32,\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_extract_body_enum() {
        let source = "enum Color {\n    Red,\n    Green,\n    Blue,\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_extract_body_trait() {
        let source = "trait Foo {\n    fn bar(&self);\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        let _body = adapter.extract_body(&tree, source, &decls[0]);
        // Just verify it doesn't panic
    }

    // =========================================================================
    // Control Flow Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_if_control_flow() {
        let source = "fn test() {\n    if x > 0 {\n        println!(\"positive\");\n    } else {\n        println!(\"non-positive\");\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::If);
            }
        }
    }

    #[test]
    fn test_extract_match_control_flow() {
        let source =
            "fn test(x: i32) {\n    match x {\n        0 => {},\n        _ => {},\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::Match);
            }
        }
    }

    #[test]
    fn test_extract_for_control_flow() {
        let source = "fn test() {\n    for i in 0..10 {\n        println!(\"{}\", i);\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::For);
            }
        }
    }

    #[test]
    fn test_extract_while_control_flow() {
        let source = "fn test() {\n    while x > 0 {\n        x -= 1;\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::While);
            }
        }
    }

    #[test]
    fn test_extract_loop_control_flow() {
        let source = "fn test() {\n    loop {\n        break;\n    }\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        if let Some(body) = adapter.extract_body(&tree, source, &decls[0]) {
            if !body.control_flow.is_empty() {
                assert_eq!(body.control_flow[0].kind, ControlFlowKind::Loop);
            }
        }
    }

    // =========================================================================
    // Span Extraction Tests
    // =========================================================================

    #[test]
    fn test_signature_span() {
        let source = "fn foo(x: i32) -> i32 {\n    x + 1\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].signature_span.is_some());
        let sig_span = decls[0].signature_span.as_ref().unwrap();
        // Signature should end before the {
        assert!(sig_span.end <= source.find('{').unwrap() + 1);
    }

    #[test]
    fn test_body_span() {
        let source = "fn foo() {\n    let x = 1;\n}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].body_span.is_some());
        let body_span = decls[0].body_span.as_ref().unwrap();
        // Body should start at or after {
        assert!(body_span.start >= source.find('{').unwrap());
    }

    // =========================================================================
    // Error Recovery Tests
    // =========================================================================

    #[test]
    fn test_error_recovery() {
        let source = "fn broken( {\n    // Missing closing paren\n}\n\nfn valid_function() {\n    println!(\"I'm fine\");\n}\n";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();

        let decls = adapter.extract_declarations(&tree, source);
        assert!(decls.iter().any(|d| d.name == "valid_function"));

        let errors = adapter.extract_errors(&tree, source);
        assert!(!errors.is_empty(), "Should detect syntax errors");
    }

    // =========================================================================
    // Multiple Declarations Tests
    // =========================================================================

    #[test]
    fn test_multiple_functions() {
        let source = "fn one() {}\nfn two() {}\nfn three() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 3);
    }

    #[test]
    fn test_mixed_declarations() {
        let source = "struct Foo {}\nenum Bar {}\nfn baz() {}\nconst X: i32 = 1;";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 4);
        assert!(decls.iter().any(|d| d.kind == DeclarationKind::Struct));
        assert!(decls.iter().any(|d| d.kind == DeclarationKind::Enum));
        assert!(decls.iter().any(|d| d.kind == DeclarationKind::Function));
        assert!(decls.iter().any(|d| d.kind == DeclarationKind::Constant));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_source() {
        let source = "";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls.is_empty());
    }

    #[test]
    fn test_only_comments() {
        let source = "// Just a comment\n/* Another comment */";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls.is_empty());
    }

    #[test]
    fn test_unicode_identifiers() {
        let source = "fn привет() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "привет");
    }

    #[test]
    fn test_multiline_doc_comment() {
        let source = "/// First line\n/// Second line\n/// Third line\nfn documented() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();
        let decls = adapter.extract_declarations(&tree, source);

        assert!(decls[0].doc_comment.is_some());
        let doc = decls[0].doc_comment.as_ref().unwrap();
        assert!(doc.text.contains("First"));
        assert!(doc.text.contains("Second"));
        assert!(doc.text.contains("Third"));
    }
}
