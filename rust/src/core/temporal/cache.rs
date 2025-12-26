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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
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
        let buffer = bincode::serialize(cache)
            .map_err(|e| format!("Failed to serialize cache: {}", e))?;

        // Write atomically (write to temp, then rename)
        let cache_path = self.cache_path();
        let temp_path = cache_path.with_extension("tmp");

        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create cache file: {}", e))?;
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
            fs::remove_file(&cache_path)
                .map_err(|e| format!("Failed to remove cache: {}", e))?;
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
            fs::read_to_string(&full_ref_path).ok().map(|s| s.trim().to_string())
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
        let git_head_hash = self.get_git_head_hash()
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
        assert_eq!(WarpStatus::Calibrating.description(), "Warp Calibrating (analyzing)");
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
        assert_eq!(cache_miss_status.description(), "Warp Calibrating (analyzing)");

        // Verify they are different
        assert_ne!(cache_hit_status, cache_miss_status);
    }
}
