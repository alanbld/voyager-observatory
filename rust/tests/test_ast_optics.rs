//! AST Optics Integration Tests
//!
//! Phase 5 TDD verification tests for voyager-ast integration.
//! These tests ensure:
//! 1. AST-based analysis correctly identifies code structures
//! 2. Human-facing output remains jargon-free (Telescope philosophy)

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test project with Rust code containing known structures
fn create_rust_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a Rust file with a struct and methods (like ContextEngine)
    fs::write(
        temp_dir.path().join("engine.rs"),
        r#"//! Engine module
//!
//! This module provides the core engine functionality.

use std::path::Path;

/// The main context engine for processing files
///
/// This struct orchestrates file walking, parsing, and serialization.
pub struct ContextEngine {
    /// Configuration options
    config: Config,
    /// Internal state
    state: EngineState,
}

impl ContextEngine {
    /// Create a new engine with default configuration
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            state: EngineState::Idle,
        }
    }

    /// Process a directory and generate context
    pub fn process(&mut self, path: &Path) -> Result<Output, Error> {
        self.state = EngineState::Processing;
        // Implementation details...
        Ok(Output::default())
    }

    /// Get the current engine state
    pub fn state(&self) -> &EngineState {
        &self.state
    }

    /// Internal helper method
    fn validate_config(&self) -> bool {
        true
    }
}

/// Configuration for the engine
#[derive(Default)]
pub struct Config {
    pub max_files: usize,
    pub include_hidden: bool,
}

/// Engine processing state
pub enum EngineState {
    Idle,
    Processing,
    Complete,
    Error(String),
}

/// Output from the engine
#[derive(Default)]
pub struct Output {
    pub files_processed: usize,
    pub total_bytes: usize,
}

/// Error type for engine operations
pub struct Error {
    pub message: String,
}
"#,
    )
    .unwrap();

    // Create a lib.rs that references engine
    fs::write(
        temp_dir.path().join("lib.rs"),
        r#"//! Library entry point
pub mod engine;
pub use engine::{ContextEngine, Config, Output};
"#,
    )
    .unwrap();

    // Create a Cargo.toml for proper project detection
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    temp_dir
}

// ============================================================================
// Requirement 1: AST-based Structure Detection
// ============================================================================

#[test]
fn test_ast_detects_struct_contextengine() {
    let temp_dir = create_rust_test_project();

    // Run vo on the test project with skeleton mode (which uses AST analysis)
    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--skeleton")
        .arg("auto")
        .arg("--format")
        .arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify ContextEngine struct is detected
    assert!(
        stdout.contains("ContextEngine"),
        "AST should detect ContextEngine struct. Output:\n{}",
        stdout
    );
}

#[test]
fn test_ast_detects_struct_methods() {
    let temp_dir = create_rust_test_project();

    // Run with skeleton mode to extract method signatures
    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--skeleton")
        .arg("auto")
        .arg("--format")
        .arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify key methods are detected
    assert!(
        stdout.contains("fn new"),
        "AST should detect 'new' method. Output:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fn process"),
        "AST should detect 'process' method. Output:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fn state"),
        "AST should detect 'state' method. Output:\n{}",
        stdout
    );
}

#[test]
fn test_ast_detects_enum_variants() {
    let temp_dir = create_rust_test_project();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--skeleton")
        .arg("auto")
        .arg("--format")
        .arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify enum is detected
    assert!(
        stdout.contains("EngineState"),
        "AST should detect EngineState enum. Output:\n{}",
        stdout
    );
}

#[test]
fn test_ast_detects_impl_blocks() {
    let temp_dir = create_rust_test_project();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--skeleton")
        .arg("auto")
        .arg("--format")
        .arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify impl block methods are extracted (impl ContextEngine)
    // The skeleton should include the public methods
    assert!(
        stdout.contains("pub fn new") || stdout.contains("fn new"),
        "AST should detect impl methods. Output:\n{}",
        stdout
    );
}

#[test]
fn test_ast_detects_doc_comments() {
    let temp_dir = create_rust_test_project();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path()).arg("--format").arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify doc comments are captured
    assert!(
        stdout.contains("context engine") || stdout.contains("main context engine"),
        "AST should capture doc comments. Output fragment:\n{}",
        &stdout[..stdout.len().min(2000)]
    );
}

// ============================================================================
// Requirement 2: Jargon-Free Output (Telescope Philosophy)
// ============================================================================

/// List of forbidden jargon terms that should never appear in user-facing output
const FORBIDDEN_JARGON: &[&str] = &[
    "AST",         // Abstract Syntax Tree - too technical
    "Tree-sitter", // Implementation detail
    "tree_sitter", // snake_case variant
    "TreeSitter",  // PascalCase variant
    "node",        // Tree terminology (when referring to AST nodes)
    "subtree",     // Tree terminology
    "parse tree",  // Compiler terminology
    "syntax tree", // Compiler terminology
    "grammar",     // Compiler terminology (unless in doc context)
];

/// Terms that ARE allowed (our metaphor)
const ALLOWED_TERMS: &[&str] = &[
    "star",          // VO metaphor for symbols
    "nebula",        // VO metaphor for modules
    "constellation", // VO metaphor for related code
    "telescope",     // VO metaphor for exploration
    "zoom",          // VO metaphor for focus
    "function",      // Standard programming term
    "struct",        // Standard Rust term
    "method",        // Standard programming term
];

#[test]
fn test_output_is_jargon_free_default_format() {
    let temp_dir = create_rust_test_project();

    // Run with default format (plusminus)
    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let stdout_lower = stdout.to_lowercase();

    // Check that forbidden jargon doesn't appear
    for jargon in FORBIDDEN_JARGON {
        let jargon_lower = jargon.to_lowercase();
        // Allow "AST" in file paths like "voyager-ast" but not as standalone term
        if *jargon == "AST" {
            // Check for standalone "AST" (not part of a path)
            let has_standalone_ast = stdout
                .split_whitespace()
                .any(|word| word == "AST" || word.starts_with("AST:") || word.ends_with(":AST"));
            assert!(
                !has_standalone_ast,
                "Output should not contain standalone jargon term '{}' in user-facing text.\nFound in output.",
                jargon
            );
        } else {
            assert!(
                !stdout_lower.contains(&jargon_lower),
                "Output should not contain jargon term '{}' in user-facing text.\nOutput excerpt:\n{}",
                jargon,
                &stdout[..stdout.len().min(1000)]
            );
        }
    }
}

#[test]
fn test_output_is_jargon_free_xml_format() {
    let temp_dir = create_rust_test_project();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path()).arg("--format").arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let stdout_lower = stdout.to_lowercase();

    // Check specific jargon that should never appear
    for jargon in &[
        "Tree-sitter",
        "tree_sitter",
        "TreeSitter",
        "parse tree",
        "syntax tree",
    ] {
        assert!(
            !stdout_lower.contains(&jargon.to_lowercase()),
            "XML output should not contain '{}' jargon",
            jargon
        );
    }
}

#[test]
fn test_output_is_jargon_free_skeleton_mode() {
    let temp_dir = create_rust_test_project();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--skeleton")
        .arg("auto")
        .arg("--format")
        .arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let stdout_lower = stdout.to_lowercase();

    // Skeleton mode especially should be jargon-free since it uses AST heavily
    for jargon in &[
        "Tree-sitter",
        "tree_sitter",
        "TreeSitter",
        "subtree",
        "parse tree",
    ] {
        assert!(
            !stdout_lower.contains(&jargon.to_lowercase()),
            "Skeleton output should not contain '{}' jargon. Output:\n{}",
            jargon,
            &stdout[..stdout.len().min(500)]
        );
    }
}

#[test]
fn test_error_messages_are_jargon_free() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with syntax errors to trigger error handling
    fs::write(
        temp_dir.path().join("broken.rs"),
        r#"
// Intentionally broken Rust code
pub fn broken( {
    let x =
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path()).arg("--skeleton").arg("auto");

    // The command should still succeed (graceful degradation)
    // but any warnings/errors should be jargon-free
    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);

    // Combine output for checking
    let combined = format!("{}{}", stdout, stderr).to_lowercase();

    // Even error messages should avoid implementation jargon
    for jargon in &["tree-sitter", "tree_sitter", "parse error at node"] {
        assert!(
            !combined.contains(&jargon.to_lowercase()),
            "Error messages should not expose '{}' jargon",
            jargon
        );
    }
}

// ============================================================================
// Edge Cases and Robustness
// ============================================================================

#[test]
fn test_ast_handles_empty_project() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path());

    // Should not crash on empty directory
    cmd.assert().success();
}

#[test]
fn test_ast_handles_non_rust_files_gracefully() {
    let temp_dir = TempDir::new().unwrap();

    // Create a Python file (AST fallback to regex)
    fs::write(
        temp_dir.path().join("main.py"),
        r#"
class ContextEngine:
    """Python context engine."""
    def process(self):
        pass
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("vo").unwrap();
    cmd.arg(temp_dir.path()).arg("--skeleton").arg("auto");

    // Should handle Python gracefully (regex fallback)
    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Python class should still be detected via regex heuristics
    assert!(
        stdout.contains("ContextEngine") || stdout.contains("class"),
        "Should detect Python structures via fallback. Output:\n{}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_ast_analysis_on_actual_project() {
    // Run on the actual vo project to verify real-world behavior
    // Use zoom to target a specific file since vo requires directories
    let mut cmd = Command::cargo_bin("vo").unwrap();

    // Get the project root (parent of target directory)
    let project_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    cmd.current_dir(&project_root)
        .arg("src/core") // Run on the core directory
        .arg("--skeleton")
        .arg("auto")
        .arg("--format")
        .arg("xml");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // The actual ContextEngine should be detected in one of the files
    assert!(
        stdout.contains("ContextEngine"),
        "Should detect ContextEngine in actual project. Output fragment:\n{}",
        &stdout[..stdout.len().min(2000)]
    );
}

// ============================================================================
// voyager-ast Direct Integration Tests
// ============================================================================

#[test]
fn test_voyager_ast_rust_adapter() {
    use pm_encoder::core::ast_bridge::AstBridge;
    use voyager_ast::LanguageId;

    let bridge = AstBridge::new();

    // Test that Rust is supported
    assert!(
        bridge.supports(LanguageId::Rust),
        "AstBridge should support Rust"
    );

    // Parse some Rust code
    let source = r#"
/// A test struct
pub struct TestStruct {
    field: i32,
}

impl TestStruct {
    pub fn new() -> Self {
        Self { field: 0 }
    }
}
"#;

    let file = bridge.analyze_file(source, LanguageId::Rust);
    assert!(file.is_some(), "Should successfully parse Rust code");

    let file = file.unwrap();
    let stars = bridge.extract_stars(&file);

    // Should find the struct and its method
    assert!(
        stars.iter().any(|s| s.name == "TestStruct"),
        "Should extract TestStruct as a star"
    );
}

#[test]
fn test_voyager_ast_graceful_fallback() {
    use pm_encoder::core::ast_bridge::AstBridge;
    use voyager_ast::LanguageId;

    let bridge = AstBridge::new();

    // Go is not yet supported - should return None gracefully, not panic
    let result = bridge.analyze_file("package main\nfunc main() {}", LanguageId::Go);
    assert!(result.is_none(), "Go should not be supported yet");

    // Python is now supported (Phase 1B Core Fleet) - should return valid AST
    let result = bridge.analyze_file("def foo(): pass", LanguageId::Python);
    assert!(result.is_some(), "Python should be supported");
    let file = result.unwrap();
    assert!(
        !file.declarations.is_empty(),
        "Python should extract declarations"
    );
    assert!(
        file.declarations.iter().any(|d| d.name == "foo"),
        "Should extract 'foo' function"
    );
}
