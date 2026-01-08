//! Chronos Engine - Git History Extraction
//!
//! This module provides the core temporal analysis engine using git2
//! for optimized history extraction. Includes the Chronos Warp caching
//! system for near-instantaneous repeat scans.

use chrono::{DateTime, Duration, Utc};
use git2::{Commit, DiffOptions, Oid, Repository};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::metrics::{
    AgeClassification, AncientStar, ChronosMetrics, ChronosState, ChurnClassification,
    ConstellationChurn, FileChurn, Observer, ObserverImpact, StellarAge, Supernova, TectonicShift,
    TemporalCensus, VolcanicChurn,
};

use super::cache::{
    CachedGalaxyStats, CachedObservation, ChronosCache, ChronosCacheManager, WarpStatus,
};

// =============================================================================
// Constants
// =============================================================================

/// Maximum commit depth for performance (configurable)
/// Default is 1000 for "Shallow Chronos" mode - fast surveys
pub const DEFAULT_COMMIT_DEPTH: usize = 1_000;

/// Full depth for complete history analysis
pub const FULL_COMMIT_DEPTH: usize = 100_000;

/// Days to consider for "recent" activity
const CHURN_WINDOW_30D: i64 = 30;
const CHURN_WINDOW_90D: i64 = 90;
const CHURN_WINDOW_YEAR: i64 = 365;

/// Ancient star threshold (2 years)
const ANCIENT_THRESHOLD_DAYS: u64 = 730;

/// Supernova threshold (30 commits in 30 days)
const SUPERNOVA_THRESHOLD: usize = 30;

// =============================================================================
// Chronos Engine
// =============================================================================

/// The Chronos Engine - temporal analysis of code history
pub struct ChronosEngine {
    /// The git repository
    repo: Repository,
    /// Root path of the repository
    root: PathBuf,
    /// Maximum commit depth to analyze
    commit_depth: usize,
    /// Cached file histories (path -> observations)
    file_histories: HashMap<String, Vec<FileObservation>>,
    /// Galaxy-level statistics
    galaxy_stats: GalaxyStats,
    /// Engine state
    state: ChronosState,
    /// Cache manager for Chronos Warp
    cache_manager: ChronosCacheManager,
    /// Current warp status
    warp_status: WarpStatus,
}

/// A single observation (commit) affecting a file
#[derive(Debug, Clone)]
struct FileObservation {
    /// Commit timestamp
    timestamp: DateTime<Utc>,
    /// Observer (author) name
    observer_name: String,
    /// Observer email
    observer_email: String,
    /// Lines added
    lines_added: usize,
    /// Lines removed
    lines_removed: usize,
}

/// Extracted data from a commit (for borrow-checker friendly processing)
#[derive(Debug, Clone)]
struct CommitData {
    /// Commit timestamp
    timestamp: DateTime<Utc>,
    /// Observer name
    observer_name: String,
    /// Observer email
    observer_email: String,
    /// Files changed in this commit
    files_changed: Vec<String>,
}

/// Galaxy-level statistics
#[derive(Debug, Clone, Default)]
struct GalaxyStats {
    /// Total observations analyzed
    total_observations: usize,
    /// Unique observers
    observers: HashMap<String, ObserverStats>,
    /// First observation timestamp
    first_observation: Option<DateTime<Utc>>,
    /// Last observation timestamp
    last_observation: Option<DateTime<Utc>>,
}

/// Stats for a single observer
#[derive(Debug, Clone, Default)]
struct ObserverStats {
    name: String,
    email: String,
    observations: usize,
    lines_added: usize,
    lines_removed: usize,
    first_seen: Option<DateTime<Utc>>,
    last_seen: Option<DateTime<Utc>>,
}

impl ChronosEngine {
    /// Create a new Chronos Engine for a repository
    pub fn new(root: &Path) -> Option<Self> {
        Self::with_depth(root, DEFAULT_COMMIT_DEPTH)
    }

    /// Create a new engine with custom commit depth
    pub fn with_depth(root: &Path, commit_depth: usize) -> Option<Self> {
        // Try to open the repository
        let repo = match Repository::discover(root) {
            Ok(r) => r,
            Err(_) => return None,
        };

        let root_path = repo.workdir()?.to_path_buf();
        let cache_manager = ChronosCacheManager::new(&root_path);

        Some(Self {
            repo,
            root: root_path.clone(),
            commit_depth,
            file_histories: HashMap::new(),
            galaxy_stats: GalaxyStats::default(),
            state: ChronosState::StaticGalaxy,
            cache_manager,
            warp_status: WarpStatus::Calibrating,
        })
    }

    /// Get the current Warp status
    pub fn warp_status(&self) -> WarpStatus {
        self.warp_status
    }

    /// Get the engine state
    pub fn state(&self) -> &ChronosState {
        &self.state
    }

    /// Extract history for all files
    pub fn extract_history(&mut self) -> Result<(), String> {
        let now = Utc::now();

        // First, collect all commit data to avoid borrow conflicts
        let (commit_data, hit_depth_limit): (Vec<CommitData>, bool) = {
            let mut revwalk = self
                .repo
                .revwalk()
                .map_err(|e| format!("Failed to create revwalk: {}", e))?;

            revwalk
                .push_head()
                .map_err(|e| format!("Failed to push HEAD: {}", e))?;

            // Take one extra to detect if we hit the limit
            let oids: Vec<Oid> = revwalk
                .take(self.commit_depth + 1)
                .filter_map(|r| r.ok())
                .collect();

            let hit_limit = oids.len() > self.commit_depth;
            let oids_to_process: Vec<Oid> = if hit_limit {
                oids.into_iter().take(self.commit_depth).collect()
            } else {
                oids
            };

            // Extract data from each commit
            let mut data = Vec::with_capacity(oids_to_process.len());
            for oid in oids_to_process {
                if let Ok(commit) = self.repo.find_commit(oid) {
                    if let Some(cd) = self.extract_commit_data(&commit) {
                        data.push(cd);
                    }
                }
            }
            (data, hit_limit)
        };

        // Process the extracted data
        let mut commit_count = 0;
        let mut first_timestamp: Option<DateTime<Utc>> = None;
        let mut last_timestamp: Option<DateTime<Utc>> = None;

        for data in commit_data {
            commit_count += 1;

            // Track timestamps
            if first_timestamp.is_none() || data.timestamp < first_timestamp.unwrap() {
                first_timestamp = Some(data.timestamp);
            }
            if last_timestamp.is_none() || data.timestamp > last_timestamp.unwrap() {
                last_timestamp = Some(data.timestamp);
            }

            // Update observer stats
            let observer_key = data.observer_email.clone();
            let observer_stats = self
                .galaxy_stats
                .observers
                .entry(observer_key)
                .or_insert_with(|| ObserverStats {
                    name: data.observer_name.clone(),
                    email: data.observer_email.clone(),
                    ..Default::default()
                });
            observer_stats.observations += 1;
            if observer_stats.first_seen.is_none()
                || data.timestamp < observer_stats.first_seen.unwrap()
            {
                observer_stats.first_seen = Some(data.timestamp);
            }
            if observer_stats.last_seen.is_none()
                || data.timestamp > observer_stats.last_seen.unwrap()
            {
                observer_stats.last_seen = Some(data.timestamp);
            }

            // Add file observations
            for path in data.files_changed {
                let observation = FileObservation {
                    timestamp: data.timestamp,
                    observer_name: data.observer_name.clone(),
                    observer_email: data.observer_email.clone(),
                    lines_added: 0,
                    lines_removed: 0,
                };

                self.file_histories
                    .entry(path)
                    .or_insert_with(Vec::new)
                    .push(observation);
            }
        }

        self.galaxy_stats.first_observation = first_timestamp;
        self.galaxy_stats.last_observation = last_timestamp;
        self.galaxy_stats.total_observations = commit_count;

        // Calculate galaxy age
        let galaxy_age_days = first_timestamp
            .map(|first| (now - first).num_days().max(0) as u64)
            .unwrap_or(0);

        // Set appropriate state based on whether we hit depth limit
        self.state = if hit_depth_limit {
            ChronosState::ShallowCensus {
                total_events: commit_count,
                galaxy_age_days,
                observer_count: self.galaxy_stats.observers.len(),
                depth_limit: self.commit_depth,
            }
        } else {
            ChronosState::Active {
                total_events: commit_count,
                galaxy_age_days,
                observer_count: self.galaxy_stats.observers.len(),
            }
        };

        Ok(())
    }

    /// Extract history with Chronos Warp caching
    ///
    /// This method first checks the cache for valid data. If the cache is valid
    /// (git HEAD hasn't changed, cache isn't expired), it loads from cache for
    /// near-instantaneous results. Otherwise, it performs full git analysis and
    /// saves to cache.
    pub fn extract_history_cached(&mut self) -> Result<(), String> {
        // Try loading from cache first
        if let Some(cache) = self.cache_manager.load() {
            // Cache hit - restore state from cache
            self.restore_from_cache(cache);
            self.warp_status = WarpStatus::Engaged;
            return Ok(());
        }

        // Cache miss - do full extraction
        self.warp_status = WarpStatus::Calibrating;
        self.extract_history()?;

        // Save to cache for next time
        if let Err(e) = self.save_to_cache() {
            // Log but don't fail - cache is optional optimization
            eprintln!("Warning: Failed to save Chronos cache: {}", e);
        }

        Ok(())
    }

    /// Restore engine state from cache
    fn restore_from_cache(&mut self, cache: ChronosCache) {
        // Convert cached observations back to FileObservation
        self.file_histories.clear();
        for (path, cached_obs) in cache.file_histories {
            let observations: Vec<FileObservation> = cached_obs
                .into_iter()
                .map(|co| FileObservation {
                    timestamp: DateTime::from_timestamp(co.timestamp_secs, 0)
                        .unwrap_or_else(Utc::now),
                    observer_name: co.observer_name,
                    observer_email: co.observer_email_hash, // Already hashed in cache
                    lines_added: co.lines_added,
                    lines_removed: co.lines_removed,
                })
                .collect();
            self.file_histories.insert(path, observations);
        }

        // Restore galaxy stats
        self.galaxy_stats.total_observations = cache.galaxy_stats.total_observations;
        self.galaxy_stats.first_observation = cache
            .galaxy_stats
            .first_observation_secs
            .and_then(|s| DateTime::from_timestamp(s, 0));
        self.galaxy_stats.last_observation = cache
            .galaxy_stats
            .last_observation_secs
            .and_then(|s| DateTime::from_timestamp(s, 0));

        // Restore state
        let now = Utc::now();
        let galaxy_age_days = self
            .galaxy_stats
            .first_observation
            .map(|first| (now - first).num_days().max(0) as u64)
            .unwrap_or(0);

        if cache.hit_depth_limit {
            self.state = ChronosState::ShallowCensus {
                total_events: cache.total_observations,
                galaxy_age_days,
                observer_count: cache.galaxy_stats.observer_count,
                depth_limit: cache.commit_depth,
            };
        } else {
            self.state = ChronosState::Active {
                total_events: cache.total_observations,
                galaxy_age_days,
                observer_count: cache.galaxy_stats.observer_count,
            };
        }
    }

    /// Save current state to cache
    fn save_to_cache(&self) -> Result<(), String> {
        // Convert file histories to cached format
        let mut cached_histories: HashMap<String, Vec<CachedObservation>> = HashMap::new();
        for (path, observations) in &self.file_histories {
            let cached: Vec<CachedObservation> = observations
                .iter()
                .map(|o| CachedObservation {
                    timestamp_secs: o.timestamp.timestamp(),
                    observer_name: o.observer_name.clone(),
                    observer_email_hash: hash_email(&o.observer_email),
                    lines_added: o.lines_added,
                    lines_removed: o.lines_removed,
                })
                .collect();
            cached_histories.insert(path.clone(), cached);
        }

        // Build galaxy stats
        let top_observers: Vec<(String, usize)> = self
            .galaxy_stats
            .observers
            .values()
            .map(|o| (o.name.clone(), o.observations))
            .collect();

        let galaxy_stats = CachedGalaxyStats {
            total_observations: self.galaxy_stats.total_observations,
            first_observation_secs: self.galaxy_stats.first_observation.map(|t| t.timestamp()),
            last_observation_secs: self.galaxy_stats.last_observation.map(|t| t.timestamp()),
            observer_count: self.galaxy_stats.observers.len(),
            top_observers,
        };

        // Determine if we hit depth limit
        let hit_depth_limit = matches!(self.state, ChronosState::ShallowCensus { .. });

        // Create and save cache
        let cache = self.cache_manager.create_cache(
            cached_histories,
            galaxy_stats,
            self.galaxy_stats.total_observations,
            hit_depth_limit,
            self.commit_depth,
        )?;

        self.cache_manager.save(&cache)
    }

    /// Invalidate the cache (force full re-analysis on next run)
    pub fn invalidate_cache(&self) -> Result<(), String> {
        self.cache_manager.invalidate()
    }

    /// Check if cache is valid without loading
    pub fn is_cache_valid(&self) -> bool {
        self.cache_manager.warp_engaged()
    }

    /// Extract data from a single commit (pure, no mutation)
    fn extract_commit_data(&self, commit: &Commit) -> Option<CommitData> {
        let timestamp = commit_timestamp(commit);
        let author = commit.author();
        let observer_name = author.name().unwrap_or("Unknown").to_string();
        let observer_email = author.email().unwrap_or("unknown@unknown").to_string();

        // Get diff with parent
        let parent = commit.parent(0).ok();
        let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());
        let commit_tree = commit.tree().ok()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(false);

        let diff = self
            .repo
            .diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&commit_tree),
                Some(&mut diff_opts),
            )
            .ok()?;

        // Collect changed files
        let mut files_changed = Vec::new();
        let _ = diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files_changed.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        );

        Some(CommitData {
            timestamp,
            observer_name,
            observer_email,
            files_changed,
        })
    }

    /// Get metrics for a specific file
    pub fn file_metrics(&self, path: &str) -> ChronosMetrics {
        let now = Utc::now();
        let normalized_path = normalize_path(path, &self.root);

        let observations = self.file_histories.get(&normalized_path);

        if let Some(obs) = observations {
            self.calculate_file_metrics(obs, &now)
        } else {
            ChronosMetrics::default()
        }
    }

    /// Calculate metrics from observations
    fn calculate_file_metrics(
        &self,
        observations: &[FileObservation],
        now: &DateTime<Utc>,
    ) -> ChronosMetrics {
        if observations.is_empty() {
            return ChronosMetrics::default();
        }

        // Find first and last observations
        let first = observations.iter().min_by_key(|o| o.timestamp);
        let last = observations.iter().max_by_key(|o| o.timestamp);

        // Calculate age
        let first_timestamp = first.map(|o| o.timestamp);
        let age_days = first_timestamp
            .map(|t| (*now - t).num_days().max(0) as u64)
            .unwrap_or(0);

        let stellar_age = StellarAge {
            first_observation: first_timestamp,
            age_days,
            classification: AgeClassification::from_days(age_days),
        };

        // Calculate churn
        let threshold_30d = *now - Duration::days(CHURN_WINDOW_30D);
        let threshold_90d = *now - Duration::days(CHURN_WINDOW_90D);
        let threshold_year = *now - Duration::days(CHURN_WINDOW_YEAR);

        let last_30_days = observations
            .iter()
            .filter(|o| o.timestamp > threshold_30d)
            .count();
        let last_90_days = observations
            .iter()
            .filter(|o| o.timestamp > threshold_90d)
            .count();
        let last_year = observations
            .iter()
            .filter(|o| o.timestamp > threshold_year)
            .count();

        let (lines_added_90d, lines_removed_90d) = observations
            .iter()
            .filter(|o| o.timestamp > threshold_90d)
            .fold((0, 0), |(a, r), o| (a + o.lines_added, r + o.lines_removed));

        let volcanic_churn = VolcanicChurn {
            last_30_days,
            last_90_days,
            last_year,
            lines_added_90d,
            lines_removed_90d,
            classification: ChurnClassification::from_counts(last_30_days, last_90_days),
        };

        // Calculate primary observers
        let mut observer_map: HashMap<String, (String, usize, usize, usize)> = HashMap::new();
        for obs in observations {
            let entry = observer_map
                .entry(obs.observer_email.clone())
                .or_insert_with(|| (obs.observer_name.clone(), 0, 0, 0));
            entry.1 += 1;
            entry.2 += obs.lines_added;
            entry.3 += obs.lines_removed;
        }

        let mut observers: Vec<Observer> = observer_map
            .into_iter()
            .map(|(email, (name, obs_count, added, removed))| Observer {
                name,
                email_hash: hash_email(&email),
                impact: ObserverImpact {
                    observations: obs_count,
                    lines_added: added,
                    lines_removed: removed,
                    net_impact: added as i64 - removed as i64,
                    first_seen: None,
                    last_seen: None,
                },
            })
            .collect();

        // Sort by impact and take top 3
        observers.sort_by(|a, b| b.impact.observations.cmp(&a.impact.observations));
        observers.truncate(3);

        ChronosMetrics {
            stellar_age,
            volcanic_churn,
            primary_observers: observers,
            last_observation: last.map(|o| o.timestamp),
            total_observations: observations.len(),
        }
    }

    /// Build a complete temporal census
    pub fn build_census(&self) -> TemporalCensus {
        let now = Utc::now();
        let mut census = TemporalCensus::default();

        census.state = self.state.clone();

        match &self.state {
            ChronosState::Active {
                total_events,
                galaxy_age_days,
                observer_count,
            } => {
                census.total_observations = *total_events;
                census.galaxy_age_days = *galaxy_age_days;
                census.observer_count = *observer_count;
            }
            _ => return census,
        }

        // Build file-level churn
        for (path, observations) in &self.file_histories {
            let metrics = self.calculate_file_metrics(observations, &now);

            let file_churn = FileChurn {
                path: path.clone(),
                churn_30d: metrics.volcanic_churn.last_30_days,
                churn_90d: metrics.volcanic_churn.last_90_days,
                age_days: metrics.stellar_age.age_days,
                last_observation: metrics.last_observation,
                churn_classification: metrics.volcanic_churn.classification,
                age_classification: metrics.stellar_age.classification,
            };

            census.files.insert(path.clone(), file_churn);

            // Identify supernovas
            if metrics.volcanic_churn.last_30_days > SUPERNOVA_THRESHOLD {
                census.supernovas.push(Supernova {
                    path: path.clone(),
                    observations_30d: metrics.volcanic_churn.last_30_days,
                    observer_count: metrics.primary_observers.len(),
                    lines_changed: metrics.volcanic_churn.lines_added_90d
                        + metrics.volcanic_churn.lines_removed_90d,
                    warning: format!(
                        "Extreme activity: {} observations in 30 days",
                        metrics.volcanic_churn.last_30_days
                    ),
                });
            }

            // Identify ancient stars (dormant > 2 years)
            let dormant_days = metrics
                .last_observation
                .map(|t| (now - t).num_days().max(0) as u64)
                .unwrap_or(0);

            if dormant_days > ANCIENT_THRESHOLD_DAYS {
                census.ancient_stars.push(AncientStar {
                    path: path.clone(),
                    age_days: metrics.stellar_age.age_days,
                    dormant_days,
                    star_count: 0, // To be filled by caller with census data
                    is_core: false,
                });
            }
        }

        // Build constellation-level aggregation
        let mut constellation_map: HashMap<String, Vec<&FileChurn>> = HashMap::new();
        for (path, file_churn) in &census.files {
            let constellation = Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            constellation_map
                .entry(constellation)
                .or_insert_with(Vec::new)
                .push(file_churn);
        }

        for (path, files) in constellation_map {
            let file_count = files.len();
            let churn_90d: usize = files.iter().map(|f| f.churn_90d).sum();
            let total_age: u64 = files.iter().map(|f| f.age_days).sum();
            let avg_age_days = if file_count > 0 {
                total_age / file_count as u64
            } else {
                0
            };

            let max_30d = files.iter().map(|f| f.churn_30d).max().unwrap_or(0);
            let classification =
                ChurnClassification::from_counts(max_30d, churn_90d / file_count.max(1));

            census.constellations.insert(
                path.clone(),
                ConstellationChurn {
                    path: path.clone(),
                    file_count,
                    churn_90d,
                    avg_age_days,
                    primary_observers: Vec::new(), // Simplified for now
                    classification,
                },
            );
        }

        // Build top observers
        let mut observers: Vec<Observer> = self
            .galaxy_stats
            .observers
            .values()
            .map(|stats| Observer {
                name: stats.name.clone(),
                email_hash: hash_email(&stats.email),
                impact: ObserverImpact {
                    observations: stats.observations,
                    lines_added: stats.lines_added,
                    lines_removed: stats.lines_removed,
                    net_impact: stats.lines_added as i64 - stats.lines_removed as i64,
                    first_seen: stats.first_seen,
                    last_seen: stats.last_seen,
                },
            })
            .collect();

        observers.sort_by(|a, b| b.impact.observations.cmp(&a.impact.observations));
        observers.truncate(10);
        census.top_observers = observers;

        census
    }

    /// Identify tectonic shifts (high churn + high complexity)
    pub fn identify_tectonic_shifts(
        &self,
        dark_matter_ratios: &HashMap<String, f64>,
    ) -> Vec<TectonicShift> {
        let now = Utc::now();
        let mut shifts = Vec::new();

        for (path, observations) in &self.file_histories {
            let metrics = self.calculate_file_metrics(observations, &now);
            let dark_matter = dark_matter_ratios.get(path).copied().unwrap_or(0.0);

            // High churn (>10 in 90d) + High dark matter (>20%)
            if metrics.volcanic_churn.last_90_days > 10 && dark_matter > 0.2 {
                let risk_score = (metrics.volcanic_churn.last_90_days as f64 / 30.0).min(1.0)
                    * (dark_matter / 0.5).min(1.0);

                shifts.push(TectonicShift {
                    path: path.clone(),
                    churn_90d: metrics.volcanic_churn.last_90_days,
                    dark_matter_ratio: dark_matter,
                    risk_score,
                    reason: format!(
                        "High churn ({} observations) + High complexity ({:.0}%)",
                        metrics.volcanic_churn.last_90_days,
                        dark_matter * 100.0
                    ),
                });
            }
        }

        // Sort by risk score
        shifts.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        shifts
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get timestamp from a commit
fn commit_timestamp(commit: &Commit) -> DateTime<Utc> {
    let time = commit.time();
    DateTime::from_timestamp(time.seconds(), 0).unwrap_or_else(Utc::now)
}

/// Normalize a file path relative to the repository root
fn normalize_path(path: &str, root: &Path) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        path.strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string())
    } else {
        path.to_string_lossy().to_string()
    }
}

/// Hash an email for privacy
fn hash_email(email: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    email.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_hash_email() {
        let hash1 = hash_email("test@example.com");
        let hash2 = hash_email("test@example.com");
        assert_eq!(hash1, hash2);

        let hash3 = hash_email("other@example.com");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_email_empty() {
        let hash = hash_email("");
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_hash_email_unicode() {
        let hash = hash_email("user@例え.jp");
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_normalize_path() {
        let root = Path::new("/project");
        assert_eq!(normalize_path("src/main.rs", root), "src/main.rs");
        assert_eq!(normalize_path("/project/src/main.rs", root), "src/main.rs");
    }

    #[test]
    fn test_normalize_path_relative() {
        let root = Path::new("/project");
        assert_eq!(normalize_path("lib/utils.rs", root), "lib/utils.rs");
    }

    #[test]
    fn test_normalize_path_outside_root() {
        let root = Path::new("/project");
        // Path outside root should remain unchanged
        let result = normalize_path("/other/file.rs", root);
        assert!(result.contains("file.rs"));
    }

    // ==================== Classification Tests ====================

    #[test]
    fn test_churn_classification_from_counts() {
        assert_eq!(
            ChurnClassification::from_counts(0, 0),
            ChurnClassification::Dormant
        );
        assert_eq!(
            ChurnClassification::from_counts(35, 50),
            ChurnClassification::Supernova
        );
    }

    #[test]
    fn test_churn_classification_high() {
        // High: > 10 in 90d
        assert_eq!(
            ChurnClassification::from_counts(7, 15),
            ChurnClassification::High
        );
    }

    #[test]
    fn test_churn_classification_low() {
        // Low: 1-3 in 90d
        assert_eq!(
            ChurnClassification::from_counts(0, 2),
            ChurnClassification::Low
        );
    }

    #[test]
    fn test_churn_classification_moderate() {
        // Moderate: 4-10 in 90d
        assert_eq!(
            ChurnClassification::from_counts(3, 6),
            ChurnClassification::Moderate
        );
    }

    #[test]
    fn test_age_classification_from_days() {
        assert_eq!(AgeClassification::from_days(0), AgeClassification::Newborn);
        assert_eq!(AgeClassification::from_days(15), AgeClassification::Newborn);
        assert_eq!(AgeClassification::from_days(60), AgeClassification::Young);
        assert_eq!(AgeClassification::from_days(400), AgeClassification::Mature);
        assert_eq!(
            AgeClassification::from_days(800),
            AgeClassification::Ancient
        );
        assert_eq!(
            AgeClassification::from_days(1000),
            AgeClassification::Ancient
        );
    }

    // ==================== Struct Tests ====================

    #[test]
    fn test_file_observation_creation() {
        let obs = FileObservation {
            timestamp: Utc::now(),
            observer_name: "Test User".to_string(),
            observer_email: "test@example.com".to_string(),
            lines_added: 10,
            lines_removed: 5,
        };
        assert_eq!(obs.observer_name, "Test User");
        assert_eq!(obs.lines_added, 10);
    }

    #[test]
    fn test_commit_data_creation() {
        let data = CommitData {
            timestamp: Utc::now(),
            observer_name: "Developer".to_string(),
            observer_email: "dev@example.com".to_string(),
            files_changed: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()],
        };
        assert_eq!(data.files_changed.len(), 2);
    }

    #[test]
    fn test_galaxy_stats_default() {
        let stats = GalaxyStats::default();
        assert_eq!(stats.total_observations, 0);
        assert!(stats.observers.is_empty());
        assert!(stats.first_observation.is_none());
        assert!(stats.last_observation.is_none());
    }

    #[test]
    fn test_observer_stats_default() {
        let stats = ObserverStats::default();
        assert_eq!(stats.observations, 0);
        assert_eq!(stats.lines_added, 0);
        assert!(stats.first_seen.is_none());
    }

    // ==================== Engine Creation Tests ====================

    #[test]
    fn test_engine_new_in_repo() {
        // This test runs in the actual repo
        let cwd = env::current_dir().expect("Failed to get current dir");
        let engine = ChronosEngine::new(&cwd);

        // Should succeed since we're in a git repo
        assert!(engine.is_some());
    }

    #[test]
    fn test_engine_with_depth() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        let engine = ChronosEngine::with_depth(&cwd, 100);

        assert!(engine.is_some());
        if let Some(e) = engine {
            assert_eq!(e.commit_depth, 100);
        }
    }

    #[test]
    fn test_engine_new_nonexistent_path() {
        let engine = ChronosEngine::new(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(engine.is_none());
    }

    #[test]
    fn test_engine_initial_state() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            assert!(matches!(engine.state(), ChronosState::StaticGalaxy));
            assert!(matches!(engine.warp_status(), WarpStatus::Calibrating));
        }
    }

    // ==================== Constants Tests ====================

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_COMMIT_DEPTH, 1_000);
        assert_eq!(FULL_COMMIT_DEPTH, 100_000);
        assert_eq!(CHURN_WINDOW_30D, 30);
        assert_eq!(CHURN_WINDOW_90D, 90);
        assert_eq!(CHURN_WINDOW_YEAR, 365);
        assert_eq!(ANCIENT_THRESHOLD_DAYS, 730);
        assert_eq!(SUPERNOVA_THRESHOLD, 30);
    }

    // ==================== Metrics Calculation Tests ====================

    #[test]
    fn test_calculate_file_metrics_empty() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let metrics = engine.calculate_file_metrics(&[], &now);

            assert_eq!(metrics.total_observations, 0);
            assert!(metrics.primary_observers.is_empty());
        }
    }

    #[test]
    fn test_calculate_file_metrics_single_observation() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let observations = vec![FileObservation {
                timestamp: now - Duration::days(10),
                observer_name: "Alice".to_string(),
                observer_email: "alice@example.com".to_string(),
                lines_added: 100,
                lines_removed: 20,
            }];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            assert_eq!(metrics.total_observations, 1);
            assert_eq!(metrics.volcanic_churn.last_30_days, 1);
            assert_eq!(metrics.primary_observers.len(), 1);
            assert_eq!(metrics.primary_observers[0].name, "Alice");
        }
    }

    #[test]
    fn test_calculate_file_metrics_multiple_observers() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let observations = vec![
                FileObservation {
                    timestamp: now - Duration::days(5),
                    observer_name: "Alice".to_string(),
                    observer_email: "alice@example.com".to_string(),
                    lines_added: 50,
                    lines_removed: 10,
                },
                FileObservation {
                    timestamp: now - Duration::days(10),
                    observer_name: "Bob".to_string(),
                    observer_email: "bob@example.com".to_string(),
                    lines_added: 30,
                    lines_removed: 5,
                },
                FileObservation {
                    timestamp: now - Duration::days(15),
                    observer_name: "Alice".to_string(),
                    observer_email: "alice@example.com".to_string(),
                    lines_added: 20,
                    lines_removed: 3,
                },
            ];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            assert_eq!(metrics.total_observations, 3);
            assert_eq!(metrics.volcanic_churn.last_30_days, 3);
            // Alice should be first (2 observations)
            assert_eq!(metrics.primary_observers[0].name, "Alice");
            assert_eq!(metrics.primary_observers[0].impact.observations, 2);
        }
    }

    #[test]
    fn test_calculate_file_metrics_old_observations() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let observations = vec![FileObservation {
                timestamp: now - Duration::days(100),
                observer_name: "Developer".to_string(),
                observer_email: "dev@example.com".to_string(),
                lines_added: 50,
                lines_removed: 10,
            }];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            assert_eq!(metrics.total_observations, 1);
            assert_eq!(metrics.volcanic_churn.last_30_days, 0);
            assert_eq!(metrics.volcanic_churn.last_90_days, 0);
            assert_eq!(metrics.volcanic_churn.last_year, 1);
        }
    }

    #[test]
    fn test_calculate_file_metrics_stellar_age() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let observations = vec![FileObservation {
                timestamp: now - Duration::days(500),
                observer_name: "Dev".to_string(),
                observer_email: "dev@example.com".to_string(),
                lines_added: 100,
                lines_removed: 0,
            }];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            assert!(metrics.stellar_age.age_days >= 499);
            // 500 days is Mature (365-729 days)
            assert_eq!(
                metrics.stellar_age.classification,
                AgeClassification::Mature
            );
        }
    }

    // ==================== File Metrics Tests ====================

    #[test]
    fn test_file_metrics_nonexistent_file() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let metrics = engine.file_metrics("nonexistent/file/that/does/not/exist.rs");

            // Should return default metrics
            assert_eq!(metrics.total_observations, 0);
        }
    }

    // ==================== History Extraction Tests ====================

    #[test]
    fn test_extract_history() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::new(&cwd) {
            let result = engine.extract_history();
            assert!(result.is_ok());

            // State should be either Active or ShallowCensus
            match engine.state() {
                ChronosState::Active { total_events, .. } => {
                    assert!(*total_events > 0);
                }
                ChronosState::ShallowCensus { total_events, .. } => {
                    assert!(*total_events > 0);
                }
                _ => panic!("Unexpected state after extraction"),
            }
        }
    }

    #[test]
    fn test_extract_history_cached() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::new(&cwd) {
            let result = engine.extract_history_cached();
            assert!(result.is_ok());

            // Warp status should be either Engaged or Calibrating
            match engine.warp_status() {
                WarpStatus::Engaged | WarpStatus::Calibrating => {}
                _ => panic!("Unexpected warp status"),
            }
        }
    }

    // ==================== Census Tests ====================

    #[test]
    fn test_build_census_without_extraction() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let census = engine.build_census();

            // Without extraction, census should be mostly empty
            assert!(matches!(census.state, ChronosState::StaticGalaxy));
        }
    }

    #[test]
    fn test_build_census_with_extraction() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::new(&cwd) {
            let _ = engine.extract_history();
            let census = engine.build_census();

            // After extraction, census should have data
            assert!(
                census.total_observations > 0 || matches!(census.state, ChronosState::StaticGalaxy)
            );
        }
    }

    // ==================== Tectonic Shifts Tests ====================

    #[test]
    fn test_identify_tectonic_shifts_empty() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let dark_matter: HashMap<String, f64> = HashMap::new();
            let shifts = engine.identify_tectonic_shifts(&dark_matter);

            // Without history, should be empty
            assert!(shifts.is_empty());
        }
    }

    #[test]
    fn test_identify_tectonic_shifts_with_data() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::new(&cwd) {
            let _ = engine.extract_history();

            // Create some dark matter ratios for files
            let mut dark_matter: HashMap<String, f64> = HashMap::new();
            for path in engine.file_histories.keys() {
                dark_matter.insert(path.clone(), 0.3); // 30% dark matter
            }

            let shifts = engine.identify_tectonic_shifts(&dark_matter);
            // May or may not have shifts depending on churn
            let _ = shifts;
        }
    }

    // ==================== Cache Tests ====================

    #[test]
    fn test_is_cache_valid() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            // Just verify it doesn't panic
            let _valid = engine.is_cache_valid();
        }
    }

    #[test]
    fn test_invalidate_cache() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            // Invalidate should succeed or fail gracefully
            let _ = engine.invalidate_cache();
        }
    }

    // ==================== WarpStatus Tests ====================

    #[test]
    fn test_warp_status_variants() {
        // Test that all variants exist
        let _calibrating = WarpStatus::Calibrating;
        let _engaged = WarpStatus::Engaged;
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_workflow() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::with_depth(&cwd, 50) {
            // Extract history with limited depth for speed
            let result = engine.extract_history();
            assert!(result.is_ok());

            // Check state
            let state = engine.state();
            match state {
                ChronosState::Active {
                    total_events,
                    observer_count,
                    ..
                }
                | ChronosState::ShallowCensus {
                    total_events,
                    observer_count,
                    ..
                } => {
                    assert!(*total_events > 0);
                    assert!(*observer_count > 0);
                }
                _ => {}
            }

            // Build census - just verify it doesn't panic
            let _census = engine.build_census();
        }
    }

    // ==================== Additional Coverage Tests ====================

    #[test]
    fn test_build_census_constellations() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::with_depth(&cwd, 50) {
            let _ = engine.extract_history();
            let census = engine.build_census();

            // If there are files, there should be constellations
            if !census.files.is_empty() {
                assert!(!census.constellations.is_empty());
            }
        }
    }

    #[test]
    fn test_calculate_file_metrics_lines_tracking() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let observations = vec![
                FileObservation {
                    timestamp: now - Duration::days(5),
                    observer_name: "Alice".to_string(),
                    observer_email: "alice@example.com".to_string(),
                    lines_added: 100,
                    lines_removed: 20,
                },
                FileObservation {
                    timestamp: now - Duration::days(10),
                    observer_name: "Bob".to_string(),
                    observer_email: "bob@example.com".to_string(),
                    lines_added: 50,
                    lines_removed: 30,
                },
            ];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            // Total lines added/removed should be tracked
            assert_eq!(metrics.volcanic_churn.lines_added_90d, 150);
            assert_eq!(metrics.volcanic_churn.lines_removed_90d, 50);
        }
    }

    #[test]
    fn test_calculate_file_metrics_observers_truncated() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();

            // Create 5 different observers
            let observations = vec![
                FileObservation {
                    timestamp: now - Duration::days(1),
                    observer_name: "Alice".to_string(),
                    observer_email: "alice@example.com".to_string(),
                    lines_added: 10,
                    lines_removed: 0,
                },
                FileObservation {
                    timestamp: now - Duration::days(2),
                    observer_name: "Bob".to_string(),
                    observer_email: "bob@example.com".to_string(),
                    lines_added: 10,
                    lines_removed: 0,
                },
                FileObservation {
                    timestamp: now - Duration::days(3),
                    observer_name: "Charlie".to_string(),
                    observer_email: "charlie@example.com".to_string(),
                    lines_added: 10,
                    lines_removed: 0,
                },
                FileObservation {
                    timestamp: now - Duration::days(4),
                    observer_name: "Diana".to_string(),
                    observer_email: "diana@example.com".to_string(),
                    lines_added: 10,
                    lines_removed: 0,
                },
                FileObservation {
                    timestamp: now - Duration::days(5),
                    observer_name: "Eve".to_string(),
                    observer_email: "eve@example.com".to_string(),
                    lines_added: 10,
                    lines_removed: 0,
                },
            ];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            // Should be truncated to 3 observers
            assert_eq!(metrics.primary_observers.len(), 3);
        }
    }

    #[test]
    fn test_file_metrics_with_extraction() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::with_depth(&cwd, 50) {
            let _ = engine.extract_history();

            // Try to get metrics for a file that likely exists
            if let Some(path) = engine.file_histories.keys().next().cloned() {
                let metrics = engine.file_metrics(&path);
                // Should have at least some observations
                assert!(metrics.total_observations > 0);
            }
        }
    }

    #[test]
    fn test_calculate_file_metrics_churn_windows() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(engine) = ChronosEngine::new(&cwd) {
            let now = Utc::now();
            let observations = vec![
                // Recent (in 30d)
                FileObservation {
                    timestamp: now - Duration::days(15),
                    observer_name: "Dev".to_string(),
                    observer_email: "dev@example.com".to_string(),
                    lines_added: 10,
                    lines_removed: 0,
                },
                // In 90d but not 30d
                FileObservation {
                    timestamp: now - Duration::days(60),
                    observer_name: "Dev".to_string(),
                    observer_email: "dev@example.com".to_string(),
                    lines_added: 20,
                    lines_removed: 0,
                },
                // In year but not 90d
                FileObservation {
                    timestamp: now - Duration::days(200),
                    observer_name: "Dev".to_string(),
                    observer_email: "dev@example.com".to_string(),
                    lines_added: 30,
                    lines_removed: 0,
                },
            ];

            let metrics = engine.calculate_file_metrics(&observations, &now);

            assert_eq!(metrics.volcanic_churn.last_30_days, 1);
            assert_eq!(metrics.volcanic_churn.last_90_days, 2);
            assert_eq!(metrics.volcanic_churn.last_year, 3);
        }
    }

    #[test]
    fn test_census_top_observers() {
        let cwd = env::current_dir().expect("Failed to get current dir");
        if let Some(mut engine) = ChronosEngine::with_depth(&cwd, 50) {
            let _ = engine.extract_history();
            let census = engine.build_census();

            // Top observers should be populated and limited to 10
            assert!(census.top_observers.len() <= 10);
        }
    }

    #[test]
    fn test_commit_data_clone() {
        let data = CommitData {
            timestamp: Utc::now(),
            observer_name: "Dev".to_string(),
            observer_email: "dev@example.com".to_string(),
            files_changed: vec!["file.rs".to_string()],
        };
        let cloned = data.clone();
        assert_eq!(data.observer_name, cloned.observer_name);
        assert_eq!(data.files_changed.len(), cloned.files_changed.len());
    }

    #[test]
    fn test_file_observation_clone() {
        let obs = FileObservation {
            timestamp: Utc::now(),
            observer_name: "Dev".to_string(),
            observer_email: "dev@example.com".to_string(),
            lines_added: 10,
            lines_removed: 5,
        };
        let cloned = obs.clone();
        assert_eq!(obs.lines_added, cloned.lines_added);
        assert_eq!(obs.lines_removed, cloned.lines_removed);
    }

    #[test]
    fn test_galaxy_stats_clone() {
        let stats = GalaxyStats::default();
        let cloned = stats.clone();
        assert_eq!(stats.total_observations, cloned.total_observations);
    }

    #[test]
    fn test_observer_stats_clone() {
        let stats = ObserverStats {
            name: "Dev".to_string(),
            email: "dev@example.com".to_string(),
            observations: 10,
            lines_added: 100,
            lines_removed: 20,
            first_seen: Some(Utc::now()),
            last_seen: Some(Utc::now()),
        };
        let cloned = stats.clone();
        assert_eq!(stats.observations, cloned.observations);
    }
}
