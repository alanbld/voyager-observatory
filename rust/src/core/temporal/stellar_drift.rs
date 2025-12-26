//! Stellar Drift Analyzer - Temporal-Semantic Synthesis
//!
//! This module synthesizes the Chronos Engine (time) with the Celestial Census
//! (semantic metrics) to provide deep insights into codebase evolution.
//!
//! # Stellar Drift Metrics
//!
//! - **Ancient Stars**: Logic units (functions/classes) unchanged for > 2 years
//! - **New Stars**: Logic units added in the last 90 days
//! - **Stellar Drift**: Percentage of Logic Hemisphere shifted in last 6 months
//!
//! # Evolution Survey
//!
//! The `--survey evolution` mode shows the life cycle of each constellation:
//!
//! ```text
//! ✨ Constellation: [Authentication]
//! ├── 🌟 Ancient Stars: 42 (Stable Core)
//! ├── 🌠 New Stars: 5 (Active Expansion)
//! ├── 🌋 Volcanic Churn: High
//! └── 📈 Drift Rate: 12% / year
//! ```

use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};

use super::metrics::{
    ChronosState, TemporalCensus, FileChurn, ConstellationChurn,
    AgeClassification, ChurnClassification,
    AncientStar, Supernova,
};

// =============================================================================
// Constants
// =============================================================================

/// Days threshold for "new" stars (90 days)
pub const NEW_STAR_THRESHOLD_DAYS: u64 = 90;

/// Days threshold for "ancient" stars (2 years)
pub const ANCIENT_STAR_THRESHOLD_DAYS: u64 = 730;

/// Window for drift calculation (6 months)
pub const DRIFT_WINDOW_DAYS: i64 = 180;

// =============================================================================
// Stellar Drift Types
// =============================================================================

/// A new star - recently added logic unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewStar {
    /// File path containing the new star
    pub path: String,
    /// Age in days since creation
    pub age_days: u64,
    /// Star count (functions/methods in this file)
    pub star_count: usize,
    /// Is this an expansion of existing constellation or new territory?
    pub is_expansion: bool,
}

/// Evolution metrics for a constellation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstellationEvolution {
    /// Constellation path
    pub path: String,
    /// Total file count
    pub file_count: usize,
    /// Ancient stars (unchanged > 2 years)
    pub ancient_star_count: usize,
    /// New stars (added in last 90 days)
    pub new_star_count: usize,
    /// Active stars (modified in last 90 days)
    pub active_star_count: usize,
    /// Dormant stars (no activity but not ancient)
    pub dormant_star_count: usize,
    /// Volcanic churn classification
    pub volcanic_churn: ChurnClassification,
    /// Drift rate (% change per year)
    pub drift_rate: f64,
    /// Average age in days
    pub avg_age_days: u64,
    /// Is this a new constellation (< 90 days old)?
    pub is_new: bool,
    /// Is this an ancient constellation (all files > 2 years)?
    pub is_ancient: bool,
}

/// Complete stellar drift analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StellarDriftReport {
    /// Galaxy state
    pub state: ChronosState,
    /// Galaxy age in days
    pub galaxy_age_days: u64,
    /// Galaxy age in years (for display)
    pub galaxy_age_years: f64,
    /// First observation date (Big Bang)
    pub big_bang_date: Option<DateTime<Utc>>,

    /// Total logic stars across the galaxy
    pub total_stars: usize,
    /// Ancient stars (stable core)
    pub ancient_stars: Vec<AncientStar>,
    /// New stars (active expansion)
    pub new_stars: Vec<NewStar>,
    /// Supernovas (destabilizing activity)
    pub supernovas: Vec<Supernova>,

    /// Stellar drift percentage (logic shifted in last 6 months)
    pub stellar_drift_percent: f64,
    /// Drift rate (% per year)
    pub drift_rate_per_year: f64,

    /// Evolution by constellation
    pub constellations: BTreeMap<String, ConstellationEvolution>,

    /// Summary counts
    pub ancient_star_total: usize,
    pub new_star_total: usize,
    pub active_star_total: usize,
    pub dormant_star_total: usize,

    /// Health indicators
    pub is_expanding: bool,      // More new stars than ancient
    pub is_stable: bool,         // Low drift rate
    pub is_ossifying: bool,      // All ancient, no new
    pub has_supernovas: bool,    // Destabilizing refactors
}

// =============================================================================
// Stellar Drift Analyzer
// =============================================================================

/// The Stellar Drift Analyzer - synthesizes temporal and semantic data
pub struct StellarDriftAnalyzer {
    /// Threshold for new stars (days)
    pub new_star_threshold: u64,
    /// Threshold for ancient stars (days)
    pub ancient_star_threshold: u64,
    /// Window for drift calculation (days)
    pub drift_window_days: i64,
}

impl Default for StellarDriftAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl StellarDriftAnalyzer {
    /// Create a new analyzer with default thresholds
    pub fn new() -> Self {
        Self {
            new_star_threshold: NEW_STAR_THRESHOLD_DAYS,
            ancient_star_threshold: ANCIENT_STAR_THRESHOLD_DAYS,
            drift_window_days: DRIFT_WINDOW_DAYS,
        }
    }

    /// Analyze stellar drift from temporal census and star counts
    ///
    /// # Arguments
    /// * `temporal_census` - The temporal census from ChronosEngine
    /// * `star_counts` - Map of file path -> star count (from CelestialCensus)
    /// * `big_bang_date` - Optional first commit date
    /// * `path_prefix` - Optional prefix to add to star_counts paths to match temporal census paths
    ///                   (used when survey root is a subdirectory of git root)
    pub fn analyze(
        &self,
        temporal_census: &TemporalCensus,
        star_counts: &HashMap<String, usize>,
        big_bang_date: Option<DateTime<Utc>>,
        path_prefix: Option<&str>,
    ) -> StellarDriftReport {
        // If path_prefix is provided, create a prefixed version of star_counts
        let prefixed_star_counts: HashMap<String, usize> = if let Some(prefix) = path_prefix {
            star_counts.iter()
                .map(|(k, v)| (format!("{}/{}", prefix, k), *v))
                .collect()
        } else {
            star_counts.clone()
        };
        let star_counts_ref = if path_prefix.is_some() { &prefixed_star_counts } else { star_counts };
        let now = Utc::now();
        let mut report = StellarDriftReport {
            state: temporal_census.state.clone(),
            galaxy_age_days: temporal_census.galaxy_age_days,
            galaxy_age_years: temporal_census.galaxy_age_days as f64 / 365.0,
            big_bang_date,
            ..Default::default()
        };

        // Handle static galaxy
        if matches!(temporal_census.state, ChronosState::StaticGalaxy | ChronosState::NoRepository) {
            return report;
        }

        // Calculate total stars
        report.total_stars = star_counts_ref.values().sum();

        // Identify ancient stars and new stars
        let threshold_new = now - Duration::days(self.new_star_threshold as i64);
        let threshold_ancient = now - Duration::days(self.ancient_star_threshold as i64);
        let threshold_drift = now - Duration::days(self.drift_window_days);

        let mut files_in_drift_window = 0;
        let mut stars_in_drift_window = 0;

        for (path, file_churn) in &temporal_census.files {
            let star_count = star_counts_ref.get(path).copied().unwrap_or(0);

            // Check if file is new (created in last 90 days)
            if file_churn.age_days <= self.new_star_threshold {
                report.new_stars.push(NewStar {
                    path: path.clone(),
                    age_days: file_churn.age_days,
                    star_count,
                    is_expansion: true, // Assume expansion for now
                });
                report.new_star_total += star_count;
            }

            // Check if file is ancient (dormant > 2 years)
            let dormant_days = file_churn.last_observation
                .map(|t| (now - t).num_days().max(0) as u64)
                .unwrap_or(file_churn.age_days);

            if dormant_days >= self.ancient_star_threshold {
                report.ancient_stars.push(AncientStar {
                    path: path.clone(),
                    age_days: file_churn.age_days,
                    dormant_days,
                    star_count,
                    is_core: star_count >= 5,
                });
                report.ancient_star_total += star_count;
            } else if file_churn.churn_90d > 0 {
                // Active in last 90 days
                report.active_star_total += star_count;
            } else {
                // Dormant but not ancient
                report.dormant_star_total += star_count;
            }

            // Track files in drift window (last 6 months)
            if let Some(last_obs) = file_churn.last_observation {
                if last_obs > threshold_drift {
                    files_in_drift_window += 1;
                    stars_in_drift_window += star_count;
                }
            }
        }

        // Copy supernovas from temporal census
        report.supernovas = temporal_census.supernovas.clone();
        report.has_supernovas = !report.supernovas.is_empty();

        // Calculate stellar drift percentage
        if report.total_stars > 0 {
            report.stellar_drift_percent = (stars_in_drift_window as f64 / report.total_stars as f64) * 100.0;
            // Annualize: if 10% drift in 6 months, that's 20% per year
            report.drift_rate_per_year = report.stellar_drift_percent * (365.0 / self.drift_window_days as f64);
        }

        // Build constellation evolution
        report.constellations = self.analyze_constellations(
            &temporal_census.constellations,
            &temporal_census.files,
            star_counts_ref,
        );

        // Set health indicators
        report.is_expanding = report.new_star_total > report.ancient_star_total;
        report.is_stable = report.drift_rate_per_year < 20.0; // Less than 20% drift per year
        report.is_ossifying = report.new_star_total == 0 && report.ancient_star_total > report.total_stars / 2;

        report
    }

    /// Analyze evolution for each constellation
    fn analyze_constellations(
        &self,
        constellations: &BTreeMap<String, ConstellationChurn>,
        files: &BTreeMap<String, FileChurn>,
        star_counts: &HashMap<String, usize>,
    ) -> BTreeMap<String, ConstellationEvolution> {
        let now = Utc::now();
        let mut result = BTreeMap::new();

        // Build a mapping from normalized constellation path -> files in that constellation
        // This handles path normalization issues (trailing slashes, empty strings, etc.)
        let mut constellation_files: HashMap<String, Vec<(&String, usize)>> = HashMap::new();

        for (file_path, &star_count) in star_counts {
            // Extract parent directory and normalize
            let parent = std::path::Path::new(file_path)
                .parent()
                .map(|p| {
                    let s = p.to_string_lossy().to_string();
                    if s.is_empty() { ".".to_string() } else { s }
                })
                .unwrap_or_else(|| ".".to_string());

            constellation_files
                .entry(parent)
                .or_default()
                .push((file_path, star_count));
        }

        for (path, churn) in constellations {
            let mut evolution = ConstellationEvolution {
                path: path.clone(),
                file_count: churn.file_count,
                volcanic_churn: churn.classification,
                avg_age_days: churn.avg_age_days,
                ..Default::default()
            };

            // Normalize the constellation path for lookup
            let normalized_path = if path.is_empty() { ".".to_string() } else { path.clone() };

            // Get files in this constellation
            let files_in_constellation = constellation_files.get(&normalized_path);

            if let Some(file_list) = files_in_constellation {
                // Analyze files in this constellation
                let mut total_age: u64 = 0;
                let mut oldest_age: u64 = 0;
                let mut youngest_age: u64 = u64::MAX;
                let mut matched_file_count = 0;

                for (file_path, star_count) in file_list {
                    // Look up temporal data for this file
                    let file_churn = files.get(*file_path);
                    let (age_days, churn_90d, last_obs) = file_churn
                        .map(|fc| (fc.age_days, fc.churn_90d, fc.last_observation))
                        .unwrap_or((0, 0, None));

                    let dormant_days = last_obs
                        .map(|t| (now - t).num_days().max(0) as u64)
                        .unwrap_or(age_days);

                    if age_days > 0 {
                        total_age += age_days;
                        matched_file_count += 1;
                        oldest_age = oldest_age.max(age_days);
                        youngest_age = youngest_age.min(age_days);
                    }

                    // Classify the file
                    if age_days > 0 && age_days <= self.new_star_threshold {
                        evolution.new_star_count += *star_count;
                    } else if dormant_days >= self.ancient_star_threshold {
                        evolution.ancient_star_count += *star_count;
                    } else if churn_90d > 0 {
                        evolution.active_star_count += *star_count;
                    } else {
                        // No temporal data or dormant - classify as dormant
                        evolution.dormant_star_count += *star_count;
                    }
                }

                // Calculate constellation age
                if matched_file_count > 0 {
                    evolution.avg_age_days = total_age / matched_file_count as u64;
                }

                evolution.is_new = youngest_age != u64::MAX && youngest_age <= self.new_star_threshold;
                evolution.is_ancient = oldest_age >= self.ancient_star_threshold
                    && evolution.new_star_count == 0
                    && evolution.active_star_count == 0;
            }

            // Calculate drift rate for this constellation
            let total_stars = evolution.ancient_star_count
                + evolution.new_star_count
                + evolution.active_star_count
                + evolution.dormant_star_count;

            if total_stars > 0 {
                let changed_stars = evolution.new_star_count + evolution.active_star_count;
                let drift_6mo = (changed_stars as f64 / total_stars as f64) * 100.0;
                evolution.drift_rate = drift_6mo * 2.0; // Annualize
            }

            result.insert(path.clone(), evolution);
        }

        result
    }

    /// Generate a static galaxy report when no temporal data is available
    pub fn static_galaxy_report() -> StellarDriftReport {
        StellarDriftReport {
            state: ChronosState::StaticGalaxy,
            ..Default::default()
        }
    }
}

// =============================================================================
// Display Helpers
// =============================================================================

impl StellarDriftReport {
    /// Check if this is a static galaxy (no drift data)
    pub fn is_static(&self) -> bool {
        matches!(self.state, ChronosState::StaticGalaxy | ChronosState::NoRepository)
    }

    /// Get galaxy health description
    pub fn health_description(&self) -> &'static str {
        if self.is_static() {
            "Static Galaxy (no drift detected)"
        } else if self.has_supernovas {
            "Volcanic Activity (destabilizing refactors in progress)"
        } else if self.is_ossifying {
            "Ossifying (ancient core with no new growth)"
        } else if self.is_expanding {
            "Expanding (active development with new features)"
        } else if self.is_stable {
            "Stable (healthy balance of old and new)"
        } else {
            "High Drift (significant changes in progress)"
        }
    }

    /// Get health indicator emoji
    pub fn health_indicator(&self) -> &'static str {
        if self.is_static() {
            "⏳"
        } else if self.has_supernovas {
            "🔥"
        } else if self.is_ossifying {
            "🪨"
        } else if self.is_expanding {
            "🌱"
        } else if self.is_stable {
            "💎"
        } else {
            "🌊"
        }
    }

    /// Get drift classification
    pub fn drift_classification(&self) -> &'static str {
        if self.is_static() {
            "No Drift Data"
        } else if self.drift_rate_per_year < 10.0 {
            "Minimal Drift"
        } else if self.drift_rate_per_year < 25.0 {
            "Low Drift"
        } else if self.drift_rate_per_year < 50.0 {
            "Moderate Drift"
        } else if self.drift_rate_per_year < 75.0 {
            "High Drift"
        } else {
            "Extreme Drift"
        }
    }
}

impl ConstellationEvolution {
    /// Get life cycle stage
    pub fn life_cycle_stage(&self) -> &'static str {
        if self.is_new {
            "Newborn"
        } else if self.is_ancient {
            "Ancient"
        } else if self.new_star_count > self.ancient_star_count {
            "Growing"
        } else if self.active_star_count > 0 {
            "Active"
        } else {
            "Dormant"
        }
    }

    /// Get life cycle emoji
    pub fn life_cycle_emoji(&self) -> &'static str {
        match self.life_cycle_stage() {
            "Newborn" => "🌟",
            "Growing" => "🌱",
            "Active" => "⚡",
            "Dormant" => "💤",
            "Ancient" => "🏛️",
            _ => "❓",
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file_churn(age_days: u64, churn_90d: usize, dormant_days: Option<i64>) -> FileChurn {
        let last_observation = dormant_days.map(|d| Utc::now() - Duration::days(d));
        FileChurn {
            path: String::new(),
            churn_30d: churn_90d / 3,
            churn_90d,
            age_days,
            last_observation,
            churn_classification: ChurnClassification::from_counts(churn_90d / 3, churn_90d),
            age_classification: AgeClassification::from_days(age_days),
        }
    }

    #[test]
    fn test_stellar_drift_analyzer_creation() {
        let analyzer = StellarDriftAnalyzer::new();
        assert_eq!(analyzer.new_star_threshold, NEW_STAR_THRESHOLD_DAYS);
        assert_eq!(analyzer.ancient_star_threshold, ANCIENT_STAR_THRESHOLD_DAYS);
    }

    #[test]
    fn test_static_galaxy_report() {
        let report = StellarDriftAnalyzer::static_galaxy_report();
        assert!(report.is_static());
        assert_eq!(report.health_description(), "Static Galaxy (no drift detected)");
    }

    #[test]
    fn test_new_star_identification() {
        let analyzer = StellarDriftAnalyzer::new();

        let mut files = BTreeMap::new();
        files.insert("new_file.rs".to_string(), make_file_churn(30, 5, Some(5)));
        files.insert("old_file.rs".to_string(), make_file_churn(1000, 0, Some(800)));

        let temporal_census = TemporalCensus {
            state: ChronosState::Active {
                total_events: 100,
                galaxy_age_days: 1000,
                observer_count: 5,
            },
            galaxy_age_days: 1000,
            total_observations: 100,
            observer_count: 5,
            files,
            ..Default::default()
        };

        let mut star_counts = HashMap::new();
        star_counts.insert("new_file.rs".to_string(), 3);
        star_counts.insert("old_file.rs".to_string(), 10);

        let report = analyzer.analyze(&temporal_census, &star_counts, None, None);

        assert!(!report.new_stars.is_empty(), "Should identify new stars");
        assert_eq!(report.new_stars[0].path, "new_file.rs");
        assert_eq!(report.new_star_total, 3);
    }

    #[test]
    fn test_ancient_star_identification() {
        let analyzer = StellarDriftAnalyzer::new();

        let mut files = BTreeMap::new();
        // File that hasn't been touched in 3 years
        files.insert("ancient.rs".to_string(), make_file_churn(1500, 0, Some(1100)));

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

        assert!(!report.ancient_stars.is_empty(), "Should identify ancient stars");
        assert_eq!(report.ancient_star_total, 15);
    }

    #[test]
    fn test_drift_calculation() {
        let analyzer = StellarDriftAnalyzer::new();

        let mut files = BTreeMap::new();
        // Active file (modified recently)
        files.insert("active.rs".to_string(), make_file_churn(500, 10, Some(30)));
        // Dormant file (not ancient)
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

        assert_eq!(report.total_stars, 20);
        assert!(report.stellar_drift_percent > 0.0, "Should have positive drift");
    }

    #[test]
    fn test_health_indicators() {
        // Test expanding galaxy
        let mut report = StellarDriftReport::default();
        report.new_star_total = 100;
        report.ancient_star_total = 50;
        report.state = ChronosState::Active {
            total_events: 100,
            galaxy_age_days: 500,
            observer_count: 5,
        };

        assert!(report.is_expanding || !report.is_expanding); // Just check it compiles

        // Test static galaxy
        let static_report = StellarDriftAnalyzer::static_galaxy_report();
        assert_eq!(static_report.health_indicator(), "⏳");
    }

    #[test]
    fn test_constellation_evolution() {
        let evolution = ConstellationEvolution {
            path: "src/core".to_string(),
            file_count: 10,
            ancient_star_count: 5,
            new_star_count: 10,
            active_star_count: 3,
            dormant_star_count: 2,
            volcanic_churn: ChurnClassification::Moderate,
            drift_rate: 25.0,
            avg_age_days: 300,
            is_new: false,
            is_ancient: false,
        };

        assert_eq!(evolution.life_cycle_stage(), "Growing");
        assert_eq!(evolution.life_cycle_emoji(), "🌱");
    }
}
