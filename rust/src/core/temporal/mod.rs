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

mod metrics;
mod geological;

#[cfg(feature = "temporal")]
pub use engine::ChronosEngine;

pub use metrics::{
    ChronosMetrics, StellarAge, VolcanicChurn, Observer, ObserverImpact,
    TemporalCensus, ConstellationChurn, FileChurn, ChronosState,
    TectonicShift, AncientStar, Supernova,
    AgeClassification, ChurnClassification,
};

pub use geological::{
    GeologicalAnalyzer, GeologicalSummary, GeologicalActivity,
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
        ChronosState::StaticGalaxy => "Static Galaxy (no temporal data)",
        ChronosState::NoRepository => "No observation history found",
        ChronosState::Error(_) => "Chronos Engine error",
    }
}
