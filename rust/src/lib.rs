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
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use globset::Glob;

pub mod analyzers;
pub mod lenses;

pub use lenses::{LensManager, LensConfig, AppliedLens};

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
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            ignore_patterns: vec![
                ".git".to_string(),
                "__pycache__".to_string(),
                "*.pyc".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
            ],
            include_patterns: vec![],
            sort_by: "name".to_string(),
            sort_order: "asc".to_string(),
            truncate_lines: 0,
            truncate_mode: "simple".to_string(),
            max_file_size: 5 * 1024 * 1024, // 5MB
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
            ..Default::default()
        })
    }
}

/// Version of the pm_encoder library
pub const VERSION: &str = "0.4.0";

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
                mtime,
                ctime,
            });
        }
    }

    Ok(())
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
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if max_lines == 0 || total_lines <= max_lines {
        return (content.to_string(), false);
    }

    // Keep first N lines
    let kept_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();
    let mut truncated = kept_lines.join("\n");

    // Add truncation marker (matching Python format)
    let reduced_pct = (total_lines - max_lines) * 100 / total_lines;
    let marker = format!(
        "\n\n{}\nTRUNCATED at line {}/{} ({}% reduced)\nTo get full content: --include \"{}\" --truncate 0\n{}\n",
        "=".repeat(70),
        max_lines,
        total_lines,
        reduced_pct,
        file_path,
        "=".repeat(70)
    );
    truncated.push_str(&marker);

    (truncated, true)
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

/// Truncate content using smart mode (language-aware)
///
/// Smart mode uses language analyzers to identify important sections
/// and keeps them while truncating less important parts.
pub fn truncate_smart(content: &str, max_lines: usize, file_path: &str) -> (String, bool) {
    let lines: Vec<&str> = content.lines().collect();
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

        // If we have more important lines than max_lines, fall back to simple
        if important_lines.len() > max_lines {
            return truncate_simple(content, max_lines, file_path);
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

        // Add final truncation marker
        let kept_count = important_lines.len();
        let omitted = total_lines - kept_count;
        if omitted > 0 {
            result.push_str(&format!(
                "\n{}\nSMART TRUNCATED: kept {}/{} lines ({}% reduced)\nLanguage: {} | Category: {}\n{}\n",
                "=".repeat(70),
                kept_count,
                total_lines,
                omitted * 100 / total_lines,
                analysis.language,
                analysis.category,
                "=".repeat(70)
            ));
        }

        return (result, true);
    }

    // Fall back to simple truncation if no analyzer available
    truncate_simple(content, max_lines, file_path)
}

/// Truncate content using structure mode (signatures only)
///
/// Structure mode extracts only class/function signatures, removing all bodies.
pub fn truncate_structure(content: &str, file_path: &str) -> (String, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    if total_lines == 0 {
        return (content.to_string(), false);
    }

    // Try to get an analyzer for this file type
    if let Some(analyzer) = analyzers::get_analyzer_for_file(file_path) {
        let analysis = analyzer.analyze(content, file_path);

        // Collect signature lines (class/function definitions)
        let mut signature_lines: Vec<usize> = Vec::new();

        // Always keep imports/headers (first few lines)
        for i in 1..=5.min(total_lines) {
            if lines[i-1].trim().starts_with("import ")
                || lines[i-1].trim().starts_with("from ")
                || lines[i-1].trim().starts_with("use ")
                || lines[i-1].trim().starts_with("#!")
                || lines[i-1].trim().starts_with("//!")
            {
                signature_lines.push(i);
            }
        }

        // Find all class/function definition lines
        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim_start();

            if trimmed.starts_with("class ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("pub const ")
            {
                signature_lines.push(line_num);
                // Also grab docstring/decorators above
                if line_num > 1 {
                    let prev = lines[line_num - 2].trim();
                    if prev.starts_with("@")
                        || prev.starts_with("///")
                        || prev.starts_with("#[")
                        || prev.starts_with("\"\"\"")
                    {
                        signature_lines.push(line_num - 1);
                    }
                }
            }
        }

        // Deduplicate and sort
        signature_lines.sort();
        signature_lines.dedup();

        if signature_lines.is_empty() {
            // No structure found, return first 20 lines
            let kept: Vec<&str> = lines.iter().take(20).copied().collect();
            let mut result = kept.join("\n");
            if total_lines > 20 {
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

        // Add structure marker
        let kept_count = signature_lines.len();
        result.push_str(&format!(
            "\n{}\nSTRUCTURE MODE: {} signatures extracted from {} lines\nLanguage: {} | Classes: {} | Functions: {}\n{}\n",
            "=".repeat(70),
            kept_count,
            total_lines,
            analysis.language,
            analysis.classes.len(),
            analysis.functions.len(),
            "=".repeat(70)
        ));

        return (result, true);
    }

    // Fall back to first 30 lines if no analyzer
    let kept: Vec<&str> = lines.iter().take(30).copied().collect();
    let mut result = kept.join("\n");
    if total_lines > 30 {
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
pub fn serialize_file_with_truncation(
    entry: &FileEntry,
    truncate_lines: usize,
    truncate_mode: &str,
) -> String {
    let mut output = String::new();

    // Header: ++++++++++ filename ++++++++++
    output.push_str(&format!("++++++++++ {} ++++++++++\n", entry.path));

    // Apply truncation if needed
    let content = if truncate_lines > 0 {
        match truncate_mode {
            "simple" => {
                let (truncated, _) = truncate_simple(&entry.content, truncate_lines, &entry.path);
                truncated
            }
            "smart" => {
                let (truncated, _) = truncate_smart(&entry.content, truncate_lines, &entry.path);
                truncated
            }
            "structure" => {
                let (truncated, _) = truncate_structure(&entry.content, &entry.path);
                truncated
            }
            _ => entry.content.clone(),
        }
    } else {
        entry.content.clone()
    };

    // Content
    output.push_str(&content);

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
/// * `Ok(String)` - The serialized output
/// * `Err(String)` - Error message if serialization fails
pub fn serialize_project_with_config(
    root: &str,
    config: &EncoderConfig,
) -> Result<String, String> {
    // Walk directory and collect file entries using EncoderConfig patterns
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.4.0");
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
    fn test_size_check() {
        assert!(is_too_large(10_000_000, 5_000_000)); // 10MB > 5MB
        assert!(!is_too_large(1_000_000, 5_000_000)); // 1MB < 5MB
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
        assert!(result.contains("70% reduced"));
    }
}
