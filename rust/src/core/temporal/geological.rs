//! Geological Lens - Tectonic Analysis
//!
//! This module provides the geological lens for identifying high-risk areas
//! through temporal-spatial correlation:
//!
//! - **Tectonic Shifts**: High churn + high complexity = high risk
//! - **Ancient Stars**: Dormant but core files that may need attention
//! - **Supernovas**: Destabilizing refactors in progress

#[cfg(test)]
use super::metrics::{AgeClassification, ChurnClassification};
use super::metrics::{AncientStar, FileChurn, Supernova, TectonicShift};
use std::collections::HashMap;

// =============================================================================
// Thresholds
// =============================================================================

/// Minimum churn for tectonic classification (observations in 90 days)
pub const TECTONIC_CHURN_THRESHOLD: usize = 10;

/// Minimum dark matter ratio for tectonic classification
pub const TECTONIC_DARK_MATTER_THRESHOLD: f64 = 0.20;

/// Days without observation to be considered dormant for ancient detection
pub const ANCIENT_DORMANT_DAYS: u64 = 730; // 2 years

/// Minimum star count to be considered "core" for ancient stars
pub const ANCIENT_CORE_STAR_THRESHOLD: usize = 5;

/// Observations in 30 days to trigger supernova alert
pub const SUPERNOVA_THRESHOLD: usize = 30;

// =============================================================================
// Geological Analyzer
// =============================================================================

/// The Geological Analyzer - correlates temporal and spatial data
pub struct GeologicalAnalyzer {
    /// Tectonic churn threshold
    pub tectonic_churn: usize,
    /// Tectonic dark matter threshold
    pub tectonic_dark_matter: f64,
    /// Ancient dormant days threshold
    pub ancient_dormant_days: u64,
    /// Core star threshold
    pub core_star_threshold: usize,
    /// Supernova threshold
    pub supernova_threshold: usize,
}

impl Default for GeologicalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl GeologicalAnalyzer {
    /// Create a new analyzer with default thresholds
    pub fn new() -> Self {
        Self {
            tectonic_churn: TECTONIC_CHURN_THRESHOLD,
            tectonic_dark_matter: TECTONIC_DARK_MATTER_THRESHOLD,
            ancient_dormant_days: ANCIENT_DORMANT_DAYS,
            core_star_threshold: ANCIENT_CORE_STAR_THRESHOLD,
            supernova_threshold: SUPERNOVA_THRESHOLD,
        }
    }

    /// Identify tectonic shifts (high churn + high complexity)
    pub fn identify_tectonic_shifts(
        &self,
        file_churn: &HashMap<String, FileChurn>,
        dark_matter_ratios: &HashMap<String, f64>,
    ) -> Vec<TectonicShift> {
        let mut shifts = Vec::new();

        for (path, churn) in file_churn {
            let dark_matter = dark_matter_ratios.get(path).copied().unwrap_or(0.0);

            // Check tectonic criteria
            if churn.churn_90d >= self.tectonic_churn && dark_matter >= self.tectonic_dark_matter {
                // Calculate risk score (0.0 - 1.0)
                let churn_factor =
                    (churn.churn_90d as f64 / (self.tectonic_churn as f64 * 3.0)).min(1.0);
                let dark_factor = (dark_matter / (self.tectonic_dark_matter * 2.0)).min(1.0);
                let risk_score = (churn_factor + dark_factor) / 2.0;

                shifts.push(TectonicShift {
                    path: path.clone(),
                    churn_90d: churn.churn_90d,
                    dark_matter_ratio: dark_matter,
                    risk_score,
                    reason: format!(
                        "Tectonic activity: {} observations in 90 days with {:.0}% dark matter",
                        churn.churn_90d,
                        dark_matter * 100.0
                    ),
                });
            }
        }

        // Sort by risk score (highest first)
        shifts.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        shifts
    }

    /// Identify ancient stars (dormant but potentially core)
    pub fn identify_ancient_stars(
        &self,
        file_churn: &HashMap<String, FileChurn>,
        star_counts: &HashMap<String, usize>,
    ) -> Vec<AncientStar> {
        let mut ancient = Vec::new();

        for (path, churn) in file_churn {
            // Check if dormant (no activity in threshold days)
            let dormant_days = churn
                .last_observation
                .map(|t| (chrono::Utc::now() - t).num_days().max(0) as u64)
                .unwrap_or(churn.age_days);

            if dormant_days >= self.ancient_dormant_days {
                let star_count = star_counts.get(path).copied().unwrap_or(0);
                let is_core = star_count >= self.core_star_threshold;

                ancient.push(AncientStar {
                    path: path.clone(),
                    age_days: churn.age_days,
                    dormant_days,
                    star_count,
                    is_core,
                });
            }
        }

        // Sort by importance (core files first, then by star count)
        ancient.sort_by(|a, b| match (a.is_core, b.is_core) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.star_count.cmp(&a.star_count),
        });

        ancient
    }

    /// Identify supernovas (extreme recent activity)
    pub fn identify_supernovas(&self, file_churn: &HashMap<String, FileChurn>) -> Vec<Supernova> {
        let mut novas = Vec::new();

        for (path, churn) in file_churn {
            if churn.churn_30d >= self.supernova_threshold {
                novas.push(Supernova {
                    path: path.clone(),
                    observations_30d: churn.churn_30d,
                    observer_count: 0, // Would need additional data
                    lines_changed: 0,  // Would need additional data
                    warning: format!(
                        "Supernova detected: {} observations in 30 days - potential destabilizing refactor",
                        churn.churn_30d
                    ),
                });
            }
        }

        // Sort by activity (highest first)
        novas.sort_by(|a, b| b.observations_30d.cmp(&a.observations_30d));

        novas
    }

    /// Generate a geological summary
    pub fn summarize(
        &self,
        file_churn: &HashMap<String, FileChurn>,
        dark_matter_ratios: &HashMap<String, f64>,
        star_counts: &HashMap<String, usize>,
    ) -> GeologicalSummary {
        let tectonic = self.identify_tectonic_shifts(file_churn, dark_matter_ratios);
        let ancient = self.identify_ancient_stars(file_churn, star_counts);
        let supernovas = self.identify_supernovas(file_churn);

        // Calculate overall geological activity level
        let activity_level = if !supernovas.is_empty() {
            GeologicalActivity::HighVolcanic
        } else if tectonic.len() > 5 {
            GeologicalActivity::TectonicStress
        } else if !tectonic.is_empty() {
            GeologicalActivity::MinorShifts
        } else {
            GeologicalActivity::Stable
        };

        GeologicalSummary {
            tectonic_shifts: tectonic,
            ancient_stars: ancient,
            supernovas,
            activity_level,
        }
    }
}

/// Overall geological activity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeologicalActivity {
    /// Stable geology - no significant shifts
    Stable,
    /// Minor tectonic shifts detected
    MinorShifts,
    /// Significant tectonic stress
    TectonicStress,
    /// High volcanic/supernova activity
    HighVolcanic,
}

impl GeologicalActivity {
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Stable => "Stable geology",
            Self::MinorShifts => "Minor tectonic shifts",
            Self::TectonicStress => "Tectonic stress detected",
            Self::HighVolcanic => "High volcanic activity",
        }
    }

    /// Get emoji indicator
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::Stable => "🌍",
            Self::MinorShifts => "🌋",
            Self::TectonicStress => "⚠️",
            Self::HighVolcanic => "🔥",
        }
    }
}

/// Complete geological summary
#[derive(Debug, Clone)]
pub struct GeologicalSummary {
    /// Identified tectonic shifts
    pub tectonic_shifts: Vec<TectonicShift>,
    /// Identified ancient stars
    pub ancient_stars: Vec<AncientStar>,
    /// Identified supernovas
    pub supernovas: Vec<Supernova>,
    /// Overall activity level
    pub activity_level: GeologicalActivity,
}

impl GeologicalSummary {
    /// Count total risk indicators
    pub fn risk_count(&self) -> usize {
        self.tectonic_shifts.len() + self.supernovas.len()
    }

    /// Get high-risk files (tectonic + supernovas)
    pub fn high_risk_files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = self
            .tectonic_shifts
            .iter()
            .map(|t| t.path.as_str())
            .chain(self.supernovas.iter().map(|s| s.path.as_str()))
            .collect();
        files.dedup();
        files
    }

    /// Check if there are any ancient core files
    pub fn has_ancient_core_files(&self) -> bool {
        self.ancient_stars.iter().any(|a| a.is_core)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_file_churn(path: &str, churn_30d: usize, churn_90d: usize, age_days: u64) -> FileChurn {
        FileChurn {
            path: path.to_string(),
            churn_30d,
            churn_90d,
            age_days,
            last_observation: Some(Utc::now() - chrono::Duration::days(age_days as i64 / 10)),
            churn_classification: ChurnClassification::from_counts(churn_30d, churn_90d),
            age_classification: AgeClassification::from_days(age_days),
        }
    }

    // =========================================================================
    // GeologicalAnalyzer Tests
    // =========================================================================

    #[test]
    fn test_geological_analyzer_new() {
        let analyzer = GeologicalAnalyzer::new();
        assert_eq!(analyzer.tectonic_churn, TECTONIC_CHURN_THRESHOLD);
        assert_eq!(
            analyzer.tectonic_dark_matter,
            TECTONIC_DARK_MATTER_THRESHOLD
        );
        assert_eq!(analyzer.ancient_dormant_days, ANCIENT_DORMANT_DAYS);
        assert_eq!(analyzer.core_star_threshold, ANCIENT_CORE_STAR_THRESHOLD);
        assert_eq!(analyzer.supernova_threshold, SUPERNOVA_THRESHOLD);
    }

    #[test]
    fn test_geological_analyzer_default() {
        let analyzer = GeologicalAnalyzer::default();
        assert_eq!(analyzer.tectonic_churn, 10);
        assert_eq!(analyzer.ancient_dormant_days, 730);
    }

    #[test]
    fn test_identify_tectonic_shifts() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert(
            "risky.rs".to_string(),
            make_file_churn("risky.rs", 5, 15, 100),
        );
        file_churn.insert("safe.rs".to_string(), make_file_churn("safe.rs", 1, 3, 200));

        let mut dark_matter = HashMap::new();
        dark_matter.insert("risky.rs".to_string(), 0.25);
        dark_matter.insert("safe.rs".to_string(), 0.05);

        let shifts = analyzer.identify_tectonic_shifts(&file_churn, &dark_matter);

        assert_eq!(shifts.len(), 1);
        assert_eq!(shifts[0].path, "risky.rs");
    }

    #[test]
    fn test_identify_tectonic_shifts_none() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert(
            "low_churn.rs".to_string(),
            make_file_churn("low_churn.rs", 1, 5, 100),
        );
        file_churn.insert(
            "low_complexity.rs".to_string(),
            make_file_churn("low_complexity.rs", 5, 15, 100),
        );

        let mut dark_matter = HashMap::new();
        dark_matter.insert("low_churn.rs".to_string(), 0.30);
        dark_matter.insert("low_complexity.rs".to_string(), 0.10);

        let shifts = analyzer.identify_tectonic_shifts(&file_churn, &dark_matter);
        assert!(shifts.is_empty());
    }

    #[test]
    fn test_identify_tectonic_shifts_sorting() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert(
            "medium.rs".to_string(),
            make_file_churn("medium.rs", 5, 15, 100),
        );
        file_churn.insert(
            "high.rs".to_string(),
            make_file_churn("high.rs", 10, 25, 100),
        );

        let mut dark_matter = HashMap::new();
        dark_matter.insert("medium.rs".to_string(), 0.25);
        dark_matter.insert("high.rs".to_string(), 0.40);

        let shifts = analyzer.identify_tectonic_shifts(&file_churn, &dark_matter);

        assert_eq!(shifts.len(), 2);
        // Should be sorted by risk score (highest first)
        assert!(shifts[0].risk_score >= shifts[1].risk_score);
    }

    #[test]
    fn test_identify_ancient_stars() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        // Ancient dormant file
        let mut ancient = make_file_churn("ancient.rs", 0, 0, 1000);
        ancient.last_observation = Some(Utc::now() - chrono::Duration::days(800));
        file_churn.insert("ancient.rs".to_string(), ancient);

        // Recently active file
        file_churn.insert(
            "active.rs".to_string(),
            make_file_churn("active.rs", 2, 5, 500),
        );

        let mut star_counts = HashMap::new();
        star_counts.insert("ancient.rs".to_string(), 10);
        star_counts.insert("active.rs".to_string(), 5);

        let ancient_stars = analyzer.identify_ancient_stars(&file_churn, &star_counts);

        assert_eq!(ancient_stars.len(), 1);
        assert_eq!(ancient_stars[0].path, "ancient.rs");
        assert!(ancient_stars[0].is_core);
    }

    #[test]
    fn test_identify_ancient_stars_not_core() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        let mut ancient = make_file_churn("small_ancient.rs", 0, 0, 1000);
        ancient.last_observation = Some(Utc::now() - chrono::Duration::days(800));
        file_churn.insert("small_ancient.rs".to_string(), ancient);

        let mut star_counts = HashMap::new();
        star_counts.insert("small_ancient.rs".to_string(), 2); // Below core threshold

        let ancient_stars = analyzer.identify_ancient_stars(&file_churn, &star_counts);

        assert_eq!(ancient_stars.len(), 1);
        assert!(!ancient_stars[0].is_core);
    }

    #[test]
    fn test_identify_ancient_stars_sorting() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();

        let mut ancient1 = make_file_churn("core.rs", 0, 0, 1000);
        ancient1.last_observation = Some(Utc::now() - chrono::Duration::days(800));
        file_churn.insert("core.rs".to_string(), ancient1);

        let mut ancient2 = make_file_churn("small.rs", 0, 0, 1000);
        ancient2.last_observation = Some(Utc::now() - chrono::Duration::days(900));
        file_churn.insert("small.rs".to_string(), ancient2);

        let mut star_counts = HashMap::new();
        star_counts.insert("core.rs".to_string(), 10); // Core
        star_counts.insert("small.rs".to_string(), 2); // Not core

        let ancient_stars = analyzer.identify_ancient_stars(&file_churn, &star_counts);

        assert_eq!(ancient_stars.len(), 2);
        // Core files should come first
        assert!(ancient_stars[0].is_core);
        assert!(!ancient_stars[1].is_core);
    }

    #[test]
    fn test_identify_supernovas() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert(
            "exploding.rs".to_string(),
            make_file_churn("exploding.rs", 35, 50, 100),
        );
        file_churn.insert(
            "calm.rs".to_string(),
            make_file_churn("calm.rs", 5, 10, 200),
        );

        let supernovas = analyzer.identify_supernovas(&file_churn);

        assert_eq!(supernovas.len(), 1);
        assert_eq!(supernovas[0].path, "exploding.rs");
    }

    #[test]
    fn test_identify_supernovas_sorting() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert("hot.rs".to_string(), make_file_churn("hot.rs", 40, 60, 100));
        file_churn.insert(
            "hotter.rs".to_string(),
            make_file_churn("hotter.rs", 50, 70, 100),
        );

        let supernovas = analyzer.identify_supernovas(&file_churn);

        assert_eq!(supernovas.len(), 2);
        // Should be sorted by observations_30d (highest first)
        assert!(supernovas[0].observations_30d >= supernovas[1].observations_30d);
    }

    #[test]
    fn test_summarize_stable() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert(
            "stable.rs".to_string(),
            make_file_churn("stable.rs", 2, 5, 200),
        );

        let dark_matter = HashMap::new();
        let star_counts = HashMap::new();

        let summary = analyzer.summarize(&file_churn, &dark_matter, &star_counts);

        assert_eq!(summary.activity_level, GeologicalActivity::Stable);
    }

    #[test]
    fn test_summarize_volcanic() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        file_churn.insert(
            "exploding.rs".to_string(),
            make_file_churn("exploding.rs", 35, 50, 100),
        );

        let summary = analyzer.summarize(&file_churn, &HashMap::new(), &HashMap::new());

        assert_eq!(summary.activity_level, GeologicalActivity::HighVolcanic);
    }

    #[test]
    fn test_summarize_tectonic_stress() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        let mut dark_matter = HashMap::new();

        // Add more than 5 tectonic files
        for i in 0..7 {
            let name = format!("risky{}.rs", i);
            file_churn.insert(name.clone(), make_file_churn(&name, 5, 15, 100));
            dark_matter.insert(name, 0.30);
        }

        let summary = analyzer.summarize(&file_churn, &dark_matter, &HashMap::new());

        assert_eq!(summary.activity_level, GeologicalActivity::TectonicStress);
    }

    #[test]
    fn test_summarize_minor_shifts() {
        let analyzer = GeologicalAnalyzer::new();

        let mut file_churn = HashMap::new();
        let mut dark_matter = HashMap::new();

        // Add a few tectonic files (less than 5)
        for i in 0..3 {
            let name = format!("risky{}.rs", i);
            file_churn.insert(name.clone(), make_file_churn(&name, 5, 15, 100));
            dark_matter.insert(name, 0.30);
        }

        let summary = analyzer.summarize(&file_churn, &dark_matter, &HashMap::new());

        assert_eq!(summary.activity_level, GeologicalActivity::MinorShifts);
    }

    // =========================================================================
    // GeologicalActivity Tests
    // =========================================================================

    #[test]
    fn test_geological_activity_levels() {
        assert_eq!(GeologicalActivity::Stable.description(), "Stable geology");
        assert_eq!(GeologicalActivity::HighVolcanic.indicator(), "🔥");
    }

    #[test]
    fn test_geological_activity_descriptions() {
        assert_eq!(GeologicalActivity::Stable.description(), "Stable geology");
        assert_eq!(
            GeologicalActivity::MinorShifts.description(),
            "Minor tectonic shifts"
        );
        assert_eq!(
            GeologicalActivity::TectonicStress.description(),
            "Tectonic stress detected"
        );
        assert_eq!(
            GeologicalActivity::HighVolcanic.description(),
            "High volcanic activity"
        );
    }

    #[test]
    fn test_geological_activity_indicators() {
        assert_eq!(GeologicalActivity::Stable.indicator(), "🌍");
        assert_eq!(GeologicalActivity::MinorShifts.indicator(), "🌋");
        assert_eq!(GeologicalActivity::TectonicStress.indicator(), "⚠️");
        assert_eq!(GeologicalActivity::HighVolcanic.indicator(), "🔥");
    }

    #[test]
    fn test_geological_activity_equality() {
        assert_eq!(GeologicalActivity::Stable, GeologicalActivity::Stable);
        assert_ne!(GeologicalActivity::Stable, GeologicalActivity::HighVolcanic);
    }

    // =========================================================================
    // GeologicalSummary Tests
    // =========================================================================

    #[test]
    fn test_geological_summary_risk_count() {
        let summary = GeologicalSummary {
            tectonic_shifts: vec![
                TectonicShift {
                    path: "a.rs".to_string(),
                    churn_90d: 15,
                    dark_matter_ratio: 0.3,
                    risk_score: 0.5,
                    reason: "Test".to_string(),
                },
                TectonicShift {
                    path: "b.rs".to_string(),
                    churn_90d: 20,
                    dark_matter_ratio: 0.4,
                    risk_score: 0.6,
                    reason: "Test".to_string(),
                },
            ],
            ancient_stars: vec![],
            supernovas: vec![Supernova {
                path: "c.rs".to_string(),
                observations_30d: 35,
                observer_count: 3,
                lines_changed: 1000,
                warning: "Test".to_string(),
            }],
            activity_level: GeologicalActivity::TectonicStress,
        };

        assert_eq!(summary.risk_count(), 3);
    }

    #[test]
    fn test_geological_summary_high_risk_files() {
        let summary = GeologicalSummary {
            tectonic_shifts: vec![TectonicShift {
                path: "tectonic.rs".to_string(),
                churn_90d: 15,
                dark_matter_ratio: 0.3,
                risk_score: 0.5,
                reason: "Test".to_string(),
            }],
            ancient_stars: vec![],
            supernovas: vec![Supernova {
                path: "supernova.rs".to_string(),
                observations_30d: 35,
                observer_count: 3,
                lines_changed: 1000,
                warning: "Test".to_string(),
            }],
            activity_level: GeologicalActivity::HighVolcanic,
        };

        let high_risk = summary.high_risk_files();
        assert_eq!(high_risk.len(), 2);
        assert!(high_risk.contains(&"tectonic.rs"));
        assert!(high_risk.contains(&"supernova.rs"));
    }

    #[test]
    fn test_geological_summary_has_ancient_core_files() {
        let summary_with_core = GeologicalSummary {
            tectonic_shifts: vec![],
            ancient_stars: vec![AncientStar {
                path: "core.rs".to_string(),
                age_days: 1000,
                dormant_days: 800,
                star_count: 10,
                is_core: true,
            }],
            supernovas: vec![],
            activity_level: GeologicalActivity::Stable,
        };
        assert!(summary_with_core.has_ancient_core_files());

        let summary_no_core = GeologicalSummary {
            tectonic_shifts: vec![],
            ancient_stars: vec![AncientStar {
                path: "small.rs".to_string(),
                age_days: 1000,
                dormant_days: 800,
                star_count: 2,
                is_core: false,
            }],
            supernovas: vec![],
            activity_level: GeologicalActivity::Stable,
        };
        assert!(!summary_no_core.has_ancient_core_files());
    }

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn test_constants() {
        assert_eq!(TECTONIC_CHURN_THRESHOLD, 10);
        assert_eq!(TECTONIC_DARK_MATTER_THRESHOLD, 0.20);
        assert_eq!(ANCIENT_DORMANT_DAYS, 730);
        assert_eq!(ANCIENT_CORE_STAR_THRESHOLD, 5);
        assert_eq!(SUPERNOVA_THRESHOLD, 30);
    }
}
