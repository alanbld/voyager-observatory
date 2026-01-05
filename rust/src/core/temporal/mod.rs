//! Temporal Module - The Chronos Engine
//!
//! This module provides geological analysis of code through time,
//! using repository history to identify stellar drift, volcanic churn,
//! and the ancient archaeological strata of the code galaxy.
//!
//! # Celestial Terminology
//!
//! - **Chronos Events**: Historical observations (commits) in the timeline
//! - **Stellar Age**: Time since the "Big Bang" (first observation) of a file
//! - **Volcanic Churn**: Recent activity level (observations in last 90 days)
//! - **Primary Observers**: Top contributors with most impact per constellation
//! - **Tectonic Shifts**: High-risk files with churn + complexity
//! - **Ancient Stars**: Untouched files (> 2 years) still core to logic
//! - **Supernovas**: Files with extreme recent activity (destabilizing)
//!
//! # Feature Gating
//!
//! The temporal feature requires the `temporal` feature flag and access to
//! a repository's history. When disabled or unavailable, the engine returns
//! a "Static Galaxy" state with no temporal data.

#[cfg(feature = "temporal")]
mod engine;

#[cfg(feature = "temporal")]
mod cache;

mod metrics;
mod geological;
mod stellar_drift;

#[cfg(feature = "temporal")]
pub use engine::{ChronosEngine, DEFAULT_COMMIT_DEPTH, FULL_COMMIT_DEPTH};

#[cfg(feature = "temporal")]
pub use cache::{
    ChronosCache, ChronosCacheManager, CachedObservation, CachedGalaxyStats,
    WarpStatus,
};

pub use metrics::{
    ChronosMetrics, StellarAge, VolcanicChurn, Observer, ObserverImpact,
    TemporalCensus, ConstellationChurn, FileChurn, ChronosState,
    TectonicShift, AncientStar, Supernova,
    AgeClassification, ChurnClassification,
};

pub use geological::{
    GeologicalAnalyzer, GeologicalSummary, GeologicalActivity,
};

pub use stellar_drift::{
    StellarDriftAnalyzer, StellarDriftReport, ConstellationEvolution, NewStar,
    NEW_STAR_THRESHOLD_DAYS, ANCIENT_STAR_THRESHOLD_DAYS, DRIFT_WINDOW_DAYS,
};

/// Static Galaxy fallback when temporal feature is disabled or unavailable
#[cfg(not(feature = "temporal"))]
pub struct ChronosEngine;

#[cfg(not(feature = "temporal"))]
impl ChronosEngine {
    /// Create a new engine (no-op without temporal feature)
    pub fn new(_root: &std::path::Path) -> Option<Self> {
        None
    }

    /// Returns static galaxy state
    pub fn state(&self) -> ChronosState {
        ChronosState::StaticGalaxy
    }
}

// =============================================================================
// Re-exports for convenience
// =============================================================================

/// Check if temporal analysis is available
pub fn is_temporal_available() -> bool {
    cfg!(feature = "temporal")
}

/// Get a human-readable description of temporal state
pub fn temporal_state_description(state: &ChronosState) -> &'static str {
    match state {
        ChronosState::Active { .. } => "Active Chronos Engine",
        ChronosState::ShallowCensus { .. } => "Shallow Chronos (partial history)",
        ChronosState::StaticGalaxy => "Static Galaxy (no temporal data)",
        ChronosState::NoRepository => "No observation history found",
        ChronosState::Error(_) => "Chronos Engine error",
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_temporal_available() {
        // When compiled with temporal feature, should return true
        #[cfg(feature = "temporal")]
        assert!(is_temporal_available());

        // When compiled without temporal feature, should return false
        #[cfg(not(feature = "temporal"))]
        assert!(!is_temporal_available());
    }

    #[test]
    fn test_temporal_state_description_active() {
        let state = ChronosState::Active {
            total_events: 100,
            galaxy_age_days: 365,
            observer_count: 5,
        };
        assert_eq!(temporal_state_description(&state), "Active Chronos Engine");
    }

    #[test]
    fn test_temporal_state_description_shallow() {
        let state = ChronosState::ShallowCensus {
            total_events: 1000,
            galaxy_age_days: 500,
            observer_count: 10,
            depth_limit: 1000,
        };
        assert_eq!(temporal_state_description(&state), "Shallow Chronos (partial history)");
    }

    #[test]
    fn test_temporal_state_description_static() {
        let state = ChronosState::StaticGalaxy;
        assert_eq!(temporal_state_description(&state), "Static Galaxy (no temporal data)");
    }

    #[test]
    fn test_temporal_state_description_no_repo() {
        let state = ChronosState::NoRepository;
        assert_eq!(temporal_state_description(&state), "No observation history found");
    }

    #[test]
    fn test_temporal_state_description_error() {
        let state = ChronosState::Error("Test error".to_string());
        assert_eq!(temporal_state_description(&state), "Chronos Engine error");
    }

    #[test]
    fn test_chronos_state_default() {
        let state = ChronosState::default();
        assert!(matches!(state, ChronosState::StaticGalaxy));
    }

    #[test]
    fn test_chronos_state_variants() {
        // Test Active variant
        let active = ChronosState::Active {
            total_events: 50,
            galaxy_age_days: 100,
            observer_count: 3,
        };
        if let ChronosState::Active { total_events, galaxy_age_days, observer_count } = active {
            assert_eq!(total_events, 50);
            assert_eq!(galaxy_age_days, 100);
            assert_eq!(observer_count, 3);
        }

        // Test ShallowCensus variant
        let shallow = ChronosState::ShallowCensus {
            total_events: 1000,
            galaxy_age_days: 200,
            observer_count: 5,
            depth_limit: 1000,
        };
        if let ChronosState::ShallowCensus { depth_limit, .. } = shallow {
            assert_eq!(depth_limit, 1000);
        }
    }

    #[test]
    fn test_exports_are_available() {
        // Test that all public types are properly exported
        let _age_class = AgeClassification::default();
        let _churn_class = ChurnClassification::default();
        let _chronos_state = ChronosState::default();
        let _chronos_metrics = ChronosMetrics::default();
        let _stellar_age = StellarAge::default();
        let _volcanic_churn = VolcanicChurn::default();
        let _observer = Observer::default();
        let _observer_impact = ObserverImpact::default();
        let _temporal_census = TemporalCensus::default();
        let _constellation_churn = ConstellationChurn::default();
        let _file_churn = FileChurn::default();
    }

    #[test]
    fn test_geological_exports() {
        let analyzer = GeologicalAnalyzer::new();
        assert_eq!(analyzer.tectonic_churn, 10);
        assert_eq!(analyzer.ancient_dormant_days, 730);
    }

    #[test]
    fn test_stellar_drift_exports() {
        let analyzer = StellarDriftAnalyzer::new();
        assert_eq!(analyzer.new_star_threshold, NEW_STAR_THRESHOLD_DAYS);
        assert_eq!(analyzer.ancient_star_threshold, ANCIENT_STAR_THRESHOLD_DAYS);
    }

    #[test]
    fn test_constants_exported() {
        assert_eq!(NEW_STAR_THRESHOLD_DAYS, 90);
        assert_eq!(ANCIENT_STAR_THRESHOLD_DAYS, 730);
        assert_eq!(DRIFT_WINDOW_DAYS, 180);
    }
}
