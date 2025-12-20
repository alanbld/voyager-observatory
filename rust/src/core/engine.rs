//! Context Engine - Main orchestration for pm_encoder
//!
//! The ContextEngine is the primary interface for serializing project contexts.
//! It coordinates file walking, analysis, truncation, and serialization.

use crate::core::error::{EncoderError, Result};
use crate::core::models::{EncoderConfig, FileEntry, OutputFormat, ProcessedFile};
use crate::core::serialization::{get_serializer, Serializer};
use crate::core::walker::{DefaultWalker, FileWalker, WalkConfig};
use crate::core::zoom::{ZoomAction, ZoomConfig, ZoomTarget};

/// The main context serialization engine
pub struct ContextEngine {
    /// Engine configuration
    config: EncoderConfig,
    /// File walker implementation
    walker: Box<dyn FileWalker>,
    /// Output serializer
    serializer: Box<dyn Serializer>,
}

impl ContextEngine {
    /// Create a new ContextEngine with default configuration
    pub fn new() -> Self {
        Self::with_config(EncoderConfig::default())
    }

    /// Create a new ContextEngine with custom configuration
    pub fn with_config(config: EncoderConfig) -> Self {
        let serializer = get_serializer(config.output_format);
        Self {
            config,
            walker: Box::new(DefaultWalker::new()),
            serializer,
        }
    }

    /// Builder: set a custom file walker
    pub fn with_walker(mut self, walker: impl FileWalker + 'static) -> Self {
        self.walker = Box::new(walker);
        self
    }

    /// Builder: set output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.config.output_format = format;
        self.serializer = get_serializer(format);
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &EncoderConfig {
        &self.config
    }

    /// Serialize a project directory
    pub fn serialize(&self, root: &str) -> Result<String> {
        let walk_config = WalkConfig {
            ignore_patterns: self.config.ignore_patterns.clone(),
            include_patterns: self.config.include_patterns.clone(),
            max_file_size: self.config.max_file_size,
        };

        // Walk directory
        let entries = self.walker.walk(root, &walk_config)?;

        // Sort entries
        let sorted = self.sort_entries(entries);

        // Process files (language detection, truncation)
        let processed = self.process_files(&sorted);

        // Apply token budget if set
        let final_files = if let Some(budget) = self.config.token_budget {
            self.apply_budget(processed, budget)
        } else {
            processed
        };

        // Serialize based on format
        if self.config.output_format == OutputFormat::ClaudeXml {
            self.serialize_claude_xml(&final_files)
        } else {
            Ok(self.serializer.serialize_files(&final_files))
        }
    }

    /// Serialize a zoom target
    pub fn zoom(&self, root: &str, config: &ZoomConfig) -> Result<String> {
        // First, walk and find matching files
        let walk_config = WalkConfig {
            ignore_patterns: self.config.ignore_patterns.clone(),
            include_patterns: self.config.include_patterns.clone(),
            max_file_size: self.config.max_file_size,
        };

        let entries = self.walker.walk(root, &walk_config)?;

        // Find matching content based on zoom target
        let filtered = match &config.target {
            ZoomTarget::Function(name) => {
                self.find_function(&entries, name)
            }
            ZoomTarget::Class(name) => {
                self.find_class(&entries, name)
            }
            ZoomTarget::Module(name) => {
                self.find_module(&entries, name)
            }
            ZoomTarget::File { path, start_line, end_line } => {
                self.find_file(&entries, path, *start_line, *end_line)
            }
        };

        if filtered.is_empty() {
            return Err(EncoderError::InvalidZoomTarget {
                target: config.target.to_string(),
            });
        }

        // Process and serialize
        let processed = self.process_files(&filtered);
        Ok(self.serializer.serialize_files(&processed))
    }

    /// Sort entries based on configuration
    fn sort_entries(&self, mut entries: Vec<FileEntry>) -> Vec<FileEntry> {
        let is_desc = self.config.sort_order == "desc";

        match self.config.sort_by.as_str() {
            "name" => {
                if is_desc {
                    entries.sort_by(|a, b| b.path.cmp(&a.path));
                } else {
                    entries.sort_by(|a, b| a.path.cmp(&b.path));
                }
            }
            "mtime" => {
                if is_desc {
                    entries.sort_by(|a, b| b.mtime.cmp(&a.mtime));
                } else {
                    entries.sort_by(|a, b| a.mtime.cmp(&b.mtime));
                }
            }
            "ctime" => {
                if is_desc {
                    entries.sort_by(|a, b| b.ctime.cmp(&a.ctime));
                } else {
                    entries.sort_by(|a, b| a.ctime.cmp(&b.ctime));
                }
            }
            _ => {
                entries.sort_by(|a, b| a.path.cmp(&b.path));
            }
        }

        entries
    }

    /// Process files (detect language, apply truncation)
    fn process_files(&self, entries: &[FileEntry]) -> Vec<ProcessedFile> {
        use crate::core::serialization::truncation_marker;

        entries.iter().map(|entry| {
            let language = detect_language(&entry.path);
            let priority = 50; // TODO: Get from lens manager

            let mut processed = ProcessedFile::from_entry(entry, &language, priority);

            // Apply truncation if configured
            if self.config.truncate_lines > 0 {
                let lines: Vec<&str> = entry.content.lines().collect();
                if lines.len() > self.config.truncate_lines {
                    let kept_lines = self.config.truncate_lines;
                    let original_lines = lines.len();
                    let original_tokens = entry.token_estimate();

                    // Create zoom action for this truncated file
                    let zoom_action = ZoomAction::for_file(&entry.path, original_tokens);

                    // Build truncated content with zoom affordance
                    let mut truncated: String = lines[..kept_lines].join("\n");
                    if self.config.truncate_summary {
                        truncated.push('\n');
                        truncated.push_str(&truncation_marker(
                            original_lines,
                            kept_lines,
                            Some(&zoom_action),
                        ));
                    }

                    processed = processed.with_truncation(truncated, original_tokens);
                }
            }

            processed
        }).collect()
    }

    /// Apply token budget (simple drop strategy)
    fn apply_budget(&self, files: Vec<ProcessedFile>, budget: usize) -> Vec<ProcessedFile> {
        let mut result = Vec::new();
        let mut used = 0;

        // Sort by priority (highest first)
        let mut sorted = files;
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for file in sorted {
            if used + file.tokens <= budget {
                used += file.tokens;
                result.push(file);
            }
        }

        result
    }

    /// Serialize to Claude-XML format
    fn serialize_claude_xml(&self, files: &[ProcessedFile]) -> Result<String> {
        use crate::formats::{XmlWriter, XmlConfig, AttentionEntry};

        let mut buffer = Vec::new();

        let xml_config = XmlConfig {
            package: "pm_encoder".to_string(),
            version: crate::VERSION.to_string(),
            lens: self.config.active_lens.clone(),
            token_budget: self.config.token_budget,
            utilized_tokens: Some(files.iter().map(|f| f.tokens).sum()),
            frozen: self.config.frozen,
            allow_sensitive: self.config.allow_sensitive,
            snapshot_id: if self.config.frozen {
                Some("FROZEN_SNAPSHOT".to_string())
            } else {
                None
            },
        };

        let mut writer = XmlWriter::new(&mut buffer, xml_config);

        // Build attention entries
        let attention_entries: Vec<AttentionEntry> = files.iter().map(|f| {
            AttentionEntry {
                path: f.path.clone(),
                priority: f.priority,
                tokens: f.tokens,
                truncated: f.truncated,
                dropped: false,
                utility_score: None,
            }
        }).collect();

        writer.write_context_start().map_err(|e| EncoderError::xml_error(e.to_string()))?;
        writer.write_metadata(&attention_entries).map_err(|e| EncoderError::xml_error(e.to_string()))?;
        writer.write_files_start().map_err(|e| EncoderError::xml_error(e.to_string()))?;

        for file in files {
            let zoom_cmd = if file.truncated {
                Some(format!("--include {} --truncate 0", file.path))
            } else {
                None
            };

            writer.write_file(
                &file.path,
                &file.language,
                &file.md5,
                file.priority,
                &file.content,
                file.truncated,
                file.original_tokens,
                zoom_cmd.as_deref(),
            ).map_err(|e| EncoderError::xml_error(e.to_string()))?;
        }

        writer.write_files_end().map_err(|e| EncoderError::xml_error(e.to_string()))?;
        writer.write_context_end().map_err(|e| EncoderError::xml_error(e.to_string()))?;

        String::from_utf8(buffer).map_err(EncoderError::from)
    }

    // Zoom helper methods

    fn find_function(&self, entries: &[FileEntry], name: &str) -> Vec<FileEntry> {
        let pattern = format!("fn {}|def {}|function {}", name, name, name);
        entries.iter()
            .filter(|e| e.content.contains(&format!("fn {}", name)) ||
                       e.content.contains(&format!("def {}", name)) ||
                       e.content.contains(&format!("function {}", name)))
            .cloned()
            .collect()
    }

    fn find_class(&self, entries: &[FileEntry], name: &str) -> Vec<FileEntry> {
        entries.iter()
            .filter(|e| e.content.contains(&format!("class {}", name)) ||
                       e.content.contains(&format!("struct {}", name)))
            .cloned()
            .collect()
    }

    fn find_module(&self, entries: &[FileEntry], name: &str) -> Vec<FileEntry> {
        entries.iter()
            .filter(|e| e.path.contains(name) ||
                       e.path.ends_with(&format!("{}.py", name)) ||
                       e.path.ends_with(&format!("{}.rs", name)) ||
                       e.path.ends_with(&format!("{}/mod.rs", name)))
            .cloned()
            .collect()
    }

    fn find_file(&self, entries: &[FileEntry], path: &str, start: Option<usize>, end: Option<usize>) -> Vec<FileEntry> {
        entries.iter()
            .filter(|e| e.path == path || e.path.ends_with(path))
            .map(|e| {
                if start.is_some() || end.is_some() {
                    let lines: Vec<&str> = e.content.lines().collect();
                    let s = start.unwrap_or(1).saturating_sub(1);
                    let e_idx = end.unwrap_or(lines.len()).min(lines.len());
                    let content = lines[s..e_idx].join("\n");
                    FileEntry {
                        path: e.path.clone(),
                        content,
                        md5: e.md5.clone(),
                        mtime: e.mtime,
                        ctime: e.ctime,
                    }
                } else {
                    e.clone()
                }
            })
            .collect()
    }
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect programming language from file extension
pub fn detect_language(path: &str) -> String {
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
        _ => "text",
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_engine_new() {
        let engine = ContextEngine::new();
        assert_eq!(engine.config().output_format, OutputFormat::PlusMinus);
    }

    #[test]
    fn test_engine_with_config() {
        let config = EncoderConfig::new()
            .with_format(OutputFormat::Markdown)
            .with_frozen(true);
        let engine = ContextEngine::with_config(config);

        assert_eq!(engine.config().output_format, OutputFormat::Markdown);
        assert!(engine.config().frozen);
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("test.py"), "python");
        assert_eq!(detect_language("test.rs"), "rust");
        assert_eq!(detect_language("test.unknown"), "text");
    }

    #[test]
    fn test_engine_serialize() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");
        fs::write(&file_path, "def hello(): pass").unwrap();

        let engine = ContextEngine::new();
        let result = engine.serialize(temp_dir.path().to_str().unwrap());

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("test.py"));
        assert!(output.contains("def hello()"));
    }

    #[test]
    fn test_engine_sort_entries() {
        let engine = ContextEngine::new();
        let entries = vec![
            FileEntry::new("b.txt", "b"),
            FileEntry::new("a.txt", "a"),
            FileEntry::new("c.txt", "c"),
        ];

        let sorted = engine.sort_entries(entries);
        assert_eq!(sorted[0].path, "a.txt");
        assert_eq!(sorted[1].path, "b.txt");
        assert_eq!(sorted[2].path, "c.txt");
    }

    #[test]
    fn test_engine_process_files_with_truncation() {
        let config = EncoderConfig::new().with_truncation(2, "simple");
        let engine = ContextEngine::with_config(config);

        let entries = vec![FileEntry::new("test.py", "line1\nline2\nline3\nline4")];
        let processed = engine.process_files(&entries);

        assert_eq!(processed.len(), 1);
        assert!(processed[0].truncated);
        // Content includes kept lines + truncation marker with zoom affordance
        assert!(processed[0].content.contains("line1"));
        assert!(processed[0].content.contains("line2"));
        assert!(!processed[0].content.contains("line3"));
        assert!(processed[0].content.contains("TRUNCATED"));
        assert!(processed[0].content.contains("ZOOM_AFFORDANCE"));
    }

    #[test]
    fn test_engine_apply_budget() {
        let engine = ContextEngine::new();

        let files = vec![
            ProcessedFile {
                path: "big.py".to_string(),
                content: "x".repeat(400),
                md5: "abc".to_string(),
                language: "python".to_string(),
                priority: 50,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "small.py".to_string(),
                content: "y".repeat(40),
                md5: "def".to_string(),
                language: "python".to_string(),
                priority: 100,
                tokens: 10,
                truncated: false,
                original_tokens: None,
            },
        ];

        // Budget of 50 should only include small.py (higher priority)
        let result = engine.apply_budget(files, 50);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "small.py");
    }
}
