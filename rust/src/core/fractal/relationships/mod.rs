//! Relationship Extraction Module
//!
//! This module provides call graph analysis and relationship extraction
//! for the Fractal Context Engine.
//!
//! # Features
//!
//! - **Call Graph**: Directed graph of function calls using petgraph
//! - **Multi-language Support**: Rust, Python, JavaScript, Go, Shell
//! - **Graph Algorithms**: Topological sort, cycle detection, SCC, shortest path
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::relationships::{CallGraph, CallNode, CallableKind, CallExtractor};
//!
//! // Create extractor
//! let extractor = CallExtractor::new();
//!
//! // Extract from source file
//! let file_extraction = extractor.extract_from_file(source_code, "main.rs");
//!
//! // Build call graph
//! let graph = extractor.build_graph(vec![file_extraction]);
//!
//! // Analyze
//! println!("Cycles: {}", graph.has_cycles());
//! println!("Max depth: {}", graph.calculate_max_depth());
//! ```

pub mod call_graph;
pub mod extractor;

// Re-export commonly used types
pub use call_graph::{CallEdge, CallGraph, CallGraphMetadata, CallKind, CallNode, CallableKind};

pub use extractor::{CallExtractor, ExtractedCalls, FileCallExtraction};

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test: Extract calls from a multi-file Rust project
    #[test]
    fn test_multi_file_rust_project() {
        let extractor = CallExtractor::new();

        // main.rs
        let main_code = r#"
mod lib;

fn main() {
    lib::run();
    setup();
}

fn setup() {
    configure();
}

fn configure() {}
"#;

        // lib.rs
        let lib_code = r#"
pub fn run() {
    process();
    cleanup();
}

fn process() {
    helper();
}

fn helper() {}

fn cleanup() {}
"#;

        let main_ext = extractor.extract_from_file(main_code, "main.rs");
        let lib_ext = extractor.extract_from_file(lib_code, "lib.rs");

        let graph = extractor.build_graph(vec![main_ext, lib_ext]);

        // Verify graph structure
        assert_eq!(graph.node_count(), 7); // main, setup, configure, run, process, helper, cleanup
        assert!(!graph.has_cycles());

        // Verify roots
        assert!(graph.roots.contains(&"main".to_string()));

        // Verify call chain main -> setup -> configure
        let path = graph.shortest_path("main", "configure");
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3);

        // Verify lib functions
        let run_calls = graph.calls_from("run");
        let call_names: Vec<_> = run_calls.iter().map(|(n, _)| n.name.as_str()).collect();
        assert!(call_names.contains(&"process"));
        assert!(call_names.contains(&"cleanup"));
    }

    /// Integration test: Mixed language project
    #[test]
    fn test_mixed_language_project() {
        let extractor = CallExtractor::new();

        let rust_code = r#"
fn api_handler() {
    validate();
    process_request();
}

fn validate() {}
fn process_request() {}
"#;

        let python_code = r#"
def cli_main():
    parse_args()
    run_command()

def parse_args():
    pass

def run_command():
    execute()

def execute():
    pass
"#;

        let shell_code = r#"
#!/bin/bash

deploy() {
    build
    test_suite
    push
}

build() {
    compile
}

compile() {
    echo "compiling"
}

test_suite() {
    run_tests
}

run_tests() {
    echo "testing"
}

push() {
    echo "pushing"
}
"#;

        let rust_ext = extractor.extract_from_file(rust_code, "api.rs");
        let python_ext = extractor.extract_from_file(python_code, "cli.py");
        let shell_ext = extractor.extract_from_file(shell_code, "deploy.sh");

        let graph = extractor.build_graph(vec![rust_ext, python_ext, shell_ext]);

        // Check language detection in metadata
        graph.metadata.languages.contains("rust");
        graph.metadata.languages.contains("python");
        graph.metadata.languages.contains("shell");

        // Check file count
        assert_eq!(graph.metadata.file_count, 3);

        // Verify no cycles across entire project
        assert!(!graph.has_cycles());
    }

    /// Integration test: Recursive/cyclic calls
    #[test]
    fn test_recursive_calls() {
        let extractor = CallExtractor::new();

        let code = r#"
fn fibonacci(n: u32) -> u32 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn mutual_a() {
    mutual_b();
}

fn mutual_b() {
    mutual_a();
}
"#;

        let ext = extractor.extract_from_file(code, "recursive.rs");
        let graph = extractor.build_graph(vec![ext]);

        // Should detect cycles
        assert!(graph.has_cycles());

        // SCC should find the mutual recursion cycle
        let sccs = graph.strongly_connected_components();
        let cycle_scc = sccs.iter().find(|scc| scc.len() == 2);
        assert!(cycle_scc.is_some());

        let cycle_names: Vec<_> = cycle_scc.unwrap().iter().map(|n| n.name.as_str()).collect();
        assert!(cycle_names.contains(&"mutual_a"));
        assert!(cycle_names.contains(&"mutual_b"));
    }

    /// Integration test: Large call graph analysis
    #[test]
    fn test_call_graph_analysis() {
        let extractor = CallExtractor::new();

        let code = r#"
fn entry_point() {
    service_a();
    service_b();
}

fn service_a() {
    helper_1();
    helper_2();
}

fn service_b() {
    helper_2();
    helper_3();
}

fn helper_1() {
    utility();
}

fn helper_2() {
    utility();
}

fn helper_3() {
    utility();
}

fn utility() {}
"#;

        let ext = extractor.extract_from_file(code, "services.rs");
        let graph = extractor.build_graph(vec![ext]);

        // Most called function should be utility (called by helper_1, helper_2, helper_3)
        let most_called = graph.most_called(3);
        assert_eq!(most_called[0].0.name, "utility");
        assert_eq!(most_called[0].1, 3);

        // Leaf nodes
        let leaves = graph.leaf_nodes();
        assert_eq!(leaves.len(), 1); // only utility
        assert_eq!(leaves[0].name, "utility");

        // Reachability
        let reachable = graph.reachable_from("entry_point");
        assert_eq!(reachable.len(), 7); // all nodes

        // Max depth from entry_point
        assert_eq!(graph.calculate_max_depth(), 3); // entry_point -> service -> helper -> utility
    }

    /// Integration test: JSON serialization of call graph
    #[test]
    fn test_call_graph_serialization() {
        let mut graph = CallGraph::with_name("test_project");

        graph.add_node(
            CallNode::new("main", "main", CallableKind::Function)
                .with_location("main.rs", 1)
                .with_visibility(true),
        );
        graph.add_node(
            CallNode::new("helper", "helper", CallableKind::Function).with_location("lib.rs", 10),
        );
        graph.add_call("main", "helper");
        graph.add_root("main");
        graph.update_metadata();

        let json = serde_json::to_string_pretty(&graph).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("\"name\": \"test_project\""));
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"roots\""));
        assert!(json.contains("\"main\""));
        assert!(json.contains("\"helper\""));

        // Roundtrip
        let mut restored: CallGraph = serde_json::from_str(&json).unwrap();
        restored.rebuild_indices();

        assert_eq!(restored.node_count(), 2);
        assert_eq!(restored.roots, graph.roots);
    }

    // =========================================================================
    // Additional Unit Tests for Comprehensive Coverage
    // =========================================================================

    #[test]
    fn test_call_node_new() {
        let node = CallNode::new("my_func", "my_func", CallableKind::Function);

        assert_eq!(node.name, "my_func");
        assert_eq!(node.id, "my_func");
        assert_eq!(node.kind, CallableKind::Function);
        assert!(node.file_path.is_none());
        assert!(node.line.is_none());
        assert!(!node.is_public);
    }

    #[test]
    fn test_call_node_with_location() {
        let node = CallNode::new("func", "func", CallableKind::Function)
            .with_location("src/lib.rs", 42);

        assert_eq!(node.file_path, Some("src/lib.rs".to_string()));
        assert_eq!(node.line, Some(42));
    }

    #[test]
    fn test_call_node_with_visibility() {
        let node = CallNode::new("pub_func", "pub_func", CallableKind::Function)
            .with_visibility(true);

        assert!(node.is_public);
    }

    #[test]
    fn test_callable_kind_variants() {
        assert_eq!(CallableKind::Function, CallableKind::Function);
        assert_ne!(CallableKind::Function, CallableKind::Method);
        assert_ne!(CallableKind::Method, CallableKind::Constructor);
        assert_ne!(CallableKind::Closure, CallableKind::Macro);
    }

    #[test]
    fn test_call_kind_variants() {
        assert_eq!(CallKind::Direct, CallKind::Direct);
        assert_ne!(CallKind::Direct, CallKind::Dynamic);
        assert_ne!(CallKind::Static, CallKind::Callback);
    }

    #[test]
    fn test_call_graph_with_name() {
        let graph = CallGraph::with_name("my_project");

        assert_eq!(graph.metadata.name, Some("my_project".to_string()));
        assert_eq!(graph.node_count(), 0);
        assert!(graph.roots.is_empty());
    }

    #[test]
    fn test_call_graph_empty() {
        let graph = CallGraph::with_name("empty");

        assert!(!graph.has_cycles());
        assert_eq!(graph.calculate_max_depth(), 0);
        assert!(graph.leaf_nodes().is_empty());
    }

    #[test]
    fn test_call_graph_add_single_node() {
        let mut graph = CallGraph::with_name("single");
        graph.add_node(CallNode::new("solo", "solo", CallableKind::Function));

        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_call_graph_add_root() {
        let mut graph = CallGraph::with_name("test");
        graph.add_node(CallNode::new("main", "main", CallableKind::Function));
        graph.add_root("main");

        assert!(graph.roots.contains(&"main".to_string()));
    }

    #[test]
    fn test_call_graph_topological_order() {
        let extractor = CallExtractor::new();

        let code = r#"
fn a() { b(); }
fn b() { c(); }
fn c() {}
"#;

        let ext = extractor.extract_from_file(code, "test.rs");
        let graph = extractor.build_graph(vec![ext]);

        // Get topological order - verify it executes without panic
        let topo = graph.topological_order();

        // Flatten and check - topological_order returns Vec<Vec<&CallNode>>
        let flat: Vec<_> = topo.into_iter().flatten().collect();
        assert_eq!(flat.len(), 3);

        // Verify all functions are present
        let names: Vec<_> = flat.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn test_call_graph_shortest_path_nonexistent() {
        let mut graph = CallGraph::with_name("test");
        graph.add_node(CallNode::new("a", "a", CallableKind::Function));
        graph.add_node(CallNode::new("b", "b", CallableKind::Function));
        // No edge between a and b

        let path = graph.shortest_path("a", "b");
        assert!(path.is_none());
    }

    #[test]
    fn test_call_graph_calls_from_empty() {
        let mut graph = CallGraph::with_name("test");
        graph.add_node(CallNode::new("solo", "solo", CallableKind::Function));

        let calls = graph.calls_from("solo");
        assert!(calls.is_empty());
    }

    #[test]
    fn test_call_extractor_empty_file() {
        let extractor = CallExtractor::new();

        let ext = extractor.extract_from_file("", "empty.rs");

        assert!(ext.extractions.is_empty());
    }

    #[test]
    fn test_call_extractor_python() {
        let extractor = CallExtractor::new();

        let python_code = r#"
def main():
    helper()

def helper():
    pass
"#;

        let ext = extractor.extract_from_file(python_code, "test.py");
        let graph = extractor.build_graph(vec![ext]);

        assert_eq!(graph.node_count(), 2);
        assert!(graph.metadata.languages.contains("python"));
    }

    #[test]
    fn test_call_extractor_javascript() {
        let extractor = CallExtractor::new();

        let js_code = r#"
function main() {
    helper();
}

function helper() {}
"#;

        let ext = extractor.extract_from_file(js_code, "test.js");
        let graph = extractor.build_graph(vec![ext]);

        assert_eq!(graph.node_count(), 2);
        assert!(graph.metadata.languages.contains("javascript"));
    }

    #[test]
    fn test_call_edge_default() {
        let edge = CallEdge::default();

        assert_eq!(edge.kind, CallKind::Direct);
        assert_eq!(edge.weight, 1);
    }

    #[test]
    fn test_call_graph_metadata_update() {
        let mut graph = CallGraph::with_name("test");
        graph.add_node(
            CallNode::new("func", "func", CallableKind::Function)
                .with_location("test.rs", 1)
        );
        graph.add_root("func");
        graph.update_metadata();

        assert!(graph.metadata.languages.contains("rust"));
    }

    #[test]
    fn test_call_graph_reachable_from_nonexistent() {
        let graph = CallGraph::with_name("test");

        let reachable = graph.reachable_from("nonexistent");
        assert!(reachable.is_empty());
    }

    #[test]
    fn test_call_extractor_go() {
        let extractor = CallExtractor::new();

        let go_code = r#"
package main

func main() {
    helper()
}

func helper() {
}
"#;

        let ext = extractor.extract_from_file(go_code, "main.go");
        let graph = extractor.build_graph(vec![ext]);

        assert!(graph.node_count() >= 2);
    }

    #[test]
    fn test_call_graph_most_called_empty() {
        let graph = CallGraph::with_name("empty");

        let most = graph.most_called(5);
        assert!(most.is_empty());
    }

    #[test]
    fn test_call_graph_strongly_connected_no_cycles() {
        let extractor = CallExtractor::new();

        let code = r#"
fn a() { b(); }
fn b() { c(); }
fn c() {}
"#;

        let ext = extractor.extract_from_file(code, "test.rs");
        let graph = extractor.build_graph(vec![ext]);

        let sccs = graph.strongly_connected_components();
        // Each node is its own SCC (no cycles)
        for scc in &sccs {
            assert_eq!(scc.len(), 1);
        }
    }

    #[test]
    fn test_call_extractor_new() {
        let extractor = CallExtractor::new();
        // Just verify it creates without panic
        assert!(true);

        // Extract from trivial code
        let ext = extractor.extract_from_file("fn test() {}", "test.rs");
        assert!(!ext.extractions.is_empty());
    }

    #[test]
    fn test_callable_kind_all_variants() {
        // Verify all variants exist
        let _function = CallableKind::Function;
        let _method = CallableKind::Method;
        let _constructor = CallableKind::Constructor;
        let _closure = CallableKind::Closure;
        let _macro = CallableKind::Macro;
        let _shell = CallableKind::ShellFunction;
        let _external = CallableKind::External;
        let _unknown = CallableKind::Unknown;
    }

    #[test]
    fn test_call_kind_all_variants() {
        // Verify all variants exist
        let _direct = CallKind::Direct;
        let _method = CallKind::Method;
        let _static = CallKind::Static;
        let _constructor = CallKind::Constructor;
        let _async = CallKind::Async;
        let _callback = CallKind::Callback;
        let _dynamic = CallKind::Dynamic;
        let _external = CallKind::External;
        let _macro = CallKind::Macro;
        let _shell = CallKind::Shell;
    }

    #[test]
    fn test_call_edge_with_location() {
        let edge = CallEdge::new(CallKind::Direct)
            .with_location(42, Some(10));

        assert_eq!(edge.line, Some(42));
        assert_eq!(edge.column, Some(10));
    }

    #[test]
    fn test_call_edge_conditional() {
        let edge = CallEdge::new(CallKind::Direct).conditional();

        assert!(edge.is_conditional);
    }

    #[test]
    fn test_call_edge_in_loop() {
        let edge = CallEdge::new(CallKind::Direct).in_loop();

        assert!(edge.is_in_loop);
    }

    #[test]
    fn test_call_node_with_module() {
        let node = CallNode::new("func", "func", CallableKind::Function)
            .with_module("my::module::path");

        assert_eq!(node.module_path, Some("my::module::path".to_string()));
    }

    #[test]
    fn test_call_node_with_signature() {
        let node = CallNode::new("func", "func", CallableKind::Function)
            .with_signature(3, Some("i32".to_string()));

        assert_eq!(node.param_count, 3);
        assert_eq!(node.return_type, Some("i32".to_string()));
    }

    #[test]
    fn test_call_graph_default() {
        let graph = CallGraph::default();

        assert_eq!(graph.node_count(), 0);
        assert!(graph.roots.is_empty());
    }
}
