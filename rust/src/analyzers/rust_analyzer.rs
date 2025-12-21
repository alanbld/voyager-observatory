/// Rust source code analyzer
use lazy_static::lazy_static;
use regex::Regex;
use super::{AnalysisResult, LanguageAnalyzer};
use crate::python_style_split;

lazy_static! {
    static ref STRUCT_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?struct\s+(\w+)").unwrap();
    static ref FN_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
    static ref TRAIT_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?trait\s+(\w+)").unwrap();
    static ref IMPL_PATTERN: Regex = Regex::new(r"^\s*impl(?:\s+<[^>]+>)?\s+(\w+)").unwrap();
    static ref USE_PATTERN: Regex = Regex::new(r"^\s*use\s+([^;]+);").unwrap();
    static ref ENUM_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?enum\s+(\w+)").unwrap();
    static ref MARKER_PATTERN: Regex = Regex::new(r"//\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)").unwrap();
}

pub struct RustAnalyzer;

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze Rust source code lines
    fn analyze_lines(&self, lines: &[&str], file_path: &str) -> AnalysisResult {
        let mut result = AnalysisResult::new("Rust");
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();
        let mut traits = Vec::new();
        let mut uses = Vec::new();
        let mut entry_points = Vec::new();
        let mut markers = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;

            // Structs
            if let Some(caps) = STRUCT_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    structs.push(name.as_str().to_string());
                }
            }

            // Enums
            if let Some(caps) = ENUM_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    enums.push(name.as_str().to_string());
                }
            }

            // Functions
            if let Some(caps) = FN_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    let fn_name = name.as_str().to_string();
                    functions.push(fn_name.clone());

                    // Check for main entry point
                    if fn_name == "main" {
                        entry_points.push("fn main".to_string());
                        result.critical_sections.push((line_num, line_num + 20));
                    }
                }
            }

            // Traits
            if let Some(caps) = TRAIT_PATTERN.captures(line) {
                if let Some(name) = caps.get(1) {
                    traits.push(name.as_str().to_string());
                }
            }

            // Uses
            if let Some(caps) = USE_PATTERN.captures(line) {
                if let Some(use_stmt) = caps.get(1) {
                    uses.push(use_stmt.as_str().trim().to_string());
                }
            }

            // Markers (TODO, FIXME, etc.)
            if let Some(caps) = MARKER_PATTERN.captures(line) {
                if let (Some(marker_type), Some(_marker_text)) = (caps.get(1), caps.get(2)) {
                    markers.push(format!("{} (line {})", marker_type.as_str(), line_num));
                }
            }
        }

        // Categorize based on content
        let category = if functions.contains(&"main".to_string()) {
            "application"
        } else if file_path.to_lowercase().contains("test") || file_path.contains("tests/") {
            "test"
        } else {
            "library"
        };

        // Populate result
        // Classes = structs + traits + enums (combining all type definitions)
        result.classes.extend(structs);
        result.classes.extend(traits);
        result.classes.extend(enums);

        // Limit to first 20 functions
        result.functions = functions.into_iter().take(20).collect();

        // Limit to first 10 imports
        result.imports = uses.into_iter().take(10).collect();

        result.entry_points = entry_points;

        // Limit to first 5 markers
        result.markers = markers.into_iter().take(5).collect();

        result.category = category.to_string();

        result
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn analyze(&self, content: &str, file_path: &str) -> AnalysisResult {
        let lines: Vec<&str> = python_style_split(content);
        self.analyze_lines(&lines, file_path)
    }

    fn supported_extensions(&self) -> &[&str] {
        &[".rs"]
    }

    fn language_name(&self) -> &str {
        "Rust"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "pub struct User {\n    name: String,\n}\nstruct Config;";
        let result = analyzer.analyze(content, "test.rs");

        assert_eq!(result.language, "Rust");
        assert!(result.classes.contains(&"User".to_string()));
        assert!(result.classes.contains(&"Config".to_string()));
    }

    #[test]
    fn test_function_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "fn calculate() {}\npub fn process() {}\nfn main() {}";
        let result = analyzer.analyze(content, "main.rs");

        assert!(result.functions.contains(&"calculate".to_string()));
        assert!(result.functions.contains(&"process".to_string()));
        assert!(result.functions.contains(&"main".to_string()));
        assert_eq!(result.entry_points, vec!["fn main"]);
        assert_eq!(result.category, "application");
    }

    #[test]
    fn test_enum_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "enum Status {\n    Active,\n    Inactive,\n}";
        let result = analyzer.analyze(content, "types.rs");

        assert!(result.classes.contains(&"Status".to_string()));
    }
}
