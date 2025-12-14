/// Generic regex-based language analyzer
///
/// This "Universal Adapter" allows rapid language support through regex configuration.
/// Instead of implementing separate analyzers for each language, we configure one
/// generic analyzer with language-specific patterns.

use lazy_static::lazy_static;
use regex::Regex;
use super::{AnalysisResult, LanguageAnalyzer};
use std::collections::HashMap;

/// Configuration for a language analyzer (regex patterns)
#[derive(Clone)]
pub struct AnalyzerConfig {
    pub language_name: String,
    pub extensions: Vec<String>,
    pub class_pattern: Option<Regex>,
    pub function_pattern: Option<Regex>,
    pub import_pattern: Option<Regex>,
    pub entry_point_pattern: Option<Regex>,
    pub marker_pattern: Option<Regex>,
    pub documentation_pattern: Option<Regex>,
    /// Additional patterns for language-specific features
    pub extra_patterns: HashMap<String, Regex>,
}

impl AnalyzerConfig {
    /// Create a new analyzer configuration
    pub fn new(name: &str, extensions: Vec<&str>) -> Self {
        Self {
            language_name: name.to_string(),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
            class_pattern: None,
            function_pattern: None,
            import_pattern: None,
            entry_point_pattern: None,
            marker_pattern: None,
            documentation_pattern: None,
            extra_patterns: HashMap::new(),
        }
    }

    /// Set class detection pattern
    pub fn with_class_pattern(mut self, pattern: Regex) -> Self {
        self.class_pattern = Some(pattern);
        self
    }

    /// Set function detection pattern
    pub fn with_function_pattern(mut self, pattern: Regex) -> Self {
        self.function_pattern = Some(pattern);
        self
    }

    /// Set import detection pattern
    pub fn with_import_pattern(mut self, pattern: Regex) -> Self {
        self.import_pattern = Some(pattern);
        self
    }

    /// Set entry point detection pattern
    pub fn with_entry_point_pattern(mut self, pattern: Regex) -> Self {
        self.entry_point_pattern = Some(pattern);
        self
    }

    /// Set marker detection pattern
    pub fn with_marker_pattern(mut self, pattern: Regex) -> Self {
        self.marker_pattern = Some(pattern);
        self
    }

    /// Set documentation detection pattern
    pub fn with_documentation_pattern(mut self, pattern: Regex) -> Self {
        self.documentation_pattern = Some(pattern);
        self
    }

    /// Add an extra pattern for language-specific features
    pub fn with_extra_pattern(mut self, name: &str, pattern: Regex) -> Self {
        self.extra_patterns.insert(name.to_string(), pattern);
        self
    }
}

/// Generic analyzer that works with any language via regex patterns
pub struct GenericAnalyzer {
    config: AnalyzerConfig,
}

impl GenericAnalyzer {
    /// Create a new generic analyzer with the given configuration
    pub fn new(config: AnalyzerConfig) -> Self {
        Self { config }
    }

    /// Analyze lines using configured patterns
    fn analyze_lines(&self, lines: &[&str], file_path: &str) -> AnalysisResult {
        let mut result = AnalysisResult::new(&self.config.language_name);
        let mut classes = Vec::new();
        let mut functions = Vec::new();
        let mut imports = Vec::new();
        let mut entry_points = Vec::new();
        let mut markers = Vec::new();
        let mut has_documentation = false;

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            // Class detection
            if let Some(ref pattern) = self.config.class_pattern {
                if let Some(caps) = pattern.captures(line) {
                    if let Some(name) = caps.get(1) {
                        classes.push(name.as_str().to_string());
                    }
                }
            }

            // Function detection
            if let Some(ref pattern) = self.config.function_pattern {
                if let Some(caps) = pattern.captures(line) {
                    if let Some(name) = caps.get(1) {
                        let fn_name = name.as_str().to_string();
                        functions.push(fn_name.clone());
                    }
                }
            }

            // Import detection
            if let Some(ref pattern) = self.config.import_pattern {
                if let Some(caps) = pattern.captures(line) {
                    if let Some(import) = caps.get(1) {
                        imports.push(import.as_str().trim().to_string());
                    }
                }
            }

            // Entry point detection
            if let Some(ref pattern) = self.config.entry_point_pattern {
                if pattern.is_match(line) {
                    entry_points.push(("__main__ block".to_string(), line_num));
                }
            }

            // Marker detection (TODO, FIXME, etc.)
            if let Some(ref pattern) = self.config.marker_pattern {
                if let Some(caps) = pattern.captures(line) {
                    if let Some(marker_type) = caps.get(1) {
                        markers.push(format!("{} (line {})", marker_type.as_str(), line_num));
                    }
                }
            }

            // Documentation detection
            if let Some(ref pattern) = self.config.documentation_pattern {
                if pattern.is_match(line) {
                    has_documentation = true;
                }
            }
        }

        // Categorize based on content
        let category = if !entry_points.is_empty() {
            "application"
        } else if file_path.to_lowercase().contains("test") || file_path.contains("tests/") {
            "test"
        } else {
            "library"
        };

        // Populate result
        result.classes = classes;
        result.functions = functions.into_iter().take(20).collect();
        result.imports = imports.into_iter().take(10).collect();
        result.entry_points = entry_points.iter().map(|(ep, _)| ep.clone()).collect();

        if has_documentation {
            result.documentation = vec!["docstrings".to_string()];
        }

        result.markers = markers.into_iter().take(5).collect();
        result.category = category.to_string();
        result.critical_sections = entry_points.iter().map(|(_, line)| (*line, line + 20)).collect();

        result
    }
}

impl LanguageAnalyzer for GenericAnalyzer {
    fn analyze(&self, content: &str, file_path: &str) -> AnalysisResult {
        let lines: Vec<&str> = content.lines().collect();
        self.analyze_lines(&lines, file_path)
    }

    fn supported_extensions(&self) -> &[&str] {
        // Convert Vec<String> to &[&str] - we need to leak the strings for static lifetime
        // This is safe since configs are created once at startup
        unsafe {
            std::mem::transmute(self.config.extensions.as_slice())
        }
    }

    fn language_name(&self) -> &str {
        &self.config.language_name
    }
}

// Pre-configured analyzers for common languages

lazy_static! {
    /// Python analyzer configuration
    static ref PYTHON_CONFIG: AnalyzerConfig = {
        AnalyzerConfig::new("Python", vec![".py", ".pyw"])
            .with_class_pattern(Regex::new(r"^\s*class\s+(\w+)").unwrap())
            .with_function_pattern(Regex::new(r"^\s*(?:async\s+)?def\s+(\w+)").unwrap())
            .with_import_pattern(Regex::new(r"^\s*(?:from\s+\S+\s+)?import\s+(.+)").unwrap())
            .with_entry_point_pattern(Regex::new(r#"if\s+__name__\s*==\s*['"]__main__['"]"#).unwrap())
            .with_marker_pattern(Regex::new(r"#\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)").unwrap())
            .with_documentation_pattern(Regex::new(r#"("{3}|'{3})"#).unwrap())
    };

    /// JavaScript/TypeScript analyzer configuration
    static ref JAVASCRIPT_CONFIG: AnalyzerConfig = {
        AnalyzerConfig::new("JavaScript", vec![".js", ".jsx", ".ts", ".tsx", ".mjs"])
            .with_class_pattern(Regex::new(r"^\s*(?:export\s+)?class\s+(\w+)").unwrap())
            .with_function_pattern(Regex::new(r"^\s*(?:export\s+)?(?:async\s+)?(?:function\s+(\w+)|const\s+(\w+)\s*=)").unwrap())
            .with_import_pattern(Regex::new(r#"^\s*(?:import|export|require)\s*(?:\{[^}]+\}|[\w,\s]+)?\s*(?:from\s+)?['"]([^'"]+)['"]"#).unwrap())
            .with_marker_pattern(Regex::new(r"//\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)").unwrap())
            .with_documentation_pattern(Regex::new(r"/\*\*").unwrap())
    };

    /// Shell script analyzer configuration
    static ref SHELL_CONFIG: AnalyzerConfig = {
        AnalyzerConfig::new("Shell", vec![".sh", ".bash", ".zsh"])
            .with_function_pattern(Regex::new(r"^\s*(?:function\s+)?(\w+)\s*\(\s*\)").unwrap())
            .with_entry_point_pattern(Regex::new(r"^#!/").unwrap())
            .with_marker_pattern(Regex::new(r"#\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)").unwrap())
    };

    /// Markdown analyzer configuration
    static ref MARKDOWN_CONFIG: AnalyzerConfig = {
        AnalyzerConfig::new("Markdown", vec![".md", ".markdown"])
            .with_class_pattern(Regex::new(r"^#{1,6}\s+(.+)").unwrap())  // Headers as "classes"
            .with_documentation_pattern(Regex::new(r"^#{1,6}\s").unwrap())
    };

    /// JSON analyzer configuration
    static ref JSON_CONFIG: AnalyzerConfig = {
        AnalyzerConfig::new("JSON", vec![".json"])
            // Match top-level keys like "name": or "version":
            .with_class_pattern(Regex::new(r#"^\s{0,2}"(\w+)":\s*"#).unwrap())
            .with_documentation_pattern(Regex::new(r#"^\s*\{"#).unwrap())
    };

    /// YAML analyzer configuration
    static ref YAML_CONFIG: AnalyzerConfig = {
        AnalyzerConfig::new("YAML", vec![".yml", ".yaml"])
            // Match top-level keys (no leading whitespace)
            .with_class_pattern(Regex::new(r"^(\w[\w-]*):\s*").unwrap())
            .with_documentation_pattern(Regex::new(r"^#").unwrap())
            .with_marker_pattern(Regex::new(r"#\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)").unwrap())
    };
}

/// Factory function to create a Python analyzer
pub fn create_python_analyzer() -> GenericAnalyzer {
    GenericAnalyzer::new(PYTHON_CONFIG.clone())
}

/// Factory function to create a JavaScript analyzer
pub fn create_javascript_analyzer() -> GenericAnalyzer {
    GenericAnalyzer::new(JAVASCRIPT_CONFIG.clone())
}

/// Factory function to create a Shell analyzer
pub fn create_shell_analyzer() -> GenericAnalyzer {
    GenericAnalyzer::new(SHELL_CONFIG.clone())
}

/// Factory function to create a Markdown analyzer
pub fn create_markdown_analyzer() -> GenericAnalyzer {
    GenericAnalyzer::new(MARKDOWN_CONFIG.clone())
}

/// Factory function to create a JSON analyzer
pub fn create_json_analyzer() -> GenericAnalyzer {
    GenericAnalyzer::new(JSON_CONFIG.clone())
}

/// Factory function to create a YAML analyzer
pub fn create_yaml_analyzer() -> GenericAnalyzer {
    GenericAnalyzer::new(YAML_CONFIG.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_analyzer() {
        let analyzer = create_python_analyzer();
        let content = "class User:\n    def __init__(self):\n        pass\n\nif __name__ == '__main__':\n    print('test')";
        let result = analyzer.analyze(content, "test.py");

        assert_eq!(result.language, "Python");
        assert!(result.classes.contains(&"User".to_string()));
        assert!(result.functions.contains(&"__init__".to_string()));
        assert_eq!(result.category, "application");
    }

    #[test]
    fn test_javascript_analyzer() {
        let analyzer = create_javascript_analyzer();
        let content = "class Component {}\nfunction render() {}\nconst process = () => {};";
        let result = analyzer.analyze(content, "app.js");

        assert_eq!(result.language, "JavaScript");
        assert!(result.classes.contains(&"Component".to_string()));
        assert!(result.functions.contains(&"render".to_string()) || result.functions.contains(&"process".to_string()));
    }

    #[test]
    fn test_shell_analyzer() {
        let analyzer = create_shell_analyzer();
        let content = "#!/bin/bash\nfunction deploy() {\n    echo 'deploying'\n}\nsetup() {\n    echo 'setup'\n}";
        let result = analyzer.analyze(content, "deploy.sh");

        assert_eq!(result.language, "Shell");
        assert!(result.functions.contains(&"deploy".to_string()));
        assert!(result.functions.contains(&"setup".to_string()));
        assert_eq!(result.category, "application");
    }
}
