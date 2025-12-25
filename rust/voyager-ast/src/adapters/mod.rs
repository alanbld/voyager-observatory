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

pub mod rust_adapter;
pub mod python_adapter;
pub mod typescript_adapter;

use crate::error::Result;
use crate::ir::{
    Block, Comment, Declaration, ImportLike, LanguageId, Span, UnknownNode, Visibility,
};

// Re-export all adapters
pub use rust_adapter::RustTreeSitterAdapter;
pub use python_adapter::PythonTreeSitterAdapter;
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
    fn extract_declarations(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<Declaration>;

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

    #[test]
    fn test_node_to_span() {
        // This would need a real tree-sitter parse to test properly
        // For now, just ensure the function signature is correct
    }
}
