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
