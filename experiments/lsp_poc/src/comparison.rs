//! Symbol extraction comparison: Regex vs LSP
//!
//! This module implements symbol extraction using both approaches
//! to measure accuracy differences.

use std::collections::HashSet;
use std::time::{Duration, Instant};
use regex::Regex;
use crate::metrics::SymbolMetrics;

/// A code symbol (function, class, struct, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
    Type,
    Mod,
    Class,
    Method,
    Unknown,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "fn"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::Impl => write!(f, "impl"),
            SymbolKind::Const => write!(f, "const"),
            SymbolKind::Static => write!(f, "static"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Mod => write!(f, "mod"),
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Unknown => write!(f, "unknown"),
        }
    }
}

/// Regex-based symbol extractor for Rust code
/// Patterns derived from pm_encoder::core::skeleton::parser
pub struct RegexExtractor {
    fn_pattern: Regex,
    struct_pattern: Regex,
    enum_pattern: Regex,
    trait_pattern: Regex,
    impl_pattern: Regex,
    const_pattern: Regex,
    type_pattern: Regex,
    mod_pattern: Regex,
}

impl RegexExtractor {
    pub fn new() -> Self {
        Self {
            // Function: pub/async/const fn name
            fn_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?(async\s+)?(const\s+)?(unsafe\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
            // Struct: pub struct Name
            struct_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?struct\s+([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
            // Enum: pub enum Name
            enum_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?enum\s+([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
            // Trait: pub trait Name
            trait_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?(unsafe\s+)?trait\s+([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
            // Impl: impl Name or impl Trait for Name
            impl_pattern: Regex::new(
                r"(?m)^[[:space:]]*(unsafe\s+)?impl(?:<[^>]*>)?\s+(?:([a-zA-Z_][a-zA-Z0-9_]*)\s+for\s+)?([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
            // Const: pub const NAME
            const_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?const\s+([A-Z_][A-Z0-9_]*)\s*:"
            ).unwrap(),
            // Type alias: pub type Name
            type_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?type\s+([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
            // Module: pub mod name
            mod_pattern: Regex::new(
                r"(?m)^[[:space:]]*(pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)"
            ).unwrap(),
        }
    }

    /// Extract all symbols from Rust source code
    pub fn extract(&self, source: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Helper to find line number for a byte offset
        let find_line = |offset: usize| -> usize {
            let prefix = &source[..offset];
            prefix.chars().filter(|c| *c == '\n').count() + 1
        };

        // Functions
        for cap in self.fn_pattern.captures_iter(source) {
            if let Some(name) = cap.get(5) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Structs
        for cap in self.struct_pattern.captures_iter(source) {
            if let Some(name) = cap.get(2) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Enums
        for cap in self.enum_pattern.captures_iter(source) {
            if let Some(name) = cap.get(2) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Enum,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Traits
        for cap in self.trait_pattern.captures_iter(source) {
            if let Some(name) = cap.get(3) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Trait,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Impls
        for cap in self.impl_pattern.captures_iter(source) {
            if let Some(name) = cap.get(3) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Impl,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Consts
        for cap in self.const_pattern.captures_iter(source) {
            if let Some(name) = cap.get(2) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Const,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Type aliases
        for cap in self.type_pattern.captures_iter(source) {
            if let Some(name) = cap.get(2) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Type,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        // Modules
        for cap in self.mod_pattern.captures_iter(source) {
            if let Some(name) = cap.get(2) {
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Mod,
                    line: find_line(cap.get(0).unwrap().start()),
                });
            }
        }

        symbols
    }

    /// Extract with timing
    pub fn extract_timed(&self, source: &str) -> (Vec<Symbol>, Duration) {
        let start = Instant::now();
        let symbols = self.extract(source);
        (symbols, start.elapsed())
    }
}

impl Default for RegexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// LSP-based symbol extractor (stub - to be implemented with real LSP client)
pub struct LspExtractor {
    // Will hold LSP client connection
}

impl LspExtractor {
    pub fn new() -> Self {
        Self {}
    }

    /// Extract symbols via LSP documentSymbol request
    /// Currently a stub that returns empty - will be implemented when LSP client is ready
    pub fn extract(&self, _source: &str, _file_uri: &str) -> Vec<Symbol> {
        // TODO: Implement with real LSP client
        // 1. Send textDocument/didOpen
        // 2. Send textDocument/documentSymbol
        // 3. Parse SymbolInformation[] response
        Vec::new()
    }

    /// Extract with timing
    pub fn extract_timed(&self, source: &str, file_uri: &str) -> (Vec<Symbol>, Duration) {
        let start = Instant::now();
        let symbols = self.extract(source, file_uri);
        (symbols, start.elapsed())
    }
}

impl Default for LspExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare symbols from two extractors
pub fn compare_symbols(regex_symbols: &[Symbol], lsp_symbols: &[Symbol]) -> SymbolMetrics {
    let regex_names: HashSet<&str> = regex_symbols.iter().map(|s| s.name.as_str()).collect();
    let lsp_names: HashSet<&str> = lsp_symbols.iter().map(|s| s.name.as_str()).collect();

    let matched: HashSet<&&str> = regex_names.intersection(&lsp_names).collect();

    SymbolMetrics {
        regex_count: regex_symbols.len(),
        lsp_count: lsp_symbols.len(),
        matched_count: matched.len(),
        regex_duration: Duration::ZERO,
        lsp_duration: Duration::ZERO,
    }
}

/// Run full comparison with timing
pub fn run_comparison(source: &str, file_uri: &str) -> SymbolMetrics {
    let regex_extractor = RegexExtractor::new();
    let lsp_extractor = LspExtractor::new();

    let (regex_symbols, regex_duration) = regex_extractor.extract_timed(source);
    let (lsp_symbols, lsp_duration) = lsp_extractor.extract_timed(source, file_uri);

    let mut metrics = compare_symbols(&regex_symbols, &lsp_symbols);
    metrics.regex_duration = regex_duration;
    metrics.lsp_duration = lsp_duration;

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_RUST_CODE: &str = r#"
//! Test module

use std::collections::HashMap;

pub const MAX_SIZE: usize = 1024;

pub struct Config {
    pub name: String,
    pub value: i32,
}

pub enum Status {
    Active,
    Inactive,
}

pub trait Handler {
    fn handle(&self, data: &str);
}

impl Handler for Config {
    fn handle(&self, data: &str) {
        println!("{}: {}", self.name, data);
    }
}

pub fn process_data(input: &str) -> Result<(), String> {
    Ok(())
}

async fn fetch_async() -> i32 {
    42
}

pub mod utils {
    pub fn helper() {}
}

type Result<T> = std::result::Result<T, String>;
"#;

    #[test]
    fn test_regex_extractor_functions() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let functions: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();

        assert!(functions.iter().any(|s| s.name == "process_data"));
        assert!(functions.iter().any(|s| s.name == "fetch_async"));
        assert!(functions.iter().any(|s| s.name == "handle"));
        assert!(functions.iter().any(|s| s.name == "helper"));
    }

    #[test]
    fn test_regex_extractor_structs() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let structs: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Struct)
            .collect();

        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Config");
    }

    #[test]
    fn test_regex_extractor_enums() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let enums: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .collect();

        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "Status");
    }

    #[test]
    fn test_regex_extractor_traits() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let traits: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Trait)
            .collect();

        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "Handler");
    }

    #[test]
    fn test_regex_extractor_impls() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let impls: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Impl)
            .collect();

        assert_eq!(impls.len(), 1);
        assert_eq!(impls[0].name, "Config");
    }

    #[test]
    fn test_regex_extractor_consts() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let consts: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Const)
            .collect();

        assert_eq!(consts.len(), 1);
        assert_eq!(consts[0].name, "MAX_SIZE");
    }

    #[test]
    fn test_regex_extractor_mods() {
        let extractor = RegexExtractor::new();
        let symbols = extractor.extract(TEST_RUST_CODE);

        let mods: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Mod)
            .collect();

        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].name, "utils");
    }

    #[test]
    fn test_regex_extractor_timing() {
        let extractor = RegexExtractor::new();
        let (symbols, duration) = extractor.extract_timed(TEST_RUST_CODE);

        assert!(!symbols.is_empty());
        // Should be very fast (microseconds)
        assert!(duration.as_millis() < 10);
    }

    #[test]
    fn test_compare_symbols() {
        let regex_symbols = vec![
            Symbol { name: "foo".to_string(), kind: SymbolKind::Function, line: 1 },
            Symbol { name: "bar".to_string(), kind: SymbolKind::Function, line: 2 },
            Symbol { name: "baz".to_string(), kind: SymbolKind::Struct, line: 3 },
        ];

        let lsp_symbols = vec![
            Symbol { name: "foo".to_string(), kind: SymbolKind::Function, line: 1 },
            Symbol { name: "bar".to_string(), kind: SymbolKind::Function, line: 2 },
            Symbol { name: "qux".to_string(), kind: SymbolKind::Enum, line: 4 },
        ];

        let metrics = compare_symbols(&regex_symbols, &lsp_symbols);

        assert_eq!(metrics.regex_count, 3);
        assert_eq!(metrics.lsp_count, 3);
        assert_eq!(metrics.matched_count, 2); // foo and bar
    }
}
