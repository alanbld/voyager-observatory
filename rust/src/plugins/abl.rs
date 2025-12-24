//! ABL (OpenEdge Progress 4GL) Language Plugin
//!
//! Provides analysis for ABL source files including:
//! - PROCEDURE and FUNCTION extraction
//! - DEFINE statements (TEMP-TABLE, VARIABLE, BUFFER)
//! - FOR EACH database access patterns
//! - RUN statement tracking (procedure calls)
//! - INCLUDE file dependencies
//!
//! # ABL-Specific Semantic Mapping
//!
//! ABL has distinct patterns that map to our universal semantic concepts:
//! - `PROCEDURE calculate-*:` → ConceptType::Calculation
//! - `PROCEDURE validate-*:` → ConceptType::Validation
//! - `FOR EACH` blocks → ConceptType::Transformation
//! - `DEFINE TEMP-TABLE` → Data structure (Configuration)
//! - `RUN` statements → Dependencies/calls
//!
//! # Example
//!
//! ```text
//! /* Calculate order total */
//! PROCEDURE calculate-order-total:
//!     DEFINE INPUT PARAMETER ip-order-id AS INTEGER.
//!     DEFINE OUTPUT PARAMETER op-total AS DECIMAL.
//!
//!     FOR EACH order-line WHERE order-line.order-id = ip-order-id:
//!         op-total = op-total + (order-line.qty * order-line.price).
//!     END.
//! END PROCEDURE.
//! ```

use std::collections::HashMap;

use regex::Regex;

use crate::core::fractal::{
    ExtractedSymbol, Import, Range, SymbolKind, Parameter, ConceptType, Visibility,
};

use super::{FileInfo, LanguagePlugin, PluginResult};

// =============================================================================
// ABL Constructs
// =============================================================================

/// ABL access mode for procedures/functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AblAccessMode {
    #[default]
    Internal,
    External,
    Persistent,
}

impl AblAccessMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AblAccessMode::Internal => "internal",
            AblAccessMode::External => "external",
            AblAccessMode::Persistent => "persistent",
        }
    }
}

/// Type of ABL define statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AblDefineType {
    Variable,
    Parameter,
    TempTable,
    Buffer,
    Stream,
    WorkTable,
    Dataset,
    DataSource,
    Query,
    Frame,
    Event,
    Property,
}

impl AblDefineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AblDefineType::Variable => "variable",
            AblDefineType::Parameter => "parameter",
            AblDefineType::TempTable => "temp-table",
            AblDefineType::Buffer => "buffer",
            AblDefineType::Stream => "stream",
            AblDefineType::WorkTable => "work-table",
            AblDefineType::Dataset => "dataset",
            AblDefineType::DataSource => "data-source",
            AblDefineType::Query => "query",
            AblDefineType::Frame => "frame",
            AblDefineType::Event => "event",
            AblDefineType::Property => "property",
        }
    }
}

// =============================================================================
// ABL Plugin
// =============================================================================

/// Plugin for analyzing ABL (OpenEdge Progress 4GL) source files.
pub struct AblPlugin {
    /// Pattern for PROCEDURE declarations
    procedure_pattern: Regex,
    /// Pattern for FUNCTION declarations
    function_pattern: Regex,
    /// Pattern for DEFINE statements
    define_pattern: Regex,
    /// Pattern for FOR EACH blocks
    for_each_pattern: Regex,
    /// Pattern for RUN statements
    run_pattern: Regex,
    /// Pattern for {include} directives
    include_pattern: Regex,
    /// Pattern for CLASS declarations
    class_pattern: Regex,
    /// Pattern for METHOD declarations
    method_pattern: Regex,
    /// Pattern for TRIGGER declarations
    trigger_pattern: Regex,
}

impl AblPlugin {
    pub fn new() -> Self {
        Self {
            // PROCEDURE name [PERSISTENT|EXTERNAL]:
            procedure_pattern: Regex::new(
                r"(?mi)^\s*PROCEDURE\s+([a-zA-Z_][a-zA-Z0-9_-]*)\s*(?:(EXTERNAL|PERSISTENT))?\s*:"
            ).unwrap(),

            // FUNCTION name RETURNS type [FORWARD]:
            function_pattern: Regex::new(
                r"(?mi)^\s*FUNCTION\s+([a-zA-Z_][a-zA-Z0-9_-]*)\s+RETURNS\s+(\w+(?:\s+EXTENT)?)"
            ).unwrap(),

            // DEFINE [NEW [SHARED]] [PRIVATE|PROTECTED|PUBLIC] [STATIC]
            //        INPUT|OUTPUT|INPUT-OUTPUT|RETURN PARAMETER |
            //        TEMP-TABLE | BUFFER | VARIABLE | STREAM | etc.
            define_pattern: Regex::new(
                r"(?mi)^\s*DEFINE\s+(?:(?:NEW\s+)?SHARED\s+)?(?:PRIVATE\s+|PROTECTED\s+|PUBLIC\s+)?(?:STATIC\s+)?(?:(INPUT|OUTPUT|INPUT-OUTPUT|RETURN)\s+)?(?:PARAMETER\s+)?(TEMP-TABLE|BUFFER|VARIABLE|STREAM|WORK-TABLE|DATASET|DATA-SOURCE|QUERY|FRAME|EVENT|PROPERTY)\s+([a-zA-Z_][a-zA-Z0-9_-]*)"
            ).unwrap(),

            // FOR EACH table [WHERE ...]:
            for_each_pattern: Regex::new(
                r"(?mi)\bFOR\s+(?:FIRST|LAST|EACH)\s+([a-zA-Z_][a-zA-Z0-9_-]*)"
            ).unwrap(),

            // RUN procedure [PERSISTENT|ON SERVER ...]:
            run_pattern: Regex::new(
                r"(?mi)\bRUN\s+([a-zA-Z_][a-zA-Z0-9_.-]*(?:\.p|\.w)?)"
            ).unwrap(),

            // {include-file.i} or {include-file.i param}
            include_pattern: Regex::new(
                r"\{([^}]+\.i)(?:\s+[^}]*)?\}"
            ).unwrap(),

            // CLASS namespace.ClassName [INHERITS parent] [IMPLEMENTS interfaces]:
            class_pattern: Regex::new(
                r"(?mi)^\s*CLASS\s+([\w.]+)\s*(?:INHERITS\s+([\w.]+))?"
            ).unwrap(),

            // METHOD [PRIVATE|PROTECTED|PUBLIC] [STATIC] [OVERRIDE] type name:
            method_pattern: Regex::new(
                r"(?mi)^\s*METHOD\s+(?:PRIVATE\s+|PROTECTED\s+|PUBLIC\s+)?(?:STATIC\s+)?(?:OVERRIDE\s+)?(\w+)\s+([a-zA-Z_][a-zA-Z0-9_-]*)\s*\("
            ).unwrap(),

            // ON event OF table [trigger]:
            trigger_pattern: Regex::new(
                r"(?mi)^\s*ON\s+(WRITE|CREATE|DELETE|FIND|ASSIGN)\s+OF\s+([a-zA-Z_][a-zA-Z0-9_-]*)"
            ).unwrap(),
        }
    }

    /// Extract documentation comment above a line.
    fn extract_doc_comment(&self, lines: &[&str], line_num: usize) -> Option<String> {
        let mut docs = Vec::new();
        let mut idx = line_num.saturating_sub(1);

        while idx > 0 {
            let line = lines.get(idx)?;
            let trimmed = line.trim();

            // ABL block comments: /* ... */
            if trimmed.ends_with("*/") && !trimmed.starts_with("/*") {
                // End of block comment, scan backwards
                let mut comment_lines = Vec::new();
                while idx > 0 {
                    let cline = lines.get(idx)?;
                    comment_lines.push(cline.trim().trim_start_matches("/*").trim_end_matches("*/").trim());
                    if cline.contains("/*") {
                        break;
                    }
                    idx = idx.saturating_sub(1);
                }
                comment_lines.reverse();
                return Some(comment_lines.join(" "));
            }

            // Single line block comment: /* comment */
            if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                docs.push(trimmed.trim_start_matches("/*").trim_end_matches("*/").trim());
                idx = idx.saturating_sub(1);
            } else if trimmed.is_empty() {
                // Allow blank lines between doc and declaration
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

    /// Count parameters in a procedure/function.
    fn extract_parameters(&self, content: &str, proc_start: usize, proc_end: usize) -> Vec<Parameter> {
        let block = &content[proc_start..proc_end.min(content.len())];
        let mut params = Vec::new();

        // Match DEFINE INPUT|OUTPUT|INPUT-OUTPUT PARAMETER name AS type
        let param_re = Regex::new(
            r"(?mi)DEFINE\s+(INPUT|OUTPUT|INPUT-OUTPUT)\s+PARAMETER\s+([a-zA-Z_][a-zA-Z0-9_-]*)\s+AS\s+(\w+)"
        ).unwrap();

        for cap in param_re.captures_iter(block) {
            params.push(Parameter {
                name: cap[2].to_string(),
                type_hint: Some(format!("{} {}", &cap[1], &cap[3])),
                default_value: None,
            });
        }

        params
    }

    /// Find the end of a procedure/function block.
    fn find_block_end(&self, content: &str, start: usize, block_type: &str) -> usize {
        let search = &content[start..];
        let end_pattern = match block_type {
            "PROCEDURE" => Regex::new(r"(?mi)^\s*END\s+PROCEDURE\s*\.").unwrap(),
            "FUNCTION" => Regex::new(r"(?mi)^\s*END\s+FUNCTION\s*\.").unwrap(),
            "METHOD" => Regex::new(r"(?mi)^\s*END\s+METHOD\s*\.").unwrap(),
            "CLASS" => Regex::new(r"(?mi)^\s*END\s+CLASS\s*\.").unwrap(),
            _ => return start + 100, // Default heuristic
        };

        end_pattern.find(search)
            .map(|m| start + m.end())
            .unwrap_or_else(|| content.len())
    }

    /// Classify ABL procedure/function by name patterns.
    fn classify_abl_name(&self, name: &str) -> ConceptType {
        let name_lower = name.to_lowercase();

        // Calculation patterns
        if name_lower.starts_with("calc") || name_lower.starts_with("calculate")
            || name_lower.contains("-calc") || name_lower.contains("_calc")
            || name_lower.contains("-total") || name_lower.contains("_total")
            || name_lower.contains("-sum") || name_lower.contains("_sum")
            || name_lower.contains("-avg") || name_lower.contains("_avg")
            || name_lower.contains("-price") || name_lower.contains("_price")
            || name_lower.contains("-cost") || name_lower.contains("_cost")
            || name_lower.contains("-tax") || name_lower.contains("_tax")
        {
            return ConceptType::Calculation;
        }

        // Validation patterns
        if name_lower.starts_with("validate") || name_lower.starts_with("check")
            || name_lower.starts_with("verify") || name_lower.starts_with("is-")
            || name_lower.contains("-validate") || name_lower.contains("_validate")
            || name_lower.contains("-check") || name_lower.contains("_check")
            || name_lower.contains("-valid") || name_lower.contains("_valid")
        {
            return ConceptType::Validation;
        }

        // Error handling patterns
        if name_lower.starts_with("error") || name_lower.starts_with("handle")
            || name_lower.contains("-error") || name_lower.contains("_error")
            || name_lower.contains("-exception") || name_lower.contains("_exception")
            || name_lower.contains("-recover") || name_lower.contains("_recover")
        {
            return ConceptType::ErrorHandling;
        }

        // Logging patterns
        if name_lower.starts_with("log") || name_lower.starts_with("trace")
            || name_lower.contains("-log") || name_lower.contains("_log")
            || name_lower.contains("-audit") || name_lower.contains("_audit")
            || name_lower.contains("-trace") || name_lower.contains("_trace")
        {
            return ConceptType::Logging;
        }

        // Configuration patterns
        if name_lower.starts_with("config") || name_lower.starts_with("init")
            || name_lower.starts_with("setup") || name_lower.starts_with("load-config")
            || name_lower.contains("-config") || name_lower.contains("_config")
            || name_lower.contains("-init") || name_lower.contains("_init")
            || name_lower.contains("-setup") || name_lower.contains("_setup")
        {
            return ConceptType::Configuration;
        }

        // Transformation/conversion patterns
        if name_lower.starts_with("convert") || name_lower.starts_with("transform")
            || name_lower.starts_with("format") || name_lower.starts_with("parse")
            || name_lower.contains("-convert") || name_lower.contains("_convert")
            || name_lower.contains("-transform") || name_lower.contains("_transform")
            || name_lower.contains("-to-") || name_lower.contains("_to_")
            || name_lower.contains("-format") || name_lower.contains("_format")
        {
            return ConceptType::Transformation;
        }

        // Decision/routing patterns
        if name_lower.starts_with("route") || name_lower.starts_with("dispatch")
            || name_lower.starts_with("process") || name_lower.starts_with("main")
            || name_lower.contains("-route") || name_lower.contains("_route")
            || name_lower.contains("-dispatch") || name_lower.contains("_dispatch")
            || name_lower.contains("-handler") || name_lower.contains("_handler")
        {
            return ConceptType::Decision;
        }

        // Infrastructure patterns
        if name_lower.starts_with("open") || name_lower.starts_with("close")
            || name_lower.starts_with("connect") || name_lower.starts_with("read")
            || name_lower.starts_with("write") || name_lower.starts_with("send")
            || name_lower.contains("-db") || name_lower.contains("_db")
            || name_lower.contains("-file") || name_lower.contains("_file")
            || name_lower.contains("-socket") || name_lower.contains("_socket")
        {
            return ConceptType::Infrastructure;
        }

        // Testing patterns
        if name_lower.starts_with("test") || name_lower.ends_with("-test")
            || name_lower.ends_with("_test") || name_lower.contains("-mock")
            || name_lower.contains("_mock")
        {
            return ConceptType::Testing;
        }

        // Default: Calculation for business procedures (ABL is business-oriented)
        ConceptType::Calculation
    }
}

impl Default for AblPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePlugin for AblPlugin {
    fn language_name(&self) -> &'static str {
        "abl"
    }

    fn extensions(&self) -> &[&'static str] {
        &["p", "w", "i", "cls"]
    }

    fn extract_symbols(&self, content: &str) -> PluginResult<Vec<ExtractedSymbol>> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Extract procedures
        for cap in self.procedure_pattern.captures_iter(content) {
            let name = &cap[1];
            let access_mode = cap.get(2).map(|m| m.as_str()).unwrap_or("internal");
            let match_start = cap.get(0).unwrap().start();

            // Find line number
            let line_num = content[..match_start].matches('\n').count();
            let block_end = self.find_block_end(content, match_start, "PROCEDURE");
            let end_line = content[..block_end].matches('\n').count();

            // Extract documentation
            let documentation = self.extract_doc_comment(&lines, line_num);

            // Extract parameters
            let parameters = self.extract_parameters(content, match_start, block_end);

            // Extract RUN calls within the procedure
            let block = &content[match_start..block_end.min(content.len())];
            let calls: Vec<String> = self.run_pattern
                .captures_iter(block)
                .map(|c| c[1].to_string())
                .collect();

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!("PROCEDURE {} {}:", name, access_mode.to_uppercase()),
                return_type: None,
                parameters,
                documentation,
                visibility: if access_mode.eq_ignore_ascii_case("external") {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                range: Range::line_range(line_num, end_line),
                calls,
            });
        }

        // Extract functions
        for cap in self.function_pattern.captures_iter(content) {
            let name = &cap[1];
            let return_type = &cap[2];
            let match_start = cap.get(0).unwrap().start();

            let line_num = content[..match_start].matches('\n').count();
            let block_end = self.find_block_end(content, match_start, "FUNCTION");
            let end_line = content[..block_end].matches('\n').count();

            let documentation = self.extract_doc_comment(&lines, line_num);
            let parameters = self.extract_parameters(content, match_start, block_end);

            // Extract RUN calls within the function
            let block = &content[match_start..block_end.min(content.len())];
            let calls: Vec<String> = self.run_pattern
                .captures_iter(block)
                .map(|c| c[1].to_string())
                .collect();

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!("FUNCTION {} RETURNS {}", name, return_type),
                return_type: Some(return_type.to_string()),
                parameters,
                documentation,
                visibility: Visibility::Public, // ABL functions are typically public
                range: Range::line_range(line_num, end_line),
                calls,
            });
        }

        // Extract classes
        for cap in self.class_pattern.captures_iter(content) {
            let name = &cap[1];
            let parent = cap.get(2).map(|m| m.as_str());
            let match_start = cap.get(0).unwrap().start();

            let line_num = content[..match_start].matches('\n').count();
            let block_end = self.find_block_end(content, match_start, "CLASS");
            let end_line = content[..block_end].matches('\n').count();

            let documentation = self.extract_doc_comment(&lines, line_num);

            let signature = if let Some(p) = parent {
                format!("CLASS {} INHERITS {}", name, p)
            } else {
                format!("CLASS {}", name)
            };

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Class,
                signature,
                return_type: None,
                parameters: vec![],
                documentation,
                visibility: Visibility::Public,
                range: Range::line_range(line_num, end_line),
                calls: vec![],
            });
        }

        // Extract methods (within classes)
        for cap in self.method_pattern.captures_iter(content) {
            let return_type = &cap[1];
            let name = &cap[2];
            let match_start = cap.get(0).unwrap().start();

            let line_num = content[..match_start].matches('\n').count();
            let block_end = self.find_block_end(content, match_start, "METHOD");
            let end_line = content[..block_end].matches('\n').count();

            let documentation = self.extract_doc_comment(&lines, line_num);

            // Extract RUN calls within the method
            let block = &content[match_start..block_end.min(content.len())];
            let calls: Vec<String> = self.run_pattern
                .captures_iter(block)
                .map(|c| c[1].to_string())
                .collect();

            symbols.push(ExtractedSymbol {
                name: name.to_string(),
                kind: SymbolKind::Method,
                signature: format!("METHOD {} {}", return_type, name),
                return_type: Some(return_type.to_string()),
                parameters: vec![],
                documentation,
                visibility: Visibility::Public, // Default to public
                range: Range::line_range(line_num, end_line),
                calls,
            });
        }

        // Extract DEFINE TEMP-TABLE (as data structures)
        for cap in self.define_pattern.captures_iter(content) {
            let define_type = &cap[2];
            let name = &cap[3];
            let match_start = cap.get(0).unwrap().start();

            // Only extract significant defines
            if define_type.eq_ignore_ascii_case("TEMP-TABLE")
                || define_type.eq_ignore_ascii_case("BUFFER")
                || define_type.eq_ignore_ascii_case("DATASET")
            {
                let line_num = content[..match_start].matches('\n').count();
                let documentation = self.extract_doc_comment(&lines, line_num);

                let kind = match define_type.to_uppercase().as_str() {
                    "TEMP-TABLE" | "WORK-TABLE" => SymbolKind::Struct,
                    "BUFFER" => SymbolKind::Variable,
                    "DATASET" => SymbolKind::Struct,
                    _ => SymbolKind::Variable,
                };

                symbols.push(ExtractedSymbol {
                    name: name.to_string(),
                    kind,
                    signature: format!("DEFINE {} {}", define_type.to_uppercase(), name),
                    return_type: None,
                    parameters: vec![],
                    documentation,
                    visibility: Visibility::Private, // Data structures default to private
                    range: Range::line_range(line_num, line_num + 1),
                    calls: vec![],
                });
            }
        }

        // Extract triggers
        for cap in self.trigger_pattern.captures_iter(content) {
            let event = &cap[1];
            let table = &cap[2];
            let match_start = cap.get(0).unwrap().start();

            let line_num = content[..match_start].matches('\n').count();
            let documentation = self.extract_doc_comment(&lines, line_num);

            symbols.push(ExtractedSymbol {
                name: format!("{}-{}", event.to_lowercase(), table),
                kind: SymbolKind::Function,
                signature: format!("ON {} OF {}", event, table),
                return_type: None,
                parameters: vec![],
                documentation,
                visibility: Visibility::Private, // Triggers are internal
                range: Range::line_range(line_num, line_num + 10), // Estimate
                calls: vec![],
            });
        }

        Ok(symbols)
    }

    fn extract_imports(&self, content: &str) -> PluginResult<Vec<Import>> {
        let mut imports = Vec::new();

        // Extract {include.i} references
        for cap in self.include_pattern.captures_iter(content) {
            let include_file = &cap[1];
            let match_start = cap.get(0).unwrap().start();
            let line_num = content[..match_start].matches('\n').count();

            imports.push(Import {
                module: include_file.to_string(),
                items: vec![],
                alias: None,
                line: line_num,
            });
        }

        // Extract RUN statements as dependencies
        for cap in self.run_pattern.captures_iter(content) {
            let proc_name = &cap[1];
            let match_start = cap.get(0).unwrap().start();
            let line_num = content[..match_start].matches('\n').count();

            // Avoid duplicates
            if !imports.iter().any(|i| i.module == *proc_name) {
                imports.push(Import {
                    module: proc_name.to_string(),
                    items: vec!["RUN".to_string()],
                    alias: None,
                    line: line_num,
                });
            }
        }

        Ok(imports)
    }

    fn file_info(&self, content: &str) -> PluginResult<FileInfo> {
        let symbols = self.extract_symbols(content)?;
        let line_count = content.lines().count();

        // Detect file type by extension patterns in content or structure
        let has_class = self.class_pattern.is_match(content);
        let has_procedure = self.procedure_pattern.is_match(content);

        let dialect = if has_class {
            Some("oo".to_string()) // Object-oriented ABL
        } else if has_procedure {
            Some("procedural".to_string())
        } else {
            None
        };

        // Count FOR EACH patterns (database access)
        let for_each_count = self.for_each_pattern.find_iter(content).count();
        let run_count = self.run_pattern.find_iter(content).count();

        let mut metadata = HashMap::new();
        metadata.insert("for_each_count".to_string(), for_each_count.to_string());
        metadata.insert("run_count".to_string(), run_count.to_string());

        Ok(FileInfo {
            language: "abl".to_string(),
            dialect,
            symbol_count: symbols.len(),
            line_count,
            is_test: content.to_lowercase().contains("/* test")
                || content.to_lowercase().contains("ablunit"),
            is_executable: has_procedure && !content.trim().starts_with("/*"),
            metadata,
        })
    }

    // =========================================================================
    // Semantic Mapping (ABL-Specific)
    // =========================================================================

    fn infer_concept_type(&self, symbol: &ExtractedSymbol, content: &str) -> ConceptType {
        // First check ABL-specific name patterns - these take priority
        let name_concept = self.classify_abl_name(&symbol.name);

        // Strong name-based signals should not be overridden
        // Calculation, Validation, ErrorHandling, Logging are strong signals
        match name_concept {
            ConceptType::Calculation | ConceptType::Validation |
            ConceptType::ErrorHandling | ConceptType::Logging |
            ConceptType::Configuration | ConceptType::Testing => {
                return name_concept;
            }
            _ => {}
        }

        // For weaker signals, check content patterns
        let start_byte = content.lines()
            .take(symbol.range.start_line)
            .map(|l| l.len() + 1)
            .sum::<usize>();
        let end_byte = content.lines()
            .take(symbol.range.end_line.min(content.lines().count()))
            .map(|l| l.len() + 1)
            .sum::<usize>();

        let block = &content[start_byte..end_byte.min(content.len())];

        // If name suggests Decision/Transformation, verify with content patterns
        if name_concept == ConceptType::Transformation {
            return ConceptType::Transformation;
        }

        if name_concept == ConceptType::Decision {
            return ConceptType::Decision;
        }

        // No strong name pattern - use content patterns
        // FOR EACH patterns indicate data transformation
        if self.for_each_pattern.is_match(block) {
            return ConceptType::Transformation;
        }

        // RUN statements might indicate orchestration/decision
        if self.run_pattern.find_iter(block).count() > 2 {
            return ConceptType::Decision;
        }

        // OUTPUT statements might indicate data export
        if block.to_lowercase().contains("output to") {
            return ConceptType::Infrastructure;
        }

        // Default to the name-based classification
        name_concept
    }

    fn semantic_relevance_boost(
        &self,
        symbol: &ExtractedSymbol,
        intent: &str,
        content: &str,
    ) -> f32 {
        let mut boost = 0.0f32;

        match intent.to_lowercase().as_str() {
            "business-logic" | "businesslogic" => {
                // Boost procedures with calculations
                if symbol.name.to_lowercase().contains("calc")
                    || symbol.name.to_lowercase().contains("total")
                    || symbol.name.to_lowercase().contains("price")
                {
                    boost += 0.2;
                }
                // Boost procedures with validation
                if symbol.name.to_lowercase().contains("valid") {
                    boost += 0.15;
                }
            }
            "debugging" | "debug" => {
                // Boost error handling procedures
                if symbol.name.to_lowercase().contains("error") {
                    boost += 0.3;
                }
                // Boost procedures with CATCH/FINALLY
                let start = symbol.range.start_line;
                let block = content.lines().skip(start).take(50).collect::<Vec<_>>().join("\n");
                if block.to_lowercase().contains("catch") {
                    boost += 0.2;
                }
            }
            "onboarding" | "learn" => {
                // Boost main/entry procedures
                if symbol.name.to_lowercase() == "main"
                    || symbol.name.to_lowercase().contains("process")
                    || symbol.name.to_lowercase().contains("run")
                {
                    boost += 0.2;
                }
                // Boost if has documentation
                if symbol.documentation.is_some() {
                    boost += 0.15;
                }
            }
            "security" | "security-review" => {
                // Boost validation procedures
                if symbol.name.to_lowercase().contains("valid")
                    || symbol.name.to_lowercase().contains("auth")
                    || symbol.name.to_lowercase().contains("check")
                {
                    boost += 0.25;
                }
            }
            "migration" | "migration-assessment" => {
                // Boost infrastructure procedures
                if symbol.name.to_lowercase().contains("connect")
                    || symbol.name.to_lowercase().contains("db")
                    || symbol.name.to_lowercase().contains("file")
                {
                    boost += 0.3;
                }
            }
            _ => {}
        }

        boost.clamp(-0.5, 0.5)
    }

    fn language_features(&self, symbol: &ExtractedSymbol, content: &str) -> Vec<(usize, f32)> {
        let mut features = Vec::new();

        // Feature indices (using 50-63 for language-specific features)
        const FEAT_FOR_EACH_DENSITY: usize = 50;    // Database access patterns
        const FEAT_RUN_CALLS: usize = 51;           // External procedure calls
        const FEAT_ERROR_HANDLING: usize = 52;      // CATCH/UNDO/RETRY patterns
        const FEAT_TRANSACTION: usize = 53;         // Transaction blocks
        const FEAT_BUFFER_OPS: usize = 54;          // Buffer operations

        // Extract the symbol's code block
        let start = symbol.range.start_line;
        let end = symbol.range.end_line.min(content.lines().count());
        let block: String = content.lines()
            .skip(start)
            .take(end - start + 1)
            .collect::<Vec<_>>()
            .join("\n");

        let block_lower = block.to_lowercase();
        let lines = end - start + 1;

        // FOR EACH density (normalized by lines)
        let for_each_count = self.for_each_pattern.find_iter(&block).count();
        let for_each_density = (for_each_count as f32 / lines as f32 * 10.0).min(1.0);
        features.push((FEAT_FOR_EACH_DENSITY, for_each_density));

        // RUN calls (normalized)
        let run_count = self.run_pattern.find_iter(&block).count();
        let run_density = (run_count as f32 / lines as f32 * 5.0).min(1.0);
        features.push((FEAT_RUN_CALLS, run_density));

        // Error handling patterns
        let has_catch = block_lower.contains("catch") || block_lower.contains("undo, throw");
        let has_retry = block_lower.contains("retry");
        let error_score = if has_catch && has_retry { 1.0 } else if has_catch { 0.7 } else { 0.0 };
        features.push((FEAT_ERROR_HANDLING, error_score));

        // Transaction patterns
        let has_transaction = block_lower.contains("do transaction");
        let has_validate = block_lower.contains("validate");
        let transaction_score = if has_transaction { 0.8 } else if has_validate { 0.5 } else { 0.0 };
        features.push((FEAT_TRANSACTION, transaction_score));

        // Buffer operations
        let buffer_ops = block_lower.matches("buffer-copy").count()
            + block_lower.matches("buffer-compare").count()
            + block_lower.matches("find ").count();
        let buffer_score = (buffer_ops as f32 / 5.0).min(1.0);
        features.push((FEAT_BUFFER_OPS, buffer_score));

        features
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const ABL_SAMPLE: &str = r#"
/* Order processing module */
/* Author: Business Team */

{order-defs.i}

DEFINE TEMP-TABLE tt-order-line NO-UNDO
    FIELD order-id AS INTEGER
    FIELD line-num AS INTEGER
    FIELD item-id  AS INTEGER
    FIELD qty      AS DECIMAL
    FIELD price    AS DECIMAL.

/* Calculate order total with tax */
PROCEDURE calculate-order-total:
    DEFINE INPUT PARAMETER ip-order-id AS INTEGER.
    DEFINE OUTPUT PARAMETER op-total AS DECIMAL.

    DEFINE VARIABLE v-subtotal AS DECIMAL NO-UNDO.

    FOR EACH order-line WHERE order-line.order-id = ip-order-id:
        v-subtotal = v-subtotal + (order-line.qty * order-line.price).
    END.

    op-total = v-subtotal * 1.08. /* Add 8% tax */
END PROCEDURE.

/* Validate order data */
PROCEDURE validate-order:
    DEFINE INPUT PARAMETER ip-order-id AS INTEGER.
    DEFINE OUTPUT PARAMETER op-valid AS LOGICAL.

    op-valid = TRUE.

    FOR EACH order-line WHERE order-line.order-id = ip-order-id:
        IF order-line.qty <= 0 THEN
            op-valid = FALSE.
    END.
END PROCEDURE.

FUNCTION format-currency RETURNS CHARACTER (ip-amount AS DECIMAL):
    RETURN STRING(ip-amount, ">>>,>>9.99").
END FUNCTION.
"#;

    #[test]
    fn test_abl_plugin_extensions() {
        let plugin = AblPlugin::new();
        assert!(plugin.supports_file(std::path::Path::new("file.p")));
        assert!(plugin.supports_file(std::path::Path::new("file.w")));
        assert!(plugin.supports_file(std::path::Path::new("file.i")));
        assert!(plugin.supports_file(std::path::Path::new("file.cls")));
        assert!(!plugin.supports_file(std::path::Path::new("file.rs")));
    }

    #[test]
    fn test_extract_procedures() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let procedures: Vec<_> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Function && s.signature.starts_with("PROCEDURE"))
            .collect();

        assert_eq!(procedures.len(), 2, "Should find 2 procedures");

        let calc_proc = procedures.iter().find(|s| s.name == "calculate-order-total");
        assert!(calc_proc.is_some(), "Should find calculate-order-total");

        let validate_proc = procedures.iter().find(|s| s.name == "validate-order");
        assert!(validate_proc.is_some(), "Should find validate-order");
    }

    #[test]
    fn test_extract_functions() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let functions: Vec<_> = symbols.iter()
            .filter(|s| s.signature.starts_with("FUNCTION"))
            .collect();

        assert_eq!(functions.len(), 1, "Should find 1 function");
        assert_eq!(functions[0].name, "format-currency");
        assert_eq!(functions[0].return_type.as_deref(), Some("CHARACTER"));
    }

    #[test]
    fn test_extract_temp_table() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let temp_tables: Vec<_> = symbols.iter()
            .filter(|s| s.signature.contains("TEMP-TABLE"))
            .collect();

        assert_eq!(temp_tables.len(), 1, "Should find 1 temp-table");
        assert_eq!(temp_tables[0].name, "tt-order-line");
    }

    #[test]
    fn test_extract_imports() {
        let plugin = AblPlugin::new();
        let imports = plugin.extract_imports(ABL_SAMPLE).unwrap();

        let include = imports.iter().find(|i| i.module.ends_with(".i"));
        assert!(include.is_some(), "Should find include file");
        assert_eq!(include.unwrap().module, "order-defs.i");
    }

    #[test]
    fn test_extract_parameters() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let calc_proc = symbols.iter()
            .find(|s| s.name == "calculate-order-total")
            .unwrap();

        assert_eq!(calc_proc.parameters.len(), 2, "Should have 2 parameters");

        let input_param = calc_proc.parameters.iter()
            .find(|p| p.name == "ip-order-id");
        assert!(input_param.is_some());
        assert!(input_param.unwrap().type_hint.as_ref().unwrap().contains("INPUT"));
    }

    #[test]
    fn test_concept_classification() {
        let plugin = AblPlugin::new();

        assert_eq!(plugin.classify_abl_name("calculate-order-total"), ConceptType::Calculation);
        assert_eq!(plugin.classify_abl_name("validate-order"), ConceptType::Validation);
        assert_eq!(plugin.classify_abl_name("handle-error"), ConceptType::ErrorHandling);
        assert_eq!(plugin.classify_abl_name("log-transaction"), ConceptType::Logging);
        assert_eq!(plugin.classify_abl_name("init-database"), ConceptType::Configuration);
        assert_eq!(plugin.classify_abl_name("format-currency"), ConceptType::Transformation);
        assert_eq!(plugin.classify_abl_name("process-order"), ConceptType::Decision);
    }

    #[test]
    fn test_infer_concept_type() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let calc_proc = symbols.iter().find(|s| s.name == "calculate-order-total").unwrap();
        assert_eq!(plugin.infer_concept_type(calc_proc, ABL_SAMPLE), ConceptType::Calculation);

        let validate_proc = symbols.iter().find(|s| s.name == "validate-order").unwrap();
        assert_eq!(plugin.infer_concept_type(validate_proc, ABL_SAMPLE), ConceptType::Validation);
    }

    #[test]
    fn test_semantic_relevance_boost() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let calc_proc = symbols.iter().find(|s| s.name == "calculate-order-total").unwrap();

        // Should get boost for business-logic intent
        let boost = plugin.semantic_relevance_boost(calc_proc, "business-logic", ABL_SAMPLE);
        assert!(boost > 0.0, "Should have positive boost for business-logic");
    }

    #[test]
    fn test_language_features() {
        let plugin = AblPlugin::new();
        let symbols = plugin.extract_symbols(ABL_SAMPLE).unwrap();

        let calc_proc = symbols.iter().find(|s| s.name == "calculate-order-total").unwrap();
        let features = plugin.language_features(calc_proc, ABL_SAMPLE);

        // Should have FOR EACH feature
        let for_each_feat = features.iter().find(|(idx, _)| *idx == 50);
        assert!(for_each_feat.is_some(), "Should have FOR EACH feature");
        assert!(for_each_feat.unwrap().1 > 0.0, "FOR EACH feature should be > 0");
    }

    #[test]
    fn test_file_info() {
        let plugin = AblPlugin::new();
        let info = plugin.file_info(ABL_SAMPLE).unwrap();

        assert_eq!(info.language, "abl");
        assert!(info.symbol_count > 0);
        assert!(!info.is_test);
        assert!(info.metadata.contains_key("for_each_count"));
    }
}
