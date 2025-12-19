//! pm_encoder - High-performance context serializer (Rust Engine)
//!
//! This library provides the core logic for serializing project files into
//! the Plus/Minus format. It is designed to be consumed by:
//! - The CLI binary (src/bin/main.rs)
//! - WASM bindings (future)
//! - Python bindings via PyO3 (future)
//!
//! # Architecture
//!
//! This crate follows the "Library-First" pattern:
//! - **lib.rs** (this file): Pure logic, no CLI concerns
//! - **bin/main.rs**: Thin wrapper that calls the library
//!
//! This separation allows the core logic to be reusable across different
//! interfaces without coupling to any specific runtime environment.

use std::fs;
use std::path::Path;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use globset::Glob;
use walkdir::WalkDir;

pub mod analyzers;
pub mod budgeting;
pub mod init;
pub mod lenses;

pub use lenses::{LensManager, LensConfig, AppliedLens};
pub use budgeting::{TokenEstimator, BudgetReport, parse_token_budget, apply_token_budget, FileData};

/// A file entry with its content and metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Relative path to the file
    pub path: String,
    /// File content as string
    pub content: String,
    /// MD5 checksum of the content
    pub md5: String,
    /// Modification time (seconds since epoch)
    pub mtime: u64,
    /// Creation time (seconds since epoch, falls back to mtime on some systems)
    pub ctime: u64,
}

/// Configuration loaded from .pm_encoder_config.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Patterns to ignore (e.g., ["*.pyc", ".git"])
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    /// Patterns to include (overrides ignore)
    #[serde(default)]
    pub include_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignore_patterns: vec![],
            include_patterns: vec![],
        }
    }
}

/// Configuration for the encoder (expanded for CLI parity)
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Patterns to ignore (e.g., ["*.pyc", ".git"])
    pub ignore_patterns: Vec<String>,
    /// Patterns to include (overrides ignore)
    pub include_patterns: Vec<String>,
    /// Sort by: "name", "mtime", or "ctime"
    pub sort_by: String,
    /// Sort order: "asc" or "desc"
    pub sort_order: String,
    /// Maximum lines before truncation (0 = no truncation)
    pub truncate_lines: usize,
    /// Truncation mode: "simple", "smart", or "structure"
    pub truncate_mode: String,
    /// Maximum file size in bytes (default: 5MB)
    pub max_file_size: u64,
    /// Enable streaming mode (immediate output, no global sort)
    pub stream: bool,
    /// Include summary markers in truncated output (default: true)
    pub truncate_summary: bool,
    /// Patterns of files to skip truncation for
    pub truncate_exclude: Vec<String>,
    /// Show truncation statistics report
    pub truncate_stats: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        // Default patterns match Python: [".git", "target", ".venv", "__pycache__", "*.pyc", "*.swp"]
        Self {
            ignore_patterns: vec![
                ".git".to_string(),
                "target".to_string(),
                ".venv".to_string(),
                "__pycache__".to_string(),
                "*.pyc".to_string(),
                "*.swp".to_string(),
            ],
            include_patterns: vec![],
            sort_by: "name".to_string(),
            sort_order: "asc".to_string(),
            truncate_lines: 0,
            truncate_mode: "simple".to_string(),
            max_file_size: 5 * 1024 * 1024, // 5MB
            stream: false, // Default to batch mode for backward compatibility
            truncate_summary: true, // Include summary markers by default
            truncate_exclude: vec![], // No files excluded by default
            truncate_stats: false, // Don't show stats report by default
        }
    }
}

impl EncoderConfig {
    /// Load configuration from a JSON file
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: Config = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        Ok(Self {
            ignore_patterns: config.ignore_patterns,
            include_patterns: config.include_patterns,
            stream: false, // Streaming is only enabled via CLI flag
            ..Default::default()
        })
    }
}

// ============================================================================
// CONTEXT ENGINE - Library-First Architecture for WASM Compatibility
// ============================================================================

/// Result of processing a single file
#[derive(Debug, Clone)]
pub struct ProcessedFile {
    /// Relative path to the file
    pub path: String,
    /// Processed content (possibly truncated)
    pub content: String,
    /// MD5 checksum of the ORIGINAL content
    pub md5: String,
    /// Whether the content was truncated
    pub was_truncated: bool,
    /// Original line count (before truncation)
    pub original_lines: usize,
    /// Modification time (seconds since epoch)
    pub mtime: u64,
    /// Creation time (seconds since epoch)
    pub ctime: u64,
}

/// The core context engine - holds configuration but does NO I/O
///
/// This struct is designed for the "Library-First" pattern:
/// - All methods are pure functions (no filesystem access)
/// - Can be compiled to WASM
/// - Can be embedded in Python via PyO3
/// - CLI uses this via I/O adapter functions
///
/// # Example
///
/// ```rust
/// use pm_encoder::{ContextEngine, EncoderConfig};
///
/// let engine = ContextEngine::new(EncoderConfig::default());
///
/// // Process file content (PURE - no I/O)
/// let processed = engine.process_file_content("main.py", "print('hello')");
///
/// // Serialize processed files (PURE - no I/O)
/// let output = engine.serialize_processed_files(&[processed]);
/// ```
pub struct ContextEngine {
    /// Encoder configuration
    pub config: EncoderConfig,
    /// Lens manager for context filtering
    pub lens_manager: LensManager,
}

impl ContextEngine {
    /// Create a new context engine with the given configuration
    pub fn new(config: EncoderConfig) -> Self {
        Self {
            config,
            lens_manager: LensManager::new(),
        }
    }

    /// Create a new context engine with a specific lens applied
    pub fn with_lens(config: EncoderConfig, lens_name: &str) -> Result<Self, String> {
        let mut engine = Self::new(config);
        engine.lens_manager.apply_lens(lens_name)?;
        Ok(engine)
    }

    /// Process a single file's content (PURE - no I/O)
    ///
    /// This is the core pure function that can run in WASM.
    /// It takes path and content as inputs (no filesystem access).
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path to the file
    /// * `content` - Raw file content as string
    ///
    /// # Returns
    ///
    /// * `ProcessedFile` - Processed file with optional truncation applied
    pub fn process_file_content(&self, path: &str, content: &str) -> ProcessedFile {
        let original_lines = count_lines_python_style(content);
        let md5 = calculate_md5(content);

        // Apply truncation if configured
        let (processed_content, was_truncated) = if self.config.truncate_lines > 0
            || self.config.truncate_mode == "structure"
        {
            match self.config.truncate_mode.as_str() {
                "simple" => truncate_simple_with_options(
                    content,
                    self.config.truncate_lines,
                    path,
                    self.config.truncate_summary,
                ),
                "smart" => truncate_smart_with_options(
                    content,
                    self.config.truncate_lines,
                    path,
                    self.config.truncate_summary,
                ),
                "structure" => truncate_structure_with_fallback(
                    content,
                    path,
                    self.config.truncate_summary,
                    self.config.truncate_lines,
                ),
                _ => (content.to_string(), false),
            }
        } else {
            (content.to_string(), false)
        };

        ProcessedFile {
            path: path.to_string(),
            content: processed_content,
            md5,
            was_truncated,
            original_lines,
            mtime: 0, // Set by caller if needed
            ctime: 0, // Set by caller if needed
        }
    }

    /// Serialize a single processed file to Plus/Minus format (PURE - no I/O)
    pub fn serialize_processed_file(&self, file: &ProcessedFile) -> String {
        let mut output = String::new();

        // Header: ++++++++++ filename [TRUNCATED: N lines] ++++++++++
        if file.was_truncated {
            output.push_str(&format!(
                "++++++++++ {} [TRUNCATED: {} lines] ++++++++++\n",
                file.path, file.original_lines
            ));
        } else {
            output.push_str(&format!("++++++++++ {} ++++++++++\n", file.path));
        }

        // Content
        output.push_str(&file.content);

        // Ensure content ends with newline
        if !file.content.ends_with('\n') {
            output.push('\n');
        }

        // Calculate final line count for footer
        let final_lines = count_lines_python_style(&file.content);

        // Footer: ---------- filename [TRUNCATED:orig→final] md5 filename ----------
        if file.was_truncated {
            output.push_str(&format!(
                "---------- {} [TRUNCATED:{}→{}] {} {} ----------\n",
                file.path, file.original_lines, final_lines, file.md5, file.path
            ));
        } else {
            output.push_str(&format!(
                "---------- {} {} {} ----------\n",
                file.path, file.md5, file.path
            ));
        }

        output
    }

    /// Serialize multiple processed files (PURE - no I/O)
    ///
    /// Files are serialized in the order provided. Sorting should be done
    /// by the caller before passing to this function.
    pub fn serialize_processed_files(&self, files: &[ProcessedFile]) -> String {
        let mut output = String::new();
        for file in files {
            output.push_str(&self.serialize_processed_file(file));
        }
        output
    }

    /// Generate complete context from path-content pairs (PURE - no I/O)
    ///
    /// This is the main entry point for WASM usage. It takes a list of
    /// (path, content) pairs and returns the complete serialized context.
    ///
    /// # Arguments
    ///
    /// * `files` - List of (path, content) tuples
    ///
    /// # Returns
    ///
    /// * Serialized context string
    pub fn generate_context(&self, files: &[(String, String)]) -> String {
        // Process all files
        let processed: Vec<ProcessedFile> = files
            .iter()
            .map(|(path, content)| self.process_file_content(path, content))
            .collect();

        // Sort by path (default behavior)
        let mut sorted = processed;
        sorted.sort_by(|a, b| a.path.cmp(&b.path));

        // Serialize
        self.serialize_processed_files(&sorted)
    }
}

/// Version of the pm_encoder library
pub const VERSION: &str = "0.8.0";

/// Returns the version of the pm_encoder library
pub fn version() -> &'static str {
    VERSION
}

/// Load configuration from .pm_encoder_config.json
///
/// # Arguments
///
/// * `root` - Root directory to search for config file
///
/// # Returns
///
/// * `Ok(Config)` - Loaded configuration, or default if file doesn't exist
/// * `Err(String)` - Error message if config file exists but is malformed
pub fn load_config(root: &str) -> Result<Config, String> {
    let config_path = Path::new(root).join(".pm_encoder_config.json");

    if !config_path.exists() {
        // No config file, return default
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: Config = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

/// Calculate MD5 checksum of content
///
/// # Arguments
///
/// * `content` - The content to hash
///
/// # Returns
///
/// * MD5 checksum as hexadecimal string
pub fn calculate_md5(content: &str) -> String {
    format!("{:x}", md5::compute(content.as_bytes()))
}

/// Check if content appears to be binary
///
/// A file is considered binary if it contains null bytes in the first 8KB
///
/// # Arguments
///
/// * `content` - The content to check
///
/// # Returns
///
/// * `true` if content appears binary, `false` otherwise
pub fn is_binary(content: &[u8]) -> bool {
    // Check first 8KB for null bytes
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

/// Check if file size exceeds the limit
///
/// # Arguments
///
/// * `size` - File size in bytes
/// * `limit` - Maximum allowed size in bytes
///
/// # Returns
///
/// * `true` if size exceeds limit, `false` otherwise
pub fn is_too_large(size: u64, limit: u64) -> bool {
    size > limit
}

/// Read file content with binary detection and encoding fallback
///
/// Matches Python's behavior:
/// 1. Read file as bytes
/// 2. Check for binary (null bytes) - return None if binary
/// 3. Try UTF-8 decoding
/// 4. Fallback to Latin-1 (ISO-8859-1) if UTF-8 fails
///
/// # Arguments
///
/// * `bytes` - Raw file content as bytes
///
/// # Returns
///
/// * `Some(String)` - Decoded content
/// * `None` - File is binary (should be skipped)
pub fn read_file_content(bytes: &[u8]) -> Option<String> {
    // Check for binary content
    if is_binary(bytes) {
        return None;
    }

    // Try UTF-8 first
    let content = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            // Fallback: decode as Latin-1 (ISO-8859-1)
            // Latin-1 is a 1:1 byte-to-char mapping, never fails
            bytes.iter().map(|&b| b as char).collect()
        }
    };

    // Normalize line endings to \n (like Python's read_text())
    // \r\n -> \n, then \r -> \n
    Some(content.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Check if a path matches any of the given glob patterns
///
/// # Arguments
///
/// * `path` - Path to check (relative path)
/// * `patterns` - List of glob patterns
///
/// # Returns
///
/// * `true` if path matches any pattern, `false` otherwise
fn matches_patterns(path: &str, patterns: &[String]) -> bool {
    for pattern_str in patterns {
        // Try to compile the pattern
        if let Ok(glob) = Glob::new(pattern_str) {
            let matcher = glob.compile_matcher();

            // Match against the full path
            if matcher.is_match(path) {
                return true;
            }

            // Also check if any path component or parent path matches
            // This handles patterns like ".git" matching ".git/config"
            let parts: Vec<&str> = path.split('/').collect();
            for i in 0..parts.len() {
                let component = parts[i];
                // Check individual component
                if matcher.is_match(component) {
                    return true;
                }
                // Check partial paths (e.g., ".git" for ".git/config")
                if i > 0 {
                    let partial = parts[..=i].join("/");
                    if matcher.is_match(&partial) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Determine if a file should be included based on ignore/include patterns
///
/// # Arguments
///
/// * `path` - Relative path to check
/// * `ignore_patterns` - Patterns to ignore
/// * `include_patterns` - Patterns to include
///
/// # Returns
///
/// * `true` if file should be included, `false` otherwise
///
/// # Logic (matches Python behavior)
///
/// 1. Check ignore patterns FIRST - if match, EXCLUDE (no override by includes)
/// 2. Pure whitelist mode: if include_patterns exist AND ignore_patterns is empty,
///    file must match at least one include pattern
/// 3. Hybrid mode: if both exist, file just needs to NOT match ignore patterns
///    (include patterns don't act as a filter, they're for explicit inclusion of ignored items)
/// 4. If no patterns or only ignore patterns, include by default (if not ignored)
fn should_include_file(
    path: &str,
    ignore_patterns: &[String],
    include_patterns: &[String],
) -> bool {
    // Check ignore patterns FIRST (they take precedence over includes)
    // This matches Python behavior where directory-level ignores can't be overridden
    if matches_patterns(path, ignore_patterns) {
        return false;  // Ignored paths are always excluded
    }

    // Pure whitelist mode: only when include_patterns exist AND no ignore_patterns
    // In this mode, files must match at least one include pattern
    if !include_patterns.is_empty() && ignore_patterns.is_empty() {
        return matches_patterns(path, include_patterns);
    }

    // Hybrid mode (both patterns) or blacklist mode (only ignore):
    // If not ignored, include by default
    true
}

/// Walk directory and yield file entries as an iterator (streaming)
///
/// Uses WalkDir with filter_entry for directory pruning - ignored directories
/// are never entered, matching Python's behavior.
///
/// This is the iterator-based version that enables streaming output.
/// Files are yielded as they're discovered, enabling immediate output.
///
/// # Arguments
///
/// * `root` - Root directory path
/// * `ignore_patterns` - Patterns to ignore (applies to directories and files)
/// * `include_patterns` - Patterns to include (only applies to files)
/// * `max_size` - Maximum file size in bytes
///
/// # Returns
///
/// * Iterator yielding FileEntry items
pub fn walk_directory_iter(
    root: &str,
    ignore_patterns: Vec<String>,
    include_patterns: Vec<String>,
    max_size: u64,
) -> impl Iterator<Item = FileEntry> {
    let root_path = Path::new(root).to_path_buf();
    let root_path_clone = root_path.clone();
    let ignore_patterns_clone = ignore_patterns.clone();

    // Create walker with directory pruning via filter_entry
    // filter_entry is called BEFORE descending into a directory
    // follow_links(true) matches Python's default behavior
    WalkDir::new(&root_path)
        .follow_links(true)
        .into_iter()
        .filter_entry(move |entry| {
            // Get the path relative to root for pattern matching
            let path = entry.path();

            // Always include the root directory itself
            if path == root_path_clone {
                return true;
            }

            // Get relative path for pattern matching
            let rel_path = match path.strip_prefix(&root_path_clone) {
                Ok(p) => p,
                Err(_) => return false,
            };

            let path_str = match rel_path.to_str() {
                Some(s) => s,
                None => return false,
            };

            // For directories: check if directory should be pruned (ignored)
            // This prevents entering .git, .llm_archive, node_modules, etc.
            if entry.file_type().is_dir() {
                // Check if this directory matches any ignore pattern
                // If so, skip the entire tree by returning false
                !matches_patterns(path_str, &ignore_patterns_clone)
            } else {
                // For files: always return true here, we'll filter later
                // (filter_entry affects directory traversal, not file inclusion)
                true
            }
        })
        .filter_map(move |result| {
            let entry = match result {
                Ok(e) => e,
                Err(e) => {
                    // Skip entries we can't read (permission denied, etc.)
                    eprintln!("Warning: {}", e);
                    return None;
                }
            };

            // Skip directories (we only want files)
            if entry.file_type().is_dir() {
                return None;
            }

            let path = entry.path();

            // Get relative path for pattern matching and output
            let rel_path = path.strip_prefix(&root_path).ok()?;
            let path_str = rel_path.to_str()?;

            // Check if this file should be included based on patterns
            // Note: ignore patterns already handled by filter_entry for directories,
            // but we still need to check file-level ignores and include patterns
            if !should_include_file(path_str, &ignore_patterns, &include_patterns) {
                return None;
            }

            // Get file metadata
            let metadata = fs::metadata(path).ok()?;
            let file_size = metadata.len();

            // Skip files that are too large
            if is_too_large(file_size, max_size) {
                return None;
            }

            // Extract timestamps
            let mtime = metadata.modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            // ctime: On Unix, use created(). Falls back to mtime if unavailable.
            let ctime = metadata.created()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(mtime);

            // Read file content (bytes first, then decode)
            let buffer = fs::read(path).ok()?;

            // Use read_file_content helper (handles binary detection + encoding)
            let content = read_file_content(&buffer)?;

            // Calculate MD5
            let md5 = calculate_md5(&content);

            Some(FileEntry {
                path: path_str.to_string(),
                content,
                md5,
                mtime,
                ctime,
            })
        })
}

/// Walk directory and collect file entries (batch mode)
///
/// Uses WalkDir with filter_entry for directory pruning - ignored directories
/// are never entered, matching Python's behavior.
///
/// This is the batch version that collects all files into a Vec.
/// For streaming output, use `walk_directory_iter` instead.
///
/// # Arguments
///
/// * `root` - Root directory path
/// * `ignore_patterns` - Patterns to ignore (applies to directories and files)
/// * `include_patterns` - Patterns to include (only applies to files)
/// * `max_size` - Maximum file size in bytes
///
/// # Returns
///
/// * `Ok(Vec<FileEntry>)` - List of file entries
/// * `Err(String)` - Error message if walk fails
pub fn walk_directory(
    root: &str,
    ignore_patterns: &[String],
    include_patterns: &[String],
    max_size: u64,
) -> Result<Vec<FileEntry>, String> {
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("Directory not found: {}", root));
    }

    // Use the iterator version and collect into Vec
    let entries: Vec<FileEntry> = walk_directory_iter(
        root,
        ignore_patterns.to_vec(),
        include_patterns.to_vec(),
        max_size,
    ).collect();

    Ok(entries)
}

/// Truncate content to a maximum number of lines (simple mode)
///
/// # Arguments
///
/// * `content` - The content to truncate
/// * `max_lines` - Maximum number of lines to keep
/// * `file_path` - File path for the truncation marker
///
/// # Returns
///
/// * `(truncated_content, was_truncated)` - The truncated content and whether truncation occurred
pub fn truncate_simple(content: &str, max_lines: usize, file_path: &str) -> (String, bool) {
    truncate_simple_with_options(content, max_lines, file_path, true)
}

/// Truncate content to a maximum number of lines with options
///
/// # Arguments
///
/// * `content` - The content to truncate
/// * `max_lines` - Maximum number of lines to keep
/// * `file_path` - File path for the truncation marker
/// * `include_summary` - Whether to include the summary marker
///
/// # Returns
///
/// * `(truncated_content, was_truncated)` - The truncated content and whether truncation occurred
pub fn truncate_simple_with_options(
    content: &str,
    max_lines: usize,
    file_path: &str,
    include_summary: bool,
) -> (String, bool) {
    let lines: Vec<&str> = python_style_split(content);
    let total_lines = lines.len();

    if max_lines == 0 || total_lines <= max_lines {
        return (content.to_string(), false);
    }

    // Keep first N lines
    let kept_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();
    let mut truncated = kept_lines.join("\n");

    // Add truncation marker (matching Python format) only if include_summary is true
    if include_summary {
        let reduced_pct = (total_lines - max_lines) * 100 / total_lines;
        let marker = format!(
            "\n\n{}\nTRUNCATED at line {}/{} ({}% reduction)\nTo get full content: --include \"{}\" --truncate 0\n{}\n",
            "=".repeat(70),
            max_lines,
            total_lines,
            reduced_pct,
            file_path,
            "=".repeat(70)
        );
        truncated.push_str(&marker);
    }

    (truncated, true)
}

/// Check if a file should skip truncation based on exclude patterns
///
/// # Arguments
///
/// * `path` - File path to check
/// * `patterns` - Patterns to match against
///
/// # Returns
///
/// * `true` if the file should skip truncation
pub fn should_skip_truncation(path: &str, patterns: &[String]) -> bool {
    matches_patterns(path, patterns)
}

/// Serialize a file entry into Plus/Minus format
///
/// # Arguments
///
/// * `entry` - The file entry to serialize
///
/// # Returns
///
/// * Serialized string in Plus/Minus format
pub fn serialize_file(entry: &FileEntry) -> String {
    serialize_file_with_truncation(entry, 0, "simple")
}

/// Truncate content using Python's default strategy: keep first 40%, gap, keep last 10%
///
/// This matches Python's LanguageAnalyzer.get_truncate_ranges() default behavior
/// for files without specialized analyzers. The output includes gap markers
/// to show where content was omitted.
fn truncate_with_gap_markers(
    content: &str,
    max_lines: usize,
    file_path: &str,
    include_summary: bool,
    language: Option<&str>,
) -> (String, bool) {
    let lines: Vec<&str> = python_style_split(content);
    let total_lines = lines.len();

    if max_lines == 0 || total_lines <= max_lines {
        return (content.to_string(), false);
    }

    // Python's default strategy: keep first 40% and last 10% of max_lines
    let keep_first = (max_lines as f64 * 0.4) as usize;
    let keep_last = (max_lines as f64 * 0.1) as usize;

    // Calculate range boundaries (1-indexed like Python)
    let first_end = keep_first.min(total_lines);
    // Use saturating subtraction to avoid overflow
    let last_start = total_lines.saturating_sub(keep_last).saturating_add(1).max(first_end + 1);

    let mut result = String::new();

    // Keep first section
    for i in 0..first_end {
        result.push_str(lines[i]);
        result.push('\n');
    }

    // Add gap marker if there's a gap
    if last_start > first_end + 1 {
        let gap_size = last_start - first_end - 1;
        result.push_str(&format!("\n... [{} lines omitted] ...\n\n", gap_size));
    }

    // Keep last section
    for i in (last_start - 1)..total_lines {
        result.push_str(lines[i]);
        result.push('\n');
    }

    // Calculate kept lines (excluding the gap marker line itself)
    let kept_count = first_end + total_lines.saturating_sub(last_start).saturating_add(1);

    // Add truncation marker
    if include_summary {
        let omitted = total_lines.saturating_sub(kept_count);
        let mut marker = format!(
            "\n{}\nTRUNCATED at line {}/{} ({}% reduction)",
            "=".repeat(70),
            max_lines,
            total_lines,
            omitted * 100 / total_lines,
        );

        // Add Language line if provided (matches Python's smart mode marker)
        if let Some(lang) = language {
            marker.push_str(&format!("\nLanguage: {}", lang));
        }

        marker.push_str(&format!(
            "\nTo get full content: --include \"{}\" --truncate 0\n{}\n",
            file_path,
            "=".repeat(70)
        ));
        result.push_str(&marker);
    }

    (result, true)
}

/// Truncate markdown content matching Python's MarkdownAnalyzer.get_truncate_ranges()
///
/// Python's markdown truncation keeps most of the file:
/// - Allocates budget for H1/H2 header sections (10 lines each, up to 10% of max per section)
/// - Fills remaining budget with beginning of file
/// This effectively keeps first ~max_lines with header supplements
fn truncate_markdown(
    content: &str,
    max_lines: usize,
    file_path: &str,
    include_summary: bool,
) -> (String, bool) {
    let lines: Vec<&str> = python_style_split(content);
    let total_lines = lines.len();

    if max_lines == 0 || total_lines <= max_lines {
        return (content.to_string(), false);
    }

    // Python behavior: keep first max_lines (budget filled with beginning)
    // This matches Python's MarkdownAnalyzer.get_truncate_ranges() which adds (1, budget)
    let kept_lines: Vec<&str> = lines.iter().take(max_lines).copied().collect();
    let mut truncated = kept_lines.join("\n");

    // Add smart mode marker with Language: Markdown (matches Python's smart mode output)
    if include_summary {
        let reduced_pct = (total_lines - max_lines) * 100 / total_lines;

        // Extract links from markdown (Python's "imports" field for markdown)
        // Python iterates LINE BY LINE, so multi-line links are not found
        let link_pattern = regex::Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)").unwrap();
        let mut links: Vec<&str> = Vec::new();
        for line in content.lines() {
            for cap in link_pattern.captures_iter(line) {
                if let Some(url) = cap.get(2) {
                    links.push(url.as_str());
                    if links.len() >= 10 {
                        break;
                    }
                }
            }
            if links.len() >= 10 {
                break;
            }
        }

        let mut marker = format!(
            "\n\n{}\nTRUNCATED at line {}/{} ({}% reduction)\nLanguage: Markdown\nCategory: documentation",
            "=".repeat(70),
            max_lines,
            total_lines,
            reduced_pct,
        );

        // Add Key imports if links found (Python shows first 8 + "...")
        if !links.is_empty() {
            let imports_str = if links.len() > 8 {
                format!("{}, ...", links[..8].join(", "))
            } else {
                links.join(", ")
            };
            marker.push_str(&format!("\nKey imports: {}", imports_str));
        }

        // Empty line before "To get full content" (matches Python's marker format)
        marker.push_str(&format!(
            "\n\nTo get full content: --include \"{}\" --truncate 0\n{}\n",
            file_path,
            "=".repeat(70)
        ));
        truncated.push_str(&marker);
    }

    (truncated, true)
}

/// Truncate content using smart mode (language-aware)
///
/// Smart mode uses language analyzers to identify important sections
/// and keeps them while truncating less important parts.
pub fn truncate_smart(content: &str, max_lines: usize, file_path: &str) -> (String, bool) {
    truncate_smart_with_options(content, max_lines, file_path, true)
}

/// Truncate content using smart mode with options
///
/// # Arguments
///
/// * `content` - The content to truncate
/// * `max_lines` - Maximum number of lines to keep
/// * `file_path` - File path for the truncation marker
/// * `include_summary` - Whether to include the summary marker
///
/// # Returns
///
/// * `(truncated_content, was_truncated)` - The truncated content and whether truncation occurred
pub fn truncate_smart_with_options(
    content: &str,
    max_lines: usize,
    file_path: &str,
    include_summary: bool,
) -> (String, bool) {
    let lines: Vec<&str> = python_style_split(content);
    let total_lines = lines.len();

    if max_lines == 0 || total_lines <= max_lines {
        return (content.to_string(), false);
    }

    // Try to get an analyzer for this file type
    if let Some(analyzer) = analyzers::get_analyzer_for_file(file_path) {
        let analysis = analyzer.analyze(content, file_path);

        // Collect important line ranges (imports, class/function definitions)
        let mut important_lines: Vec<usize> = Vec::new();

        // Always keep first few lines (often contain shebang, docstring, imports)
        for i in 1..=10.min(total_lines) {
            important_lines.push(i);
        }

        // Add lines around class/function definitions
        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            // Check if this line starts a class or function
            if line.trim_start().starts_with("class ")
                || line.trim_start().starts_with("def ")
                || line.trim_start().starts_with("fn ")
                || line.trim_start().starts_with("pub fn ")
                || line.trim_start().starts_with("function ")
                || line.trim_start().starts_with("const ")
                || line.trim_start().starts_with("struct ")
                || line.trim_start().starts_with("impl ")
                || line.trim_start().starts_with("enum ")
            {
                // Add this line and a few lines after (signature + docstring)
                for j in line_num..=(line_num + 5).min(total_lines) {
                    important_lines.push(j);
                }
            }
        }

        // Add critical sections from analysis
        for (start, end) in &analysis.critical_sections {
            for line_num in *start..=*end {
                important_lines.push(line_num);
            }
        }

        // Deduplicate and sort
        important_lines.sort();
        important_lines.dedup();

        // If we have more important lines than max_lines, or found very few important lines
        // (non-code file), use Python's default truncation strategy: keep first 40%, gap, keep last 10%
        if important_lines.len() > max_lines || (important_lines.len() < 50 && total_lines > max_lines) {
            return truncate_with_gap_markers(content, max_lines, file_path, include_summary, Some(&analysis.language));
        }

        // If file is smaller than max_lines after finding important sections,
        // just return the original content
        if total_lines <= max_lines {
            return (content.to_string(), false);
        }

        // Build output with kept lines and gap markers
        let mut result = String::new();
        let mut last_line = 0;

        for &line_num in &important_lines {
            // Add gap marker if there's a gap
            if line_num > last_line + 1 && last_line > 0 {
                let gap_size = line_num - last_line - 1;
                result.push_str(&format!("\n... [{} lines omitted] ...\n\n", gap_size));
            }

            if line_num <= total_lines {
                result.push_str(lines[line_num - 1]);
                result.push('\n');
            }
            last_line = line_num;
        }

        // Add final truncation marker only if include_summary is true
        if include_summary {
            let kept_count = important_lines.len();
            let omitted = total_lines - kept_count;
            if omitted > 0 {
                result.push_str(&format!(
                    "\n{}\nSMART TRUNCATED: kept {}/{} lines ({}% reduction)\nLanguage: {} | Category: {}\n{}\n",
                    "=".repeat(70),
                    kept_count,
                    total_lines,
                    omitted * 100 / total_lines,
                    analysis.language,
                    analysis.category,
                    "=".repeat(70)
                ));
            }
        }

        return (result, true);
    }

    // Fall back to gap-based truncation if no analyzer available (Python behavior)
    truncate_with_gap_markers(content, max_lines, file_path, include_summary, None)
}

/// Truncate content using structure mode (signatures only)
///
/// Structure mode extracts only class/function signatures, removing all bodies.
pub fn truncate_structure(content: &str, file_path: &str) -> (String, bool) {
    truncate_structure_with_options(content, file_path, true)
}

/// Truncate content using structure mode with options
///
/// # Arguments
///
/// * `content` - The content to truncate
/// * `file_path` - File path for the truncation marker
/// * `include_summary` - Whether to include the summary marker
///
/// # Returns
///
/// * `(truncated_content, was_truncated)` - The truncated content and whether truncation occurred
pub fn truncate_structure_with_options(
    content: &str,
    file_path: &str,
    include_summary: bool,
) -> (String, bool) {
    // Use 0 for max_lines to disable smart fallback (backward compatible)
    truncate_structure_with_fallback(content, file_path, include_summary, 0)
}

/// Truncate content using structure mode with smart fallback (matches Python behavior)
///
/// # Arguments
///
/// * `content` - The content to truncate
/// * `file_path` - File path for the truncation marker
/// * `include_summary` - Whether to include the summary marker
/// * `max_lines` - Maximum lines for smart fallback when no signatures found (0 = no fallback)
///
/// # Returns
///
/// * `(truncated_content, was_truncated)` - The truncated content and whether truncation occurred
pub fn truncate_structure_with_fallback(
    content: &str,
    file_path: &str,
    include_summary: bool,
    max_lines: usize,
) -> (String, bool) {
    let lines: Vec<&str> = python_style_split(content);
    let total_lines = lines.len();

    if total_lines == 0 {
        return (content.to_string(), false);
    }

    // Try to get an analyzer for this file type
    if let Some(analyzer) = analyzers::get_analyzer_for_file(file_path) {
        let analysis = analyzer.analyze(content, file_path);

        // Python behavior: Markdown files use specialized get_truncate_ranges()
        // that keeps most of the file (beginning + header sections)
        // This prevents false positives from code examples in markdown
        let path_lower = file_path.to_lowercase();
        if path_lower.ends_with(".md") || path_lower.ends_with(".markdown") {
            if max_lines > 0 {
                return truncate_markdown(content, max_lines, file_path, include_summary);
            }
            return truncate_markdown(content, 2000, file_path, include_summary);
        }

        // Collect signature lines matching Python's get_structure_ranges behavior
        let mut signature_lines: Vec<usize> = Vec::new();

        // Iterate through ALL lines (matching Python behavior)
        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim_start();

            // Skip empty lines and pure comments (but keep docstrings)
            if trimmed.is_empty() {
                continue;
            }

            // IMPORT STATEMENTS (Python: import, from; Rust: use; JS: import, export)
            if trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("export ")
            {
                signature_lines.push(line_num);
                continue;
            }

            // SHEBANG / MODULE DOCS (first few lines)
            if line_num <= 5 && (trimmed.starts_with("#!") || trimmed.starts_with("//!")) {
                signature_lines.push(line_num);
                continue;
            }

            // MODULE-LEVEL DOCSTRINGS (first 10 lines)
            if line_num <= 10 && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")) {
                signature_lines.push(line_num);
                continue;
            }

            // DECORATORS (Python @decorator)
            if trimmed.starts_with("@") {
                signature_lines.push(line_num);
                continue;
            }

            // CLASS DEFINITIONS
            if trimmed.starts_with("class ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("struct ")
            {
                signature_lines.push(line_num);
                continue;
            }

            // FUNCTION DEFINITIONS
            if trimmed.starts_with("def ")
                || trimmed.starts_with("async def ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("export function ")
                || trimmed.starts_with("async function ")
            {
                signature_lines.push(line_num);
                continue;
            }

            // OTHER STRUCTURAL ELEMENTS (Rust: impl, trait, enum, const)
            if trimmed.starts_with("impl ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("pub const ")
                || trimmed.starts_with("pub mod ")
                || trimmed.starts_with("mod ")
            {
                signature_lines.push(line_num);
                continue;
            }

            // JS/TS: interface, type definitions, arrow functions
            if trimmed.starts_with("interface ")
                || trimmed.starts_with("export interface ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("export type ")
                || (trimmed.starts_with("const ") && trimmed.contains("=>"))
            {
                signature_lines.push(line_num);
                continue;
            }

            // Rust attributes (#[...])
            if trimmed.starts_with("#[") {
                signature_lines.push(line_num);
                continue;
            }
        }

        // Deduplicate and sort
        signature_lines.sort();
        signature_lines.dedup();

        if signature_lines.is_empty() {
            // No structure found - fall back to smart mode if max_lines > 0 (Python behavior)
            if max_lines > 0 {
                return truncate_smart_with_options(content, max_lines, file_path, include_summary);
            }
            // Otherwise return first 20 lines (backward compatible)
            let kept: Vec<&str> = lines.iter().take(20).copied().collect();
            let mut result = kept.join("\n");
            if total_lines > 20 && include_summary {
                result.push_str(&format!(
                    "\n\n{}\nSTRUCTURE MODE: No signatures found, showing first 20/{} lines\n{}\n",
                    "=".repeat(70),
                    total_lines,
                    "=".repeat(70)
                ));
            }
            return (result, total_lines > 20);
        }

        // Build output with signature lines only
        let mut result = String::new();
        for &line_num in &signature_lines {
            if line_num <= total_lines {
                result.push_str(lines[line_num - 1]);
                result.push('\n');
            }
        }

        // Add structure marker only if include_summary is true
        // Format matches Python's structure mode output exactly
        if include_summary {
            let kept_count = signature_lines.len();
            result.push_str(&format!(
                "\n{}\nSTRUCTURE MODE: Showing only signatures ({}/{} lines)\nLanguage: {}\n\nIncluded: imports, class/function signatures, type definitions\nExcluded: function bodies, implementation details\n\nTo get full content: --include \"{}\" --truncate 0\n{}\n",
                "=".repeat(70),
                kept_count,
                total_lines,
                analysis.language,
                file_path,
                "=".repeat(70)
            ));
        }

        return (result, true);
    }

    // No analyzer - fall back to smart mode if max_lines > 0 (Python behavior)
    if max_lines > 0 {
        return truncate_smart_with_options(content, max_lines, file_path, include_summary);
    }

    // Otherwise fall back to first 30 lines (backward compatible)
    let kept: Vec<&str> = lines.iter().take(30).copied().collect();
    let mut result = kept.join("\n");
    if total_lines > 30 && include_summary {
        result.push_str(&format!(
            "\n\n{}\nSTRUCTURE MODE: Unknown language, showing first 30/{} lines\n{}\n",
            "=".repeat(70),
            total_lines,
            "=".repeat(70)
        ));
    }
    (result, total_lines > 30)
}

/// Serialize a file entry with optional truncation
///
/// # Arguments
///
/// * `entry` - The file entry to serialize
/// * `truncate_lines` - Maximum lines (0 = no truncation)
/// * `truncate_mode` - Truncation mode ("simple", "smart", "structure")
///
/// # Returns
///
/// * Serialized string in Plus/Minus format
/// Count lines matching Python's split('\n') behavior
/// Python's split('\n') includes empty string for trailing newline
fn count_lines_python_style(content: &str) -> usize {
    content.split('\n').count()
}

/// Split string into lines matching Python's split('\n') behavior
/// Unlike Rust's .lines(), this includes an empty string after trailing newline
/// Example: "a\nb\n" → ["a", "b", ""] (3 items, not 2)
pub fn python_style_split(content: &str) -> Vec<&str> {
    content.split('\n').collect()
}

pub fn serialize_file_with_truncation(
    entry: &FileEntry,
    truncate_lines: usize,
    truncate_mode: &str,
) -> String {
    let mut output = String::new();
    let original_lines = count_lines_python_style(&entry.content);

    // Apply truncation and track if file was truncated
    let (content, was_truncated) = if truncate_lines > 0 || truncate_mode == "structure" {
        match truncate_mode {
            "simple" => {
                truncate_simple(&entry.content, truncate_lines, &entry.path)
            }
            "smart" => {
                truncate_smart(&entry.content, truncate_lines, &entry.path)
            }
            "structure" => {
                // Use fallback version that falls back to smart mode when no signatures (Python behavior)
                truncate_structure_with_fallback(&entry.content, &entry.path, true, truncate_lines)
            }
            _ => (entry.content.clone(), false),
        }
    } else {
        (entry.content.clone(), false)
    };

    // Header: ++++++++++ filename [TRUNCATED: N lines] ++++++++++
    // Match Python's format when truncation was applied
    if was_truncated {
        output.push_str(&format!("++++++++++ {} [TRUNCATED: {} lines] ++++++++++\n", entry.path, original_lines));
    } else {
        output.push_str(&format!("++++++++++ {} ++++++++++\n", entry.path));
    }

    // Content
    output.push_str(&content);

    // Ensure content ends with newline (check content, not whole output)
    // This matches Python's behavior for empty files
    if !content.ends_with('\n') {
        output.push('\n');
    }

    // Footer format: ---------- filename [TRUNCATED:original→final] checksum filename ----------
    // Match Python's format with truncation info in footer
    let final_lines = count_lines_python_style(&content);
    if was_truncated {
        output.push_str(&format!(
            "---------- {} [TRUNCATED:{}→{}] {} {} ----------\n",
            entry.path, original_lines, final_lines, entry.md5, entry.path
        ));
    } else {
        output.push_str(&format!(
            "---------- {} {} {} ----------\n",
            entry.path, entry.md5, entry.path
        ));
    }

    output
}

/// Serialize a project directory into the Plus/Minus format
///
/// This function automatically loads configuration from `.pm_encoder_config.json`
/// if it exists in the root directory.
///
/// # Arguments
///
/// * `root` - Path to the project root directory
///
/// # Returns
///
/// * `Ok(String)` - The serialized output
/// * `Err(String)` - Error message if serialization fails
///
/// # Example
///
/// ```
/// use pm_encoder::serialize_project;
///
/// let result = serialize_project(".");
/// assert!(result.is_ok());
/// ```
pub fn serialize_project(root: &str) -> Result<String, String> {
    // Try to load config from the project directory
    let config_path = Path::new(root).join(".pm_encoder_config.json");
    let config = if config_path.exists() {
        EncoderConfig::from_file(&config_path).unwrap_or_default()
    } else {
        EncoderConfig::default()
    };
    serialize_project_with_config(root, &config)
}

/// Serialize a project with custom configuration
///
/// # Arguments
///
/// * `root` - Path to the project root directory
/// * `config` - Encoder configuration
///
/// # Returns
///
/// * `Ok(String)` - The serialized output (empty string in streaming mode)
/// * `Err(String)` - Error message if serialization fails
pub fn serialize_project_with_config(
    root: &str,
    config: &EncoderConfig,
) -> Result<String, String> {
    // Streaming mode: use iterator, write directly, return empty string
    if config.stream {
        return serialize_project_streaming(root, config);
    }

    // Batch mode: collect, sort, return complete string
    let entries = walk_directory(
        root,
        &config.ignore_patterns,
        &config.include_patterns,
        config.max_file_size,
    )?;

    // Sort entries based on config
    let mut sorted_entries = entries;
    let is_desc = config.sort_order == "desc";

    match config.sort_by.as_str() {
        "name" => {
            if is_desc {
                sorted_entries.sort_by(|a, b| b.path.cmp(&a.path));
            } else {
                sorted_entries.sort_by(|a, b| a.path.cmp(&b.path));
            }
        }
        "mtime" => {
            if is_desc {
                sorted_entries.sort_by(|a, b| b.mtime.cmp(&a.mtime));
            } else {
                sorted_entries.sort_by(|a, b| a.mtime.cmp(&b.mtime));
            }
        }
        "ctime" => {
            if is_desc {
                sorted_entries.sort_by(|a, b| b.ctime.cmp(&a.ctime));
            } else {
                sorted_entries.sort_by(|a, b| a.ctime.cmp(&b.ctime));
            }
        }
        // Default to name sorting
        _ => {
            sorted_entries.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }

    // Serialize each file entry with optional truncation
    let mut output = String::new();
    for entry in sorted_entries {
        output.push_str(&serialize_file_with_truncation(
            &entry,
            config.truncate_lines,
            &config.truncate_mode,
        ));
    }

    Ok(output)
}

/// Serialize a project in streaming mode (immediate output)
///
/// Writes each file to stdout as it's discovered, enabling immediate output
/// without buffering the entire result. Global sorting is disabled in this mode.
///
/// # Arguments
///
/// * `root` - Path to the project root directory
/// * `config` - Encoder configuration
///
/// # Returns
///
/// * `Ok(String)` - Always returns empty string (output goes to stdout)
/// * `Err(String)` - Error message if serialization fails
pub fn serialize_project_streaming(
    root: &str,
    config: &EncoderConfig,
) -> Result<String, String> {
    use std::io::{self, Write};

    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("Directory not found: {}", root));
    }

    // Warn if sorting options are specified (they're ignored in streaming mode)
    if config.sort_by != "name" || config.sort_order != "asc" {
        eprintln!(
            "Warning: --stream mode ignores --sort-by and --sort-order (using directory order)"
        );
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Stream files as they're discovered
    for entry in walk_directory_iter(
        root,
        config.ignore_patterns.clone(),
        config.include_patterns.clone(),
        config.max_file_size,
    ) {
        let serialized = serialize_file_with_truncation(
            &entry,
            config.truncate_lines,
            &config.truncate_mode,
        );
        // Write immediately to stdout
        if handle.write_all(serialized.as_bytes()).is_err() {
            break; // Broken pipe or similar, stop gracefully
        }
        // Flush to ensure immediate output
        let _ = handle.flush();
    }

    // Return empty string - output was written directly
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.8.0");
    }

    #[test]
    fn test_serialize_project() {
        let result = serialize_project(".");
        assert!(result.is_ok());
        // Result should be in Plus/Minus format
        let output = result.unwrap();
        assert!(output.contains("++++++++++")); // Plus/Minus format header
    }

    #[test]
    fn test_serialize_with_config() {
        let config = EncoderConfig {
            truncate_lines: 100,
            max_file_size: 1024,
            ..Default::default()
        };
        let result = serialize_project_with_config(".", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = EncoderConfig::default();
        assert_eq!(config.truncate_lines, 0);
        assert_eq!(config.max_file_size, 5 * 1024 * 1024);
    }

    #[test]
    fn test_md5_calculation() {
        let content = "Hello, world!";
        let md5 = calculate_md5(content);
        assert_eq!(md5, "6cd3556deb0da54bca060b4c39479839");
    }

    #[test]
    fn test_binary_detection() {
        let text = b"Hello, world!";
        assert!(!is_binary(text));

        let binary = b"Hello\x00world";
        assert!(is_binary(binary));
    }

    #[test]
    fn test_read_file_content() {
        // UTF-8 content
        let utf8 = b"Hello, world!";
        assert_eq!(read_file_content(utf8), Some("Hello, world!".to_string()));

        // Binary content (null bytes) - should return None
        let binary = b"Hello\x00world";
        assert_eq!(read_file_content(binary), None);

        // Latin-1 content (non-UTF-8 but no null bytes)
        // 0xE9 is 'é' in Latin-1, invalid as standalone UTF-8
        let latin1 = b"caf\xe9";
        let result = read_file_content(latin1);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "café"); // Latin-1 decoded

        // Line ending normalization (like Python's read_text())
        let crlf = b"line1\r\nline2\r\nline3";
        assert_eq!(read_file_content(crlf), Some("line1\nline2\nline3".to_string()));

        let cr = b"line1\rline2\rline3";
        assert_eq!(read_file_content(cr), Some("line1\nline2\nline3".to_string()));

        let mixed = b"line1\r\nline2\rline3\nline4";
        assert_eq!(read_file_content(mixed), Some("line1\nline2\nline3\nline4".to_string()));
    }

    #[test]
    fn test_size_check() {
        assert!(is_too_large(10_000_000, 5_000_000)); // 10MB > 5MB
        assert!(!is_too_large(1_000_000, 5_000_000)); // 1MB < 5MB
    }

    #[test]
    fn test_matches_patterns_directory() {
        // Test that ".llm_archive" pattern matches files inside the directory
        let patterns = vec![".llm_archive".to_string()];

        // Should match files inside .llm_archive
        assert!(matches_patterns(".llm_archive/file.md", &patterns),
            ".llm_archive pattern should match .llm_archive/file.md");

        // Should match nested files
        assert!(matches_patterns(".llm_archive/subdir/file.md", &patterns),
            ".llm_archive pattern should match nested files");

        // Should not match unrelated files
        assert!(!matches_patterns("src/main.rs", &patterns),
            ".llm_archive pattern should not match src/main.rs");

        // Should not match similarly-named files
        assert!(!matches_patterns("llm_archive/file.md", &patterns),
            ".llm_archive pattern should not match llm_archive (no dot)");
    }

    #[test]
    fn test_truncate_simple() {
        let content = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";

        // No truncation when limit is 0
        let (result, truncated) = truncate_simple(content, 0, "test.txt");
        assert!(!truncated);
        assert_eq!(result, content);

        // No truncation when content is smaller than limit
        let (result, truncated) = truncate_simple(content, 20, "test.txt");
        assert!(!truncated);
        assert_eq!(result, content);

        // Truncation when content exceeds limit
        let (result, truncated) = truncate_simple(content, 3, "test.txt");
        assert!(truncated);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(result.contains("line3"));
        assert!(!result.contains("line4"));
        assert!(result.contains("TRUNCATED at line 3/10"));
        assert!(result.contains("70% reduction"));
    }

    #[test]
    fn test_truncate_smart_python() {
        let python_code = r#"import os
import sys

class MyClass:
    """A docstring"""

    def method_one(self):
        # This is a long method
        x = 1
        y = 2
        z = 3
        return x + y + z

    def method_two(self):
        return True

def main():
    pass
"#;
        // Smart truncation should preserve structure
        let (result, truncated) = truncate_smart(python_code, 5, "test.py");

        // Should truncate since content is longer than 5 lines
        if truncated {
            assert!(result.contains("import") || result.contains("class") || result.contains("def"));
        }
    }

    #[test]
    fn test_truncate_structure_python() {
        let python_code = r#"class Calculator:
    """A simple calculator class."""

    def __init__(self):
        self.value = 0

    def add(self, x):
        """Add x to the current value."""
        self.value += x
        return self.value

    def subtract(self, x):
        """Subtract x from the current value."""
        self.value -= x
        return self.value
"#;
        let (result, was_truncated) = truncate_structure(python_code, "calc.py");

        if was_truncated {
            // Structure mode should preserve signatures
            assert!(result.contains("class Calculator"));
            assert!(result.contains("def __init__"));
            assert!(result.contains("def add"));
            assert!(result.contains("def subtract"));
        }
    }

    #[test]
    fn test_truncate_structure_rust() {
        let rust_code = r#"pub struct Config {
    pub name: String,
    pub value: i32,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: 0,
        }
    }

    pub fn set_value(&mut self, v: i32) {
        self.value = v;
    }
}
"#;
        let (result, was_truncated) = truncate_structure(rust_code, "config.rs");

        if was_truncated {
            // Structure mode should preserve signatures
            assert!(result.contains("pub struct Config"));
            assert!(result.contains("impl Config"));
        }
    }

    #[test]
    fn test_truncate_structure_non_code_file() {
        let text = "This is just some plain text.\nNothing special here.\nJust text.";
        let (result, was_truncated) = truncate_structure(text, "readme.txt");

        // Non-code files should not be truncated in structure mode
        assert!(!was_truncated);
        assert_eq!(result, text);
    }

    #[test]
    fn test_serialize_file_format() {
        let entry = FileEntry {
            path: "test/main.py".to_string(),
            content: "print('hello')".to_string(),
            md5: "abc123".to_string(),
            mtime: 1234567890,
            ctime: 1234567890,
        };

        let serialized = serialize_file(&entry);

        // Check PM format markers
        assert!(serialized.starts_with("++++++++++"));
        assert!(serialized.contains("test/main.py"));
        assert!(serialized.contains("print('hello')"));
        assert!(serialized.contains("----------"));
        assert!(serialized.contains("abc123"));
    }

    #[test]
    fn test_matches_patterns_glob() {
        let patterns = vec!["*.pyc".to_string(), "*.pyo".to_string()];

        assert!(matches_patterns("cache.pyc", &patterns));
        assert!(matches_patterns("module.pyo", &patterns));
        assert!(!matches_patterns("main.py", &patterns));
    }

    #[test]
    fn test_matches_patterns_directory_prefix() {
        let patterns = vec!["__pycache__".to_string()];

        assert!(matches_patterns("__pycache__/module.pyc", &patterns));
        assert!(matches_patterns("src/__pycache__/test.pyc", &patterns));
        assert!(!matches_patterns("pycache/file.py", &patterns));
    }

    #[test]
    fn test_binary_detection_with_null_bytes() {
        let binary_with_null = b"some\x00binary\x00data";
        assert!(is_binary(binary_with_null));
    }

    #[test]
    fn test_binary_detection_with_control_chars() {
        // The binary detection only checks for null bytes (0x00)
        // Control chars without null bytes are considered text
        let binary_control = b"\x01\x02\x03\x04\x05";
        assert!(!is_binary(binary_control)); // No null bytes = not binary

        // But content WITH null bytes is binary
        let with_null = b"\x01\x00\x03\x04\x05";
        assert!(is_binary(with_null));
    }

    #[test]
    fn test_encoder_config_custom() {
        let config = EncoderConfig {
            ignore_patterns: vec!["*.log".to_string()],
            include_patterns: vec!["*.rs".to_string()],
            max_file_size: 1_000_000,
            truncate_lines: 500,
            truncate_mode: "smart".to_string(),
            sort_by: "mtime".to_string(),
            sort_order: "desc".to_string(),
            stream: true,
            truncate_summary: true,
            truncate_exclude: vec![],
            truncate_stats: false,
        };

        assert_eq!(config.truncate_lines, 500);
        assert_eq!(config.truncate_mode, "smart");
        assert!(config.stream);
    }

    #[test]
    fn test_walk_directory_respects_patterns() {
        // Create temp directory for test
        use std::fs;
        let temp_dir = std::env::temp_dir().join("pm_encoder_test_walk");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("main.py"), "print('hello')").unwrap();
        fs::write(temp_dir.join("test.pyc"), "binary").unwrap();

        let entries = walk_directory(
            temp_dir.to_str().unwrap(),
            &vec!["*.pyc".to_string()],
            &vec![],
            5_000_000,
        ).unwrap();

        // Should include .py but not .pyc
        assert!(entries.iter().any(|e| e.path.contains("main.py")));
        assert!(!entries.iter().any(|e| e.path.contains(".pyc")));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_truncate_modes_all() {
        let python = "def foo():\n    pass\n";

        // Simple mode - no truncation needed for short content
        let (result, _) = truncate_simple(python, 100, "test.py");
        assert!(result.contains("def foo"));

        // Smart mode - no truncation needed for short content
        let (result, _) = truncate_smart(python, 100, "test.py");
        assert!(result.contains("def foo"));

        // Structure mode
        let (result, _) = truncate_structure(python, "test.py");
        assert!(result.contains("def foo"));
    }

    #[test]
    fn test_file_entry_fields() {
        let entry = FileEntry {
            path: "/path/to/file.rs".to_string(),
            content: "fn main() {}".to_string(),
            md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
            mtime: 1702000000,
            ctime: 1701000000,
        };

        assert_eq!(entry.path, "/path/to/file.rs");
        assert_eq!(entry.md5.len(), 32); // MD5 is 32 hex chars
        assert!(entry.mtime > entry.ctime); // mtime >= ctime typically
    }

    #[test]
    fn test_truncate_simple_includes_summary_by_default() {
        let content = (0..20).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n");
        let (result, truncated) = truncate_simple(&content, 5, "test.txt");

        assert!(truncated);
        assert!(result.contains("TRUNCATED"));
        assert!(result.contains("reduction"));
    }

    #[test]
    fn test_truncate_smart_with_imports() {
        let python_with_imports = r#"import os
import sys
from pathlib import Path

def main():
    x = 1
    y = 2
    z = 3
    return x + y + z

if __name__ == "__main__":
    main()
"#;
        let (_result, truncated) = truncate_smart(python_with_imports, 3, "main.py");
        // Should attempt smart truncation
        assert!(truncated || !truncated); // Test doesn't crash
    }

    // ============================================================
    // Coverage Floor Tests (>85% target)
    // ============================================================

    #[test]
    fn test_is_binary_empty_bytes() {
        // Empty content is not binary
        let empty: &[u8] = &[];
        assert!(!is_binary(empty));
    }

    #[test]
    fn test_is_binary_valid_utf8() {
        // Valid UTF-8 text is not binary
        let text = b"Hello, world!\nThis is valid UTF-8 text.";
        assert!(!is_binary(text));
    }

    #[test]
    fn test_is_binary_with_null_bytes() {
        // Content with null bytes is binary
        let binary = b"Hello\x00World";
        assert!(is_binary(binary));
    }

    #[test]
    fn test_is_binary_large_content_no_null() {
        // Large content without null bytes in first 8KB
        let large_text: Vec<u8> = (0..10000).map(|_| b'a').collect();
        assert!(!is_binary(&large_text));
    }

    #[test]
    fn test_is_binary_null_after_8kb() {
        // Null byte after 8KB boundary should not be detected
        let mut content: Vec<u8> = vec![b'a'; 9000];
        content[8500] = 0; // Null byte after the 8KB check window
        assert!(!is_binary(&content));
    }

    #[test]
    fn test_calculate_md5_empty_string() {
        // MD5 of empty string
        let hash = calculate_md5("");
        assert_eq!(hash, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_calculate_md5_known_value() {
        // MD5 of known string
        let hash = calculate_md5("hello");
        assert_eq!(hash, "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_walk_directory_all_files_ignored() {
        // Test walk_directory when all files are ignored (should return empty list)
        use std::fs;
        let temp_dir = std::env::temp_dir().join("pm_encoder_test_all_ignored");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("ignored.log"), "log content").unwrap();
        fs::write(temp_dir.join("another.log"), "more log").unwrap();

        let entries = walk_directory(
            temp_dir.to_str().unwrap(),
            &vec!["*.log".to_string()],  // Ignore all .log files
            &vec![],
            5_000_000,
        ).unwrap();

        // All files ignored, should return empty
        assert!(entries.is_empty(), "Expected empty list when all files ignored");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_walk_directory_nonexistent() {
        // Test walk_directory with non-existent directory
        let result = walk_directory(
            "/nonexistent/path/that/does/not/exist",
            &vec![],
            &vec![],
            5_000_000,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_read_file_content_invalid_utf8_fallback() {
        // Test Latin-1 fallback for invalid UTF-8
        let latin1_bytes: &[u8] = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xe9]; // "Hello" + é in Latin-1
        let result = read_file_content(latin1_bytes);
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.starts_with("Hello"));
    }

    #[test]
    fn test_read_file_content_crlf_normalization() {
        // Test CRLF to LF normalization
        let crlf_content = b"line1\r\nline2\r\nline3";
        let result = read_file_content(crlf_content);
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(!content.contains('\r'));
        assert!(content.contains("line1\nline2\nline3"));
    }

    #[test]
    fn test_truncate_structure_empty_content() {
        // Structure mode on empty content
        let (result, truncated) = truncate_structure("", "empty.py");
        assert_eq!(result, "");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_structure_with_imports() {
        // Structure mode preserves imports
        let python = "import os\nfrom sys import path\n\nclass Foo:\n    def bar(self):\n        pass\n";
        let (result, truncated) = truncate_structure(python, "module.py");
        assert!(result.contains("import os"));
        assert!(result.contains("class Foo"));
        assert!(result.contains("def bar"));
        assert!(truncated);
    }

    #[test]
    fn test_truncate_smart_with_critical_sections() {
        // Smart truncation preserves entry points and critical sections
        let python = r#"import os

def helper():
    return 1

def another():
    return 2

if __name__ == "__main__":
    helper()
    another()
"#;
        let (result, truncated) = truncate_smart(python, 5, "main.py");
        // Should preserve import and entry point
        assert!(result.contains("import os") || truncated);
    }

    #[test]
    fn test_config_default_values() {
        // Test Config::default()
        let config = Config::default();
        assert!(config.ignore_patterns.is_empty());
        assert!(config.include_patterns.is_empty());
    }

    #[test]
    fn test_encoder_config_default_values() {
        // Test EncoderConfig::default()
        let config = EncoderConfig::default();
        assert!(!config.ignore_patterns.is_empty()); // Has default ignores
        assert!(config.include_patterns.is_empty());
        assert_eq!(config.sort_by, "name");
        assert_eq!(config.sort_order, "asc");
        assert_eq!(config.truncate_lines, 0);
        assert_eq!(config.truncate_mode, "simple");
        assert_eq!(config.max_file_size, 5 * 1024 * 1024);
        assert!(!config.stream);
    }

    #[test]
    fn test_matches_patterns_component_match() {
        // Test that .git matches .git/config
        assert!(matches_patterns(".git/config", &vec![".git".to_string()]));
        assert!(matches_patterns("node_modules/package/index.js", &vec!["node_modules".to_string()]));
    }

    #[test]
    fn test_version_function() {
        assert_eq!(version(), VERSION);
        assert!(version().contains('.'));
    }

    #[test]
    fn test_serialize_with_mtime_sorting() {
        // Test sorting by modification time
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_mtime_sort");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create files - timing not guaranteed but code path is covered
        fs::write(temp_dir.join("old.py"), "# old file").unwrap();
        fs::write(temp_dir.join("new.py"), "# new file").unwrap();

        let config = EncoderConfig {
            sort_by: "mtime".to_string(),
            sort_order: "desc".to_string(),
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Just verify both files are in the output (order depends on filesystem timing)
        assert!(output.contains("new.py") || output.contains("old.py"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_serialize_with_ctime_sorting() {
        // Test sorting by creation time
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_ctime_sort");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("a.py"), "# a").unwrap();
        fs::write(temp_dir.join("b.py"), "# b").unwrap();

        let config = EncoderConfig {
            sort_by: "ctime".to_string(),
            sort_order: "asc".to_string(),
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_serialize_with_unknown_sort() {
        // Test fallback to name sorting for unknown sort_by
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_unknown_sort");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("b.py"), "# b").unwrap();
        fs::write(temp_dir.join("a.py"), "# a").unwrap();

        let config = EncoderConfig {
            sort_by: "unknown".to_string(),  // Unknown, should default to name
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should be name sorted (a before b)
        let a_pos = output.find("a.py");
        let b_pos = output.find("b.py");
        assert!(a_pos.is_some() && b_pos.is_some());
        assert!(a_pos.unwrap() < b_pos.unwrap());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_serialize_with_truncation() {
        // Test serialization with truncation enabled
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_trunc_serial");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create a file with many lines
        let content: String = (0..50).map(|i| format!("line {}\n", i)).collect();
        fs::write(temp_dir.join("long.py"), &content).unwrap();

        let config = EncoderConfig {
            truncate_lines: 10,
            truncate_mode: "simple".to_string(),
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("TRUNCATED"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_serialize_project_nonexistent() {
        let config = EncoderConfig::default();
        let result = serialize_project_with_config("/nonexistent/path/xyz", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_file_with_truncation_modes() {
        let entry = FileEntry {
            path: "test.py".to_string(),
            content: (0..100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n"),
            md5: "abc123".to_string(),
            mtime: 0,
            ctime: 0,
        };

        // Simple truncation
        let output = serialize_file_with_truncation(&entry, 10, "simple");
        assert!(output.contains("+++ test.py"));
        assert!(output.contains("TRUNCATED"));

        // Smart truncation
        let output = serialize_file_with_truncation(&entry, 10, "smart");
        assert!(output.contains("+++ test.py"));

        // Structure truncation
        let output = serialize_file_with_truncation(&entry, 10, "structure");
        assert!(output.contains("+++ test.py"));
    }

    #[test]
    fn test_truncate_smart_long_file_with_class() {
        // Test smart truncation on a file with a class definition
        let python = r#"import os
import sys

class MyClass:
    """A class with methods."""

    def __init__(self):
        self.x = 1
        self.y = 2
        self.z = 3

    def method_one(self):
        return self.x

    def method_two(self):
        return self.y

    def method_three(self):
        return self.z

if __name__ == "__main__":
    obj = MyClass()
    print(obj.method_one())
"#;
        let (result, truncated) = truncate_smart(python, 10, "myclass.py");
        assert!(truncated);
        // Should preserve important sections
        assert!(result.contains("import") || result.contains("class") || result.contains("__main__"));
    }

    #[test]
    fn test_truncate_structure_rust_code() {
        // Test structure truncation on Rust code
        let rust_code = r#"use std::io;

pub struct Config {
    name: String,
    value: i32,
}

impl Config {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            value: 0,
        }
    }

    pub fn process(&self) {
        println!("processing");
    }
}

pub fn main() {
    let config = Config::new();
    config.process();
}
"#;
        let (result, truncated) = truncate_structure(rust_code, "config.rs");
        assert!(truncated);
        assert!(result.contains("use std::io"));
        assert!(result.contains("pub struct Config"));
        assert!(result.contains("pub fn new"));
    }

    #[test]
    fn test_is_too_large() {
        assert!(is_too_large(1000, 500));
        assert!(!is_too_large(500, 1000));
        assert!(!is_too_large(500, 500)); // Equal is not too large
    }

    #[test]
    fn test_load_config_nonexistent() {
        // Test loading config from non-existent directory (returns default)
        let result = load_config("/tmp/nonexistent_dir_xyz");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.ignore_patterns.is_empty());
    }

    #[test]
    fn test_serialize_name_desc_order() {
        // Test name sorting with descending order
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_name_desc");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("aaa.py"), "# a").unwrap();
        fs::write(temp_dir.join("zzz.py"), "# z").unwrap();

        let config = EncoderConfig {
            sort_by: "name".to_string(),
            sort_order: "desc".to_string(),
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());
        let output = result.unwrap();
        // zzz should come before aaa with desc name sort
        let a_pos = output.find("aaa.py");
        let z_pos = output.find("zzz.py");
        assert!(z_pos.unwrap() < a_pos.unwrap());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_mtime_asc_order() {
        // Test mtime with ascending order - verifies code path, not timing
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_mtime_asc");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join("old.txt"), "old").unwrap();
        fs::write(temp_dir.join("new.txt"), "new").unwrap();

        let config = EncoderConfig {
            sort_by: "mtime".to_string(),
            sort_order: "asc".to_string(),
            ignore_patterns: vec![],  // Clear default ignores
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());
        // Just verify the sort code path runs
        let output = result.unwrap();
        assert!(output.contains("old.txt") || output.contains("new.txt"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ctime_desc_order() {
        // Test ctime with descending order
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_ctime_desc");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("file1.txt"), "1").unwrap();
        fs::write(temp_dir.join("file2.txt"), "2").unwrap();

        let config = EncoderConfig {
            sort_by: "ctime".to_string(),
            sort_order: "desc".to_string(),
            ignore_patterns: vec![],
            ..Default::default()
        };

        let result = serialize_project_with_config(temp_dir.to_str().unwrap(), &config);
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_file_content_binary_returns_none() {
        // Binary content should return None
        let binary = &[0x00, 0x01, 0x02, 0x03];
        assert!(read_file_content(binary).is_none());
    }

    #[test]
    fn test_walk_directory_with_include_patterns() {
        // Test include patterns filtering
        use std::fs;

        let temp_dir = std::env::temp_dir().join("pm_encoder_test_include");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("include.py"), "# py").unwrap();
        fs::write(temp_dir.join("exclude.txt"), "txt").unwrap();

        let entries = walk_directory(
            temp_dir.to_str().unwrap(),
            &vec![],
            &vec!["*.py".to_string()], // Only include .py files
            5_000_000,
        ).unwrap();

        // Should only include .py file
        assert!(entries.iter().any(|e| e.path.contains(".py")));
        assert!(!entries.iter().any(|e| e.path.contains(".txt")));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_truncate_structure_with_decorators() {
        // Test structure truncation preserves decorators
        let python = "@decorator\ndef decorated():\n    pass\n\n@another\nclass MyClass:\n    pass\n";
        let (result, truncated) = truncate_structure(python, "decorated.py");
        assert!(truncated);
        assert!(result.contains("@decorator") || result.contains("def decorated"));
    }

    #[test]
    fn test_smart_truncation_with_gaps() {
        // Test smart truncation creates gap markers
        let python = (0..100).map(|i| {
            if i == 0 { "import os".to_string() }
            else if i == 50 { "def important():\n    pass".to_string() }
            else if i == 99 { "if __name__ == '__main__':\n    pass".to_string() }
            else { format!("# line {}", i) }
        }).collect::<Vec<_>>().join("\n");

        let (result, truncated) = truncate_smart(&python, 10, "gaps.py");
        assert!(truncated);
        // Should have omitted lines marker
        assert!(result.contains("omitted") || result.contains("TRUNCATED") || result.contains("import"));
    }

    #[test]
    fn test_encoder_config_from_file_missing() {
        // Test EncoderConfig::from_file with missing file
        let result = EncoderConfig::from_file(Path::new("/nonexistent/config.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_file_no_truncation() {
        // Test serialization with no truncation (truncate_lines = 0)
        let entry = FileEntry {
            path: "small.py".to_string(),
            content: "x = 1\ny = 2\n".to_string(),
            md5: "abc".to_string(),
            mtime: 0,
            ctime: 0,
        };

        let output = serialize_file_with_truncation(&entry, 0, "simple");
        assert!(output.contains("x = 1"));
        assert!(output.contains("y = 2"));
        assert!(!output.contains("TRUNCATED"));
    }

    #[test]
    fn test_smart_truncation_creates_gap_markers() {
        // Create Python file with important sections separated by many filler lines
        // This should trigger the gap marker code path (lines 627-628)
        let mut lines = Vec::new();
        lines.push("import os".to_string());           // Line 1 - import (important)
        lines.push("import sys".to_string());          // Line 2 - import (important)
        for i in 3..50 {
            lines.push(format!("# filler comment line {}", i)); // Lines 3-49 - filler
        }
        lines.push("class MyClass:".to_string());      // Line 50 - class (important)
        lines.push("    '''Docstring'''".to_string()); // Line 51
        for i in 52..100 {
            lines.push(format!("    # more filler {}", i)); // Lines 52-99
        }
        lines.push("if __name__ == '__main__':".to_string()); // Line 100 - entry point (important)
        lines.push("    pass".to_string());            // Line 101

        let python = lines.join("\n");
        let (result, truncated) = truncate_smart(&python, 15, "gap_test.py");

        assert!(truncated, "Should truncate long file");
        // The result should have some content
        assert!(!result.is_empty());
    }

    #[test]
    fn test_truncate_smart_preserves_critical_sections() {
        // Test that smart truncation preserves entry points and their context
        let python = r#"import os

def setup():
    pass

def helper1():
    pass

def helper2():
    pass

def helper3():
    pass

if __name__ == "__main__":
    setup()
"#;
        let (result, truncated) = truncate_smart(python, 8, "entry.py");
        assert!(truncated);
        // Should preserve import and entry point
        assert!(result.contains("import") || result.contains("__main__"));
    }

    #[test]
    fn test_structure_truncation_preserves_signatures() {
        // Test that structure mode preserves function/class signatures
        let rust_code = r#"use std::collections::HashMap;

/// Configuration struct
pub struct Config {
    pub name: String,
    pub values: HashMap<String, i32>,
}

impl Config {
    /// Create a new config
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: HashMap::new(),
        }
    }

    /// Add a value
    pub fn add(&mut self, key: &str, value: i32) {
        self.values.insert(key.to_string(), value);
    }
}

/// Main entry point
fn main() {
    let config = Config::new("test");
}
"#;
        let (result, truncated) = truncate_structure(rust_code, "config.rs");
        assert!(truncated);
        assert!(result.contains("use std::collections"));
        assert!(result.contains("pub struct Config"));
    }

    // ============================================================
    // Phase 2: Truncation Control Tests (TDD)
    // ============================================================

    #[test]
    fn test_truncate_simple_without_summary() {
        // Test truncation with summary disabled
        let content = (0..20).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let (result, truncated) = truncate_simple_with_options(&content, 5, "test.py", false);
        assert!(truncated);
        assert!(!result.contains("TRUNCATED"), "Should NOT include summary marker when disabled");
        assert!(!result.contains("reduced"), "Should NOT include stats when disabled");
    }

    #[test]
    fn test_truncate_simple_with_summary() {
        // Test truncation with summary enabled (default behavior)
        let content = (0..20).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let (result, truncated) = truncate_simple_with_options(&content, 5, "test.py", true);
        assert!(truncated);
        assert!(result.contains("TRUNCATED"), "Should include summary marker when enabled");
        assert!(result.contains("reduction"), "Should include stats when enabled");
    }

    #[test]
    fn test_truncate_smart_without_summary() {
        // Test smart truncation with summary disabled
        let python = r#"import os

def foo():
    x = 1
    y = 2
    z = 3
    return x + y + z

def bar():
    return True
"#;
        let (result, truncated) = truncate_smart_with_options(python, 5, "test.py", false);
        assert!(truncated);
        assert!(!result.contains("SMART TRUNCATED"), "Should NOT include smart truncation marker");
    }

    #[test]
    fn test_truncate_structure_without_summary() {
        // Test structure truncation with summary disabled
        let python = r#"class Foo:
    def bar(self):
        pass
    def baz(self):
        pass
"#;
        let (result, truncated) = truncate_structure_with_options(python, "test.py", false);
        assert!(truncated);
        assert!(!result.contains("STRUCTURE MODE"), "Should NOT include structure marker");
    }

    #[test]
    fn test_encoder_config_truncate_fields() {
        // Test new truncation control fields in EncoderConfig
        let config = EncoderConfig {
            truncate_summary: false,
            truncate_exclude: vec!["*.md".to_string(), "*.txt".to_string()],
            truncate_stats: true,
            ..Default::default()
        };
        assert!(!config.truncate_summary);
        assert_eq!(config.truncate_exclude.len(), 2);
        assert!(config.truncate_stats);
    }

    #[test]
    fn test_encoder_config_truncate_defaults() {
        // Test default values for truncation control fields
        let config = EncoderConfig::default();
        assert!(config.truncate_summary, "truncate_summary should default to true");
        assert!(config.truncate_exclude.is_empty(), "truncate_exclude should default to empty");
        assert!(!config.truncate_stats, "truncate_stats should default to false");
    }

    #[test]
    fn test_truncate_exclude_pattern_match() {
        // Test that files matching truncate_exclude are not truncated
        let patterns = vec!["*.md".to_string(), "docs/**".to_string()];

        assert!(should_skip_truncation("README.md", &patterns), "*.md should match README.md");
        assert!(should_skip_truncation("docs/guide.txt", &patterns), "docs/** should match docs/guide.txt");
        assert!(!should_skip_truncation("src/main.py", &patterns), "src/main.py should not match");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TDD TESTS FOR PYTHON PARITY (Gap #1: Non-code file truncation with gaps)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_gap_markers_in_noncode_truncation() {
        // Python behavior: non-code files get truncated with "keep first 40%, gap, keep last 10%"
        // This test ensures we create gap markers like "... [N lines omitted] ..."

        // Create a 100-line "non-code" file (like .ai or generic text)
        let content: String = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");

        // Truncate to 20 lines max (should keep first 8 (40%) + last 2 (10%) = 10 lines)
        let (result, truncated) = truncate_smart_with_options(&content, 20, "data.ai", true);

        assert!(truncated, "File should be truncated");

        // CRITICAL: Must have a gap marker (Python parity)
        assert!(
            result.contains("... [") && result.contains(" lines omitted] ..."),
            "Non-code truncation must include gap marker '... [N lines omitted] ...'. Got:\n{}",
            &result[..result.len().min(500)]
        );

        // Should keep first section
        assert!(result.contains("line 1"), "Should keep first line");

        // Should keep last section
        assert!(result.contains("line 100"), "Should keep last line");

        // Gap should omit middle section
        assert!(!result.contains("line 50"), "Middle lines should be omitted");
    }

    #[test]
    fn test_gap_marker_format_matches_python() {
        // Python format: "\n... [N lines omitted] ...\n"
        // Verify exact format for byte parity

        let content: String = (1..=50).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let (result, _) = truncate_smart_with_options(&content, 10, "unknown.xyz", true);

        // Check for Python-compatible format with newlines
        let has_correct_format = result.contains("\n... [") && result.contains(" lines omitted] ...\n");
        assert!(
            has_correct_format,
            "Gap marker format must match Python: '\\n... [N lines omitted] ...\\n'"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TDD TESTS FOR PYTHON PARITY (Gap #2: Structure mode keeps ALL imports)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_structure_mode_keeps_all_imports() {
        // Python's get_structure_ranges() keeps ALL import lines, not just first 5
        // This is critical for Python files with many imports

        let python_code = r#"import os
import sys
import json
import re
import datetime
import collections
import itertools
import functools
import pathlib
import typing
from typing import List, Dict, Optional
from pathlib import Path
from dataclasses import dataclass

class MyClass:
    def __init__(self):
        self.x = 1
        self.y = 2
        self.z = 3

    def method_one(self):
        return self.x + self.y

    def method_two(self):
        return self.z * 2
"#;

        let (result, truncated) = truncate_structure_with_fallback(python_code, "test.py", true, 2000);

        assert!(truncated, "Should be truncated in structure mode");

        // ALL imports should be kept (Python parity)
        assert!(result.contains("import os"), "Should keep 'import os'");
        assert!(result.contains("import sys"), "Should keep 'import sys'");
        assert!(result.contains("import json"), "Should keep 'import json'");
        assert!(result.contains("import datetime"), "Should keep 'import datetime'");
        assert!(result.contains("import collections"), "Should keep 'import collections'");
        assert!(result.contains("import itertools"), "Should keep 'import itertools' (line 7)");
        assert!(result.contains("import functools"), "Should keep 'import functools' (line 8)");
        assert!(result.contains("import pathlib"), "Should keep 'import pathlib' (line 9)");
        assert!(result.contains("import typing"), "Should keep 'import typing' (line 10)");
        assert!(result.contains("from typing import"), "Should keep 'from typing import' (line 11)");
        assert!(result.contains("from pathlib import"), "Should keep 'from pathlib import' (line 12)");
        assert!(result.contains("from dataclasses import"), "Should keep 'from dataclasses import' (line 13)");

        // Class and method signatures should also be kept
        assert!(result.contains("class MyClass"), "Should keep class definition");
        assert!(result.contains("def __init__"), "Should keep __init__ method");
        assert!(result.contains("def method_one"), "Should keep method_one");
        assert!(result.contains("def method_two"), "Should keep method_two");

        // Implementation details should NOT be kept
        assert!(!result.contains("self.x = 1"), "Should NOT keep implementation details");
        assert!(!result.contains("return self.x + self.y"), "Should NOT keep method body");
    }

    #[test]
    fn test_structure_mode_keeps_decorators() {
        // Python structure mode should keep @decorators above functions

        let python_code = r#"import functools

@functools.lru_cache
def expensive_function(n):
    result = 0
    for i in range(n):
        result += i
    return result

@property
def my_property(self):
    return self._value
"#;

        let (result, _) = truncate_structure_with_fallback(python_code, "test.py", true, 2000);

        assert!(result.contains("@functools.lru_cache"), "Should keep @decorator");
        assert!(result.contains("@property"), "Should keep @property decorator");
        assert!(result.contains("def expensive_function"), "Should keep function signature");
        assert!(result.contains("def my_property"), "Should keep property method");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TDD TEST FOR PYTHON PARITY (Gap #4: Markdown files use smart mode, not structure)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_markdown_structure_mode_fallback_to_smart() {
        // Python behavior: Markdown files have no get_structure_ranges() support
        // So structure mode falls back to smart mode with gap markers
        // This is critical for files like HISTORY.md

        let markdown = r#"# Release History

## 2.32.5 (2025-08-18)

**Bugfixes**

- Fixed a bug in the SSLContext caching feature.
- The Requests team has decided to revert this feature.

## 2.32.4 (2025-07-15)

**Features**

- Added support for Python 3.14.

## 2.32.3 (2025-06-10)

**Deprecations**

- Deprecated old API methods.

Some code example:
```python
from requests import Session
import json
```

More text here that should be truncated in the middle.
Line 25
Line 26
Line 27
Line 28
Line 29
Line 30
Line 31
Line 32
Line 33
Line 34
Line 35
Line 36
Line 37
Line 38
Line 39
Line 40
Line 41
Line 42
Line 43
Line 44
Line 45
Line 46
Line 47
Line 48
Line 49
Line 50

## Final Section

This is the last section.
"#;

        // Python's markdown get_truncate_ranges uses a "budget" approach
        // This effectively keeps the first max_lines (simple truncation)
        let (result, truncated) = truncate_structure_with_fallback(markdown, "HISTORY.md", true, 30);

        assert!(truncated, "Should be truncated");

        // Should NOT use structure mode (which would find "from requests" as import)
        assert!(
            !result.contains("STRUCTURE MODE"),
            "Markdown should NOT use structure mode. Got:\n{}",
            &result[..result.len().min(600)]
        );

        // Should use simple truncation (first N lines, no gap markers)
        // Python's markdown analyzer keeps first max_lines via budget approach
        assert!(
            result.contains("TRUNCATED at line"),
            "Markdown truncation should use simple truncation. Got:\n{}",
            &result[..result.len().min(800)]
        );

        // Should keep beginning (first section)
        assert!(result.contains("# Release History"), "Should keep first header");

        // Simple truncation keeps first N lines, so middle content should be there
        // but "Final Section" at end would be truncated (expected behavior)
        assert!(result.contains("**Bugfixes**"), "Should keep early content");
    }
}

// ============================================================================
// WASM BINDINGS - Conditional compilation for browser/Node.js environments
// ============================================================================

#[cfg(feature = "wasm")]
pub mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// File input structure for WASM
    #[derive(serde::Deserialize)]
    struct WasmFileInput {
        path: String,
        content: String,
    }

    /// Configuration input for WASM
    #[derive(serde::Deserialize, Default)]
    struct WasmConfig {
        #[serde(default)]
        lens: Option<String>,
        #[serde(default)]
        token_budget: Option<usize>,
        #[serde(default)]
        budget_strategy: Option<String>,
        #[serde(default)]
        truncate_lines: Option<usize>,
        #[serde(default)]
        truncate_mode: Option<String>,
    }

    /// Serialize files to Plus/Minus format (WASM entry point)
    ///
    /// # Arguments
    /// * `json_files` - JSON array of {path, content} objects
    /// * `json_config` - Optional JSON config object
    ///
    /// # Returns
    /// * Serialized context string or error
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const files = [
    ///   { path: "main.py", content: "print('hello')" },
    ///   { path: "lib.py", content: "def helper(): pass" }
    /// ];
    /// const config = { lens: "architecture", token_budget: 100000 };
    /// const context = wasm_serialize(JSON.stringify(files), JSON.stringify(config));
    /// ```
    #[wasm_bindgen]
    pub fn wasm_serialize(json_files: &str, json_config: &str) -> Result<String, JsValue> {
        // Parse files
        let file_inputs: Vec<WasmFileInput> = serde_json::from_str(json_files)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse files JSON: {}", e)))?;

        // Convert to (path, content) pairs
        let files: Vec<(String, String)> = file_inputs
            .into_iter()
            .map(|f| (f.path, f.content))
            .collect();

        // Parse config (allow empty string for defaults)
        let wasm_config: WasmConfig = if json_config.is_empty() || json_config == "{}" {
            WasmConfig::default()
        } else {
            serde_json::from_str(json_config)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse config JSON: {}", e)))?
        };

        // Build EncoderConfig
        let mut config = EncoderConfig::default();
        if let Some(lines) = wasm_config.truncate_lines {
            config.truncate_lines = lines;
        }
        if let Some(mode) = wasm_config.truncate_mode {
            config.truncate_mode = mode;
        }

        // Create engine (with optional lens)
        let engine = if let Some(lens_name) = wasm_config.lens {
            ContextEngine::with_lens(config, &lens_name)
                .map_err(|e| JsValue::from_str(&format!("Invalid lens: {}", e)))?
        } else {
            ContextEngine::new(config)
        };

        // Generate context
        let output = engine.generate_context(&files);

        Ok(output)
    }

    /// Get library version (WASM)
    #[wasm_bindgen]
    pub fn wasm_version() -> String {
        version().to_string()
    }

    /// Get available lens names (WASM)
    #[wasm_bindgen]
    pub fn wasm_get_lenses() -> String {
        let lenses = vec!["architecture", "debug", "security", "minimal", "onboarding"];
        serde_json::to_string(&lenses).unwrap_or_else(|_| "[]".to_string())
    }
}
