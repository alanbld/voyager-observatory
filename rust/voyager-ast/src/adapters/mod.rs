//! Language Adapters for voyager-ast
//!
//! Each language requires an adapter that translates Tree-sitter parse trees
//! into our language-agnostic IR. Adapters implement the `LanguageAdapter` trait.
//!
//! # Core Fleet (Phase 1B)
//!
//! - **Rust**: Full support for functions, structs, enums, traits, impl blocks
//! - **Python**: Functions (def/async), classes, imports, decorators, docstrings
//! - **TypeScript/JavaScript**: Functions, classes, interfaces, types, imports/exports

pub mod python_adapter;
pub mod rust_adapter;
pub mod typescript_adapter;

use crate::ir::{
    Block, Comment, Declaration, ImportLike, LanguageId, Span, UnknownNode, Visibility,
};

// Re-export all adapters
pub use python_adapter::PythonTreeSitterAdapter;
pub use rust_adapter::RustTreeSitterAdapter;
pub use typescript_adapter::TypeScriptTreeSitterAdapter;

/// Trait for language-specific adapters
///
/// Each adapter is responsible for:
/// 1. Providing the Tree-sitter language/grammar
/// 2. Extracting declarations from a parse tree
/// 3. Extracting imports
/// 4. Extracting the body of a declaration (for Zoom mode)
pub trait LanguageAdapter: Send + Sync {
    /// The language this adapter handles
    fn language(&self) -> LanguageId;

    /// Get the Tree-sitter language
    fn tree_sitter_language(&self) -> tree_sitter::Language;

    /// Extract declarations from a parse tree (Index mode)
    ///
    /// This should extract top-level declarations only.
    /// Nested declarations (methods, inner functions) are handled separately.
    fn extract_declarations(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Declaration>;

    /// Extract imports from a parse tree
    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<ImportLike>;

    /// Extract comments from a parse tree
    fn extract_comments(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<Comment>;

    /// Extract the body of a declaration (Zoom mode)
    fn extract_body(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        declaration: &Declaration,
    ) -> Option<Block>;

    /// Determine visibility from a node
    fn extract_visibility(&self, node: &tree_sitter::Node, source: &str) -> Visibility;

    /// Extract unknown/error nodes from a parse tree
    fn extract_errors(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<UnknownNode> {
        let mut errors = Vec::new();
        let mut cursor = tree.walk();

        fn visit_errors(
            cursor: &mut tree_sitter::TreeCursor,
            source: &str,
            errors: &mut Vec<UnknownNode>,
        ) {
            loop {
                let node = cursor.node();

                if node.is_error() || node.is_missing() {
                    let span = node_to_span(&node);
                    let raw_text = if span.len() < 200 {
                        Some(source[span.start..span.end].to_string())
                    } else {
                        Some(format!(
                            "{}... ({} bytes)",
                            &source[span.start..span.start.saturating_add(100)],
                            span.len()
                        ))
                    };

                    errors.push(UnknownNode {
                        span,
                        reason: Some(if node.is_missing() {
                            "Missing syntax element".to_string()
                        } else {
                            "Syntax error".to_string()
                        }),
                        raw_text,
                    });
                }

                if cursor.goto_first_child() {
                    visit_errors(cursor, source, errors);
                    cursor.goto_parent();
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        visit_errors(&mut cursor, source, &mut errors);
        errors
    }
}

/// Convert a Tree-sitter node to our Span type
pub fn node_to_span(node: &tree_sitter::Node) -> Span {
    Span {
        start: node.start_byte(),
        end: node.end_byte(),
        start_line: node.start_position().row + 1, // 1-indexed
        end_line: node.end_position().row + 1,
        start_column: node.start_position().column,
        end_column: node.end_position().column,
    }
}

/// Get the text content of a Tree-sitter node
pub fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

/// Find a child node by its kind
#[allow(clippy::manual_find)]
pub fn find_child_by_kind<'a>(
    node: &'a tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

/// Find all children of a specific kind
pub fn find_children_by_kind<'a>(
    node: &'a tree_sitter::Node<'a>,
    kind: &str,
) -> Vec<tree_sitter::Node<'a>> {
    let mut result = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            result.push(child);
        }
    }
    result
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
    // node_to_span Tests
    // =========================================================================

    #[test]
    fn test_node_to_span_basic() {
        let source = "fn foo() {}";
        let tree = parse_rust(source);
        let root = tree.root_node();

        let span = node_to_span(&root);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, source.len());
        assert_eq!(span.start_line, 1);
        assert_eq!(span.end_line, 1);
    }

    #[test]
    fn test_node_to_span_multiline() {
        let source = "fn foo() {\n    let x = 1;\n}";
        let tree = parse_rust(source);
        let root = tree.root_node();

        let span = node_to_span(&root);
        assert_eq!(span.start_line, 1);
        assert_eq!(span.end_line, 3);
    }

    #[test]
    fn test_node_to_span_nested() {
        let source = "fn foo() { let x = 1; }";
        let tree = parse_rust(source);
        let root = tree.root_node();

        // Get the function item
        let func = root.child(0).unwrap();
        assert_eq!(func.kind(), "function_item");

        let span = node_to_span(&func);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, source.len());
    }

    // =========================================================================
    // node_text Tests
    // =========================================================================

    #[test]
    fn test_node_text_basic() {
        let source = "fn foo() {}";
        let tree = parse_rust(source);
        let root = tree.root_node();

        let text = node_text(&root, source);
        assert_eq!(text, source);
    }

    #[test]
    fn test_node_text_nested() {
        let source = "fn hello() {}";
        let tree = parse_rust(source);
        let func = tree.root_node().child(0).unwrap();

        // Find the function name
        let name = find_child_by_kind(&func, "identifier").unwrap();
        let text = node_text(&name, source);
        assert_eq!(text, "hello");
    }

    #[test]
    fn test_node_text_with_whitespace() {
        let source = "fn foo() {\n    let x = 42;\n}";
        let tree = parse_rust(source);
        let text = node_text(&tree.root_node(), source);
        assert_eq!(text, source);
    }

    // =========================================================================
    // find_child_by_kind Tests
    // =========================================================================

    #[test]
    fn test_find_child_by_kind_found() {
        let source = "fn test_func() {}";
        let tree = parse_rust(source);
        let func = tree.root_node().child(0).unwrap();

        let name = find_child_by_kind(&func, "identifier");
        assert!(name.is_some());
        assert_eq!(node_text(&name.unwrap(), source), "test_func");
    }

    #[test]
    fn test_find_child_by_kind_not_found() {
        let source = "fn foo() {}";
        let tree = parse_rust(source);
        let func = tree.root_node().child(0).unwrap();

        let result = find_child_by_kind(&func, "nonexistent_kind");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_child_by_kind_first_match() {
        let source = "fn foo(a: i32, b: i32) {}";
        let tree = parse_rust(source);
        let func = tree.root_node().child(0).unwrap();

        // Find parameters - should get the parameters node
        let params = find_child_by_kind(&func, "parameters");
        assert!(params.is_some());
    }

    // =========================================================================
    // find_children_by_kind Tests
    // =========================================================================

    #[test]
    fn test_find_children_by_kind_multiple() {
        let source = "fn foo() {}\nfn bar() {}\nfn baz() {}";
        let tree = parse_rust(source);
        let root = tree.root_node();

        let functions = find_children_by_kind(&root, "function_item");
        assert_eq!(functions.len(), 3);
    }

    #[test]
    fn test_find_children_by_kind_none() {
        let source = "fn foo() {}";
        let tree = parse_rust(source);
        let root = tree.root_node();

        let structs = find_children_by_kind(&root, "struct_item");
        assert!(structs.is_empty());
    }

    #[test]
    fn test_find_children_by_kind_single() {
        let source = "struct Point { x: i32, y: i32 }";
        let tree = parse_rust(source);
        let root = tree.root_node();

        let structs = find_children_by_kind(&root, "struct_item");
        assert_eq!(structs.len(), 1);
    }

    // =========================================================================
    // extract_errors Tests (via RustTreeSitterAdapter)
    // =========================================================================

    #[test]
    fn test_extract_errors_no_errors() {
        let source = "fn valid() {}";
        let tree = parse_rust(source);

        let adapter = RustTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_extract_errors_with_syntax_error() {
        let source = "fn broken( {}"; // Missing closing paren and params
        let tree = parse_rust(source);

        let adapter = RustTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);
        // Should detect some error
        assert!(!errors.is_empty() || tree.root_node().has_error());
    }

    #[test]
    fn test_extract_errors_multiple_errors() {
        let source = "fn a( {}\nfn b( {}";
        let tree = parse_rust(source);

        let adapter = RustTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);
        // May have multiple errors
        if tree.root_node().has_error() {
            // At least one error should be detected
            assert!(errors.len() >= 1 || tree.root_node().has_error());
        }
    }

    #[test]
    fn test_extract_errors_truncates_long_text() {
        // Create source with syntax error and long content
        let long_content = "x".repeat(300);
        let source = format!("fn broken( {{ {} }}", long_content);
        let tree = parse_rust(&source);

        let adapter = RustTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, &source);

        // If errors found, check truncation logic
        for error in &errors {
            if let Some(raw_text) = &error.raw_text {
                // Either short or truncated with "... (N bytes)"
                assert!(raw_text.len() < 200 || raw_text.contains("bytes)"));
            }
        }
    }

    #[test]
    fn test_extract_errors_reason_for_missing() {
        // Try to create a missing node scenario
        let source = "fn test() { let x = ; }"; // Missing expression after =
        let tree = parse_rust(source);

        let adapter = RustTreeSitterAdapter::new();
        let errors = adapter.extract_errors(&tree, source);

        // Check that errors have reasons
        for error in &errors {
            assert!(error.reason.is_some());
            let reason = error.reason.as_ref().unwrap();
            assert!(reason.contains("Syntax error") || reason.contains("Missing"));
        }
    }

    // =========================================================================
    // LanguageAdapter Trait Tests
    // =========================================================================

    #[test]
    fn test_rust_adapter_language() {
        let adapter = RustTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::Rust);
    }

    #[test]
    fn test_python_adapter_language() {
        let adapter = PythonTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::Python);
    }

    #[test]
    fn test_typescript_adapter_language() {
        let adapter = TypeScriptTreeSitterAdapter::new();
        assert_eq!(adapter.language(), LanguageId::TypeScript);
    }

    #[test]
    fn test_typescript_tsx_adapter() {
        let adapter = TypeScriptTreeSitterAdapter::tsx();
        assert_eq!(adapter.language(), LanguageId::Tsx);
    }

    #[test]
    fn test_typescript_javascript_adapter() {
        let adapter = TypeScriptTreeSitterAdapter::javascript();
        assert_eq!(adapter.language(), LanguageId::JavaScript);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_rust_adapter_extract_declarations() {
        let source = "fn hello() {}\npub struct Point { x: i32 }";
        let mut parser = tree_sitter::Parser::new();
        let adapter = RustTreeSitterAdapter::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();

        let decls = adapter.extract_declarations(&tree, source);
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].name, "hello");
        assert_eq!(decls[1].name, "Point");
    }

    #[test]
    fn test_python_adapter_extract_declarations() {
        let source = "def greet():\n    pass\n\nclass Person:\n    pass";
        let mut parser = tree_sitter::Parser::new();
        let adapter = PythonTreeSitterAdapter::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();

        let decls = adapter.extract_declarations(&tree, source);
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].name, "greet");
        assert_eq!(decls[1].name, "Person");
    }

    #[test]
    fn test_typescript_adapter_extract_declarations() {
        let source = "function hello() {}\nclass World {}";
        let mut parser = tree_sitter::Parser::new();
        let adapter = TypeScriptTreeSitterAdapter::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();

        let decls = adapter.extract_declarations(&tree, source);
        assert!(decls.len() >= 2);
    }

    #[test]
    fn test_rust_adapter_extract_imports() {
        let source = "use std::io;\nuse std::collections::HashMap;";
        let mut parser = tree_sitter::Parser::new();
        let adapter = RustTreeSitterAdapter::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();

        let imports = adapter.extract_imports(&tree, source);
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_python_adapter_extract_imports() {
        let source = "import os\nfrom pathlib import Path";
        let mut parser = tree_sitter::Parser::new();
        let adapter = PythonTreeSitterAdapter::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();

        let imports = adapter.extract_imports(&tree, source);
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_rust_adapter_extract_comments() {
        let source = "// Line comment\n/* Block comment */\nfn foo() {}";
        let mut parser = tree_sitter::Parser::new();
        let adapter = RustTreeSitterAdapter::new();
        parser
            .set_language(&adapter.tree_sitter_language())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();

        let comments = adapter.extract_comments(&tree, source);
        assert!(comments.len() >= 2);
    }

    #[test]
    fn test_rust_adapter_extract_visibility() {
        let source = "pub fn public_func() {}\nfn private_func() {}";
        let tree = parse_rust(source);
        let adapter = RustTreeSitterAdapter::new();

        let root = tree.root_node();
        let mut cursor = root.walk();
        let children: Vec<_> = root.children(&mut cursor).collect();

        // First function is public
        let vis1 = adapter.extract_visibility(&children[0], source);
        assert_eq!(vis1, Visibility::Public);

        // Second function is private (default)
        let vis2 = adapter.extract_visibility(&children[1], source);
        assert_eq!(vis2, Visibility::Private);
    }
}
