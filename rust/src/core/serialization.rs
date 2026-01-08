//! Serialization module for pm_encoder
//!
//! This module provides output format serializers for different formats:
//! - Plus/Minus (default)
//! - XML
//! - Markdown
//! - Claude-XML (semantic with CDATA)

use crate::core::models::{CompressionLevel, MetadataMode, OutputFormat, ProcessedFile};
use crate::core::zoom::ZoomAction;
use chrono::{TimeZone, Utc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Metadata Formatting (Chronos v2.3)
// ============================================================================

/// Format bytes to human readable (B, K, M, G)
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{}B", bytes)
    } else {
        format!("{:.1}{}", value, UNITS[unit_index])
    }
}

/// Format timestamp for 'All' mode (full precision, UTC)
pub fn format_timestamp_full(mtime: u64) -> String {
    if mtime == 0 {
        return "Unknown".to_string();
    }
    let datetime = Utc.timestamp_opt(mtime as i64, 0);
    match datetime {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M UTC").to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Format timestamp for 'Auto' mode (compact relative)
pub fn format_timestamp_compact(mtime: u64) -> String {
    if mtime == 0 {
        return "?".to_string();
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();

    if mtime > now {
        return "future".to_string();
    }

    let age_secs = now - mtime;

    if age_secs < 60 {
        format!("{}s", age_secs)
    } else if age_secs < 3600 {
        format!("{}m", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h", age_secs / 3600)
    } else if age_secs < 30 * 86400 {
        format!("{}d", age_secs / 86400)
    } else {
        // For older files: show year-month
        let datetime = Utc.timestamp_opt(mtime as i64, 0);
        match datetime {
            chrono::LocalResult::Single(dt) => dt.format("%Y-%m").to_string(),
            _ => "old".to_string(),
        }
    }
}

/// Main header metadata formatting with mode logic
pub fn format_metadata_suffix(size: u64, mtime: u64, mode: MetadataMode) -> String {
    match mode {
        MetadataMode::None => String::new(),

        MetadataMode::All => {
            let time_str = format_timestamp_full(mtime);
            format!(" [S:{} M:{}]", human_bytes(size), time_str)
        }

        MetadataMode::SizeOnly => {
            format!(" [S:{}]", human_bytes(size))
        }

        MetadataMode::Auto => {
            let mut parts = Vec::new();

            // Show size if > 10KB
            if size > 10_000 {
                parts.push(format!("S:{}", human_bytes(size)));
            }

            // Show time if recent (<30d) OR ancient (>5y)
            if mtime > 0 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                if mtime <= now {
                    let age_days = (now - mtime) / 86400;

                    if age_days < 30 || age_days > 5 * 365 {
                        parts.push(format!("M:{}", format_timestamp_compact(mtime)));
                    }
                }
            }

            if parts.is_empty() {
                String::new()
            } else {
                format!(" [{}]", parts.join(" "))
            }
        }
    }
}

/// Format a Plus/Minus header line with optional metadata
pub fn format_plusminus_header(path: &str, size: u64, mtime: u64, mode: MetadataMode) -> String {
    let metadata = format_metadata_suffix(size, mtime, mode);
    format!("+++ {}{}\n", path, metadata)
}

/// Format an XML file element opening tag with optional metadata attributes
pub fn format_xml_header_attrs(size: u64, mtime: u64, mode: MetadataMode) -> String {
    match mode {
        MetadataMode::None => String::new(),
        MetadataMode::All => {
            format!(
                " size=\"{}\" mtime=\"{}\" mtime_human=\"{}\"",
                size,
                mtime,
                format_timestamp_full(mtime)
            )
        }
        MetadataMode::SizeOnly => {
            format!(" size=\"{}\"", size)
        }
        MetadataMode::Auto => {
            let mut attrs = Vec::new();

            // Show size if > 10KB
            if size > 10_000 {
                attrs.push(format!("size=\"{}\"", size));
            }

            // Show time if recent (<30d) OR ancient (>5y)
            if mtime > 0 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                if mtime <= now {
                    let age_days = (now - mtime) / 86400;
                    if age_days < 30 || age_days > 5 * 365 {
                        attrs.push(format!("mtime=\"{}\"", mtime));
                    }
                }
            }

            if attrs.is_empty() {
                String::new()
            } else {
                format!(" {}", attrs.join(" "))
            }
        }
    }
}

/// Format a Markdown header with optional metadata
pub fn format_markdown_header(path: &str, size: u64, mtime: u64, mode: MetadataMode) -> String {
    let metadata = format_metadata_suffix(size, mtime, mode);
    format!("## {}{}\n\n", path, metadata)
}

/// Trait for output format serializers
pub trait Serializer: Send + Sync {
    /// Serialize a single file entry
    fn serialize_file(&self, file: &ProcessedFile) -> String;

    /// Serialize multiple files with header/footer
    fn serialize_files(&self, files: &[ProcessedFile]) -> String {
        files.iter().map(|f| self.serialize_file(f)).collect()
    }

    /// Get the file extension for this format
    fn extension(&self) -> &'static str;
}

/// Plus/Minus format serializer (default)
pub struct PlusMinusSerializer;

impl PlusMinusSerializer {
    /// Create a new PlusMinusSerializer
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlusMinusSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for PlusMinusSerializer {
    fn serialize_file(&self, file: &ProcessedFile) -> String {
        let mut output = String::new();

        // Get brightness indicator from journal utility
        let brightness = file.brightness_indicator();

        // Build header with optional brightness indicator and [SKELETON] tag
        let header = if file.compression_level == CompressionLevel::Skeleton {
            if let Some(orig) = file.original_tokens {
                format!(
                    "+++ {}{} [SKELETON] (original: {} tokens)\n",
                    brightness, file.path, orig
                )
            } else {
                format!("+++ {}{} [SKELETON]\n", brightness, file.path)
            }
        } else {
            format!("+++ {}{}\n", brightness, file.path)
        };

        output.push_str(&header);
        for line in file.content.lines() {
            output.push_str(&format!("+ {}\n", line));
        }
        output.push_str(&format!("--- {} [md5:{}]\n\n", file.path, file.md5));
        output
    }

    fn extension(&self) -> &'static str {
        "txt"
    }
}

/// XML format serializer
pub struct XmlSerializer;

impl XmlSerializer {
    /// Create a new XmlSerializer
    pub fn new() -> Self {
        Self
    }

    /// Escape XML special characters
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

impl Default for XmlSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for XmlSerializer {
    fn serialize_file(&self, file: &ProcessedFile) -> String {
        let mut output = String::new();

        // Build brightness attribute from journal utility
        let brightness_attr = match file.utility {
            Some(u) => format!(" utility=\"{:.2}\" bright=\"{}\"", u, file.is_bright_star()),
            None => String::new(),
        };

        // Build file element with skeleton attributes if applicable
        let skeleton_attr = if file.compression_level == CompressionLevel::Skeleton {
            if let Some(orig) = file.original_tokens {
                format!(" skeleton=\"true\" original_tokens=\"{}\"", orig)
            } else {
                " skeleton=\"true\"".to_string()
            }
        } else {
            String::new()
        };

        output.push_str(&format!(
            "<file path=\"{}\" md5=\"{}\" language=\"{}\"{}{}>\n",
            Self::escape_xml(&file.path),
            file.md5,
            file.language,
            brightness_attr,
            skeleton_attr
        ));
        output.push_str(&Self::escape_xml(&file.content));
        output.push_str("\n</file>\n");
        output
    }

    fn serialize_files(&self, files: &[ProcessedFile]) -> String {
        let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<context>\n");
        for file in files {
            output.push_str(&self.serialize_file(file));
        }
        output.push_str("</context>\n");
        output
    }

    fn extension(&self) -> &'static str {
        "xml"
    }
}

/// Markdown format serializer
pub struct MarkdownSerializer;

impl MarkdownSerializer {
    /// Create a new MarkdownSerializer
    pub fn new() -> Self {
        Self
    }

    /// Detect language for code block
    fn detect_language(path: &str) -> &'static str {
        let ext = path.rsplit('.').next().unwrap_or("");
        match ext.to_lowercase().as_str() {
            "py" => "python",
            "rs" => "rust",
            "js" => "javascript",
            "ts" => "typescript",
            "jsx" => "jsx",
            "tsx" => "tsx",
            "sh" | "bash" => "bash",
            "md" => "markdown",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "html" => "html",
            "css" => "css",
            "sql" => "sql",
            "go" => "go",
            "java" => "java",
            "c" => "c",
            "cpp" | "cc" | "cxx" => "cpp",
            "h" | "hpp" => "cpp",
            "rb" => "ruby",
            "php" => "php",
            _ => "",
        }
    }
}

impl Default for MarkdownSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for MarkdownSerializer {
    fn serialize_file(&self, file: &ProcessedFile) -> String {
        let lang = Self::detect_language(&file.path);
        let mut output = String::new();

        // Get brightness indicator from journal utility
        let brightness = file.brightness_indicator();

        // Build header with optional brightness and [SKELETON] tag
        let header = if file.compression_level == CompressionLevel::Skeleton {
            if let Some(orig) = file.original_tokens {
                format!(
                    "## {}{} [SKELETON] (original: {} tokens)\n\n",
                    brightness, file.path, orig
                )
            } else {
                format!("## {}{} [SKELETON]\n\n", brightness, file.path)
            }
        } else {
            format!("## {}{}\n\n", brightness, file.path)
        };

        output.push_str(&header);
        output.push_str(&format!("```{}\n", lang));
        output.push_str(&file.content);
        if !file.content.ends_with('\n') {
            output.push('\n');
        }
        output.push_str("```\n\n");
        output
    }

    fn extension(&self) -> &'static str {
        "md"
    }
}

/// Get the appropriate serializer for an output format
pub fn get_serializer(format: OutputFormat) -> Box<dyn Serializer> {
    match format {
        OutputFormat::PlusMinus => Box::new(PlusMinusSerializer::new()),
        OutputFormat::Xml => Box::new(XmlSerializer::new()),
        OutputFormat::Markdown => Box::new(MarkdownSerializer::new()),
        OutputFormat::ClaudeXml => Box::new(PlusMinusSerializer::new()), // Use XmlWriter instead
    }
}

/// Generate a truncation marker with zoom affordance
pub fn truncation_marker(
    original_lines: usize,
    kept_lines: usize,
    zoom_action: Option<&ZoomAction>,
) -> String {
    let mut marker = String::new();
    marker.push_str(&format!(
        "/* TRUNCATED: {} lines → {} lines */\n",
        original_lines, kept_lines
    ));
    if let Some(action) = zoom_action {
        marker.push_str(&action.to_affordance_comment());
        marker.push('\n');
    }
    marker
}

/// Generate a gap marker for smart truncation
pub fn gap_marker(start_line: usize, end_line: usize, context: &str) -> String {
    format!(
        "\n/* ... {} lines omitted ({}) [lines {}-{}] ... */\n",
        end_line - start_line,
        context,
        start_line,
        end_line
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::FileEntry;

    fn sample_file() -> ProcessedFile {
        let entry = FileEntry::new("src/main.rs", "fn main() {\n    println!(\"Hello\");\n}");
        ProcessedFile::from_entry(&entry, "rust", 100)
    }

    #[test]
    fn test_plus_minus_serializer() {
        let serializer = PlusMinusSerializer::new();
        let file = sample_file();
        let output = serializer.serialize_file(&file);

        assert!(output.starts_with("+++ src/main.rs"));
        assert!(output.contains("+ fn main()"));
        assert!(output.contains("--- src/main.rs"));
    }

    #[test]
    fn test_xml_serializer() {
        let serializer = XmlSerializer::new();
        let file = sample_file();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("<file path=\"src/main.rs\""));
        assert!(output.contains("language=\"rust\""));
        assert!(output.contains("</file>"));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(
            XmlSerializer::escape_xml("<>&\"'"),
            "&lt;&gt;&amp;&quot;&apos;"
        );
    }

    #[test]
    fn test_markdown_serializer() {
        let serializer = MarkdownSerializer::new();
        let file = sample_file();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("## src/main.rs"));
        assert!(output.contains("```rust"));
        assert!(output.contains("fn main()"));
        assert!(output.ends_with("```\n\n"));
    }

    #[test]
    fn test_markdown_detect_language() {
        assert_eq!(MarkdownSerializer::detect_language("test.py"), "python");
        assert_eq!(MarkdownSerializer::detect_language("test.rs"), "rust");
        assert_eq!(MarkdownSerializer::detect_language("test.unknown"), "");
    }

    #[test]
    fn test_truncation_marker_without_zoom() {
        let marker = truncation_marker(100, 50, None);
        assert!(marker.contains("100 lines → 50 lines"));
        assert!(!marker.contains("ZOOM_AFFORDANCE"));
    }

    #[test]
    fn test_truncation_marker_with_zoom() {
        let action = ZoomAction::for_function("main", 1000);
        let marker = truncation_marker(100, 50, Some(&action));
        assert!(marker.contains("ZOOM_AFFORDANCE"));
        assert!(marker.contains("function=main"));
    }

    #[test]
    fn test_gap_marker() {
        let marker = gap_marker(10, 50, "implementation details");
        assert!(marker.contains("40 lines omitted"));
        assert!(marker.contains("lines 10-50"));
    }

    #[test]
    fn test_get_serializer() {
        let pm = get_serializer(OutputFormat::PlusMinus);
        assert_eq!(pm.extension(), "txt");

        let xml = get_serializer(OutputFormat::Xml);
        assert_eq!(xml.extension(), "xml");

        let md = get_serializer(OutputFormat::Markdown);
        assert_eq!(md.extension(), "md");
    }

    // ========================================================================
    // Chronos v2.3 Metadata Formatting Tests
    // ========================================================================

    #[test]
    fn test_human_bytes_formatting() {
        assert_eq!(human_bytes(0), "0B");
        assert_eq!(human_bytes(500), "500B");
        assert_eq!(human_bytes(1024), "1.0K");
        assert_eq!(human_bytes(15_000), "14.6K");
        assert_eq!(human_bytes(1_500_000), "1.4M");
        assert_eq!(human_bytes(3_000_000_000), "2.8G");
        assert_eq!(human_bytes(1_099_511_627_776), "1.0T");
    }

    #[test]
    fn test_format_timestamp_full() {
        // Test known timestamp: 2024-01-15 12:00:00 UTC
        let mtime = 1705320000_u64;
        let result = format_timestamp_full(mtime);
        assert!(result.contains("2024-01-15"));
        assert!(result.contains("UTC"));
    }

    #[test]
    fn test_format_timestamp_full_zero() {
        assert_eq!(format_timestamp_full(0), "Unknown");
    }

    #[test]
    fn test_format_timestamp_compact_recent() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 30 seconds ago
        let result = format_timestamp_compact(now - 30);
        assert!(result.ends_with("s"));

        // 10 minutes ago
        let result = format_timestamp_compact(now - 600);
        assert!(result.ends_with("m"));

        // 5 hours ago
        let result = format_timestamp_compact(now - 18000);
        assert!(result.ends_with("h"));

        // 10 days ago
        let result = format_timestamp_compact(now - 864000);
        assert!(result.ends_with("d"));
    }

    #[test]
    fn test_format_timestamp_compact_old() {
        // 2020-01-15 - should show YYYY-MM
        let old_mtime = 1579046400_u64;
        let result = format_timestamp_compact(old_mtime);
        assert!(result.contains("2020"));
    }

    #[test]
    fn test_format_timestamp_compact_zero() {
        assert_eq!(format_timestamp_compact(0), "?");
    }

    #[test]
    fn test_metadata_mode_none() {
        let result = format_metadata_suffix(150_000, 1705320000, MetadataMode::None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_metadata_mode_all() {
        let result = format_metadata_suffix(150_000, 1705320000, MetadataMode::All);
        assert!(result.contains("S:"));
        assert!(result.contains("M:"));
        assert!(result.contains("UTC"));
    }

    #[test]
    fn test_metadata_mode_size_only() {
        let result = format_metadata_suffix(150_000, 1705320000, MetadataMode::SizeOnly);
        assert!(result.contains("S:"));
        assert!(!result.contains("M:"));
        assert!(!result.contains("UTC"));
    }

    #[test]
    fn test_metadata_mode_auto_large_recent() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Large (>10KB) and recent (<30 days) - should show both
        let result = format_metadata_suffix(150_000, now - 86400, MetadataMode::Auto);
        assert!(result.contains("S:"));
        assert!(result.contains("M:"));
    }

    #[test]
    fn test_metadata_mode_auto_small_old() {
        // Small (<10KB) and old (>30d, <5y) - should show nothing
        let old_mtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (365 * 86400); // 1 year ago

        let result = format_metadata_suffix(5_000, old_mtime, MetadataMode::Auto);
        assert!(result.is_empty());
    }

    #[test]
    fn test_metadata_mode_auto_ancient() {
        // Small but ancient (>5 years) - should show time only
        let ancient_mtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (6 * 365 * 86400); // 6 years ago

        let result = format_metadata_suffix(5_000, ancient_mtime, MetadataMode::Auto);
        assert!(!result.contains("S:")); // Small, so no size
        assert!(result.contains("M:")); // Ancient, so show time
    }

    #[test]
    fn test_format_plusminus_header() {
        let header = format_plusminus_header("src/main.rs", 15_000, 1705320000, MetadataMode::All);
        assert!(header.starts_with("+++ src/main.rs"));
        assert!(header.contains("[S:"));
        assert!(header.contains("M:"));
    }

    #[test]
    fn test_format_plusminus_header_none_mode() {
        let header = format_plusminus_header("src/main.rs", 15_000, 1705320000, MetadataMode::None);
        assert_eq!(header, "+++ src/main.rs\n");
    }

    #[test]
    fn test_format_xml_header_attrs_all() {
        let attrs = format_xml_header_attrs(15_000, 1705320000, MetadataMode::All);
        assert!(attrs.contains("size=\"15000\""));
        assert!(attrs.contains("mtime=\"1705320000\""));
        assert!(attrs.contains("mtime_human="));
    }

    #[test]
    fn test_format_xml_header_attrs_none() {
        let attrs = format_xml_header_attrs(15_000, 1705320000, MetadataMode::None);
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_format_xml_header_attrs_size_only() {
        let attrs = format_xml_header_attrs(15_000, 1705320000, MetadataMode::SizeOnly);
        assert!(attrs.contains("size=\"15000\""));
        assert!(!attrs.contains("mtime="));
    }

    #[test]
    fn test_format_markdown_header() {
        let header = format_markdown_header("README.md", 50_000, 1705320000, MetadataMode::All);
        assert!(header.starts_with("## README.md"));
        assert!(header.contains("[S:"));
    }

    #[test]
    fn test_metadata_mode_parse() {
        assert_eq!(MetadataMode::parse("auto"), Some(MetadataMode::Auto));
        assert_eq!(MetadataMode::parse("AUTO"), Some(MetadataMode::Auto));
        assert_eq!(MetadataMode::parse("all"), Some(MetadataMode::All));
        assert_eq!(MetadataMode::parse("none"), Some(MetadataMode::None));
        assert_eq!(
            MetadataMode::parse("size-only"),
            Some(MetadataMode::SizeOnly)
        );
        assert_eq!(
            MetadataMode::parse("size_only"),
            Some(MetadataMode::SizeOnly)
        );
        assert_eq!(MetadataMode::parse("invalid"), None);
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_format_timestamp_compact_future() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Future timestamp
        let result = format_timestamp_compact(now + 86400);
        assert_eq!(result, "future");
    }

    #[test]
    fn test_xml_serialize_files_wrapper() {
        let serializer = XmlSerializer::new();
        let file = sample_file();
        let output = serializer.serialize_files(&[file]);

        assert!(output.starts_with("<?xml version=\"1.0\""));
        assert!(output.contains("<context>"));
        assert!(output.contains("</context>"));
        assert!(output.contains("<file path=\"src/main.rs\""));
    }

    #[test]
    fn test_plusminus_serialize_files_trait() {
        let serializer = PlusMinusSerializer::new();
        let file = sample_file();
        let output = serializer.serialize_files(&[file.clone(), file]);

        // Should have two file entries
        assert_eq!(output.matches("+++ src/main.rs").count(), 2);
    }

    #[test]
    fn test_markdown_serialize_files_trait() {
        let serializer = MarkdownSerializer::new();
        let file = sample_file();
        let output = serializer.serialize_files(&[file]);

        assert!(output.contains("## src/main.rs"));
    }

    #[test]
    fn test_serializer_default_constructors() {
        let pm = PlusMinusSerializer::default();
        assert_eq!(pm.extension(), "txt");

        let xml = XmlSerializer::default();
        assert_eq!(xml.extension(), "xml");

        let md = MarkdownSerializer::default();
        assert_eq!(md.extension(), "md");
    }

    #[test]
    fn test_skeleton_mode_plusminus() {
        let mut file = sample_file();
        file.compression_level = CompressionLevel::Skeleton;
        file.original_tokens = Some(500);

        let serializer = PlusMinusSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("[SKELETON]"));
        assert!(output.contains("original: 500 tokens"));
    }

    #[test]
    fn test_skeleton_mode_plusminus_no_tokens() {
        let mut file = sample_file();
        file.compression_level = CompressionLevel::Skeleton;
        file.original_tokens = None;

        let serializer = PlusMinusSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("[SKELETON]"));
        assert!(!output.contains("original:"));
    }

    #[test]
    fn test_skeleton_mode_xml() {
        let mut file = sample_file();
        file.compression_level = CompressionLevel::Skeleton;
        file.original_tokens = Some(500);

        let serializer = XmlSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("skeleton=\"true\""));
        assert!(output.contains("original_tokens=\"500\""));
    }

    #[test]
    fn test_skeleton_mode_xml_no_tokens() {
        let mut file = sample_file();
        file.compression_level = CompressionLevel::Skeleton;
        file.original_tokens = None;

        let serializer = XmlSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("skeleton=\"true\""));
        assert!(!output.contains("original_tokens"));
    }

    #[test]
    fn test_skeleton_mode_markdown() {
        let mut file = sample_file();
        file.compression_level = CompressionLevel::Skeleton;
        file.original_tokens = Some(500);

        let serializer = MarkdownSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("[SKELETON]"));
        assert!(output.contains("original: 500 tokens"));
    }

    #[test]
    fn test_skeleton_mode_markdown_no_tokens() {
        let mut file = sample_file();
        file.compression_level = CompressionLevel::Skeleton;
        file.original_tokens = None;

        let serializer = MarkdownSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("[SKELETON]"));
        assert!(!output.contains("original:"));
    }

    #[test]
    fn test_xml_utility_brightness() {
        let mut file = sample_file();
        file.utility = Some(0.9);

        let serializer = XmlSerializer::new();
        let output = serializer.serialize_file(&file);

        assert!(output.contains("utility=\"0.90\""));
        assert!(output.contains("bright=\"true\""));
    }

    #[test]
    fn test_markdown_language_detection_all() {
        assert_eq!(MarkdownSerializer::detect_language("test.jsx"), "jsx");
        assert_eq!(MarkdownSerializer::detect_language("test.tsx"), "tsx");
        assert_eq!(MarkdownSerializer::detect_language("test.sh"), "bash");
        assert_eq!(MarkdownSerializer::detect_language("test.bash"), "bash");
        assert_eq!(MarkdownSerializer::detect_language("test.json"), "json");
        assert_eq!(MarkdownSerializer::detect_language("test.yaml"), "yaml");
        assert_eq!(MarkdownSerializer::detect_language("test.yml"), "yaml");
        assert_eq!(MarkdownSerializer::detect_language("test.toml"), "toml");
        assert_eq!(MarkdownSerializer::detect_language("test.html"), "html");
        assert_eq!(MarkdownSerializer::detect_language("test.css"), "css");
        assert_eq!(MarkdownSerializer::detect_language("test.sql"), "sql");
        assert_eq!(MarkdownSerializer::detect_language("test.go"), "go");
        assert_eq!(MarkdownSerializer::detect_language("test.java"), "java");
        assert_eq!(MarkdownSerializer::detect_language("test.c"), "c");
        assert_eq!(MarkdownSerializer::detect_language("test.cpp"), "cpp");
        assert_eq!(MarkdownSerializer::detect_language("test.cc"), "cpp");
        assert_eq!(MarkdownSerializer::detect_language("test.cxx"), "cpp");
        assert_eq!(MarkdownSerializer::detect_language("test.h"), "cpp");
        assert_eq!(MarkdownSerializer::detect_language("test.hpp"), "cpp");
        assert_eq!(MarkdownSerializer::detect_language("test.rb"), "ruby");
        assert_eq!(MarkdownSerializer::detect_language("test.php"), "php");
    }

    #[test]
    fn test_get_serializer_claude_xml() {
        let serializer = get_serializer(OutputFormat::ClaudeXml);
        // ClaudeXml falls back to PlusMinus (uses XmlWriter instead)
        assert_eq!(serializer.extension(), "txt");
    }

    #[test]
    fn test_human_bytes_terabytes() {
        // Test the T (terabyte) case
        let bytes = 2_199_023_255_552_u64; // ~2TB
        let result = human_bytes(bytes);
        assert!(result.contains("T"));
    }

    #[test]
    fn test_format_xml_header_attrs_auto_large_recent() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Large and recent
        let attrs = format_xml_header_attrs(50_000, now - 86400, MetadataMode::Auto);
        assert!(attrs.contains("size=\"50000\""));
        assert!(attrs.contains("mtime="));
    }

    #[test]
    fn test_format_xml_header_attrs_auto_small_old() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Small and old (not recent, not ancient)
        let attrs = format_xml_header_attrs(5_000, now - (365 * 86400), MetadataMode::Auto);
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_markdown_content_no_trailing_newline() {
        let entry = FileEntry::new("test.rs", "fn main() {}");
        let file = ProcessedFile::from_entry(&entry, "rust", 100);

        let serializer = MarkdownSerializer::new();
        let output = serializer.serialize_file(&file);

        // Should add newline if content doesn't end with one
        assert!(output.contains("fn main() {}\n```"));
    }
}
