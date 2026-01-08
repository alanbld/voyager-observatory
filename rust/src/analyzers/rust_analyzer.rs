use super::{AnalysisResult, LanguageAnalyzer};
use crate::python_style_split;
/// Rust source code analyzer
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref STRUCT_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?struct\s+(\w+)").unwrap();
    static ref FN_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
    static ref TRAIT_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?trait\s+(\w+)").unwrap();
    static ref IMPL_PATTERN: Regex = Regex::new(r"^\s*impl(?:\s+<[^>]+>)?\s+(\w+)").unwrap();
    static ref USE_PATTERN: Regex = Regex::new(r"^\s*use\s+([^;]+);").unwrap();
    static ref ENUM_PATTERN: Regex = Regex::new(r"^\s*(?:pub\s+)?enum\s+(\w+)").unwrap();
    static ref MARKER_PATTERN: Regex =
        Regex::new(r"//\s*(TODO|FIXME|XXX|HACK|NOTE):?\s*(.+)").unwrap();
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

    #[test]
    fn test_default_trait() {
        let analyzer = RustAnalyzer::default();
        assert_eq!(analyzer.language_name(), "Rust");
    }

    #[test]
    fn test_supported_extensions() {
        let analyzer = RustAnalyzer::new();
        assert_eq!(analyzer.supported_extensions(), &[".rs"]);
    }

    #[test]
    fn test_trait_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "pub trait Drawable {\n    fn draw(&self);\n}\ntrait Updateable {}";
        let result = analyzer.analyze(content, "traits.rs");

        assert!(result.classes.contains(&"Drawable".to_string()));
        assert!(result.classes.contains(&"Updateable".to_string()));
    }

    #[test]
    fn test_use_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "use std::io;\nuse std::collections::HashMap;\nuse crate::config::Config;";
        let result = analyzer.analyze(content, "lib.rs");

        assert!(result.imports.contains(&"std::io".to_string()));
        assert!(result
            .imports
            .contains(&"std::collections::HashMap".to_string()));
        assert!(result
            .imports
            .contains(&"crate::config::Config".to_string()));
    }

    #[test]
    fn test_marker_detection_todo() {
        let analyzer = RustAnalyzer::new();
        let content = "fn foo() {\n    // TODO: implement this\n}";
        let result = analyzer.analyze(content, "lib.rs");

        assert!(!result.markers.is_empty());
        assert!(result.markers[0].contains("TODO"));
    }

    #[test]
    fn test_marker_detection_fixme() {
        let analyzer = RustAnalyzer::new();
        let content = "fn bar() {\n    // FIXME: this is broken\n}";
        let result = analyzer.analyze(content, "lib.rs");

        assert!(!result.markers.is_empty());
        assert!(result.markers[0].contains("FIXME"));
    }

    #[test]
    fn test_marker_detection_all_types() {
        let analyzer = RustAnalyzer::new();
        let content = r#"
// TODO: task 1
// FIXME: fix this
// XXX: warning
// HACK: workaround
// NOTE: important
"#;
        let result = analyzer.analyze(content, "lib.rs");

        assert_eq!(result.markers.len(), 5);
    }

    #[test]
    fn test_category_test_file_by_name() {
        let analyzer = RustAnalyzer::new();
        let content = "fn test_something() {}";
        let result = analyzer.analyze(content, "test_module.rs");

        assert_eq!(result.category, "test");
    }

    #[test]
    fn test_category_test_file_by_path() {
        let analyzer = RustAnalyzer::new();
        let content = "fn something() {}";
        let result = analyzer.analyze(content, "tests/integration.rs");

        assert_eq!(result.category, "test");
    }

    #[test]
    fn test_category_library() {
        let analyzer = RustAnalyzer::new();
        let content = "pub fn helper() {}";
        let result = analyzer.analyze(content, "src/utils.rs");

        assert_eq!(result.category, "library");
    }

    #[test]
    fn test_async_function_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "pub async fn fetch_data() {}\nasync fn process() {}";
        let result = analyzer.analyze(content, "async.rs");

        assert!(result.functions.contains(&"fetch_data".to_string()));
        assert!(result.functions.contains(&"process".to_string()));
    }

    #[test]
    fn test_pub_enum_detection() {
        let analyzer = RustAnalyzer::new();
        let content = "pub enum Color {\n    Red,\n    Green,\n    Blue,\n}";
        let result = analyzer.analyze(content, "colors.rs");

        assert!(result.classes.contains(&"Color".to_string()));
    }

    #[test]
    fn test_critical_sections_for_main() {
        let analyzer = RustAnalyzer::new();
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let result = analyzer.analyze(content, "main.rs");

        assert!(!result.critical_sections.is_empty());
        assert_eq!(result.critical_sections[0].0, 1); // starts at line 1
    }

    #[test]
    fn test_function_limit_20() {
        let analyzer = RustAnalyzer::new();
        let mut content = String::new();
        for i in 0..25 {
            content.push_str(&format!("fn func{}() {{}}\n", i));
        }
        let result = analyzer.analyze(&content, "many_funcs.rs");

        assert_eq!(result.functions.len(), 20);
    }

    #[test]
    fn test_imports_limit_10() {
        let analyzer = RustAnalyzer::new();
        let mut content = String::new();
        for i in 0..15 {
            content.push_str(&format!("use crate::module{};\n", i));
        }
        let result = analyzer.analyze(&content, "imports.rs");

        assert_eq!(result.imports.len(), 10);
    }

    #[test]
    fn test_markers_limit_5() {
        let analyzer = RustAnalyzer::new();
        let mut content = String::new();
        for i in 0..10 {
            content.push_str(&format!("// TODO: task {}\n", i));
        }
        let result = analyzer.analyze(&content, "tasks.rs");

        assert_eq!(result.markers.len(), 5);
    }

    #[test]
    fn test_empty_content() {
        let analyzer = RustAnalyzer::new();
        let result = analyzer.analyze("", "empty.rs");

        assert_eq!(result.language, "Rust");
        assert!(result.classes.is_empty());
        assert!(result.functions.is_empty());
        assert!(result.imports.is_empty());
        assert_eq!(result.category, "library");
    }

    #[test]
    fn test_combined_analysis() {
        let analyzer = RustAnalyzer::new();
        let content = r#"
use std::io;
use crate::config;

pub struct App {
    name: String,
}

pub enum State {
    Running,
    Stopped,
}

pub trait Service {
    fn start(&self);
}

impl App {
    pub fn new() -> Self {
        // TODO: initialize properly
        Self { name: String::new() }
    }
}

fn main() {
    println!("Starting");
}
"#;
        let result = analyzer.analyze(content, "app.rs");

        assert_eq!(result.language, "Rust");
        assert!(result.classes.contains(&"App".to_string()));
        assert!(result.classes.contains(&"State".to_string()));
        assert!(result.classes.contains(&"Service".to_string()));
        assert!(result.functions.contains(&"main".to_string()));
        assert!(result.functions.contains(&"new".to_string()));
        assert!(result.imports.contains(&"std::io".to_string()));
        assert!(!result.markers.is_empty());
        assert_eq!(result.category, "application");
        assert!(!result.critical_sections.is_empty());
    }

    #[test]
    fn test_marker_with_colon() {
        let analyzer = RustAnalyzer::new();
        let content = "// TODO: implement feature";
        let result = analyzer.analyze(content, "lib.rs");

        assert!(result.markers[0].contains("TODO"));
    }

    #[test]
    fn test_marker_without_colon() {
        let analyzer = RustAnalyzer::new();
        let content = "// TODO implement feature";
        let result = analyzer.analyze(content, "lib.rs");

        assert!(result.markers[0].contains("TODO"));
    }

    #[test]
    fn test_analyze_result_new() {
        let result = AnalysisResult::new("TestLang");
        assert_eq!(result.language, "TestLang");
        assert!(result.classes.is_empty());
        assert!(result.functions.is_empty());
        assert!(result.imports.is_empty());
        assert!(result.entry_points.is_empty());
        assert!(result.markers.is_empty());
        assert!(result.critical_sections.is_empty());
    }

    #[test]
    fn test_language_name() {
        let analyzer = RustAnalyzer::new();
        assert_eq!(analyzer.language_name(), "Rust");
    }

    #[test]
    fn test_struct_with_generics() {
        let analyzer = RustAnalyzer::new();
        let content = "pub struct Container<T> {\n    data: T,\n}";
        let result = analyzer.analyze(content, "container.rs");

        // Should capture "Container" not "Container<T>"
        assert!(result.classes.iter().any(|c| c.starts_with("Container")));
    }

    #[test]
    fn test_impl_pattern() {
        let analyzer = RustAnalyzer::new();
        let content = "impl MyStruct {\n    fn method(&self) {}\n}";
        let result = analyzer.analyze(content, "impl.rs");

        // impl blocks detected but not added to classes
        assert!(result.functions.contains(&"method".to_string()));
    }

    #[test]
    fn test_impl_with_generics() {
        let analyzer = RustAnalyzer::new();
        let content = "impl<T> Container<T> {\n    fn new() -> Self {}\n}";
        let result = analyzer.analyze(content, "container.rs");

        assert!(result.functions.contains(&"new".to_string()));
    }
}
