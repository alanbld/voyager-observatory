//! Test vectors for Rust/Python parity validation
//!
//! These tests load JSON test vectors that define expected behavior
//! (validated by Python engine) and verify Rust produces identical output.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Test vector structure
#[derive(Debug, Deserialize, Serialize)]
struct TestVector {
    name: String,
    description: String,
    category: String,
    input: TestInput,
    expected: TestExpected,
    python_validated: bool,
    rust_status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestInput {
    #[serde(default)]
    files: HashMap<String, String>,
    #[serde(default)]
    config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    cli_args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestExpected {
    output_format: String,
    #[serde(default)]
    files_included: Vec<String>,
    #[serde(default)]
    files_excluded: Vec<String>,
    #[serde(default)]
    output_contains: Vec<String>,
    #[serde(default)]
    output_hash: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, serde_json::Value>,
}

/// Load a test vector from JSON file
fn load_vector(name: &str) -> TestVector {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go up to repo root
    path.push("test_vectors");
    path.push("rust_parity");
    path.push(format!("{}.json", name));

    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load test vector {}: {}", name, e));

    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse test vector {}: {}", name, e))
}

// ============================================================================
// Config System Tests (5 vectors)
// ============================================================================

#[test]
fn test_config_01_file_loading() {
    let vector = load_vector("config_01_file_loading");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with test files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Check that expected files are included
    for file in &vector.expected.files_included {
        assert!(
            output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should contain file: {}",
            file
        );
    }

    // Check that expected files are excluded
    for file in &vector.expected.files_excluded {
        assert!(
            !output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should NOT contain file: {}",
            file
        );
    }

    // Check for specific content strings
    for content_str in &vector.expected.output_contains {
        assert!(
            output.contains(content_str),
            "Output should contain: {}",
            content_str
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore] // Requires CLI parsing
fn test_config_02_cli_override() {
    let vector = load_vector("config_02_cli_override");
    assert!(vector.python_validated);
    // TODO: Implement CLI argument parsing
    // This test requires --include CLI flag support
    panic!("Not yet implemented - requires CLI parsing");
}

#[test]
fn test_config_03_ignore_patterns() {
    let vector = load_vector("config_03_ignore_patterns");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with test files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Check that expected files are included
    for file in &vector.expected.files_included {
        assert!(
            output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should contain file: {}",
            file
        );
    }

    // Check that expected files are excluded
    for file in &vector.expected.files_excluded {
        assert!(
            !output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should NOT contain file: {}",
            file
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_config_04_include_patterns() {
    let vector = load_vector("config_04_include_patterns");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with test files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Check that expected files are included
    for file in &vector.expected.files_included {
        assert!(
            output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should contain file: {}",
            file
        );
    }

    // Check that expected files are excluded
    for file in &vector.expected.files_excluded {
        assert!(
            !output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should NOT contain file: {}",
            file
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_config_05_pattern_precedence() {
    let vector = load_vector("config_05_pattern_precedence");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with test files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Check that expected files are included
    for file in &vector.expected.files_included {
        assert!(
            output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should contain file: {}",
            file
        );
    }

    // Check that expected files are excluded
    for file in &vector.expected.files_excluded {
        assert!(
            !output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should NOT contain file: {}",
            file
        );
    }

    // Check for specific content strings
    for content_str in &vector.expected.output_contains {
        assert!(
            output.contains(content_str),
            "Output should contain: {}",
            content_str
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

// ============================================================================
// Serialization Tests (5 vectors)
// ============================================================================

#[test]
fn test_serial_01_basic_sorting() {
    let vector = load_vector("serial_01_basic_sorting");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with test files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Verify files appear in correct order
    let file_positions: Vec<_> = vector.expected.files_included.iter()
        .map(|file| {
            let header = format!("++++++++++ {} ++++++++++", file);
            output.find(&header).expect(&format!("File {} not found in output", file))
        })
        .collect();

    // Check that positions are in ascending order (alphabetical)
    for i in 1..file_positions.len() {
        assert!(
            file_positions[i] > file_positions[i - 1],
            "Files not in alphabetical order: {} should come before {}",
            vector.expected.files_included[i - 1],
            vector.expected.files_included[i]
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_serial_02_empty_directory() {
    let vector = load_vector("serial_02_empty_directory");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with no files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Output should be empty
    assert_eq!(output, "", "Empty directory should produce empty output");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_serial_03_single_file() {
    let vector = load_vector("serial_03_single_file");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with single file
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test file
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Check that expected content strings are present
    for content_str in &vector.expected.output_contains {
        assert!(
            output.contains(content_str),
            "Output should contain: {}",
            content_str
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_serial_04_nested_structure() {
    let vector = load_vector("serial_04_nested_structure");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with nested files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Verify all files are included
    for file in &vector.expected.files_included {
        assert!(
            output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should contain file: {}",
            file
        );
    }

    // Verify sort order
    let file_positions: Vec<_> = vector.expected.files_included.iter()
        .map(|file| {
            let header = format!("++++++++++ {} ++++++++++", file);
            output.find(&header).expect(&format!("File {} not found in output", file))
        })
        .collect();

    // Check that positions are in ascending order
    for i in 1..file_positions.len() {
        assert!(
            file_positions[i] > file_positions[i - 1],
            "Files not in alphabetical order"
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_serial_05_newline_handling() {
    let vector = load_vector("serial_05_newline_handling");
    assert!(vector.python_validated, "Vector not validated by Python");

    // Create temp directory with test files
    let temp_dir = std::env::temp_dir().join(format!("pm_encoder_test_{}", vector.name));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write test files
    for (file_path, content) in &vector.input.files {
        let full_path = temp_dir.join(file_path);
        fs::write(&full_path, content).expect("Failed to write test file");
    }

    // Run serialization
    let output = pm_encoder::serialize_project(temp_dir.to_str().unwrap())
        .expect("Serialization failed");

    // Verify all files are included
    for file in &vector.expected.files_included {
        assert!(
            output.contains(&format!("++++++++++ {} ++++++++++", file)),
            "Output should contain file: {}",
            file
        );
    }

    // Verify content strings are present
    for content_str in &vector.expected.output_contains {
        assert!(
            output.contains(content_str),
            "Output should contain: {}",
            content_str
        );
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

// ============================================================================
// Infrastructure Tests
// ============================================================================

// Test that we can load the schema itself
#[test]
fn test_vector_loading_works() {
    // This test passes once we create the first vector
    // For now, just verify the infrastructure exists
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vectors_dir = manifest_dir.parent().unwrap().join("test_vectors").join("rust_parity");
    assert!(vectors_dir.exists(), "Test vectors directory should exist");
}
