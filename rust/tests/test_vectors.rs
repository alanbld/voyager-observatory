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
// Analyzer Tests (10 vectors) - Phase 2
// ============================================================================

#[test]
fn test_analyzer_01_python_class() {
    let vector = load_vector("analyzer_01_python_class");
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
fn test_analyzer_02_python_function() {
    let vector = load_vector("analyzer_02_python_function");
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
fn test_analyzer_03_python_imports() {
    let vector = load_vector("analyzer_03_python_imports");
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
fn test_analyzer_04_javascript_function() {
    let vector = load_vector("analyzer_04_javascript_function");
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
fn test_analyzer_05_javascript_imports() {
    let vector = load_vector("analyzer_05_javascript_imports");
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
fn test_analyzer_06_rust_struct() {
    let vector = load_vector("analyzer_06_rust_struct");
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
fn test_analyzer_07_rust_function() {
    let vector = load_vector("analyzer_07_rust_function");
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
fn test_analyzer_08_shell_functions() {
    let vector = load_vector("analyzer_08_shell_functions");
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
fn test_analyzer_09_mixed_project() {
    let vector = load_vector("analyzer_09_mixed_project");
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
fn test_analyzer_10_structure_preservation() {
    let vector = load_vector("analyzer_10_structure_preservation");
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
// CLI Tests (4 vectors) - Interface Parity
// ============================================================================

use std::process::Command;

/// CLI test vector structure (different from serialization vectors)
#[derive(Debug, Deserialize)]
struct CliTestVector {
    name: String,
    description: String,
    category: String,
    input: CliTestInput,
    expected: CliTestExpected,
    validation_mode: String,
    #[serde(default)]
    notes: String,
    python_validated: bool,
    rust_status: String,
}

#[derive(Debug, Deserialize)]
struct CliTestInput {
    cli_args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CliTestExpected {
    #[serde(default)]
    exit_code: Option<i32>,
    #[serde(default)]
    exit_code_nonzero: Option<bool>,
    #[serde(default)]
    stdout_contains: Vec<String>,
    #[serde(default)]
    stdout_contains_any: Vec<String>,
    #[serde(default)]
    stdout_regex: Option<String>,
    #[serde(default)]
    stderr: Option<String>,
    #[serde(default)]
    stderr_contains: Vec<String>,
    #[serde(default)]
    stderr_contains_any: Vec<String>,
    #[serde(default)]
    reference_output: Option<String>,
    #[serde(default)]
    reference_stderr: Option<String>,
}

/// Load a CLI test vector from JSON file
fn load_cli_vector(name: &str) -> CliTestVector {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go up to repo root
    path.push("test_vectors");
    path.push("rust_parity");
    path.push(format!("{}.json", name));

    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load CLI test vector {}: {}", name, e));

    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse CLI test vector {}: {}", name, e))
}

/// Run the pm_encoder binary with given arguments
fn run_cli(args: &[String]) -> std::process::Output {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("pm_encoder");

    Command::new(&path)
        .args(args)
        .output()
        .expect("Failed to execute pm_encoder binary")
}

#[test]
fn test_cli_01_help() {
    let vector = load_cli_vector("cli_01_help");
    assert!(vector.python_validated, "Vector not validated by Python");

    let output = run_cli(&vector.input.cli_args);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check exit code
    if let Some(expected_code) = vector.expected.exit_code {
        assert_eq!(
            output.status.code().unwrap_or(-1),
            expected_code,
            "Exit code mismatch"
        );
    }

    // Check that required flags are present (semantic validation)
    for flag in &vector.expected.stdout_contains {
        assert!(
            stdout.contains(flag),
            "Help output should contain flag: '{}'",
            flag
        );
    }

    // Check that at least one description is present
    if !vector.expected.stdout_contains_any.is_empty() {
        let has_any = vector.expected.stdout_contains_any.iter()
            .any(|desc| stdout.to_lowercase().contains(&desc.to_lowercase()));
        assert!(
            has_any,
            "Help output should contain at least one of: {:?}",
            vector.expected.stdout_contains_any
        );
    }
}

#[test]
fn test_cli_02_version() {
    let vector = load_cli_vector("cli_02_version");
    assert!(vector.python_validated, "Vector not validated by Python");

    let output = run_cli(&vector.input.cli_args);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check exit code
    if let Some(expected_code) = vector.expected.exit_code {
        assert_eq!(
            output.status.code().unwrap_or(-1),
            expected_code,
            "Exit code mismatch"
        );
    }

    // Check version format using regex
    if let Some(regex_pattern) = &vector.expected.stdout_regex {
        let re = regex::Regex::new(regex_pattern).expect("Invalid regex in test vector");
        assert!(
            re.is_match(&stdout),
            "Version output '{}' should match pattern '{}'",
            stdout.trim(),
            regex_pattern
        );
    }
}

#[test]
fn test_cli_03_invalid_arg() {
    let vector = load_cli_vector("cli_03_invalid_arg");
    assert!(vector.python_validated, "Vector not validated by Python");

    let output = run_cli(&vector.input.cli_args);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for non-zero exit code
    if vector.expected.exit_code_nonzero == Some(true) {
        assert!(
            !output.status.success(),
            "Command should fail with non-zero exit code"
        );
    }

    // Check that error message contains expected terms
    if !vector.expected.stderr_contains_any.is_empty() {
        let has_any = vector.expected.stderr_contains_any.iter()
            .any(|term| stderr.to_lowercase().contains(&term.to_lowercase()));
        assert!(
            has_any,
            "Error output '{}' should contain at least one of: {:?}",
            stderr,
            vector.expected.stderr_contains_any
        );
    }
}

#[test]
fn test_cli_04_missing_dir() {
    let vector = load_cli_vector("cli_04_missing_dir");
    assert!(vector.python_validated, "Vector not validated by Python");

    let output = run_cli(&vector.input.cli_args);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for non-zero exit code
    if vector.expected.exit_code_nonzero == Some(true) {
        assert!(
            !output.status.success(),
            "Command should fail with non-zero exit code for missing directory"
        );
    }

    // Check that error message indicates the problem
    if !vector.expected.stderr_contains_any.is_empty() {
        let has_any = vector.expected.stderr_contains_any.iter()
            .any(|term| stderr.to_lowercase().contains(&term.to_lowercase()));
        assert!(
            has_any,
            "Error output '{}' should indicate missing directory",
            stderr
        );
    }
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
