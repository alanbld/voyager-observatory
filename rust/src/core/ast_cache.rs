//! AST Parse Cache - Persistent cache for parsed ASTs
//!
//! Avoids re-parsing unchanged files across repeated `--survey` invocations.
//! Invalidation is per-file, keyed by content hash (the MD5 already computed
//! during directory walking) rather than mtime, so a cache hit survives
//! touches/checkouts that change mtime without changing content.
//!
//! Serialized as JSON rather than bincode: voyager-ast's IR types use
//! `#[serde(skip_serializing_if = ...)]` on several fields (by design, to
//! keep JSON AST exports compact), which relies on a self-describing format
//! to reconstruct skipped fields on the way back in. Bincode's positional
//! encoding has no field names to skip against, so it desyncs on any
//! omitted field; JSON's map-based encoding handles it correctly.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use voyager_ast::File as AstFile;

/// Cache directory name (relative to project root)
const CACHE_DIR: &str = ".voyager/cache/ast";

/// Cache file name
const CACHE_FILE: &str = "parse_cache.json";

/// Cache format version (bump to invalidate old caches)
const CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedParse {
    md5: String,
    file: AstFile,
}

/// A persistent, content-hash-keyed cache of parsed AST files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseCache {
    version: u32,
    entries: HashMap<String, CachedParse>,
}

impl ParseCache {
    /// Create a new, empty cache
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }

    /// Look up a cached parse result for `path`, valid only if `md5` matches
    /// the content hash the entry was cached under.
    pub fn get(&self, path: &str, md5: &str) -> Option<&AstFile> {
        self.entries
            .get(path)
            .filter(|entry| entry.md5 == md5)
            .map(|entry| &entry.file)
    }

    /// Insert (or overwrite) a parse result for a file
    pub fn insert(&mut self, path: String, md5: String, file: AstFile) {
        self.entries.insert(path, CachedParse { md5, file });
    }

    /// Number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache holds no entries
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages the on-disk parse cache for a project root
pub struct ParseCacheManager {
    cache_dir: PathBuf,
}

impl ParseCacheManager {
    /// Create a new cache manager for a project root
    pub fn new(project_root: &Path) -> Self {
        Self {
            cache_dir: project_root.join(CACHE_DIR),
        }
    }

    fn cache_path(&self) -> PathBuf {
        self.cache_dir.join(CACHE_FILE)
    }

    /// Load the cache, or an empty one if missing, corrupt, or from an
    /// incompatible format version.
    pub fn load(&self) -> ParseCache {
        self.try_load().unwrap_or_default()
    }

    fn try_load(&self) -> Option<ParseCache> {
        let mut file = File::open(self.cache_path()).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let cache: ParseCache = serde_json::from_slice(&buffer).ok()?;
        if cache.version != CACHE_VERSION {
            return None;
        }

        Some(cache)
    }

    /// Persist the cache, writing atomically (temp file + rename)
    pub fn save(&self, cache: &ParseCache) -> Result<(), String> {
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        let buffer = serde_json::to_vec(cache)
            .map_err(|e| format!("Failed to serialize parse cache: {}", e))?;

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

    /// Delete the cache file (forces a full re-parse on next run)
    pub fn invalidate(&self) -> Result<(), String> {
        let cache_path = self.cache_path();
        if cache_path.exists() {
            fs::remove_file(&cache_path).map_err(|e| format!("Failed to remove cache: {}", e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voyager_ast::{AdapterRegistry, LanguageId};

    fn sample_file() -> AstFile {
        AdapterRegistry::new()
            .parse("fn hello() {}", LanguageId::Rust)
            .expect("sample source should parse")
    }

    #[test]
    fn test_new_cache_is_empty() {
        let cache = ParseCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_get_miss_on_empty_cache() {
        let cache = ParseCache::new();
        assert!(cache.get("src/main.rs", "abc123").is_none());
    }

    #[test]
    fn test_insert_then_get_hit() {
        let mut cache = ParseCache::new();
        cache.insert(
            "src/main.rs".to_string(),
            "abc123".to_string(),
            sample_file(),
        );

        let hit = cache.get("src/main.rs", "abc123");
        assert!(hit.is_some());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_get_miss_on_md5_mismatch() {
        let mut cache = ParseCache::new();
        cache.insert(
            "src/main.rs".to_string(),
            "abc123".to_string(),
            sample_file(),
        );

        // Content changed since caching -> different md5 -> miss
        assert!(cache.get("src/main.rs", "def456").is_none());
    }

    #[test]
    fn test_insert_overwrites_existing_entry() {
        let mut cache = ParseCache::new();
        cache.insert(
            "src/main.rs".to_string(),
            "abc123".to_string(),
            sample_file(),
        );
        cache.insert(
            "src/main.rs".to_string(),
            "def456".to_string(),
            sample_file(),
        );

        assert_eq!(cache.len(), 1);
        assert!(cache.get("src/main.rs", "abc123").is_none());
        assert!(cache.get("src/main.rs", "def456").is_some());
    }

    #[test]
    fn test_default_matches_new() {
        let default_cache = ParseCache::default();
        assert!(default_cache.is_empty());
    }

    #[test]
    fn test_manager_cache_path() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_manager_path");
        let manager = ParseCacheManager::new(&temp_dir);
        assert_eq!(
            manager.cache_path(),
            temp_dir.join(".voyager/cache/ast/parse_cache.json")
        );
    }

    #[test]
    fn test_load_missing_file_returns_empty_cache() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_load_missing");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ParseCacheManager::new(&temp_dir);
        let cache = manager.load();
        assert!(cache.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_round_trip");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ParseCacheManager::new(&temp_dir);
        let mut cache = ParseCache::new();
        cache.insert("src/lib.rs".to_string(), "hash1".to_string(), sample_file());

        manager.save(&cache).unwrap();
        assert!(manager.cache_path().exists());

        let loaded = manager.load();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.get("src/lib.rs", "hash1").is_some());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_creates_cache_directory() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_creates_dir");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ParseCacheManager::new(&temp_dir);
        manager.save(&ParseCache::new()).unwrap();

        assert!(manager.cache_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_rejects_version_mismatch() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_version_mismatch");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ParseCacheManager::new(&temp_dir);
        let mut wrong_version_cache = ParseCache::new();
        wrong_version_cache.version = 999;

        fs::create_dir_all(&manager.cache_dir).unwrap();
        let buffer = serde_json::to_vec(&wrong_version_cache).unwrap();
        fs::write(manager.cache_path(), buffer).unwrap();

        // Version mismatch -> treated as absent, falls back to empty cache
        let loaded = manager.load();
        assert!(loaded.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_invalidate_removes_cache_file() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_invalidate");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ParseCacheManager::new(&temp_dir);
        manager.save(&ParseCache::new()).unwrap();
        assert!(manager.cache_path().exists());

        manager.invalidate().unwrap();
        assert!(!manager.cache_path().exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_invalidate_no_file_is_ok() {
        let temp_dir = std::env::temp_dir().join("test_ast_cache_invalidate_no_file");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = ParseCacheManager::new(&temp_dir);
        assert!(manager.invalidate().is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
