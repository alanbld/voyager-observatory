//! Fractal Context Layers
//!
//! This module defines the hierarchical zoom levels and context layers
//! that form the backbone of the fractal context system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Hierarchical zoom levels from broadest to most detailed.
///
/// The levels form a natural hierarchy:
/// ```text
/// Project → Module → File → Symbol → Block → Line → Expression → Token
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoomLevel {
    /// Entire project/repository
    Project,
    /// Directory/namespace/module
    Module,
    /// Single source file
    File,
    /// Symbol (function, class, struct, trait)
    Symbol,
    /// Code block (if, loop, match, etc.)
    Block,
    /// Individual line
    Line,
    /// Sub-expression
    Expression,
    /// Individual token
    Token,
}

impl ZoomLevel {
    /// Get the next more detailed level (zoom in).
    ///
    /// Returns `None` if already at the most detailed level (Token).
    pub fn zoom_in(&self) -> Option<Self> {
        match self {
            ZoomLevel::Project => Some(ZoomLevel::Module),
            ZoomLevel::Module => Some(ZoomLevel::File),
            ZoomLevel::File => Some(ZoomLevel::Symbol),
            ZoomLevel::Symbol => Some(ZoomLevel::Block),
            ZoomLevel::Block => Some(ZoomLevel::Line),
            ZoomLevel::Line => Some(ZoomLevel::Expression),
            ZoomLevel::Expression => Some(ZoomLevel::Token),
            ZoomLevel::Token => None,
        }
    }

    /// Get the next broader level (zoom out).
    ///
    /// Returns `None` if already at the broadest level (Project).
    pub fn zoom_out(&self) -> Option<Self> {
        match self {
            ZoomLevel::Project => None,
            ZoomLevel::Module => Some(ZoomLevel::Project),
            ZoomLevel::File => Some(ZoomLevel::Module),
            ZoomLevel::Symbol => Some(ZoomLevel::File),
            ZoomLevel::Block => Some(ZoomLevel::Symbol),
            ZoomLevel::Line => Some(ZoomLevel::Block),
            ZoomLevel::Expression => Some(ZoomLevel::Line),
            ZoomLevel::Token => Some(ZoomLevel::Expression),
        }
    }

    /// Get the numeric depth (0 = Project, 7 = Token).
    pub fn depth(&self) -> u8 {
        match self {
            ZoomLevel::Project => 0,
            ZoomLevel::Module => 1,
            ZoomLevel::File => 2,
            ZoomLevel::Symbol => 3,
            ZoomLevel::Block => 4,
            ZoomLevel::Line => 5,
            ZoomLevel::Expression => 6,
            ZoomLevel::Token => 7,
        }
    }

    /// Check if this level can zoom in further.
    pub fn can_zoom_in(&self) -> bool {
        self.zoom_in().is_some()
    }

    /// Check if this level can zoom out further.
    pub fn can_zoom_out(&self) -> bool {
        self.zoom_out().is_some()
    }

    /// Get all levels from Project to Token.
    pub fn all() -> &'static [ZoomLevel] {
        &[
            ZoomLevel::Project,
            ZoomLevel::Module,
            ZoomLevel::File,
            ZoomLevel::Symbol,
            ZoomLevel::Block,
            ZoomLevel::Line,
            ZoomLevel::Expression,
            ZoomLevel::Token,
        ]
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        ZoomLevel::File
    }
}

impl std::fmt::Display for ZoomLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZoomLevel::Project => write!(f, "project"),
            ZoomLevel::Module => write!(f, "module"),
            ZoomLevel::File => write!(f, "file"),
            ZoomLevel::Symbol => write!(f, "symbol"),
            ZoomLevel::Block => write!(f, "block"),
            ZoomLevel::Line => write!(f, "line"),
            ZoomLevel::Expression => write!(f, "expression"),
            ZoomLevel::Token => write!(f, "token"),
        }
    }
}

/// Symbol kinds for the Symbol zoom level.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    Module,
    Constant,
    Variable,
    Type,
    Macro,
    Test,
    Unknown,
}

impl Default for SymbolKind {
    fn default() -> Self {
        SymbolKind::Unknown
    }
}

/// Block types for the Block zoom level.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    If,
    Else,
    ElseIf,
    For,
    While,
    Loop,
    Match,
    Case,
    Try,
    Catch,
    Finally,
    With,
    Unsafe,
    Async,
    Closure,
    Unknown,
}

impl Default for BlockType {
    fn default() -> Self {
        BlockType::Unknown
    }
}

/// Token types for the Token zoom level.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Keyword,
    Identifier,
    Literal,
    Operator,
    Punctuation,
    Comment,
    Whitespace,
    Unknown,
}

impl Default for TokenType {
    fn default() -> Self {
        TokenType::Unknown
    }
}

/// Visibility of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    #[default]
    Internal,
}

/// A range in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Range {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Range {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn single_line(line: usize) -> Self {
        Self {
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: usize::MAX,
        }
    }

    pub fn line_range(start: usize, end: usize) -> Self {
        Self {
            start_line: start,
            start_col: 0,
            end_line: end,
            end_col: usize::MAX,
        }
    }

    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

/// Position in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Content specific to each zoom level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerContent {
    /// Project-level content
    Project {
        name: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        root_path: Option<PathBuf>,
        #[serde(default)]
        file_count: usize,
        #[serde(default)]
        dependencies: Vec<Dependency>,
    },

    /// Module-level content
    Module {
        name: String,
        #[serde(default)]
        path: Option<PathBuf>,
        #[serde(default)]
        file_count: usize,
        #[serde(default)]
        exports: Vec<String>,
    },

    /// File-level content
    File {
        path: PathBuf,
        #[serde(default)]
        language: String,
        #[serde(default)]
        size_bytes: u64,
        #[serde(default)]
        line_count: usize,
        #[serde(default)]
        symbol_count: usize,
        #[serde(default)]
        imports: Vec<Import>,
    },

    /// Symbol-level content (function, class, struct, etc.)
    Symbol {
        name: String,
        kind: SymbolKind,
        #[serde(default)]
        signature: String,
        #[serde(default)]
        return_type: Option<String>,
        #[serde(default)]
        parameters: Vec<Parameter>,
        #[serde(default)]
        documentation: Option<String>,
        #[serde(default)]
        visibility: Visibility,
        #[serde(default)]
        range: Range,
    },

    /// Block-level content (if, loop, match, etc.)
    Block {
        block_type: BlockType,
        #[serde(default)]
        condition: Option<String>,
        #[serde(default)]
        body_preview: String,
        #[serde(default)]
        nested_depth: usize,
        #[serde(default)]
        range: Range,
    },

    /// Line-level content
    Line {
        number: usize,
        text: String,
        #[serde(default)]
        indentation: usize,
        #[serde(default)]
        is_comment: bool,
        #[serde(default)]
        is_blank: bool,
    },

    /// Expression-level content
    Expression {
        expression: String,
        #[serde(default)]
        type_hint: Option<String>,
        #[serde(default)]
        range: Range,
    },

    /// Token-level content
    Token {
        token_type: TokenType,
        value: String,
        #[serde(default)]
        position: Position,
    },
}

impl LayerContent {
    /// Get the zoom level for this content.
    pub fn zoom_level(&self) -> ZoomLevel {
        match self {
            LayerContent::Project { .. } => ZoomLevel::Project,
            LayerContent::Module { .. } => ZoomLevel::Module,
            LayerContent::File { .. } => ZoomLevel::File,
            LayerContent::Symbol { .. } => ZoomLevel::Symbol,
            LayerContent::Block { .. } => ZoomLevel::Block,
            LayerContent::Line { .. } => ZoomLevel::Line,
            LayerContent::Expression { .. } => ZoomLevel::Expression,
            LayerContent::Token { .. } => ZoomLevel::Token,
        }
    }

    /// Get the name/label for this content.
    pub fn name(&self) -> &str {
        match self {
            LayerContent::Project { name, .. } => name,
            LayerContent::Module { name, .. } => name,
            LayerContent::File { path, .. } => path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown"),
            LayerContent::Symbol { name, .. } => name,
            LayerContent::Block { block_type, .. } => match block_type {
                BlockType::If => "if",
                BlockType::Else => "else",
                BlockType::ElseIf => "else if",
                BlockType::For => "for",
                BlockType::While => "while",
                BlockType::Loop => "loop",
                BlockType::Match => "match",
                BlockType::Case => "case",
                BlockType::Try => "try",
                BlockType::Catch => "catch",
                BlockType::Finally => "finally",
                BlockType::With => "with",
                BlockType::Unsafe => "unsafe",
                BlockType::Async => "async",
                BlockType::Closure => "closure",
                BlockType::Unknown => "block",
            },
            LayerContent::Line { .. } => {
                // Return empty string - line numbers are numeric
                ""
            }
            LayerContent::Expression { expression, .. } => {
                if expression.len() > 20 {
                    &expression[..20]
                } else {
                    expression
                }
            }
            LayerContent::Token { value, .. } => value,
        }
    }
}

/// A dependency reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    #[default]
    Normal,
    Dev,
    Build,
    Optional,
}

/// An import statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub line: usize,
}

/// A function/method parameter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(default)]
    pub type_hint: Option<String>,
    #[serde(default)]
    pub default_value: Option<String>,
}

/// Metadata about a context layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerMetadata {
    /// Source line where this layer starts
    #[serde(default)]
    pub source_line: usize,
    /// How this layer was extracted
    #[serde(default)]
    pub extraction_method: String,
    /// Confidence in extraction accuracy (0.0 - 1.0)
    #[serde(default)]
    pub confidence: f32,
    /// Custom properties
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

/// A context layer in the fractal hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLayer {
    /// Unique identifier for this layer
    pub id: String,
    /// The zoom level of this layer
    pub level: ZoomLevel,
    /// The actual content at this level
    pub content: LayerContent,
    /// Metadata about extraction
    #[serde(default)]
    pub metadata: LayerMetadata,
    /// IDs of child layers (more detailed)
    #[serde(default)]
    pub child_ids: Vec<String>,
    /// ID of parent layer (broader context)
    #[serde(default)]
    pub parent_id: Option<String>,
    /// IDs of sibling layers (same level, related)
    #[serde(default)]
    pub sibling_ids: Vec<String>,
}

impl ContextLayer {
    /// Create a new context layer.
    pub fn new(id: impl Into<String>, content: LayerContent) -> Self {
        let level = content.zoom_level();
        Self {
            id: id.into(),
            level,
            content,
            metadata: LayerMetadata::default(),
            child_ids: Vec::new(),
            parent_id: None,
            sibling_ids: Vec::new(),
        }
    }

    /// Create with metadata.
    pub fn with_metadata(mut self, metadata: LayerMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set parent ID.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Add a child ID.
    pub fn add_child(&mut self, child_id: impl Into<String>) {
        self.child_ids.push(child_id.into());
    }

    /// Add a sibling ID.
    pub fn add_sibling(&mut self, sibling_id: impl Into<String>) {
        self.sibling_ids.push(sibling_id.into());
    }

    /// Check if this layer has children.
    pub fn has_children(&self) -> bool {
        !self.child_ids.is_empty()
    }

    /// Check if this layer has a parent.
    pub fn has_parent(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Get the name of this layer's content.
    pub fn name(&self) -> &str {
        self.content.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ZoomLevel Tests (TDD)
    // =========================================================================

    #[test]
    fn test_zoom_level_zoom_in() {
        assert_eq!(ZoomLevel::Project.zoom_in(), Some(ZoomLevel::Module));
        assert_eq!(ZoomLevel::Module.zoom_in(), Some(ZoomLevel::File));
        assert_eq!(ZoomLevel::File.zoom_in(), Some(ZoomLevel::Symbol));
        assert_eq!(ZoomLevel::Symbol.zoom_in(), Some(ZoomLevel::Block));
        assert_eq!(ZoomLevel::Block.zoom_in(), Some(ZoomLevel::Line));
        assert_eq!(ZoomLevel::Line.zoom_in(), Some(ZoomLevel::Expression));
        assert_eq!(ZoomLevel::Expression.zoom_in(), Some(ZoomLevel::Token));
        assert_eq!(ZoomLevel::Token.zoom_in(), None);
    }

    #[test]
    fn test_zoom_level_zoom_out() {
        assert_eq!(ZoomLevel::Token.zoom_out(), Some(ZoomLevel::Expression));
        assert_eq!(ZoomLevel::Expression.zoom_out(), Some(ZoomLevel::Line));
        assert_eq!(ZoomLevel::Line.zoom_out(), Some(ZoomLevel::Block));
        assert_eq!(ZoomLevel::Block.zoom_out(), Some(ZoomLevel::Symbol));
        assert_eq!(ZoomLevel::Symbol.zoom_out(), Some(ZoomLevel::File));
        assert_eq!(ZoomLevel::File.zoom_out(), Some(ZoomLevel::Module));
        assert_eq!(ZoomLevel::Module.zoom_out(), Some(ZoomLevel::Project));
        assert_eq!(ZoomLevel::Project.zoom_out(), None);
    }

    #[test]
    fn test_zoom_level_depth() {
        assert_eq!(ZoomLevel::Project.depth(), 0);
        assert_eq!(ZoomLevel::Module.depth(), 1);
        assert_eq!(ZoomLevel::File.depth(), 2);
        assert_eq!(ZoomLevel::Symbol.depth(), 3);
        assert_eq!(ZoomLevel::Block.depth(), 4);
        assert_eq!(ZoomLevel::Line.depth(), 5);
        assert_eq!(ZoomLevel::Expression.depth(), 6);
        assert_eq!(ZoomLevel::Token.depth(), 7);
    }

    #[test]
    fn test_zoom_level_can_zoom() {
        assert!(ZoomLevel::Project.can_zoom_in());
        assert!(!ZoomLevel::Project.can_zoom_out());

        assert!(ZoomLevel::Token.can_zoom_out());
        assert!(!ZoomLevel::Token.can_zoom_in());

        assert!(ZoomLevel::File.can_zoom_in());
        assert!(ZoomLevel::File.can_zoom_out());
    }

    #[test]
    fn test_zoom_level_all() {
        let all = ZoomLevel::all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0], ZoomLevel::Project);
        assert_eq!(all[7], ZoomLevel::Token);
    }

    #[test]
    fn test_zoom_level_default() {
        assert_eq!(ZoomLevel::default(), ZoomLevel::File);
    }

    #[test]
    fn test_zoom_level_display() {
        assert_eq!(format!("{}", ZoomLevel::Project), "project");
        assert_eq!(format!("{}", ZoomLevel::Symbol), "symbol");
        assert_eq!(format!("{}", ZoomLevel::Token), "token");
    }

    // =========================================================================
    // LayerContent Tests (TDD)
    // =========================================================================

    #[test]
    fn test_layer_content_zoom_level() {
        let project = LayerContent::Project {
            name: "test".to_string(),
            description: None,
            root_path: None,
            file_count: 0,
            dependencies: vec![],
        };
        assert_eq!(project.zoom_level(), ZoomLevel::Project);

        let file = LayerContent::File {
            path: PathBuf::from("test.rs"),
            language: "rust".to_string(),
            size_bytes: 100,
            line_count: 10,
            symbol_count: 2,
            imports: vec![],
        };
        assert_eq!(file.zoom_level(), ZoomLevel::File);

        let symbol = LayerContent::Symbol {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            signature: "fn main()".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::default(),
        };
        assert_eq!(symbol.zoom_level(), ZoomLevel::Symbol);
    }

    #[test]
    fn test_layer_content_name() {
        let project = LayerContent::Project {
            name: "my_project".to_string(),
            description: None,
            root_path: None,
            file_count: 0,
            dependencies: vec![],
        };
        assert_eq!(project.name(), "my_project");

        let symbol = LayerContent::Symbol {
            name: "process_data".to_string(),
            kind: SymbolKind::Function,
            signature: String::new(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::default(),
            range: Range::default(),
        };
        assert_eq!(symbol.name(), "process_data");
    }

    // =========================================================================
    // ContextLayer Tests (TDD)
    // =========================================================================

    #[test]
    fn test_context_layer_new() {
        let content = LayerContent::File {
            path: PathBuf::from("src/main.rs"),
            language: "rust".to_string(),
            size_bytes: 1024,
            line_count: 50,
            symbol_count: 5,
            imports: vec![],
        };

        let layer = ContextLayer::new("layer_001", content);

        assert_eq!(layer.id, "layer_001");
        assert_eq!(layer.level, ZoomLevel::File);
        assert!(!layer.has_children());
        assert!(!layer.has_parent());
    }

    #[test]
    fn test_context_layer_with_parent() {
        let content = LayerContent::Symbol {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            signature: "fn main()".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::default(),
        };

        let layer = ContextLayer::new("sym_001", content)
            .with_parent("file_001");

        assert!(layer.has_parent());
        assert_eq!(layer.parent_id, Some("file_001".to_string()));
    }

    #[test]
    fn test_context_layer_add_children() {
        let content = LayerContent::File {
            path: PathBuf::from("test.rs"),
            language: "rust".to_string(),
            size_bytes: 0,
            line_count: 0,
            symbol_count: 0,
            imports: vec![],
        };

        let mut layer = ContextLayer::new("file_001", content);
        assert!(!layer.has_children());

        layer.add_child("sym_001");
        layer.add_child("sym_002");

        assert!(layer.has_children());
        assert_eq!(layer.child_ids.len(), 2);
    }

    #[test]
    fn test_context_layer_siblings() {
        let content = LayerContent::Symbol {
            name: "func_a".to_string(),
            kind: SymbolKind::Function,
            signature: String::new(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::default(),
            range: Range::default(),
        };

        let mut layer = ContextLayer::new("sym_a", content);
        layer.add_sibling("sym_b");
        layer.add_sibling("sym_c");

        assert_eq!(layer.sibling_ids.len(), 2);
        assert!(layer.sibling_ids.contains(&"sym_b".to_string()));
        assert!(layer.sibling_ids.contains(&"sym_c".to_string()));
    }

    // =========================================================================
    // Range Tests
    // =========================================================================

    #[test]
    fn test_range_line_count() {
        let range = Range::new(10, 0, 20, 0);
        assert_eq!(range.line_count(), 11); // Lines 10-20 inclusive

        let single = Range::single_line(5);
        assert_eq!(single.line_count(), 1);
    }

    #[test]
    fn test_range_contains_line() {
        let range = Range::line_range(10, 20);
        assert!(range.contains_line(10));
        assert!(range.contains_line(15));
        assert!(range.contains_line(20));
        assert!(!range.contains_line(9));
        assert!(!range.contains_line(21));
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_zoom_level_serialization() {
        let level = ZoomLevel::Symbol;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"symbol\"");

        let deserialized: ZoomLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ZoomLevel::Symbol);
    }

    #[test]
    fn test_layer_content_serialization() {
        let content = LayerContent::Symbol {
            name: "test_fn".to_string(),
            kind: SymbolKind::Function,
            signature: "fn test_fn()".to_string(),
            return_type: Some("i32".to_string()),
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::default(),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"symbol\""));
        assert!(json.contains("\"name\":\"test_fn\""));

        let deserialized: LayerContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.zoom_level(), ZoomLevel::Symbol);
    }

    #[test]
    fn test_context_layer_serialization_roundtrip() {
        let content = LayerContent::File {
            path: PathBuf::from("src/lib.rs"),
            language: "rust".to_string(),
            size_bytes: 2048,
            line_count: 100,
            symbol_count: 10,
            imports: vec![
                Import {
                    module: "std::io".to_string(),
                    items: vec!["Read".to_string(), "Write".to_string()],
                    alias: None,
                    line: 1,
                },
            ],
        };

        let mut layer = ContextLayer::new("file_001", content);
        layer.add_child("sym_001");
        layer = layer.with_parent("mod_001");

        let json = serde_json::to_string_pretty(&layer).unwrap();
        let deserialized: ContextLayer = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "file_001");
        assert_eq!(deserialized.level, ZoomLevel::File);
        assert_eq!(deserialized.child_ids.len(), 1);
        assert_eq!(deserialized.parent_id, Some("mod_001".to_string()));
    }

    // =========================================================================
    // Default Trait Tests
    // =========================================================================

    #[test]
    fn test_symbol_kind_default() {
        assert_eq!(SymbolKind::default(), SymbolKind::Unknown);
    }

    #[test]
    fn test_block_type_default() {
        assert_eq!(BlockType::default(), BlockType::Unknown);
    }

    #[test]
    fn test_token_type_default() {
        assert_eq!(TokenType::default(), TokenType::Unknown);
    }

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Internal);
    }

    #[test]
    fn test_dependency_kind_default() {
        assert_eq!(DependencyKind::default(), DependencyKind::Normal);
    }

    // =========================================================================
    // Position Tests
    // =========================================================================

    #[test]
    fn test_position_new() {
        let pos = Position::new(42, 10);
        assert_eq!(pos.line, 42);
        assert_eq!(pos.column, 10);
    }

    #[test]
    fn test_position_default() {
        let pos = Position::default();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 0);
    }

    // =========================================================================
    // LayerContent zoom_level Tests (all variants)
    // =========================================================================

    #[test]
    fn test_layer_content_zoom_level_module() {
        let module = LayerContent::Module {
            name: "core".to_string(),
            path: Some(PathBuf::from("src/core")),
            file_count: 5,
            exports: vec!["Engine".to_string()],
        };
        assert_eq!(module.zoom_level(), ZoomLevel::Module);
    }

    #[test]
    fn test_layer_content_zoom_level_block() {
        let block = LayerContent::Block {
            block_type: BlockType::If,
            condition: Some("x > 0".to_string()),
            body_preview: "...".to_string(),
            nested_depth: 1,
            range: Range::default(),
        };
        assert_eq!(block.zoom_level(), ZoomLevel::Block);
    }

    #[test]
    fn test_layer_content_zoom_level_line() {
        let line = LayerContent::Line {
            number: 42,
            text: "let x = 1;".to_string(),
            indentation: 4,
            is_comment: false,
            is_blank: false,
        };
        assert_eq!(line.zoom_level(), ZoomLevel::Line);
    }

    #[test]
    fn test_layer_content_zoom_level_expression() {
        let expr = LayerContent::Expression {
            expression: "x + y * z".to_string(),
            type_hint: Some("i32".to_string()),
            range: Range::default(),
        };
        assert_eq!(expr.zoom_level(), ZoomLevel::Expression);
    }

    #[test]
    fn test_layer_content_zoom_level_token() {
        let token = LayerContent::Token {
            token_type: TokenType::Keyword,
            value: "fn".to_string(),
            position: Position::new(1, 1),
        };
        assert_eq!(token.zoom_level(), ZoomLevel::Token);
    }

    // =========================================================================
    // LayerContent name Tests (all variants)
    // =========================================================================

    #[test]
    fn test_layer_content_name_module() {
        let module = LayerContent::Module {
            name: "serialization".to_string(),
            path: None,
            file_count: 0,
            exports: vec![],
        };
        assert_eq!(module.name(), "serialization");
    }

    #[test]
    fn test_layer_content_name_file() {
        let file = LayerContent::File {
            path: PathBuf::from("src/core/engine.rs"),
            language: "rust".to_string(),
            size_bytes: 0,
            line_count: 0,
            symbol_count: 0,
            imports: vec![],
        };
        assert_eq!(file.name(), "engine.rs");
    }

    #[test]
    fn test_layer_content_name_block_all_types() {
        let test_cases = [
            (BlockType::If, "if"),
            (BlockType::Else, "else"),
            (BlockType::ElseIf, "else if"),
            (BlockType::For, "for"),
            (BlockType::While, "while"),
            (BlockType::Loop, "loop"),
            (BlockType::Match, "match"),
            (BlockType::Case, "case"),
            (BlockType::Try, "try"),
            (BlockType::Catch, "catch"),
            (BlockType::Finally, "finally"),
            (BlockType::With, "with"),
            (BlockType::Unsafe, "unsafe"),
            (BlockType::Async, "async"),
            (BlockType::Closure, "closure"),
            (BlockType::Unknown, "block"),
        ];

        for (block_type, expected_name) in test_cases {
            let block = LayerContent::Block {
                block_type: block_type.clone(),
                condition: None,
                body_preview: String::new(),
                nested_depth: 0,
                range: Range::default(),
            };
            assert_eq!(block.name(), expected_name, "BlockType::{:?} should have name '{}'", block_type, expected_name);
        }
    }

    #[test]
    fn test_layer_content_name_line_returns_empty() {
        let line = LayerContent::Line {
            number: 100,
            text: "return result;".to_string(),
            indentation: 8,
            is_comment: false,
            is_blank: false,
        };
        assert_eq!(line.name(), "");
    }

    #[test]
    fn test_layer_content_name_expression_short() {
        let expr = LayerContent::Expression {
            expression: "x + y".to_string(),
            type_hint: None,
            range: Range::default(),
        };
        assert_eq!(expr.name(), "x + y");
    }

    #[test]
    fn test_layer_content_name_expression_long_truncated() {
        let expr = LayerContent::Expression {
            expression: "some_very_long_expression_that_exceeds_twenty_characters".to_string(),
            type_hint: None,
            range: Range::default(),
        };
        assert_eq!(expr.name(), "some_very_long_expre");
        assert_eq!(expr.name().len(), 20);
    }

    #[test]
    fn test_layer_content_name_token() {
        let token = LayerContent::Token {
            token_type: TokenType::Identifier,
            value: "my_variable".to_string(),
            position: Position::default(),
        };
        assert_eq!(token.name(), "my_variable");
    }

    // =========================================================================
    // ContextLayer with_metadata Test
    // =========================================================================

    #[test]
    fn test_context_layer_with_metadata() {
        let content = LayerContent::Symbol {
            name: "test".to_string(),
            kind: SymbolKind::Function,
            signature: String::new(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::default(),
            range: Range::default(),
        };

        let mut metadata = LayerMetadata::default();
        metadata.source_line = 42;
        metadata.extraction_method = "ast".to_string();
        metadata.confidence = 0.95;
        metadata.properties.insert("test_key".to_string(), "test_value".to_string());

        let layer = ContextLayer::new("test_layer", content)
            .with_metadata(metadata);

        assert_eq!(layer.metadata.source_line, 42);
        assert_eq!(layer.metadata.extraction_method, "ast");
        assert_eq!(layer.metadata.confidence, 0.95);
        assert_eq!(layer.metadata.properties.get("test_key"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_context_layer_name() {
        let content = LayerContent::Project {
            name: "my_awesome_project".to_string(),
            description: Some("A test project".to_string()),
            root_path: Some(PathBuf::from("/home/user/project")),
            file_count: 42,
            dependencies: vec![],
        };

        let layer = ContextLayer::new("proj_001", content);
        assert_eq!(layer.name(), "my_awesome_project");
    }

    // =========================================================================
    // ZoomLevel Display Tests (remaining variants)
    // =========================================================================

    #[test]
    fn test_zoom_level_display_all() {
        assert_eq!(format!("{}", ZoomLevel::Project), "project");
        assert_eq!(format!("{}", ZoomLevel::Module), "module");
        assert_eq!(format!("{}", ZoomLevel::File), "file");
        assert_eq!(format!("{}", ZoomLevel::Symbol), "symbol");
        assert_eq!(format!("{}", ZoomLevel::Block), "block");
        assert_eq!(format!("{}", ZoomLevel::Line), "line");
        assert_eq!(format!("{}", ZoomLevel::Expression), "expression");
        assert_eq!(format!("{}", ZoomLevel::Token), "token");
    }

    // =========================================================================
    // Dependency Tests
    // =========================================================================

    #[test]
    fn test_dependency_creation() {
        let dep = Dependency {
            name: "serde".to_string(),
            version: Some("1.0".to_string()),
            kind: DependencyKind::Normal,
        };
        assert_eq!(dep.name, "serde");
        assert_eq!(dep.version, Some("1.0".to_string()));
        assert_eq!(dep.kind, DependencyKind::Normal);
    }

    #[test]
    fn test_dependency_kinds() {
        assert_eq!(DependencyKind::Normal, DependencyKind::default());

        // Ensure all kinds are distinct
        let kinds = [
            DependencyKind::Normal,
            DependencyKind::Dev,
            DependencyKind::Build,
            DependencyKind::Optional,
        ];
        for (i, k1) in kinds.iter().enumerate() {
            for (j, k2) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(k1, k2);
                }
            }
        }
    }

    // =========================================================================
    // Import Tests
    // =========================================================================

    #[test]
    fn test_import_creation() {
        let import = Import {
            module: "std::collections".to_string(),
            items: vec!["HashMap".to_string(), "HashSet".to_string()],
            alias: Some("coll".to_string()),
            line: 5,
        };
        assert_eq!(import.module, "std::collections");
        assert_eq!(import.items.len(), 2);
        assert_eq!(import.alias, Some("coll".to_string()));
        assert_eq!(import.line, 5);
    }

    // =========================================================================
    // Parameter Tests
    // =========================================================================

    #[test]
    fn test_parameter_creation() {
        let param = Parameter {
            name: "count".to_string(),
            type_hint: Some("usize".to_string()),
            default_value: Some("0".to_string()),
        };
        assert_eq!(param.name, "count");
        assert_eq!(param.type_hint, Some("usize".to_string()));
        assert_eq!(param.default_value, Some("0".to_string()));
    }

    // =========================================================================
    // SymbolKind Variants Test
    // =========================================================================

    #[test]
    fn test_symbol_kind_variants_distinct() {
        let kinds = [
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Class,
            SymbolKind::Struct,
            SymbolKind::Enum,
            SymbolKind::Trait,
            SymbolKind::Interface,
            SymbolKind::Module,
            SymbolKind::Constant,
            SymbolKind::Variable,
            SymbolKind::Type,
            SymbolKind::Macro,
            SymbolKind::Test,
            SymbolKind::Unknown,
        ];

        // All variants should be distinct
        for (i, k1) in kinds.iter().enumerate() {
            for (j, k2) in kinds.iter().enumerate() {
                if i != j {
                    assert_ne!(k1, k2);
                }
            }
        }
    }

    // =========================================================================
    // TokenType Variants Test
    // =========================================================================

    #[test]
    fn test_token_type_variants_distinct() {
        let types = [
            TokenType::Keyword,
            TokenType::Identifier,
            TokenType::Literal,
            TokenType::Operator,
            TokenType::Punctuation,
            TokenType::Comment,
            TokenType::Whitespace,
            TokenType::Unknown,
        ];

        for (i, t1) in types.iter().enumerate() {
            for (j, t2) in types.iter().enumerate() {
                if i != j {
                    assert_ne!(t1, t2);
                }
            }
        }
    }

    // =========================================================================
    // Visibility Variants Test
    // =========================================================================

    #[test]
    fn test_visibility_variants() {
        let vis = [
            Visibility::Public,
            Visibility::Private,
            Visibility::Protected,
            Visibility::Internal,
        ];

        for (i, v1) in vis.iter().enumerate() {
            for (j, v2) in vis.iter().enumerate() {
                if i != j {
                    assert_ne!(v1, v2);
                }
            }
        }
    }

    // =========================================================================
    // Range Edge Cases
    // =========================================================================

    #[test]
    fn test_range_default() {
        let range = Range::default();
        assert_eq!(range.start_line, 0);
        assert_eq!(range.start_col, 0);
        assert_eq!(range.end_line, 0);
        assert_eq!(range.end_col, 0);
    }

    #[test]
    fn test_range_single_line_properties() {
        let range = Range::single_line(42);
        assert_eq!(range.start_line, 42);
        assert_eq!(range.end_line, 42);
        assert_eq!(range.start_col, 0);
        assert_eq!(range.end_col, usize::MAX);
        assert!(range.contains_line(42));
        assert!(!range.contains_line(41));
        assert!(!range.contains_line(43));
    }

    #[test]
    fn test_range_line_count_same_line() {
        let range = Range::new(5, 0, 5, 10);
        assert_eq!(range.line_count(), 1);
    }

    // =========================================================================
    // LayerMetadata Tests
    // =========================================================================

    #[test]
    fn test_layer_metadata_default() {
        let meta = LayerMetadata::default();
        assert_eq!(meta.source_line, 0);
        assert_eq!(meta.extraction_method, "");
        assert_eq!(meta.confidence, 0.0);
        assert!(meta.properties.is_empty());
    }
}
