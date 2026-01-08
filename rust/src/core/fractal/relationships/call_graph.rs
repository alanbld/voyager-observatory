//! Call Graph - Directed graph of function/method calls
//!
//! Uses petgraph for efficient graph algorithms including:
//! - Topological sorting
//! - Cycle detection
//! - Shortest path between functions
//! - Strongly connected components

use std::collections::{HashMap, HashSet};

use petgraph::algo::{dijkstra, tarjan_scc, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::{Deserialize, Serialize};

// =============================================================================
// Call Node - A function or method in the call graph
// =============================================================================

/// A node in the call graph representing a callable entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallNode {
    /// Unique identifier (usually fully qualified name)
    pub id: String,
    /// Display name (short name)
    pub name: String,
    /// File path containing this callable
    pub file_path: Option<String>,
    /// Line number where defined
    pub line: Option<usize>,
    /// Kind of callable
    pub kind: CallableKind,
    /// Module/namespace path
    pub module_path: Option<String>,
    /// Whether this is a public/exported function
    pub is_public: bool,
    /// Number of parameters
    pub param_count: usize,
    /// Return type (if known)
    pub return_type: Option<String>,
}

impl CallNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: CallableKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            file_path: None,
            line: None,
            kind,
            module_path: None,
            is_public: false,
            param_count: 0,
            return_type: None,
        }
    }

    pub fn with_location(mut self, file_path: impl Into<String>, line: usize) -> Self {
        self.file_path = Some(file_path.into());
        self.line = Some(line);
        self
    }

    pub fn with_module(mut self, module_path: impl Into<String>) -> Self {
        self.module_path = Some(module_path.into());
        self
    }

    pub fn with_visibility(mut self, is_public: bool) -> Self {
        self.is_public = is_public;
        self
    }

    pub fn with_signature(mut self, param_count: usize, return_type: Option<String>) -> Self {
        self.param_count = param_count;
        self.return_type = return_type;
        self
    }
}

/// Kind of callable entity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CallableKind {
    Function,
    Method,
    Constructor,
    Closure,
    Macro,
    ShellFunction,
    External,
    Unknown,
}

// =============================================================================
// Call Edge - A call relationship between nodes
// =============================================================================

/// An edge in the call graph representing a call relationship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallEdge {
    /// Kind of call
    pub kind: CallKind,
    /// Line number where call occurs
    pub line: Option<usize>,
    /// Column where call occurs
    pub column: Option<usize>,
    /// Whether this is a conditional call (in if/match/etc.)
    pub is_conditional: bool,
    /// Whether this call is in a loop
    pub is_in_loop: bool,
    /// Call count (for profiling/analysis)
    pub weight: u32,
}

impl CallEdge {
    pub fn new(kind: CallKind) -> Self {
        Self {
            kind,
            line: None,
            column: None,
            is_conditional: false,
            is_in_loop: false,
            weight: 1,
        }
    }

    pub fn with_location(mut self, line: usize, column: Option<usize>) -> Self {
        self.line = Some(line);
        self.column = column;
        self
    }

    pub fn conditional(mut self) -> Self {
        self.is_conditional = true;
        self
    }

    pub fn in_loop(mut self) -> Self {
        self.is_in_loop = true;
        self
    }
}

impl Default for CallEdge {
    fn default() -> Self {
        Self::new(CallKind::Direct)
    }
}

/// Kind of call relationship.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CallKind {
    /// Direct function call: `foo()`
    Direct,
    /// Method call: `obj.foo()`
    Method,
    /// Static method: `Type::foo()`
    Static,
    /// Constructor: `new Foo()` or `Foo::new()`
    Constructor,
    /// Async call: `foo().await`
    Async,
    /// Callback/closure: passed as argument
    Callback,
    /// Dynamic/indirect call (function pointer, reflection)
    Dynamic,
    /// External/FFI call
    External,
    /// Macro invocation
    Macro,
    /// Shell command/source
    Shell,
}

// =============================================================================
// Call Graph - The main graph structure
// =============================================================================

/// A directed graph of function calls.
///
/// Uses petgraph internally for efficient graph algorithms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    /// Internal graph (nodes are indices, stored separately)
    #[serde(skip)]
    graph: DiGraph<(), CallEdge>,

    /// Node data indexed by node index
    nodes: Vec<CallNode>,

    /// Map from node ID to graph index
    #[serde(skip)]
    id_to_index: HashMap<String, NodeIndex>,

    /// Root nodes (entry points)
    pub roots: Vec<String>,

    /// Graph metadata
    pub metadata: CallGraphMetadata,
}

/// Metadata about the call graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallGraphMetadata {
    /// Project or module name
    pub name: Option<String>,
    /// Number of files analyzed
    pub file_count: usize,
    /// Languages detected
    pub languages: HashSet<String>,
    /// Whether graph has cycles
    pub has_cycles: bool,
    /// Maximum call depth from roots
    pub max_depth: usize,
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CallGraph {
    /// Create a new empty call graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: Vec::new(),
            id_to_index: HashMap::new(),
            roots: Vec::new(),
            metadata: CallGraphMetadata::default(),
        }
    }

    /// Create a call graph with a name.
    pub fn with_name(name: impl Into<String>) -> Self {
        let mut graph = Self::new();
        graph.metadata.name = Some(name.into());
        graph
    }

    // -------------------------------------------------------------------------
    // Node Operations
    // -------------------------------------------------------------------------

    /// Add a node to the graph. Returns the node ID.
    pub fn add_node(&mut self, node: CallNode) -> String {
        let id = node.id.clone();

        if self.id_to_index.contains_key(&id) {
            return id;
        }

        let index = self.graph.add_node(());
        self.id_to_index.insert(id.clone(), index);
        self.nodes.push(node);

        id
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&CallNode> {
        self.id_to_index
            .get(id)
            .and_then(|&idx| self.nodes.get(idx.index()))
    }

    /// Get a mutable node by ID.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut CallNode> {
        self.id_to_index
            .get(id)
            .and_then(|&idx| self.nodes.get_mut(idx.index()))
    }

    /// Check if a node exists.
    pub fn has_node(&self, id: &str) -> bool {
        self.id_to_index.contains_key(id)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> &[CallNode] {
        &self.nodes
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Remove a node and all its edges.
    pub fn remove_node(&mut self, id: &str) -> Option<CallNode> {
        if let Some(&index) = self.id_to_index.get(id) {
            self.graph.remove_node(index);
            self.id_to_index.remove(id);

            // Find and remove from nodes vec
            if let Some(pos) = self.nodes.iter().position(|n| n.id == id) {
                return Some(self.nodes.remove(pos));
            }
        }
        None
    }

    // -------------------------------------------------------------------------
    // Edge Operations
    // -------------------------------------------------------------------------

    /// Add a call edge between two nodes.
    pub fn add_edge(&mut self, from: &str, to: &str, edge: CallEdge) -> bool {
        let from_idx = match self.id_to_index.get(from) {
            Some(&idx) => idx,
            None => return false,
        };
        let to_idx = match self.id_to_index.get(to) {
            Some(&idx) => idx,
            None => return false,
        };

        self.graph.add_edge(from_idx, to_idx, edge);
        true
    }

    /// Add a simple direct call edge.
    pub fn add_call(&mut self, from: &str, to: &str) -> bool {
        self.add_edge(from, to, CallEdge::default())
    }

    /// Get edges from a node (outgoing calls).
    pub fn calls_from(&self, id: &str) -> Vec<(&CallNode, &CallEdge)> {
        let Some(&idx) = self.id_to_index.get(id) else {
            return Vec::new();
        };

        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter_map(|edge| {
                let target_idx = edge.target();
                self.nodes
                    .get(target_idx.index())
                    .map(|node| (node, edge.weight()))
            })
            .collect()
    }

    /// Get edges to a node (incoming calls / callers).
    pub fn callers_of(&self, id: &str) -> Vec<(&CallNode, &CallEdge)> {
        let Some(&idx) = self.id_to_index.get(id) else {
            return Vec::new();
        };

        self.graph
            .edges_directed(idx, Direction::Incoming)
            .filter_map(|edge| {
                let source_idx = edge.source();
                self.nodes
                    .get(source_idx.index())
                    .map(|node| (node, edge.weight()))
            })
            .collect()
    }

    /// Get edge count.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    // -------------------------------------------------------------------------
    // Graph Algorithms
    // -------------------------------------------------------------------------

    /// Check if the graph has cycles.
    pub fn has_cycles(&self) -> bool {
        toposort(&self.graph, None).is_err()
    }

    /// Get topological ordering of nodes (if acyclic).
    pub fn topological_order(&self) -> Option<Vec<&CallNode>> {
        toposort(&self.graph, None).ok().map(|indices| {
            indices
                .iter()
                .filter_map(|&idx| self.nodes.get(idx.index()))
                .collect()
        })
    }

    /// Find strongly connected components (for cycle analysis).
    pub fn strongly_connected_components(&self) -> Vec<Vec<&CallNode>> {
        tarjan_scc(&self.graph)
            .into_iter()
            .map(|component| {
                component
                    .into_iter()
                    .filter_map(|idx| self.nodes.get(idx.index()))
                    .collect()
            })
            .collect()
    }

    /// Find shortest path between two nodes (by call count).
    pub fn shortest_path(&self, from: &str, to: &str) -> Option<Vec<&CallNode>> {
        let from_idx = *self.id_to_index.get(from)?;
        let to_idx = *self.id_to_index.get(to)?;

        // Use dijkstra with uniform weights
        let predecessors = dijkstra(&self.graph, from_idx, Some(to_idx), |_| 1u32);

        if !predecessors.contains_key(&to_idx) {
            return None;
        }

        // Reconstruct path (dijkstra returns costs, not paths)
        // We need to trace back using BFS/DFS
        let mut path = Vec::new();

        // Simple BFS to find path
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: Vec<(NodeIndex, Vec<NodeIndex>)> = vec![(from_idx, vec![from_idx])];

        while let Some((node, current_path)) = queue.pop() {
            if node == to_idx {
                path = current_path;
                break;
            }
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node);

            for neighbor in self.graph.neighbors(node) {
                if !visited.contains(&neighbor) {
                    let mut new_path = current_path.clone();
                    new_path.push(neighbor);
                    queue.push((neighbor, new_path));
                }
            }
        }

        if path.is_empty() {
            return None;
        }

        Some(
            path.iter()
                .filter_map(|&idx| self.nodes.get(idx.index()))
                .collect(),
        )
    }

    /// Get all reachable nodes from a starting node.
    pub fn reachable_from(&self, id: &str) -> Vec<&CallNode> {
        let Some(&start_idx) = self.id_to_index.get(id) else {
            return Vec::new();
        };

        let costs = dijkstra(&self.graph, start_idx, None, |_| 1u32);

        costs
            .keys()
            .filter_map(|&idx| self.nodes.get(idx.index()))
            .collect()
    }

    /// Calculate the depth of the call graph from roots.
    pub fn calculate_max_depth(&self) -> usize {
        if self.roots.is_empty() {
            return 0;
        }

        let mut max_depth = 0;

        for root in &self.roots {
            if let Some(&idx) = self.id_to_index.get(root) {
                let costs = dijkstra(&self.graph, idx, None, |_| 1u32);
                if let Some(&depth) = costs.values().max() {
                    max_depth = max_depth.max(depth as usize);
                }
            }
        }

        max_depth
    }

    // -------------------------------------------------------------------------
    // Root Management
    // -------------------------------------------------------------------------

    /// Mark a node as a root/entry point.
    pub fn add_root(&mut self, id: impl Into<String>) {
        let id = id.into();
        if self.has_node(&id) && !self.roots.contains(&id) {
            self.roots.push(id);
        }
    }

    /// Auto-detect root nodes (nodes with no incoming edges).
    pub fn detect_roots(&mut self) {
        self.roots.clear();

        for (id, &idx) in &self.id_to_index {
            if self
                .graph
                .edges_directed(idx, Direction::Incoming)
                .next()
                .is_none()
            {
                self.roots.push(id.clone());
            }
        }
    }

    // -------------------------------------------------------------------------
    // Analysis Helpers
    // -------------------------------------------------------------------------

    /// Get nodes by kind.
    pub fn nodes_by_kind(&self, kind: CallableKind) -> Vec<&CallNode> {
        self.nodes.iter().filter(|n| n.kind == kind).collect()
    }

    /// Get nodes in a specific file.
    pub fn nodes_in_file(&self, file_path: &str) -> Vec<&CallNode> {
        self.nodes
            .iter()
            .filter(|n| n.file_path.as_deref() == Some(file_path))
            .collect()
    }

    /// Get leaf nodes (no outgoing calls).
    pub fn leaf_nodes(&self) -> Vec<&CallNode> {
        self.nodes
            .iter()
            .filter(|n| {
                self.id_to_index.get(&n.id).map_or(false, |&idx| {
                    self.graph
                        .edges_directed(idx, Direction::Outgoing)
                        .next()
                        .is_none()
                })
            })
            .collect()
    }

    /// Get nodes with most incoming calls (most called functions).
    pub fn most_called(&self, limit: usize) -> Vec<(&CallNode, usize)> {
        let mut counts: Vec<_> = self
            .nodes
            .iter()
            .map(|n| {
                let count = self
                    .id_to_index
                    .get(&n.id)
                    .map(|&idx| self.graph.edges_directed(idx, Direction::Incoming).count())
                    .unwrap_or(0);
                (n, count)
            })
            .collect();

        counts.sort_by(|a, b| b.1.cmp(&a.1));
        counts.truncate(limit);
        counts
    }

    /// Update metadata based on current graph state.
    pub fn update_metadata(&mut self) {
        self.metadata.has_cycles = self.has_cycles();
        self.metadata.max_depth = self.calculate_max_depth();

        // Collect languages from nodes
        self.metadata.languages.clear();
        for node in &self.nodes {
            if let Some(ref path) = node.file_path {
                if let Some(ext) = std::path::Path::new(path).extension() {
                    let lang = match ext.to_str() {
                        Some("rs") => "rust",
                        Some("py") => "python",
                        Some("js") | Some("ts") => "javascript",
                        Some("go") => "go",
                        Some("sh") | Some("bash") => "shell",
                        Some("rb") => "ruby",
                        Some("java") => "java",
                        _ => "unknown",
                    };
                    self.metadata.languages.insert(lang.to_string());
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Serialization
    // -------------------------------------------------------------------------

    /// Rebuild internal indices after deserialization.
    pub fn rebuild_indices(&mut self) {
        self.graph = DiGraph::new();
        self.id_to_index.clear();

        // Add all nodes
        for node in &self.nodes {
            let index = self.graph.add_node(());
            self.id_to_index.insert(node.id.clone(), index);
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> CallGraph {
        let mut graph = CallGraph::with_name("test");

        // Create nodes: main -> foo -> bar -> baz
        //                    -> qux (parallel)
        graph.add_node(
            CallNode::new("main", "main", CallableKind::Function)
                .with_location("main.rs", 1)
                .with_visibility(true),
        );
        graph.add_node(
            CallNode::new("foo", "foo", CallableKind::Function).with_location("lib.rs", 10),
        );
        graph.add_node(
            CallNode::new("bar", "bar", CallableKind::Function).with_location("lib.rs", 20),
        );
        graph.add_node(
            CallNode::new("baz", "baz", CallableKind::Function).with_location("lib.rs", 30),
        );
        graph.add_node(
            CallNode::new("qux", "qux", CallableKind::Function).with_location("util.rs", 1),
        );

        graph.add_call("main", "foo");
        graph.add_call("main", "qux");
        graph.add_call("foo", "bar");
        graph.add_call("bar", "baz");

        graph.add_root("main");
        graph.update_metadata();

        graph
    }

    // -------------------------------------------------------------------------
    // Basic Operations
    // -------------------------------------------------------------------------

    #[test]
    fn test_new_graph() {
        let graph = CallGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.roots.is_empty());
    }

    #[test]
    fn test_with_name() {
        let graph = CallGraph::with_name("my_project");
        assert_eq!(graph.metadata.name.as_deref(), Some("my_project"));
    }

    #[test]
    fn test_add_node() {
        let mut graph = CallGraph::new();
        let id = graph.add_node(CallNode::new("test_fn", "test_fn", CallableKind::Function));
        assert_eq!(id, "test_fn");
        assert_eq!(graph.node_count(), 1);
        assert!(graph.has_node("test_fn"));
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("test_fn", "test_fn", CallableKind::Function));
        graph.add_node(CallNode::new("test_fn", "different", CallableKind::Method));

        assert_eq!(graph.node_count(), 1);
        // First one wins
        assert_eq!(graph.get_node("test_fn").unwrap().name, "test_fn");
    }

    #[test]
    fn test_get_node() {
        let graph = create_test_graph();

        let node = graph.get_node("foo").unwrap();
        assert_eq!(node.name, "foo");
        assert_eq!(node.file_path.as_deref(), Some("lib.rs"));
        assert_eq!(node.line, Some(10));
    }

    #[test]
    fn test_get_node_not_found() {
        let graph = create_test_graph();
        assert!(graph.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_remove_node() {
        let mut graph = create_test_graph();
        let removed = graph.remove_node("qux");

        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "qux");
        assert!(!graph.has_node("qux"));
    }

    // -------------------------------------------------------------------------
    // Edge Operations
    // -------------------------------------------------------------------------

    #[test]
    fn test_add_edge() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));
        graph.add_node(CallNode::new("b", "b", CallableKind::Function));

        let edge = CallEdge::new(CallKind::Method)
            .with_location(10, Some(5))
            .conditional();

        assert!(graph.add_edge("a", "b", edge));
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_add_edge_nonexistent_node() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));

        assert!(!graph.add_edge("a", "nonexistent", CallEdge::default()));
    }

    #[test]
    fn test_calls_from() {
        let graph = create_test_graph();
        let calls = graph.calls_from("main");

        assert_eq!(calls.len(), 2);
        let names: Vec<_> = calls.iter().map(|(n, _)| n.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"qux"));
    }

    #[test]
    fn test_callers_of() {
        let graph = create_test_graph();
        let callers = graph.callers_of("bar");

        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].0.name, "foo");
    }

    // -------------------------------------------------------------------------
    // Graph Algorithms
    // -------------------------------------------------------------------------

    #[test]
    fn test_has_cycles_no_cycle() {
        let graph = create_test_graph();
        assert!(!graph.has_cycles());
    }

    #[test]
    fn test_has_cycles_with_cycle() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));
        graph.add_node(CallNode::new("b", "b", CallableKind::Function));
        graph.add_node(CallNode::new("c", "c", CallableKind::Function));

        graph.add_call("a", "b");
        graph.add_call("b", "c");
        graph.add_call("c", "a"); // Cycle!

        assert!(graph.has_cycles());
    }

    #[test]
    fn test_topological_order() {
        let graph = create_test_graph();
        let order = graph.topological_order().unwrap();

        // main should come before foo
        let main_pos = order.iter().position(|n| n.name == "main").unwrap();
        let foo_pos = order.iter().position(|n| n.name == "foo").unwrap();
        assert!(main_pos < foo_pos);

        // foo should come before bar
        let bar_pos = order.iter().position(|n| n.name == "bar").unwrap();
        assert!(foo_pos < bar_pos);
    }

    #[test]
    fn test_topological_order_with_cycle() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));
        graph.add_node(CallNode::new("b", "b", CallableKind::Function));
        graph.add_call("a", "b");
        graph.add_call("b", "a");

        assert!(graph.topological_order().is_none());
    }

    #[test]
    fn test_strongly_connected_components() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));
        graph.add_node(CallNode::new("b", "b", CallableKind::Function));
        graph.add_node(CallNode::new("c", "c", CallableKind::Function));
        graph.add_node(CallNode::new("d", "d", CallableKind::Function));

        // a <-> b form a cycle
        graph.add_call("a", "b");
        graph.add_call("b", "a");
        // c -> d (no cycle)
        graph.add_call("c", "d");

        let sccs = graph.strongly_connected_components();

        // Should have 3 SCCs: {a,b}, {c}, {d}
        assert_eq!(sccs.len(), 3);

        // One SCC should have 2 elements (the cycle)
        let cycle_scc = sccs.iter().find(|scc| scc.len() == 2);
        assert!(cycle_scc.is_some());
    }

    #[test]
    fn test_shortest_path() {
        let graph = create_test_graph();

        let path = graph.shortest_path("main", "baz").unwrap();
        let names: Vec<_> = path.iter().map(|n| n.name.as_str()).collect();

        // main -> foo -> bar -> baz
        assert_eq!(names, vec!["main", "foo", "bar", "baz"]);
    }

    #[test]
    fn test_shortest_path_no_path() {
        let graph = create_test_graph();
        // baz has no outgoing edges, can't reach main
        assert!(graph.shortest_path("baz", "main").is_none());
    }

    #[test]
    fn test_reachable_from() {
        let graph = create_test_graph();
        let reachable = graph.reachable_from("main");

        // main can reach all nodes
        assert_eq!(reachable.len(), 5);
    }

    #[test]
    fn test_calculate_max_depth() {
        let graph = create_test_graph();
        // main -> foo -> bar -> baz = depth 3
        assert_eq!(graph.calculate_max_depth(), 3);
    }

    // -------------------------------------------------------------------------
    // Root Detection
    // -------------------------------------------------------------------------

    #[test]
    fn test_detect_roots() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("entry1", "entry1", CallableKind::Function));
        graph.add_node(CallNode::new("entry2", "entry2", CallableKind::Function));
        graph.add_node(CallNode::new(
            "internal",
            "internal",
            CallableKind::Function,
        ));

        graph.add_call("entry1", "internal");
        graph.add_call("entry2", "internal");

        graph.detect_roots();

        assert_eq!(graph.roots.len(), 2);
        assert!(graph.roots.contains(&"entry1".to_string()));
        assert!(graph.roots.contains(&"entry2".to_string()));
        assert!(!graph.roots.contains(&"internal".to_string()));
    }

    // -------------------------------------------------------------------------
    // Analysis Helpers
    // -------------------------------------------------------------------------

    #[test]
    fn test_nodes_by_kind() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("fn1", "fn1", CallableKind::Function));
        graph.add_node(CallNode::new("m1", "m1", CallableKind::Method));
        graph.add_node(CallNode::new("fn2", "fn2", CallableKind::Function));

        let functions = graph.nodes_by_kind(CallableKind::Function);
        assert_eq!(functions.len(), 2);

        let methods = graph.nodes_by_kind(CallableKind::Method);
        assert_eq!(methods.len(), 1);
    }

    #[test]
    fn test_nodes_in_file() {
        let graph = create_test_graph();

        let lib_nodes = graph.nodes_in_file("lib.rs");
        assert_eq!(lib_nodes.len(), 3); // foo, bar, baz
    }

    #[test]
    fn test_leaf_nodes() {
        let graph = create_test_graph();
        let leaves = graph.leaf_nodes();

        // baz and qux are leaves (no outgoing calls)
        assert_eq!(leaves.len(), 2);
        let names: Vec<_> = leaves.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"baz"));
        assert!(names.contains(&"qux"));
    }

    #[test]
    fn test_most_called() {
        let mut graph = CallGraph::new();
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));
        graph.add_node(CallNode::new("b", "b", CallableKind::Function));
        graph.add_node(CallNode::new("c", "c", CallableKind::Function));
        graph.add_node(CallNode::new("popular", "popular", CallableKind::Function));

        graph.add_call("a", "popular");
        graph.add_call("b", "popular");
        graph.add_call("c", "popular");
        graph.add_call("a", "b");

        let most = graph.most_called(2);
        assert_eq!(most[0].0.name, "popular");
        assert_eq!(most[0].1, 3);
    }

    // -------------------------------------------------------------------------
    // Metadata
    // -------------------------------------------------------------------------

    #[test]
    fn test_update_metadata() {
        let mut graph = create_test_graph();
        graph.update_metadata();

        assert!(!graph.metadata.has_cycles);
        assert_eq!(graph.metadata.max_depth, 3);
        assert!(graph.metadata.languages.contains("rust"));
    }

    // -------------------------------------------------------------------------
    // Node Builder
    // -------------------------------------------------------------------------

    #[test]
    fn test_call_node_builder() {
        let node = CallNode::new("my_func", "my_func", CallableKind::Method)
            .with_location("src/lib.rs", 42)
            .with_module("my_crate::module")
            .with_visibility(true)
            .with_signature(2, Some("Result<()>".to_string()));

        assert_eq!(node.id, "my_func");
        assert_eq!(node.file_path.as_deref(), Some("src/lib.rs"));
        assert_eq!(node.line, Some(42));
        assert_eq!(node.module_path.as_deref(), Some("my_crate::module"));
        assert!(node.is_public);
        assert_eq!(node.param_count, 2);
        assert_eq!(node.return_type.as_deref(), Some("Result<()>"));
    }

    #[test]
    fn test_call_edge_builder() {
        let edge = CallEdge::new(CallKind::Async)
            .with_location(100, Some(15))
            .conditional()
            .in_loop();

        assert_eq!(edge.kind, CallKind::Async);
        assert_eq!(edge.line, Some(100));
        assert_eq!(edge.column, Some(15));
        assert!(edge.is_conditional);
        assert!(edge.is_in_loop);
    }

    // -------------------------------------------------------------------------
    // Serialization
    // -------------------------------------------------------------------------

    #[test]
    fn test_serialization_roundtrip() {
        let graph = create_test_graph();

        let json = serde_json::to_string(&graph).unwrap();
        let mut deserialized: CallGraph = serde_json::from_str(&json).unwrap();

        // Rebuild indices after deserialization
        deserialized.rebuild_indices();

        assert_eq!(deserialized.node_count(), graph.node_count());
        assert_eq!(deserialized.roots, graph.roots);
    }
}
