//! Fractal Context: The main hierarchical context structure
//!
//! This module defines `FractalContext`, the primary data structure that holds
//! the complete hierarchical, zoomable context for a file or project.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::layers::{ContextLayer, ZoomLevel, Range};

/// The main fractal context structure - hierarchical, zoomable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractalContext {
    /// Unique identifier for this context
    pub id: String,
    /// Current view (zoom level and focus)
    pub current_view: ZoomView,
    /// All layers indexed by ID
    pub layers: HashMap<String, ContextLayer>,
    /// ID of the root layer (usually Project or File level)
    pub root_id: String,
    /// Relationships between elements
    pub relationships: RelationshipGraph,
    /// Semantic clusters (grouped by similarity)
    #[serde(default)]
    pub semantic_clusters: Vec<SemanticCluster>,
    /// Metadata about extraction
    pub metadata: ExtractionMetadata,
}

impl FractalContext {
    /// Create a new fractal context with a root layer.
    pub fn new(id: impl Into<String>, root_layer: ContextLayer) -> Self {
        let id = id.into();
        let root_id = root_layer.id.clone();
        let level = root_layer.level;

        let mut layers = HashMap::new();
        layers.insert(root_id.clone(), root_layer);

        Self {
            id,
            current_view: ZoomView {
                level,
                focus_id: Some(root_id.clone()),
                visible_range: None,
            },
            layers,
            root_id,
            relationships: RelationshipGraph::default(),
            semantic_clusters: Vec::new(),
            metadata: ExtractionMetadata::default(),
        }
    }

    /// Add a layer to the context.
    pub fn add_layer(&mut self, layer: ContextLayer) {
        self.layers.insert(layer.id.clone(), layer);
    }

    /// Get a layer by ID.
    pub fn get_layer(&self, id: &str) -> Option<&ContextLayer> {
        self.layers.get(id)
    }

    /// Get a mutable layer by ID.
    pub fn get_layer_mut(&mut self, id: &str) -> Option<&mut ContextLayer> {
        self.layers.get_mut(id)
    }

    /// Get the root layer.
    pub fn root(&self) -> Option<&ContextLayer> {
        self.layers.get(&self.root_id)
    }

    /// Get the current focused layer.
    pub fn current(&self) -> Option<&ContextLayer> {
        self.current_view.focus_id.as_ref()
            .and_then(|id| self.layers.get(id))
    }

    /// Get all layers at a specific zoom level.
    pub fn layers_at_level(&self, level: ZoomLevel) -> Vec<&ContextLayer> {
        self.layers.values()
            .filter(|l| l.level == level)
            .collect()
    }

    /// Get children of a layer.
    pub fn children(&self, layer_id: &str) -> Vec<&ContextLayer> {
        self.get_layer(layer_id)
            .map(|layer| {
                layer.child_ids.iter()
                    .filter_map(|id| self.layers.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get parent of a layer.
    pub fn parent(&self, layer_id: &str) -> Option<&ContextLayer> {
        self.get_layer(layer_id)
            .and_then(|layer| layer.parent_id.as_ref())
            .and_then(|id| self.layers.get(id))
    }

    /// Get siblings of a layer (including itself).
    pub fn siblings(&self, layer_id: &str) -> Vec<&ContextLayer> {
        self.get_layer(layer_id)
            .and_then(|layer| layer.parent_id.as_ref())
            .and_then(|parent_id| self.get_layer(parent_id))
            .map(|parent| self.children(&parent.id))
            .unwrap_or_default()
    }

    /// Count total layers.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Get all layer IDs.
    pub fn layer_ids(&self) -> Vec<&str> {
        self.layers.keys().map(|s| s.as_str()).collect()
    }

    /// Navigate to a specific layer (zoom).
    pub fn navigate_to(&mut self, layer_id: &str) -> bool {
        if let Some(layer) = self.layers.get(layer_id) {
            self.current_view.level = layer.level;
            self.current_view.focus_id = Some(layer_id.to_string());
            true
        } else {
            false
        }
    }

    /// Zoom into the first child of the current layer.
    pub fn zoom_in(&mut self) -> bool {
        let target_id = self.current_view.focus_id.as_ref()
            .and_then(|id| self.layers.get(id))
            .and_then(|layer| layer.child_ids.first())
            .cloned();

        if let Some(child_id) = target_id {
            self.navigate_to(&child_id)
        } else {
            false
        }
    }

    /// Zoom out to the parent of the current layer.
    pub fn zoom_out(&mut self) -> bool {
        let target_id = self.current_view.focus_id.as_ref()
            .and_then(|id| self.layers.get(id))
            .and_then(|layer| layer.parent_id.clone());

        if let Some(parent_id) = target_id {
            self.navigate_to(&parent_id)
        } else {
            false
        }
    }

    /// Build a hierarchical view starting from a layer.
    pub fn hierarchical_view(&self, layer_id: &str, max_depth: usize) -> Option<HierarchicalView> {
        self.get_layer(layer_id).map(|layer| {
            self.build_hierarchical_view(layer, 0, max_depth)
        })
    }

    fn build_hierarchical_view(&self, layer: &ContextLayer, depth: usize, max_depth: usize) -> HierarchicalView {
        let children = if depth < max_depth {
            layer.child_ids.iter()
                .filter_map(|id| self.layers.get(id))
                .map(|child| self.build_hierarchical_view(child, depth + 1, max_depth))
                .collect()
        } else {
            Vec::new()
        };

        HierarchicalView {
            id: layer.id.clone(),
            level: layer.level,
            name: layer.name().to_string(),
            children,
        }
    }
}

/// Current view state (zoom level and focus).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoomView {
    /// Current zoom level
    pub level: ZoomLevel,
    /// ID of the focused element
    pub focus_id: Option<String>,
    /// Visible range (for partial views)
    pub visible_range: Option<Range>,
}

impl Default for ZoomView {
    fn default() -> Self {
        Self {
            level: ZoomLevel::File,
            focus_id: None,
            visible_range: None,
        }
    }
}

/// A hierarchical view of context layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalView {
    pub id: String,
    pub level: ZoomLevel,
    pub name: String,
    pub children: Vec<HierarchicalView>,
}

/// Relationship graph between context elements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationshipGraph {
    /// Nodes in the graph (layer IDs)
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    /// Edges between nodes
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
}

impl RelationshipGraph {
    /// Add a node to the graph.
    pub fn add_node(&mut self, node: GraphNode) {
        if !self.nodes.iter().any(|n| n.id == node.id) {
            self.nodes.push(node);
        }
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// Get all edges from a node.
    pub fn edges_from(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter()
            .filter(|e| e.source == node_id)
            .collect()
    }

    /// Get all edges to a node.
    pub fn edges_to(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter()
            .filter(|e| e.target == node_id)
            .collect()
    }

    /// Find nodes connected to a given node.
    pub fn connected_nodes(&self, node_id: &str) -> Vec<&str> {
        let mut connected = Vec::new();

        for edge in &self.edges {
            if edge.source == node_id {
                connected.push(edge.target.as_str());
            }
            if edge.target == node_id {
                connected.push(edge.source.as_str());
            }
        }

        connected
    }
}

/// A node in the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub node_type: NodeType,
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

/// Types of graph nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    #[default]
    Symbol,
    File,
    Module,
    Dependency,
    External,
}

/// An edge in the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relationship: RelationshipType,
    #[serde(default)]
    pub weight: f32,
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

/// Types of relationships between elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Source calls target
    #[default]
    Calls,
    /// Source is called by target
    CalledBy,
    /// Source imports target
    Imports,
    /// Source depends on target
    DependsOn,
    /// Source contains target
    Contains,
    /// Source is similar to target
    SimilarTo,
    /// Source implements target
    Implements,
    /// Source extends target
    Extends,
}

/// Semantic clustering of related elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCluster {
    pub id: String,
    pub name: String,
    /// IDs of elements in this cluster
    pub element_ids: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Similarity threshold used for clustering
    #[serde(default)]
    pub similarity_threshold: f32,
}

/// Metadata about the extraction process.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    /// How long extraction took
    #[serde(default, with = "duration_serde")]
    pub extraction_time: Duration,
    /// Source path that was analyzed
    #[serde(default)]
    pub source_path: Option<PathBuf>,
    /// Detected language
    #[serde(default)]
    pub language: Option<String>,
    /// Confidence scores for different aspects
    #[serde(default)]
    pub confidence_scores: HashMap<String, f32>,
    /// Cache statistics
    #[serde(default)]
    pub cache_hits: usize,
    #[serde(default)]
    pub cache_misses: usize,
    /// Version of the extractor
    #[serde(default)]
    pub extractor_version: String,
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::layers::{LayerContent, SymbolKind, Visibility};

    // =========================================================================
    // FractalContext Tests (TDD)
    // =========================================================================

    fn create_test_file_layer() -> ContextLayer {
        ContextLayer::new(
            "file_001",
            LayerContent::File {
                path: PathBuf::from("src/main.rs"),
                language: "rust".to_string(),
                size_bytes: 1024,
                line_count: 50,
                symbol_count: 3,
                imports: vec![],
            },
        )
    }

    fn create_test_symbol_layer(id: &str, name: &str, parent_id: &str) -> ContextLayer {
        ContextLayer::new(
            id,
            LayerContent::Symbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!("fn {}()", name),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: Visibility::Public,
                range: Range::default(),
            },
        ).with_parent(parent_id)
    }

    #[test]
    fn test_fractal_context_new() {
        let root = create_test_file_layer();
        let ctx = FractalContext::new("ctx_001", root);

        assert_eq!(ctx.id, "ctx_001");
        assert_eq!(ctx.root_id, "file_001");
        assert_eq!(ctx.layer_count(), 1);
        assert!(ctx.root().is_some());
    }

    #[test]
    fn test_fractal_context_add_and_get_layer() {
        let root = create_test_file_layer();
        let mut ctx = FractalContext::new("ctx_001", root);

        let symbol = create_test_symbol_layer("sym_001", "main", "file_001");
        ctx.add_layer(symbol);

        assert_eq!(ctx.layer_count(), 2);
        assert!(ctx.get_layer("sym_001").is_some());
        assert!(ctx.get_layer("nonexistent").is_none());
    }

    #[test]
    fn test_fractal_context_layers_at_level() {
        let mut root = create_test_file_layer();
        root.add_child("sym_001");
        root.add_child("sym_002");

        let mut ctx = FractalContext::new("ctx_001", root);

        ctx.add_layer(create_test_symbol_layer("sym_001", "func_a", "file_001"));
        ctx.add_layer(create_test_symbol_layer("sym_002", "func_b", "file_001"));

        let file_layers = ctx.layers_at_level(ZoomLevel::File);
        assert_eq!(file_layers.len(), 1);

        let symbol_layers = ctx.layers_at_level(ZoomLevel::Symbol);
        assert_eq!(symbol_layers.len(), 2);
    }

    #[test]
    fn test_fractal_context_children_and_parent() {
        let mut root = create_test_file_layer();
        root.add_child("sym_001");
        root.add_child("sym_002");

        let mut ctx = FractalContext::new("ctx_001", root);
        ctx.add_layer(create_test_symbol_layer("sym_001", "main", "file_001"));
        ctx.add_layer(create_test_symbol_layer("sym_002", "helper", "file_001"));

        // Test children
        let children = ctx.children("file_001");
        assert_eq!(children.len(), 2);

        // Test parent
        let parent = ctx.parent("sym_001");
        assert!(parent.is_some());
        assert_eq!(parent.unwrap().id, "file_001");
    }

    #[test]
    fn test_fractal_context_navigation() {
        let mut root = create_test_file_layer();
        root.add_child("sym_001");

        let mut ctx = FractalContext::new("ctx_001", root);
        ctx.add_layer(create_test_symbol_layer("sym_001", "main", "file_001"));

        // Initially at file level
        assert_eq!(ctx.current_view.level, ZoomLevel::File);

        // Navigate to symbol
        assert!(ctx.navigate_to("sym_001"));
        assert_eq!(ctx.current_view.level, ZoomLevel::Symbol);
        assert_eq!(ctx.current_view.focus_id, Some("sym_001".to_string()));

        // Navigate back
        assert!(ctx.navigate_to("file_001"));
        assert_eq!(ctx.current_view.level, ZoomLevel::File);
    }

    #[test]
    fn test_fractal_context_zoom_in_out() {
        let mut root = create_test_file_layer();
        root.add_child("sym_001");

        let mut ctx = FractalContext::new("ctx_001", root);
        ctx.add_layer(create_test_symbol_layer("sym_001", "main", "file_001"));

        // Initially at file level
        assert_eq!(ctx.current_view.focus_id, Some("file_001".to_string()));

        // Zoom in to symbol
        assert!(ctx.zoom_in());
        assert_eq!(ctx.current_view.focus_id, Some("sym_001".to_string()));
        assert_eq!(ctx.current_view.level, ZoomLevel::Symbol);

        // Zoom out back to file
        assert!(ctx.zoom_out());
        assert_eq!(ctx.current_view.focus_id, Some("file_001".to_string()));
        assert_eq!(ctx.current_view.level, ZoomLevel::File);

        // Can't zoom out further (no parent)
        assert!(!ctx.zoom_out());
    }

    #[test]
    fn test_fractal_context_hierarchical_view() {
        let mut root = create_test_file_layer();
        root.add_child("sym_001");
        root.add_child("sym_002");

        let mut ctx = FractalContext::new("ctx_001", root);
        ctx.add_layer(create_test_symbol_layer("sym_001", "func_a", "file_001"));
        ctx.add_layer(create_test_symbol_layer("sym_002", "func_b", "file_001"));

        let view = ctx.hierarchical_view("file_001", 2).unwrap();
        assert_eq!(view.id, "file_001");
        assert_eq!(view.level, ZoomLevel::File);
        assert_eq!(view.children.len(), 2);
    }

    // =========================================================================
    // RelationshipGraph Tests (TDD)
    // =========================================================================

    #[test]
    fn test_relationship_graph_add_node() {
        let mut graph = RelationshipGraph::default();

        graph.add_node(GraphNode {
            id: "node_1".to_string(),
            label: "Function A".to_string(),
            node_type: NodeType::Symbol,
            properties: HashMap::new(),
        });

        assert_eq!(graph.nodes.len(), 1);

        // Adding same node again shouldn't duplicate
        graph.add_node(GraphNode {
            id: "node_1".to_string(),
            label: "Function A".to_string(),
            node_type: NodeType::Symbol,
            properties: HashMap::new(),
        });

        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_relationship_graph_add_edge() {
        let mut graph = RelationshipGraph::default();

        graph.add_edge(GraphEdge {
            source: "node_1".to_string(),
            target: "node_2".to_string(),
            relationship: RelationshipType::Calls,
            weight: 1.0,
            properties: HashMap::new(),
        });

        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_relationship_graph_edges_from_to() {
        let mut graph = RelationshipGraph::default();

        graph.add_edge(GraphEdge {
            source: "a".to_string(),
            target: "b".to_string(),
            relationship: RelationshipType::Calls,
            weight: 1.0,
            properties: HashMap::new(),
        });

        graph.add_edge(GraphEdge {
            source: "a".to_string(),
            target: "c".to_string(),
            relationship: RelationshipType::Calls,
            weight: 1.0,
            properties: HashMap::new(),
        });

        graph.add_edge(GraphEdge {
            source: "b".to_string(),
            target: "c".to_string(),
            relationship: RelationshipType::Calls,
            weight: 1.0,
            properties: HashMap::new(),
        });

        let from_a = graph.edges_from("a");
        assert_eq!(from_a.len(), 2);

        let to_c = graph.edges_to("c");
        assert_eq!(to_c.len(), 2);
    }

    #[test]
    fn test_relationship_graph_connected_nodes() {
        let mut graph = RelationshipGraph::default();

        graph.add_edge(GraphEdge {
            source: "a".to_string(),
            target: "b".to_string(),
            relationship: RelationshipType::Calls,
            weight: 1.0,
            properties: HashMap::new(),
        });

        graph.add_edge(GraphEdge {
            source: "c".to_string(),
            target: "a".to_string(),
            relationship: RelationshipType::Calls,
            weight: 1.0,
            properties: HashMap::new(),
        });

        let connected = graph.connected_nodes("a");
        assert_eq!(connected.len(), 2);
        assert!(connected.contains(&"b"));
        assert!(connected.contains(&"c"));
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_fractal_context_serialization_roundtrip() {
        let mut root = create_test_file_layer();
        root.add_child("sym_001");

        let mut ctx = FractalContext::new("ctx_001", root);
        ctx.add_layer(create_test_symbol_layer("sym_001", "main", "file_001"));

        ctx.relationships.add_node(GraphNode {
            id: "main".to_string(),
            label: "main".to_string(),
            node_type: NodeType::Symbol,
            properties: HashMap::new(),
        });

        let json = serde_json::to_string_pretty(&ctx).unwrap();
        let deserialized: FractalContext = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "ctx_001");
        assert_eq!(deserialized.layer_count(), 2);
        assert_eq!(deserialized.relationships.nodes.len(), 1);
    }

    #[test]
    fn test_zoom_view_default() {
        let view = ZoomView::default();
        assert_eq!(view.level, ZoomLevel::File);
        assert!(view.focus_id.is_none());
        assert!(view.visible_range.is_none());
    }

    #[test]
    fn test_extraction_metadata_serialization() {
        let metadata = ExtractionMetadata {
            extraction_time: Duration::from_millis(150),
            source_path: Some(PathBuf::from("src/main.rs")),
            language: Some("rust".to_string()),
            confidence_scores: HashMap::from([
                ("symbols".to_string(), 0.95),
                ("relationships".to_string(), 0.85),
            ]),
            cache_hits: 10,
            cache_misses: 2,
            extractor_version: "1.0.0".to_string(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ExtractionMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.extraction_time, Duration::from_millis(150));
        assert_eq!(deserialized.cache_hits, 10);
    }
}
