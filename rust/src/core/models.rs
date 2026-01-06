//! Core data models for pm_encoder
//!
//! This module contains the fundamental data structures used throughout the encoder.

use serde::{Deserialize, Serialize};
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
    /// Modification time (seconds since epoch)
    pub mtime: u64,
    /// Creation time (seconds since epoch, falls back to mtime on some systems)
    pub ctime: u64,
    /// File size in bytes
    pub size: u64,
}

impl FileEntry {
    /// Create a new FileEntry
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let size = content.len() as u64;
        let md5 = calculate_md5(&content);
        Self {
            path: path.into(),
            content,
            md5,
            mtime: 0,
            ctime: 0,
            size,
        }
    }

    /// Create a FileEntry with timestamps
    pub fn with_timestamps(mut self, mtime: u64, ctime: u64) -> Self {
        self.mtime = mtime;
        self.ctime = ctime;
        self
    }

    /// Create a FileEntry with size (overrides content-based size)
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.path).extension().and_then(|e| e.to_str())
    }

    /// Estimate token count (~4 chars per token)
    pub fn token_estimate(&self) -> usize {
        self.content.len() / 4
    }
}

/// Configuration loaded from .pm_encoder_config.json
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Patterns to ignore (globs)
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Patterns to include (globs)
    #[serde(default)]
    pub include: Vec<String>,
    /// Maximum file size in bytes
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
}

fn default_max_file_size() -> u64 {
    1_048_576 // 1MB
}

/// Output format for serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plus/Minus format (default)
    #[default]
    PlusMinus,
    /// XML format
    Xml,
    /// Markdown format
    Markdown,
    /// Claude-optimized XML with CDATA and semantic metadata
    ClaudeXml,
}

impl OutputFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::PlusMinus => "txt",
            OutputFormat::Xml => "xml",
            OutputFormat::Markdown => "md",
            OutputFormat::ClaudeXml => "xml",
        }
    }

    /// Parse format from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "plus-minus" | "pm" | "plus_minus" => Some(OutputFormat::PlusMinus),
            "xml" => Some(OutputFormat::Xml),
            "markdown" | "md" => Some(OutputFormat::Markdown),
            "claude-xml" | "claude_xml" => Some(OutputFormat::ClaudeXml),
            _ => None,
        }
    }
}

/// Runtime configuration for the encoder
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Patterns to include
    pub include_patterns: Vec<String>,
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Maximum lines before truncation (0 = no limit)
    pub truncate_lines: usize,
    /// Truncation mode: "simple", "smart", or "structure"
    pub truncate_mode: String,
    /// Sort field: "name", "mtime", or "ctime"
    pub sort_by: String,
    /// Sort order: "asc" or "desc"
    pub sort_order: String,
    /// Enable streaming mode
    pub stream: bool,
    /// Include summary in truncation markers
    pub truncate_summary: bool,
    /// Patterns to exclude from truncation
    pub truncate_exclude: Vec<String>,
    /// Show truncation statistics
    pub truncate_stats: bool,
    /// Output format
    pub output_format: OutputFormat,
    /// Frozen mode for deterministic output
    pub frozen: bool,
    /// Allow sensitive metadata in output
    pub allow_sensitive: bool,
    /// Active lens name
    pub active_lens: Option<String>,
    /// Token budget
    pub token_budget: Option<usize>,
    /// Enable skeleton mode ("auto", "true", "false")
    /// - "auto": Enable if token_budget is set
    /// - "true": Always enable
    /// - "false": Always disable
    pub skeleton_mode: SkeletonMode,
    /// Metadata display mode for file headers (Chronos v2.3)
    pub metadata_mode: MetadataMode,
    /// Follow symbolic links (default: false, skip broken symlinks silently)
    pub follow_symlinks: bool,
}

/// Skeleton mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkeletonMode {
    /// Enable skeleton compression if token_budget is set
    #[default]
    Auto,
    /// Always enable skeleton compression
    Enabled,
    /// Always disable skeleton compression
    Disabled,
}

/// Metadata display mode for file headers (Chronos v2.3)
///
/// Controls whether and how file metadata (size, modification time) appears
/// in serialized output headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MetadataMode {
    /// Smart logic: show if file >10KB OR modified <30d OR modified >5y
    #[default]
    Auto,
    /// Digital archaeology: always show full metadata (size + timestamp UTC)
    All,
    /// Testing/diffing: no metadata for deterministic output
    None,
    /// Bundle analysis: always show size, never show time
    SizeOnly,
}

impl MetadataMode {
    /// Parse metadata mode from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(MetadataMode::Auto),
            "all" => Some(MetadataMode::All),
            "none" => Some(MetadataMode::None),
            "size-only" | "size_only" | "sizeonly" => Some(MetadataMode::SizeOnly),
            _ => None,
        }
    }
}

impl SkeletonMode {
    /// Parse skeleton mode from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(SkeletonMode::Auto),
            "true" | "enabled" | "on" | "yes" => Some(SkeletonMode::Enabled),
            "false" | "disabled" | "off" | "no" => Some(SkeletonMode::Disabled),
            _ => None,
        }
    }

    /// Check if skeleton should be enabled given a token budget
    pub fn is_enabled(&self, has_budget: bool) -> bool {
        match self {
            SkeletonMode::Auto => has_budget,
            SkeletonMode::Enabled => true,
            SkeletonMode::Disabled => false,
        }
    }
}

impl Default for EncoderConfig {
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
            truncate_lines: 0,
            truncate_mode: "simple".to_string(),
            sort_by: "name".to_string(),
            sort_order: "asc".to_string(),
            stream: false,
            truncate_summary: true,
            truncate_exclude: vec![],
            truncate_stats: false,
            output_format: OutputFormat::PlusMinus,
            frozen: false,
            allow_sensitive: false,
            active_lens: None,
            token_budget: None,
            skeleton_mode: SkeletonMode::Auto,
            metadata_mode: MetadataMode::Auto,
            follow_symlinks: false,
        }
    }
}

impl EncoderConfig {
    /// Create a new EncoderConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder pattern: set truncation
    pub fn with_truncation(mut self, lines: usize, mode: &str) -> Self {
        self.truncate_lines = lines;
        self.truncate_mode = mode.to_string();
        self
    }

    /// Builder pattern: set output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Builder pattern: set frozen mode
    pub fn with_frozen(mut self, frozen: bool) -> Self {
        self.frozen = frozen;
        self
    }

    /// Builder pattern: set token budget
    pub fn with_budget(mut self, budget: usize) -> Self {
        self.token_budget = Some(budget);
        self
    }

    /// Builder pattern: set lens
    pub fn with_lens(mut self, lens: &str) -> Self {
        self.active_lens = Some(lens.to_string());
        self
    }

    /// Builder pattern: set skeleton mode
    pub fn with_skeleton_mode(mut self, mode: SkeletonMode) -> Self {
        self.skeleton_mode = mode;
        self
    }

    /// Builder pattern: set metadata mode (Chronos v2.3)
    pub fn with_metadata_mode(mut self, mode: MetadataMode) -> Self {
        self.metadata_mode = mode;
        self
    }
}

/// Compression level for skeleton protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    /// Full content preserved
    #[default]
    Full,
    /// Skeleton: signatures only
    Skeleton,
    /// File dropped from output
    Drop,
}

/// A processed file ready for serialization
#[derive(Debug, Clone)]
pub struct ProcessedFile {
    /// File path
    pub path: String,
    /// File content (possibly truncated or skeletonized)
    pub content: String,
    /// MD5 checksum of original content
    pub md5: String,
    /// Detected language
    pub language: String,
    /// Priority (from lens)
    pub priority: i32,
    /// Token count estimate
    pub tokens: usize,
    /// Whether the file was truncated
    pub truncated: bool,
    /// Original token count (if truncated or skeletonized)
    pub original_tokens: Option<usize>,
    /// Compression level (Full, Skeleton, Drop)
    pub compression_level: CompressionLevel,
    /// Utility score from Observer's Journal (0.0-1.0)
    /// Stars with utility >= 0.8 are "bright" and display ⭐
    pub utility: Option<f64>,
}

impl ProcessedFile {
    /// Create from a FileEntry
    pub fn from_entry(entry: &FileEntry, language: &str, priority: i32) -> Self {
        Self {
            path: entry.path.clone(),
            content: entry.content.clone(),
            md5: entry.md5.clone(),
            language: language.to_string(),
            priority,
            tokens: entry.token_estimate(),
            truncated: false,
            original_tokens: None,
            compression_level: CompressionLevel::Full,
            utility: None,
        }
    }

    /// Set utility score from journal
    pub fn with_utility(mut self, utility: f64) -> Self {
        self.utility = Some(utility);
        self
    }

    /// Check if this is a "bright star" (utility >= 0.8)
    pub fn is_bright_star(&self) -> bool {
        self.utility.map(|u| u >= 0.8).unwrap_or(false)
    }

    /// Get the brightness indicator for output
    pub fn brightness_indicator(&self) -> &'static str {
        match self.utility {
            Some(u) if u >= 0.9 => "🌟 ",  // Very bright
            Some(u) if u >= 0.8 => "⭐ ",  // Bright
            Some(u) if u >= 0.5 => "✨ ",  // Notable
            Some(_) => "",                 // Dim
            None => "",                    // Unknown
        }
    }

    /// Mark as truncated
    pub fn with_truncation(mut self, content: String, original_tokens: usize) -> Self {
        self.tokens = content.len() / 4;
        self.content = content;
        self.truncated = true;
        self.original_tokens = Some(original_tokens);
        self
    }

    /// Mark as skeletonized
    pub fn with_skeleton(mut self, skeleton_content: String, original_tokens: usize) -> Self {
        self.tokens = skeleton_content.len() / 4;
        self.content = skeleton_content;
        self.compression_level = CompressionLevel::Skeleton;
        self.original_tokens = Some(original_tokens);
        self
    }

    /// Check if file is skeletonized
    pub fn is_skeleton(&self) -> bool {
        self.compression_level == CompressionLevel::Skeleton
    }
}

impl Default for ProcessedFile {
    fn default() -> Self {
        Self {
            path: String::new(),
            content: String::new(),
            md5: String::new(),
            language: String::new(),
            priority: 0,
            tokens: 0,
            truncated: false,
            original_tokens: None,
            compression_level: CompressionLevel::Full,
            utility: None,
        }
    }
}

/// Calculate MD5 hash of content
pub fn calculate_md5(content: &str) -> String {
    format!("{:x}", md5::compute(content.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_new() {
        let entry = FileEntry::new("test.py", "def hello(): pass");
        assert_eq!(entry.path, "test.py");
        assert!(!entry.md5.is_empty());
        assert_eq!(entry.extension(), Some("py"));
    }

    #[test]
    fn test_file_entry_token_estimate() {
        let entry = FileEntry::new("test.py", "a".repeat(400));
        assert_eq!(entry.token_estimate(), 100);
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::parse("plus-minus"), Some(OutputFormat::PlusMinus));
        assert_eq!(OutputFormat::parse("claude-xml"), Some(OutputFormat::ClaudeXml));
        assert_eq!(OutputFormat::parse("invalid"), None);
    }

    #[test]
    fn test_encoder_config_builder() {
        let config = EncoderConfig::new()
            .with_truncation(500, "smart")
            .with_format(OutputFormat::ClaudeXml)
            .with_frozen(true)
            .with_budget(10000)
            .with_lens("architecture");

        assert_eq!(config.truncate_lines, 500);
        assert_eq!(config.output_format, OutputFormat::ClaudeXml);
        assert!(config.frozen);
        assert_eq!(config.token_budget, Some(10000));
        assert_eq!(config.active_lens, Some("architecture".to_string()));
    }

    #[test]
    fn test_processed_file_from_entry() {
        let entry = FileEntry::new("src/main.rs", "fn main() {}");
        let processed = ProcessedFile::from_entry(&entry, "rust", 100);

        assert_eq!(processed.path, "src/main.rs");
        assert_eq!(processed.language, "rust");
        assert_eq!(processed.priority, 100);
        assert!(!processed.truncated);
    }

    #[test]
    fn test_calculate_md5() {
        let hash = calculate_md5("hello world");
        assert_eq!(hash, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_file_entry_with_timestamps() {
        let entry = FileEntry::new("test.rs", "fn main() {}")
            .with_timestamps(1234567890, 1234567800);

        assert_eq!(entry.mtime, 1234567890);
        assert_eq!(entry.ctime, 1234567800);
    }

    #[test]
    fn test_file_entry_with_size() {
        let entry = FileEntry::new("test.rs", "fn main() {}")
            .with_size(9999);

        assert_eq!(entry.size, 9999);
    }

    #[test]
    fn test_file_entry_extension_none() {
        let entry = FileEntry::new("Makefile", "all: build");
        assert_eq!(entry.extension(), None);
    }

    #[test]
    fn test_output_format_extension() {
        assert_eq!(OutputFormat::PlusMinus.extension(), "txt");
        assert_eq!(OutputFormat::Xml.extension(), "xml");
        assert_eq!(OutputFormat::Markdown.extension(), "md");
        assert_eq!(OutputFormat::ClaudeXml.extension(), "xml");
    }

    #[test]
    fn test_output_format_parse_variants() {
        // PlusMinus variants
        assert_eq!(OutputFormat::parse("pm"), Some(OutputFormat::PlusMinus));
        assert_eq!(OutputFormat::parse("plus_minus"), Some(OutputFormat::PlusMinus));

        // XML
        assert_eq!(OutputFormat::parse("xml"), Some(OutputFormat::Xml));

        // Markdown variants
        assert_eq!(OutputFormat::parse("markdown"), Some(OutputFormat::Markdown));
        assert_eq!(OutputFormat::parse("md"), Some(OutputFormat::Markdown));

        // ClaudeXml variants
        assert_eq!(OutputFormat::parse("claude_xml"), Some(OutputFormat::ClaudeXml));
    }

    #[test]
    fn test_output_format_default() {
        let format: OutputFormat = Default::default();
        assert_eq!(format, OutputFormat::PlusMinus);
    }

    #[test]
    fn test_skeleton_mode_parse() {
        assert_eq!(SkeletonMode::parse("auto"), Some(SkeletonMode::Auto));
        assert_eq!(SkeletonMode::parse("true"), Some(SkeletonMode::Enabled));
        assert_eq!(SkeletonMode::parse("enabled"), Some(SkeletonMode::Enabled));
        assert_eq!(SkeletonMode::parse("on"), Some(SkeletonMode::Enabled));
        assert_eq!(SkeletonMode::parse("yes"), Some(SkeletonMode::Enabled));
        assert_eq!(SkeletonMode::parse("false"), Some(SkeletonMode::Disabled));
        assert_eq!(SkeletonMode::parse("disabled"), Some(SkeletonMode::Disabled));
        assert_eq!(SkeletonMode::parse("off"), Some(SkeletonMode::Disabled));
        assert_eq!(SkeletonMode::parse("no"), Some(SkeletonMode::Disabled));
        assert_eq!(SkeletonMode::parse("invalid"), None);
    }

    #[test]
    fn test_skeleton_mode_is_enabled() {
        assert!(!SkeletonMode::Auto.is_enabled(false));
        assert!(SkeletonMode::Auto.is_enabled(true));
        assert!(SkeletonMode::Enabled.is_enabled(false));
        assert!(SkeletonMode::Enabled.is_enabled(true));
        assert!(!SkeletonMode::Disabled.is_enabled(false));
        assert!(!SkeletonMode::Disabled.is_enabled(true));
    }

    #[test]
    fn test_skeleton_mode_default() {
        let mode: SkeletonMode = Default::default();
        assert_eq!(mode, SkeletonMode::Auto);
    }

    #[test]
    fn test_metadata_mode_parse() {
        assert_eq!(MetadataMode::parse("auto"), Some(MetadataMode::Auto));
        assert_eq!(MetadataMode::parse("all"), Some(MetadataMode::All));
        assert_eq!(MetadataMode::parse("none"), Some(MetadataMode::None));
        assert_eq!(MetadataMode::parse("size-only"), Some(MetadataMode::SizeOnly));
        assert_eq!(MetadataMode::parse("size_only"), Some(MetadataMode::SizeOnly));
        assert_eq!(MetadataMode::parse("sizeonly"), Some(MetadataMode::SizeOnly));
        assert_eq!(MetadataMode::parse("invalid"), None);
    }

    #[test]
    fn test_metadata_mode_default() {
        let mode: MetadataMode = Default::default();
        assert_eq!(mode, MetadataMode::Auto);
    }

    #[test]
    fn test_encoder_config_with_skeleton_mode() {
        let config = EncoderConfig::new()
            .with_skeleton_mode(SkeletonMode::Enabled);

        assert_eq!(config.skeleton_mode, SkeletonMode::Enabled);
    }

    #[test]
    fn test_encoder_config_with_metadata_mode() {
        let config = EncoderConfig::new()
            .with_metadata_mode(MetadataMode::All);

        assert_eq!(config.metadata_mode, MetadataMode::All);
    }

    #[test]
    fn test_compression_level_default() {
        let level: CompressionLevel = Default::default();
        assert_eq!(level, CompressionLevel::Full);
    }

    #[test]
    fn test_processed_file_default() {
        let file: ProcessedFile = Default::default();
        assert!(file.path.is_empty());
        assert!(file.content.is_empty());
        assert_eq!(file.tokens, 0);
        assert!(!file.truncated);
        assert_eq!(file.compression_level, CompressionLevel::Full);
        assert_eq!(file.utility, None);
    }

    #[test]
    fn test_processed_file_with_utility() {
        let entry = FileEntry::new("test.rs", "fn main() {}");
        let processed = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_utility(0.95);

        assert_eq!(processed.utility, Some(0.95));
    }

    #[test]
    fn test_processed_file_is_bright_star() {
        let entry = FileEntry::new("test.rs", "fn main() {}");

        // No utility
        let processed = ProcessedFile::from_entry(&entry, "rust", 100);
        assert!(!processed.is_bright_star());

        // Low utility
        let processed = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_utility(0.5);
        assert!(!processed.is_bright_star());

        // High utility
        let processed = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_utility(0.8);
        assert!(processed.is_bright_star());

        let processed = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_utility(0.95);
        assert!(processed.is_bright_star());
    }

    #[test]
    fn test_processed_file_brightness_indicator() {
        let entry = FileEntry::new("test.rs", "fn main() {}");

        // Very bright (>= 0.9)
        let p = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.95);
        assert_eq!(p.brightness_indicator(), "🌟 ");

        // Bright (>= 0.8)
        let p = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.85);
        assert_eq!(p.brightness_indicator(), "⭐ ");

        // Notable (>= 0.5)
        let p = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.6);
        assert_eq!(p.brightness_indicator(), "✨ ");

        // Dim (< 0.5)
        let p = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.3);
        assert_eq!(p.brightness_indicator(), "");

        // No utility
        let p = ProcessedFile::from_entry(&entry, "rust", 100);
        assert_eq!(p.brightness_indicator(), "");
    }

    #[test]
    fn test_processed_file_with_truncation() {
        let entry = FileEntry::new("test.rs", "a".repeat(1000));
        let original_tokens = entry.token_estimate();

        let processed = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_truncation("truncated content".to_string(), original_tokens);

        assert!(processed.truncated);
        assert_eq!(processed.original_tokens, Some(250));
        assert_eq!(processed.content, "truncated content");
    }

    #[test]
    fn test_processed_file_with_skeleton() {
        let entry = FileEntry::new("test.rs", "a".repeat(1000));
        let original_tokens = entry.token_estimate();

        let processed = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_skeleton("fn main();".to_string(), original_tokens);

        assert_eq!(processed.compression_level, CompressionLevel::Skeleton);
        assert_eq!(processed.original_tokens, Some(250));
        assert_eq!(processed.content, "fn main();");
        assert!(processed.is_skeleton());
    }

    #[test]
    fn test_processed_file_is_skeleton() {
        let entry = FileEntry::new("test.rs", "fn main() {}");

        let full = ProcessedFile::from_entry(&entry, "rust", 100);
        assert!(!full.is_skeleton());

        let skeleton = ProcessedFile::from_entry(&entry, "rust", 100)
            .with_skeleton("fn main();".to_string(), 10);
        assert!(skeleton.is_skeleton());
    }

    #[test]
    fn test_config_serde() {
        let config = Config {
            ignore: vec!["*.pyc".to_string()],
            include: vec!["*.rs".to_string()],
            max_file_size: 2_000_000,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.ignore, config.ignore);
        assert_eq!(parsed.include, config.include);
        assert_eq!(parsed.max_file_size, config.max_file_size);
    }

    #[test]
    fn test_config_default() {
        // Note: serde default function is only used during deserialization
        // Default trait uses u64 default (0)
        let config: Config = Default::default();
        assert!(config.ignore.is_empty());
        assert!(config.include.is_empty());
        // max_file_size is 0 with Default trait (serde default not applied)
        assert_eq!(config.max_file_size, 0);
    }

    #[test]
    fn test_config_serde_default() {
        // Serde defaults are applied during deserialization
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.ignore.is_empty());
        assert!(config.include.is_empty());
        assert_eq!(config.max_file_size, 1_048_576); // serde default applied
    }

    #[test]
    fn test_encoder_config_default() {
        let config: EncoderConfig = Default::default();
        assert!(!config.ignore_patterns.is_empty());
        assert!(config.include_patterns.is_empty());
        assert_eq!(config.truncate_lines, 0);
        assert_eq!(config.truncate_mode, "simple");
        assert_eq!(config.sort_by, "name");
        assert_eq!(config.sort_order, "asc");
        assert!(!config.stream);
        assert!(config.truncate_summary);
        assert!(!config.frozen);
        assert!(!config.allow_sensitive);
        assert!(!config.follow_symlinks);
    }
}
