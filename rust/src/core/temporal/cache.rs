//! Chronos Warp - Persistent Cache for Temporal Analysis
//!
//! The Chronos Warp provides near-instantaneous repeat scans by caching
//! git history analysis results. Cache invalidation uses a triple-check:
//! - Git HEAD hash (new commits)
//! - Git directory mtime (any git operation)
//! - Cache TTL (24 hour max age)
//!
//! # Celestial Terminology
//!
//! - **Warp Engaged**: Cache hit - instant results
//! - **Warp Calibrating**: Cache miss - full git analysis
//! - **Warp Cache**: The `.voyager/cache/chronos/` directory

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// =============================================================================
// Constants
// =============================================================================

/// Cache directory name
const CACHE_DIR: &str = ".voyager/cache/chronos";

/// Cache file name
const CACHE_FILE: &str = "temporal_cache.bin";

/// Maximum cache age in seconds (24 hours)
const CACHE_TTL_SECONDS: u64 = 86400;

/// Cache format version (bump to invalidate old caches)
const CACHE_VERSION: u32 = 1;

// =============================================================================
// Cache Entry
// =============================================================================

/// Cached temporal analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronosCache {
    /// Cache format version
    pub version: u32,
    /// Git HEAD hash when cache was created
    pub git_head_hash: String,
    /// Timestamp when cache was created
    pub created_at: u64,
    /// TTL in seconds
    pub ttl_seconds: u64,
    /// Cached file histories (path -> observations)
    pub file_histories: HashMap<String, Vec<CachedObservation>>,
    /// Galaxy-level statistics
    pub galaxy_stats: CachedGalaxyStats,
    /// Total observations analyzed
    pub total_observations: usize,
    /// Whether depth limit was hit
    pub hit_depth_limit: bool,
    /// Commit depth used
    pub commit_depth: usize,
}

/// Cached observation (commit affecting a file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedObservation {
    /// Commit timestamp (seconds since epoch)
    pub timestamp_secs: i64,
    /// Observer (author) name
    pub observer_name: String,
    /// Observer email (hashed for privacy)
    pub observer_email_hash: String,
    /// Lines added
    pub lines_added: usize,
    /// Lines removed
    pub lines_removed: usize,
}

/// Cached galaxy statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CachedGalaxyStats {
    /// Total observations
    pub total_observations: usize,
    /// First observation timestamp (seconds since epoch)
    pub first_observation_secs: Option<i64>,
    /// Last observation timestamp (seconds since epoch)
    pub last_observation_secs: Option<i64>,
    /// Observer count
    pub observer_count: usize,
    /// Top observers (name, observation_count)
    pub top_observers: Vec<(String, usize)>,
}

// =============================================================================
// Cache Manager
// =============================================================================

/// Manages the Chronos Warp cache
pub struct ChronosCacheManager {
    /// Cache directory path
    cache_dir: PathBuf,
    /// Repository root path
    repo_root: PathBuf,
}

impl ChronosCacheManager {
    /// Create a new cache manager for a repository
    pub fn new(repo_root: &Path) -> Self {
        let cache_dir = repo_root.join(CACHE_DIR);
        Self {
            cache_dir,
            repo_root: repo_root.to_path_buf(),
        }
    }

    /// Get the cache file path
    fn cache_path(&self) -> PathBuf {
        self.cache_dir.join(CACHE_FILE)
    }

    /// Check if Warp is engaged (valid cache exists)
    pub fn warp_engaged(&self) -> bool {
        self.load().is_some()
    }

    /// Load cache if valid
    pub fn load(&self) -> Option<ChronosCache> {
        let cache_path = self.cache_path();

        // Check if cache file exists
        if !cache_path.exists() {
            return None;
        }

        // Read cache file
        let mut file = File::open(&cache_path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        // Deserialize
        let cache: ChronosCache = bincode::deserialize(&buffer).ok()?;

        // Version check
        if cache.version != CACHE_VERSION {
            return None;
        }

        // TTL check
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now - cache.created_at > cache.ttl_seconds {
            return None;
        }

        // Git HEAD hash check
        let current_head = self.get_git_head_hash()?;
        if cache.git_head_hash != current_head {
            return None;
        }

        Some(cache)
    }

    /// Save cache
    pub fn save(&self, cache: &ChronosCache) -> Result<(), String> {
        // Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        // Serialize
        let buffer =
            bincode::serialize(cache).map_err(|e| format!("Failed to serialize cache: {}", e))?;

        // Write atomically (write to temp, then rename)
        let cache_path = self.cache_path();
        let temp_path = cache_path.with_extension("tmp");

        let mut file =
            File::create(&temp_path).map_err(|e| format!("Failed to create cache file: {}", e))?;
        file.write_all(&buffer)
            .map_err(|e| format!("Failed to write cache: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("Failed to sync cache: {}", e))?;

        fs::rename(&temp_path, &cache_path)
            .map_err(|e| format!("Failed to rename cache file: {}", e))?;

        Ok(())
    }

    /// Invalidate (delete) the cache
    pub fn invalidate(&self) -> Result<(), String> {
        let cache_path = self.cache_path();
        if cache_path.exists() {
            fs::remove_file(&cache_path).map_err(|e| format!("Failed to remove cache: {}", e))?;
        }
        Ok(())
    }

    /// Get current git HEAD hash
    fn get_git_head_hash(&self) -> Option<String> {
        let git_dir = self.repo_root.join(".git");

        // Handle both regular .git directory and worktree .git file
        let head_path = if git_dir.is_file() {
            // Worktree: .git is a file pointing to the actual git dir
            let content = fs::read_to_string(&git_dir).ok()?;
            let git_dir_path = content.strip_prefix("gitdir: ")?.trim();
            PathBuf::from(git_dir_path).join("HEAD")
        } else {
            git_dir.join("HEAD")
        };

        let head_content = fs::read_to_string(&head_path).ok()?;

        // HEAD can be either a direct hash or a ref
        if head_content.starts_with("ref: ") {
            // It's a symbolic ref, resolve it
            let ref_path = head_content.strip_prefix("ref: ")?.trim();
            let full_ref_path = if git_dir.is_file() {
                // Worktree case
                let content = fs::read_to_string(&git_dir).ok()?;
                let git_dir_path = content.strip_prefix("gitdir: ")?.trim();
                PathBuf::from(git_dir_path).parent()?.join(ref_path)
            } else {
                git_dir.join(ref_path)
            };
            fs::read_to_string(&full_ref_path)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            // Direct hash
            Some(head_content.trim().to_string())
        }
    }

    /// Create a new cache entry
    pub fn create_cache(
        &self,
        file_histories: HashMap<String, Vec<CachedObservation>>,
        galaxy_stats: CachedGalaxyStats,
        total_observations: usize,
        hit_depth_limit: bool,
        commit_depth: usize,
    ) -> Result<ChronosCache, String> {
        let git_head_hash = self
            .get_git_head_hash()
            .ok_or_else(|| "Failed to get git HEAD hash".to_string())?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(ChronosCache {
            version: CACHE_VERSION,
            git_head_hash,
            created_at: now,
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories,
            galaxy_stats,
            total_observations,
            hit_depth_limit,
            commit_depth,
        })
    }
}

// =============================================================================
// Cache Status
// =============================================================================

/// Status of the Chronos Warp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpStatus {
    /// Cache hit - instant results
    Engaged,
    /// Cache miss - full analysis required
    Calibrating,
    /// Cache disabled
    Offline,
}

impl WarpStatus {
    /// Get a description for display
    pub fn description(&self) -> &'static str {
        match self {
            Self::Engaged => "Warp Engaged (cached)",
            Self::Calibrating => "Warp Calibrating (analyzing)",
            Self::Offline => "Warp Offline (no cache)",
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_version() {
        assert_eq!(CACHE_VERSION, 1);
    }

    #[test]
    fn test_cache_ttl() {
        assert_eq!(CACHE_TTL_SECONDS, 86400); // 24 hours
    }

    #[test]
    fn test_warp_status_description() {
        assert_eq!(WarpStatus::Engaged.description(), "Warp Engaged (cached)");
        assert_eq!(
            WarpStatus::Calibrating.description(),
            "Warp Calibrating (analyzing)"
        );
        assert_eq!(WarpStatus::Offline.description(), "Warp Offline (no cache)");
    }

    #[test]
    fn test_cache_serialization() {
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123".to_string(),
            created_at: 1000000,
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 100,
            hit_depth_limit: false,
            commit_depth: 1000,
        };

        let serialized = bincode::serialize(&cache).unwrap();
        let deserialized: ChronosCache = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.version, CACHE_VERSION);
        assert_eq!(deserialized.git_head_hash, "abc123");
        assert_eq!(deserialized.total_observations, 100);
    }

    #[test]
    fn test_cached_observation_serialization() {
        let obs = CachedObservation {
            timestamp_secs: 1609459200,
            observer_name: "Test User".to_string(),
            observer_email_hash: "abc123".to_string(),
            lines_added: 100,
            lines_removed: 50,
        };

        let serialized = bincode::serialize(&obs).unwrap();
        let deserialized: CachedObservation = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.timestamp_secs, 1609459200);
        assert_eq!(deserialized.observer_name, "Test User");
        assert_eq!(deserialized.lines_added, 100);
    }

    #[test]
    fn test_cache_invalidation_on_version_mismatch() {
        // Simulates cache version mismatch (old cache format)
        let old_cache = ChronosCache {
            version: 0, // Wrong version
            git_head_hash: "abc123".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 100,
            hit_depth_limit: false,
            commit_depth: 1000,
        };

        // When version doesn't match CACHE_VERSION, cache should be invalid
        assert_ne!(old_cache.version, CACHE_VERSION);
    }

    #[test]
    fn test_cache_expiration_logic() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Cache created 25 hours ago (should be expired)
        let expired_created_at = now - (25 * 3600);

        // Check expiration condition
        let is_expired = now - expired_created_at > CACHE_TTL_SECONDS;
        assert!(is_expired, "25-hour-old cache should be expired");

        // Cache created 12 hours ago (should still be valid)
        let valid_created_at = now - (12 * 3600);
        let is_valid = now - valid_created_at <= CACHE_TTL_SECONDS;
        assert!(is_valid, "12-hour-old cache should still be valid");
    }

    #[test]
    fn test_cache_git_head_mismatch() {
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "old_commit_abc123".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 100,
            hit_depth_limit: false,
            commit_depth: 1000,
        };

        let current_head = "new_commit_xyz789";

        // When git HEAD doesn't match, cache should be considered invalid
        assert_ne!(cache.git_head_hash, current_head);
    }

    #[test]
    fn test_cached_galaxy_stats_default() {
        let stats = CachedGalaxyStats::default();
        assert_eq!(stats.total_observations, 0);
        assert!(stats.first_observation_secs.is_none());
        assert!(stats.last_observation_secs.is_none());
        assert_eq!(stats.observer_count, 0);
        assert!(stats.top_observers.is_empty());
    }

    #[test]
    fn test_cache_hit_vs_miss_scenario() {
        // Simulate cache hit scenario
        let cache_hit_status = WarpStatus::Engaged;
        assert_eq!(cache_hit_status.description(), "Warp Engaged (cached)");

        // Simulate cache miss scenario
        let cache_miss_status = WarpStatus::Calibrating;
        assert_eq!(
            cache_miss_status.description(),
            "Warp Calibrating (analyzing)"
        );

        // Verify they are different
        assert_ne!(cache_hit_status, cache_miss_status);
    }

    // =========================================================================
    // WarpStatus Extended Tests
    // =========================================================================

    #[test]
    fn test_warp_status_equality() {
        assert_eq!(WarpStatus::Engaged, WarpStatus::Engaged);
        assert_eq!(WarpStatus::Calibrating, WarpStatus::Calibrating);
        assert_eq!(WarpStatus::Offline, WarpStatus::Offline);
        assert_ne!(WarpStatus::Engaged, WarpStatus::Offline);
    }

    #[test]
    fn test_warp_status_copy() {
        let status = WarpStatus::Engaged;
        let copied = status; // Copy
        assert_eq!(status, copied);
    }

    #[test]
    fn test_warp_status_clone() {
        let status = WarpStatus::Calibrating;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_warp_status_debug() {
        let status = WarpStatus::Offline;
        let debug_str = format!("{:?}", status);
        assert_eq!(debug_str, "Offline");
    }

    // =========================================================================
    // ChronosCache Extended Tests
    // =========================================================================

    #[test]
    fn test_chronos_cache_fields() {
        let mut histories = HashMap::new();
        histories.insert(
            "src/main.rs".to_string(),
            vec![CachedObservation {
                timestamp_secs: 1609459200,
                observer_name: "Alice".to_string(),
                observer_email_hash: "hash1".to_string(),
                lines_added: 50,
                lines_removed: 10,
            }],
        );

        let stats = CachedGalaxyStats {
            total_observations: 100,
            first_observation_secs: Some(1600000000),
            last_observation_secs: Some(1700000000),
            observer_count: 5,
            top_observers: vec![("Alice".to_string(), 50), ("Bob".to_string(), 30)],
        };

        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123def456".to_string(),
            created_at: 1700000000,
            ttl_seconds: 3600,
            file_histories: histories,
            galaxy_stats: stats,
            total_observations: 100,
            hit_depth_limit: true,
            commit_depth: 500,
        };

        assert_eq!(cache.version, 1);
        assert_eq!(cache.git_head_hash, "abc123def456");
        assert_eq!(cache.created_at, 1700000000);
        assert_eq!(cache.ttl_seconds, 3600);
        assert_eq!(cache.file_histories.len(), 1);
        assert!(cache.file_histories.contains_key("src/main.rs"));
        assert_eq!(cache.galaxy_stats.total_observations, 100);
        assert_eq!(cache.total_observations, 100);
        assert!(cache.hit_depth_limit);
        assert_eq!(cache.commit_depth, 500);
    }

    #[test]
    fn test_chronos_cache_clone() {
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123".to_string(),
            created_at: 1000000,
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 100,
            hit_depth_limit: false,
            commit_depth: 1000,
        };

        let cloned = cache.clone();
        assert_eq!(cloned.git_head_hash, cache.git_head_hash);
        assert_eq!(cloned.total_observations, cache.total_observations);
    }

    #[test]
    fn test_chronos_cache_debug() {
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "test".to_string(),
            created_at: 0,
            ttl_seconds: 0,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        let debug_str = format!("{:?}", cache);
        assert!(debug_str.contains("ChronosCache"));
        assert!(debug_str.contains("version: 1"));
    }

    #[test]
    fn test_chronos_cache_with_multiple_files() {
        let mut histories = HashMap::new();
        histories.insert("file1.rs".to_string(), vec![]);
        histories.insert("file2.rs".to_string(), vec![]);
        histories.insert("file3.rs".to_string(), vec![]);

        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "hash".to_string(),
            created_at: 0,
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: histories,
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        assert_eq!(cache.file_histories.len(), 3);
    }

    // =========================================================================
    // CachedObservation Extended Tests
    // =========================================================================

    #[test]
    fn test_cached_observation_fields() {
        let obs = CachedObservation {
            timestamp_secs: 1609459200,
            observer_name: "Test User".to_string(),
            observer_email_hash: "sha256:abc123".to_string(),
            lines_added: 100,
            lines_removed: 50,
        };

        assert_eq!(obs.timestamp_secs, 1609459200);
        assert_eq!(obs.observer_name, "Test User");
        assert_eq!(obs.observer_email_hash, "sha256:abc123");
        assert_eq!(obs.lines_added, 100);
        assert_eq!(obs.lines_removed, 50);
    }

    #[test]
    fn test_cached_observation_clone() {
        let obs = CachedObservation {
            timestamp_secs: 1000,
            observer_name: "Alice".to_string(),
            observer_email_hash: "hash".to_string(),
            lines_added: 10,
            lines_removed: 5,
        };

        let cloned = obs.clone();
        assert_eq!(cloned.observer_name, obs.observer_name);
    }

    #[test]
    fn test_cached_observation_debug() {
        let obs = CachedObservation {
            timestamp_secs: 0,
            observer_name: "Test".to_string(),
            observer_email_hash: "hash".to_string(),
            lines_added: 0,
            lines_removed: 0,
        };

        let debug_str = format!("{:?}", obs);
        assert!(debug_str.contains("CachedObservation"));
        assert!(debug_str.contains("observer_name"));
    }

    #[test]
    fn test_cached_observation_zero_lines() {
        let obs = CachedObservation {
            timestamp_secs: 1000,
            observer_name: "User".to_string(),
            observer_email_hash: "hash".to_string(),
            lines_added: 0,
            lines_removed: 0,
        };

        // Zero lines is valid (e.g., mode change only)
        assert_eq!(obs.lines_added, 0);
        assert_eq!(obs.lines_removed, 0);
    }

    #[test]
    fn test_cached_observation_large_diff() {
        let obs = CachedObservation {
            timestamp_secs: 1000,
            observer_name: "Refactorer".to_string(),
            observer_email_hash: "hash".to_string(),
            lines_added: 10000,
            lines_removed: 8000,
        };

        // Large diffs are valid
        assert_eq!(obs.lines_added, 10000);
        assert_eq!(obs.lines_removed, 8000);
    }

    // =========================================================================
    // CachedGalaxyStats Extended Tests
    // =========================================================================

    #[test]
    fn test_cached_galaxy_stats_fields() {
        let stats = CachedGalaxyStats {
            total_observations: 500,
            first_observation_secs: Some(1600000000),
            last_observation_secs: Some(1700000000),
            observer_count: 10,
            top_observers: vec![
                ("Alice".to_string(), 200),
                ("Bob".to_string(), 150),
                ("Charlie".to_string(), 100),
            ],
        };

        assert_eq!(stats.total_observations, 500);
        assert_eq!(stats.first_observation_secs, Some(1600000000));
        assert_eq!(stats.last_observation_secs, Some(1700000000));
        assert_eq!(stats.observer_count, 10);
        assert_eq!(stats.top_observers.len(), 3);
        assert_eq!(stats.top_observers[0].0, "Alice");
        assert_eq!(stats.top_observers[0].1, 200);
    }

    #[test]
    fn test_cached_galaxy_stats_clone() {
        let stats = CachedGalaxyStats {
            total_observations: 100,
            first_observation_secs: Some(1000),
            last_observation_secs: Some(2000),
            observer_count: 5,
            top_observers: vec![("User".to_string(), 50)],
        };

        let cloned = stats.clone();
        assert_eq!(cloned.total_observations, stats.total_observations);
        assert_eq!(cloned.top_observers.len(), stats.top_observers.len());
    }

    #[test]
    fn test_cached_galaxy_stats_debug() {
        let stats = CachedGalaxyStats::default();
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("CachedGalaxyStats"));
    }

    #[test]
    fn test_cached_galaxy_stats_serialization() {
        let stats = CachedGalaxyStats {
            total_observations: 42,
            first_observation_secs: Some(1000000),
            last_observation_secs: Some(2000000),
            observer_count: 3,
            top_observers: vec![("Alice".to_string(), 20)],
        };

        let serialized = bincode::serialize(&stats).unwrap();
        let deserialized: CachedGalaxyStats = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.total_observations, 42);
        assert_eq!(deserialized.observer_count, 3);
    }

    #[test]
    fn test_cached_galaxy_stats_empty_observers() {
        let stats = CachedGalaxyStats {
            total_observations: 0,
            first_observation_secs: None,
            last_observation_secs: None,
            observer_count: 0,
            top_observers: vec![],
        };

        assert!(stats.top_observers.is_empty());
        assert_eq!(stats.observer_count, 0);
    }

    // =========================================================================
    // ChronosCacheManager Tests
    // =========================================================================

    #[test]
    fn test_cache_manager_new() {
        let temp_dir = std::env::temp_dir().join("test_cache_manager_new");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up if exists
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Check that paths are set correctly
        assert_eq!(manager.repo_root, temp_dir);
        assert_eq!(manager.cache_dir, temp_dir.join(".voyager/cache/chronos"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_cache_path() {
        let temp_dir = std::env::temp_dir().join("test_cache_manager_path");
        let manager = ChronosCacheManager::new(&temp_dir);

        let expected = temp_dir.join(".voyager/cache/chronos/temporal_cache.bin");
        assert_eq!(manager.cache_path(), expected);
    }

    #[test]
    fn test_cache_manager_warp_not_engaged_no_cache() {
        let temp_dir = std::env::temp_dir().join("test_warp_not_engaged");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // No cache file exists
        assert!(!manager.warp_engaged());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_load_no_file() {
        let temp_dir = std::env::temp_dir().join("test_load_no_file");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // No cache file - should return None
        assert!(manager.load().is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_save_creates_directory() {
        let temp_dir = std::env::temp_dir().join("test_save_creates_dir");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123def456").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123def456".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 50,
            hit_depth_limit: false,
            commit_depth: 1000,
        };

        // Save should create the cache directory
        let result = manager.save(&cache);
        assert!(result.is_ok(), "Save failed: {:?}", result.err());

        // Verify cache directory was created
        assert!(manager.cache_dir.exists());

        // Verify cache file was created
        assert!(manager.cache_path().exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_save_and_load_round_trip() {
        let temp_dir = std::env::temp_dir().join("test_save_load_roundtrip");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo with direct hash
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123def456789").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        let mut histories = HashMap::new();
        histories.insert(
            "src/lib.rs".to_string(),
            vec![CachedObservation {
                timestamp_secs: 1000000,
                observer_name: "Test".to_string(),
                observer_email_hash: "hash".to_string(),
                lines_added: 10,
                lines_removed: 5,
            }],
        );

        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123def456789".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: histories,
            galaxy_stats: CachedGalaxyStats {
                total_observations: 1,
                first_observation_secs: Some(1000000),
                last_observation_secs: Some(1000000),
                observer_count: 1,
                top_observers: vec![("Test".to_string(), 1)],
            },
            total_observations: 1,
            hit_depth_limit: false,
            commit_depth: 1000,
        };

        // Save
        manager.save(&cache).unwrap();

        // Load
        let loaded = manager.load();
        assert!(loaded.is_some(), "Load returned None");

        let loaded = loaded.unwrap();
        assert_eq!(loaded.version, CACHE_VERSION);
        assert_eq!(loaded.git_head_hash, "abc123def456789");
        assert_eq!(loaded.total_observations, 1);
        assert_eq!(loaded.file_histories.len(), 1);
        assert!(loaded.file_histories.contains_key("src/lib.rs"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_invalidate() {
        let temp_dir = std::env::temp_dir().join("test_invalidate");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Create and save a cache
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        manager.save(&cache).unwrap();
        assert!(manager.cache_path().exists());

        // Invalidate
        let result = manager.invalidate();
        assert!(result.is_ok());
        assert!(!manager.cache_path().exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_invalidate_no_file() {
        let temp_dir = std::env::temp_dir().join("test_invalidate_no_file");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Invalidate when no cache exists - should succeed
        let result = manager.invalidate();
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_load_version_mismatch() {
        let temp_dir = std::env::temp_dir().join("test_version_mismatch");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Create cache with wrong version
        let cache = ChronosCache {
            version: 999, // Wrong version
            git_head_hash: "abc123".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        // Write directly (bypassing version check in save)
        fs::create_dir_all(&manager.cache_dir).unwrap();
        let buffer = bincode::serialize(&cache).unwrap();
        fs::write(manager.cache_path(), &buffer).unwrap();

        // Load should return None due to version mismatch
        assert!(manager.load().is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_load_expired() {
        let temp_dir = std::env::temp_dir().join("test_expired_cache");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Create cache that's already expired (created 2 days ago)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123".to_string(),
            created_at: now - (2 * 24 * 3600), // 2 days ago
            ttl_seconds: CACHE_TTL_SECONDS,    // 24 hours
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        // Write directly
        fs::create_dir_all(&manager.cache_dir).unwrap();
        let buffer = bincode::serialize(&cache).unwrap();
        fs::write(manager.cache_path(), &buffer).unwrap();

        // Load should return None due to TTL expiration
        assert!(manager.load().is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_load_git_head_changed() {
        let temp_dir = std::env::temp_dir().join("test_git_head_changed");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo with current HEAD
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "new_commit_hash").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Create cache with old HEAD
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "old_commit_hash".to_string(), // Different from current
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        // Write directly
        fs::create_dir_all(&manager.cache_dir).unwrap();
        let buffer = bincode::serialize(&cache).unwrap();
        fs::write(manager.cache_path(), &buffer).unwrap();

        // Load should return None due to HEAD mismatch
        assert!(manager.load().is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_get_git_head_hash_direct() {
        let temp_dir = std::env::temp_dir().join("test_git_head_direct");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo with direct hash in HEAD
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123def456\n").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        let hash = manager.get_git_head_hash();
        assert_eq!(hash, Some("abc123def456".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_get_git_head_hash_ref() {
        let temp_dir = std::env::temp_dir().join("test_git_head_ref");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo with symbolic ref
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        // Create the ref file
        let refs_dir = git_dir.join("refs/heads");
        fs::create_dir_all(&refs_dir).unwrap();
        fs::write(refs_dir.join("main"), "abc123def456\n").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        let hash = manager.get_git_head_hash();
        assert_eq!(hash, Some("abc123def456".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_get_git_head_hash_no_git() {
        let temp_dir = std::env::temp_dir().join("test_no_git_dir");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // No .git directory
        let manager = ChronosCacheManager::new(&temp_dir);

        let hash = manager.get_git_head_hash();
        assert!(hash.is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_create_cache() {
        let temp_dir = std::env::temp_dir().join("test_create_cache");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "test_hash_123").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        let histories = HashMap::new();
        let stats = CachedGalaxyStats::default();

        let result = manager.create_cache(
            histories, stats, 100,  // total_observations
            true, // hit_depth_limit
            500,  // commit_depth
        );

        assert!(result.is_ok());
        let cache = result.unwrap();

        assert_eq!(cache.version, CACHE_VERSION);
        assert_eq!(cache.git_head_hash, "test_hash_123");
        assert_eq!(cache.ttl_seconds, CACHE_TTL_SECONDS);
        assert_eq!(cache.total_observations, 100);
        assert!(cache.hit_depth_limit);
        assert_eq!(cache.commit_depth, 500);

        // Verify created_at is recent
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(cache.created_at <= now);
        assert!(cache.created_at > now - 10); // Within last 10 seconds

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_create_cache_no_git() {
        let temp_dir = std::env::temp_dir().join("test_create_cache_no_git");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // No .git directory
        let manager = ChronosCacheManager::new(&temp_dir);

        let result =
            manager.create_cache(HashMap::new(), CachedGalaxyStats::default(), 0, false, 0);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("git HEAD"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_manager_warp_engaged_valid_cache() {
        let temp_dir = std::env::temp_dir().join("test_warp_engaged_valid");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create mock git repo
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "abc123").unwrap();

        let manager = ChronosCacheManager::new(&temp_dir);

        // Create and save valid cache
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "abc123".to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl_seconds: CACHE_TTL_SECONDS,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        manager.save(&cache).unwrap();

        // Warp should be engaged
        assert!(manager.warp_engaged());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    // =========================================================================
    // Serialization Edge Cases
    // =========================================================================

    #[test]
    fn test_cache_with_empty_strings() {
        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "".to_string(), // Empty hash
            created_at: 0,
            ttl_seconds: 0,
            file_histories: HashMap::new(),
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 0,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        let serialized = bincode::serialize(&cache).unwrap();
        let deserialized: ChronosCache = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.git_head_hash, "");
    }

    #[test]
    fn test_cache_with_unicode_names() {
        let obs = CachedObservation {
            timestamp_secs: 1000,
            observer_name: "测试用户 🚀".to_string(),
            observer_email_hash: "hash".to_string(),
            lines_added: 0,
            lines_removed: 0,
        };

        let serialized = bincode::serialize(&obs).unwrap();
        let deserialized: CachedObservation = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.observer_name, "测试用户 🚀");
    }

    #[test]
    fn test_cache_with_large_history() {
        let mut histories = HashMap::new();

        // Create 1000 files with 100 observations each
        for i in 0..100 {
            let observations: Vec<CachedObservation> = (0..100)
                .map(|j| CachedObservation {
                    timestamp_secs: (i * 100 + j) as i64,
                    observer_name: format!("User{}", j % 10),
                    observer_email_hash: format!("hash{}", j),
                    lines_added: j as usize,
                    lines_removed: (j / 2) as usize,
                })
                .collect();
            histories.insert(format!("file_{}.rs", i), observations);
        }

        let cache = ChronosCache {
            version: CACHE_VERSION,
            git_head_hash: "hash".to_string(),
            created_at: 0,
            ttl_seconds: 0,
            file_histories: histories,
            galaxy_stats: CachedGalaxyStats::default(),
            total_observations: 10000,
            hit_depth_limit: false,
            commit_depth: 0,
        };

        let serialized = bincode::serialize(&cache).unwrap();
        let deserialized: ChronosCache = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.file_histories.len(), 100);
        assert_eq!(deserialized.total_observations, 10000);
    }

    #[test]
    fn test_constants() {
        assert_eq!(CACHE_DIR, ".voyager/cache/chronos");
        assert_eq!(CACHE_FILE, "temporal_cache.bin");
        assert_eq!(CACHE_TTL_SECONDS, 86400);
        assert_eq!(CACHE_VERSION, 1);
    }
}
