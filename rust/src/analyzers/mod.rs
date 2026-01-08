pub mod generic;
/// Language analyzers for extracting metadata from source files
pub mod rust_analyzer;

pub use generic::{
    create_javascript_analyzer, create_json_analyzer, create_markdown_analyzer,
    create_python_analyzer, create_shell_analyzer, create_yaml_analyzer, AnalyzerConfig,
    GenericAnalyzer,
};
pub use rust_analyzer::RustAnalyzer;

/// Result of file analysis containing extracted metadata
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub language: String,
    pub classes: Vec<String>,
    pub functions: Vec<String>,
    pub imports: Vec<String>,
    pub entry_points: Vec<String>,
    pub config_keys: Vec<String>,
    pub documentation: Vec<String>,
    pub markers: Vec<String>,
    pub category: String,
    pub critical_sections: Vec<(usize, usize)>,
    /// Structure ranges for truncation (1-indexed line numbers)
    pub structure_ranges: Vec<(usize, usize)>,
}

impl AnalysisResult {
    /// Create a new empty analysis result
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            classes: Vec::new(),
            functions: Vec::new(),
            imports: Vec::new(),
            entry_points: Vec::new(),
            config_keys: Vec::new(),
            documentation: Vec::new(),
            markers: Vec::new(),
            category: "library".to_string(),
            critical_sections: Vec::new(),
            structure_ranges: Vec::new(),
        }
    }
}

/// Get the appropriate analyzer for a file based on its extension
pub fn get_analyzer_for_file(file_path: &str) -> Option<Box<dyn LanguageAnalyzer>> {
    let path = std::path::Path::new(file_path);
    let ext = path.extension()?.to_str()?;

    match ext {
        "py" | "pyw" => Some(Box::new(create_python_analyzer())),
        "js" | "jsx" | "ts" | "tsx" | "mjs" => Some(Box::new(create_javascript_analyzer())),
        "sh" | "bash" | "zsh" => Some(Box::new(create_shell_analyzer())),
        "rs" => Some(Box::new(RustAnalyzer::new())),
        "md" | "markdown" => Some(Box::new(create_markdown_analyzer())),
        "json" => Some(Box::new(create_json_analyzer())),
        "yml" | "yaml" => Some(Box::new(create_yaml_analyzer())),
        _ => None,
    }
}

/// Trait for language analyzers
pub trait LanguageAnalyzer {
    /// Analyze source code content and extract metadata
    fn analyze(&self, content: &str, file_path: &str) -> AnalysisResult;

    /// Get supported file extensions
    fn supported_extensions(&self) -> &[&str];

    /// Get language name
    fn language_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_result_new() {
        let result = AnalysisResult::new("rust");
        assert_eq!(result.language, "rust");
        assert!(result.classes.is_empty());
        assert!(result.functions.is_empty());
        assert!(result.imports.is_empty());
        assert!(result.entry_points.is_empty());
        assert!(result.config_keys.is_empty());
        assert!(result.documentation.is_empty());
        assert!(result.markers.is_empty());
        assert_eq!(result.category, "library");
        assert!(result.critical_sections.is_empty());
        assert!(result.structure_ranges.is_empty());
    }

    #[test]
    fn test_analysis_result_new_different_languages() {
        let rust = AnalysisResult::new("rust");
        let python = AnalysisResult::new("python");
        let javascript = AnalysisResult::new("javascript");

        assert_eq!(rust.language, "rust");
        assert_eq!(python.language, "python");
        assert_eq!(javascript.language, "javascript");
    }

    #[test]
    fn test_analysis_result_default_category() {
        let result = AnalysisResult::new("any");
        assert_eq!(result.category, "library");
    }

    #[test]
    fn test_get_analyzer_for_python() {
        let analyzer = get_analyzer_for_file("test.py");
        assert!(analyzer.is_some());
        let a = analyzer.unwrap();
        assert_eq!(a.language_name(), "Python");
    }

    #[test]
    fn test_get_analyzer_for_python_pyw() {
        let analyzer = get_analyzer_for_file("test.pyw");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_get_analyzer_for_javascript() {
        let analyzer = get_analyzer_for_file("test.js");
        assert!(analyzer.is_some());
        let a = analyzer.unwrap();
        assert_eq!(a.language_name(), "JavaScript");
    }

    #[test]
    fn test_get_analyzer_for_jsx() {
        let analyzer = get_analyzer_for_file("Component.jsx");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_get_analyzer_for_typescript() {
        let analyzer = get_analyzer_for_file("test.ts");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_get_analyzer_for_tsx() {
        let analyzer = get_analyzer_for_file("Component.tsx");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_get_analyzer_for_mjs() {
        let analyzer = get_analyzer_for_file("module.mjs");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_get_analyzer_for_shell() {
        let sh = get_analyzer_for_file("script.sh");
        let bash = get_analyzer_for_file("script.bash");
        let zsh = get_analyzer_for_file("script.zsh");

        assert!(sh.is_some());
        assert!(bash.is_some());
        assert!(zsh.is_some());
    }

    #[test]
    fn test_get_analyzer_for_rust() {
        let analyzer = get_analyzer_for_file("main.rs");
        assert!(analyzer.is_some());
        let a = analyzer.unwrap();
        assert_eq!(a.language_name(), "Rust");
    }

    #[test]
    fn test_get_analyzer_for_markdown() {
        let md = get_analyzer_for_file("README.md");
        let markdown = get_analyzer_for_file("doc.markdown");

        assert!(md.is_some());
        assert!(markdown.is_some());
    }

    #[test]
    fn test_get_analyzer_for_json() {
        let analyzer = get_analyzer_for_file("config.json");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_get_analyzer_for_yaml() {
        let yml = get_analyzer_for_file("config.yml");
        let yaml = get_analyzer_for_file("config.yaml");

        assert!(yml.is_some());
        assert!(yaml.is_some());
    }

    #[test]
    fn test_get_analyzer_for_unknown_extension() {
        let analyzer = get_analyzer_for_file("data.xyz");
        assert!(analyzer.is_none());
    }

    #[test]
    fn test_get_analyzer_for_no_extension() {
        let analyzer = get_analyzer_for_file("Makefile");
        assert!(analyzer.is_none());
    }

    #[test]
    fn test_get_analyzer_for_path_with_directories() {
        let analyzer = get_analyzer_for_file("src/lib/utils.rs");
        assert!(analyzer.is_some());
    }

    #[test]
    fn test_analysis_result_clone() {
        let mut original = AnalysisResult::new("rust");
        original.functions.push("main".to_string());
        original.classes.push("Config".to_string());

        let cloned = original.clone();
        assert_eq!(cloned.language, "rust");
        assert_eq!(cloned.functions, vec!["main"]);
        assert_eq!(cloned.classes, vec!["Config"]);
    }

    #[test]
    fn test_analysis_result_debug() {
        let result = AnalysisResult::new("rust");
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("rust"));
        assert!(debug_str.contains("library"));
    }
}
