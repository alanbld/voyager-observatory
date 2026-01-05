//! Normalized AST - Language-Agnostic Code Representation
//!
//! This module defines a unified data model for representing code structure
//! across all 25 supported languages. The NormalizedAst serves as the
//! "lingua franca" between Tree-sitter parsers and the Voyager Observatory.
//!
//! # Design Philosophy
//!
//! Different languages have wildly different syntax and semantics:
//! - Rust has traits, Python has decorators, TypeScript has interfaces
//! - Some languages use classes, others use modules, others use both
//! - Visibility rules differ (public/private vs. export/import)
//!
//! The NormalizedAst abstracts these differences into a common vocabulary:
//! - **Symbol**: Any named code entity (function, class, variable, etc.)
//! - **Import**: Any dependency on external code
//! - **Module**: Any grouping of related symbols
//! - **Scope**: The visibility/accessibility of a symbol

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A language-agnostic representation of parsed source code
///
/// This is the primary output of syntax analysis. It contains all
/// extracted symbols, imports, and structural information in a
/// normalized format that can be processed uniformly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NormalizedAst {
    /// All symbols found in the source
    pub symbols: Vec<Symbol>,

    /// All import/require statements
    pub imports: Vec<Import>,

    /// Module structure (for languages with explicit modules)
    pub modules: Vec<Module>,

    /// File-level documentation
    pub doc_comment: Option<String>,

    /// Language-specific metadata
    pub metadata: HashMap<String, String>,

    /// Parse errors (non-fatal)
    pub errors: Vec<ParseDiagnostic>,
}

impl NormalizedAst {
    /// Create an empty AST
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all symbols of a specific kind
    pub fn symbols_of_kind(&self, kind: SymbolKind) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.kind == kind).collect()
    }

    /// Get all public/exported symbols
    pub fn public_symbols(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| matches!(s.visibility, SymbolVisibility::Public | SymbolVisibility::Export))
            .collect()
    }

    /// Get all functions (including methods)
    pub fn functions(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .collect()
    }

    /// Get all classes/structs/types
    pub fn types(&self) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Class
                        | SymbolKind::Struct
                        | SymbolKind::Interface
                        | SymbolKind::Trait
                        | SymbolKind::Enum
                        | SymbolKind::TypeAlias
                )
            })
            .collect()
    }

    /// Find a symbol by name (first match)
    pub fn find_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Find all symbols matching a pattern
    pub fn find_symbols(&self, pattern: &str) -> Vec<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| s.name.contains(pattern))
            .collect()
    }

    /// Get the total line count covered by symbols
    pub fn symbol_line_coverage(&self) -> usize {
        self.symbols
            .iter()
            .filter_map(|s| s.span.as_ref())
            .map(|span| span.end_line.saturating_sub(span.start_line) + 1)
            .sum()
    }

    /// Check if the AST has any parse errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Merge another AST into this one
    pub fn merge(&mut self, other: NormalizedAst) {
        self.symbols.extend(other.symbols);
        self.imports.extend(other.imports);
        self.modules.extend(other.modules);
        self.errors.extend(other.errors);

        for (key, value) in other.metadata {
            self.metadata.entry(key).or_insert(value);
        }
    }
}

/// A code symbol (function, class, variable, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// The symbol's name
    pub name: String,

    /// What kind of symbol this is
    pub kind: SymbolKind,

    /// Visibility/accessibility
    pub visibility: SymbolVisibility,

    /// Location in source file
    pub location: Location,

    /// Full span of the symbol (start to end)
    pub span: Option<Span>,

    /// Documentation comment
    pub doc_comment: Option<String>,

    /// Parent symbol (for nested items)
    pub parent: Option<String>,

    /// Child symbols (for containers like classes)
    pub children: Vec<String>,

    /// Type signature (if available)
    pub signature: Option<String>,

    /// Function parameters (for functions/methods)
    pub parameters: Vec<Parameter>,

    /// Return type (for functions/methods)
    pub return_type: Option<String>,

    /// Decorators/attributes (Python decorators, Rust attributes, etc.)
    pub decorators: Vec<String>,

    /// Generic type parameters
    pub type_parameters: Vec<String>,

    /// Language-specific metadata
    pub metadata: HashMap<String, String>,
}

impl Symbol {
    /// Create a new symbol with minimal information
    pub fn new(name: impl Into<String>, kind: SymbolKind, location: Location) -> Self {
        Self {
            name: name.into(),
            kind,
            visibility: SymbolVisibility::default(),
            location,
            span: None,
            doc_comment: None,
            parent: None,
            children: Vec::new(),
            signature: None,
            parameters: Vec::new(),
            return_type: None,
            decorators: Vec::new(),
            type_parameters: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Check if this symbol is a container (can have children)
    pub fn is_container(&self) -> bool {
        matches!(
            self.kind,
            SymbolKind::Class
                | SymbolKind::Struct
                | SymbolKind::Interface
                | SymbolKind::Trait
                | SymbolKind::Module
                | SymbolKind::Namespace
                | SymbolKind::Enum
        )
    }

    /// Check if this symbol is callable
    pub fn is_callable(&self) -> bool {
        matches!(
            self.kind,
            SymbolKind::Function | SymbolKind::Method | SymbolKind::Constructor
        )
    }

    /// Get the fully qualified name (parent.name)
    pub fn qualified_name(&self) -> String {
        match &self.parent {
            Some(parent) => format!("{}.{}", parent, self.name),
            None => self.name.clone(),
        }
    }
}

/// The type of a code symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    // Callables
    Function,
    Method,
    Constructor,
    Lambda,

    // Types
    Class,
    Struct,
    Interface,
    Trait,
    Enum,
    TypeAlias,

    // Data
    Variable,
    Constant,
    Field,
    Property,
    EnumVariant,

    // Containers
    Module,
    Namespace,
    Package,

    // Other
    Macro,
    Decorator,
    Attribute,
    Unknown,
}

impl SymbolKind {
    /// Get a short label for this kind
    pub fn label(&self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Method => "method",
            SymbolKind::Constructor => "ctor",
            SymbolKind::Lambda => "lambda",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Interface => "interface",
            SymbolKind::Trait => "trait",
            SymbolKind::Enum => "enum",
            SymbolKind::TypeAlias => "type",
            SymbolKind::Variable => "var",
            SymbolKind::Constant => "const",
            SymbolKind::Field => "field",
            SymbolKind::Property => "prop",
            SymbolKind::EnumVariant => "variant",
            SymbolKind::Module => "mod",
            SymbolKind::Namespace => "ns",
            SymbolKind::Package => "pkg",
            SymbolKind::Macro => "macro",
            SymbolKind::Decorator => "decorator",
            SymbolKind::Attribute => "attr",
            SymbolKind::Unknown => "?",
        }
    }

    /// Get the icon/emoji for this kind
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::Function | SymbolKind::Method => "⚡",
            SymbolKind::Constructor => "🔨",
            SymbolKind::Lambda => "λ",
            SymbolKind::Class => "📦",
            SymbolKind::Struct => "🔳",
            SymbolKind::Interface | SymbolKind::Trait => "🔌",
            SymbolKind::Enum => "📋",
            SymbolKind::TypeAlias => "📐",
            SymbolKind::Variable => "📝",
            SymbolKind::Constant => "🔒",
            SymbolKind::Field | SymbolKind::Property => "•",
            SymbolKind::EnumVariant => "◦",
            SymbolKind::Module | SymbolKind::Namespace | SymbolKind::Package => "📁",
            SymbolKind::Macro => "⚙️",
            SymbolKind::Decorator | SymbolKind::Attribute => "@",
            SymbolKind::Unknown => "?",
        }
    }
}

/// Visibility/accessibility of a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SymbolVisibility {
    /// Public to all (Rust pub, TypeScript export, Python __all__)
    Public,

    /// Exported from module (similar to Public but explicit)
    Export,

    /// Private to containing scope
    Private,

    /// Protected (accessible to subclasses)
    Protected,

    /// Internal (package/crate private)
    Internal,

    /// Visibility not specified or not applicable
    #[default]
    Unspecified,
}

/// An import/require statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// What is being imported
    pub source: String,

    /// How it's imported
    pub kind: ImportKind,

    /// Alias if renamed (import X as Y)
    pub alias: Option<String>,

    /// Specific items imported (for selective imports)
    pub items: Vec<String>,

    /// Location in source
    pub location: Location,

    /// Whether this is a type-only import (TypeScript)
    pub type_only: bool,
}

/// The kind of import statement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportKind {
    /// Import everything (import * from X, use X::*)
    Wildcard,

    /// Import specific items (from X import a, b)
    Selective,

    /// Import the module itself (import X)
    Module,

    /// Re-export (export { X } from Y)
    ReExport,

    /// Side-effect import (import 'polyfill')
    SideEffect,
}

/// A module/namespace declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Module name
    pub name: String,

    /// Nested path (for nested modules)
    pub path: Vec<String>,

    /// Location in source
    pub location: Location,

    /// Documentation
    pub doc_comment: Option<String>,

    /// Visibility
    pub visibility: SymbolVisibility,
}

/// A scope or block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    /// Scope kind (function, class, block, etc.)
    pub kind: String,

    /// Span of the scope
    pub span: Span,

    /// Parent scope index
    pub parent: Option<usize>,

    /// Symbols defined in this scope
    pub symbols: Vec<usize>,
}

/// Location in source code (single point)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,

    /// Byte offset in source
    pub offset: usize,
}

impl Location {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
}

/// Span in source code (range)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Start line (1-indexed)
    pub start_line: usize,

    /// Start column (1-indexed)
    pub start_column: usize,

    /// End line (1-indexed)
    pub end_line: usize,

    /// End column (1-indexed)
    pub end_column: usize,

    /// Start byte offset
    pub start_offset: usize,

    /// End byte offset
    pub end_offset: usize,
}

impl Span {
    pub fn new(
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
            start_offset: 0,
            end_offset: 0,
        }
    }

    /// Get the number of lines in this span
    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    /// Check if this span contains a location
    pub fn contains(&self, loc: &Location) -> bool {
        if loc.line < self.start_line || loc.line > self.end_line {
            return false;
        }
        if loc.line == self.start_line && loc.column < self.start_column {
            return false;
        }
        if loc.line == self.end_line && loc.column > self.end_column {
            return false;
        }
        true
    }
}

/// A function/method parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Type annotation (if present)
    pub type_annotation: Option<String>,

    /// Default value (if present)
    pub default_value: Option<String>,

    /// Is this a rest/variadic parameter?
    pub is_rest: bool,

    /// Is this a keyword-only parameter? (Python)
    pub is_keyword_only: bool,
}

/// A parse diagnostic (error or warning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseDiagnostic {
    /// Severity level
    pub severity: DiagnosticSeverity,

    /// Error message
    pub message: String,

    /// Location
    pub location: Location,

    /// Span of affected code
    pub span: Option<Span>,
}

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // NormalizedAst Tests
    // =========================================================================

    #[test]
    fn test_normalized_ast_new() {
        let ast = NormalizedAst::new();
        assert!(ast.symbols.is_empty());
        assert!(ast.imports.is_empty());
        assert!(!ast.has_errors());
    }

    #[test]
    fn test_normalized_ast_default() {
        let ast = NormalizedAst::default();
        assert!(ast.symbols.is_empty());
        assert!(ast.modules.is_empty());
        assert!(ast.doc_comment.is_none());
        assert!(ast.metadata.is_empty());
        assert!(ast.errors.is_empty());
    }

    #[test]
    fn test_symbols_of_kind() {
        let mut ast = NormalizedAst::new();
        ast.symbols.push(Symbol::new("func1", SymbolKind::Function, Location::default()));
        ast.symbols.push(Symbol::new("class1", SymbolKind::Class, Location::default()));
        ast.symbols.push(Symbol::new("func2", SymbolKind::Function, Location::default()));
        ast.symbols.push(Symbol::new("struct1", SymbolKind::Struct, Location::default()));

        let functions = ast.symbols_of_kind(SymbolKind::Function);
        assert_eq!(functions.len(), 2);

        let classes = ast.symbols_of_kind(SymbolKind::Class);
        assert_eq!(classes.len(), 1);

        let traits = ast.symbols_of_kind(SymbolKind::Trait);
        assert_eq!(traits.len(), 0);
    }

    #[test]
    fn test_public_symbols() {
        let mut ast = NormalizedAst::new();

        let mut pub_sym = Symbol::new("public_func", SymbolKind::Function, Location::default());
        pub_sym.visibility = SymbolVisibility::Public;
        ast.symbols.push(pub_sym);

        let mut exp_sym = Symbol::new("exported_func", SymbolKind::Function, Location::default());
        exp_sym.visibility = SymbolVisibility::Export;
        ast.symbols.push(exp_sym);

        let mut priv_sym = Symbol::new("private_func", SymbolKind::Function, Location::default());
        priv_sym.visibility = SymbolVisibility::Private;
        ast.symbols.push(priv_sym);

        let public = ast.public_symbols();
        assert_eq!(public.len(), 2);
    }

    #[test]
    fn test_functions() {
        let mut ast = NormalizedAst::new();
        ast.symbols.push(Symbol::new("func", SymbolKind::Function, Location::default()));
        ast.symbols.push(Symbol::new("method", SymbolKind::Method, Location::default()));
        ast.symbols.push(Symbol::new("class", SymbolKind::Class, Location::default()));
        ast.symbols.push(Symbol::new("ctor", SymbolKind::Constructor, Location::default()));

        let funcs = ast.functions();
        assert_eq!(funcs.len(), 2); // Function and Method, not Constructor
    }

    #[test]
    fn test_types() {
        let mut ast = NormalizedAst::new();
        ast.symbols.push(Symbol::new("MyClass", SymbolKind::Class, Location::default()));
        ast.symbols.push(Symbol::new("MyStruct", SymbolKind::Struct, Location::default()));
        ast.symbols.push(Symbol::new("MyInterface", SymbolKind::Interface, Location::default()));
        ast.symbols.push(Symbol::new("MyTrait", SymbolKind::Trait, Location::default()));
        ast.symbols.push(Symbol::new("MyEnum", SymbolKind::Enum, Location::default()));
        ast.symbols.push(Symbol::new("MyType", SymbolKind::TypeAlias, Location::default()));
        ast.symbols.push(Symbol::new("my_func", SymbolKind::Function, Location::default()));

        let types = ast.types();
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_find_symbol() {
        let mut ast = NormalizedAst::new();
        ast.symbols.push(Symbol::new("alpha", SymbolKind::Function, Location::default()));
        ast.symbols.push(Symbol::new("beta", SymbolKind::Class, Location::default()));

        assert!(ast.find_symbol("alpha").is_some());
        assert!(ast.find_symbol("beta").is_some());
        assert!(ast.find_symbol("gamma").is_none());
    }

    #[test]
    fn test_ast_find_symbols() {
        let mut ast = NormalizedAst::new();

        ast.symbols.push(Symbol::new("calculate_total", SymbolKind::Function, Location::default()));
        ast.symbols.push(Symbol::new("calculate_tax", SymbolKind::Function, Location::default()));
        ast.symbols.push(Symbol::new("other_func", SymbolKind::Function, Location::default()));

        let matches = ast.find_symbols("calculate");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_symbol_line_coverage() {
        let mut ast = NormalizedAst::new();

        let mut sym1 = Symbol::new("func1", SymbolKind::Function, Location::default());
        sym1.span = Some(Span::new(1, 1, 10, 1)); // 10 lines
        ast.symbols.push(sym1);

        let mut sym2 = Symbol::new("func2", SymbolKind::Function, Location::default());
        sym2.span = Some(Span::new(15, 1, 20, 1)); // 6 lines
        ast.symbols.push(sym2);

        // Symbol without span
        ast.symbols.push(Symbol::new("no_span", SymbolKind::Variable, Location::default()));

        assert_eq!(ast.symbol_line_coverage(), 16);
    }

    #[test]
    fn test_has_errors() {
        let mut ast = NormalizedAst::new();
        assert!(!ast.has_errors());

        ast.errors.push(ParseDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: "Test error".to_string(),
            location: Location::default(),
            span: None,
        });
        assert!(ast.has_errors());
    }

    #[test]
    fn test_ast_merge() {
        let mut ast1 = NormalizedAst::new();
        ast1.symbols.push(Symbol::new("a", SymbolKind::Function, Location::default()));
        ast1.imports.push(Import {
            source: "mod1".to_string(),
            kind: ImportKind::Module,
            alias: None,
            items: vec![],
            location: Location::default(),
            type_only: false,
        });
        ast1.metadata.insert("key1".to_string(), "val1".to_string());

        let mut ast2 = NormalizedAst::new();
        ast2.symbols.push(Symbol::new("b", SymbolKind::Function, Location::default()));
        ast2.imports.push(Import {
            source: "mod2".to_string(),
            kind: ImportKind::Selective,
            alias: Some("alias".to_string()),
            items: vec!["item1".to_string()],
            location: Location::default(),
            type_only: true,
        });
        ast2.modules.push(Module {
            name: "mymod".to_string(),
            path: vec!["a".to_string(), "b".to_string()],
            location: Location::default(),
            doc_comment: Some("Module doc".to_string()),
            visibility: SymbolVisibility::Public,
        });
        ast2.errors.push(ParseDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "warn".to_string(),
            location: Location::default(),
            span: None,
        });
        ast2.metadata.insert("key2".to_string(), "val2".to_string());
        // Test that existing key is not overwritten
        ast2.metadata.insert("key1".to_string(), "val1_new".to_string());

        ast1.merge(ast2);
        assert_eq!(ast1.symbols.len(), 2);
        assert_eq!(ast1.imports.len(), 2);
        assert_eq!(ast1.modules.len(), 1);
        assert_eq!(ast1.errors.len(), 1);
        assert_eq!(ast1.metadata.len(), 2);
        assert_eq!(ast1.metadata.get("key1"), Some(&"val1".to_string())); // Original preserved
    }

    // =========================================================================
    // Symbol Tests
    // =========================================================================

    #[test]
    fn test_symbol_creation() {
        let loc = Location::new(1, 1, 0);
        let symbol = Symbol::new("my_function", SymbolKind::Function, loc);

        assert_eq!(symbol.name, "my_function");
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert!(symbol.is_callable());
        assert!(!symbol.is_container());
    }

    #[test]
    fn test_symbol_qualified_name() {
        let loc = Location::new(5, 1, 100);
        let mut symbol = Symbol::new("method", SymbolKind::Method, loc);
        symbol.parent = Some("MyClass".to_string());

        assert_eq!(symbol.qualified_name(), "MyClass.method");
    }

    #[test]
    fn test_symbol_qualified_name_no_parent() {
        let symbol = Symbol::new("standalone", SymbolKind::Function, Location::default());
        assert_eq!(symbol.qualified_name(), "standalone");
    }

    #[test]
    fn test_symbol_is_container() {
        assert!(Symbol::new("", SymbolKind::Class, Location::default()).is_container());
        assert!(Symbol::new("", SymbolKind::Struct, Location::default()).is_container());
        assert!(Symbol::new("", SymbolKind::Interface, Location::default()).is_container());
        assert!(Symbol::new("", SymbolKind::Trait, Location::default()).is_container());
        assert!(Symbol::new("", SymbolKind::Module, Location::default()).is_container());
        assert!(Symbol::new("", SymbolKind::Namespace, Location::default()).is_container());
        assert!(Symbol::new("", SymbolKind::Enum, Location::default()).is_container());

        assert!(!Symbol::new("", SymbolKind::Function, Location::default()).is_container());
        assert!(!Symbol::new("", SymbolKind::Variable, Location::default()).is_container());
    }

    #[test]
    fn test_symbol_is_callable() {
        assert!(Symbol::new("", SymbolKind::Function, Location::default()).is_callable());
        assert!(Symbol::new("", SymbolKind::Method, Location::default()).is_callable());
        assert!(Symbol::new("", SymbolKind::Constructor, Location::default()).is_callable());

        assert!(!Symbol::new("", SymbolKind::Lambda, Location::default()).is_callable());
        assert!(!Symbol::new("", SymbolKind::Class, Location::default()).is_callable());
        assert!(!Symbol::new("", SymbolKind::Variable, Location::default()).is_callable());
    }

    #[test]
    fn test_symbol_with_all_fields() {
        let loc = Location::new(10, 5, 200);
        let mut symbol = Symbol::new("complex_method", SymbolKind::Method, loc);
        symbol.visibility = SymbolVisibility::Public;
        symbol.span = Some(Span::new(10, 5, 25, 1));
        symbol.doc_comment = Some("Documentation".to_string());
        symbol.parent = Some("ParentClass".to_string());
        symbol.children = vec!["child1".to_string(), "child2".to_string()];
        symbol.signature = Some("fn complex_method(x: i32) -> String".to_string());
        symbol.parameters = vec![Parameter {
            name: "x".to_string(),
            type_annotation: Some("i32".to_string()),
            default_value: None,
            is_rest: false,
            is_keyword_only: false,
        }];
        symbol.return_type = Some("String".to_string());
        symbol.decorators = vec!["@decorator".to_string()];
        symbol.type_parameters = vec!["T".to_string()];
        symbol.metadata.insert("key".to_string(), "value".to_string());

        assert_eq!(symbol.name, "complex_method");
        assert_eq!(symbol.children.len(), 2);
        assert_eq!(symbol.parameters.len(), 1);
        assert_eq!(symbol.decorators.len(), 1);
    }

    // =========================================================================
    // SymbolKind Tests
    // =========================================================================

    #[test]
    fn test_symbol_kind_labels() {
        assert_eq!(SymbolKind::Function.label(), "fn");
        assert_eq!(SymbolKind::Method.label(), "method");
        assert_eq!(SymbolKind::Constructor.label(), "ctor");
        assert_eq!(SymbolKind::Lambda.label(), "lambda");
        assert_eq!(SymbolKind::Class.label(), "class");
        assert_eq!(SymbolKind::Struct.label(), "struct");
        assert_eq!(SymbolKind::Interface.label(), "interface");
        assert_eq!(SymbolKind::Trait.label(), "trait");
        assert_eq!(SymbolKind::Enum.label(), "enum");
        assert_eq!(SymbolKind::TypeAlias.label(), "type");
        assert_eq!(SymbolKind::Variable.label(), "var");
        assert_eq!(SymbolKind::Constant.label(), "const");
        assert_eq!(SymbolKind::Field.label(), "field");
        assert_eq!(SymbolKind::Property.label(), "prop");
        assert_eq!(SymbolKind::EnumVariant.label(), "variant");
        assert_eq!(SymbolKind::Module.label(), "mod");
        assert_eq!(SymbolKind::Namespace.label(), "ns");
        assert_eq!(SymbolKind::Package.label(), "pkg");
        assert_eq!(SymbolKind::Macro.label(), "macro");
        assert_eq!(SymbolKind::Decorator.label(), "decorator");
        assert_eq!(SymbolKind::Attribute.label(), "attr");
        assert_eq!(SymbolKind::Unknown.label(), "?");
    }

    #[test]
    fn test_symbol_kind_icons() {
        assert_eq!(SymbolKind::Function.icon(), "⚡");
        assert_eq!(SymbolKind::Method.icon(), "⚡");
        assert_eq!(SymbolKind::Constructor.icon(), "🔨");
        assert_eq!(SymbolKind::Lambda.icon(), "λ");
        assert_eq!(SymbolKind::Class.icon(), "📦");
        assert_eq!(SymbolKind::Struct.icon(), "🔳");
        assert_eq!(SymbolKind::Interface.icon(), "🔌");
        assert_eq!(SymbolKind::Trait.icon(), "🔌");
        assert_eq!(SymbolKind::Enum.icon(), "📋");
        assert_eq!(SymbolKind::TypeAlias.icon(), "📐");
        assert_eq!(SymbolKind::Variable.icon(), "📝");
        assert_eq!(SymbolKind::Constant.icon(), "🔒");
        assert_eq!(SymbolKind::Field.icon(), "•");
        assert_eq!(SymbolKind::Property.icon(), "•");
        assert_eq!(SymbolKind::EnumVariant.icon(), "◦");
        assert_eq!(SymbolKind::Module.icon(), "📁");
        assert_eq!(SymbolKind::Namespace.icon(), "📁");
        assert_eq!(SymbolKind::Package.icon(), "📁");
        assert_eq!(SymbolKind::Macro.icon(), "⚙️");
        assert_eq!(SymbolKind::Decorator.icon(), "@");
        assert_eq!(SymbolKind::Attribute.icon(), "@");
        assert_eq!(SymbolKind::Unknown.icon(), "?");
    }

    #[test]
    fn test_symbol_kind_hash_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SymbolKind::Function);
        set.insert(SymbolKind::Class);
        set.insert(SymbolKind::Function); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&SymbolKind::Function));
    }

    #[test]
    fn test_symbol_kind_clone_copy() {
        let kind = SymbolKind::Interface;
        let copied = kind;
        let cloned = kind.clone();
        assert_eq!(kind, copied);
        assert_eq!(kind, cloned);
    }

    // =========================================================================
    // SymbolVisibility Tests
    // =========================================================================

    #[test]
    fn test_symbol_visibility_default() {
        let vis = SymbolVisibility::default();
        assert_eq!(vis, SymbolVisibility::Unspecified);
    }

    #[test]
    fn test_symbol_visibility_variants() {
        assert_ne!(SymbolVisibility::Public, SymbolVisibility::Private);
        assert_ne!(SymbolVisibility::Export, SymbolVisibility::Internal);
        assert_ne!(SymbolVisibility::Protected, SymbolVisibility::Unspecified);
    }

    #[test]
    fn test_symbol_visibility_clone_copy() {
        let vis = SymbolVisibility::Public;
        let copied = vis;
        let cloned = vis.clone();
        assert_eq!(vis, copied);
        assert_eq!(vis, cloned);
    }

    // =========================================================================
    // ImportKind Tests
    // =========================================================================

    #[test]
    fn test_import_kind_variants() {
        assert_ne!(ImportKind::Wildcard, ImportKind::Selective);
        assert_ne!(ImportKind::Module, ImportKind::ReExport);
        assert_ne!(ImportKind::SideEffect, ImportKind::Wildcard);
    }

    #[test]
    fn test_import_kind_clone_copy() {
        let kind = ImportKind::Selective;
        let copied = kind;
        let cloned = kind.clone();
        assert_eq!(kind, copied);
        assert_eq!(kind, cloned);
    }

    // =========================================================================
    // Location Tests
    // =========================================================================

    #[test]
    fn test_location_new() {
        let loc = Location::new(42, 10, 500);
        assert_eq!(loc.line, 42);
        assert_eq!(loc.column, 10);
        assert_eq!(loc.offset, 500);
    }

    #[test]
    fn test_location_default() {
        let loc = Location::default();
        assert_eq!(loc.line, 0);
        assert_eq!(loc.column, 0);
        assert_eq!(loc.offset, 0);
    }

    #[test]
    fn test_location_eq() {
        let loc1 = Location::new(1, 2, 3);
        let loc2 = Location::new(1, 2, 3);
        let loc3 = Location::new(1, 2, 4);

        assert_eq!(loc1, loc2);
        assert_ne!(loc1, loc3);
    }

    #[test]
    fn test_location_clone_copy() {
        let loc = Location::new(10, 20, 30);
        let copied = loc;
        let cloned = loc.clone();
        assert_eq!(loc, copied);
        assert_eq!(loc, cloned);
    }

    // =========================================================================
    // Span Tests
    // =========================================================================

    #[test]
    fn test_span_new() {
        let span = Span::new(1, 5, 10, 20);
        assert_eq!(span.start_line, 1);
        assert_eq!(span.start_column, 5);
        assert_eq!(span.end_line, 10);
        assert_eq!(span.end_column, 20);
        assert_eq!(span.start_offset, 0);
        assert_eq!(span.end_offset, 0);
    }

    #[test]
    fn test_span_default() {
        let span = Span::default();
        assert_eq!(span.start_line, 0);
        assert_eq!(span.end_line, 0);
    }

    #[test]
    fn test_span_line_count() {
        let span1 = Span::new(1, 1, 10, 1);
        assert_eq!(span1.line_count(), 10);

        let span2 = Span::new(5, 1, 5, 10);
        assert_eq!(span2.line_count(), 1);

        let span3 = Span::new(100, 1, 200, 1);
        assert_eq!(span3.line_count(), 101);
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(10, 1, 20, 50);

        // Inside the span
        assert!(span.contains(&Location::new(15, 25, 0)));

        // At the boundaries
        assert!(span.contains(&Location::new(10, 1, 0)));
        assert!(span.contains(&Location::new(20, 50, 0)));

        // Outside before
        assert!(!span.contains(&Location::new(9, 1, 0)));
        assert!(!span.contains(&Location::new(10, 0, 0))); // Before start column on start line

        // Outside after
        assert!(!span.contains(&Location::new(21, 1, 0)));
        assert!(!span.contains(&Location::new(20, 51, 0))); // After end column on end line
    }

    #[test]
    fn test_span_contains_edge_cases() {
        let span = Span::new(5, 10, 5, 30); // Single line span

        assert!(span.contains(&Location::new(5, 15, 0)));
        assert!(span.contains(&Location::new(5, 10, 0)));
        assert!(span.contains(&Location::new(5, 30, 0)));
        assert!(!span.contains(&Location::new(5, 9, 0)));
        assert!(!span.contains(&Location::new(5, 31, 0)));
    }

    #[test]
    fn test_span_eq() {
        let span1 = Span::new(1, 2, 3, 4);
        let span2 = Span::new(1, 2, 3, 4);
        let span3 = Span::new(1, 2, 3, 5);

        assert_eq!(span1, span2);
        assert_ne!(span1, span3);
    }

    // =========================================================================
    // Parameter Tests
    // =========================================================================

    #[test]
    fn test_parameter_minimal() {
        let param = Parameter {
            name: "x".to_string(),
            type_annotation: None,
            default_value: None,
            is_rest: false,
            is_keyword_only: false,
        };
        assert_eq!(param.name, "x");
        assert!(!param.is_rest);
    }

    #[test]
    fn test_parameter_with_type() {
        let param = Parameter {
            name: "count".to_string(),
            type_annotation: Some("i32".to_string()),
            default_value: Some("0".to_string()),
            is_rest: false,
            is_keyword_only: true,
        };
        assert_eq!(param.type_annotation, Some("i32".to_string()));
        assert_eq!(param.default_value, Some("0".to_string()));
        assert!(param.is_keyword_only);
    }

    #[test]
    fn test_parameter_rest() {
        let param = Parameter {
            name: "args".to_string(),
            type_annotation: None,
            default_value: None,
            is_rest: true,
            is_keyword_only: false,
        };
        assert!(param.is_rest);
    }

    // =========================================================================
    // ParseDiagnostic Tests
    // =========================================================================

    #[test]
    fn test_parse_diagnostic_error() {
        let diag = ParseDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: "Unexpected token".to_string(),
            location: Location::new(5, 10, 100),
            span: Some(Span::new(5, 10, 5, 15)),
        };
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert!(diag.message.contains("Unexpected"));
    }

    #[test]
    fn test_parse_diagnostic_warning() {
        let diag = ParseDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "Unused variable".to_string(),
            location: Location::default(),
            span: None,
        };
        assert_eq!(diag.severity, DiagnosticSeverity::Warning);
        assert!(diag.span.is_none());
    }

    #[test]
    fn test_diagnostic_severity_variants() {
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
        assert_ne!(DiagnosticSeverity::Info, DiagnosticSeverity::Hint);

        // Test clone/copy
        let sev = DiagnosticSeverity::Error;
        let copied = sev;
        assert_eq!(sev, copied);
    }

    // =========================================================================
    // Import Tests
    // =========================================================================

    #[test]
    fn test_import_full() {
        let import = Import {
            source: "std::collections::HashMap".to_string(),
            kind: ImportKind::Selective,
            alias: Some("Map".to_string()),
            items: vec!["get".to_string(), "insert".to_string()],
            location: Location::new(1, 1, 0),
            type_only: true,
        };

        assert_eq!(import.source, "std::collections::HashMap");
        assert_eq!(import.kind, ImportKind::Selective);
        assert_eq!(import.alias, Some("Map".to_string()));
        assert_eq!(import.items.len(), 2);
        assert!(import.type_only);
    }

    // =========================================================================
    // Module Tests
    // =========================================================================

    #[test]
    fn test_module_creation() {
        let module = Module {
            name: "utils".to_string(),
            path: vec!["src".to_string(), "lib".to_string()],
            location: Location::new(1, 1, 0),
            doc_comment: Some("Utility functions".to_string()),
            visibility: SymbolVisibility::Public,
        };

        assert_eq!(module.name, "utils");
        assert_eq!(module.path.len(), 2);
        assert!(module.doc_comment.is_some());
        assert_eq!(module.visibility, SymbolVisibility::Public);
    }

    // =========================================================================
    // Scope Tests
    // =========================================================================

    #[test]
    fn test_scope_creation() {
        let scope = Scope {
            kind: "function".to_string(),
            span: Span::new(10, 1, 50, 1),
            parent: Some(0),
            symbols: vec![1, 2, 3],
        };

        assert_eq!(scope.kind, "function");
        assert_eq!(scope.parent, Some(0));
        assert_eq!(scope.symbols.len(), 3);
    }

    #[test]
    fn test_scope_without_parent() {
        let scope = Scope {
            kind: "module".to_string(),
            span: Span::default(),
            parent: None,
            symbols: vec![],
        };

        assert!(scope.parent.is_none());
        assert!(scope.symbols.is_empty());
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_ast_serialization() {
        let mut ast = NormalizedAst::new();
        ast.symbols.push(Symbol::new("test_func", SymbolKind::Function, Location::new(1, 1, 0)));

        let json = serde_json::to_string(&ast).unwrap();
        let parsed: NormalizedAst = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.symbols.len(), 1);
        assert_eq!(parsed.symbols[0].name, "test_func");
    }

    #[test]
    fn test_symbol_kind_serialization() {
        let kind = SymbolKind::Interface;
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: SymbolKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }

    #[test]
    fn test_import_kind_serialization() {
        let kind = ImportKind::ReExport;
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: ImportKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }
}
