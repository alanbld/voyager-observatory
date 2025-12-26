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

    #[test]
    fn test_age_classification() {
        assert_eq!(AgeClassification::from_days(15), AgeClassification::Newborn);
        assert_eq!(AgeClassification::from_days(100), AgeClassification::Young);
        assert_eq!(AgeClassification::from_days(500), AgeClassification::Mature);
        assert_eq!(AgeClassification::from_days(800), AgeClassification::Ancient);
    }

    #[test]
    fn test_churn_classification() {
        assert_eq!(ChurnClassification::from_counts(0, 0), ChurnClassification::Dormant);
        assert_eq!(ChurnClassification::from_counts(1, 2), ChurnClassification::Low);
        assert_eq!(ChurnClassification::from_counts(3, 8), ChurnClassification::Moderate);
        assert_eq!(ChurnClassification::from_counts(5, 15), ChurnClassification::High);
        assert_eq!(ChurnClassification::from_counts(35, 50), ChurnClassification::Supernova);
    }

    #[test]
    fn test_chronos_state_default() {
        let state = ChronosState::default();
        assert!(matches!(state, ChronosState::StaticGalaxy));
    }

    #[test]
    fn test_temporal_census_determinism() {
        let census = TemporalCensus::default();
        // BTreeMap ensures deterministic ordering
        assert!(census.constellations.is_empty());
        assert!(census.files.is_empty());
    }
}
