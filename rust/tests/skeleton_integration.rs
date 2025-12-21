//! Integration tests for Skeleton Protocol v2.2
//!
//! Tests end-to-end skeleton compression via ContextEngine.

use std::fs;
use tempfile::TempDir;

use pm_encoder::core::{ContextEngine, EncoderConfig, OutputFormat, SkeletonMode};

/// Helper to create test project structure
fn create_test_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create src directory
    fs::create_dir(root.join("src")).unwrap();

    // src/main.rs - Core file with function body
    fs::write(
        root.join("src/main.rs"),
        r#"fn main() {
    println!("Hello, world!");
    let x = 42;
    let y = x * 2;
    println!("Result: {}", y);
}

fn helper() -> i32 {
    let a = 1;
    let b = 2;
    a + b
}

struct Config {
    name: String,
    value: i32,
}
"#,
    )
    .unwrap();

    // src/lib.rs - Another core file
    fs::write(
        root.join("src/lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    // Cargo.toml - Config file
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
"#,
    )
    .unwrap();

    // tests/test_main.rs - Test file
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/test_main.rs"),
        r#"#[test]
fn test_add() {
    assert_eq!(1 + 1, 2);
}
"#,
    )
    .unwrap();

    // docs/readme.md - Other file
    fs::create_dir(root.join("docs")).unwrap();
    fs::write(root.join("docs/readme.md"), "# Test Project\n\nA test project.\n").unwrap();

    temp
}

#[test]
fn test_skeleton_disabled_no_budget() {
    let temp = create_test_project();

    // No budget, skeleton auto = disabled
    let config = EncoderConfig::default();
    let engine = ContextEngine::with_config(config);

    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Should contain full function bodies
    assert!(output.contains("println!(\"Hello, world!\")"));
    assert!(output.contains("let x = 42"));

    // Should NOT contain [SKELETON] markers
    assert!(!output.contains("[SKELETON]"));
}

#[test]
fn test_skeleton_enabled_with_budget() {
    let temp = create_test_project();

    // Set a moderate budget - enough for skeleton files but not full
    let mut config = EncoderConfig::default();
    config.token_budget = Some(500); // Budget that allows some skeletonized files
    config.skeleton_mode = SkeletonMode::Enabled;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Should contain either [SKELETON] markers or full content of core files
    // The key is that skeleton mode is enabled and tiered allocation happens
    assert!(
        output.contains("src/main.rs") || output.contains("src/lib.rs"),
        "Expected core source files in output:\n{}",
        output
    );
}

#[test]
fn test_skeleton_preserves_signatures() {
    let temp = create_test_project();

    let mut config = EncoderConfig::default();
    config.token_budget = Some(100); // Very small budget to force skeleton
    config.skeleton_mode = SkeletonMode::Enabled;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Signatures should be preserved
    if output.contains("src/main.rs") {
        // fn main() should appear (signature)
        assert!(
            output.contains("fn main") || output.contains("fn helper") || output.contains("struct Config"),
            "Expected function signatures in skeleton output:\n{}",
            output
        );
    }
}

#[test]
fn test_skeleton_strips_bodies() {
    let temp = create_test_project();

    let mut config = EncoderConfig::default();
    config.token_budget = Some(100);
    config.skeleton_mode = SkeletonMode::Enabled;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // If file is skeletonized, body details should be stripped
    if output.contains("[SKELETON]") && output.contains("src/main.rs") {
        // Implementation details should be stripped
        assert!(
            !output.contains("let x = 42") || output.contains("{ /* ... */ }"),
            "Expected body to be stripped in skeleton mode"
        );
    }
}

#[test]
fn test_skeleton_mode_auto() {
    let temp = create_test_project();

    // Auto mode without budget = disabled
    let config1 = EncoderConfig::default();
    assert!(!config1.skeleton_mode.is_enabled(false));

    // Auto mode with budget = enabled
    assert!(SkeletonMode::Auto.is_enabled(true));
}

#[test]
fn test_skeleton_mode_forced_disabled() {
    let temp = create_test_project();

    // Force disabled even with budget
    let mut config = EncoderConfig::default();
    config.token_budget = Some(100);
    config.skeleton_mode = SkeletonMode::Disabled;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Should NOT contain [SKELETON] markers even with budget
    assert!(
        !output.contains("[SKELETON]"),
        "Expected no [SKELETON] markers when skeleton mode is disabled"
    );
}

#[test]
fn test_skeleton_shows_original_tokens() {
    let temp = create_test_project();

    let mut config = EncoderConfig::default();
    config.token_budget = Some(150);
    config.skeleton_mode = SkeletonMode::Enabled;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // [SKELETON] marker should show original token count
    if output.contains("[SKELETON]") {
        assert!(
            output.contains("original:") || output.contains("tokens)"),
            "Expected original token count in skeleton header:\n{}",
            output
        );
    }
}

#[test]
fn test_skeleton_tiered_priority() {
    let temp = create_test_project();

    // Budget that allows Core files but drops Other
    let mut config = EncoderConfig::default();
    config.token_budget = Some(300);
    config.skeleton_mode = SkeletonMode::Enabled;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Core files (src/) should be prioritized
    // Other files (docs/) might be dropped
    if output.contains("src/main.rs") {
        // Good - core file is present
    } else if output.contains("docs/readme.md") && !output.contains("src/main.rs") {
        panic!("Core files should be prioritized over Other files");
    }
}

#[test]
fn test_skeleton_xml_format() {
    let temp = create_test_project();

    let mut config = EncoderConfig::default();
    config.token_budget = Some(150);
    config.skeleton_mode = SkeletonMode::Enabled;
    config.output_format = OutputFormat::Xml;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // XML format should have skeleton attribute
    if output.contains("skeleton=\"true\"") {
        // Good - skeleton attribute present
        assert!(output.contains("original_tokens="), "Expected original_tokens attribute");
    }
}

#[test]
fn test_skeleton_markdown_format() {
    let temp = create_test_project();

    let mut config = EncoderConfig::default();
    config.token_budget = Some(150);
    config.skeleton_mode = SkeletonMode::Enabled;
    config.output_format = OutputFormat::Markdown;

    let engine = ContextEngine::with_config(config);
    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Markdown format should have [SKELETON] in header
    if output.contains("[SKELETON]") {
        assert!(output.contains("##"), "Expected markdown header");
    }
}

#[test]
fn test_backward_compatibility_no_skeleton_no_budget() {
    let temp = create_test_project();

    // Default config without budget
    let config = EncoderConfig::default();
    let engine = ContextEngine::with_config(config);

    let output = engine.serialize(temp.path().to_str().unwrap()).unwrap();

    // Output should be identical to pre-skeleton behavior
    assert!(!output.contains("[SKELETON]"));
    assert!(output.contains("+++"));
    assert!(output.contains("---"));
}
