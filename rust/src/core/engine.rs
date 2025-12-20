//! Context Engine - Main orchestration for pm_encoder
//!
//! The ContextEngine is the primary interface for serializing project contexts.
//! It coordinates file walking, analysis, truncation, and serialization.

use crate::core::error::{EncoderError, Result};
use crate::core::manifest::{ProjectManifest, ProjectType};
use crate::core::models::{EncoderConfig, FileEntry, OutputFormat, ProcessedFile};
use crate::core::serialization::{get_serializer, Serializer};
use crate::core::walker::{DefaultWalker, FileWalker, WalkConfig};
use crate::core::zoom::{ZoomAction, ZoomConfig, ZoomTarget};

/// File tier for prioritized budgeting
/// Core domain files get budget first, then config, tests last
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileTier {
    /// Core domain: src/, lib/, main source files
    Core = 0,
    /// Configuration: Cargo.toml, package.json, config files
    Config = 1,
    /// Tests: tests/, test_*, *_test.*, examples/
    Tests = 2,
    /// Other: docs, scripts, misc
    Other = 3,
}

/// Statistics about budget allocation across tiers
#[derive(Debug, Default, Clone)]
pub struct BudgetStats {
    pub core_count: usize,
    pub core_tokens: usize,
    pub config_count: usize,
    pub config_tokens: usize,
    pub test_count: usize,
    pub test_tokens: usize,
    pub other_count: usize,
    pub other_tokens: usize,
}

impl BudgetStats {
    /// Total files across all tiers
    pub fn total_files(&self) -> usize {
        self.core_count + self.config_count + self.test_count + self.other_count
    }

    /// Total tokens across all tiers
    pub fn total_tokens(&self) -> usize {
        self.core_tokens + self.config_tokens + self.test_tokens + self.other_tokens
    }
}

impl FileTier {
    /// Classify a file path into a tier based on project structure
    /// Uses project manifest to understand project type and adjust classification
    pub fn classify(path: &str, manifest: Option<&ProjectManifest>) -> Self {
        let path_lower = path.to_lowercase();

        // Config files (high value/token ratio)
        if Self::is_config_file(&path_lower) {
            return FileTier::Config;
        }

        // Test files
        if Self::is_test_file(&path_lower) {
            return FileTier::Tests;
        }

        // Core domain files
        if Self::is_core_file(&path_lower, manifest) {
            return FileTier::Core;
        }

        // Everything else
        FileTier::Other
    }

    /// Check if path is a configuration file
    fn is_config_file(path: &str) -> bool {
        // Manifest files
        let config_names = [
            "cargo.toml", "package.json", "pyproject.toml", "setup.py",
            "go.mod", "pom.xml", "build.gradle", "composer.json",
            "gemfile", "requirements.txt", "pipfile",
        ];

        // Check if the filename matches a config file
        if let Some(filename) = path.rsplit('/').next() {
            if config_names.iter().any(|c| filename == *c) {
                return true;
            }
        }

        // Config directories and extensions
        path.contains("/config/") ||
        path.contains("/configs/") ||
        path.ends_with(".toml") ||
        path.ends_with(".yaml") ||
        path.ends_with(".yml") ||
        path.ends_with(".json") && !path.contains("/test")
    }

    /// Check if path is a test file
    fn is_test_file(path: &str) -> bool {
        // Test directories
        if path.starts_with("tests/") ||
           path.starts_with("test/") ||
           path.contains("/tests/") ||
           path.contains("/test/") ||
           path.starts_with("examples/") ||
           path.contains("/examples/") ||
           path.starts_with("benches/") ||
           path.contains("/benches/") {
            return true;
        }

        // Test file patterns
        if let Some(filename) = path.rsplit('/').next() {
            let fname_lower = filename.to_lowercase();
            if fname_lower.starts_with("test_") ||
               fname_lower.ends_with("_test.py") ||
               fname_lower.ends_with("_test.rs") ||
               fname_lower.ends_with("_test.go") ||
               fname_lower.ends_with(".test.js") ||
               fname_lower.ends_with(".test.ts") ||
               fname_lower.ends_with(".spec.js") ||
               fname_lower.ends_with(".spec.ts") {
                return true;
            }
        }

        false
    }

    /// Check if path is a core domain file
    fn is_core_file(path: &str, manifest: Option<&ProjectManifest>) -> bool {
        // Standard source directories
        let core_dirs = ["src/", "lib/", "pkg/", "internal/", "app/", "core/"];

        for dir in core_dirs {
            if path.starts_with(dir) || path.contains(&format!("/{}", dir)) {
                return true;
            }
        }

        // Project-type specific logic
        if let Some(m) = manifest {
            match m.project_type {
                ProjectType::Rust => {
                    // Rust: src/ is core, also lib.rs, main.rs at root
                    if path == "lib.rs" || path == "main.rs" {
                        return true;
                    }
                }
                ProjectType::Python => {
                    // Python: any .py file not in tests
                    if path.ends_with(".py") && !Self::is_test_file(path) {
                        return true;
                    }
                }
                ProjectType::Node => {
                    // Node: src/, lib/, index.js, index.ts
                    if path == "index.js" || path == "index.ts" {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }
}

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

    /// Apply token budget with tiered allocation strategy
    ///
    /// Algorithm:
    /// 1. Classify files into tiers (Core, Config, Tests, Other)
    /// 2. Fill budget with Core files first (highest priority)
    /// 3. Then Config files (high value/token ratio)
    /// 4. Then Tests (if budget remains)
    /// 5. Finally Other files
    ///
    /// Within each tier, files are sorted by priority (highest first)
    fn apply_budget(&self, files: Vec<ProcessedFile>, budget: usize) -> Vec<ProcessedFile> {
        self.apply_budget_with_manifest(files, budget, None)
    }

    /// Apply tiered budget with optional project manifest for smarter classification
    pub fn apply_budget_with_manifest(
        &self,
        files: Vec<ProcessedFile>,
        budget: usize,
        manifest: Option<&ProjectManifest>,
    ) -> Vec<ProcessedFile> {
        // Classify files into tiers
        let mut core_files = Vec::new();
        let mut config_files = Vec::new();
        let mut test_files = Vec::new();
        let mut other_files = Vec::new();

        for file in files {
            match FileTier::classify(&file.path, manifest) {
                FileTier::Core => core_files.push(file),
                FileTier::Config => config_files.push(file),
                FileTier::Tests => test_files.push(file),
                FileTier::Other => other_files.push(file),
            }
        }

        // Sort each tier by priority (highest first)
        core_files.sort_by(|a, b| b.priority.cmp(&a.priority));
        config_files.sort_by(|a, b| b.priority.cmp(&a.priority));
        test_files.sort_by(|a, b| b.priority.cmp(&a.priority));
        other_files.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut result = Vec::new();
        let mut used = 0;

        // Fill in tier order: Core -> Config -> Tests -> Other
        for file in core_files.into_iter()
            .chain(config_files)
            .chain(test_files)
            .chain(other_files)
        {
            if used + file.tokens <= budget {
                used += file.tokens;
                result.push(file);
            }
        }

        result
    }

    /// Get budget allocation statistics (for debugging/UI)
    pub fn budget_stats(&self, files: &[ProcessedFile], manifest: Option<&ProjectManifest>) -> BudgetStats {
        let mut stats = BudgetStats::default();

        for file in files {
            match FileTier::classify(&file.path, manifest) {
                FileTier::Core => {
                    stats.core_count += 1;
                    stats.core_tokens += file.tokens;
                }
                FileTier::Config => {
                    stats.config_count += 1;
                    stats.config_tokens += file.tokens;
                }
                FileTier::Tests => {
                    stats.test_count += 1;
                    stats.test_tokens += file.tokens;
                }
                FileTier::Other => {
                    stats.other_count += 1;
                    stats.other_tokens += file.tokens;
                }
            }
        }

        stats
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

    // Tiered Budgeting Tests

    #[test]
    fn test_file_tier_classify_core() {
        assert_eq!(FileTier::classify("src/main.rs", None), FileTier::Core);
        assert_eq!(FileTier::classify("src/lib.rs", None), FileTier::Core);
        assert_eq!(FileTier::classify("src/core/engine.rs", None), FileTier::Core);
        assert_eq!(FileTier::classify("lib/utils.py", None), FileTier::Core);
        assert_eq!(FileTier::classify("pkg/handler.go", None), FileTier::Core);
        assert_eq!(FileTier::classify("internal/service.go", None), FileTier::Core);
        assert_eq!(FileTier::classify("app/models/user.rb", None), FileTier::Core);
    }

    #[test]
    fn test_file_tier_classify_config() {
        assert_eq!(FileTier::classify("Cargo.toml", None), FileTier::Config);
        assert_eq!(FileTier::classify("package.json", None), FileTier::Config);
        assert_eq!(FileTier::classify("pyproject.toml", None), FileTier::Config);
        assert_eq!(FileTier::classify("config/settings.yaml", None), FileTier::Config);
        assert_eq!(FileTier::classify("configs/prod.yml", None), FileTier::Config);
    }

    #[test]
    fn test_file_tier_classify_tests() {
        assert_eq!(FileTier::classify("tests/test_main.py", None), FileTier::Tests);
        assert_eq!(FileTier::classify("test/unit_test.rs", None), FileTier::Tests);
        assert_eq!(FileTier::classify("src/tests/integration.rs", None), FileTier::Tests);
        assert_eq!(FileTier::classify("examples/demo.py", None), FileTier::Tests);
        assert_eq!(FileTier::classify("benches/bench_main.rs", None), FileTier::Tests);
        assert_eq!(FileTier::classify("test_utils.py", None), FileTier::Tests);
        assert_eq!(FileTier::classify("handler_test.go", None), FileTier::Tests);
        assert_eq!(FileTier::classify("component.spec.ts", None), FileTier::Tests);
    }

    #[test]
    fn test_file_tier_classify_other() {
        assert_eq!(FileTier::classify("README.md", None), FileTier::Other);
        assert_eq!(FileTier::classify("docs/guide.md", None), FileTier::Other);
        assert_eq!(FileTier::classify("scripts/deploy.sh", None), FileTier::Other);
        assert_eq!(FileTier::classify("Makefile", None), FileTier::Other);
    }

    #[test]
    fn test_tiered_budget_core_first() {
        let engine = ContextEngine::new();

        // Create files from different tiers with same priority
        let files = vec![
            ProcessedFile {
                path: "tests/test_main.py".to_string(),
                content: "test".to_string(),
                md5: "test".to_string(),
                language: "python".to_string(),
                priority: 50,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "src/main.rs".to_string(),
                content: "fn main".to_string(),
                md5: "main".to_string(),
                language: "rust".to_string(),
                priority: 50,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "README.md".to_string(),
                content: "readme".to_string(),
                md5: "readme".to_string(),
                language: "markdown".to_string(),
                priority: 50,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
        ];

        // Budget for only one file - should pick Core (src/main.rs)
        let result = engine.apply_budget(files, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.rs");
    }

    #[test]
    fn test_tiered_budget_order() {
        let engine = ContextEngine::new();

        // Create one file from each tier
        let files = vec![
            ProcessedFile {
                path: "docs/guide.md".to_string(),  // Other
                content: "guide".to_string(),
                md5: "guide".to_string(),
                language: "markdown".to_string(),
                priority: 50,
                tokens: 50,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "tests/test.py".to_string(),  // Tests
                content: "test".to_string(),
                md5: "test".to_string(),
                language: "python".to_string(),
                priority: 50,
                tokens: 50,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "Cargo.toml".to_string(),  // Config
                content: "[package]".to_string(),
                md5: "cargo".to_string(),
                language: "toml".to_string(),
                priority: 50,
                tokens: 50,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "src/lib.rs".to_string(),  // Core
                content: "pub fn".to_string(),
                md5: "lib".to_string(),
                language: "rust".to_string(),
                priority: 50,
                tokens: 50,
                truncated: false,
                original_tokens: None,
            },
        ];

        // Budget for 3 files - should pick Core, Config, Tests (drop Other)
        let result = engine.apply_budget(files, 150);
        assert_eq!(result.len(), 3);

        // Verify order: Core -> Config -> Tests
        assert_eq!(result[0].path, "src/lib.rs");      // Core
        assert_eq!(result[1].path, "Cargo.toml");       // Config
        assert_eq!(result[2].path, "tests/test.py");    // Tests
    }

    #[test]
    fn test_budget_stats() {
        let engine = ContextEngine::new();

        let files = vec![
            ProcessedFile {
                path: "src/main.rs".to_string(),
                content: "fn main".to_string(),
                md5: "main".to_string(),
                language: "rust".to_string(),
                priority: 50,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "src/lib.rs".to_string(),
                content: "pub fn".to_string(),
                md5: "lib".to_string(),
                language: "rust".to_string(),
                priority: 50,
                tokens: 150,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "Cargo.toml".to_string(),
                content: "[package]".to_string(),
                md5: "cargo".to_string(),
                language: "toml".to_string(),
                priority: 50,
                tokens: 50,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "tests/test.py".to_string(),
                content: "test".to_string(),
                md5: "test".to_string(),
                language: "python".to_string(),
                priority: 50,
                tokens: 80,
                truncated: false,
                original_tokens: None,
            },
        ];

        let stats = engine.budget_stats(&files, None);

        assert_eq!(stats.core_count, 2);
        assert_eq!(stats.core_tokens, 250);
        assert_eq!(stats.config_count, 1);
        assert_eq!(stats.config_tokens, 50);
        assert_eq!(stats.test_count, 1);
        assert_eq!(stats.test_tokens, 80);
        assert_eq!(stats.other_count, 0);
        assert_eq!(stats.other_tokens, 0);

        assert_eq!(stats.total_files(), 4);
        assert_eq!(stats.total_tokens(), 380);
    }

    #[test]
    fn test_tiered_budget_with_priority_within_tier() {
        let engine = ContextEngine::new();

        // Two core files with different priorities
        let files = vec![
            ProcessedFile {
                path: "src/low_priority.rs".to_string(),
                content: "low".to_string(),
                md5: "low".to_string(),
                language: "rust".to_string(),
                priority: 30,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
            ProcessedFile {
                path: "src/high_priority.rs".to_string(),
                content: "high".to_string(),
                md5: "high".to_string(),
                language: "rust".to_string(),
                priority: 80,
                tokens: 100,
                truncated: false,
                original_tokens: None,
            },
        ];

        // Budget for one file - should pick higher priority within Core tier
        let result = engine.apply_budget(files, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/high_priority.rs");
    }

    #[test]
    fn test_file_tier_with_rust_manifest() {
        use crate::core::manifest::{ProjectManifest, ProjectType};
        use std::path::PathBuf;

        let manifest = ProjectManifest {
            root: PathBuf::from("/project"),
            project_type: ProjectType::Rust,
            manifest_files: vec![PathBuf::from("Cargo.toml")],
            is_workspace: false,
        };

        // Root lib.rs should be Core for Rust projects
        assert_eq!(FileTier::classify("lib.rs", Some(&manifest)), FileTier::Core);
        assert_eq!(FileTier::classify("main.rs", Some(&manifest)), FileTier::Core);
    }

    #[test]
    fn test_file_tier_with_python_manifest() {
        use crate::core::manifest::{ProjectManifest, ProjectType};
        use std::path::PathBuf;

        let manifest = ProjectManifest {
            root: PathBuf::from("/project"),
            project_type: ProjectType::Python,
            manifest_files: vec![PathBuf::from("pyproject.toml")],
            is_workspace: false,
        };

        // Any .py file not in tests should be Core for Python projects
        assert_eq!(FileTier::classify("utils.py", Some(&manifest)), FileTier::Core);
        assert_eq!(FileTier::classify("module/handler.py", Some(&manifest)), FileTier::Core);

        // But test files are still Tests
        assert_eq!(FileTier::classify("test_utils.py", Some(&manifest)), FileTier::Tests);
    }
}
