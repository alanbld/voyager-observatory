//! Serialization module for pm_encoder
//!
//! This module provides output format serializers for different formats:
//! - Plus/Minus (default)
//! - XML
//! - Markdown
//! - Claude-XML (semantic with CDATA)

use crate::core::models::{CompressionLevel, OutputFormat, ProcessedFile};
use crate::core::zoom::ZoomAction;

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

        // Build header with optional [SKELETON] tag
        let header = if file.compression_level == CompressionLevel::Skeleton {
            if let Some(orig) = file.original_tokens {
                format!("+++ {} [SKELETON] (original: {} tokens)\n", file.path, orig)
            } else {
                format!("+++ {} [SKELETON]\n", file.path)
            }
        } else {
            format!("+++ {}\n", file.path)
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
            "<file path=\"{}\" md5=\"{}\" language=\"{}\"{}>\n",
            Self::escape_xml(&file.path),
            file.md5,
            file.language,
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

        // Build header with optional [SKELETON] tag
        let header = if file.compression_level == CompressionLevel::Skeleton {
            if let Some(orig) = file.original_tokens {
                format!("## {} [SKELETON] (original: {} tokens)\n\n", file.path, orig)
            } else {
                format!("## {} [SKELETON]\n\n", file.path)
            }
        } else {
            format!("## {}\n\n", file.path)
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
        assert_eq!(XmlSerializer::escape_xml("<>&\"'"), "&lt;&gt;&amp;&quot;&apos;");
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
}
