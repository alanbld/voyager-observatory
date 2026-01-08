//! Context Store v2 - Learning Layer
//!
//! This module implements adaptive file prioritization based on real-world utility feedback.
//! Files that are frequently useful to AI agents accumulate higher utility scores over time.
//!
//! # Architecture
//!
//! - `FileUtility`: Tracks utility score using Exponential Moving Average (EMA)
//! - `ContextStore`: Manages file utilities with persistence and privacy
//! - Integration with `LensManager` via Priority Blend formula

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Default EMA alpha coefficient for utility score updates
/// Higher alpha = more weight on recent feedback, faster adaptation
/// Lower alpha = more weight on historical data, slower but more stable
pub const DEFAULT_ALPHA: f64 = 0.3;

/// File utility tracking using Exponential Moving Average
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUtility {
    /// Utility score (0.0 to 1.0)
    pub score: f64,

    /// Number of feedback entries received
    pub access_count: u32,

    /// Last update timestamp (ISO 8601)
    #[serde(default)]
    pub last_accessed: String,

    /// Optional tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for FileUtility {
    fn default() -> Self {
        Self {
            score: 0.5, // Neutral starting point
            access_count: 0,
            last_accessed: String::new(),
            tags: Vec::new(),
        }
    }
}

impl FileUtility {
    /// Create a new FileUtility with the given initial score
    pub fn new(initial_score: f64) -> Self {
        Self {
            score: initial_score.clamp(0.0, 1.0),
            access_count: 0,
            last_accessed: String::new(),
            tags: Vec::new(),
        }
    }

    /// Update the utility score using Exponential Moving Average
    ///
    /// Formula: new_score = (alpha * session_utility) + ((1.0 - alpha) * current_score)
    ///
    /// # Arguments
    /// * `session_utility` - The utility observed in the current session (0.0 to 1.0)
    /// * `alpha` - The smoothing factor (0.0 to 1.0), defaults to 0.3
    pub fn update(&mut self, session_utility: f64, alpha: f64) {
        let clamped_utility = session_utility.clamp(0.0, 1.0);
        let clamped_alpha = alpha.clamp(0.0, 1.0);

        self.score = (clamped_alpha * clamped_utility) + ((1.0 - clamped_alpha) * self.score);
        self.access_count += 1;
        self.last_accessed = chrono::Utc::now().to_rfc3339();
    }

    /// Apply a utility bump (e.g., when a file is zoomed into)
    ///
    /// Uses the standard EMA but with a small positive adjustment
    pub fn bump(&mut self, bump_amount: f64, alpha: f64) {
        let new_utility = (self.score + bump_amount).clamp(0.0, 1.0);
        self.update(new_utility, alpha);
    }
}

/// Context Store v2 - Persistent file utility tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStore {
    /// Store version for forward compatibility
    pub version: String,

    /// File utilities indexed by path (or hashed path if privacy enabled)
    pub files: HashMap<String, FileUtility>,

    /// Lens-specific learning profiles
    #[serde(default)]
    pub lens_profiles: HashMap<String, LensProfile>,

    /// Whether paths are hashed for privacy
    #[serde(default)]
    pub paths_hashed: bool,
}

/// Lens-specific learning profile
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LensProfile {
    /// Learned priority adjustments per file pattern
    #[serde(default)]
    pub learned_priorities: HashMap<String, i32>,

    /// Overall effectiveness score of this lens
    #[serde(default)]
    pub effectiveness_score: f64,
}

impl Default for ContextStore {
    fn default() -> Self {
        Self {
            version: "2.0.0".to_string(),
            files: HashMap::new(),
            lens_profiles: HashMap::new(),
            paths_hashed: false,
        }
    }
}

impl ContextStore {
    /// Create a new empty ContextStore
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a ContextStore with privacy-hashing enabled
    pub fn with_privacy() -> Self {
        Self {
            paths_hashed: true,
            ..Self::default()
        }
    }

    /// Hash a file path for privacy
    fn hash_path(path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get the storage key for a file path (hashed if privacy enabled)
    fn storage_key(&self, path: &str) -> String {
        if self.paths_hashed {
            Self::hash_path(path)
        } else {
            path.to_string()
        }
    }

    /// Get utility for a file path
    pub fn get_utility(&self, path: &str) -> Option<&FileUtility> {
        let key = self.storage_key(path);
        self.files.get(&key)
    }

    /// Get utility score for a file (returns 0.5 default if not found)
    pub fn get_utility_score(&self, path: &str) -> f64 {
        self.get_utility(path).map(|u| u.score).unwrap_or(0.5)
    }

    /// Report utility for a file
    ///
    /// # Arguments
    /// * `path` - File path
    /// * `utility` - Utility score (0.0 to 1.0)
    /// * `alpha` - EMA smoothing factor (default: 0.3)
    pub fn report_utility(&mut self, path: &str, utility: f64, alpha: f64) {
        let key = self.storage_key(path);

        let file_utility = self.files.entry(key).or_default();
        file_utility.update(utility, alpha);
    }

    /// Apply a utility bump (e.g., when a file is zoomed)
    pub fn bump_utility(&mut self, path: &str, bump: f64, alpha: f64) {
        let key = self.storage_key(path);

        let file_utility = self.files.entry(key).or_default();
        file_utility.bump(bump, alpha);
    }

    /// Calculate blended priority for a file
    ///
    /// Priority Blend: final = (static_priority * 0.7) + (learned_score * 100 * 0.3)
    ///
    /// # Arguments
    /// * `path` - File path
    /// * `static_priority` - Priority from lens configuration
    ///
    /// # Returns
    /// Blended priority value
    pub fn blend_priority(&self, path: &str, static_priority: i32) -> i32 {
        let learned_score = self.get_utility_score(path);

        let static_component = static_priority as f64 * 0.7;
        let learned_component = learned_score * 100.0 * 0.3;

        (static_component + learned_component).round() as i32
    }

    /// Get total number of tracked files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Clear all stored utilities
    pub fn clear(&mut self) {
        self.files.clear();
        self.lens_profiles.clear();
    }

    /// Load from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load from file path, returning default if file doesn't exist or is malformed
    pub fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => Self::from_json(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to file path
    pub fn save_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(path, json)
    }

    /// Get the default store path for a project
    pub fn default_path(project_root: &Path) -> std::path::PathBuf {
        project_root.join(".pm_encoder").join("context_store.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Phase 1: EMA Convergence Tests
    // ============================================================

    #[test]
    fn test_file_utility_default() {
        let utility = FileUtility::default();
        assert_eq!(utility.score, 0.5);
        assert_eq!(utility.access_count, 0);
    }

    #[test]
    fn test_file_utility_new() {
        let utility = FileUtility::new(0.8);
        assert_eq!(utility.score, 0.8);

        // Test clamping
        let utility_over = FileUtility::new(1.5);
        assert_eq!(utility_over.score, 1.0);

        let utility_under = FileUtility::new(-0.5);
        assert_eq!(utility_under.score, 0.0);
    }

    #[test]
    fn test_ema_single_update() {
        let mut utility = FileUtility::new(0.5);

        // Update with 1.0 utility, alpha=0.3
        // new = (0.3 * 1.0) + (0.7 * 0.5) = 0.3 + 0.35 = 0.65
        utility.update(1.0, 0.3);

        assert!((utility.score - 0.65).abs() < 0.001);
        assert_eq!(utility.access_count, 1);
    }

    #[test]
    fn test_ema_convergence_to_high() {
        let mut utility = FileUtility::new(0.5);

        // Multiple updates with 1.0 should converge toward 1.0
        for _ in 0..10 {
            utility.update(1.0, 0.3);
        }

        // After 10 updates, should be close to 1.0
        assert!(
            utility.score > 0.95,
            "Score should converge to 1.0, got {}",
            utility.score
        );
    }

    #[test]
    fn test_ema_convergence_to_low() {
        let mut utility = FileUtility::new(0.5);

        // Multiple updates with 0.0 should converge toward 0.0
        for _ in 0..10 {
            utility.update(0.0, 0.3);
        }

        // After 10 updates, should be close to 0.0
        assert!(
            utility.score < 0.05,
            "Score should converge to 0.0, got {}",
            utility.score
        );
    }

    #[test]
    fn test_ema_stability_with_consistent_feedback() {
        let mut utility = FileUtility::new(0.5);

        // Update with same value repeatedly - should converge exactly
        for _ in 0..20 {
            utility.update(0.8, 0.3);
        }

        assert!(
            (utility.score - 0.8).abs() < 0.01,
            "Should converge to 0.8, got {}",
            utility.score
        );
    }

    #[test]
    fn test_ema_mixed_feedback() {
        let mut utility = FileUtility::new(0.5);

        // Alternate between high and low feedback
        for i in 0..10 {
            let feedback = if i % 2 == 0 { 1.0 } else { 0.0 };
            utility.update(feedback, 0.3);
        }

        // Should be somewhere in the middle, slightly below 0.5 due to order
        assert!(
            utility.score > 0.3 && utility.score < 0.7,
            "Score should be in middle range, got {}",
            utility.score
        );
    }

    #[test]
    fn test_ema_alpha_high() {
        // High alpha = fast adaptation
        let mut utility = FileUtility::new(0.5);
        utility.update(1.0, 0.9);

        // With alpha=0.9: new = (0.9 * 1.0) + (0.1 * 0.5) = 0.95
        assert!((utility.score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_ema_alpha_low() {
        // Low alpha = slow adaptation
        let mut utility = FileUtility::new(0.5);
        utility.update(1.0, 0.1);

        // With alpha=0.1: new = (0.1 * 1.0) + (0.9 * 0.5) = 0.55
        assert!((utility.score - 0.55).abs() < 0.001);
    }

    #[test]
    fn test_utility_bump() {
        let mut utility = FileUtility::new(0.5);

        // Bump by 0.1
        utility.bump(0.1, 0.3);

        // Expected: update with 0.6, so (0.3 * 0.6) + (0.7 * 0.5) = 0.53
        assert!((utility.score - 0.53).abs() < 0.01);
    }

    // ============================================================
    // Phase 2: Context Store Tests
    // ============================================================

    #[test]
    fn test_context_store_default() {
        let store = ContextStore::new();
        assert_eq!(store.version, "2.0.0");
        assert!(store.files.is_empty());
        assert!(!store.paths_hashed);
    }

    #[test]
    fn test_context_store_with_privacy() {
        let store = ContextStore::with_privacy();
        assert!(store.paths_hashed);
    }

    #[test]
    fn test_report_and_get_utility() {
        let mut store = ContextStore::new();

        store.report_utility("src/main.rs", 0.9, DEFAULT_ALPHA);

        let utility = store.get_utility("src/main.rs").unwrap();
        assert!(utility.score > 0.5);
        assert_eq!(utility.access_count, 1);
    }

    #[test]
    fn test_get_utility_score_default() {
        let store = ContextStore::new();

        // Unknown file returns default 0.5
        assert_eq!(store.get_utility_score("unknown.py"), 0.5);
    }

    #[test]
    fn test_multiple_reports_converge() {
        let mut store = ContextStore::new();

        // Report high utility multiple times
        for _ in 0..5 {
            store.report_utility("important.py", 1.0, DEFAULT_ALPHA);
        }

        let score = store.get_utility_score("important.py");
        assert!(score > 0.9, "Score should converge high, got {}", score);
    }

    #[test]
    fn test_bump_utility() {
        let mut store = ContextStore::new();

        // Initialize with neutral
        store.report_utility("zoomed.rs", 0.5, DEFAULT_ALPHA);
        let before = store.get_utility_score("zoomed.rs");

        // Bump by 0.1
        store.bump_utility("zoomed.rs", 0.1, DEFAULT_ALPHA);
        let after = store.get_utility_score("zoomed.rs");

        assert!(after > before, "Bump should increase score");
    }

    // ============================================================
    // Phase 2: Priority Blend Tests
    // ============================================================

    #[test]
    fn test_blend_priority_neutral() {
        let store = ContextStore::new();

        // Unknown file has 0.5 score
        // Blend: (100 * 0.7) + (0.5 * 100 * 0.3) = 70 + 15 = 85
        let blended = store.blend_priority("unknown.py", 100);
        assert_eq!(blended, 85);
    }

    #[test]
    fn test_blend_priority_high_utility() {
        let mut store = ContextStore::new();

        // Set high utility
        for _ in 0..10 {
            store.report_utility("important.py", 1.0, DEFAULT_ALPHA);
        }

        // Blend with static priority 50
        // Score ~1.0, so: (50 * 0.7) + (1.0 * 100 * 0.3) = 35 + 30 = 65
        let blended = store.blend_priority("important.py", 50);
        assert!(
            blended >= 60 && blended <= 70,
            "Expected ~65, got {}",
            blended
        );
    }

    #[test]
    fn test_blend_priority_low_utility() {
        let mut store = ContextStore::new();

        // Set low utility
        for _ in 0..10 {
            store.report_utility("useless.txt", 0.0, DEFAULT_ALPHA);
        }

        // Blend with static priority 50
        // Score ~0.0, so: (50 * 0.7) + (0.0 * 100 * 0.3) = 35 + 0 = 35
        let blended = store.blend_priority("useless.txt", 50);
        assert!(
            blended >= 30 && blended <= 40,
            "Expected ~35, got {}",
            blended
        );
    }

    // ============================================================
    // Phase 3: Persistence Tests
    // ============================================================

    #[test]
    fn test_json_serialization() {
        let mut store = ContextStore::new();
        store.report_utility("test.py", 0.8, DEFAULT_ALPHA);

        let json = store.to_json().unwrap();
        assert!(json.contains("test.py"));
        assert!(json.contains("2.0.0"));
    }

    #[test]
    fn test_json_deserialization() {
        let json = r#"{
            "version": "2.0.0",
            "files": {
                "test.py": {
                    "score": 0.75,
                    "access_count": 5,
                    "last_accessed": "",
                    "tags": []
                }
            },
            "lens_profiles": {},
            "paths_hashed": false
        }"#;

        let store = ContextStore::from_json(json).unwrap();
        assert_eq!(store.get_utility_score("test.py"), 0.75);
    }

    #[test]
    fn test_malformed_json_returns_default() {
        let bad_json = "{ not valid json }";
        let store = ContextStore::from_json(bad_json);
        assert!(store.is_err());
    }

    #[test]
    fn test_privacy_hashing() {
        let mut store = ContextStore::with_privacy();
        store.report_utility("secret/path.py", 0.9, DEFAULT_ALPHA);

        let json = store.to_json().unwrap();

        // The actual path should NOT appear in JSON
        assert!(!json.contains("secret/path.py"));
        // But a hash should
        assert!(json.contains(&ContextStore::hash_path("secret/path.py")));
    }

    #[test]
    fn test_hash_path_deterministic() {
        let hash1 = ContextStore::hash_path("test/file.py");
        let hash2 = ContextStore::hash_path("test/file.py");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_path_different_inputs() {
        let hash1 = ContextStore::hash_path("file1.py");
        let hash2 = ContextStore::hash_path("file2.py");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_file_operations() {
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_store_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let store_path = temp_dir.join(".pm_encoder").join("context_store.json");

        // Create and save store
        let mut store = ContextStore::new();
        store.report_utility("main.py", 0.95, DEFAULT_ALPHA);
        store.save_to_file(&store_path).unwrap();

        assert!(store_path.exists());

        // Load store
        let loaded = ContextStore::load_from_file(&store_path);
        let score = loaded.get_utility_score("main.py");
        assert!(score > 0.6, "Loaded score should be high, got {}", score);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = Path::new("/nonexistent/path/store.json");
        let store = ContextStore::load_from_file(path);
        assert!(store.files.is_empty());
    }

    #[test]
    fn test_default_path() {
        let project_root = Path::new("/home/user/project");
        let store_path = ContextStore::default_path(project_root);
        assert_eq!(
            store_path,
            Path::new("/home/user/project/.pm_encoder/context_store.json")
        );
    }

    #[test]
    fn test_file_count() {
        let mut store = ContextStore::new();
        assert_eq!(store.file_count(), 0);

        store.report_utility("a.py", 0.5, 0.3);
        store.report_utility("b.py", 0.5, 0.3);
        store.report_utility("c.py", 0.5, 0.3);

        assert_eq!(store.file_count(), 3);
    }

    #[test]
    fn test_clear_store() {
        let mut store = ContextStore::new();
        store.report_utility("test.py", 0.9, 0.3);
        assert_eq!(store.file_count(), 1);

        store.clear();
        assert_eq!(store.file_count(), 0);
    }

    // ============================================================
    // Phase 4: Zoom Bump Integration Tests
    // ============================================================

    #[test]
    fn test_zoom_bump_increases_utility() {
        let mut store = ContextStore::new();

        // Initialize file
        store.report_utility("zoomed.rs", 0.5, DEFAULT_ALPHA);

        // Simulate zoom bump (+0.05)
        let before = store.get_utility_score("zoomed.rs");
        store.bump_utility("zoomed.rs", 0.05, DEFAULT_ALPHA);
        let after = store.get_utility_score("zoomed.rs");

        assert!(after > before, "Zoom bump should increase utility");
    }

    #[test]
    fn test_repeated_zooms_increase_utility() {
        let mut store = ContextStore::new();

        // Multiple zooms should keep increasing utility
        for _ in 0..5 {
            store.bump_utility("hot_file.py", 0.05, DEFAULT_ALPHA);
        }

        let score = store.get_utility_score("hot_file.py");
        assert!(
            score > 0.55,
            "Multiple zooms should increase score, got {}",
            score
        );
    }
}
