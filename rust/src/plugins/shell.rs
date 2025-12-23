//! Shell Language Plugin
//!
//! Provides analysis for shell scripts including:
//! - Bash, sh, zsh, ksh dialect detection
//! - Function extraction
//! - Variable and export detection
//! - Source/import tracking
//! - Command analysis

use std::collections::HashMap;

use regex::Regex;

use crate::core::fractal::{ExtractedSymbol, Import, Range, SymbolKind, Visibility};

use super::{FileInfo, LanguagePlugin, PluginResult};

// =============================================================================
// Shell Dialect
// =============================================================================

/// Shell dialect variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellDialect {
    #[default]
    Sh,
    Bash,
    Zsh,
    Ksh,
    Fish,
}

impl ShellDialect {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShellDialect::Sh => "sh",
            ShellDialect::Bash => "bash",
            ShellDialect::Zsh => "zsh",
            ShellDialect::Ksh => "ksh",
            ShellDialect::Fish => "fish",
        }
    }

    pub fn from_shebang(shebang: &str) -> Self {
        let shebang_lower = shebang.to_lowercase();
        if shebang_lower.contains("bash") {
            ShellDialect::Bash
        } else if shebang_lower.contains("zsh") {
            ShellDialect::Zsh
        } else if shebang_lower.contains("ksh") {
            ShellDialect::Ksh
        } else if shebang_lower.contains("fish") {
            ShellDialect::Fish
        } else {
            ShellDialect::Sh
        }
    }
}

// =============================================================================
// Shell Plugin
// =============================================================================

/// Plugin for analyzing shell scripts.
pub struct ShellPlugin {
    /// Pattern for function declarations: `name() {` or `function name {`
    function_pattern: Regex,
    /// Pattern for variables: `NAME=value`
    variable_pattern: Regex,
    /// Pattern for exports: `export NAME=value`
    export_pattern: Regex,
    /// Pattern for source/dot commands: `source file` or `. file`
    source_pattern: Regex,
    /// Pattern for shebang: `#!/bin/bash`
    shebang_pattern: Regex,
    /// Pattern for local variables: `local NAME=value`
    local_pattern: Regex,
    /// Pattern for readonly variables: `readonly NAME=value`
    readonly_pattern: Regex,
}

impl ShellPlugin {
    pub fn new() -> Self {
        Self {
            // Function: name() { or function name or function name()
            function_pattern: Regex::new(
                r"(?m)^[ \t]*(?:function\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(\s*\)\s*\{|^[ \t]*function\s+([a-zA-Z_][a-zA-Z0-9_]*)(?:\s*\(\s*\))?\s*\{"
            ).unwrap(),
            // Variable assignment: NAME=value (not local, not export)
            variable_pattern: Regex::new(
                r"(?m)^[ \t]*([A-Z_][A-Z0-9_]*)=(.*)$"
            ).unwrap(),
            // Export: export NAME=value or export NAME
            export_pattern: Regex::new(
                r"(?m)^[ \t]*export\s+([A-Z_][A-Z0-9_]*)(?:=(.*))?$"
            ).unwrap(),
            // Source command: source file or . file
            source_pattern: Regex::new(
                r#"(?m)^[ \t]*(?:source|\.)\s+["']?([^"'\s]+)["']?"#
            ).unwrap(),
            // Shebang: #!/bin/bash or #!/usr/bin/env bash
            shebang_pattern: Regex::new(
                r"^#!\s*(?:/usr/bin/env\s+)?(?:/(?:usr/)?(?:local/)?bin/)?(\w+)"
            ).unwrap(),
            // Local variable: local NAME=value
            local_pattern: Regex::new(
                r"(?m)^[ \t]*local\s+([a-zA-Z_][a-zA-Z0-9_]*)(?:=(.*))?$"
            ).unwrap(),
            // Readonly: readonly NAME=value
            readonly_pattern: Regex::new(
                r"(?m)^[ \t]*readonly\s+([A-Z_][A-Z0-9_]*)(?:=(.*))?$"
            ).unwrap(),
        }
    }

    /// Detect the shell dialect from content.
    pub fn detect_dialect(&self, content: &str) -> ShellDialect {
        // Check shebang first
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                return ShellDialect::from_shebang(first_line);
            }
        }

        // Heuristic detection based on syntax
        if content.contains("[[ ") || content.contains("$((") {
            ShellDialect::Bash
        } else if content.contains("#compdef") || content.contains("autoload") {
            ShellDialect::Zsh
        } else if content.contains("typeset") && content.contains("integer") {
            ShellDialect::Ksh
        } else if content.contains("set -e") || content.contains("set -u") {
            ShellDialect::Sh // POSIX-style
        } else {
            ShellDialect::Sh
        }
    }

    /// Extract doc comment above a line.
    fn extract_doc_comment(&self, lines: &[&str], line_num: usize) -> Option<String> {
        let mut docs = Vec::new();
        let mut idx = line_num.saturating_sub(1);

        while idx > 0 {
            let line = lines.get(idx)?;
            let trimmed = line.trim();

            if trimmed.starts_with('#') && !trimmed.starts_with("#!") {
                docs.push(trimmed.trim_start_matches('#').trim());
                idx = idx.saturating_sub(1);
            } else if trimmed.is_empty() {
                idx = idx.saturating_sub(1);
            } else {
                break;
            }
        }

        if docs.is_empty() {
            None
        } else {
            docs.reverse();
            Some(docs.join(" "))
        }
    }

    /// Check if a line is inside a function body.
    fn is_in_function(&self, _content: &str, _line_num: usize) -> bool {
        // Simplified: would need brace matching for accuracy
        false
    }
}

impl Default for ShellPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePlugin for ShellPlugin {
    fn language_name(&self) -> &'static str {
        "shell"
    }

    fn extensions(&self) -> &[&'static str] {
        &["sh", "bash", "zsh", "ksh", "fish"]
    }

    fn extract_symbols(&self, content: &str) -> PluginResult<Vec<ExtractedSymbol>> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Extract functions
        for cap in self.function_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            // Get function name from either group
            let name = cap
                .get(1)
                .or_else(|| cap.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");

            if name.is_empty() {
                continue;
            }

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!("{}()", name),
                return_type: None,
                parameters: Vec::new(),
                documentation: self.extract_doc_comment(&lines, start_line),
                visibility: Visibility::Public,
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract exported variables
        for cap in self.export_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value = cap.get(2).map(|m| m.as_str());

            if name.is_empty() {
                continue;
            }

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Variable,
                signature: if let Some(v) = value {
                    format!("export {}={}", name, v)
                } else {
                    format!("export {}", name)
                },
                return_type: Some("string".to_string()),
                parameters: Vec::new(),
                documentation: self.extract_doc_comment(&lines, start_line),
                visibility: Visibility::Public,
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        // Extract readonly constants
        for cap in self.readonly_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let start_line = content[..full_match.start()].lines().count();

            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value = cap.get(2).map(|m| m.as_str());

            if name.is_empty() {
                continue;
            }

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Constant,
                signature: if let Some(v) = value {
                    format!("readonly {}={}", name, v)
                } else {
                    format!("readonly {}", name)
                },
                return_type: Some("string".to_string()),
                parameters: Vec::new(),
                documentation: self.extract_doc_comment(&lines, start_line),
                visibility: Visibility::Public,
                range: Range::single_line(start_line + 1),
                calls: Vec::new(),
            });
        }

        Ok(symbols)
    }

    fn extract_imports(&self, content: &str) -> PluginResult<Vec<Import>> {
        let mut imports = Vec::new();

        for cap in self.source_pattern.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let line_num = content[..full_match.start()].lines().count() + 1;

            let file = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            if !file.is_empty() {
                imports.push(Import {
                    module: file.to_string(),
                    items: Vec::new(),
                    alias: None,
                    line: line_num,
                });
            }
        }

        Ok(imports)
    }

    fn file_info(&self, content: &str) -> PluginResult<FileInfo> {
        let dialect = self.detect_dialect(content);
        let symbols = self.extract_symbols(content)?;
        let line_count = content.lines().count();

        // Check if it's a test file
        let is_test = content.contains("@test") // bats
            || content.contains("shunit2")
            || content.contains("assert_")
            || content.contains("test_");

        // Check for executable patterns
        let is_executable = content.starts_with("#!")
            || content.contains("main()")
            || content.contains("#!/");

        let mut metadata = HashMap::new();

        // Extract shebang interpreter
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                if let Some(cap) = self.shebang_pattern.captures(first_line) {
                    if let Some(interp) = cap.get(1) {
                        metadata.insert("interpreter".to_string(), interp.as_str().to_string());
                    }
                }
            }
        }

        Ok(FileInfo {
            language: "shell".to_string(),
            dialect: Some(dialect.as_str().to_string()),
            symbol_count: symbols.len(),
            line_count,
            is_test,
            is_executable,
            metadata,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin() -> ShellPlugin {
        ShellPlugin::new()
    }

    // =========================================================================
    // Dialect Detection Tests
    // =========================================================================

    #[test]
    fn test_detect_bash_from_shebang() {
        let content = "#!/bin/bash\necho hello";
        assert_eq!(plugin().detect_dialect(content), ShellDialect::Bash);
    }

    #[test]
    fn test_detect_zsh_from_shebang() {
        let content = "#!/usr/bin/env zsh\necho hello";
        assert_eq!(plugin().detect_dialect(content), ShellDialect::Zsh);
    }

    #[test]
    fn test_detect_sh_from_shebang() {
        let content = "#!/bin/sh\necho hello";
        assert_eq!(plugin().detect_dialect(content), ShellDialect::Sh);
    }

    #[test]
    fn test_detect_bash_from_syntax() {
        let content = "if [[ -f file ]]; then\n  echo exists\nfi";
        assert_eq!(plugin().detect_dialect(content), ShellDialect::Bash);
    }

    #[test]
    fn test_detect_zsh_from_syntax() {
        let content = "#compdef mycommand\nautoload -Uz compinit";
        assert_eq!(plugin().detect_dialect(content), ShellDialect::Zsh);
    }

    // =========================================================================
    // Function Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_function_parens_style() {
        let content = r#"
hello() {
    echo "Hello"
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_function_keyword_style() {
        let content = r#"
function greet {
    echo "Hi"
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
    }

    #[test]
    fn test_extract_function_keyword_with_parens() {
        let content = r#"
function process() {
    echo "Processing"
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "process");
    }

    #[test]
    fn test_extract_multiple_functions() {
        let content = r#"
deploy() {
    echo "Deploying..."
}

test() {
    echo "Testing..."
}

cleanup() {
    docker system prune -f
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 3);
    }

    #[test]
    fn test_extract_function_with_doc_comment() {
        let content = r#"
# Deploy the application to production
# This handles building and pushing
deploy() {
    docker build -t app .
}
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].documentation.is_some());
        assert!(symbols[0]
            .documentation
            .as_ref()
            .unwrap()
            .contains("Deploy"));
    }

    // =========================================================================
    // Export/Variable Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_export() {
        let content = r#"
export PATH="/usr/local/bin:$PATH"
export HOME
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 2);
        assert!(symbols.iter().any(|s| s.name == "PATH"));
        assert!(symbols.iter().any(|s| s.name == "HOME"));
    }

    #[test]
    fn test_extract_readonly() {
        let content = r#"
readonly VERSION="1.0.0"
readonly CONFIG_FILE="/etc/app.conf"
"#;
        let symbols = plugin().extract_symbols(content).unwrap();
        assert_eq!(symbols.len(), 2);
        assert!(symbols.iter().all(|s| s.kind == SymbolKind::Constant));
    }

    // =========================================================================
    // Import/Source Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_source_command() {
        let content = r#"
source /etc/profile
. ~/.bashrc
source "./lib/utils.sh"
"#;
        let imports = plugin().extract_imports(content).unwrap();
        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|i| i.module.contains("profile")));
        assert!(imports.iter().any(|i| i.module.contains("bashrc")));
        assert!(imports.iter().any(|i| i.module.contains("utils")));
    }

    // =========================================================================
    // File Info Tests
    // =========================================================================

    #[test]
    fn test_file_info_bash() {
        let content = r#"#!/bin/bash
set -euo pipefail

deploy() {
    echo "Deploying..."
}
"#;
        let info = plugin().file_info(content).unwrap();
        assert_eq!(info.language, "shell");
        assert_eq!(info.dialect, Some("bash".to_string()));
        assert_eq!(info.symbol_count, 1);
        assert!(info.is_executable);
    }

    #[test]
    fn test_file_info_test_file() {
        let content = r#"#!/bin/bash
# Test suite

test_deployment() {
    assert_equal "expected" "actual"
}
"#;
        let info = plugin().file_info(content).unwrap();
        assert!(info.is_test);
    }

    #[test]
    fn test_file_info_metadata() {
        let content = "#!/usr/bin/env bash\necho hello";
        let info = plugin().file_info(content).unwrap();
        assert_eq!(info.metadata.get("interpreter"), Some(&"bash".to_string()));
    }

    // =========================================================================
    // Plugin Interface Tests
    // =========================================================================

    #[test]
    fn test_language_name() {
        assert_eq!(plugin().language_name(), "shell");
    }

    #[test]
    fn test_extensions() {
        let p = plugin();
        let exts = p.extensions();
        assert!(exts.contains(&"sh"));
        assert!(exts.contains(&"bash"));
        assert!(exts.contains(&"zsh"));
    }

    #[test]
    fn test_supports_file() {
        let p = plugin();
        assert!(p.supports_file(std::path::Path::new("script.sh")));
        assert!(p.supports_file(std::path::Path::new("deploy.bash")));
        assert!(p.supports_file(std::path::Path::new("init.zsh")));
        assert!(!p.supports_file(std::path::Path::new("main.rs")));
        assert!(!p.supports_file(std::path::Path::new("app.py")));
    }
}
