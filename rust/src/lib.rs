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
use std::io::Read;
use std::path::Path;
use serde::{Deserialize, Serialize};
use globset::Glob;

/// A file entry with its content and metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Relative path to the file
    pub path: String,
    /// File content as string
    pub content: String,
    /// MD5 checksum of the content
    pub md5: String,
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

/// Configuration for the encoder
pub struct EncoderConfig {
    /// Enable truncation of large files
    pub truncate: bool,
    /// Maximum file size in bytes (default: 5MB)
    pub max_file_size: u64,
    // Future fields:
    // pub max_lines: usize,
    // pub truncate_mode: TruncateMode,
    // pub sort_by: SortBy,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            truncate: false,
            max_file_size: 5 * 1024 * 1024, // 5MB
        }
    }
}

/// Returns the version of the pm_encoder library
pub fn version() -> &'static str {
    "0.3.0"
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
/// * `include_patterns` - Patterns to include (overrides ignore)
///
/// # Returns
///
/// * `true` if file should be included, `false` otherwise
///
/// # Logic
///
/// - If include_patterns is non-empty and ignore_patterns is empty:
///   → Whitelist mode: only include files matching include_patterns
/// - If both include_patterns and ignore_patterns are non-empty:
///   → Precedence mode: include_patterns override ignore_patterns,
///     but files matching neither follow ignore rules
/// - If only ignore_patterns is non-empty:
///   → Blacklist mode: exclude files matching ignore_patterns
fn should_include_file(
    path: &str,
    ignore_patterns: &[String],
    include_patterns: &[String],
) -> bool {
    let has_include = !include_patterns.is_empty();
    let has_ignore = !ignore_patterns.is_empty();

    // Case 1: Only include patterns (whitelist mode)
    if has_include && !has_ignore {
        return matches_patterns(path, include_patterns);
    }

    // Case 2: Both include and ignore patterns (precedence mode)
    if has_include && has_ignore {
        // Include patterns take precedence
        if matches_patterns(path, include_patterns) {
            return true;
        }
        // Otherwise apply ignore patterns
        return !matches_patterns(path, ignore_patterns);
    }

    // Case 3: Only ignore patterns (blacklist mode)
    !matches_patterns(path, ignore_patterns)
}

/// Walk directory and collect file entries
///
/// # Arguments
///
/// * `root` - Root directory path
/// * `ignore_patterns` - Patterns to ignore
/// * `include_patterns` - Patterns to include (overrides ignore)
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

    let mut entries = Vec::new();
    walk_recursive(
        root_path,
        root_path,
        &mut entries,
        max_size,
        ignore_patterns,
        include_patterns,
    )?;

    Ok(entries)
}

/// Recursive directory walker helper
fn walk_recursive(
    current: &Path,
    root: &Path,
    entries: &mut Vec<FileEntry>,
    max_size: u64,
    ignore_patterns: &[String],
    include_patterns: &[String],
) -> Result<(), String> {
    if !current.is_dir() {
        return Ok(());
    }

    let read_dir = fs::read_dir(current).map_err(|e| format!("Failed to read dir: {}", e))?;

    for entry_result in read_dir {
        let entry = entry_result.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            // For directories, always recurse (don't filter by patterns yet)
            // because files inside might match even if the directory doesn't
            walk_recursive(&path, root, entries, max_size, ignore_patterns, include_patterns)?;
        } else if path.is_file() {
            // Calculate relative path for pattern matching
            let rel_path = path
                .strip_prefix(root)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;

            let path_str = rel_path
                .to_str()
                .ok_or_else(|| format!("Path is not valid UTF-8: {}", rel_path.display()))?;

            // Check if this file should be included based on patterns
            if !should_include_file(path_str, ignore_patterns, include_patterns) {
                continue;
            }
            // Get file metadata
            let metadata = fs::metadata(&path).map_err(|e| format!("Failed to read metadata: {}", e))?;
            let file_size = metadata.len();

            // Skip files that are too large
            if is_too_large(file_size, max_size) {
                continue;
            }

            // Read file content
            let mut file = fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            // Skip binary files
            if is_binary(&buffer) {
                continue;
            }

            // Convert to UTF-8 string
            let content = String::from_utf8(buffer)
                .map_err(|_| format!("File is not valid UTF-8: {}", path.display()))?;

            // Calculate MD5
            let md5 = calculate_md5(&content);

            entries.push(FileEntry {
                path: path_str.to_string(),
                content,
                md5,
            });
        }
    }

    Ok(())
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
    let mut output = String::new();

    // Header: ++++++++++ filename ++++++++++
    output.push_str(&format!("++++++++++ {} ++++++++++\n", entry.path));

    // Content
    output.push_str(&entry.content);

    // Ensure content ends with newline
    if !output.ends_with('\n') {
        output.push('\n');
    }

    // Footer: ---------- filename checksum filename ----------
    output.push_str(&format!(
        "---------- {} {} {} ----------\n",
        entry.path, entry.md5, entry.path
    ));

    output
}

/// Serialize a project directory into the Plus/Minus format
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
    let config = EncoderConfig::default();
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
/// * `Ok(String)` - The serialized output
/// * `Err(String)` - Error message if serialization fails
pub fn serialize_project_with_config(
    root: &str,
    config: &EncoderConfig,
) -> Result<String, String> {
    // Load .pm_encoder_config.json if it exists
    let file_config = load_config(root)?;

    // Walk directory and collect file entries
    let entries = walk_directory(
        root,
        &file_config.ignore_patterns,
        &file_config.include_patterns,
        config.max_file_size,
    )?;

    // Sort entries by path name (ascending)
    let mut sorted_entries = entries;
    sorted_entries.sort_by(|a, b| a.path.cmp(&b.path));

    // Serialize each file entry
    let mut output = String::new();
    for entry in sorted_entries {
        output.push_str(&serialize_file(&entry));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.3.0");
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
            truncate: true,
            max_file_size: 1024,
        };
        let result = serialize_project_with_config(".", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = EncoderConfig::default();
        assert_eq!(config.truncate, false);
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
    fn test_size_check() {
        assert!(is_too_large(10_000_000, 5_000_000)); // 10MB > 5MB
        assert!(!is_too_large(1_000_000, 5_000_000)); // 1MB < 5MB
    }
}
