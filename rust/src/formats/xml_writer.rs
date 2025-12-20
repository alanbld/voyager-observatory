//! Streaming XML Writer for Claude-XML format
//!
//! This module provides a zero-copy, streaming XML writer that writes directly
//! to any `std::io::Write` implementation. Designed for O(1) memory overhead
//! relative to repository size.
//!
//! # WASM Compatibility
//! This module uses only `std::io::Write` trait, no filesystem operations,
//! making it compatible with WASM targets.

use std::collections::BTreeMap;
use std::io::{self, Write};

/// Error type for XML writing operations
#[derive(Debug)]
pub enum XmlError {
    Io(io::Error),
    InvalidState(String),
}

impl From<io::Error> for XmlError {
    fn from(e: io::Error) -> Self {
        XmlError::Io(e)
    }
}

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlError::Io(e) => write!(f, "IO error: {}", e),
            XmlError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl std::error::Error for XmlError {}

pub type Result<T> = std::result::Result<T, XmlError>;

/// Metadata for attention mapping in XML output
#[derive(Debug, Clone)]
pub struct AttentionEntry {
    pub path: String,
    pub priority: i32,
    pub tokens: usize,
    pub truncated: bool,
    pub dropped: bool,
    /// Learned utility score from Context Store (0.0-1.0)
    pub utility_score: Option<f64>,
}

/// Configuration for XML generation
#[derive(Debug, Clone)]
pub struct XmlConfig {
    pub package: String,
    pub version: String,
    pub lens: Option<String>,
    pub token_budget: Option<usize>,
    pub utilized_tokens: Option<usize>,
    pub frozen: bool,
    pub allow_sensitive: bool,
    pub snapshot_id: Option<String>,
}

impl Default for XmlConfig {
    fn default() -> Self {
        Self {
            package: "pm_encoder".to_string(),
            version: crate::VERSION.to_string(),
            lens: None,
            token_budget: None,
            utilized_tokens: None,
            frozen: false,
            allow_sensitive: false,
            snapshot_id: None,
        }
    }
}

/// Streaming XML writer with zero-copy I/O
///
/// Writes directly to the provided `Write` handle, maintaining O(1) memory
/// overhead regardless of repository size.
pub struct XmlWriter<W: Write> {
    writer: W,
    config: XmlConfig,
    in_files_section: bool,
}

impl<W: Write> XmlWriter<W> {
    /// Create a new XmlWriter with the given configuration
    pub fn new(writer: W, config: XmlConfig) -> Self {
        Self {
            writer,
            config,
            in_files_section: false,
        }
    }

    /// Write the opening <context> tag with attributes
    pub fn write_context_start(&mut self) -> Result<()> {
        // Use BTreeMap for deterministic attribute ordering
        let mut attrs: BTreeMap<String, String> = BTreeMap::new();

        attrs.insert("package".to_string(), self.config.package.clone());

        if let Some(ref lens) = self.config.lens {
            attrs.insert("lens".to_string(), lens.clone());
        }

        if let Some(budget) = self.config.token_budget {
            attrs.insert("token_budget".to_string(), budget.to_string());
        }

        if let Some(utilized) = self.config.utilized_tokens {
            attrs.insert("utilized".to_string(), utilized.to_string());
        }

        write!(self.writer, "<context")?;
        for (key, value) in &attrs {
            write!(self.writer, "\n  {}=\"{}\"", key, escape_xml_attr(&value))?;
        }
        writeln!(self.writer, ">")?;

        Ok(())
    }

    /// Write the metadata section
    pub fn write_metadata(&mut self, attention_entries: &[AttentionEntry]) -> Result<()> {
        writeln!(self.writer, "  <metadata>")?;
        writeln!(self.writer, "    <version>{}</version>", self.config.version)?;
        writeln!(self.writer, "    <frozen>{}</frozen>", self.config.frozen)?;

        // Timestamp only in non-frozen mode
        if !self.config.frozen {
            let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
            writeln!(self.writer, "    <timestamp>{}</timestamp>", timestamp)?;
        } else if let Some(ref snapshot_id) = self.config.snapshot_id {
            writeln!(self.writer, "    <snapshot_id>{}</snapshot_id>", snapshot_id)?;
        }

        // Attention map with priority tiers
        if !attention_entries.is_empty() {
            writeln!(self.writer, "    <attention_map>")?;

            // Group entries by priority tier for LLM attention priming
            let critical: Vec<_> = attention_entries.iter()
                .filter(|e| !e.dropped && (e.priority >= 95 || e.utility_score.unwrap_or(0.0) > 0.8))
                .collect();
            let high: Vec<_> = attention_entries.iter()
                .filter(|e| !e.dropped && e.priority >= 80 && e.priority < 95 && e.utility_score.unwrap_or(0.0) <= 0.8)
                .collect();
            let dropped: Vec<_> = attention_entries.iter()
                .filter(|e| e.dropped)
                .collect();

            // Critical tier (priority >= 95 or utility > 0.8)
            if !critical.is_empty() {
                writeln!(self.writer, "      <priority_tier level=\"critical\">")?;
                for entry in &critical {
                    self.write_attention_entry(entry, "hotspot")?;
                }
                writeln!(self.writer, "      </priority_tier>")?;
            }

            // High tier (priority 80-94)
            if !high.is_empty() {
                writeln!(self.writer, "      <priority_tier level=\"high\">")?;
                for entry in &high {
                    self.write_attention_entry(entry, "hotspot")?;
                }
                writeln!(self.writer, "      </priority_tier>")?;
            }

            // Coldspots (dropped files)
            if !dropped.is_empty() {
                writeln!(self.writer, "      <coldspots>")?;
                for entry in &dropped {
                    self.write_attention_entry(entry, "coldspot")?;
                }
                writeln!(self.writer, "      </coldspots>")?;
            }

            writeln!(self.writer, "    </attention_map>")?;
        }

        // Lens config
        if let Some(ref lens) = self.config.lens {
            writeln!(self.writer, "    <lens_config>")?;
            writeln!(self.writer, "      <name>{}</name>", lens)?;
            writeln!(self.writer, "    </lens_config>")?;
        }

        writeln!(self.writer, "  </metadata>")?;
        writeln!(self.writer)?;

        Ok(())
    }

    /// Write a single attention entry (hotspot or coldspot)
    fn write_attention_entry(&mut self, entry: &AttentionEntry, tag: &str) -> Result<()> {
        write!(self.writer, "        <{} path=\"{}\" priority=\"{}\" tokens=\"{}\"",
            tag, escape_xml_attr(&entry.path), entry.priority, entry.tokens)?;

        if entry.truncated {
            write!(self.writer, " truncated=\"true\"")?;
        }
        if entry.dropped {
            write!(self.writer, " dropped=\"true\"")?;
        }
        if let Some(utility) = entry.utility_score {
            write!(self.writer, " utility=\"{:.2}\"", utility)?;
        }

        writeln!(self.writer, " />")?;
        Ok(())
    }

    /// Start the files section
    pub fn write_files_start(&mut self) -> Result<()> {
        writeln!(self.writer, "  <files>")?;
        self.in_files_section = true;
        Ok(())
    }

    /// Write a single file entry with streaming content
    pub fn write_file(
        &mut self,
        path: &str,
        language: &str,
        md5: &str,
        priority: i32,
        content: &str,
        truncated: bool,
        original_tokens: Option<usize>,
        zoom_command: Option<&str>,
    ) -> Result<()> {
        if !self.in_files_section {
            return Err(XmlError::InvalidState(
                "Must call write_files_start before write_file".to_string()
            ));
        }

        // Sanitize path if not allowing sensitive data
        let display_path = if self.config.allow_sensitive {
            path.to_string()
        } else {
            sanitize_path(path)
        };

        // Use BTreeMap for deterministic attribute ordering (alphabetical)
        let mut attrs: BTreeMap<String, String> = BTreeMap::new();
        attrs.insert("language".to_string(), language.to_string());
        attrs.insert("md5".to_string(), md5.to_string());
        attrs.insert("path".to_string(), display_path);
        attrs.insert("priority".to_string(), priority.to_string());

        if truncated {
            attrs.insert("truncated".to_string(), "true".to_string());
            if let Some(orig) = original_tokens {
                attrs.insert("original_tokens".to_string(), orig.to_string());
            }
        }

        // Write file tag with sorted attributes
        write!(self.writer, "    <file")?;
        for (key, value) in &attrs {
            write!(self.writer, "\n      {}=\"{}\"", key, escape_xml_attr(&value))?;
        }
        writeln!(self.writer, ">")?;

        // Write CDATA content with proper escaping
        write!(self.writer, "      <![CDATA[")?;
        write!(self.writer, "{}", escape_cdata(content))?;
        writeln!(self.writer, "]]>")?;

        // Zoom affordances for truncated files
        if truncated {
            writeln!(self.writer, "      <zoom_actions>")?;

            // Primary expand action
            if let Some(cmd) = zoom_command {
                writeln!(self.writer, "        <action type=\"expand\" cmd=\"{}\" />",
                    escape_xml_attr(cmd))?;
            }

            // Structure-only view (always available for truncated files)
            writeln!(self.writer, "        <action type=\"structure\" cmd=\"pm_encoder --zoom file={} --depth signature\" />",
                escape_xml_attr(path))?;

            // Full file (no truncation) - use single quotes for shell arg
            writeln!(self.writer, "        <action type=\"full\" cmd=\"pm_encoder --truncate 0 --include '{}'\" />",
                escape_xml_attr(path))?;

            writeln!(self.writer, "      </zoom_actions>")?;
        }

        writeln!(self.writer, "    </file>")?;

        Ok(())
    }

    /// End the files section
    pub fn write_files_end(&mut self) -> Result<()> {
        if !self.in_files_section {
            return Err(XmlError::InvalidState(
                "write_files_end called without write_files_start".to_string()
            ));
        }
        writeln!(self.writer, "  </files>")?;
        self.in_files_section = false;
        Ok(())
    }

    /// Write the closing </context> tag
    pub fn write_context_end(&mut self) -> Result<()> {
        writeln!(self.writer, "</context>")?;
        Ok(())
    }

    /// Flush the underlying writer
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// Consume the writer and return the inner Write handle
    pub fn into_inner(self) -> W {
        self.writer
    }
}

/// Escape CDATA content by splitting ]]> sequences
///
/// The sequence `]]>` cannot appear inside CDATA, so we split it:
/// `]]>` becomes `]]]]><![CDATA[>`
///
/// This preserves the original content when parsed.
pub fn escape_cdata(content: &str) -> String {
    content.replace("]]>", "]]]]><![CDATA[>")
}

/// Escape XML attribute values
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Sanitize file paths for privacy (remove absolute path prefixes)
fn sanitize_path(path: &str) -> String {
    // Remove common absolute path prefixes
    if path.starts_with('/') {
        // Unix absolute path - extract relative portion
        if let Some(pos) = path.rfind("/src/") {
            return path[pos + 1..].to_string();
        }
        if let Some(pos) = path.rfind("/lib/") {
            return path[pos + 1..].to_string();
        }
        // Just use the filename if no recognizable structure
        if let Some(pos) = path.rfind('/') {
            return path[pos + 1..].to_string();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_cdata_single() {
        assert_eq!(escape_cdata("hello]]>world"), "hello]]]]><![CDATA[>world");
    }

    #[test]
    fn test_escape_cdata_multiple() {
        let input = "]]>nested]]>poison]]>";
        let escaped = escape_cdata(input);
        assert_eq!(escaped, "]]]]><![CDATA[>nested]]]]><![CDATA[>poison]]]]><![CDATA[>");
        assert!(!escaped.contains("]]>]")); // No raw ]]> followed by ]
    }

    #[test]
    fn test_escape_cdata_no_poison() {
        assert_eq!(escape_cdata("clean content"), "clean content");
    }

    #[test]
    fn test_escape_xml_attr() {
        assert_eq!(escape_xml_attr("a<b>c"), "a&lt;b&gt;c");
        assert_eq!(escape_xml_attr("a\"b'c"), "a&quot;b&apos;c");
        assert_eq!(escape_xml_attr("a&b"), "a&amp;b");
    }

    #[test]
    fn test_sanitize_path_absolute() {
        assert_eq!(sanitize_path("/home/user/project/src/main.rs"), "src/main.rs");
        assert_eq!(sanitize_path("/var/lib/data.json"), "lib/data.json");
        assert_eq!(sanitize_path("/root/file.txt"), "file.txt");
    }

    #[test]
    fn test_sanitize_path_relative() {
        assert_eq!(sanitize_path("src/main.rs"), "src/main.rs");
        assert_eq!(sanitize_path("file.txt"), "file.txt");
    }

    #[test]
    fn test_xml_writer_deterministic_attrs() {
        let mut output = Vec::new();
        let config = XmlConfig {
            package: "test".to_string(),
            lens: Some("arch".to_string()),
            token_budget: Some(1000),
            ..Default::default()
        };

        let mut writer = XmlWriter::new(&mut output, config);
        writer.write_context_start().unwrap();

        let xml = String::from_utf8(output).unwrap();
        // Attributes should be in alphabetical order
        let lens_pos = xml.find("lens=").unwrap();
        let package_pos = xml.find("package=").unwrap();
        let token_pos = xml.find("token_budget=").unwrap();

        assert!(lens_pos < package_pos, "lens should come before package");
        assert!(package_pos < token_pos, "package should come before token_budget");
    }

    #[test]
    fn test_xml_writer_frozen_no_timestamp() {
        let mut output = Vec::new();
        let config = XmlConfig {
            frozen: true,
            snapshot_id: Some("FROZEN_SNAPSHOT".to_string()),
            ..Default::default()
        };

        let mut writer = XmlWriter::new(&mut output, config);
        writer.write_context_start().unwrap();
        writer.write_metadata(&[]).unwrap();

        let xml = String::from_utf8(output).unwrap();
        assert!(!xml.contains("<timestamp>"), "Frozen mode should not have timestamp");
        assert!(xml.contains("<snapshot_id>FROZEN_SNAPSHOT</snapshot_id>"));
    }

    #[test]
    fn test_xml_writer_file_with_poison() {
        let mut output = Vec::new();
        let config = XmlConfig::default();

        let mut writer = XmlWriter::new(&mut output, config);
        writer.write_context_start().unwrap();
        writer.write_metadata(&[]).unwrap();
        writer.write_files_start().unwrap();
        writer.write_file(
            "test.rs",
            "rust",
            "abc123",
            100,
            "let x = arr[arr.len() - 1]]>;",
            false,
            None,
            None,
        ).unwrap();
        writer.write_files_end().unwrap();
        writer.write_context_end().unwrap();

        let xml = String::from_utf8(output).unwrap();
        assert!(xml.contains("]]]]><![CDATA[>"), "CDATA poison should be escaped");
        assert!(!xml.contains("]]>;"), "Raw poison should not appear");
    }

    #[test]
    fn test_xml_writer_zoom_affordance() {
        let mut output = Vec::new();
        let config = XmlConfig::default();

        let mut writer = XmlWriter::new(&mut output, config);
        writer.write_context_start().unwrap();
        writer.write_metadata(&[]).unwrap();
        writer.write_files_start().unwrap();
        writer.write_file(
            "large.rs",
            "rust",
            "def456",
            95,
            "// truncated content",
            true,
            Some(5000),
            Some("--include large.rs --truncate 0"),
        ).unwrap();
        writer.write_files_end().unwrap();
        writer.write_context_end().unwrap();

        let xml = String::from_utf8(output).unwrap();
        assert!(xml.contains("<zoom_actions>"));
        assert!(xml.contains("type=\"expand\""));
        assert!(xml.contains("--include large.rs --truncate 0"));
        assert!(xml.contains("truncated=\"true\""));
        assert!(xml.contains("original_tokens=\"5000\""));
    }
}
