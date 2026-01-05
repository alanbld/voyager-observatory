//! Universal Spectrograph - Spectral Signatures for 80+ Languages
//!
//! This module provides the core pattern repository for language analysis.
//! It is always available (not gated behind the plugins feature) because
//! it powers the Celestial Census fallback pattern matching.
//!
//! For the Lua plugin API (vo.patterns.*), see `plugins::bridges::patterns`.
//!
//! # The Stellar Library
//!
//! Languages are organized into tiers based on their place in the code galaxy:
//! - **Tier 1: The Giants** - Modern mainstream languages
//! - **Tier 2: Infrastructure** - Automation and configuration
//! - **Tier 3: Ancient Stars** - Legacy languages (COBOL, Simula, Logo)
//! - **Tier 4: Functional** - Functional and logic paradigms
//! - **Tier 5: Stellar Nurseries** - Emerging languages
//! - **Tier 6: Scientific** - Domain-specific and scripting
//!
//! # Pattern Types
//!
//! Each language has two pattern categories:
//! - **Stars**: Function/method/class/procedure definitions
//! - **Nebulae**: Comment blocks (single-line and multi-line)

use std::collections::HashMap;

// =============================================================================
// STELLAR LIBRARY - Core Pattern Repository
// =============================================================================

/// A spectral signature for a programming language
#[derive(Debug, Clone)]
pub struct SpectralSignature {
    /// Pattern to find function/class/procedure definitions (Stars)
    pub star_pattern: &'static str,
    /// Pattern to find single-line comments
    pub comment_single: &'static str,
    /// Pattern to find multi-line comment start
    pub comment_multi_start: &'static str,
    /// Pattern to find multi-line comment end
    pub comment_multi_end: &'static str,
    /// Language category for Mission Log
    pub hemisphere: Hemisphere,
    /// Human-readable language name
    pub display_name: &'static str,
    /// File extensions
    pub extensions: &'static [&'static str],
}

impl SpectralSignature {
    /// Count lines in source content, returning (total, code, comments, blanks)
    ///
    /// This provides tokei-style line counting using the language's comment patterns.
    pub fn count_lines(&self, content: &str) -> (usize, usize, usize, usize) {
        let mut total = 0;
        let mut code = 0;
        let mut comments = 0;
        let mut blanks = 0;
        let mut in_multiline_comment = false;

        for line in content.lines() {
            total += 1;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                blanks += 1;
                continue;
            }

            // Check for multi-line comment state
            if in_multiline_comment {
                comments += 1;
                // Check if this line ends the multi-line comment
                if !self.comment_multi_end.is_empty() && trimmed.contains(self.comment_multi_end) {
                    in_multiline_comment = false;
                }
                continue;
            }

            // Check for start of multi-line comment
            if !self.comment_multi_start.is_empty() && trimmed.contains(self.comment_multi_start) {
                comments += 1;
                // Check if it also ends on the same line
                if self.comment_multi_end.is_empty() || !trimmed.contains(self.comment_multi_end) {
                    in_multiline_comment = true;
                }
                continue;
            }

            // Check for single-line comment
            if !self.comment_single.is_empty() && trimmed.starts_with(self.comment_single) {
                comments += 1;
                continue;
            }

            // It's code
            code += 1;
        }

        (total, code, comments, blanks)
    }
}

/// Language hemisphere classification for Mission Log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hemisphere {
    /// Logic/backend (functions, algorithms)
    Logic,
    /// Interface/frontend (UI, markup)
    Interface,
    /// Automation/DevOps (scripts, configs)
    Automation,
    /// Data/Schema (queries, definitions)
    Data,
}

impl Hemisphere {
    pub fn as_str(&self) -> &'static str {
        match self {
            Hemisphere::Logic => "Logic",
            Hemisphere::Interface => "Interface",
            Hemisphere::Automation => "Automation",
            Hemisphere::Data => "Data",
        }
    }
}

/// The Universal Spectrograph - contains all spectral signatures
pub struct StellarLibrary {
    signatures: HashMap<&'static str, SpectralSignature>,
}

impl StellarLibrary {
    /// Create a new Stellar Library with all 80+ language signatures
    pub fn new() -> Self {
        let mut signatures = HashMap::new();

        // =========================================================================
        // TIER 1: THE GIANTS (Modern Core)
        // =========================================================================

        signatures.insert("rust", SpectralSignature {
            star_pattern: r#"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Rust",
            extensions: &["rs"],
        });

        signatures.insert("python", SpectralSignature {
            star_pattern: r#"(?:async\s+)?def\s+(\w+)|class\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"(?:'''|""")"#,
            comment_multi_end: r#"(?:'''|""")"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Python",
            extensions: &["py", "pyw", "pyi"],
        });

        signatures.insert("javascript", SpectralSignature {
            star_pattern: r#"(?:async\s+)?function\s+(\w+)|class\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "JavaScript",
            extensions: &["js", "mjs", "cjs", "jsx"],
        });

        signatures.insert("typescript", SpectralSignature {
            star_pattern: r#"(?:export\s+)?(?:async\s+)?function\s+(\w+)|(?:export\s+)?class\s+(\w+)|(?:export\s+)?interface\s+(\w+)|(?:export\s+)?type\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "TypeScript",
            extensions: &["ts", "tsx", "mts", "cts"],
        });

        signatures.insert("java", SpectralSignature {
            star_pattern: r#"(?:public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(?:class|interface|enum)\s+(\w+)|(?:public|private|protected)?\s*(?:static\s+)?(?:\w+\s+)+(\w+)\s*\("#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Java",
            extensions: &["java"],
        });

        signatures.insert("csharp", SpectralSignature {
            star_pattern: r#"(?:public|private|protected|internal)?\s*(?:static\s+)?(?:partial\s+)?(?:class|interface|struct|enum|record)\s+(\w+)|(?:public|private|protected|internal)?\s*(?:static\s+)?(?:async\s+)?(?:\w+\s+)+(\w+)\s*\("#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "C#",
            extensions: &["cs", "csx"],
        });

        signatures.insert("cpp", SpectralSignature {
            star_pattern: r#"(?:class|struct|enum)\s+(\w+)|(?:\w+\s+)+(\w+)\s*\([^)]*\)\s*(?:const\s*)?(?:override\s*)?(?:final\s*)?\{"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "C++",
            extensions: &["cpp", "cxx", "cc", "c++", "hpp", "hxx", "h++", "hh"],
        });

        signatures.insert("c", SpectralSignature {
            star_pattern: r#"(?:struct|enum|union)\s+(\w+)|(?:\w+\s+)+(\w+)\s*\([^)]*\)\s*\{"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "C",
            extensions: &["c", "h"],
        });

        signatures.insert("php", SpectralSignature {
            star_pattern: r#"(?:public|private|protected)?\s*(?:static\s+)?function\s+(\w+)|class\s+(\w+)|interface\s+(\w+)|trait\s+(\w+)"#,
            comment_single: r#"(?://|#)"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "PHP",
            extensions: &["php", "phtml", "php3", "php4", "php5", "phps"],
        });

        signatures.insert("go", SpectralSignature {
            star_pattern: r#"func\s+(?:\([^)]+\)\s+)?(\w+)|type\s+(\w+)\s+(?:struct|interface)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Go",
            extensions: &["go"],
        });

        signatures.insert("ruby", SpectralSignature {
            star_pattern: r#"def\s+(\w+[!?]?)|class\s+(\w+)|module\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"=begin"#,
            comment_multi_end: r#"=end"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Ruby",
            extensions: &["rb", "rake", "gemspec"],
        });

        signatures.insert("kotlin", SpectralSignature {
            star_pattern: r#"(?:suspend\s+)?fun\s+(\w+)|class\s+(\w+)|interface\s+(\w+)|object\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Kotlin",
            extensions: &["kt", "kts"],
        });

        signatures.insert("swift", SpectralSignature {
            star_pattern: r#"func\s+(\w+)|class\s+(\w+)|struct\s+(\w+)|enum\s+(\w+)|protocol\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Swift",
            extensions: &["swift"],
        });

        signatures.insert("dart", SpectralSignature {
            star_pattern: r#"(?:Future\s+)?(\w+)\s+(\w+)\s*\(|class\s+(\w+)|mixin\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Dart",
            extensions: &["dart"],
        });

        // =========================================================================
        // TIER 2: INFRASTRUCTURE & AUTOMATION
        // =========================================================================

        signatures.insert("html", SpectralSignature {
            star_pattern: r#"<(\w+)[^>]*>"#,
            comment_single: r#"$^"#, // No single-line comments in HTML
            comment_multi_start: r#"<!--"#,
            comment_multi_end: r#"-->"#,
            hemisphere: Hemisphere::Interface,
            display_name: "HTML",
            extensions: &["html", "htm", "xhtml"],
        });

        signatures.insert("css", SpectralSignature {
            star_pattern: r#"([.#]?\w+(?:-\w+)*)\s*\{"#,
            comment_single: r#"$^"#, // No single-line comments in CSS
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Interface,
            display_name: "CSS",
            extensions: &["css"],
        });

        signatures.insert("scss", SpectralSignature {
            star_pattern: r#"@mixin\s+(\w+)|@function\s+(\w+)|([.#]?\w+(?:-\w+)*)\s*\{"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Interface,
            display_name: "SCSS",
            extensions: &["scss", "sass"],
        });

        signatures.insert("sql", SpectralSignature {
            star_pattern: r#"CREATE\s+(?:OR\s+REPLACE\s+)?(?:TABLE|VIEW|FUNCTION|PROCEDURE|TRIGGER)\s+(?:\w+\.)?(\w+)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Data,
            display_name: "SQL",
            extensions: &["sql"],
        });

        signatures.insert("json", SpectralSignature {
            star_pattern: r#""(\w+)"\s*:"#,
            comment_single: r#"$^"#, // No comments in JSON
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "JSON",
            extensions: &["json", "jsonc"],
        });

        signatures.insert("yaml", SpectralSignature {
            star_pattern: r#"^(\w+):"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "YAML",
            extensions: &["yaml", "yml"],
        });

        signatures.insert("xml", SpectralSignature {
            star_pattern: r#"<(\w+)[^>]*>"#,
            comment_single: r#"$^"#,
            comment_multi_start: r#"<!--"#,
            comment_multi_end: r#"-->"#,
            hemisphere: Hemisphere::Data,
            display_name: "XML",
            extensions: &["xml", "xsl", "xslt", "xsd"],
        });

        signatures.insert("markdown", SpectralSignature {
            star_pattern: r#"^#{1,6}\s+(.+)$"#,
            comment_single: r#"$^"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Interface,
            display_name: "Markdown",
            extensions: &["md", "markdown", "mdown"],
        });

        signatures.insert("hcl", SpectralSignature {
            star_pattern: r#"(?:resource|data|variable|output|module|provider)\s+"(\w+)""#,
            comment_single: r#"(?://|#)"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Automation,
            display_name: "HCL (Terraform)",
            extensions: &["tf", "tfvars", "hcl"],
        });

        signatures.insert("makefile", SpectralSignature {
            star_pattern: r#"^(\w+)\s*:"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Automation,
            display_name: "Makefile",
            extensions: &["makefile", "mk"],
        });

        signatures.insert("dockerfile", SpectralSignature {
            star_pattern: r#"^(FROM|RUN|CMD|ENTRYPOINT|COPY|ADD|ENV|EXPOSE|WORKDIR|LABEL)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Automation,
            display_name: "Dockerfile",
            extensions: &["dockerfile"],
        });

        signatures.insert("nix", SpectralSignature {
            star_pattern: r#"(\w+)\s*=\s*(?:\{|let|rec)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Automation,
            display_name: "Nix",
            extensions: &["nix"],
        });

        signatures.insert("powershell", SpectralSignature {
            star_pattern: r#"function\s+(\w+(?:-\w+)*)|class\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"<#"#,
            comment_multi_end: r#"#>"#,
            hemisphere: Hemisphere::Automation,
            display_name: "PowerShell",
            extensions: &["ps1", "psm1", "psd1"],
        });

        signatures.insert("bash", SpectralSignature {
            star_pattern: r#"(?:function\s+)?(\w+)\s*\(\s*\)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Automation,
            display_name: "Bash",
            extensions: &["sh", "bash", "zsh"],
        });

        signatures.insert("tcl", SpectralSignature {
            star_pattern: r#"proc\s+([^\s\{]+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Automation,
            display_name: "Tcl",
            extensions: &["tcl", "tk"],
        });

        signatures.insert("fish", SpectralSignature {
            star_pattern: r#"function\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Automation,
            display_name: "Fish",
            extensions: &["fish"],
        });

        // =========================================================================
        // TIER 3: THE ANCIENT STARS (Legacy Kings)
        // =========================================================================

        signatures.insert("cobol", SpectralSignature {
            star_pattern: r#"(?:PROCEDURE|SECTION|PARAGRAPH)\s+(\w+(?:-\w+)*)|(\w+(?:-\w+)*)\s+SECTION\.|(\d{2,4})-(\w+(?:-\w+)*)\."#,
            comment_single: r#"^\*"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "COBOL",
            extensions: &["cob", "cbl", "cpy", "cobol"],
        });

        signatures.insert("simula", SpectralSignature {
            star_pattern: r#"(?:procedure|class)\s+(\w+)"#,
            comment_single: r#"!"#,
            comment_multi_start: r#"comment"#,
            comment_multi_end: r#";"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Simula",
            extensions: &["sim"],
        });

        signatures.insert("logo", SpectralSignature {
            star_pattern: r#"to\s+(\w+)"#,
            comment_single: r#";"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Logo",
            extensions: &["logo", "lg"],
        });

        signatures.insert("fortran", SpectralSignature {
            star_pattern: r#"(?:subroutine|function|program|module)\s+(\w+)"#,
            comment_single: r#"!"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Fortran",
            extensions: &["f", "for", "f90", "f95", "f03", "f08"],
        });

        signatures.insert("ada", SpectralSignature {
            star_pattern: r#"(?:procedure|function|package|task)\s+(\w+)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Ada",
            extensions: &["ada", "adb", "ads"],
        });

        signatures.insert("pascal", SpectralSignature {
            star_pattern: r#"(?:procedure|function|program|unit)\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"(?:\{|\(\*)"#,
            comment_multi_end: r#"(?:\}|\*\))"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Pascal",
            extensions: &["pas", "pp", "p"],
        });

        signatures.insert("delphi", SpectralSignature {
            star_pattern: r#"(?:procedure|function|class|unit)\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"(?:\{|\(\*)"#,
            comment_multi_end: r#"(?:\}|\*\))"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Delphi",
            extensions: &["dpr", "dpk", "dfm"],
        });

        signatures.insert("lisp", SpectralSignature {
            star_pattern: r#"\(def(?:un|macro|var|parameter|constant)\s+(\w+(?:-\w+)*)"#,
            comment_single: r#";"#,
            comment_multi_start: r#"#\|"#,
            comment_multi_end: r#"\|#"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Lisp",
            extensions: &["lisp", "lsp", "cl"],
        });

        signatures.insert("prolog", SpectralSignature {
            star_pattern: r#"(\w+)\s*\([^)]*\)\s*:-"#,
            comment_single: r#"%"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Prolog",
            extensions: &["pl", "pro", "prolog"],
        });

        signatures.insert("rpg", SpectralSignature {
            star_pattern: r#"(?:DCL-PROC|BEGSR)\s+(\w+)"#,
            comment_single: r#"\*"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "RPG",
            extensions: &["rpg", "rpgle", "sqlrpgle"],
        });

        signatures.insert("plsql", SpectralSignature {
            star_pattern: r#"(?:PROCEDURE|FUNCTION|PACKAGE)\s+(\w+)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Data,
            display_name: "PL/SQL",
            extensions: &["pls", "plsql", "pck", "pkb", "pks"],
        });

        signatures.insert("abap", SpectralSignature {
            star_pattern: r#"(?:FORM|METHOD|FUNCTION|CLASS)\s+(\w+)"#,
            comment_single: r#"\*"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "ABAP",
            extensions: &["abap"],
        });

        signatures.insert("sas", SpectralSignature {
            star_pattern: r#"(?:%macro|proc|data)\s+(\w+)"#,
            comment_single: r#"\*"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Data,
            display_name: "SAS",
            extensions: &["sas"],
        });

        signatures.insert("foxpro", SpectralSignature {
            star_pattern: r#"(?:PROCEDURE|FUNCTION)\s+(\w+)"#,
            comment_single: r#"(?:\*|&&)"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "FoxPro",
            extensions: &["prg", "fxp"],
        });

        signatures.insert("vhdl", SpectralSignature {
            star_pattern: r#"(?:entity|architecture|process|procedure|function)\s+(\w+)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "VHDL",
            extensions: &["vhd", "vhdl"],
        });

        signatures.insert("verilog", SpectralSignature {
            star_pattern: r#"(?:module|task|function)\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Verilog",
            extensions: &["v", "vh", "sv", "svh"],
        });

        // =========================================================================
        // TIER 4: FUNCTIONAL & LOGIC
        // =========================================================================

        signatures.insert("haskell", SpectralSignature {
            star_pattern: r#"(\w+)\s*::|^(\w+)\s+.*="#,
            comment_single: r#"--"#,
            comment_multi_start: r#"\{-"#,
            comment_multi_end: r#"-\}"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Haskell",
            extensions: &["hs", "lhs"],
        });

        signatures.insert("elixir", SpectralSignature {
            star_pattern: r#"def(?:p?)\s+(\w+)|defmodule\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Elixir",
            extensions: &["ex", "exs"],
        });

        signatures.insert("erlang", SpectralSignature {
            star_pattern: r#"(\w+)\s*\([^)]*\)\s*->"#,
            comment_single: r#"%"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Erlang",
            extensions: &["erl", "hrl"],
        });

        signatures.insert("clojure", SpectralSignature {
            star_pattern: r#"\(def(?:n|macro|multi|method)?\s+(\w+(?:-\w+)*)"#,
            comment_single: r#";"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Clojure",
            extensions: &["clj", "cljs", "cljc", "edn"],
        });

        signatures.insert("scala", SpectralSignature {
            star_pattern: r#"def\s+(\w+)|class\s+(\w+)|object\s+(\w+)|trait\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Scala",
            extensions: &["scala", "sc"],
        });

        signatures.insert("fsharp", SpectralSignature {
            star_pattern: r#"let\s+(?:rec\s+)?(\w+)|type\s+(\w+)|module\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"\(\*"#,
            comment_multi_end: r#"\*\)"#,
            hemisphere: Hemisphere::Logic,
            display_name: "F#",
            extensions: &["fs", "fsi", "fsx"],
        });

        signatures.insert("ocaml", SpectralSignature {
            star_pattern: r#"let\s+(?:rec\s+)?(\w+)|type\s+(\w+)|module\s+(\w+)"#,
            comment_single: r#"$^"#,
            comment_multi_start: r#"\(\*"#,
            comment_multi_end: r#"\*\)"#,
            hemisphere: Hemisphere::Logic,
            display_name: "OCaml",
            extensions: &["ml", "mli"],
        });

        signatures.insert("scheme", SpectralSignature {
            star_pattern: r#"\(define\s+(?:\((\w+)|\(\s*(\w+))"#,
            comment_single: r#";"#,
            comment_multi_start: r#"#\|"#,
            comment_multi_end: r#"\|#"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Scheme",
            extensions: &["scm", "ss"],
        });

        signatures.insert("racket", SpectralSignature {
            star_pattern: r#"\(define\s+(?:\((\w+)|\(\s*(\w+))"#,
            comment_single: r#";"#,
            comment_multi_start: r#"#\|"#,
            comment_multi_end: r#"\|#"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Racket",
            extensions: &["rkt", "rktl"],
        });

        // =========================================================================
        // TIER 5: STELLAR NURSERIES (Emerging/Niche)
        // =========================================================================

        signatures.insert("zig", SpectralSignature {
            star_pattern: r#"(?:pub\s+)?fn\s+(\w+)|(?:pub\s+)?const\s+(\w+)\s*="#,
            comment_single: r#"//"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Zig",
            extensions: &["zig"],
        });

        signatures.insert("nim", SpectralSignature {
            star_pattern: r#"proc\s+(\w+)|func\s+(\w+)|type\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"#\["#,
            comment_multi_end: r#"\]#"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Nim",
            extensions: &["nim", "nims"],
        });

        signatures.insert("crystal", SpectralSignature {
            star_pattern: r#"def\s+(\w+[!?]?)|class\s+(\w+)|module\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Crystal",
            extensions: &["cr"],
        });

        signatures.insert("vlang", SpectralSignature {
            star_pattern: r#"fn\s+(\w+)|struct\s+(\w+)|enum\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "V",
            extensions: &["v"],
        });

        signatures.insert("elm", SpectralSignature {
            star_pattern: r#"(\w+)\s*:|type\s+(?:alias\s+)?(\w+)|module\s+(\w+)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"\{-"#,
            comment_multi_end: r#"-\}"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Elm",
            extensions: &["elm"],
        });

        signatures.insert("purescript", SpectralSignature {
            star_pattern: r#"(\w+)\s*::|data\s+(\w+)|type\s+(\w+)|module\s+(\w+)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"\{-"#,
            comment_multi_end: r#"-\}"#,
            hemisphere: Hemisphere::Logic,
            display_name: "PureScript",
            extensions: &["purs"],
        });

        signatures.insert("solidity", SpectralSignature {
            star_pattern: r#"(?:function|contract|interface|library|struct|enum|event)\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Solidity",
            extensions: &["sol"],
        });

        signatures.insert("vyper", SpectralSignature {
            star_pattern: r#"@(?:external|internal|view|pure)\s*\ndef\s+(\w+)|event\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Vyper",
            extensions: &["vy"],
        });

        signatures.insert("mojo", SpectralSignature {
            star_pattern: r#"fn\s+(\w+)|def\s+(\w+)|struct\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Mojo",
            extensions: &["mojo", "🔥"],
        });

        signatures.insert("gleam", SpectralSignature {
            star_pattern: r#"(?:pub\s+)?fn\s+(\w+)|(?:pub\s+)?type\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Gleam",
            extensions: &["gleam"],
        });

        // =========================================================================
        // TIER 6: SCIENTIFIC & SCRIPTING
        // =========================================================================

        signatures.insert("graphql", SpectralSignature {
            star_pattern: r#"(?:type|interface|enum|input|scalar|union|query|mutation|subscription)\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "GraphQL",
            extensions: &["graphql", "gql"],
        });

        signatures.insert("protobuf", SpectralSignature {
            star_pattern: r#"(?:message|service|enum|rpc)\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Data,
            display_name: "Protocol Buffers",
            extensions: &["proto"],
        });

        signatures.insert("thrift", SpectralSignature {
            star_pattern: r#"(?:struct|service|enum|exception)\s+(\w+)"#,
            comment_single: r#"(?://|#)"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Data,
            display_name: "Thrift",
            extensions: &["thrift"],
        });

        signatures.insert("gherkin", SpectralSignature {
            star_pattern: r#"(?:Feature|Scenario|Given|When|Then|And|But):\s*(.+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "Gherkin",
            extensions: &["feature"],
        });

        signatures.insert("lua", SpectralSignature {
            star_pattern: r#"(?:local\s+)?function\s+(\w+(?:\.\w+)*)"#,
            comment_single: r#"--"#,
            comment_multi_start: r#"--\[\["#,
            comment_multi_end: r#"\]\]"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Lua",
            extensions: &["lua"],
        });

        signatures.insert("perl", SpectralSignature {
            star_pattern: r#"sub\s+(\w+)|package\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"=pod"#,
            comment_multi_end: r#"=cut"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Perl",
            extensions: &["pl", "pm", "t", "pod"],
        });

        signatures.insert("r", SpectralSignature {
            star_pattern: r#"(\w+)\s*<-\s*function|setGeneric\s*\(\s*"(\w+)""#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "R",
            extensions: &["r", "R", "rmd", "Rmd"],
        });

        signatures.insert("julia", SpectralSignature {
            star_pattern: r#"function\s+(\w+)|macro\s+(\w+)|struct\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"#="#,
            comment_multi_end: r#"=#"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Julia",
            extensions: &["jl"],
        });

        signatures.insert("matlab", SpectralSignature {
            star_pattern: r#"function\s+(?:\[?[^\]]*\]?\s*=\s*)?(\w+)|classdef\s+(\w+)"#,
            comment_single: r#"%"#,
            comment_multi_start: r#"%\{"#,
            comment_multi_end: r#"%\}"#,
            hemisphere: Hemisphere::Logic,
            display_name: "MATLAB",
            extensions: &["m", "mat"],
        });

        signatures.insert("actionscript", SpectralSignature {
            star_pattern: r#"(?:public|private|protected)?\s*(?:static\s+)?function\s+(\w+)|class\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "ActionScript",
            extensions: &["as"],
        });

        signatures.insert("groovy", SpectralSignature {
            star_pattern: r#"def\s+(\w+)|class\s+(\w+)|interface\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Groovy",
            extensions: &["groovy", "gvy", "gy", "gsh"],
        });

        signatures.insert("coffeescript", SpectralSignature {
            star_pattern: r#"(\w+)\s*[=:]\s*\([^)]*\)\s*->|class\s+(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"###"#,
            comment_multi_end: r#"###"#,
            hemisphere: Hemisphere::Logic,
            display_name: "CoffeeScript",
            extensions: &["coffee"],
        });

        signatures.insert("smalltalk", SpectralSignature {
            star_pattern: r#"(\w+)\s*:\s*|(\w+)\s+>>"#,
            comment_single: r#"$^"#,
            comment_multi_start: r#""""#,
            comment_multi_end: r#""""#,
            hemisphere: Hemisphere::Logic,
            display_name: "Smalltalk",
            extensions: &["st"],
        });

        // Additional languages for completeness

        signatures.insert("objectivec", SpectralSignature {
            star_pattern: r#"[-+]\s*\([^)]+\)\s*(\w+)|@interface\s+(\w+)|@implementation\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"/\*"#,
            comment_multi_end: r#"\*/"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Objective-C",
            extensions: &["m", "mm", "h"],
        });

        signatures.insert("assembly", SpectralSignature {
            star_pattern: r#"(\w+):"#,
            comment_single: r#";"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Logic,
            display_name: "Assembly",
            extensions: &["asm", "s", "S"],
        });

        signatures.insert("latex", SpectralSignature {
            star_pattern: r#"\\(?:section|subsection|chapter|newcommand|def)\{?(\w+)"#,
            comment_single: r#"%"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Interface,
            display_name: "LaTeX",
            extensions: &["tex", "latex", "sty", "cls"],
        });

        signatures.insert("toml", SpectralSignature {
            star_pattern: r#"\[(\w+(?:\.\w+)*)\]"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "TOML",
            extensions: &["toml"],
        });

        signatures.insert("ini", SpectralSignature {
            star_pattern: r#"\[(\w+)\]"#,
            comment_single: r#"[;#]"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Data,
            display_name: "INI",
            extensions: &["ini", "cfg"],
        });

        signatures.insert("cmake", SpectralSignature {
            star_pattern: r#"(?:function|macro)\s*\(\s*(\w+)"#,
            comment_single: r#"#"#,
            comment_multi_start: r#"$^"#,
            comment_multi_end: r#"$^"#,
            hemisphere: Hemisphere::Automation,
            display_name: "CMake",
            extensions: &["cmake"],
        });

        signatures.insert("vue", SpectralSignature {
            star_pattern: r#"<template>|<script>|<style>|export\s+default\s*\{|defineComponent"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"<!--"#,
            comment_multi_end: r#"-->"#,
            hemisphere: Hemisphere::Interface,
            display_name: "Vue",
            extensions: &["vue"],
        });

        signatures.insert("svelte", SpectralSignature {
            star_pattern: r#"<script>|<style>|export\s+let\s+(\w+)|function\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"<!--"#,
            comment_multi_end: r#"-->"#,
            hemisphere: Hemisphere::Interface,
            display_name: "Svelte",
            extensions: &["svelte"],
        });

        signatures.insert("astro", SpectralSignature {
            star_pattern: r#"---.*---|export\s+(?:const|let|function)\s+(\w+)"#,
            comment_single: r#"//"#,
            comment_multi_start: r#"<!--"#,
            comment_multi_end: r#"-->"#,
            hemisphere: Hemisphere::Interface,
            display_name: "Astro",
            extensions: &["astro"],
        });

        signatures.insert("wasm", SpectralSignature {
            star_pattern: r#"\(func\s+\$(\w+)|\(module"#,
            comment_single: r#";;"#,
            comment_multi_start: r#"\(;"#,
            comment_multi_end: r#";\)"#,
            hemisphere: Hemisphere::Logic,
            display_name: "WebAssembly Text",
            extensions: &["wat", "wast"],
        });

        Self { signatures }
    }

    /// Get a signature by language name
    pub fn get(&self, language: &str) -> Option<&SpectralSignature> {
        self.signatures.get(language.to_lowercase().as_str())
    }

    /// Get a signature by file extension
    pub fn get_by_extension(&self, ext: &str) -> Option<&SpectralSignature> {
        let ext_lower = ext.to_lowercase();
        self.signatures.values()
            .find(|sig| sig.extensions.iter().any(|e| *e == ext_lower))
    }

    /// Get all supported languages
    pub fn languages(&self) -> Vec<&str> {
        self.signatures.keys().copied().collect()
    }

    /// Get the number of supported languages
    pub fn language_count(&self) -> usize {
        self.signatures.len()
    }

    /// Get languages by hemisphere
    pub fn get_by_hemisphere(&self, hemisphere: Hemisphere) -> Vec<&SpectralSignature> {
        self.signatures.values()
            .filter(|sig| sig.hemisphere == hemisphere)
            .collect()
    }
}

impl Default for StellarLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// Global singleton for pattern caching
lazy_static::lazy_static! {
    pub static ref STELLAR_LIBRARY: StellarLibrary = StellarLibrary::new();
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stellar_library_creation() {
        let library = StellarLibrary::new();
        assert!(library.language_count() >= 60, "Should have 60+ languages");
    }

    #[test]
    fn test_get_by_extension() {
        let library = StellarLibrary::new();

        // Test various extensions
        assert!(library.get_by_extension("rs").is_some());
        assert!(library.get_by_extension("py").is_some());
        assert!(library.get_by_extension("js").is_some());
        assert!(library.get_by_extension("cob").is_some()); // COBOL
        assert!(library.get_by_extension("sim").is_some()); // Simula
        assert!(library.get_by_extension("logo").is_some()); // Logo
        assert!(library.get_by_extension("tcl").is_some()); // Tcl
    }

    #[test]
    fn test_ancient_stars_present() {
        let library = StellarLibrary::new();

        // The Ancient Stars must be present
        assert!(library.get("cobol").is_some(), "COBOL must be supported");
        assert!(library.get("simula").is_some(), "Simula must be supported");
        assert!(library.get("logo").is_some(), "Logo must be supported");
        assert!(library.get("fortran").is_some(), "Fortran must be supported");
        assert!(library.get("pascal").is_some(), "Pascal must be supported");
        assert!(library.get("lisp").is_some(), "Lisp must be supported");
        assert!(library.get("prolog").is_some(), "Prolog must be supported");
    }

    #[test]
    fn test_modern_giants_present() {
        let library = StellarLibrary::new();

        // The Modern Giants
        assert!(library.get("rust").is_some());
        assert!(library.get("python").is_some());
        assert!(library.get("javascript").is_some());
        assert!(library.get("typescript").is_some());
        assert!(library.get("java").is_some());
        assert!(library.get("go").is_some());
    }

    #[test]
    fn test_emerging_languages_present() {
        let library = StellarLibrary::new();

        // Stellar Nurseries
        assert!(library.get("zig").is_some());
        assert!(library.get("nim").is_some());
        assert!(library.get("gleam").is_some());
        assert!(library.get("solidity").is_some());
    }

    #[test]
    fn test_hemisphere_classification() {
        let library = StellarLibrary::new();

        // Logic hemisphere
        let rust = library.get("rust").unwrap();
        assert_eq!(rust.hemisphere, Hemisphere::Logic);

        // Interface hemisphere
        let html = library.get("html").unwrap();
        assert_eq!(html.hemisphere, Hemisphere::Interface);

        // Automation hemisphere
        let bash = library.get("bash").unwrap();
        assert_eq!(bash.hemisphere, Hemisphere::Automation);

        // Data hemisphere
        let sql = library.get("sql").unwrap();
        assert_eq!(sql.hemisphere, Hemisphere::Data);
    }

    #[test]
    fn test_star_pattern_extraction() {
        let library = StellarLibrary::new();

        // Test that star patterns are valid regexes
        for (name, sig) in &library.signatures {
            let result = regex::Regex::new(sig.star_pattern);
            assert!(result.is_ok(), "Invalid star pattern for {}: {}", name, sig.star_pattern);
        }
    }

    #[test]
    fn test_tcl_pattern() {
        let library = StellarLibrary::new();
        let tcl = library.get("tcl").unwrap();

        let regex = regex::Regex::new(tcl.star_pattern).unwrap();

        // Test Tcl procedure extraction
        assert!(regex.is_match("proc myFunction {args} {"));
        assert!(regex.is_match("proc hello_world {} {"));

        let caps = regex.captures("proc calculateTotal {a b} {").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "calculateTotal");
    }

    #[test]
    fn test_simula_pattern() {
        let library = StellarLibrary::new();
        let simula = library.get("simula").unwrap();

        let regex = regex::Regex::new(simula.star_pattern).unwrap();

        // Test Simula class and procedure extraction
        assert!(regex.is_match("class Point"));
        assert!(regex.is_match("procedure Draw"));

        let caps = regex.captures("class Vehicle").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "Vehicle");
    }

    #[test]
    fn test_logo_pattern() {
        let library = StellarLibrary::new();
        let logo = library.get("logo").unwrap();

        let regex = regex::Regex::new(logo.star_pattern).unwrap();

        // Test Logo procedure extraction
        assert!(regex.is_match("to square"));
        assert!(regex.is_match("to drawCircle"));

        let caps = regex.captures("to triangle").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "triangle");
    }

    #[test]
    fn test_cobol_pattern() {
        let library = StellarLibrary::new();
        let cobol = library.get("cobol").unwrap();

        let regex = regex::Regex::new(cobol.star_pattern).unwrap();

        // Test COBOL procedure extraction
        assert!(regex.is_match("PROCEDURE DIVISION"));
        assert!(regex.is_match("MAIN-PROCEDURE SECTION."));
    }
}
