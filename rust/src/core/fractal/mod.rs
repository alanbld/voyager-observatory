//! Fractal Context Engine
//!
//! This module provides hierarchical, zoomable context extraction for LLM reasoning.
//! It transforms flat source code into nested semantic layers that can be navigated
//! like a fractal structure.
//!
//! # Architecture
//!
//! ```text
//! Project → Module → File → Symbol → Block → Line → Expression → Token
//! ```
//!
//! Each level contains the next, forming a natural hierarchy. Users can:
//! - **Zoom in**: Navigate to more detailed layers
//! - **Zoom out**: Navigate to broader context
//! - **Pan**: Move to related elements at the same level
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::{FractalContext, ZoomLevel, ContextLayer, LayerContent};
//! use std::path::PathBuf;
//!
//! // Create a file layer
//! let file_layer = ContextLayer::new("file_001", LayerContent::File {
//!     path: PathBuf::from("src/main.rs"),
//!     language: "rust".to_string(),
//!     size_bytes: 1024,
//!     line_count: 50,
//!     symbol_count: 3,
//!     imports: vec![],
//! });
//!
//! // Create the context
//! let mut ctx = FractalContext::new("ctx_001", file_layer);
//!
//! // Navigate the hierarchy
//! ctx.zoom_in();   // File → Symbol
//! ctx.zoom_out();  // Symbol → File
//! ```

pub mod layers;
pub mod context;
pub mod builder;

// Re-export commonly used types
pub use layers::{
    ZoomLevel,
    ContextLayer,
    LayerContent,
    LayerMetadata,
    Range,
    Position,
    // Symbol types
    SymbolKind,
    BlockType,
    TokenType,
    Visibility,
    // Supporting types
    Dependency,
    DependencyKind,
    Import,
    Parameter,
};

pub use context::{
    FractalContext,
    ZoomView,
    HierarchicalView,
    // Relationship graph
    RelationshipGraph,
    GraphNode,
    GraphEdge,
    NodeType,
    RelationshipType,
    // Clustering
    SemanticCluster,
    // Metadata
    ExtractionMetadata,
};

pub use builder::{
    // Builder
    FractalContextBuilder,
    // Configuration
    BuilderConfig,
    ExtractionDepth,
    // Errors
    BuilderError,
    BuilderResult,
    // Extraction
    ExtractedSymbol,
    extract_symbols_regex,
    detect_language,
};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    /// Integration test: Build a complete fractal context hierarchy
    #[test]
    fn test_complete_hierarchy() {
        // Create project layer
        let project = ContextLayer::new("proj_001", LayerContent::Project {
            name: "my_project".to_string(),
            description: Some("Test project".to_string()),
            root_path: Some(PathBuf::from("/project")),
            file_count: 5,
            dependencies: vec![],
        });

        let mut ctx = FractalContext::new("ctx_001", project);

        // Add module layer
        let module = ContextLayer::new("mod_001", LayerContent::Module {
            name: "core".to_string(),
            path: Some(PathBuf::from("/project/src/core")),
            file_count: 3,
            exports: vec!["Engine".to_string(), "Config".to_string()],
        }).with_parent("proj_001");
        ctx.add_layer(module);

        // Add file layer
        let file = ContextLayer::new("file_001", LayerContent::File {
            path: PathBuf::from("/project/src/core/engine.rs"),
            language: "rust".to_string(),
            size_bytes: 2048,
            line_count: 100,
            symbol_count: 5,
            imports: vec![
                Import {
                    module: "std::io".to_string(),
                    items: vec!["Read".to_string()],
                    alias: None,
                    line: 1,
                },
            ],
        }).with_parent("mod_001");
        ctx.add_layer(file);

        // Add symbol layers
        let sym1 = ContextLayer::new("sym_001", LayerContent::Symbol {
            name: "Engine".to_string(),
            kind: SymbolKind::Struct,
            signature: "pub struct Engine".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: Some("Main engine struct".to_string()),
            visibility: Visibility::Public,
            range: Range::line_range(10, 50),
        }).with_parent("file_001");
        ctx.add_layer(sym1);

        let sym2 = ContextLayer::new("sym_002", LayerContent::Symbol {
            name: "process".to_string(),
            kind: SymbolKind::Method,
            signature: "pub fn process(&self) -> Result<()>".to_string(),
            return_type: Some("Result<()>".to_string()),
            parameters: vec![
                Parameter {
                    name: "self".to_string(),
                    type_hint: Some("&self".to_string()),
                    default_value: None,
                },
            ],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::line_range(55, 75),
        }).with_parent("file_001");
        ctx.add_layer(sym2);

        // Link children
        if let Some(proj) = ctx.get_layer_mut("proj_001") {
            proj.add_child("mod_001");
        }
        if let Some(mod_layer) = ctx.get_layer_mut("mod_001") {
            mod_layer.add_child("file_001");
        }
        if let Some(file_layer) = ctx.get_layer_mut("file_001") {
            file_layer.add_child("sym_001");
            file_layer.add_child("sym_002");
        }

        // Make symbols siblings
        if let Some(sym) = ctx.get_layer_mut("sym_001") {
            sym.add_sibling("sym_002");
        }
        if let Some(sym) = ctx.get_layer_mut("sym_002") {
            sym.add_sibling("sym_001");
        }

        // Add relationships
        ctx.relationships.add_node(GraphNode {
            id: "Engine".to_string(),
            label: "Engine".to_string(),
            node_type: NodeType::Symbol,
            properties: Default::default(),
        });
        ctx.relationships.add_node(GraphNode {
            id: "process".to_string(),
            label: "process".to_string(),
            node_type: NodeType::Symbol,
            properties: Default::default(),
        });
        ctx.relationships.add_edge(GraphEdge {
            source: "Engine".to_string(),
            target: "process".to_string(),
            relationship: RelationshipType::Contains,
            weight: 1.0,
            properties: Default::default(),
        });

        // Verify structure
        assert_eq!(ctx.layer_count(), 5);

        // Verify navigation
        assert!(ctx.navigate_to("proj_001"));
        assert_eq!(ctx.current_view.level, ZoomLevel::Project);

        assert!(ctx.zoom_in()); // proj -> mod
        assert_eq!(ctx.current_view.level, ZoomLevel::Module);

        assert!(ctx.zoom_in()); // mod -> file
        assert_eq!(ctx.current_view.level, ZoomLevel::File);

        assert!(ctx.zoom_in()); // file -> symbol
        assert_eq!(ctx.current_view.level, ZoomLevel::Symbol);

        // Verify hierarchical view
        let view = ctx.hierarchical_view("proj_001", 10).unwrap();
        assert_eq!(view.name, "my_project");
        assert_eq!(view.children.len(), 1);
        assert_eq!(view.children[0].name, "core");
        assert_eq!(view.children[0].children.len(), 1); // file
        assert_eq!(view.children[0].children[0].children.len(), 2); // symbols

        // Verify relationships
        let edges = ctx.relationships.edges_from("Engine");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, "process");
    }

    /// Test JSON serialization of a complete context
    #[test]
    fn test_context_json_output() {
        let file = ContextLayer::new("file_001", LayerContent::File {
            path: PathBuf::from("test.rs"),
            language: "rust".to_string(),
            size_bytes: 512,
            line_count: 25,
            symbol_count: 2,
            imports: vec![],
        });

        let mut ctx = FractalContext::new("ctx_001", file);

        let sym = ContextLayer::new("sym_001", LayerContent::Symbol {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            signature: "fn main()".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: Visibility::Public,
            range: Range::line_range(1, 10),
        }).with_parent("file_001");
        ctx.add_layer(sym);

        if let Some(file_layer) = ctx.get_layer_mut("file_001") {
            file_layer.add_child("sym_001");
        }

        let json = serde_json::to_string_pretty(&ctx).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"id\": \"ctx_001\""));
        assert!(json.contains("\"level\": \"file\""));
        assert!(json.contains("\"type\": \"symbol\""));
        assert!(json.contains("\"name\": \"main\""));

        // Verify roundtrip
        let deserialized: FractalContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.layer_count(), 2);
    }
}
