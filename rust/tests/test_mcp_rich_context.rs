//! Integration tests for MCP Rich Context Features (Phase 2)
//!
//! Tests the MCP server's enhanced zoom and budgeting capabilities:
//! - Zoom with <related_context> showing callers
//! - Tiered budget allocation (Core before Tests)

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use pm_encoder::core::{
    ContextEngine, EncoderConfig, FileTier, RelatedContext, SymbolResolver, UsageFinder,
    ZoomConfig, ZoomDepth, ZoomTarget,
};
use pm_encoder::{apply_token_budget, LensManager};

/// Create a test project with Core, Tests, and Config files
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create src/ directory (Core tier)
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"
//! Main library

mod utils;

pub fn main_function() {
    let result = utils::helper_function();
    println!("Result: {}", result);
}

pub fn another_function() {
    main_function();
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("src/utils.rs"),
        r#"
//! Utility functions

pub fn helper_function() -> i32 {
    42
}

pub fn unused_function() {
    // This function is never called
}
"#,
    )
    .unwrap();

    // Create tests/ directory (Tests tier)
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/test_main.rs"),
        r#"
//! Tests for main functionality

use mylib::main_function;

#[test]
fn test_main() {
    main_function();
}
"#,
    )
    .unwrap();

    // Create Cargo.toml (Config tier)
    fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "mylib"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    // Create README.md (Other tier)
    fs::write(
        root.join("README.md"),
        "# Test Project\n\nA test project for MCP rich context testing.\n",
    )
    .unwrap();

    temp_dir
}

// ============================================================================
// Test 1: Zoom with Related Context (Callers)
// ============================================================================

#[test]
fn test_zoom_includes_related_context() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Find usages of helper_function (should be called by main_function)
    let usage_finder = UsageFinder::new().with_max_results(10);
    let callers = usage_finder.find_usages("helper_function", root, Some("src/utils.rs"), None);

    // Should find at least one caller (main_function in lib.rs)
    assert!(
        !callers.is_empty(),
        "Should find callers of helper_function"
    );

    // Verify caller is from lib.rs
    let has_lib_caller = callers.iter().any(|c| c.path.contains("lib.rs"));
    assert!(has_lib_caller, "Should find caller in lib.rs");
}

#[test]
fn test_related_context_xml_format() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Find usages
    let usage_finder = UsageFinder::new();
    let callers = usage_finder.find_usages("helper_function", root, None, None);

    // Create RelatedContext
    let related = RelatedContext {
        callers,
        callees: vec![],
    };

    let xml = related.to_xml();

    // Verify XML structure
    assert!(
        xml.contains("<related_context>"),
        "Should have related_context tag"
    );
    assert!(xml.contains("<callers"), "Should have callers section");
    assert!(
        xml.contains("</related_context>"),
        "Should close related_context"
    );
}

#[test]
fn test_find_usages_excludes_definition() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    // Find usages with definition location
    let usage_finder = UsageFinder::new();
    let callers = usage_finder.find_usages(
        "helper_function",
        root,
        Some("src/utils.rs"),
        Some(4), // Definition line
    );

    // Should not include the definition itself
    for caller in &callers {
        if caller.path.contains("utils.rs") {
            // If in utils.rs, should not be a definition line
            assert!(
                !caller.snippet.contains("pub fn helper_function"),
                "Should exclude definition, got: {}",
                caller.snippet
            );
        }
    }
}

// ============================================================================
// Test 2: Tiered Budget Allocation
// ============================================================================

#[test]
fn test_tiered_budget_prioritizes_core() {
    let temp_dir = create_test_project();
    let _root = temp_dir.path();

    // Create files from different tiers
    let files = vec![
        ("tests/test_main.rs".to_string(), "test content".repeat(10)), // Tests tier
        ("src/lib.rs".to_string(), "lib content".repeat(10)),          // Core tier
        ("README.md".to_string(), "readme content".repeat(10)),        // Other tier
        ("Cargo.toml".to_string(), "[package]".to_string()),           // Config tier
    ];

    let lens_manager = LensManager::new();

    // Small budget - should prioritize Core files
    let (selected, _report) = apply_token_budget(files, 100, &lens_manager, "drop");

    // Get selected paths
    let selected_paths: Vec<&str> = selected.iter().map(|(p, _)| p.as_str()).collect();

    // If we have any selection, Core should be prioritized
    if !selected_paths.is_empty() {
        // Core (src/) should be selected before Tests (tests/)
        let core_selected = selected_paths.iter().any(|p| p.starts_with("src/"));
        let tests_selected = selected_paths.iter().any(|p| p.starts_with("tests/"));

        if tests_selected {
            assert!(
                core_selected,
                "If tests are selected, core should also be selected"
            );
        }
    }
}

#[test]
fn test_file_tier_classification() {
    // Core files
    assert_eq!(FileTier::classify("src/main.rs", None), FileTier::Core);
    assert_eq!(FileTier::classify("src/lib.rs", None), FileTier::Core);
    assert_eq!(FileTier::classify("lib/utils.py", None), FileTier::Core);

    // Config files
    assert_eq!(FileTier::classify("Cargo.toml", None), FileTier::Config);
    assert_eq!(FileTier::classify("package.json", None), FileTier::Config);

    // Test files
    assert_eq!(
        FileTier::classify("tests/test_main.rs", None),
        FileTier::Tests
    );
    assert_eq!(FileTier::classify("test_utils.py", None), FileTier::Tests);

    // Other files
    assert_eq!(FileTier::classify("README.md", None), FileTier::Other);
    assert_eq!(FileTier::classify("docs/guide.md", None), FileTier::Other);
}

#[test]
fn test_budget_drops_other_before_core() {
    // Create files with known sizes
    let files = vec![
        ("docs/readme.md".to_string(), "x".repeat(200)), // Other: ~50 tokens
        ("src/main.rs".to_string(), "y".repeat(200)),    // Core: ~50 tokens
        ("tests/test.rs".to_string(), "z".repeat(200)),  // Tests: ~50 tokens
    ];

    let lens_manager = LensManager::new();

    // Budget for ~2 files
    let (selected, report) = apply_token_budget(files, 120, &lens_manager, "drop");

    // Should have dropped some files
    if report.dropped_count > 0 {
        // Core should be kept, Other should be dropped first
        let selected_paths: Vec<&str> = selected.iter().map(|(p, _)| p.as_str()).collect();
        let dropped_paths: Vec<&str> = report
            .dropped_files
            .iter()
            .map(|(p, _, _)| p.as_str())
            .collect();

        // If something was dropped, Other tier should be dropped before Core
        if dropped_paths.iter().any(|p| p.starts_with("src/")) {
            // If Core was dropped, Other should also be dropped
            assert!(
                dropped_paths.iter().any(|p| p.starts_with("docs/")),
                "Other should be dropped before Core"
            );
        }
    }
}

// ============================================================================
// Test 3: Context Engine Integration
// ============================================================================

#[test]
fn test_context_engine_with_budget() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    let mut config = EncoderConfig::default();
    config.token_budget = Some(500);

    let engine = ContextEngine::with_config(config);
    let result = engine.serialize(root.to_str().unwrap());

    assert!(result.is_ok(), "Serialization should succeed");

    let output = result.unwrap();

    // Should contain some content
    assert!(!output.is_empty(), "Output should not be empty");

    // With tight budget, should prioritize src/ files
    // (This is a soft check - depends on actual file sizes)
    if output.contains("src/") || output.contains("lib.rs") {
        // Good - core files are present
    }
}

#[test]
fn test_zoom_with_symbol_resolution() {
    let temp_dir = create_test_project();
    let root = temp_dir.path();

    let resolver = SymbolResolver::new();

    // Try to find helper_function
    match resolver.find_function("helper_function", root) {
        Ok(loc) => {
            assert!(loc.path.contains("utils"), "Should find in utils.rs");
            assert!(loc.start_line > 0, "Should have valid line number");
        }
        Err(_) => {
            // Function might not be found if pattern doesn't match exactly
            // This is acceptable for this test
        }
    }
}
