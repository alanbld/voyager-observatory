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
}
