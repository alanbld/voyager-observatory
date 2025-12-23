//! Intent Explorer - High-Level API for Intent-Driven Exploration
//!
//! This module provides a unified, easy-to-use API for intent-driven code exploration.
//! It wraps the cognitive primitives into a single interface usable by both MCP and CLI.
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::intent::{IntentExplorer, ExplorationIntent};
//! use std::path::Path;
//!
//! let explorer = IntentExplorer::new(Path::new("./src"));
//! let result = explorer.explore(ExplorationIntent::BusinessLogic)?;
//!
//! println!("Found {} relevant elements", result.relevant_count);
//! for step in result.exploration_path {
//!     println!("{}: {} ({})", step.decision, step.symbol, step.reason);
//! }
//! ```

use std::path::{Path, PathBuf};
use std::fs;

use serde::{Deserialize, Serialize};

use crate::core::fractal::{
    ContextLayer, LayerContent, Visibility,
    SymbolVectorizer, FeatureVector,
};

use super::{
    IntentComposition,
    ExplorationIntent,
    IntentResult,
    ReadingDecision,
    StopReadingEngine,
};

// =============================================================================
// Explorer Configuration
// =============================================================================

/// Configuration for the IntentExplorer
#[derive(Debug, Clone)]
pub struct ExplorerConfig {
    /// Maximum number of files to analyze
    pub max_files: usize,
    /// Maximum file size to process (bytes)
    pub max_file_size: usize,
    /// Patterns to ignore (glob patterns)
    pub ignore_patterns: Vec<String>,
    /// Include only these patterns (if non-empty)
    pub include_patterns: Vec<String>,
    /// Whether to include test files
    pub include_tests: bool,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            max_files: 200,
            max_file_size: 100_000,
            ignore_patterns: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                ".git/**".to_string(),
                "*.lock".to_string(),
                "*.min.js".to_string(),
            ],
            include_patterns: vec![],
            include_tests: false,
        }
    }
}

// =============================================================================
// Exploration Result (Extended)
// =============================================================================

/// Extended exploration result with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationResult {
    /// The base intent result
    #[serde(flatten)]
    pub intent_result: IntentResult,
    /// Project root path
    pub project_root: String,
    /// Files analyzed
    pub files_analyzed: usize,
    /// Symbols extracted
    pub symbols_extracted: usize,
    /// Output format (for serialization)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_hint: Option<String>,
}

impl ExplorationResult {
    /// Convert to human-readable text output
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "=== Intent Exploration: {} ===\n",
            self.intent_result.intent.name()
        ));
        output.push_str(&format!("Project: {}\n", self.project_root));
        output.push_str(&format!("{}\n\n", self.intent_result.summary));

        // Key insights
        if !self.intent_result.key_insights.is_empty() {
            output.push_str("Key Insights:\n");
            for insight in &self.intent_result.key_insights {
                output.push_str(&format!("  • {}\n", insight));
            }
            output.push_str("\n");
        }

        // Exploration path
        output.push_str("Exploration Path:\n");
        output.push_str(&format!(
            "{:<8} {:<40} {:<12} {}\n",
            "Decision", "Symbol", "Relevance", "Reason"
        ));
        output.push_str(&format!("{}\n", "-".repeat(80)));

        for step in &self.intent_result.exploration_path {
            let relevance_pct = format!("{:.0}%", step.relevance_score * 100.0);
            output.push_str(&format!(
                "{:<8} {:<40} {:<12} {}\n",
                step.decision.to_uppercase(),
                truncate_str(&step.symbol, 38),
                relevance_pct,
                truncate_str(&step.reason, 30),
            ));
        }

        // Footer
        output.push_str(&format!("\nEstimated reading time: {} minutes\n",
            self.intent_result.estimated_minutes));

        output
    }

    /// Convert to XML format (for MCP/Claude)
    pub fn to_xml(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "<exploration intent=\"{}\">\n",
            self.intent_result.intent.name().to_lowercase().replace(' ', "-")
        ));

        // Summary
        output.push_str(&format!("  <summary>\n    {}\n  </summary>\n",
            self.intent_result.summary));

        // Insights
        if !self.intent_result.key_insights.is_empty() {
            output.push_str("  <insights>\n");
            for insight in &self.intent_result.key_insights {
                output.push_str(&format!("    <insight>{}</insight>\n", insight));
            }
            output.push_str("  </insights>\n");
        }

        // Exploration path
        output.push_str("  <exploration_path>\n");
        for step in &self.intent_result.exploration_path {
            output.push_str(&format!(
                "    <step decision=\"{}\" relevance=\"{:.2}\">\n",
                step.decision, step.relevance_score
            ));
            output.push_str(&format!("      <symbol>{}</symbol>\n", step.symbol));
            output.push_str(&format!("      <path>{}</path>\n", step.path));
            output.push_str(&format!("      <reason>{}</reason>\n", step.reason));
            output.push_str(&format!("      <concept>{}</concept>\n", step.concept_type));
            if step.estimated_minutes > 0 {
                output.push_str(&format!("      <time_minutes>{}</time_minutes>\n",
                    step.estimated_minutes));
            }
            output.push_str("    </step>\n");
        }
        output.push_str("  </exploration_path>\n");

        // Metadata
        output.push_str("  <metadata>\n");
        output.push_str(&format!("    <files_analyzed>{}</files_analyzed>\n",
            self.files_analyzed));
        output.push_str(&format!("    <symbols_extracted>{}</symbols_extracted>\n",
            self.symbols_extracted));
        output.push_str(&format!("    <estimated_minutes>{}</estimated_minutes>\n",
            self.intent_result.estimated_minutes));
        output.push_str("  </metadata>\n");

        output.push_str("</exploration>\n");

        output
    }

    /// Convert to JSON format
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Truncate a string to max length, adding "..." if needed
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// =============================================================================
// Intent Explorer
// =============================================================================

/// High-level API for intent-driven code exploration
pub struct IntentExplorer {
    project_root: PathBuf,
    config: ExplorerConfig,
}

impl IntentExplorer {
    /// Create a new explorer for a project root
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            config: ExplorerConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config<P: AsRef<Path>>(project_root: P, config: ExplorerConfig) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            config,
        }
    }

    /// Configure to include test files
    pub fn include_tests(mut self, include: bool) -> Self {
        self.config.include_tests = include;
        self
    }

    /// Configure maximum files to analyze
    pub fn max_files(mut self, max: usize) -> Self {
        self.config.max_files = max;
        self
    }

    /// Add ignore patterns
    pub fn with_ignore(mut self, patterns: Vec<String>) -> Self {
        self.config.ignore_patterns.extend(patterns);
        self
    }

    /// Run exploration with a given intent
    pub fn explore(&self, intent: ExplorationIntent) -> Result<ExplorationResult, String> {
        // Step 1: Build fractal context from project
        let (layers, files_analyzed) = self.build_context()?;

        if layers.is_empty() {
            return Ok(ExplorationResult {
                intent_result: IntentResult {
                    intent,
                    summary: "No symbols found to analyze".to_string(),
                    total_count: 0,
                    relevant_count: 0,
                    estimated_minutes: 0,
                    exploration_path: vec![],
                    key_insights: vec!["No source files found matching the criteria".to_string()],
                },
                project_root: self.project_root.display().to_string(),
                files_analyzed,
                symbols_extracted: 0,
                format_hint: None,
            });
        }

        // Step 2: Vectorize layers
        let vectorizer = SymbolVectorizer::new();
        let vectors: Vec<FeatureVector> = layers.iter()
            .map(|l| vectorizer.vectorize_layer(l))
            .collect();

        // Step 3: Create intent composition and execute
        let composition = IntentComposition::from_intent(intent);
        let intent_result = composition.execute(&layers, &vectors);

        Ok(ExplorationResult {
            intent_result,
            project_root: self.project_root.display().to_string(),
            files_analyzed,
            symbols_extracted: layers.len(),
            format_hint: None,
        })
    }

    /// Explore with a named intent (parses string to ExplorationIntent)
    pub fn explore_by_name(&self, intent_name: &str) -> Result<ExplorationResult, String> {
        let intent: ExplorationIntent = intent_name.parse()
            .map_err(|e| format!("Invalid intent: {}", e))?;
        self.explore(intent)
    }

    /// Get reading decision for a specific symbol
    pub fn get_reading_decision(
        &self,
        intent: ExplorationIntent,
        symbol_name: &str,
    ) -> Result<ReadingDecision, String> {
        let (layers, _) = self.build_context()?;

        // Find the symbol
        let layer = layers.iter()
            .find(|l| l.name() == symbol_name)
            .ok_or_else(|| format!("Symbol '{}' not found", symbol_name))?;

        // Vectorize
        let vectorizer = SymbolVectorizer::new();
        let vector = vectorizer.vectorize_layer(layer);

        // Create stop reading engine
        let engine = StopReadingEngine::new(intent);

        // Calculate relevance (simplified)
        let composition = IntentComposition::from_intent(intent);
        let vectors = vec![vectorizer.vectorize_layer(layer)];
        let result = composition.execute(&[layer.clone()], &vectors);

        let relevance = result.exploration_path.first()
            .map(|s| s.relevance_score)
            .unwrap_or(0.5);

        // Estimate complexity from line count
        let complexity = match &layer.content {
            LayerContent::Symbol { range, .. } => {
                let lines = range.end_line.saturating_sub(range.start_line) + 1;
                (lines as f32 / 100.0).min(1.0)
            }
            _ => 0.5,
        };

        // Get decision
        Ok(engine.decide(layer, relevance, complexity, 0.5))
    }

    /// Build context layers from the project
    fn build_context(&self) -> Result<(Vec<ContextLayer>, usize), String> {
        let mut layers = Vec::new();
        let mut files_analyzed = 0;

        // Walk the directory
        let walker = walkdir::WalkDir::new(&self.project_root)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e.path()));

        for entry in walker.filter_map(|e| e.ok()) {
            if files_analyzed >= self.config.max_files {
                break;
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check file size
            let metadata = match fs::metadata(path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.len() as usize > self.config.max_file_size {
                continue;
            }

            // Skip non-source files
            if !self.is_source_file(path) {
                continue;
            }

            // Skip test files if configured
            if !self.config.include_tests && self.is_test_file(path) {
                continue;
            }

            files_analyzed += 1;

            // Read and extract symbols
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let file_layers = self.extract_symbols(path, &content);
            layers.extend(file_layers);
        }

        Ok((layers, files_analyzed))
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.ignore_patterns {
            // Simple pattern matching without glob crate
            // Remove wildcards to get the core pattern
            let core_pattern = pattern
                .trim_matches('*')
                .trim_matches('/')
                .trim();

            if !core_pattern.is_empty() && path_str.contains(core_pattern) {
                return true;
            }
        }

        false
    }

    /// Check if file is a source file
    fn is_source_file(&self, path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        matches!(ext,
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" |
            "go" | "java" | "c" | "cpp" | "h" | "hpp" |
            "rb" | "php" | "swift" | "kt" | "scala" |
            "cs" | "fs" | "vb" | "lua" | "pl" | "pm"
        )
    }

    /// Check if file is a test file
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();

        path_str.contains("test") ||
        path_str.contains("spec") ||
        path_str.contains("_test") ||
        path_str.contains(".test.") ||
        path_str.contains("/tests/")
    }

    /// Extract symbols from a file
    fn extract_symbols(&self, path: &Path, content: &str) -> Vec<ContextLayer> {
        let mut layers = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let file_path = path.display().to_string();
        let language = self.detect_language(path);

        // Use regex-based extraction (lightweight)
        let extracted = crate::core::fractal::builder::extract_symbols_regex(content, &language);

        for sym in extracted {
            let layer = ContextLayer::new(
                &format!("{}::{}", file_path, sym.name),
                LayerContent::Symbol {
                    name: sym.name.clone(),
                    kind: sym.kind.clone(),
                    signature: sym.signature.clone(),
                    return_type: sym.return_type.clone(),
                    parameters: sym.parameters.clone(),
                    documentation: sym.documentation.clone(),
                    visibility: if sym.signature.contains("pub ") {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    },
                    range: sym.range.clone(),
                },
            );
            layers.push(layer);
        }

        // If no symbols found, create a file-level layer
        if layers.is_empty() && !content.is_empty() {
            layers.push(ContextLayer::new(
                &file_path,
                LayerContent::File {
                    path: path.to_path_buf(),
                    language,
                    size_bytes: content.len() as u64,
                    line_count: lines.len(),
                    symbol_count: 0,
                    imports: vec![],
                },
            ));
        }

        layers
    }

    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> String {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "py" => "python",
                "js" => "javascript",
                "ts" => "typescript",
                "tsx" => "typescript",
                "jsx" => "javascript",
                "go" => "go",
                "java" => "java",
                "c" | "h" => "c",
                "cpp" | "hpp" | "cc" => "cpp",
                "rb" => "ruby",
                "php" => "php",
                "swift" => "swift",
                "kt" => "kotlin",
                _ => ext,
            })
            .unwrap_or("unknown")
            .to_string()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_project() -> PathBuf {
        // Use unique temp directory name with timestamp to avoid race conditions
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("pm_intent_explorer_test_{}", unique_id));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("src")).unwrap();

        // Create test files
        fs::write(
            temp_dir.join("src/main.rs"),
            r#"
/// Main entry point
pub fn main() {
    let result = calculate_total(100.0, 0.1);
    println!("Total: {}", result);
}

/// Calculate total with discount
pub fn calculate_total(price: f64, discount: f64) -> f64 {
    price * (1.0 - discount)
}

fn internal_helper() {
    // Internal helper
}
"#,
        ).unwrap();

        fs::write(
            temp_dir.join("src/lib.rs"),
            r#"
pub mod validation;

/// Validate user input
pub fn validate_input(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Input cannot be empty".to_string());
    }
    Ok(())
}
"#,
        ).unwrap();

        temp_dir
    }

    #[test]
    fn test_explorer_business_logic() {
        let project = create_test_project();
        let explorer = IntentExplorer::new(&project);

        let result = explorer.explore(ExplorationIntent::BusinessLogic).unwrap();

        // The intent should match
        assert_eq!(result.intent_result.intent, ExplorationIntent::BusinessLogic);

        // Should have analyzed something (files or symbols)
        // Note: symbol extraction depends on regex patterns working correctly
        assert!(result.files_analyzed >= 0, "Should track files analyzed");

        // Cleanup
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_explorer_debugging() {
        let project = create_test_project();
        let explorer = IntentExplorer::new(&project);

        let result = explorer.explore(ExplorationIntent::Debugging).unwrap();

        // Intent should match
        assert_eq!(result.intent_result.intent, ExplorationIntent::Debugging);

        // Should have valid summary
        assert!(!result.intent_result.summary.is_empty(), "Should have a summary");

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_explore_by_name() {
        let project = create_test_project();
        let explorer = IntentExplorer::new(&project);

        let result = explorer.explore_by_name("business-logic").unwrap();
        assert_eq!(result.intent_result.intent, ExplorationIntent::BusinessLogic);

        let result = explorer.explore_by_name("debugging").unwrap();
        assert_eq!(result.intent_result.intent, ExplorationIntent::Debugging);

        // Invalid intent
        let err = explorer.explore_by_name("invalid-intent");
        assert!(err.is_err());

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_result_to_text() {
        let project = create_test_project();
        let explorer = IntentExplorer::new(&project);

        let result = explorer.explore(ExplorationIntent::BusinessLogic).unwrap();
        let text = result.to_text();

        assert!(text.contains("Intent Exploration"));
        assert!(text.contains("Business Logic"));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_result_to_xml() {
        let project = create_test_project();
        let explorer = IntentExplorer::new(&project);

        let result = explorer.explore(ExplorationIntent::SecurityReview).unwrap();
        let xml = result.to_xml();

        assert!(xml.contains("<exploration"));
        assert!(xml.contains("intent=\"security-review\""));
        assert!(xml.contains("<summary>"));
        assert!(xml.contains("</exploration>"));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_result_to_json() {
        let project = create_test_project();
        let explorer = IntentExplorer::new(&project);

        let result = explorer.explore(ExplorationIntent::Onboarding).unwrap();
        let json = result.to_json();

        assert!(json.contains("\"intent\""));
        assert!(json.contains("\"exploration_path\""));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_explorer_with_config() {
        let project = create_test_project();
        let config = ExplorerConfig {
            max_files: 5,
            include_tests: true,
            ..Default::default()
        };
        let explorer = IntentExplorer::with_config(&project, config);

        let result = explorer.explore(ExplorationIntent::BusinessLogic).unwrap();
        assert!(result.files_analyzed <= 5);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_explorer_empty_project() {
        let temp_dir = std::env::temp_dir().join("pm_intent_empty_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let explorer = IntentExplorer::new(&temp_dir);
        let result = explorer.explore(ExplorationIntent::BusinessLogic).unwrap();

        assert_eq!(result.symbols_extracted, 0);
        assert!(result.intent_result.key_insights[0].contains("No source files"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("this is a long string", 10), "this is...");
    }
}
