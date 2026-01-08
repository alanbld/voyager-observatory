//! TDD Test Suite for Skeleton Protocol v2.2
//!
//! These tests define the expected behavior of the Skeletonizer and AdaptiveAllocator
//! BEFORE the implementation exists. They should fail to compile initially (red phase).

use pm_encoder::core::skeleton::{
    AdaptiveAllocator, CompressionLevel, FileAllocation, Language, SkeletonResult, Skeletonizer,
};
use pm_encoder::core::FileTier;

// ============================================================================
// Section A: Rust Regex Parsing Tests
// ============================================================================

#[test]
fn test_skeletonize_rust_function() {
    let input = r#"
/// Process data with validation
pub fn process_data(input: &[u8], config: &Config) -> Result<Output, Error> {
    let validated = validate(input)?;
    let parsed = parse(&validated)?;

    for item in parsed.items {
        if config.should_include(&item) {
            result.add(item);
        }
    }

    Ok(result)
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Should contain signature
    assert!(
        result.content.contains("pub fn process_data"),
        "Should preserve function signature"
    );
    assert!(
        result.content.contains("Result<Output, Error>"),
        "Should preserve return type"
    );

    // Should NOT contain body implementation
    assert!(
        !result.content.contains("validate(input)"),
        "Should strip function body"
    );
    assert!(
        !result.content.contains("for item in"),
        "Should strip loop body"
    );

    // Should achieve compression
    assert!(
        result.compression_ratio > 0.5,
        "Expected >50% compression, got {}%",
        result.compression_ratio * 100.0
    );
}

#[test]
fn test_skeletonize_rust_struct_and_impl() {
    let input = r#"
/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub debug: bool,
}

impl Config {
    /// Create from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Ok(Self { host, port, debug: false })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::InvalidPort);
        }
        Ok(())
    }
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Struct should be fully preserved (it's a signature)
    assert!(result.content.contains("pub struct Config"));
    assert!(result.content.contains("pub host: String"));
    assert!(result.content.contains("pub port: u16"));

    // Impl block should show method signatures
    assert!(result.content.contains("impl Config"));
    assert!(result.content.contains("pub fn from_env()"));
    assert!(result.content.contains("pub fn validate(&self)"));

    // Bodies should be stripped
    assert!(
        !result.content.contains("std::env::var"),
        "Should strip method bodies"
    );
    assert!(
        !result.content.contains("InvalidPort"),
        "Should strip error handling"
    );
}

#[test]
fn test_skeletonize_rust_nested_braces() {
    let input = r#"
fn complex_function() {
    if condition {
        match value {
            Some(x) => {
                for i in 0..x {
                    if i > 5 {
                        break;
                    }
                }
            }
            None => {}
        }
    }
}

fn simple_function() -> i32 {
    42
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Both function signatures should be present
    assert!(result.content.contains("fn complex_function()"));
    assert!(result.content.contains("fn simple_function() -> i32"));

    // Nested content should be stripped
    assert!(!result.content.contains("match value"));
    assert!(!result.content.contains("break"));
}

#[test]
fn test_skeletonize_rust_preserves_imports() {
    let input = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
use crate::config::Config;

mod submodule;

pub fn main() {
    let map = HashMap::new();
    println!("Hello");
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Imports should be preserved
    assert!(result.content.contains("use std::collections::HashMap"));
    assert!(result.content.contains("use std::io::{Read, Write}"));
    assert!(result.content.contains("use crate::config::Config"));
    assert!(result.content.contains("mod submodule"));

    // Function body should be stripped
    assert!(!result.content.contains("HashMap::new()"));
    assert!(!result.content.contains("println!"));
}

#[test]
fn test_skeletonize_rust_preserves_constants() {
    let input = r#"
pub const MAX_RETRIES: usize = 5;
pub const DEFAULT_TIMEOUT: u64 = 30_000;

static GLOBAL_STATE: AtomicUsize = AtomicUsize::new(0);

pub fn retry() {
    for _ in 0..MAX_RETRIES {
        // retry logic
    }
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Constants should be preserved
    assert!(result.content.contains("pub const MAX_RETRIES: usize = 5"));
    assert!(result
        .content
        .contains("pub const DEFAULT_TIMEOUT: u64 = 30_000"));
    assert!(result.content.contains("static GLOBAL_STATE"));

    // Function body should be stripped
    assert!(!result.content.contains("retry logic"));
}

// ============================================================================
// Section B: Python Regex Parsing Tests
// ============================================================================

#[test]
fn test_skeletonize_python_class() {
    let input = r#"
class DataProcessor:
    """Processes data files with configurable transformations."""

    def __init__(self, config: Config):
        """Initialize with configuration."""
        self.config = config
        self.cache = {}
        self._setup_handlers()

    def process(self, data: bytes) -> ProcessedData:
        """Process raw bytes into structured data."""
        validated = self._validate(data)
        parsed = self._parse(validated)
        return self._transform(parsed)

    def _validate(self, data: bytes) -> bytes:
        if not data:
            raise ValueError("Empty data")
        return data
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Python);

    // Class and method signatures should be preserved
    assert!(result.content.contains("class DataProcessor:"));
    assert!(result
        .content
        .contains("def __init__(self, config: Config):"));
    assert!(result
        .content
        .contains("def process(self, data: bytes) -> ProcessedData:"));
    assert!(result
        .content
        .contains("def _validate(self, data: bytes) -> bytes:"));

    // Docstrings should be preserved (L1 behavior)
    assert!(result.content.contains("Processes data files"));

    // Body content should be stripped
    assert!(!result.content.contains("self.cache = {}"));
    assert!(!result.content.contains("self._setup_handlers()"));
    assert!(!result.content.contains("raise ValueError"));
}

#[test]
fn test_skeletonize_python_functions() {
    let input = r#"
import os
from pathlib import Path
from typing import Optional, List

def load_config(path: str) -> dict:
    """Load configuration from file."""
    with open(path) as f:
        return json.load(f)

async def fetch_data(url: str) -> bytes:
    """Fetch data from URL."""
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.read()

def helper():
    pass
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Python);

    // Imports should be preserved
    assert!(result.content.contains("import os"));
    assert!(result.content.contains("from pathlib import Path"));
    assert!(result.content.contains("from typing import Optional, List"));

    // Function signatures should be preserved
    assert!(result
        .content
        .contains("def load_config(path: str) -> dict:"));
    assert!(result
        .content
        .contains("async def fetch_data(url: str) -> bytes:"));
    assert!(result.content.contains("def helper():"));

    // Bodies should be stripped
    assert!(!result.content.contains("json.load(f)"));
    assert!(!result.content.contains("aiohttp.ClientSession"));
}

#[test]
fn test_skeletonize_python_nested_class() {
    let input = r#"
class Outer:
    """Outer class."""

    class Inner:
        """Inner class."""

        def inner_method(self):
            for i in range(10):
                print(i)

    def outer_method(self):
        inner = self.Inner()
        inner.inner_method()
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Python);

    // Both classes should be preserved
    assert!(result.content.contains("class Outer:"));
    assert!(result.content.contains("class Inner:"));

    // Method signatures should be preserved
    assert!(result.content.contains("def inner_method(self):"));
    assert!(result.content.contains("def outer_method(self):"));

    // Bodies should be stripped
    assert!(!result.content.contains("range(10)"));
    assert!(!result.content.contains("print(i)"));
}

// ============================================================================
// Section C: Allocator Logic Tests
// ============================================================================

#[test]
fn test_allocator_upgrades_core_first() {
    // Setup: 3 files, 100 tokens each full, 10 tokens skeleton
    let files = vec![
        FileAllocation::new("src/core.rs", FileTier::Core, 100, 10),
        FileAllocation::new("config.toml", FileTier::Config, 100, 10),
        FileAllocation::new("tests/test.rs", FileTier::Tests, 100, 10),
    ];

    // Budget = 150 tokens
    let allocator = AdaptiveAllocator::new(150);
    let result = allocator.allocate(files);

    // Pass 1: All skeleton = 30 tokens. Remaining = 120.
    // Pass 2: Upgrade Core (10->100). Total = 120. Remaining = 30.
    // Pass 3: Cannot upgrade Config (would need 90 more). Stay skeleton.

    let core = result.iter().find(|f| f.path == "src/core.rs").unwrap();
    let config = result.iter().find(|f| f.path == "config.toml").unwrap();
    let tests = result.iter().find(|f| f.path == "tests/test.rs").unwrap();

    assert_eq!(
        core.level,
        CompressionLevel::Full,
        "Core should be upgraded to Full"
    );
    assert_eq!(
        config.level,
        CompressionLevel::Skeleton,
        "Config should stay Skeleton"
    );
    assert_eq!(
        tests.level,
        CompressionLevel::Skeleton,
        "Tests should stay Skeleton"
    );
}

#[test]
fn test_allocator_upgrades_config_after_core() {
    // Budget = 250 tokens (enough for Core full + Config full + Tests skeleton)
    let files = vec![
        FileAllocation::new("src/core.rs", FileTier::Core, 100, 10),
        FileAllocation::new("config.toml", FileTier::Config, 100, 10),
        FileAllocation::new("tests/test.rs", FileTier::Tests, 100, 10),
    ];

    let allocator = AdaptiveAllocator::new(250);
    let result = allocator.allocate(files);

    // Pass 1: All skeleton = 30 tokens. Remaining = 220.
    // Pass 2: Upgrade Core. Total = 120. Remaining = 130.
    // Pass 3: Upgrade Config. Total = 210. Remaining = 40.
    // Cannot upgrade Tests (would need 90 more).

    let core = result.iter().find(|f| f.path == "src/core.rs").unwrap();
    let config = result.iter().find(|f| f.path == "config.toml").unwrap();
    let tests = result.iter().find(|f| f.path == "tests/test.rs").unwrap();

    assert_eq!(core.level, CompressionLevel::Full);
    assert_eq!(config.level, CompressionLevel::Full);
    assert_eq!(tests.level, CompressionLevel::Skeleton);
}

#[test]
fn test_allocator_drops_other_tier_first() {
    // Budget = 20 tokens (only room for 2 skeletons)
    let files = vec![
        FileAllocation::new("src/core.rs", FileTier::Core, 100, 10),
        FileAllocation::new("docs/readme.md", FileTier::Other, 100, 10),
        FileAllocation::new("tests/test.rs", FileTier::Tests, 100, 10),
    ];

    let allocator = AdaptiveAllocator::new(20);
    let result = allocator.allocate(files);

    // Pass 1: All skeleton = 30 tokens. Exceeds budget.
    // Fallback: Drop Other tier first. Now = 20 tokens. Fits!

    let core = result.iter().find(|f| f.path == "src/core.rs").unwrap();
    let docs = result.iter().find(|f| f.path == "docs/readme.md").unwrap();
    let tests = result.iter().find(|f| f.path == "tests/test.rs").unwrap();

    assert_eq!(
        core.level,
        CompressionLevel::Skeleton,
        "Core should be Skeleton"
    );
    assert_eq!(docs.level, CompressionLevel::Drop, "Docs should be Dropped");
    assert_eq!(
        tests.level,
        CompressionLevel::Skeleton,
        "Tests should be Skeleton"
    );
}

#[test]
fn test_allocator_drops_tests_before_core() {
    // Budget = 10 tokens (only room for 1 skeleton)
    let files = vec![
        FileAllocation::new("src/core.rs", FileTier::Core, 100, 10),
        FileAllocation::new("tests/test.rs", FileTier::Tests, 100, 10),
    ];

    let allocator = AdaptiveAllocator::new(10);
    let result = allocator.allocate(files);

    // Pass 1: All skeleton = 20 tokens. Exceeds budget.
    // Fallback: Drop Tests tier. Now = 10 tokens. Fits!

    let core = result.iter().find(|f| f.path == "src/core.rs").unwrap();
    let tests = result.iter().find(|f| f.path == "tests/test.rs").unwrap();

    assert_eq!(
        core.level,
        CompressionLevel::Skeleton,
        "Core should be Skeleton"
    );
    assert_eq!(
        tests.level,
        CompressionLevel::Drop,
        "Tests should be Dropped"
    );
}

#[test]
fn test_allocator_all_full_when_budget_allows() {
    // Budget = 1000 tokens (plenty of room)
    let files = vec![
        FileAllocation::new("src/core.rs", FileTier::Core, 100, 10),
        FileAllocation::new("config.toml", FileTier::Config, 100, 10),
        FileAllocation::new("tests/test.rs", FileTier::Tests, 100, 10),
    ];

    let allocator = AdaptiveAllocator::new(1000);
    let result = allocator.allocate(files);

    // All files should be Full
    for file in &result {
        assert_eq!(
            file.level,
            CompressionLevel::Full,
            "{} should be Full",
            file.path
        );
    }
}

#[test]
fn test_allocator_empty_files() {
    let files: Vec<FileAllocation> = vec![];
    let allocator = AdaptiveAllocator::new(100);
    let result = allocator.allocate(files);

    assert!(result.is_empty());
}

#[test]
fn test_allocator_zero_budget_drops_all() {
    let files = vec![FileAllocation::new("src/core.rs", FileTier::Core, 100, 10)];

    let allocator = AdaptiveAllocator::new(0);
    let result = allocator.allocate(files);

    let core = result.iter().find(|f| f.path == "src/core.rs").unwrap();
    assert_eq!(core.level, CompressionLevel::Drop);
}

// ============================================================================
// Section D: SkeletonResult Tests
// ============================================================================

#[test]
fn test_skeleton_result_compression_ratio() {
    let result = SkeletonResult {
        content: "fn main();".to_string(),
        original_tokens: 100,
        skeleton_tokens: 10,
        compression_ratio: 0.9,
        preserved_symbols: vec!["main".to_string()],
    };

    assert_eq!(result.compression_ratio, 0.9);
    assert_eq!(result.original_tokens, 100);
    assert_eq!(result.skeleton_tokens, 10);
}

#[test]
fn test_skeleton_result_preserved_symbols() {
    let input = r#"
pub fn foo() {}
pub fn bar() {}
struct Baz {}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    assert!(result.preserved_symbols.contains(&"foo".to_string()));
    assert!(result.preserved_symbols.contains(&"bar".to_string()));
    assert!(result.preserved_symbols.contains(&"Baz".to_string()));
}

// ============================================================================
// Section E: Language Detection Tests
// ============================================================================

#[test]
fn test_language_from_extension() {
    assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    assert_eq!(Language::from_extension("py"), Some(Language::Python));
    assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
    assert_eq!(Language::from_extension("go"), Some(Language::Go));
    assert_eq!(Language::from_extension("txt"), None);
}

// ============================================================================
// Section F: Edge Cases & Fallback Tests
// ============================================================================

#[test]
fn test_skeletonize_unbalanced_braces_fallback() {
    // Malformed Rust code with unbalanced braces
    let input = r#"
fn broken() {
    if true {
        // missing closing brace

fn another() {
    println!("hello");
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Fallback: Should return something (first N lines or best effort)
    assert!(
        !result.content.is_empty(),
        "Should not return empty on malformed input"
    );
    // Should still try to extract signatures
    assert!(
        result.content.contains("fn broken()") || result.content.contains("fn another()"),
        "Should extract at least some signatures"
    );
}

#[test]
fn test_skeletonize_empty_input() {
    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize("", Language::Rust);

    assert!(result.content.is_empty());
    assert_eq!(result.original_tokens, 0);
    assert_eq!(result.skeleton_tokens, 0);
}

#[test]
fn test_skeletonize_only_comments() {
    let input = r#"
// This is a comment
// Another comment
/* Block comment
   spanning multiple lines */
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Comments-only file: skeleton should be minimal or empty
    assert!(
        result.skeleton_tokens <= result.original_tokens,
        "Skeleton should not be larger than original"
    );
}

#[test]
fn test_skeletonize_preserves_type_definitions() {
    let input = r#"
pub type Result<T> = std::result::Result<T, Error>;
pub type Callback = Box<dyn Fn() -> ()>;

fn uses_types() -> Result<()> {
    Ok(())
}
"#;

    let skeletonizer = Skeletonizer::new();
    let result = skeletonizer.skeletonize(input, Language::Rust);

    // Type aliases should be preserved
    assert!(result.content.contains("pub type Result<T>"));
    assert!(result.content.contains("pub type Callback"));
}
