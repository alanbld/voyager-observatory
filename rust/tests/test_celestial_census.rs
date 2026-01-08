//! TDD Verification Tests for Celestial Census (Phase 1C)
//!
//! These tests verify the Celestial Census correctly identifies:
//! - Dark Matter (syntax errors, unparsed regions)
//! - Two Hemispheres (Logic vs Interface)
//! - Composition metrics (Stars, Nebulae, Dark Matter)

use pm_encoder::core::metrics::MetricCollector;
use pm_encoder::core::{
    CelestialCensus, CensusMetrics, DarkMatterMetric, GalaxyCensus, HealthRating,
    NebulaeCountMetric, StarCountMetric,
};
use voyager_ast::ir::{
    Comment, CommentKind, Declaration, DeclarationKind, File, LanguageId, Span, UnknownNode,
    Visibility,
};

// =============================================================================
// Helper Functions
// =============================================================================

fn create_clean_file() -> File {
    let mut file = File::new("clean.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 500, 1, 25);

    // Add documented function
    let mut func = Declaration::new(
        "calculate_total".to_string(),
        DeclarationKind::Function,
        Span::new(0, 100, 1, 5),
    );
    func.visibility = Visibility::Public;
    func.doc_comment = Some(Comment {
        text: "Calculates the total value.".to_string(),
        kind: CommentKind::Doc,
        span: Span::new(0, 30, 1, 1),
        attached_to: None,
    });
    file.declarations.push(func);

    // Add undocumented method
    file.declarations.push(Declaration::new(
        "helper_method".to_string(),
        DeclarationKind::Method,
        Span::new(100, 200, 6, 10),
    ));

    file
}

fn create_file_with_dark_matter() -> File {
    let mut file = File::new("broken.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 1000, 1, 50);

    // Add function
    file.declarations.push(Declaration::new(
        "working_func".to_string(),
        DeclarationKind::Function,
        Span::new(0, 100, 1, 5),
    ));

    // Add unknown/unparsed regions (Dark Matter)
    file.unknown_regions.push(UnknownNode {
        span: Span::new(200, 350, 10, 15),
        reason: Some("Syntax error: unexpected token".to_string()),
        raw_text: Some("fn broken( { invalid syntax".to_string()),
    });

    file.unknown_regions.push(UnknownNode {
        span: Span::new(400, 500, 20, 25),
        reason: Some("Unrecognized construct".to_string()),
        raw_text: None,
    });

    file
}

fn create_file_with_deep_nesting() -> File {
    let mut file = File::new("nested.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 1000, 1, 50);

    // Create deeply nested structure (> 4 levels)
    let mut outer = Declaration::new(
        "level1".to_string(),
        DeclarationKind::Function,
        Span::new(0, 900, 1, 45),
    );

    let mut level2 = Declaration::new(
        "level2".to_string(),
        DeclarationKind::Method,
        Span::new(10, 800, 2, 40),
    );

    let mut level3 = Declaration::new(
        "level3".to_string(),
        DeclarationKind::Method,
        Span::new(20, 700, 3, 35),
    );

    let mut level4 = Declaration::new(
        "level4".to_string(),
        DeclarationKind::Method,
        Span::new(30, 600, 4, 30),
    );

    let level5 = Declaration::new(
        "level5".to_string(),
        DeclarationKind::Method,
        Span::new(40, 500, 5, 25),
    );

    level4.children.push(level5);
    level3.children.push(level4);
    level2.children.push(level3);
    outer.children.push(level2);
    file.declarations.push(outer);

    file
}

// =============================================================================
// Test: Dark Matter Detection
// =============================================================================

#[test]
fn test_dark_matter_detection_with_syntax_errors() {
    let file = create_file_with_dark_matter();
    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // Should detect 2 unknown regions
    assert_eq!(
        metrics.dark_matter.unknown_regions, 2,
        "Should detect 2 dark matter regions (unknown/unparsed code)"
    );

    // Should count total bytes in unknown regions
    assert!(
        metrics.dark_matter.unknown_bytes > 0,
        "Should count bytes in dark matter regions"
    );

    // Dark matter ratio should be > 0
    assert!(
        metrics.derived.dark_matter_ratio > 0.0,
        "Dark matter ratio should be positive"
    );
}

#[test]
fn test_dark_matter_volcanic_regions() {
    let file = create_file_with_deep_nesting();
    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // Should detect volcanic regions (nesting > 4)
    assert!(
        metrics.dark_matter.volcanic_regions > 0,
        "Should detect volcanic regions from deep nesting"
    );

    // Max nesting should be at least 5
    assert!(
        metrics.dark_matter.max_nesting_depth >= 4,
        "Max nesting depth should be at least 4"
    );
}

#[test]
fn test_clean_file_no_dark_matter() {
    let file = create_clean_file();
    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // Clean file should have no dark matter
    assert_eq!(
        metrics.dark_matter.unknown_regions, 0,
        "Clean file should have no unknown regions"
    );
    assert_eq!(
        metrics.dark_matter.unknown_bytes, 0,
        "Clean file should have 0 unknown bytes"
    );
    assert_eq!(
        metrics.dark_matter.volcanic_regions, 0,
        "Clean file should have no volcanic regions"
    );
}

// =============================================================================
// Test: Two Hemispheres (Logic vs Interface)
// =============================================================================

#[test]
fn test_two_hemispheres_logic_detection() {
    let mut file = File::new("logic.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 500, 1, 25);

    // Logic hemisphere: Functions and methods
    file.declarations.push(Declaration::new(
        "compute".to_string(),
        DeclarationKind::Function,
        Span::new(0, 50, 1, 3),
    ));
    file.declarations.push(Declaration::new(
        "process".to_string(),
        DeclarationKind::Function,
        Span::new(50, 100, 4, 6),
    ));
    file.declarations.push(Declaration::new(
        "transform".to_string(),
        DeclarationKind::Method,
        Span::new(100, 150, 7, 9),
    ));

    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // Should count stars (logic units)
    assert_eq!(metrics.stars.count, 3, "Should count 3 stars (logic units)");
    assert_eq!(metrics.stars.functions, 2, "Should count 2 functions");
    assert_eq!(metrics.stars.methods, 1, "Should count 1 method");
}

#[test]
fn test_two_hemispheres_interface_detection() {
    let mut file = File::new("types.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 500, 1, 25);

    // Interface hemisphere: Types, traits, interfaces
    file.declarations.push(Declaration::new(
        "Config".to_string(),
        DeclarationKind::Struct,
        Span::new(0, 50, 1, 3),
    ));
    file.declarations.push(Declaration::new(
        "Processor".to_string(),
        DeclarationKind::Trait,
        Span::new(50, 100, 4, 6),
    ));
    file.declarations.push(Declaration::new(
        "Status".to_string(),
        DeclarationKind::Enum,
        Span::new(100, 150, 7, 9),
    ));

    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // Should count types (interface units)
    assert_eq!(
        metrics.stars.types, 3,
        "Should count 3 types (interface units)"
    );
    // Functions should be 0
    assert_eq!(metrics.stars.functions, 0, "Should count 0 functions");
}

// =============================================================================
// Test: Composition Table
// =============================================================================

#[test]
fn test_composition_stars_count() {
    let file = create_clean_file();
    let metric = StarCountMetric;
    let result = metric.analyze(&file);

    // 1 function + 1 method = 2 stars
    assert_eq!(result.value, 2.0, "Star count should be 2");
    assert!(result.confidence > 0.9, "Should have high confidence");
}

#[test]
fn test_composition_nebulae_count() {
    let mut file = create_clean_file();

    // Add file-level comment
    file.comments.push(Comment {
        text: "This is a module comment.".to_string(),
        kind: CommentKind::Block,
        span: Span::new(0, 30, 1, 2),
        attached_to: None,
    });

    let metric = NebulaeCountMetric;
    let result = metric.analyze(&file);

    // Should count doc lines + comment lines
    assert!(result.value > 0.0, "Should have nebulae (documentation)");
}

#[test]
fn test_composition_dark_matter_count() {
    let file = create_file_with_dark_matter();
    let metric = DarkMatterMetric;
    let result = metric.analyze(&file);

    // Should count dark matter regions
    assert_eq!(result.value, 2.0, "Dark matter count should be 2");
}

// =============================================================================
// Test: Derived Metrics
// =============================================================================

#[test]
fn test_stellar_density_calculation() {
    let mut file = File::new("dense.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 100, 1, 10); // 10 lines

    // Add 3 functions
    for i in 0..3 {
        file.declarations.push(Declaration::new(
            format!("func{}", i),
            DeclarationKind::Function,
            Span::new(i * 10, i * 10 + 10, i + 1, i + 2),
        ));
    }

    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // 3 stars in 10 lines = 300 stars per 1000 LOC
    assert!(
        metrics.derived.stellar_density > 200.0,
        "Stellar density should be high for dense code"
    );
}

#[test]
fn test_nebula_ratio_calculation() {
    let mut file = File::new("documented.rs".to_string(), LanguageId::Rust);
    file.span = Span::new(0, 100, 1, 10);

    // Add 3 functions, 2 documented
    for i in 0..3 {
        let mut func = Declaration::new(
            format!("func{}", i),
            DeclarationKind::Function,
            Span::new(i * 10, i * 10 + 10, i + 1, i + 2),
        );
        if i < 2 {
            func.doc_comment = Some(Comment {
                text: format!("Doc for func{}", i),
                kind: CommentKind::Doc,
                span: Span::new(i * 10, i * 10 + 5, i + 1, i + 1),
                attached_to: None,
            });
        }
        file.declarations.push(func);
    }

    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);

    // 2 out of 3 documented = ~66.7%
    assert!(
        metrics.derived.nebula_ratio >= 0.6 && metrics.derived.nebula_ratio <= 0.7,
        "Nebula ratio should be around 66%: actual={}",
        metrics.derived.nebula_ratio
    );
}

// =============================================================================
// Test: Health Rating
// =============================================================================

#[test]
fn test_health_rating_clean_code() {
    let file = create_clean_file();
    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);
    let rating = census.rate_health(&metrics);

    // Clean code should be Healthy or Stable
    assert!(
        rating == HealthRating::Healthy || rating == HealthRating::Stable,
        "Clean code should be rated Healthy or Stable"
    );
}

#[test]
fn test_health_rating_high_dark_matter() {
    let file = create_file_with_dark_matter();
    let census = CelestialCensus::new();
    let metrics = census.analyze(&file);
    let rating = census.rate_health(&metrics);

    // Code with dark matter should be flagged
    assert!(
        rating == HealthRating::HighDarkMatter || rating == HealthRating::Critical,
        "Code with dark matter should be rated HighDarkMatter or Critical"
    );
}

// =============================================================================
// Test: Galaxy Census Aggregation
// =============================================================================

#[test]
fn test_galaxy_census_aggregation() {
    let census = CelestialCensus::new();
    let mut galaxy = GalaxyCensus::new("test_project".to_string());

    // Add files from different constellations
    let file1 = create_clean_file();
    let metrics1 = census.analyze(&file1);
    galaxy.add_file("src/lib.rs", metrics1);

    let mut file2 = create_clean_file();
    file2.path = "tests/test.rs".to_string();
    let metrics2 = census.analyze(&file2);
    galaxy.add_file("tests/test.rs", metrics2);

    galaxy.finalize();

    // Should have 2 files
    assert_eq!(galaxy.total_files, 2, "Should have 2 files");

    // Should have 2 constellations (src and tests)
    assert_eq!(
        galaxy.constellations.len(),
        2,
        "Should have 2 constellations"
    );

    // Totals should be aggregated
    assert_eq!(
        galaxy.totals.stars.count, 4,
        "Total stars should be 4 (2+2)"
    );
}

// =============================================================================
// Test: Determinism (BTreeMap ordering)
// =============================================================================

#[test]
fn test_census_deterministic_output() {
    let file = create_clean_file();
    let census = CelestialCensus::new();

    // Run analysis multiple times
    let metrics1 = census.analyze(&file);
    let metrics2 = census.analyze(&file);

    // Results should be identical
    assert_eq!(metrics1.stars.count, metrics2.stars.count);
    assert_eq!(metrics1.nebulae.doc_lines, metrics2.nebulae.doc_lines);
    assert_eq!(
        metrics1.dark_matter.unknown_regions,
        metrics2.dark_matter.unknown_regions
    );
    assert!((metrics1.derived.health_score - metrics2.derived.health_score).abs() < 0.001);
}

#[test]
fn test_galaxy_census_deterministic_ordering() {
    let census = CelestialCensus::new();

    // Create galaxy with files in random order
    let files = vec![
        ("z/zebra.rs", create_clean_file()),
        ("a/apple.rs", create_clean_file()),
        ("m/mango.rs", create_clean_file()),
    ];

    let mut galaxy1 = GalaxyCensus::new(".".to_string());
    let mut galaxy2 = GalaxyCensus::new(".".to_string());

    for (path, file) in &files {
        let metrics = census.analyze(file);
        galaxy1.add_file(path, metrics.clone());
    }

    // Add in reverse order
    for (path, file) in files.iter().rev() {
        let metrics = census.analyze(file);
        galaxy2.add_file(path, metrics.clone());
    }

    galaxy1.finalize();
    galaxy2.finalize();

    // Constellation ordering should be identical (BTreeMap)
    let keys1: Vec<_> = galaxy1.constellations.keys().collect();
    let keys2: Vec<_> = galaxy2.constellations.keys().collect();
    assert_eq!(
        keys1, keys2,
        "Constellation ordering should be deterministic"
    );
}

// =============================================================================
// Test: Performance (< 50ms overhead)
// =============================================================================

#[test]
fn test_census_performance() {
    use std::time::Instant;

    let file = create_clean_file();
    let census = CelestialCensus::new();

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = census.analyze(&file);
    }
    let elapsed = start.elapsed();

    // 1000 analyses should complete in < 100ms (0.1ms per file)
    assert!(
        elapsed.as_millis() < 100,
        "Census should be fast: {}ms for 1000 files",
        elapsed.as_millis()
    );
}
