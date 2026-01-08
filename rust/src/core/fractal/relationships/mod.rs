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
}
