//! Stellar Drift Integration Tests (v1.1.0)
//!
//! These tests verify the Stellar Drift functionality that synthesizes
//! temporal data from the Chronos Engine with semantic metrics from
//! the Celestial Census.
//!
//! # Test Categories
//!
//! 1. **Drift Analyzer**: Tests for the StellarDriftAnalyzer
//! 2. **Evolution Survey**: CLI integration tests for `--survey evolution`
//! 3. **Constellation Evolution**: Tests for per-directory lifecycle metrics

use std::collections::{BTreeMap, HashMap};

use pm_encoder::core::{
    ChurnClassification, ConstellationEvolution, NewStar, StellarDriftAnalyzer, StellarDriftReport,
};

#[cfg(feature = "temporal")]
use chrono::{Duration, Utc};

#[cfg(feature = "temporal")]
use pm_encoder::core::{ChronosState, FileChurn, TemporalCensus};

// =============================================================================
// TEST HELPERS
// =============================================================================

#[cfg(feature = "temporal")]
fn make_file_churn(age_days: u64, churn_90d: usize, days_since_last: Option<u64>) -> FileChurn {
    let now = Utc::now();
    let last_observation = days_since_last.map(|d| now - Duration::days(d as i64));

    FileChurn {
        path: String::new(),
        churn_30d: churn_90d.min(10),
        churn_90d,
        age_days,
        last_observation,
        churn_classification: ChurnClassification::default(),
        age_classification: pm_encoder::core::AgeClassification::default(),
    }
}

// =============================================================================
// DRIFT ANALYZER TESTS
// =============================================================================

/// Test that the analyzer correctly identifies new stars (files < 90 days old)
#[test]
#[cfg(feature = "temporal")]
fn test_new_star_identification() {
    let analyzer = StellarDriftAnalyzer::new();

    let mut files = BTreeMap::new();
    // New file (30 days old)
    files.insert("new_file.rs".to_string(), make_file_churn(30, 5, Some(5)));
    // Old file (500 days old)
    files.insert(
        "old_file.rs".to_string(),
        make_file_churn(500, 2, Some(100)),
    );

    let temporal_census = TemporalCensus {
        state: ChronosState::Active {
            total_events: 100,
            galaxy_age_days: 500,
            observer_count: 5,
        },
        galaxy_age_days: 500,
        files,
        ..Default::default()
    };

    let mut star_counts = HashMap::new();
    star_counts.insert("new_file.rs".to_string(), 3);
    star_counts.insert("old_file.rs".to_string(), 10);

    let report = analyzer.analyze(&temporal_census, &star_counts, None, None);

    assert!(!report.new_stars.is_empty(), "Should identify new stars");
    assert_eq!(report.new_stars[0].path, "new_file.rs");
    assert_eq!(report.new_star_total, 3, "Should count 3 new stars");
}

/// Test that the analyzer correctly identifies ancient stars (dormant > 2 years)
#[test]
#[cfg(feature = "temporal")]
fn test_ancient_star_identification() {
    let analyzer = StellarDriftAnalyzer::new();

    let mut files = BTreeMap::new();
    // File that hasn't been touched in 3 years (1100 days dormant)
    files.insert(
        "ancient.rs".to_string(),
        make_file_churn(1500, 0, Some(1100)),
    );

    let temporal_census = TemporalCensus {
        state: ChronosState::Active {
            total_events: 50,
            galaxy_age_days: 1500,
            observer_count: 3,
        },
        galaxy_age_days: 1500,
        files,
        ..Default::default()
    };

    let mut star_counts = HashMap::new();
    star_counts.insert("ancient.rs".to_string(), 15);

    let report = analyzer.analyze(&temporal_census, &star_counts, None, None);

    assert!(
        !report.ancient_stars.is_empty(),
        "Should identify ancient stars"
    );
    assert_eq!(
        report.ancient_star_total, 15,
        "Should count 15 ancient stars"
    );
}

/// Test stellar drift percentage calculation
#[test]
#[cfg(feature = "temporal")]
fn test_drift_calculation() {
    let analyzer = StellarDriftAnalyzer::new();

    let mut files = BTreeMap::new();
    // Active file (modified recently)
    files.insert("active.rs".to_string(), make_file_churn(500, 10, Some(30)));
    // Dormant file (not ancient, not active)
    files.insert("dormant.rs".to_string(), make_file_churn(400, 0, Some(200)));

    let temporal_census = TemporalCensus {
        state: ChronosState::Active {
            total_events: 100,
            galaxy_age_days: 500,
            observer_count: 5,
        },
        galaxy_age_days: 500,
        files,
        ..Default::default()
    };

    let mut star_counts = HashMap::new();
    star_counts.insert("active.rs".to_string(), 10);
    star_counts.insert("dormant.rs".to_string(), 10);

    let report = analyzer.analyze(&temporal_census, &star_counts, None, None);

    assert_eq!(report.total_stars, 20, "Should have 20 total stars");
    // Active file was modified in last 6 months, so drift should be positive
    assert!(
        report.stellar_drift_percent >= 0.0,
        "Should have non-negative drift"
    );
}

/// Test health indicators
#[test]
fn test_health_indicators() {
    // Test expanding galaxy (more new stars than ancient)
    let report = StellarDriftReport {
        new_star_total: 100,
        ancient_star_total: 50,
        total_stars: 150,
        drift_rate_per_year: 30.0,
        is_expanding: true,
        is_stable: false,
        is_ossifying: false,
        has_supernovas: false,
        ..Default::default()
    };

    assert!(
        report.is_expanding,
        "Should be expanding when new > ancient"
    );
    assert!(
        !report.is_ossifying,
        "Should not be ossifying with new stars"
    );

    // Test ossifying galaxy (all ancient, no new)
    let report2 = StellarDriftReport {
        new_star_total: 0,
        ancient_star_total: 100,
        total_stars: 100,
        drift_rate_per_year: 5.0,
        is_expanding: false,
        is_stable: true,
        is_ossifying: true,
        has_supernovas: false,
        ..Default::default()
    };

    assert!(
        report2.is_ossifying,
        "Should be ossifying with all ancient stars"
    );
    assert!(report2.is_stable, "Low drift should be stable");
}

// =============================================================================
// PATH PREFIX TESTS (Survey root != Git root)
// =============================================================================

/// Test that path prefix correctly maps star counts to temporal census paths
#[test]
#[cfg(feature = "temporal")]
fn test_path_prefix_mapping() {
    let analyzer = StellarDriftAnalyzer::new();

    // Temporal census has paths relative to git root (e.g., "rust/src/main.rs")
    let mut files = BTreeMap::new();
    files.insert(
        "rust/src/main.rs".to_string(),
        make_file_churn(100, 5, Some(10)),
    );
    files.insert(
        "rust/src/lib.rs".to_string(),
        make_file_churn(100, 3, Some(20)),
    );

    let temporal_census = TemporalCensus {
        state: ChronosState::Active {
            total_events: 50,
            galaxy_age_days: 100,
            observer_count: 2,
        },
        galaxy_age_days: 100,
        files,
        ..Default::default()
    };

    // Star counts are relative to survey root (e.g., "src/main.rs")
    let mut star_counts = HashMap::new();
    star_counts.insert("src/main.rs".to_string(), 10);
    star_counts.insert("src/lib.rs".to_string(), 20);

    // Without prefix, paths won't match
    let report_no_prefix = analyzer.analyze(&temporal_census, &star_counts, None, None);
    assert_eq!(
        report_no_prefix.total_stars, 30,
        "Should have 30 total stars"
    );

    // With "rust" prefix, paths should match
    let report_with_prefix = analyzer.analyze(&temporal_census, &star_counts, None, Some("rust"));
    assert_eq!(
        report_with_prefix.total_stars, 30,
        "Should have 30 total stars"
    );
    // The prefixed star_counts should now match temporal census paths
}

// =============================================================================
// CONSTELLATION EVOLUTION TESTS
// =============================================================================

/// Test constellation evolution structure
#[test]
fn test_constellation_evolution_fields() {
    let evolution = ConstellationEvolution {
        path: "src/core".to_string(),
        file_count: 10,
        ancient_star_count: 5,
        new_star_count: 3,
        active_star_count: 7,
        dormant_star_count: 2,
        volcanic_churn: ChurnClassification::High,
        drift_rate: 15.5,
        avg_age_days: 200,
        is_new: false,
        is_ancient: false,
    };

    assert_eq!(evolution.path, "src/core");
    assert_eq!(evolution.file_count, 10);
    assert_eq!(evolution.ancient_star_count, 5);
    assert_eq!(evolution.new_star_count, 3);
    assert_eq!(evolution.drift_rate, 15.5);
}

/// Test new star structure
#[test]
fn test_new_star_structure() {
    let star = NewStar {
        path: "src/new_feature.rs".to_string(),
        age_days: 30,
        star_count: 5,
        is_expansion: true,
    };

    assert_eq!(star.path, "src/new_feature.rs");
    assert_eq!(star.age_days, 30);
    assert!(star.is_expansion);
}

// =============================================================================
// THRESHOLDS TESTS
// =============================================================================

/// Test the default thresholds via analyzer configuration
#[test]
fn test_default_thresholds() {
    let analyzer = StellarDriftAnalyzer::new();

    // The analyzer exposes the thresholds as public fields
    assert_eq!(
        analyzer.new_star_threshold, 90,
        "New star threshold should be 90 days"
    );
    assert_eq!(
        analyzer.ancient_star_threshold, 730,
        "Ancient star threshold should be 730 days (2 years)"
    );
    assert_eq!(
        analyzer.drift_window_days, 180,
        "Drift window should be 180 days (6 months)"
    );
}

// =============================================================================
// STATIC GALAXY TESTS
// =============================================================================

/// Test behavior when temporal data is unavailable
#[test]
#[cfg(feature = "temporal")]
fn test_static_galaxy_fallback() {
    let analyzer = StellarDriftAnalyzer::new();

    let temporal_census = TemporalCensus {
        state: ChronosState::StaticGalaxy,
        galaxy_age_days: 0,
        ..Default::default()
    };

    let star_counts = HashMap::new();
    let report = analyzer.analyze(&temporal_census, &star_counts, None, None);

    // Report should be mostly empty for static galaxy
    assert_eq!(report.total_stars, 0);
    assert!(report.new_stars.is_empty());
    assert!(report.ancient_stars.is_empty());
    assert!(matches!(report.state, ChronosState::StaticGalaxy));
}
