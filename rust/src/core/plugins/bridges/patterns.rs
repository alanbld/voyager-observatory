//! Lua Bridge for Universal Spectrograph
//!
//! This module provides the Lua API for accessing spectral signatures
//! from the Universal Spectrograph. It creates the `vo.patterns.*` table
//! that plugins can use to access language patterns.
//!
//! The core pattern definitions live in `core::spectrograph`, and this
//! module just provides the Lua bridge when the `plugins` feature is enabled.

use mlua::{Lua, Result as LuaResult, Table};

// Re-export core types for backwards compatibility
pub use crate::core::spectrograph::{
    Hemisphere, SpectralSignature, StellarLibrary, STELLAR_LIBRARY,
};

/// Create the patterns table with all spectral signatures for Lua plugins
pub fn create_patterns_table(lua: &Lua) -> LuaResult<Table> {
    let patterns = lua.create_table()?;

    // -------------------------------------------------------------------------
    // TIER 1: THE GIANTS (Modern Core)
    // -------------------------------------------------------------------------

    // Rust patterns
    patterns.set("rust_fn", r#"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)"#)?;
    patterns.set("rust_struct", r#"(?:pub\s+)?struct\s+(\w+)"#)?;
    patterns.set("rust_enum", r#"(?:pub\s+)?enum\s+(\w+)"#)?;
    patterns.set(
        "rust_impl",
        r#"impl(?:<[^>]+>)?\s+(?:(\w+)\s+for\s+)?(\w+)"#,
    )?;
    patterns.set("rust_trait", r#"(?:pub\s+)?trait\s+(\w+)"#)?;
    patterns.set("rust_mod", r#"(?:pub\s+)?mod\s+(\w+)"#)?;
    patterns.set("rust_use", r#"use\s+([^;]+)"#)?;
    patterns.set("rust_const", r#"(?:pub\s+)?const\s+(\w+)"#)?;
    patterns.set("rust_static", r#"(?:pub\s+)?static\s+(\w+)"#)?;
    patterns.set("rust_type", r#"(?:pub\s+)?type\s+(\w+)"#)?;

    // Python patterns
    patterns.set("python_def", r#"(?:async\s+)?def\s+(\w+)"#)?;
    patterns.set("python_class", r#"class\s+(\w+)"#)?;
    patterns.set("python_import", r#"(?:from\s+[\w.]+\s+)?import\s+(.+)"#)?;
    patterns.set("python_decorator", r#"@(\w+)"#)?;
    patterns.set("python_async_def", r#"async\s+def\s+(\w+)"#)?;

    // JavaScript patterns
    patterns.set("js_function", r#"(?:async\s+)?function\s+(\w+)"#)?;
    patterns.set("js_const", r#"const\s+(\w+)"#)?;
    patterns.set("js_let", r#"let\s+(\w+)"#)?;
    patterns.set("js_class", r#"class\s+(\w+)"#)?;
    patterns.set(
        "js_arrow",
        r#"(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>"#,
    )?;
    patterns.set(
        "js_import",
        r#"import\s+(?:\{[^}]+\}|\*\s+as\s+\w+|\w+)\s+from\s+["']([^"']+)["']"#,
    )?;
    patterns.set(
        "js_export",
        r#"export\s+(?:default\s+)?(?:function|class|const|let|var)\s+(\w+)"#,
    )?;

    // TypeScript patterns
    patterns.set("ts_interface", r#"(?:export\s+)?interface\s+(\w+)"#)?;
    patterns.set("ts_type", r#"(?:export\s+)?type\s+(\w+)"#)?;
    patterns.set("ts_enum", r#"(?:export\s+)?enum\s+(\w+)"#)?;

    // Java patterns
    patterns.set(
        "java_class",
        r#"(?:public|private|protected)?\s*(?:abstract\s+)?class\s+(\w+)"#,
    )?;
    patterns.set(
        "java_interface",
        r#"(?:public|private|protected)?\s*interface\s+(\w+)"#,
    )?;
    patterns.set(
        "java_method",
        r#"(?:public|private|protected)?\s*(?:static\s+)?(?:\w+\s+)+(\w+)\s*\("#,
    )?;
    patterns.set("java_import", r#"import\s+([^;]+)"#)?;
    patterns.set("java_package", r#"package\s+([^;]+)"#)?;

    // C# patterns
    patterns.set(
        "csharp_class",
        r#"(?:public|private|protected|internal)?\s*(?:partial\s+)?class\s+(\w+)"#,
    )?;
    patterns.set(
        "csharp_interface",
        r#"(?:public|private|protected|internal)?\s*interface\s+(\w+)"#,
    )?;
    patterns.set("csharp_method", r#"(?:public|private|protected|internal)?\s*(?:static\s+)?(?:async\s+)?(?:\w+\s+)+(\w+)\s*\("#)?;
    patterns.set(
        "csharp_struct",
        r#"(?:public|private|protected|internal)?\s*struct\s+(\w+)"#,
    )?;
    patterns.set(
        "csharp_enum",
        r#"(?:public|private|protected|internal)?\s*enum\s+(\w+)"#,
    )?;

    // C++ patterns
    patterns.set("cpp_class", r#"class\s+(\w+)"#)?;
    patterns.set("cpp_struct", r#"struct\s+(\w+)"#)?;
    patterns.set(
        "cpp_function",
        r#"(?:\w+\s+)+(\w+)\s*\([^)]*\)\s*(?:const\s*)?(?:override\s*)?(?:final\s*)?\{"#,
    )?;
    patterns.set("cpp_namespace", r#"namespace\s+(\w+)"#)?;
    patterns.set("cpp_template", r#"template\s*<[^>]+>"#)?;

    // C patterns
    patterns.set("c_function", r#"(?:\w+\s+)+(\w+)\s*\([^)]*\)\s*\{"#)?;
    patterns.set("c_struct", r#"struct\s+(\w+)"#)?;
    patterns.set("c_typedef", r#"typedef\s+.+\s+(\w+)\s*;"#)?;
    patterns.set("c_define", r#"#define\s+(\w+)"#)?;

    // Go patterns
    patterns.set("go_func", r#"func\s+(?:\([^)]+\)\s+)?(\w+)"#)?;
    patterns.set("go_type", r#"type\s+(\w+)"#)?;
    patterns.set("go_struct", r#"type\s+(\w+)\s+struct"#)?;
    patterns.set("go_interface", r#"type\s+(\w+)\s+interface"#)?;
    patterns.set("go_package", r#"package\s+(\w+)"#)?;
    patterns.set("go_import", r#"import\s+(?:\(\s*)?["']([^"']+)["']"#)?;

    // PHP patterns
    patterns.set(
        "php_function",
        r#"(?:public|private|protected)?\s*(?:static\s+)?function\s+(\w+)"#,
    )?;
    patterns.set("php_class", r#"class\s+(\w+)"#)?;
    patterns.set("php_interface", r#"interface\s+(\w+)"#)?;
    patterns.set("php_trait", r#"trait\s+(\w+)"#)?;

    // Ruby patterns
    patterns.set("ruby_def", r#"def\s+(\w+[!?]?)"#)?;
    patterns.set("ruby_class", r#"class\s+(\w+)"#)?;
    patterns.set("ruby_module", r#"module\s+(\w+)"#)?;

    // Kotlin patterns
    patterns.set("kotlin_fun", r#"(?:suspend\s+)?fun\s+(\w+)"#)?;
    patterns.set("kotlin_class", r#"class\s+(\w+)"#)?;
    patterns.set("kotlin_interface", r#"interface\s+(\w+)"#)?;
    patterns.set("kotlin_object", r#"object\s+(\w+)"#)?;

    // Swift patterns
    patterns.set("swift_func", r#"func\s+(\w+)"#)?;
    patterns.set("swift_class", r#"class\s+(\w+)"#)?;
    patterns.set("swift_struct", r#"struct\s+(\w+)"#)?;
    patterns.set("swift_protocol", r#"protocol\s+(\w+)"#)?;
    patterns.set("swift_enum", r#"enum\s+(\w+)"#)?;

    // Dart patterns
    patterns.set("dart_class", r#"class\s+(\w+)"#)?;
    patterns.set("dart_function", r#"(\w+)\s+(\w+)\s*\("#)?;
    patterns.set("dart_mixin", r#"mixin\s+(\w+)"#)?;

    // -------------------------------------------------------------------------
    // TIER 2: INFRASTRUCTURE & AUTOMATION
    // -------------------------------------------------------------------------

    // Shell/Bash patterns
    patterns.set("bash_function", r#"(?:function\s+)?(\w+)\s*\(\s*\)"#)?;

    // Tcl patterns
    patterns.set("tcl_proc", r#"proc\s+([^\s\{]+)"#)?;

    // PowerShell patterns
    patterns.set("powershell_function", r#"function\s+(\w+(?:-\w+)*)"#)?;
    patterns.set("powershell_class", r#"class\s+(\w+)"#)?;

    // Makefile patterns
    patterns.set("makefile_target", r#"^(\w+)\s*:"#)?;

    // Docker patterns
    patterns.set(
        "dockerfile_instruction",
        r#"^(FROM|RUN|CMD|ENTRYPOINT|COPY|ADD|ENV|EXPOSE|WORKDIR|LABEL)"#,
    )?;

    // Terraform/HCL patterns
    patterns.set(
        "hcl_resource",
        r#"(?:resource|data|variable|output|module|provider)\s+"(\w+)""#,
    )?;

    // Nix patterns
    patterns.set("nix_binding", r#"(\w+)\s*=\s*(?:\{|let|rec)"#)?;

    // -------------------------------------------------------------------------
    // TIER 3: ANCIENT STARS (Legacy Kings)
    // -------------------------------------------------------------------------

    // COBOL patterns
    patterns.set(
        "cobol_procedure",
        r#"(?:PROCEDURE|SECTION|PARAGRAPH)\s+(\w+(?:-\w+)*)"#,
    )?;
    patterns.set("cobol_division", r#"(\w+(?:-\w+)*)\s+DIVISION"#)?;

    // Simula patterns (The OO Pioneer!)
    patterns.set("simula_procedure", r#"procedure\s+(\w+)"#)?;
    patterns.set("simula_class", r#"class\s+(\w+)"#)?;

    // Logo patterns (Turtle Graphics!)
    patterns.set("logo_to", r#"to\s+(\w+)"#)?;

    // Fortran patterns
    patterns.set("fortran_subroutine", r#"subroutine\s+(\w+)"#)?;
    patterns.set("fortran_function", r#"function\s+(\w+)"#)?;
    patterns.set("fortran_program", r#"program\s+(\w+)"#)?;
    patterns.set("fortran_module", r#"module\s+(\w+)"#)?;

    // Ada patterns
    patterns.set("ada_procedure", r#"procedure\s+(\w+)"#)?;
    patterns.set("ada_function", r#"function\s+(\w+)"#)?;
    patterns.set("ada_package", r#"package\s+(\w+)"#)?;

    // Pascal patterns
    patterns.set("pascal_procedure", r#"procedure\s+(\w+)"#)?;
    patterns.set("pascal_function", r#"function\s+(\w+)"#)?;
    patterns.set("pascal_program", r#"program\s+(\w+)"#)?;
    patterns.set("pascal_unit", r#"unit\s+(\w+)"#)?;

    // Lisp patterns
    patterns.set("lisp_defun", r#"\(defun\s+(\w+(?:-\w+)*)"#)?;
    patterns.set("lisp_defmacro", r#"\(defmacro\s+(\w+(?:-\w+)*)"#)?;

    // Prolog patterns
    patterns.set("prolog_predicate", r#"(\w+)\s*\([^)]*\)\s*:-"#)?;

    // VHDL patterns
    patterns.set("vhdl_entity", r#"entity\s+(\w+)"#)?;
    patterns.set("vhdl_architecture", r#"architecture\s+(\w+)"#)?;
    patterns.set("vhdl_process", r#"process\s*(?:\([^)]*\))?"#)?;

    // Verilog patterns
    patterns.set("verilog_module", r#"module\s+(\w+)"#)?;
    patterns.set("verilog_task", r#"task\s+(\w+)"#)?;
    patterns.set("verilog_function", r#"function\s+(\w+)"#)?;

    // RPG patterns
    patterns.set("rpg_procedure", r#"DCL-PROC\s+(\w+)"#)?;
    patterns.set("rpg_subroutine", r#"BEGSR\s+(\w+)"#)?;

    // ABAP patterns
    patterns.set("abap_form", r#"FORM\s+(\w+)"#)?;
    patterns.set("abap_method", r#"METHOD\s+(\w+)"#)?;
    patterns.set("abap_class", r#"CLASS\s+(\w+)"#)?;

    // -------------------------------------------------------------------------
    // TIER 4: FUNCTIONAL & LOGIC
    // -------------------------------------------------------------------------

    // Haskell patterns
    patterns.set("haskell_function", r#"(\w+)\s*::"#)?;
    patterns.set("haskell_data", r#"data\s+(\w+)"#)?;
    patterns.set("haskell_newtype", r#"newtype\s+(\w+)"#)?;
    patterns.set("haskell_class", r#"class\s+(\w+)"#)?;

    // Elixir patterns
    patterns.set("elixir_def", r#"def(?:p)?\s+(\w+)"#)?;
    patterns.set("elixir_defmodule", r#"defmodule\s+(\w+)"#)?;

    // Erlang patterns
    patterns.set("erlang_function", r#"(\w+)\s*\([^)]*\)\s*->"#)?;

    // Clojure patterns
    patterns.set("clojure_defn", r#"\(defn\s+(\w+(?:-\w+)*)"#)?;
    patterns.set("clojure_defmacro", r#"\(defmacro\s+(\w+(?:-\w+)*)"#)?;

    // Scala patterns
    patterns.set("scala_def", r#"def\s+(\w+)"#)?;
    patterns.set("scala_class", r#"class\s+(\w+)"#)?;
    patterns.set("scala_object", r#"object\s+(\w+)"#)?;
    patterns.set("scala_trait", r#"trait\s+(\w+)"#)?;

    // F# patterns
    patterns.set("fsharp_let", r#"let\s+(?:rec\s+)?(\w+)"#)?;
    patterns.set("fsharp_type", r#"type\s+(\w+)"#)?;
    patterns.set("fsharp_module", r#"module\s+(\w+)"#)?;

    // OCaml patterns
    patterns.set("ocaml_let", r#"let\s+(?:rec\s+)?(\w+)"#)?;
    patterns.set("ocaml_type", r#"type\s+(\w+)"#)?;
    patterns.set("ocaml_module", r#"module\s+(\w+)"#)?;

    // Scheme patterns
    patterns.set("scheme_define", r#"\(define\s+(?:\((\w+)|\(\s*(\w+))"#)?;

    // -------------------------------------------------------------------------
    // TIER 5: STELLAR NURSERIES (Emerging)
    // -------------------------------------------------------------------------

    // Zig patterns
    patterns.set("zig_fn", r#"(?:pub\s+)?fn\s+(\w+)"#)?;
    patterns.set("zig_const", r#"(?:pub\s+)?const\s+(\w+)\s*="#)?;

    // Nim patterns
    patterns.set("nim_proc", r#"proc\s+(\w+)"#)?;
    patterns.set("nim_func", r#"func\s+(\w+)"#)?;
    patterns.set("nim_type", r#"type\s+(\w+)"#)?;

    // Crystal patterns
    patterns.set("crystal_def", r#"def\s+(\w+[!?]?)"#)?;
    patterns.set("crystal_class", r#"class\s+(\w+)"#)?;

    // V patterns
    patterns.set("v_fn", r#"fn\s+(\w+)"#)?;
    patterns.set("v_struct", r#"struct\s+(\w+)"#)?;

    // Elm patterns
    patterns.set("elm_function", r#"(\w+)\s*:"#)?;
    patterns.set("elm_type", r#"type\s+(?:alias\s+)?(\w+)"#)?;

    // Solidity patterns
    patterns.set("solidity_function", r#"function\s+(\w+)"#)?;
    patterns.set("solidity_contract", r#"contract\s+(\w+)"#)?;
    patterns.set("solidity_event", r#"event\s+(\w+)"#)?;

    // Gleam patterns
    patterns.set("gleam_fn", r#"(?:pub\s+)?fn\s+(\w+)"#)?;
    patterns.set("gleam_type", r#"(?:pub\s+)?type\s+(\w+)"#)?;

    // -------------------------------------------------------------------------
    // TIER 6: SCIENTIFIC & SCRIPTING
    // -------------------------------------------------------------------------

    // GraphQL patterns
    patterns.set("graphql_type", r#"type\s+(\w+)"#)?;
    patterns.set("graphql_query", r#"query\s+(\w+)"#)?;
    patterns.set("graphql_mutation", r#"mutation\s+(\w+)"#)?;
    patterns.set("graphql_interface", r#"interface\s+(\w+)"#)?;

    // Protobuf patterns
    patterns.set("protobuf_message", r#"message\s+(\w+)"#)?;
    patterns.set("protobuf_service", r#"service\s+(\w+)"#)?;
    patterns.set("protobuf_enum", r#"enum\s+(\w+)"#)?;

    // Lua patterns
    patterns.set("lua_function", r#"(?:local\s+)?function\s+(\w+(?:\.\w+)*)"#)?;

    // Perl patterns
    patterns.set("perl_sub", r#"sub\s+(\w+)"#)?;
    patterns.set("perl_package", r#"package\s+(\w+)"#)?;

    // R patterns
    patterns.set("r_function", r#"(\w+)\s*<-\s*function"#)?;

    // Julia patterns
    patterns.set("julia_function", r#"function\s+(\w+)"#)?;
    patterns.set("julia_struct", r#"struct\s+(\w+)"#)?;
    patterns.set("julia_macro", r#"macro\s+(\w+)"#)?;

    // MATLAB patterns
    patterns.set(
        "matlab_function",
        r#"function\s+(?:\[?[^\]]*\]?\s*=\s*)?(\w+)"#,
    )?;
    patterns.set("matlab_classdef", r#"classdef\s+(\w+)"#)?;

    // Groovy patterns
    patterns.set("groovy_def", r#"def\s+(\w+)"#)?;
    patterns.set("groovy_class", r#"class\s+(\w+)"#)?;

    // -------------------------------------------------------------------------
    // COMMON PATTERNS (Cross-language)
    // -------------------------------------------------------------------------

    patterns.set("todo_comment", r#"(?://|#|/\*|--|;)\s*TODO[:\s]"#)?;
    patterns.set("fixme_comment", r#"(?://|#|/\*|--|;)\s*FIXME[:\s]"#)?;
    patterns.set("hack_comment", r#"(?://|#|/\*|--|;)\s*HACK[:\s]"#)?;
    patterns.set("note_comment", r#"(?://|#|/\*|--|;)\s*NOTE[:\s]"#)?;
    patterns.set("bug_comment", r#"(?://|#|/\*|--|;)\s*BUG[:\s]"#)?;
    patterns.set("xxx_comment", r#"(?://|#|/\*|--|;)\s*XXX[:\s]"#)?;
    patterns.set("url", r#"https?://[^\s<>"{}|\\^`\[\]]+"#)?;
    patterns.set(
        "email",
        r#"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"#,
    )?;
    patterns.set("semver", r#"\d+\.\d+\.\d+(?:-[\w.]+)?(?:\+[\w.]+)?"#)?;
    patterns.set(
        "uuid",
        r#"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"#,
    )?;

    Ok(patterns)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_patterns_table_creation() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        // Verify core patterns exist
        let rust_fn: String = patterns.get("rust_fn").unwrap();
        assert!(rust_fn.contains("fn"));

        let python_def: String = patterns.get("python_def").unwrap();
        assert!(python_def.contains("def"));

        // Verify ancient star patterns
        let simula_class: String = patterns.get("simula_class").unwrap();
        assert!(simula_class.contains("class"));

        let logo_to: String = patterns.get("logo_to").unwrap();
        assert!(logo_to.contains("to"));

        let tcl_proc: String = patterns.get("tcl_proc").unwrap();
        assert!(tcl_proc.contains("proc"));
    }

    // === TIER 1: Modern Core Tests ===

    #[test]
    fn test_rust_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let rust_patterns = [
            "rust_fn", "rust_struct", "rust_enum", "rust_impl",
            "rust_trait", "rust_mod", "rust_use", "rust_const",
            "rust_static", "rust_type",
        ];

        for name in rust_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_rust_fn_pattern_matches() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();
        let pattern: String = patterns.get("rust_fn").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("fn main()"));
        assert!(re.is_match("pub fn helper()"));
        assert!(re.is_match("async fn async_fn()"));
        assert!(re.is_match("pub async fn pub_async_fn()"));
    }

    #[test]
    fn test_python_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let python_patterns = [
            "python_def", "python_class", "python_import",
            "python_decorator", "python_async_def",
        ];

        for name in python_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_python_def_pattern_matches() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();
        let pattern: String = patterns.get("python_def").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("def my_function():"));
        assert!(re.is_match("async def async_func():"));
    }

    #[test]
    fn test_javascript_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let js_patterns = [
            "js_function", "js_const", "js_let", "js_class",
            "js_arrow", "js_import", "js_export",
        ];

        for name in js_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_typescript_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let ts_patterns = ["ts_interface", "ts_type", "ts_enum"];

        for name in ts_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_java_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let java_patterns = [
            "java_class", "java_interface", "java_method",
            "java_import", "java_package",
        ];

        for name in java_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_csharp_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let csharp_patterns = [
            "csharp_class", "csharp_interface", "csharp_method",
            "csharp_struct", "csharp_enum",
        ];

        for name in csharp_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_cpp_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let cpp_patterns = [
            "cpp_class", "cpp_struct", "cpp_function",
            "cpp_namespace", "cpp_template",
        ];

        for name in cpp_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_go_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let go_patterns = [
            "go_func", "go_type", "go_struct",
            "go_interface", "go_package", "go_import",
        ];

        for name in go_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    // === TIER 2: Infrastructure & Automation ===

    #[test]
    fn test_shell_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let pattern: String = patterns.get("bash_function").unwrap();
        assert!(Regex::new(&pattern).is_ok());

        let pattern: String = patterns.get("powershell_function").unwrap();
        assert!(Regex::new(&pattern).is_ok());
    }

    #[test]
    fn test_hcl_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let pattern: String = patterns.get("hcl_resource").unwrap();
        assert!(Regex::new(&pattern).is_ok());
    }

    #[test]
    fn test_dockerfile_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let pattern: String = patterns.get("dockerfile_instruction").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("FROM ubuntu:22.04"));
        assert!(re.is_match("RUN apt-get update"));
        assert!(re.is_match("COPY . /app"));
    }

    // === TIER 3: Ancient Stars (Legacy Kings) ===

    #[test]
    fn test_cobol_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let cobol_patterns = ["cobol_procedure", "cobol_division"];

        for name in cobol_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_fortran_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let fortran_patterns = [
            "fortran_subroutine", "fortran_function",
            "fortran_program", "fortran_module",
        ];

        for name in fortran_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_pascal_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let pascal_patterns = [
            "pascal_procedure", "pascal_function",
            "pascal_program", "pascal_unit",
        ];

        for name in pascal_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_lisp_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let lisp_patterns = ["lisp_defun", "lisp_defmacro"];

        for name in lisp_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_vhdl_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let vhdl_patterns = ["vhdl_entity", "vhdl_architecture", "vhdl_process"];

        for name in vhdl_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    // === TIER 4: Functional & Logic ===

    #[test]
    fn test_haskell_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let haskell_patterns = [
            "haskell_function", "haskell_data",
            "haskell_newtype", "haskell_class",
        ];

        for name in haskell_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_elixir_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let elixir_patterns = ["elixir_def", "elixir_defmodule"];

        for name in elixir_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_scala_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let scala_patterns = [
            "scala_def", "scala_class", "scala_object", "scala_trait",
        ];

        for name in scala_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_clojure_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let clojure_patterns = ["clojure_defn", "clojure_defmacro"];

        for name in clojure_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    // === TIER 5: Stellar Nurseries (Emerging) ===

    #[test]
    fn test_zig_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let zig_patterns = ["zig_fn", "zig_const"];

        for name in zig_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_nim_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let nim_patterns = ["nim_proc", "nim_func", "nim_type"];

        for name in nim_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_solidity_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let solidity_patterns = ["solidity_function", "solidity_contract", "solidity_event"];

        for name in solidity_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    // === TIER 6: Scientific & Scripting ===

    #[test]
    fn test_graphql_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let graphql_patterns = [
            "graphql_type", "graphql_query",
            "graphql_mutation", "graphql_interface",
        ];

        for name in graphql_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_protobuf_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let protobuf_patterns = ["protobuf_message", "protobuf_service", "protobuf_enum"];

        for name in protobuf_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    #[test]
    fn test_julia_patterns_valid_regex() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let julia_patterns = ["julia_function", "julia_struct", "julia_macro"];

        for name in julia_patterns {
            let pattern: String = patterns.get(name).unwrap();
            assert!(Regex::new(&pattern).is_ok(), "Invalid regex for {}", name);
        }
    }

    // === Common Patterns Tests ===

    #[test]
    fn test_todo_comment_pattern() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();
        let pattern: String = patterns.get("todo_comment").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("// TODO: fix this"));
        assert!(re.is_match("# TODO: implement later"));
        assert!(re.is_match("/* TODO: refactor */"));
    }

    #[test]
    fn test_url_pattern() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();
        let pattern: String = patterns.get("url").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("https://example.com"));
        assert!(re.is_match("http://localhost:8080/api"));
    }

    #[test]
    fn test_semver_pattern() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();
        let pattern: String = patterns.get("semver").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("1.0.0"));
        assert!(re.is_match("2.3.4-beta.1"));
        assert!(re.is_match("1.0.0+build.123"));
    }

    #[test]
    fn test_uuid_pattern() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();
        let pattern: String = patterns.get("uuid").unwrap();
        let re = Regex::new(&pattern).unwrap();

        assert!(re.is_match("550e8400-e29b-41d4-a716-446655440000"));
        assert!(re.is_match("123e4567-e89b-12d3-a456-426614174000"));
    }

    // === Re-export Tests ===

    #[test]
    fn test_stellar_library_re_export() {
        // Test that re-exports from spectrograph work
        assert!(STELLAR_LIBRARY.get("rust").is_some());
        assert!(STELLAR_LIBRARY.get("python").is_some());
        assert!(STELLAR_LIBRARY.get("javascript").is_some());
    }

    #[test]
    fn test_hemisphere_re_export() {
        // Test Hemisphere enum is accessible
        let _logic = Hemisphere::Logic;
        let _interface = Hemisphere::Interface;
        let _automation = Hemisphere::Automation;
    }
}
