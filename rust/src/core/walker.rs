//! Directory traversal for pm_encoder
//!
//! This module provides the FileWalker trait and default implementation
//! for walking directory trees and discovering files.

use crate::core::error::{EncoderError, Result};
use crate::core::models::FileEntry;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;
use std::time::SystemTime;

#[cfg(test)]
use mockall::automock;

/// Normalize path separators for cross-platform compatibility.
/// - Converts Windows backslashes to forward slashes
/// - Strips Windows UNC prefix `\\?\` if present
pub fn normalize_path_separators(path: &str) -> String {
    let mut normalized = path.to_string();

    // Strip Windows UNC prefix (\\?\ or \\.\)
    if normalized.starts_with(r"\\?\") || normalized.starts_with(r"\\.\") {
        normalized = normalized[4..].to_string();
    }

    // Convert backslashes to forward slashes
    normalized.replace('\\', "/")
}

/// Trait for file system walking
///
/// This trait allows for mocking in tests and alternative implementations
/// (e.g., virtual file systems, remote sources).
#[cfg_attr(test, automock)]
pub trait FileWalker: Send + Sync {
    /// Walk a directory and return file entries
    fn walk(&self, root: &str, config: &WalkConfig) -> Result<Vec<FileEntry>>;

    /// Check if a path matches ignore patterns
    fn should_ignore(&self, path: &str, patterns: &[String]) -> bool;

    /// Check if a file is too large
    fn is_too_large(&self, size: u64, limit: u64) -> bool {
        size > limit
    }
}

/// Configuration for directory walking
#[derive(Debug, Clone)]
pub struct WalkConfig {
    /// Patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Patterns to include (empty = all)
    pub include_patterns: Vec<String>,
    /// Maximum file size in bytes
    pub max_file_size: u64,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "__pycache__".to_string(),
                "*.pyc".to_string(),
                ".DS_Store".to_string(),
                "target".to_string(),
            ],
            include_patterns: vec![],
            max_file_size: 1_048_576,
        }
    }
}

/// Default file walker implementation
pub struct DefaultWalker;

impl DefaultWalker {
    /// Create a new DefaultWalker
    pub fn new() -> Self {
        Self
    }

    /// Build a GlobSet from patterns
    fn build_globset(patterns: &[String]) -> Option<GlobSet> {
        if patterns.is_empty() {
            return None;
        }

        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        builder.build().ok()
    }

    /// Check if path matches any pattern
    fn matches_patterns(path: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            // Check for exact match
            if path == pattern {
                return true;
            }

            // Check for directory component match
            for component in path.split('/') {
                if component == pattern {
                    return true;
                }
            }

            // Check for glob match
            if let Ok(glob) = Glob::new(pattern) {
                let matcher = glob.compile_matcher();
                if matcher.is_match(path) {
                    return true;
                }
            }

            // Check for prefix match (directory)
            if path.starts_with(&format!("{}/", pattern)) {
                return true;
            }
        }
        false
    }
}

impl Default for DefaultWalker {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWalker for DefaultWalker {
    fn walk(&self, root: &str, config: &WalkConfig) -> Result<Vec<FileEntry>> {
        let root_path = Path::new(root);
        if !root_path.exists() {
            return Err(EncoderError::DirectoryNotFound {
                path: root_path.to_path_buf(),
            });
        }
        if !root_path.is_dir() {
            return Err(EncoderError::invalid_config(format!(
                "'{}' is not a directory",
                root
            )));
        }

        let include_set = Self::build_globset(&config.include_patterns);
        let mut entries = Vec::new();

        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            // Skip directories
            if entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path();
            let relative_path = normalize_path_separators(
                &path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy(),
            );

            // Skip ignored files
            if self.should_ignore(&relative_path, &config.ignore_patterns) {
                continue;
            }

            // Check include patterns if specified
            if let Some(ref include_set) = include_set {
                if !include_set.is_match(&relative_path) {
                    continue;
                }
            }

            // Check file size
            let metadata = entry.metadata().ok();
            if let Some(ref meta) = metadata {
                if self.is_too_large(meta.len(), config.max_file_size) {
                    continue;
                }
            }

            // Read file content
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(_) => continue,
            };

            // Skip binary files
            if is_binary(&bytes) {
                continue;
            }

            // Convert to string
            let content = match read_file_content(&bytes) {
                Some(c) => c,
                None => continue,
            };

            // Get timestamps and size
            let (mtime, ctime, size) = metadata
                .map(|m| {
                    let mtime = m.modified()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let ctime = m.created()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(mtime);
                    let size = m.len();
                    (mtime, ctime, size)
                })
                .unwrap_or((0, 0, content.len() as u64));

            entries.push(FileEntry::new(&relative_path, content).with_timestamps(mtime, ctime).with_size(size));
        }

        Ok(entries)
    }

    fn should_ignore(&self, path: &str, patterns: &[String]) -> bool {
        Self::matches_patterns(path, patterns)
    }
}

/// Check if content appears to be binary
pub fn is_binary(content: &[u8]) -> bool {
    // Empty is not binary
    if content.is_empty() {
        return false;
    }

    // Check first 8KB for null bytes (common binary indicator)
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

/// Read file content, handling encoding
pub fn read_file_content(bytes: &[u8]) -> Option<String> {
    // Try UTF-8 first
    if let Ok(s) = std::str::from_utf8(bytes) {
        // Normalize line endings
        return Some(s.replace("\r\n", "\n"));
    }

    // Try lossy conversion
    let s = String::from_utf8_lossy(bytes);
    if s.chars().filter(|c| *c == '\u{FFFD}').count() < s.len() / 10 {
        Some(s.replace("\r\n", "\n"))
    } else {
        None // Too many replacement characters, likely binary
    }
}

// ============================================================================
// SmartWalker - Intelligent file walker with boundary awareness
// ============================================================================

use ignore::{WalkBuilder, WalkState};
use std::sync::mpsc;
use crate::core::manifest::ProjectManifest;

/// Hard-coded exclusion patterns (hygiene layer).
/// These are ALWAYS excluded regardless of .gitignore.
const HYGIENE_EXCLUSIONS: &[&str] = &[
    // Version control
    ".git",
    ".hg",
    ".svn",
    // Package managers / dependencies
    "node_modules",
    ".npm",
    ".yarn",
    // Python environments
    ".venv",
    "venv",
    "env",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".eggs",
    // Build artifacts
    "target",
    "dist",
    "build",
    "out",
    "_build",
    ".build",
    // IDE / Editor
    ".idea",
    ".vscode",
    // OS artifacts
    ".DS_Store",
    "Thumbs.db",
];

/// Wildcard exclusion patterns (matched by suffix).
const HYGIENE_WILDCARDS: &[&str] = &[
    ".egg-info",
    ".swp",
    ".swo",
    ".pyc",
];

/// Result of walking a directory with SmartWalker.
#[derive(Debug, Clone)]
pub struct WalkEntry {
    /// Absolute path to the file.
    pub path: std::path::PathBuf,
    /// Relative path from project root.
    pub relative_path: std::path::PathBuf,
    /// Whether this is a file (always true for walk results).
    pub is_file: bool,
}

/// Configuration for SmartWalker.
#[derive(Debug, Clone)]
pub struct SmartWalkConfig {
    /// Follow symlinks (default: false for safety).
    pub follow_symlinks: bool,

    /// Respect .gitignore files (default: true).
    pub respect_gitignore: bool,

    /// Include hidden files (default: false).
    pub include_hidden: bool,

    /// Maximum depth to traverse (None = unlimited).
    pub max_depth: Option<usize>,

    /// Additional patterns to exclude.
    pub extra_excludes: Vec<String>,

    /// Maximum file size in bytes.
    pub max_file_size: u64,
}

impl Default for SmartWalkConfig {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            respect_gitignore: true,
            include_hidden: false,
            max_depth: None,
            extra_excludes: vec![],
            max_file_size: 1_048_576, // 1MB
        }
    }
}

/// Intelligent file walker with boundary awareness.
///
/// SmartWalker uses the `ignore` crate for efficient gitignore-aware traversal
/// and applies a "hygiene layer" that always excludes .venv, node_modules, etc.
pub struct SmartWalker {
    root: std::path::PathBuf,
    manifest: ProjectManifest,
    config: SmartWalkConfig,
}

impl SmartWalker {
    /// Create a new SmartWalker for the given path.
    pub fn new(path: &Path) -> Self {
        let manifest = ProjectManifest::detect(path);
        Self {
            root: manifest.root.clone(),
            manifest,
            config: SmartWalkConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(path: &Path, config: SmartWalkConfig) -> Self {
        let manifest = ProjectManifest::detect(path);
        Self {
            root: manifest.root.clone(),
            manifest,
            config,
        }
    }

    /// Get the detected project manifest.
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    /// Get the project root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Check if a path should be excluded by hygiene rules.
    pub fn is_hygiene_excluded(path: &Path) -> bool {
        path.components().any(|c| {
            let name = c.as_os_str().to_string_lossy();

            // Check exact matches
            if HYGIENE_EXCLUSIONS.iter().any(|&pattern| name == pattern) {
                return true;
            }

            // Check wildcard patterns (suffix match)
            if HYGIENE_WILDCARDS.iter().any(|&pattern| name.ends_with(pattern)) {
                return true;
            }

            false
        })
    }

    /// Walk the directory and collect file entries.
    pub fn walk(&self) -> std::result::Result<Vec<WalkEntry>, String> {
        let mut builder = WalkBuilder::new(&self.root);

        // Configure based on SmartWalkConfig
        builder
            .follow_links(self.config.follow_symlinks)
            .git_ignore(self.config.respect_gitignore)
            .git_global(self.config.respect_gitignore)
            .git_exclude(self.config.respect_gitignore)
            .hidden(!self.config.include_hidden);

        if let Some(depth) = self.config.max_depth {
            builder.max_depth(Some(depth));
        }

        // Collect entries
        let mut entries = Vec::new();

        for result in builder.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();

                    // Apply hygiene exclusions
                    if Self::is_hygiene_excluded(path) {
                        continue;
                    }

                    // Only include files (not directories)
                    if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        // Check file size
                        if let Ok(meta) = entry.metadata() {
                            if meta.len() > self.config.max_file_size {
                                continue;
                            }
                        }

                        let relative = path
                            .strip_prefix(&self.root)
                            .unwrap_or(path)
                            .to_path_buf();

                        entries.push(WalkEntry {
                            path: path.to_path_buf(),
                            relative_path: relative,
                            is_file: true,
                        });
                    }
                }
                Err(e) => {
                    // Check if this is a broken symlink error (silently skip when not following symlinks)
                    // The error message typically contains "No such file or directory"
                    // or the IO error kind is NotFound
                    let error_str = e.to_string();
                    let is_not_found = error_str.contains("No such file or directory")
                        || error_str.contains("cannot access")
                        || e.io_error().map_or(false, |io| {
                            io.kind() == std::io::ErrorKind::NotFound
                        });

                    // When not following symlinks, silently skip broken symlinks
                    // When following symlinks, report all errors including broken links
                    if self.config.follow_symlinks || !is_not_found {
                        eprintln!("[WARN] Walk error: {}", e);
                    }
                    // else: silently skip broken symlinks (default behavior)
                }
            }
        }

        Ok(entries)
    }

    /// Walk with parallel processing (for large repos).
    pub fn walk_parallel(&self) -> std::result::Result<Vec<WalkEntry>, String> {
        let mut builder = WalkBuilder::new(&self.root);

        builder
            .follow_links(self.config.follow_symlinks)
            .git_ignore(self.config.respect_gitignore)
            .hidden(!self.config.include_hidden);

        let (tx, rx) = mpsc::channel();
        let root = self.root.clone();
        let max_file_size = self.config.max_file_size;

        builder.build_parallel().run(|| {
            let tx = tx.clone();
            let root = root.clone();

            Box::new(move |entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return WalkState::Continue,
                };

                let path = entry.path();

                // Hygiene check - skip entire subtree for directories
                if Self::is_hygiene_excluded(path) {
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        return WalkState::Skip;
                    }
                    return WalkState::Continue;
                }

                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    // Check file size
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() > max_file_size {
                            return WalkState::Continue;
                        }
                    }

                    let relative = path.strip_prefix(&root).unwrap_or(path).to_path_buf();

                    let _ = tx.send(WalkEntry {
                        path: path.to_path_buf(),
                        relative_path: relative,
                        is_file: true,
                    });
                }

                WalkState::Continue
            })
        });

        drop(tx); // Close sender

        let entries: Vec<_> = rx.into_iter().collect();
        Ok(entries)
    }

    /// Convert walk entries to FileEntry format for compatibility.
    pub fn walk_as_file_entries(&self) -> Result<Vec<FileEntry>> {
        let walk_entries = self.walk().map_err(EncoderError::invalid_config)?;

        let mut file_entries = Vec::new();

        for entry in walk_entries {
            // Read file content
            let bytes = match std::fs::read(&entry.path) {
                Ok(b) => b,
                Err(_) => continue,
            };

            // Skip binary files
            if is_binary(&bytes) {
                continue;
            }

            // Convert to string
            let content = match read_file_content(&bytes) {
                Some(c) => c,
                None => continue,
            };

            // Get timestamps
            let (mtime, ctime) = std::fs::metadata(&entry.path)
                .map(|m| {
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let ctime = m
                        .created()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(mtime);
                    (mtime, ctime)
                })
                .unwrap_or((0, 0));

            file_entries.push(
                FileEntry::new(
                    normalize_path_separators(&entry.relative_path.to_string_lossy()),
                    content,
                )
                .with_timestamps(mtime, ctime),
            );
        }

        Ok(file_entries)
    }
}

impl FileWalker for SmartWalker {
    fn walk(&self, _root: &str, config: &WalkConfig) -> Result<Vec<FileEntry>> {
        // Create a new SmartWalker with merged config
        let smart_config = SmartWalkConfig {
            max_file_size: config.max_file_size,
            extra_excludes: config.ignore_patterns.clone(),
            ..self.config.clone()
        };

        let walker = SmartWalker {
            root: self.root.clone(),
            manifest: self.manifest.clone(),
            config: smart_config,
        };

        walker.walk_as_file_entries()
    }

    fn should_ignore(&self, path: &str, _patterns: &[String]) -> bool {
        Self::is_hygiene_excluded(Path::new(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_normalize_path_separators_backslashes() {
        assert_eq!(normalize_path_separators(r"src\main.rs"), "src/main.rs");
        assert_eq!(normalize_path_separators(r"a\b\c\d"), "a/b/c/d");
    }

    #[test]
    fn test_normalize_path_separators_unc_prefix() {
        assert_eq!(
            normalize_path_separators(r"\\?\C:\project\src\main.rs"),
            "C:/project/src/main.rs"
        );
        assert_eq!(normalize_path_separators(r"\\.\device"), "device");
    }

    #[test]
    fn test_normalize_path_separators_unix_unchanged() {
        assert_eq!(normalize_path_separators("src/main.rs"), "src/main.rs");
        assert_eq!(normalize_path_separators("/home/user/file.txt"), "/home/user/file.txt");
    }

    #[test]
    fn test_walk_config_default() {
        let config = WalkConfig::default();
        assert!(config.ignore_patterns.contains(&".git".to_string()));
        assert_eq!(config.max_file_size, 1_048_576);
    }

    #[test]
    fn test_is_binary_empty() {
        assert!(!is_binary(&[]));
    }

    #[test]
    fn test_is_binary_with_null() {
        assert!(is_binary(&[0x00, 0x01, 0x02]));
    }

    #[test]
    fn test_is_binary_text() {
        assert!(!is_binary(b"Hello, world!"));
    }

    #[test]
    fn test_read_file_content_utf8() {
        let content = read_file_content(b"Hello, world!");
        assert_eq!(content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_read_file_content_crlf() {
        let content = read_file_content(b"line1\r\nline2");
        assert_eq!(content, Some("line1\nline2".to_string()));
    }

    #[test]
    fn test_default_walker_nonexistent() {
        let walker = DefaultWalker::new();
        let config = WalkConfig::default();
        let result = walker.walk("/nonexistent/path/xyz", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_walker_walk() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, world!").unwrap();

        let walker = DefaultWalker::new();
        let config = WalkConfig::default();
        let entries = walker.walk(temp_dir.path().to_str().unwrap(), &config).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].path.ends_with("test.txt"));
        assert_eq!(entries[0].content, "Hello, world!");
    }

    #[test]
    fn test_should_ignore() {
        let walker = DefaultWalker::new();
        assert!(walker.should_ignore(".git/config", &vec![".git".to_string()]));
        assert!(walker.should_ignore("node_modules/pkg/index.js", &vec!["node_modules".to_string()]));
        assert!(!walker.should_ignore("src/main.rs", &vec![".git".to_string()]));
    }

    #[test]
    fn test_is_too_large() {
        let walker = DefaultWalker::new();
        assert!(walker.is_too_large(2_000_000, 1_000_000));
        assert!(!walker.is_too_large(500_000, 1_000_000));
    }

    #[test]
    fn test_matches_patterns_glob() {
        assert!(DefaultWalker::matches_patterns("test.pyc", &vec!["*.pyc".to_string()]));
        assert!(!DefaultWalker::matches_patterns("test.py", &vec!["*.pyc".to_string()]));
    }

    // ========================================================================
    // SmartWalker Tests
    // ========================================================================

    fn create_pollution_test_project(tmp: &TempDir) {
        // Create project structure
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::create_dir_all(tmp.path().join(".venv/lib")).unwrap();
        fs::create_dir_all(tmp.path().join("node_modules/lodash")).unwrap();
        fs::create_dir_all(tmp.path().join("target/debug")).unwrap();
        fs::create_dir_all(tmp.path().join("__pycache__")).unwrap();
        fs::create_dir_all(tmp.path().join(".git/objects")).unwrap();

        // Create files
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("src/lib.rs"), "pub fn hello() {}").unwrap();
        fs::write(tmp.path().join(".venv/lib/secrets.py"), "SECRET='bad'").unwrap();
        fs::write(
            tmp.path().join("node_modules/lodash/index.js"),
            "module.exports = {}",
        )
        .unwrap();
        fs::write(tmp.path().join("target/debug/binary"), "ELF").unwrap();
        fs::write(tmp.path().join("__pycache__/module.pyc"), "bytecode").unwrap();
    }

    #[test]
    fn test_smart_walker_excludes_venv() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        // Should include src files
        assert!(paths.iter().any(|p| p.contains("main.rs")));
        assert!(paths.iter().any(|p| p.contains("lib.rs")));

        // Should exclude .venv
        assert!(!paths.iter().any(|p| p.contains(".venv")));
        assert!(!paths.iter().any(|p| p.contains("secrets.py")));
    }

    #[test]
    fn test_smart_walker_excludes_node_modules() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(!paths.iter().any(|p| p.contains("node_modules")));
        assert!(!paths.iter().any(|p| p.contains("lodash")));
    }

    #[test]
    fn test_smart_walker_excludes_target() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(!paths.iter().any(|p| p.contains("target")));
    }

    #[test]
    fn test_smart_walker_excludes_pycache() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(!paths.iter().any(|p| p.contains("__pycache__")));
        assert!(!paths.iter().any(|p| p.contains(".pyc")));
    }

    #[test]
    fn test_smart_walker_excludes_git() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(!paths.iter().any(|p| p.contains(".git")));
    }

    #[test]
    fn test_hygiene_exclusion_check() {
        assert!(SmartWalker::is_hygiene_excluded(Path::new(
            "/project/.venv/lib/foo.py"
        )));
        assert!(SmartWalker::is_hygiene_excluded(Path::new(
            "/project/node_modules/x/y.js"
        )));
        assert!(SmartWalker::is_hygiene_excluded(Path::new(
            "/project/target/debug/bin"
        )));
        assert!(SmartWalker::is_hygiene_excluded(Path::new(
            "/project/__pycache__/x.pyc"
        )));
        assert!(SmartWalker::is_hygiene_excluded(Path::new(
            "/project/.vscode/settings.json"
        )));
        assert!(SmartWalker::is_hygiene_excluded(Path::new(
            "/project/pkg.egg-info/PKG-INFO"
        )));

        assert!(!SmartWalker::is_hygiene_excluded(Path::new(
            "/project/src/main.rs"
        )));
        assert!(!SmartWalker::is_hygiene_excluded(Path::new(
            "/project/lib/utils.py"
        )));
    }

    #[test]
    fn test_smart_walker_parallel_same_result() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());

        let sequential = walker.walk().unwrap();
        let parallel = walker.walk_parallel().unwrap();

        // Same count (order may differ)
        assert_eq!(sequential.len(), parallel.len());
    }

    #[test]
    fn test_smart_walker_detects_project_root() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src/nested/deep")).unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("src/nested/deep/file.rs"), "code").unwrap();

        // Start from deep nested directory
        let walker = SmartWalker::new(&tmp.path().join("src/nested/deep"));

        // Root should be detected at Cargo.toml level
        assert_eq!(walker.manifest().root, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn test_smart_walker_as_file_entries() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();

        let walker = SmartWalker::new(tmp.path());
        let entries = walker.walk_as_file_entries().unwrap();

        assert!(entries.len() >= 1);
        assert!(entries.iter().any(|e| e.path.contains("main.rs")));
    }

    #[test]
    fn test_smart_walker_file_walker_trait() {
        let tmp = TempDir::new().unwrap();
        create_pollution_test_project(&tmp);

        let walker = SmartWalker::new(tmp.path());
        let config = WalkConfig::default();
        let entries = FileWalker::walk(&walker, tmp.path().to_str().unwrap(), &config).unwrap();

        // Should include project files
        assert!(entries.iter().any(|e| e.path.contains("main.rs")));

        // Should exclude pollution
        assert!(!entries.iter().any(|e| e.path.contains(".venv")));
        assert!(!entries.iter().any(|e| e.path.contains("node_modules")));
    }

    #[test]
    fn test_smart_walk_config_default() {
        let config = SmartWalkConfig::default();
        assert!(!config.follow_symlinks);
        assert!(config.respect_gitignore);
        assert!(!config.include_hidden);
        assert_eq!(config.max_file_size, 1_048_576);
    }

    // ========================================================================
    // Symlink Handling Tests
    // ========================================================================

    #[cfg(unix)]
    #[test]
    fn test_smart_walker_skips_broken_symlinks_silently() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();

        // Create a normal file
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();

        // Create a broken symlink (points to non-existent file)
        let broken_link = tmp.path().join("broken_link");
        symlink("/nonexistent/path/that/does/not/exist", &broken_link).unwrap();

        // Verify the broken link exists as a symlink
        assert!(broken_link.symlink_metadata().is_ok());
        assert!(broken_link.read_link().is_ok());
        assert!(!broken_link.exists()); // Target doesn't exist

        // Walk should complete successfully without warnings for broken symlinks
        let config = SmartWalkConfig {
            follow_symlinks: false, // Default: skip broken symlinks silently
            ..Default::default()
        };
        let walker = SmartWalker::with_config(tmp.path(), config);
        let entries = walker.walk().unwrap();

        // Should find the normal file
        assert!(entries.iter().any(|e| e.relative_path.to_string_lossy().contains("main.rs")));

        // Should not include the broken symlink (it's not a valid file)
        assert!(!entries.iter().any(|e| e.relative_path.to_string_lossy().contains("broken_link")));
    }

    #[cfg(unix)]
    #[test]
    fn test_smart_walker_follows_valid_symlinks() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();

        // Create a normal file
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        let target_file = tmp.path().join("src/target.txt");
        fs::write(&target_file, "target content").unwrap();

        // Create a valid symlink to the file
        let valid_link = tmp.path().join("link_to_target.txt");
        symlink(&target_file, &valid_link).unwrap();

        // Verify the symlink is valid
        assert!(valid_link.exists());

        // Walk with follow_symlinks = true
        let config = SmartWalkConfig {
            follow_symlinks: true,
            ..Default::default()
        };
        let walker = SmartWalker::with_config(tmp.path(), config);
        let entries = walker.walk().unwrap();

        // Should find both the target and the symlink
        let paths: Vec<_> = entries.iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(paths.iter().any(|p| p.contains("target.txt")));
        // Note: With follow_links enabled, the symlink may or may not appear
        // depending on the ignore crate's behavior, but no error should occur
    }

    #[cfg(unix)]
    #[test]
    fn test_smart_walker_with_symlink_to_directory() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();

        // Create a directory with a file
        fs::create_dir_all(tmp.path().join("real_dir")).unwrap();
        fs::write(tmp.path().join("real_dir/file.txt"), "content").unwrap();

        // Create a symlink to the directory
        let dir_link = tmp.path().join("linked_dir");
        symlink(tmp.path().join("real_dir"), &dir_link).unwrap();

        // Walk should work without errors
        let walker = SmartWalker::new(tmp.path());
        let result = walker.walk();

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Should find the file in the real directory
        assert!(entries.iter().any(|e| e.relative_path.to_string_lossy().contains("real_dir")));
    }

    // ========================================================================
    // Additional Coverage Tests
    // ========================================================================

    #[test]
    fn test_default_walker_default_trait() {
        let walker = DefaultWalker::default();
        // Just verify Default trait works
        assert!(!walker.is_too_large(100, 1000));
    }

    #[test]
    fn test_default_walker_not_a_directory() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not_a_dir.txt");
        fs::write(&file_path, "content").unwrap();

        let walker = DefaultWalker::new();
        let config = WalkConfig::default();
        let result = walker.walk(file_path.to_str().unwrap(), &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_content_binary_like() {
        // Create content with many replacement characters (binary-like)
        let bytes: Vec<u8> = vec![0xFF, 0xFE, 0xFF, 0xFE, 0xFF]; // Invalid UTF-8
        let result = read_file_content(&bytes);
        // Should return None due to too many replacement chars
        assert!(result.is_none());
    }

    #[test]
    fn test_read_file_content_partial_invalid() {
        // Valid UTF-8 with a few invalid bytes mixed in
        // Need enough valid chars so invalid is < 10%
        let mut bytes = b"Hello World, this is a longer string with enough chars".to_vec();
        bytes.extend(&[0xFF]); // Invalid byte
        bytes.extend(b" to make the ratio work correctly");
        let result = read_file_content(&bytes);
        // Should use lossy conversion since < 10% replacement (1 invalid in ~90 chars)
        assert!(result.is_some());
        assert!(result.unwrap().contains("Hello"));
    }

    #[test]
    fn test_smart_walker_accessors() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let walker = SmartWalker::new(tmp.path());

        // Test manifest() accessor
        let manifest = walker.manifest();
        assert_eq!(manifest.root, tmp.path().canonicalize().unwrap());

        // Test root() accessor
        let root = walker.root();
        assert_eq!(root, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn test_smart_walker_with_config() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.txt"), "content").unwrap();

        let config = SmartWalkConfig {
            follow_symlinks: true,
            include_hidden: true,
            max_depth: Some(5),
            ..Default::default()
        };

        let walker = SmartWalker::with_config(tmp.path(), config);
        let entries = walker.walk().unwrap();

        assert!(!entries.is_empty());
    }

    #[test]
    fn test_smart_walk_config_with_max_depth() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("a/b/c/d/e")).unwrap();
        fs::write(tmp.path().join("a/b/c/d/e/deep.txt"), "deep").unwrap();
        fs::write(tmp.path().join("a/shallow.txt"), "shallow").unwrap();

        let config = SmartWalkConfig {
            max_depth: Some(2),
            ..Default::default()
        };

        let walker = SmartWalker::with_config(tmp.path(), config);
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries.iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        // shallow.txt is at depth 2 (a/shallow.txt)
        assert!(paths.iter().any(|p| p.contains("shallow.txt")));
        // deep.txt is at depth 6, should be excluded
        assert!(!paths.iter().any(|p| p.contains("deep.txt")));
    }

    #[test]
    fn test_smart_walker_max_file_size() {
        let tmp = TempDir::new().unwrap();

        // Create a small file
        fs::write(tmp.path().join("small.txt"), "small").unwrap();

        // Create a "large" file (larger than our limit)
        fs::write(tmp.path().join("large.txt"), "x".repeat(1000)).unwrap();

        let config = SmartWalkConfig {
            max_file_size: 100, // Only 100 bytes
            ..Default::default()
        };

        let walker = SmartWalker::with_config(tmp.path(), config);
        let entries = walker.walk().unwrap();

        let paths: Vec<_> = entries.iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(paths.iter().any(|p| p.contains("small.txt")));
        assert!(!paths.iter().any(|p| p.contains("large.txt")));
    }

    #[test]
    fn test_walk_entry_fields() {
        let entry = WalkEntry {
            path: std::path::PathBuf::from("/abs/path/file.txt"),
            relative_path: std::path::PathBuf::from("file.txt"),
            is_file: true,
        };

        assert!(entry.is_file);
        assert_eq!(entry.path.to_string_lossy(), "/abs/path/file.txt");
        assert_eq!(entry.relative_path.to_string_lossy(), "file.txt");
    }

    #[test]
    fn test_walk_config_custom() {
        let config = WalkConfig {
            ignore_patterns: vec!["custom".to_string()],
            include_patterns: vec!["*.rs".to_string()],
            max_file_size: 500_000,
        };

        assert!(config.ignore_patterns.contains(&"custom".to_string()));
        assert!(config.include_patterns.contains(&"*.rs".to_string()));
        assert_eq!(config.max_file_size, 500_000);
    }

    #[test]
    fn test_default_walker_include_patterns() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("include.rs"), "rust code").unwrap();
        fs::write(tmp.path().join("exclude.txt"), "text").unwrap();

        let walker = DefaultWalker::new();
        let config = WalkConfig {
            ignore_patterns: vec![],
            include_patterns: vec!["*.rs".to_string()],
            max_file_size: 1_048_576,
        };
        let entries = walker.walk(tmp.path().to_str().unwrap(), &config).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].path.ends_with("include.rs"));
    }

    #[test]
    fn test_default_walker_skips_binary() {
        let tmp = TempDir::new().unwrap();
        // Create a binary file with null bytes
        fs::write(tmp.path().join("binary.bin"), &[0x00, 0x01, 0x02]).unwrap();
        fs::write(tmp.path().join("text.txt"), "text content").unwrap();

        let walker = DefaultWalker::new();
        let config = WalkConfig::default();
        let entries = walker.walk(tmp.path().to_str().unwrap(), &config).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].path.ends_with("text.txt"));
    }

    #[test]
    fn test_default_walker_skips_large_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("small.txt"), "small").unwrap();
        fs::write(tmp.path().join("large.txt"), "x".repeat(2_000_000)).unwrap();

        let walker = DefaultWalker::new();
        let config = WalkConfig {
            max_file_size: 1_000_000,
            ..Default::default()
        };
        let entries = walker.walk(tmp.path().to_str().unwrap(), &config).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].path.ends_with("small.txt"));
    }

    #[test]
    fn test_matches_patterns_prefix_match() {
        // Test prefix match (directory)
        assert!(DefaultWalker::matches_patterns("hidden/file.txt", &vec!["hidden".to_string()]));
    }

    #[test]
    fn test_smart_walker_should_ignore_trait_impl() {
        let tmp = TempDir::new().unwrap();
        let walker = SmartWalker::new(tmp.path());

        assert!(walker.should_ignore(".venv/lib/file.py", &vec![]));
        assert!(walker.should_ignore("node_modules/pkg/index.js", &vec![]));
        assert!(!walker.should_ignore("src/main.rs", &vec![]));
    }

    #[test]
    fn test_hygiene_wildcard_patterns() {
        // Test .egg-info wildcard
        assert!(SmartWalker::is_hygiene_excluded(Path::new("mypackage.egg-info/PKG-INFO")));
        // Test .swp wildcard
        assert!(SmartWalker::is_hygiene_excluded(Path::new("file.swp")));
        // Test .swo wildcard
        assert!(SmartWalker::is_hygiene_excluded(Path::new("file.swo")));
        // Test .pyc wildcard
        assert!(SmartWalker::is_hygiene_excluded(Path::new("module.pyc")));
    }

    #[test]
    fn test_smart_walk_config_clone() {
        let config = SmartWalkConfig {
            follow_symlinks: true,
            respect_gitignore: false,
            include_hidden: true,
            max_depth: Some(10),
            extra_excludes: vec!["custom".to_string()],
            max_file_size: 500_000,
        };

        let cloned = config.clone();
        assert_eq!(config.follow_symlinks, cloned.follow_symlinks);
        assert_eq!(config.max_depth, cloned.max_depth);
        assert_eq!(config.extra_excludes, cloned.extra_excludes);
    }

    #[test]
    fn test_walk_entry_clone() {
        let entry = WalkEntry {
            path: std::path::PathBuf::from("/path/to/file.txt"),
            relative_path: std::path::PathBuf::from("file.txt"),
            is_file: true,
        };

        let cloned = entry.clone();
        assert_eq!(entry.path, cloned.path);
        assert_eq!(entry.relative_path, cloned.relative_path);
    }
}
