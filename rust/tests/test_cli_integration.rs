//! CLI Integration Tests for pm_encoder
//!
//! These tests execute the binary and verify correct behavior for:
//! - Output formats (claude-xml, plus-minus, etc.)
//! - Frozen mode (deterministic output)
//! - Zoom functionality
//! - Error handling

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test directory with sample files
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a Python file
    fs::write(
        temp_dir.path().join("main.py"),
        r#"#!/usr/bin/env python3
"""Main module for the application."""

import os
import sys

def main():
    """Entry point for the application."""
    print("Hello, World!")
    return 0

class Calculator:
    """A simple calculator class."""

    def add(self, a: int, b: int) -> int:
        """Add two numbers."""
        return a + b

    def subtract(self, a: int, b: int) -> int:
        """Subtract b from a."""
        return a - b

if __name__ == "__main__":
    sys.exit(main())
"#,
    )
    .unwrap();

    // Create a Rust file
    fs::write(
        temp_dir.path().join("lib.rs"),
        r#"//! Library crate

/// Add two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Subtract two numbers
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#,
    )
    .unwrap();

    // Create a config file
    fs::write(
        temp_dir.path().join("config.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .unwrap();

    temp_dir
}

// ============================================================================
// Format Tests
// ============================================================================

#[test]
fn test_format_claude_xml_produces_valid_xml() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--format")
        .arg("claude-xml");

    cmd.assert()
        .success()
        .stdout(predicate::str::starts_with("<context"))
        .stdout(predicate::str::contains("</context>"))
        .stdout(predicate::str::contains("<files>"))
        .stdout(predicate::str::contains("</files>"))
        .stdout(predicate::str::contains("<metadata>"))
        .stdout(predicate::str::contains("<![CDATA["));
}

#[test]
fn test_format_plus_minus_default() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("++++++++++"))
        .stdout(predicate::str::contains("----------"));
}

#[test]
fn test_format_markdown() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--format")
        .arg("markdown");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("## "))
        .stdout(predicate::str::contains("```python"))
        .stdout(predicate::str::contains("```rust"));
}

#[test]
fn test_format_xml() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--format")
        .arg("xml");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<file path="))
        .stdout(predicate::str::contains("</file>"))
        .stdout(predicate::str::contains("md5="));
}

// ============================================================================
// Frozen Mode Tests
// ============================================================================

#[test]
fn test_frozen_mode_deterministic_output() {
    let temp_dir = create_test_project();

    // First run
    let mut cmd1 = Command::cargo_bin("pm_encoder").unwrap();
    let output1 = cmd1
        .arg(temp_dir.path())
        .arg("--frozen")
        .arg("--format")
        .arg("claude-xml")
        .output()
        .unwrap();

    // Second run (should be identical)
    let mut cmd2 = Command::cargo_bin("pm_encoder").unwrap();
    let output2 = cmd2
        .arg(temp_dir.path())
        .arg("--frozen")
        .arg("--format")
        .arg("claude-xml")
        .output()
        .unwrap();

    assert!(output1.status.success());
    assert!(output2.status.success());

    // Extract outputs
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    // In frozen mode, timestamps should be replaced with snapshot IDs
    // The outputs should be byte-identical
    assert_eq!(stdout1, stdout2, "Frozen mode should produce identical output");
}

#[test]
fn test_frozen_mode_no_timestamp_variation() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--frozen")
        .arg("--format")
        .arg("claude-xml");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("<frozen>true</frozen>"))
        .stdout(predicate::str::contains("FROZEN_SNAPSHOT"));
}

// ============================================================================
// Zoom Tests
// ============================================================================

#[test]
fn test_zoom_file_basic() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("file=main.py");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.py"))
        .stdout(predicate::str::contains("def main()"));
}

#[test]
fn test_zoom_file_with_line_range() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("file=main.py:1-10");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.py"))
        .stdout(predicate::str::contains("Main module"))
        // Line 1-10 should include imports but not the Calculator class (which starts around line 15)
        .stdout(predicate::str::contains("import"));
}

#[test]
fn test_zoom_function() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("fn=main");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("def main()"));
}

#[test]
fn test_zoom_class() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("class=Calculator");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("class Calculator"));
}

// ============================================================================
// Zoom Error Handling Tests
// ============================================================================

#[test]
fn test_zoom_invalid_target_type() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("invalid=something");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Unknown zoom type"))
        .stderr(predicate::str::contains("fn, class, mod, file"));
}

#[test]
fn test_zoom_malformed_target() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("notavalidformat");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid zoom format"))
        .stderr(predicate::str::contains("Expected <TYPE>=<TARGET>"));
}

#[test]
fn test_zoom_nonexistent_target() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("fn=nonexistent_function_xyz");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Zoom error"))
        .stderr(predicate::str::contains("Invalid zoom target"));
}

#[test]
fn test_zoom_invalid_line_range() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--zoom")
        .arg("file=main.py:abc-def");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid"));
}

// ============================================================================
// Truncation Tests
// ============================================================================

#[test]
fn test_truncation_with_zoom_affordance() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--truncate")
        .arg("5");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TRUNCATED"))
        .stdout(predicate::str::contains("ZOOM_AFFORDANCE"));
}

#[test]
fn test_truncation_no_summary() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--truncate")
        .arg("5")
        .arg("--no-truncate-summary");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    // Files should still be present
    assert!(stdout.contains("main.py") || stdout.contains("lib.rs"));
    // With --no-truncate-summary, we should have fewer "TRUNCATED" markers or
    // reduced detail. The [TRUNCATED: X lines] header should still appear
    // but the detailed block may be suppressed.
}

// ============================================================================
// Help and Version Tests
// ============================================================================

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--zoom"))
        .stdout(predicate::str::contains("--frozen"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--truncate"))
        .stdout(predicate::str::contains("--lens"));
}

#[test]
fn test_version_output() {
    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("1.0.0"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_missing_project_root() {
    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    // No arguments

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("PROJECT_ROOT"));
}

#[test]
fn test_nonexistent_directory() {
    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg("/nonexistent/path/that/does/not/exist");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_invalid_format() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--format")
        .arg("invalid_format");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

// ============================================================================
// Token Budget Tests
// ============================================================================

#[test]
fn test_token_budget_basic() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--token-budget")
        .arg("1000");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Budget"));
}

#[test]
fn test_token_budget_shorthand() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--token-budget")
        .arg("1k");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Budget"));
}

// ============================================================================
// Lens Tests
// ============================================================================

#[test]
fn test_lens_architecture() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--lens")
        .arg("architecture")
        .arg("--token-budget")
        .arg("10000");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("LENS: architecture"));
}

// ============================================================================
// Include/Exclude Pattern Tests
// ============================================================================

#[test]
fn test_include_pattern() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--include")
        .arg("*.py");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("main.py"), "Should include Python files");
    // With --include *.py, only .py files should be in output
    assert!(!stdout.contains("++++++++++  lib.rs"), "Should not include lib.rs when filtering for *.py");
}

#[test]
fn test_exclude_pattern() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--exclude")
        .arg("*.json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("config.json").not());
}

// ============================================================================
// Sorting Tests
// ============================================================================

#[test]
fn test_sort_by_name_asc() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--sort-by")
        .arg("name")
        .arg("--sort-order")
        .arg("asc");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // config.json should come before lib.rs which comes before main.py
    let config_pos = stdout.find("config.json");
    let lib_pos = stdout.find("lib.rs");
    let main_pos = stdout.find("main.py");

    assert!(config_pos.is_some());
    assert!(lib_pos.is_some());
    assert!(main_pos.is_some());
    assert!(config_pos < lib_pos);
    assert!(lib_pos < main_pos);
}

#[test]
fn test_sort_by_name_desc() {
    let temp_dir = create_test_project();

    let mut cmd = Command::cargo_bin("pm_encoder").unwrap();
    cmd.arg(temp_dir.path())
        .arg("--sort-by")
        .arg("name")
        .arg("--sort-order")
        .arg("desc");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // main.py should come before lib.rs which comes before config.json
    let config_pos = stdout.find("config.json");
    let lib_pos = stdout.find("lib.rs");
    let main_pos = stdout.find("main.py");

    assert!(config_pos.is_some());
    assert!(lib_pos.is_some());
    assert!(main_pos.is_some());
    assert!(main_pos < lib_pos);
    assert!(lib_pos < config_pos);
}
