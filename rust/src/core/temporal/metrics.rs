//! Temporal Metrics - Geological Data Structures
//!
//! This module defines the data structures for temporal analysis,
//! including stellar age, volcanic churn, and observer impact.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// =============================================================================
// Core Temporal Types
// =============================================================================

/// State of the Chronos Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChronosState {
    /// Active engine with observation history
    Active {
        /// Total number of chronos events analyzed
        total_events: usize,
        /// Age of the galaxy (time since first event)
        galaxy_age_days: u64,
        /// Number of unique observers
        observer_count: usize,
    },
    /// Shallow census - commit limit reached (performance optimization)
    ShallowCensus {
        /// Total events analyzed (up to limit)
        total_events: usize,
        /// Age of the galaxy
        galaxy_age_days: u64,
        /// Number of unique observers
        observer_count: usize,
        /// Maximum depth limit that was used
        depth_limit: usize,
    },
    /// No temporal data available (feature disabled)
    StaticGalaxy,
    /// No observation history found (.git missing)
    NoRepository,
    /// Error accessing temporal data
    Error(String),
}

impl Default for ChronosState {
    fn default() -> Self {
        Self::StaticGalaxy
    }
}

/// Complete temporal metrics for a file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChronosMetrics {
    /// Stellar age (time since first observation)
    pub stellar_age: StellarAge,
    /// Volcanic churn (recent activity)
    pub volcanic_churn: VolcanicChurn,
    /// Primary observers (top contributors)
    pub primary_observers: Vec<Observer>,
    /// Last observation timestamp
    pub last_observation: Option<DateTime<Utc>>,
    /// Total observation count
    pub total_observations: usize,
}

/// Stellar Age - time since the "Big Bang" of a file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StellarAge {
    /// First observation timestamp (creation date)
    pub first_observation: Option<DateTime<Utc>>,
    /// Age in days
    pub age_days: u64,
    /// Classification based on age
    pub classification: AgeClassification,
}

/// Age classification for stars
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgeClassification {
    /// Recently formed (< 30 days)
    Newborn,
    /// Young star (30 days - 1 year)
    Young,
    /// Mature star (1-2 years)
    Mature,
    /// Ancient star (> 2 years)
    Ancient,
    /// Unknown age
    #[default]
    Unknown,
}

impl AgeClassification {
    /// Determine classification from age in days
    pub fn from_days(days: u64) -> Self {
        if days < 30 {
            Self::Newborn
        } else if days < 365 {
            Self::Young
        } else if days < 730 {
            Self::Mature
        } else {
            Self::Ancient
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Newborn => "Newborn Star",
            Self::Young => "Young Star",
            Self::Mature => "Mature Star",
            Self::Ancient => "Ancient Star",
            Self::Unknown => "Unknown Age",
        }
    }
}

/// Volcanic Churn - recent activity level
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolcanicChurn {
    /// Observations in last 30 days
    pub last_30_days: usize,
    /// Observations in last 90 days
    pub last_90_days: usize,
    /// Observations in last year
    pub last_year: usize,
    /// Lines added in last 90 days
    pub lines_added_90d: usize,
    /// Lines removed in last 90 days
    pub lines_removed_90d: usize,
    /// Churn classification
    pub classification: ChurnClassification,
}

/// Churn classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChurnClassification {
    /// Dormant (0 observations in 90 days)
    Dormant,
    /// Low activity (1-3 observations in 90 days)
    Low,
    /// Moderate activity (4-10 observations in 90 days)
    #[default]
    Moderate,
    /// High activity (11-30 observations in 90 days)
    High,
    /// Supernova (> 30 observations in 30 days)
    Supernova,
}

impl ChurnClassification {
    /// Determine classification from observation counts
    pub fn from_counts(last_30: usize, last_90: usize) -> Self {
        if last_30 > 30 {
            Self::Supernova
        } else if last_90 > 10 {
            Self::High
        } else if last_90 > 3 {
            Self::Moderate
        } else if last_90 > 0 {
            Self::Low
        } else {
            Self::Dormant
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Dormant => "Dormant",
            Self::Low => "Low Activity",
            Self::Moderate => "Moderate Activity",
            Self::High => "High Activity",
            Self::Supernova => "Supernova (Extreme Activity)",
        }
    }

    /// Check if this is considered high churn
    pub fn is_high(&self) -> bool {
        matches!(self, Self::High | Self::Supernova)
    }
}

/// An observer (contributor) in the galaxy
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Observer {
    /// Observer name
    pub name: String,
    /// Observer email (hashed for privacy)
    pub email_hash: String,
    /// Impact metrics
    pub impact: ObserverImpact,
}

/// Impact metrics for an observer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObserverImpact {
    /// Total observations (commits) by this observer
    pub observations: usize,
    /// Lines added
    pub lines_added: usize,
    /// Lines removed
    pub lines_removed: usize,
    /// Net impact (added - removed)
    pub net_impact: i64,
    /// First observation timestamp
    pub first_seen: Option<DateTime<Utc>>,
    /// Last observation timestamp
    pub last_seen: Option<DateTime<Utc>>,
}

// =============================================================================
// Aggregation Types
// =============================================================================

/// Temporal census for an entire galaxy (project)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalCensus {
    /// Chronos engine state
    pub state: ChronosState,
    /// Galaxy-level metrics
    pub galaxy_age_days: u64,
    /// Total observations analyzed
    pub total_observations: usize,
    /// Unique observer count
    pub observer_count: usize,
    /// Churn by constellation (BTreeMap for determinism)
    pub constellations: BTreeMap<String, ConstellationChurn>,
    /// File-level churn (BTreeMap for determinism)
    pub files: BTreeMap<String, FileChurn>,
    /// Top observers across the galaxy
    pub top_observers: Vec<Observer>,
    /// Tectonic shifts identified
    pub tectonic_shifts: Vec<TectonicShift>,
    /// Ancient stars identified
    pub ancient_stars: Vec<AncientStar>,
    /// Supernovas identified
    pub supernovas: Vec<Supernova>,
}

/// Churn metrics for a constellation (directory)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstellationChurn {
    /// Constellation path
    pub path: String,
    /// File count
    pub file_count: usize,
    /// Total observations in last 90 days
    pub churn_90d: usize,
    /// Average age of stars (days)
    pub avg_age_days: u64,
    /// Primary observers for this constellation
    pub primary_observers: Vec<Observer>,
    /// Churn classification
    pub classification: ChurnClassification,
}

/// Churn metrics for a single file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileChurn {
    /// File path
    pub path: String,
    /// Observations in last 30 days
    pub churn_30d: usize,
    /// Observations in last 90 days
    pub churn_90d: usize,
    /// Age in days
    pub age_days: u64,
    /// Last observation
    pub last_observation: Option<DateTime<Utc>>,
    /// Classification
    pub churn_classification: ChurnClassification,
    pub age_classification: AgeClassification,
}

// =============================================================================
// Risk Identification Types
// =============================================================================

/// A tectonic shift - high-risk file with churn + complexity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TectonicShift {
    /// File path
    pub path: String,
    /// Churn in last 90 days
    pub churn_90d: usize,
    /// Dark matter ratio (complexity)
    pub dark_matter_ratio: f64,
    /// Risk score (0.0 - 1.0)
    pub risk_score: f64,
    /// Reason for flagging
    pub reason: String,
}

/// An ancient star - untouched but core to logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AncientStar {
    /// File path
    pub path: String,
    /// Age in days
    pub age_days: u64,
    /// Days since last observation
    pub dormant_days: u64,
    /// Star count (functions/methods) - indicates importance
    pub star_count: usize,
    /// Is this a core file?
    pub is_core: bool,
}

/// A supernova - file with extreme recent activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supernova {
    /// File path
    pub path: String,
    /// Observations in last 30 days
    pub observations_30d: usize,
    /// Observers involved
    pub observer_count: usize,
    /// Lines changed
    pub lines_changed: usize,
    /// Warning message
    pub warning: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AgeClassification Tests
    // =========================================================================

    #[test]
    fn test_age_classification() {
        assert_eq!(AgeClassification::from_days(15), AgeClassification::Newborn);
        assert_eq!(AgeClassification::from_days(100), AgeClassification::Young);
        assert_eq!(AgeClassification::from_days(500), AgeClassification::Mature);
        assert_eq!(AgeClassification::from_days(800), AgeClassification::Ancient);
    }

    #[test]
    fn test_age_classification_boundaries() {
        // Exactly at boundaries
        assert_eq!(AgeClassification::from_days(0), AgeClassification::Newborn);
        assert_eq!(AgeClassification::from_days(29), AgeClassification::Newborn);
        assert_eq!(AgeClassification::from_days(30), AgeClassification::Young);
        assert_eq!(AgeClassification::from_days(364), AgeClassification::Young);
        assert_eq!(AgeClassification::from_days(365), AgeClassification::Mature);
        assert_eq!(AgeClassification::from_days(729), AgeClassification::Mature);
        assert_eq!(AgeClassification::from_days(730), AgeClassification::Ancient);
    }

    #[test]
    fn test_age_classification_description() {
        assert_eq!(AgeClassification::Newborn.description(), "Newborn Star");
        assert_eq!(AgeClassification::Young.description(), "Young Star");
        assert_eq!(AgeClassification::Mature.description(), "Mature Star");
        assert_eq!(AgeClassification::Ancient.description(), "Ancient Star");
        assert_eq!(AgeClassification::Unknown.description(), "Unknown Age");
    }

    #[test]
    fn test_age_classification_default() {
        assert_eq!(AgeClassification::default(), AgeClassification::Unknown);
    }

    #[test]
    fn test_age_classification_equality() {
        assert_eq!(AgeClassification::Newborn, AgeClassification::Newborn);
        assert_ne!(AgeClassification::Newborn, AgeClassification::Ancient);
    }

    // =========================================================================
    // ChurnClassification Tests
    // =========================================================================

    #[test]
    fn test_churn_classification() {
        assert_eq!(ChurnClassification::from_counts(0, 0), ChurnClassification::Dormant);
        assert_eq!(ChurnClassification::from_counts(1, 2), ChurnClassification::Low);
        assert_eq!(ChurnClassification::from_counts(3, 8), ChurnClassification::Moderate);
        assert_eq!(ChurnClassification::from_counts(5, 15), ChurnClassification::High);
        assert_eq!(ChurnClassification::from_counts(35, 50), ChurnClassification::Supernova);
    }

    #[test]
    fn test_churn_classification_boundaries() {
        // Dormant: last_90 = 0
        assert_eq!(ChurnClassification::from_counts(0, 0), ChurnClassification::Dormant);

        // Low: 0 < last_90 <= 3
        assert_eq!(ChurnClassification::from_counts(0, 1), ChurnClassification::Low);
        assert_eq!(ChurnClassification::from_counts(0, 3), ChurnClassification::Low);

        // Moderate: 3 < last_90 <= 10
        assert_eq!(ChurnClassification::from_counts(0, 4), ChurnClassification::Moderate);
        assert_eq!(ChurnClassification::from_counts(0, 10), ChurnClassification::Moderate);

        // High: last_90 > 10
        assert_eq!(ChurnClassification::from_counts(0, 11), ChurnClassification::High);

        // Supernova: last_30 > 30 (overrides everything)
        assert_eq!(ChurnClassification::from_counts(31, 0), ChurnClassification::Supernova);
        assert_eq!(ChurnClassification::from_counts(31, 100), ChurnClassification::Supernova);
    }

    #[test]
    fn test_churn_classification_description() {
        assert_eq!(ChurnClassification::Dormant.description(), "Dormant");
        assert_eq!(ChurnClassification::Low.description(), "Low Activity");
        assert_eq!(ChurnClassification::Moderate.description(), "Moderate Activity");
        assert_eq!(ChurnClassification::High.description(), "High Activity");
        assert_eq!(ChurnClassification::Supernova.description(), "Supernova (Extreme Activity)");
    }

    #[test]
    fn test_churn_classification_is_high() {
        assert!(!ChurnClassification::Dormant.is_high());
        assert!(!ChurnClassification::Low.is_high());
        assert!(!ChurnClassification::Moderate.is_high());
        assert!(ChurnClassification::High.is_high());
        assert!(ChurnClassification::Supernova.is_high());
    }

    #[test]
    fn test_churn_classification_default() {
        assert_eq!(ChurnClassification::default(), ChurnClassification::Moderate);
    }

    // =========================================================================
    // ChronosState Tests
    // =========================================================================

    #[test]
    fn test_chronos_state_default() {
        let state = ChronosState::default();
        assert!(matches!(state, ChronosState::StaticGalaxy));
    }

    #[test]
    fn test_chronos_state_serialization() {
        let state = ChronosState::Active {
            total_events: 100,
            galaxy_age_days: 365,
            observer_count: 5,
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ChronosState = serde_json::from_str(&json).unwrap();
        if let ChronosState::Active { total_events, .. } = parsed {
            assert_eq!(total_events, 100);
        } else {
            panic!("Expected Active state");
        }
    }

    #[test]
    fn test_chronos_state_error_serialization() {
        let state = ChronosState::Error("Test error message".to_string());
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ChronosState = serde_json::from_str(&json).unwrap();
        if let ChronosState::Error(msg) = parsed {
            assert_eq!(msg, "Test error message");
        } else {
            panic!("Expected Error state");
        }
    }

    // =========================================================================
    // ChronosMetrics Tests
    // =========================================================================

    #[test]
    fn test_chronos_metrics_default() {
        let metrics = ChronosMetrics::default();
        assert!(metrics.primary_observers.is_empty());
        assert!(metrics.last_observation.is_none());
        assert_eq!(metrics.total_observations, 0);
    }

    #[test]
    fn test_chronos_metrics_serialization() {
        let metrics = ChronosMetrics {
            stellar_age: StellarAge::default(),
            volcanic_churn: VolcanicChurn::default(),
            primary_observers: vec![Observer::default()],
            last_observation: Some(Utc::now()),
            total_observations: 42,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        let parsed: ChronosMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_observations, 42);
        assert_eq!(parsed.primary_observers.len(), 1);
    }

    // =========================================================================
    // StellarAge Tests
    // =========================================================================

    #[test]
    fn test_stellar_age_default() {
        let age = StellarAge::default();
        assert!(age.first_observation.is_none());
        assert_eq!(age.age_days, 0);
        assert_eq!(age.classification, AgeClassification::Unknown);
    }

    #[test]
    fn test_stellar_age_with_values() {
        let age = StellarAge {
            first_observation: Some(Utc::now()),
            age_days: 100,
            classification: AgeClassification::Young,
        };
        assert!(age.first_observation.is_some());
        assert_eq!(age.age_days, 100);
    }

    // =========================================================================
    // VolcanicChurn Tests
    // =========================================================================

    #[test]
    fn test_volcanic_churn_default() {
        let churn = VolcanicChurn::default();
        assert_eq!(churn.last_30_days, 0);
        assert_eq!(churn.last_90_days, 0);
        assert_eq!(churn.last_year, 0);
        assert_eq!(churn.lines_added_90d, 0);
        assert_eq!(churn.lines_removed_90d, 0);
    }

    #[test]
    fn test_volcanic_churn_with_values() {
        let churn = VolcanicChurn {
            last_30_days: 10,
            last_90_days: 25,
            last_year: 100,
            lines_added_90d: 500,
            lines_removed_90d: 200,
            classification: ChurnClassification::High,
        };
        assert_eq!(churn.last_30_days, 10);
        assert!(churn.classification.is_high());
    }

    // =========================================================================
    // Observer Tests
    // =========================================================================

    #[test]
    fn test_observer_default() {
        let observer = Observer::default();
        assert!(observer.name.is_empty());
        assert!(observer.email_hash.is_empty());
    }

    #[test]
    fn test_observer_with_values() {
        let observer = Observer {
            name: "Test User".to_string(),
            email_hash: "abc123".to_string(),
            impact: ObserverImpact {
                observations: 50,
                lines_added: 1000,
                lines_removed: 500,
                net_impact: 500,
                first_seen: Some(Utc::now()),
                last_seen: Some(Utc::now()),
            },
        };
        assert_eq!(observer.name, "Test User");
        assert_eq!(observer.impact.observations, 50);
        assert_eq!(observer.impact.net_impact, 500);
    }

    // =========================================================================
    // ObserverImpact Tests
    // =========================================================================

    #[test]
    fn test_observer_impact_default() {
        let impact = ObserverImpact::default();
        assert_eq!(impact.observations, 0);
        assert_eq!(impact.lines_added, 0);
        assert_eq!(impact.lines_removed, 0);
        assert_eq!(impact.net_impact, 0);
        assert!(impact.first_seen.is_none());
        assert!(impact.last_seen.is_none());
    }

    // =========================================================================
    // TemporalCensus Tests
    // =========================================================================

    #[test]
    fn test_temporal_census_determinism() {
        let census = TemporalCensus::default();
        // BTreeMap ensures deterministic ordering
        assert!(census.constellations.is_empty());
        assert!(census.files.is_empty());
    }

    #[test]
    fn test_temporal_census_default() {
        let census = TemporalCensus::default();
        assert_eq!(census.galaxy_age_days, 0);
        assert_eq!(census.total_observations, 0);
        assert_eq!(census.observer_count, 0);
        assert!(census.top_observers.is_empty());
        assert!(census.tectonic_shifts.is_empty());
        assert!(census.ancient_stars.is_empty());
        assert!(census.supernovas.is_empty());
    }

    #[test]
    fn test_temporal_census_with_data() {
        let mut census = TemporalCensus::default();
        census.galaxy_age_days = 1000;
        census.total_observations = 500;
        census.observer_count = 10;

        census.constellations.insert("src".to_string(), ConstellationChurn::default());
        census.files.insert("src/main.rs".to_string(), FileChurn::default());

        assert_eq!(census.constellations.len(), 1);
        assert_eq!(census.files.len(), 1);
    }

    // =========================================================================
    // ConstellationChurn Tests
    // =========================================================================

    #[test]
    fn test_constellation_churn_default() {
        let churn = ConstellationChurn::default();
        assert!(churn.path.is_empty());
        assert_eq!(churn.file_count, 0);
        assert_eq!(churn.churn_90d, 0);
        assert_eq!(churn.avg_age_days, 0);
        assert!(churn.primary_observers.is_empty());
    }

    #[test]
    fn test_constellation_churn_with_values() {
        let churn = ConstellationChurn {
            path: "src/core".to_string(),
            file_count: 15,
            churn_90d: 50,
            avg_age_days: 300,
            primary_observers: vec![Observer::default()],
            classification: ChurnClassification::High,
        };
        assert_eq!(churn.path, "src/core");
        assert_eq!(churn.file_count, 15);
    }

    // =========================================================================
    // FileChurn Tests
    // =========================================================================

    #[test]
    fn test_file_churn_default() {
        let churn = FileChurn::default();
        assert!(churn.path.is_empty());
        assert_eq!(churn.churn_30d, 0);
        assert_eq!(churn.churn_90d, 0);
        assert_eq!(churn.age_days, 0);
        assert!(churn.last_observation.is_none());
    }

    #[test]
    fn test_file_churn_with_values() {
        let churn = FileChurn {
            path: "src/main.rs".to_string(),
            churn_30d: 5,
            churn_90d: 15,
            age_days: 500,
            last_observation: Some(Utc::now()),
            churn_classification: ChurnClassification::High,
            age_classification: AgeClassification::Mature,
        };
        assert_eq!(churn.path, "src/main.rs");
        assert_eq!(churn.churn_30d, 5);
        assert!(churn.churn_classification.is_high());
    }

    // =========================================================================
    // TectonicShift Tests
    // =========================================================================

    #[test]
    fn test_tectonic_shift_creation() {
        let shift = TectonicShift {
            path: "risky_file.rs".to_string(),
            churn_90d: 25,
            dark_matter_ratio: 0.35,
            risk_score: 0.75,
            reason: "High churn + high complexity".to_string(),
        };
        assert_eq!(shift.path, "risky_file.rs");
        assert_eq!(shift.churn_90d, 25);
        assert!(shift.risk_score > 0.5);
    }

    #[test]
    fn test_tectonic_shift_serialization() {
        let shift = TectonicShift {
            path: "test.rs".to_string(),
            churn_90d: 20,
            dark_matter_ratio: 0.3,
            risk_score: 0.6,
            reason: "Test".to_string(),
        };
        let json = serde_json::to_string(&shift).unwrap();
        let parsed: TectonicShift = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "test.rs");
    }

    // =========================================================================
    // AncientStar Tests
    // =========================================================================

    #[test]
    fn test_ancient_star_creation() {
        let star = AncientStar {
            path: "old_core.rs".to_string(),
            age_days: 1500,
            dormant_days: 1000,
            star_count: 25,
            is_core: true,
        };
        assert_eq!(star.path, "old_core.rs");
        assert!(star.is_core);
        assert!(star.dormant_days >= 730);
    }

    #[test]
    fn test_ancient_star_serialization() {
        let star = AncientStar {
            path: "ancient.rs".to_string(),
            age_days: 2000,
            dormant_days: 1500,
            star_count: 10,
            is_core: false,
        };
        let json = serde_json::to_string(&star).unwrap();
        let parsed: AncientStar = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.dormant_days, 1500);
    }

    // =========================================================================
    // Supernova Tests
    // =========================================================================

    #[test]
    fn test_supernova_creation() {
        let nova = Supernova {
            path: "exploding.rs".to_string(),
            observations_30d: 45,
            observer_count: 5,
            lines_changed: 2000,
            warning: "Extreme activity detected".to_string(),
        };
        assert_eq!(nova.path, "exploding.rs");
        assert!(nova.observations_30d > 30);
    }

    #[test]
    fn test_supernova_serialization() {
        let nova = Supernova {
            path: "nova.rs".to_string(),
            observations_30d: 50,
            observer_count: 3,
            lines_changed: 1000,
            warning: "Warning".to_string(),
        };
        let json = serde_json::to_string(&nova).unwrap();
        let parsed: Supernova = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.observations_30d, 50);
    }
}
