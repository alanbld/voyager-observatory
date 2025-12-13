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
    "0.2.0"
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

/// Walk directory and collect file entries
///
/// # Arguments
///
/// * `root` - Root directory path
/// * `include` - Include patterns (empty = all files)
/// * `exclude` - Exclude patterns
/// * `max_size` - Maximum file size in bytes
///
/// # Returns
///
/// * `Ok(Vec<FileEntry>)` - List of file entries
/// * `Err(String)` - Error message if walk fails
pub fn walk_directory(
    root: &str,
    _include: &[&str],
    _exclude: &[&str],
    max_size: u64,
) -> Result<Vec<FileEntry>, String> {
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("Directory not found: {}", root));
    }

    let mut entries = Vec::new();
    walk_recursive(root_path, root_path, &mut entries, max_size)?;

    Ok(entries)
}

/// Recursive directory walker helper
fn walk_recursive(
    current: &Path,
    root: &Path,
    entries: &mut Vec<FileEntry>,
    max_size: u64,
) -> Result<(), String> {
    if !current.is_dir() {
        return Ok(());
    }

    let read_dir = fs::read_dir(current).map_err(|e| format!("Failed to read dir: {}", e))?;

    for entry_result in read_dir {
        let entry = entry_result.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        // Skip hidden files and common ignore patterns
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        if path.is_dir() {
            // Recursively walk subdirectories
            walk_recursive(&path, root, entries, max_size)?;
        } else if path.is_file() {
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

            // Calculate relative path
            let rel_path = path
                .strip_prefix(root)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;

            let path_str = rel_path
                .to_str()
                .ok_or_else(|| format!("Path is not valid UTF-8: {}", rel_path.display()))?
                .to_string();

            // Calculate MD5
            let md5 = calculate_md5(&content);

            entries.push(FileEntry {
                path: path_str,
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
    // Walk directory and collect file entries
    let entries = walk_directory(root, &[], &[], config.max_file_size)?;

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
        assert_eq!(version(), "0.2.0");
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
