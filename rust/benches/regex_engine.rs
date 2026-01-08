//! Benchmarks for the centralized Regex Engine
//!
//! These benchmarks validate the <1μs per-call overhead target for the regex engine bridge.
//! Run with: `cargo bench --bench regex_engine`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pm_encoder::core::regex_engine::{global_engine, CompiledRegex, PatternSet, RegexEngine};

// =============================================================================
// Benchmark Data
// =============================================================================

// Sample text for matching (simulates code content)
const SHORT_TEXT: &str = "user@example.com";
const MEDIUM_TEXT: &str = r#"
Contact us at: alice@example.com or bob@test.org
Phone: 555-123-4567 or 555-987-6543
Visit https://www.example.com for more info
"#;
const LONG_TEXT: &str = include_str!("../src/core/regex_engine.rs"); // Use our own source as test data

// Common patterns (from Shell/ABL plugins)
const SHELL_FUNCTION_PATTERN: &str = r"(?m)^[ \t]*(?:function\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(\s*\)\s*\{|^[ \t]*function\s+([a-zA-Z_][a-zA-Z0-9_]*)(?:\s*\(\s*\))?\s*\{";
const EMAIL_PATTERN: &str = r"(?P<user>\w+)@(?P<domain>\w+)\.(?P<tld>\w+)";
const SIMPLE_PATTERN: &str = r"\d+";
const WORD_PATTERN: &str = r"\w+";

// =============================================================================
// Cache Performance Benchmarks
// =============================================================================

fn bench_compile_cache_hit(c: &mut Criterion) {
    let engine = RegexEngine::new();

    // Warm up the cache
    let _ = engine.compile(EMAIL_PATTERN).unwrap();

    c.bench_function("compile_cache_hit", |b| {
        b.iter(|| engine.compile(black_box(EMAIL_PATTERN)).unwrap())
    });
}

fn bench_compile_cache_miss(c: &mut Criterion) {
    c.bench_function("compile_cache_miss", |b| {
        let engine = RegexEngine::new();
        let mut counter = 0;
        b.iter(|| {
            // Generate unique patterns to avoid cache hits
            let pattern = format!(r"\d{{{}}}unique", counter);
            counter += 1;
            engine.compile(black_box(&pattern)).unwrap()
        })
    });
}

fn bench_global_engine_compile(c: &mut Criterion) {
    // Warm up global engine
    let _ = global_engine().compile(EMAIL_PATTERN);

    c.bench_function("global_engine_compile", |b| {
        b.iter(|| global_engine().compile(black_box(EMAIL_PATTERN)).unwrap())
    });
}

// =============================================================================
// Matching Performance Benchmarks
// =============================================================================

fn bench_is_match(c: &mut Criterion) {
    let engine = RegexEngine::new();
    let regex = engine.compile(EMAIL_PATTERN).unwrap();

    let mut group = c.benchmark_group("is_match");

    group.bench_with_input(BenchmarkId::new("short", "17B"), SHORT_TEXT, |b, text| {
        b.iter(|| engine.is_match(&regex, black_box(text)))
    });

    group.bench_with_input(
        BenchmarkId::new("medium", "200B"),
        MEDIUM_TEXT,
        |b, text| b.iter(|| engine.is_match(&regex, black_box(text))),
    );

    group.bench_with_input(BenchmarkId::new("long", "~10KB"), LONG_TEXT, |b, text| {
        b.iter(|| engine.is_match(&regex, black_box(text)))
    });

    group.finish();
}

fn bench_find_iter(c: &mut Criterion) {
    let engine = RegexEngine::new();
    let regex = engine.compile(WORD_PATTERN).unwrap();

    let mut group = c.benchmark_group("find_iter");

    group.bench_with_input(
        BenchmarkId::new("medium", "200B"),
        MEDIUM_TEXT,
        |b, text| b.iter(|| engine.find_iter(&regex, black_box(text))),
    );

    group.bench_with_input(BenchmarkId::new("long", "~10KB"), LONG_TEXT, |b, text| {
        b.iter(|| engine.find_iter(&regex, black_box(text)))
    });

    group.finish();
}

fn bench_captures(c: &mut Criterion) {
    let engine = RegexEngine::new();
    let regex = engine.compile(EMAIL_PATTERN).unwrap();

    c.bench_function("match_captures", |b| {
        b.iter(|| engine.match_captures(&regex, black_box(SHORT_TEXT)))
    });
}

fn bench_captures_iter(c: &mut Criterion) {
    let engine = RegexEngine::new();
    let regex = engine.compile(EMAIL_PATTERN).unwrap();

    let mut group = c.benchmark_group("captures_iter");

    group.bench_with_input(
        BenchmarkId::new("medium", "200B"),
        MEDIUM_TEXT,
        |b, text| b.iter(|| engine.captures_iter(&regex, black_box(text))),
    );

    group.finish();
}

fn bench_replace_all(c: &mut Criterion) {
    let engine = RegexEngine::new();
    let regex = engine.compile(SIMPLE_PATTERN).unwrap();

    let mut group = c.benchmark_group("replace_all");

    group.bench_with_input(
        BenchmarkId::new("medium", "200B"),
        MEDIUM_TEXT,
        |b, text| b.iter(|| engine.replace_all(&regex, black_box(text), "XXX")),
    );

    group.finish();
}

// =============================================================================
// Plugin Simulation Benchmarks
// =============================================================================

fn bench_shell_plugin_simulation(c: &mut Criterion) {
    let engine = RegexEngine::new();

    // Simulate Shell plugin pattern compilation (should use cache)
    let function_re = engine.compile(SHELL_FUNCTION_PATTERN).unwrap();
    let export_re = engine
        .compile(r"(?m)^[ \t]*export\s+([A-Z_][A-Z0-9_]*)(?:=(.*))?$")
        .unwrap();
    let source_re = engine
        .compile(r#"(?m)^[ \t]*(?:source|\.)\s+["']?([^"'\s]+)["']?"#)
        .unwrap();

    let shell_script = r#"
#!/bin/bash
set -euo pipefail

export PATH="/usr/local/bin:$PATH"
export HOME

source /etc/profile
. ~/.bashrc

function deploy() {
    echo "Deploying..."
}

test_something() {
    echo "Testing..."
}

cleanup() {
    docker system prune -f
}
"#;

    c.bench_function("shell_plugin_extract_symbols", |b| {
        b.iter(|| {
            let _functions: Vec<_> = function_re.captures_iter(black_box(shell_script)).collect();
            let _exports: Vec<_> = export_re.captures_iter(black_box(shell_script)).collect();
            let _sources: Vec<_> = source_re.captures_iter(black_box(shell_script)).collect();
        })
    });
}

fn bench_abl_plugin_simulation(c: &mut Criterion) {
    let engine = RegexEngine::new();

    // Simulate ABL plugin pattern compilation
    let procedure_re = engine
        .compile(r"(?mi)^\s*PROCEDURE\s+([a-zA-Z_][a-zA-Z0-9_-]*)\s*(?:(EXTERNAL|PERSISTENT))?\s*:")
        .unwrap();
    let function_re = engine
        .compile(r"(?mi)^\s*FUNCTION\s+([a-zA-Z_][a-zA-Z0-9_-]*)\s+RETURNS\s+(\w+(?:\s+EXTENT)?)")
        .unwrap();
    let for_each_re = engine
        .compile(r"(?mi)\bFOR\s+(?:FIRST|LAST|EACH)\s+([a-zA-Z_][a-zA-Z0-9_-]*)")
        .unwrap();

    let abl_code = r#"
/* Order processing module */
DEFINE TEMP-TABLE tt-order-line NO-UNDO
    FIELD order-id AS INTEGER
    FIELD qty AS DECIMAL
    FIELD price AS DECIMAL.

PROCEDURE calculate-order-total:
    DEFINE INPUT PARAMETER ip-order-id AS INTEGER.
    DEFINE OUTPUT PARAMETER op-total AS DECIMAL.

    FOR EACH order-line WHERE order-line.order-id = ip-order-id:
        op-total = op-total + (order-line.qty * order-line.price).
    END.
END PROCEDURE.

FUNCTION format-currency RETURNS CHARACTER (ip-amount AS DECIMAL):
    RETURN STRING(ip-amount, ">>>,>>9.99").
END FUNCTION.
"#;

    c.bench_function("abl_plugin_extract_symbols", |b| {
        b.iter(|| {
            let _procs: Vec<_> = procedure_re.captures_iter(black_box(abl_code)).collect();
            let _funcs: Vec<_> = function_re.captures_iter(black_box(abl_code)).collect();
            let _for_each: Vec<_> = for_each_re.captures_iter(black_box(abl_code)).collect();
        })
    });
}

// =============================================================================
// PatternSet Benchmarks
// =============================================================================

fn bench_pattern_set(c: &mut Criterion) {
    let mut set = PatternSet::new();
    set.add("email", EMAIL_PATTERN).unwrap();
    set.add("digits", SIMPLE_PATTERN).unwrap();
    set.add("words", WORD_PATTERN).unwrap();
    set.add("url", r"https?://\S+").unwrap();

    c.bench_function("pattern_set_match_all", |b| {
        b.iter(|| set.match_all(black_box(MEDIUM_TEXT)))
    });

    c.bench_function("pattern_set_first_match", |b| {
        b.iter(|| set.first_match(black_box(MEDIUM_TEXT)))
    });
}

// =============================================================================
// Thread Safety Benchmarks
// =============================================================================

fn bench_concurrent_compile(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;

    let engine = Arc::new(RegexEngine::new());

    c.bench_function("concurrent_compile_4_threads", |b| {
        b.iter(|| {
            let mut handles = vec![];
            for i in 0..4 {
                let engine = engine.clone();
                let pattern = format!(r"test_pattern_{}", i % 10);
                handles.push(thread::spawn(move || engine.compile(&pattern).unwrap()));
            }
            for handle in handles {
                let _ = handle.join().unwrap();
            }
        })
    });
}

// =============================================================================
// Comparison: Raw Regex vs Engine
// =============================================================================

fn bench_raw_regex_comparison(c: &mut Criterion) {
    use regex::Regex;

    let mut group = c.benchmark_group("raw_vs_engine");

    // Raw regex (no caching)
    group.bench_function("raw_regex_compile", |b| {
        b.iter(|| Regex::new(black_box(EMAIL_PATTERN)).unwrap())
    });

    // Engine with cache hit
    let engine = RegexEngine::new();
    let _ = engine.compile(EMAIL_PATTERN);

    group.bench_function("engine_cache_hit", |b| {
        b.iter(|| engine.compile(black_box(EMAIL_PATTERN)).unwrap())
    });

    // Raw regex match
    let raw_re = Regex::new(EMAIL_PATTERN).unwrap();
    group.bench_function("raw_regex_is_match", |b| {
        b.iter(|| raw_re.is_match(black_box(SHORT_TEXT)))
    });

    // Engine match (through wrapper)
    let compiled = engine.compile(EMAIL_PATTERN).unwrap();
    group.bench_function("engine_is_match", |b| {
        b.iter(|| compiled.is_match(black_box(SHORT_TEXT)))
    });

    group.finish();
}

// =============================================================================
// Criterion Configuration
// =============================================================================

criterion_group!(
    cache_benchmarks,
    bench_compile_cache_hit,
    bench_compile_cache_miss,
    bench_global_engine_compile,
);

criterion_group!(
    matching_benchmarks,
    bench_is_match,
    bench_find_iter,
    bench_captures,
    bench_captures_iter,
    bench_replace_all,
);

criterion_group!(
    plugin_benchmarks,
    bench_shell_plugin_simulation,
    bench_abl_plugin_simulation,
);

criterion_group!(
    advanced_benchmarks,
    bench_pattern_set,
    bench_concurrent_compile,
    bench_raw_regex_comparison,
);

criterion_main!(
    cache_benchmarks,
    matching_benchmarks,
    plugin_benchmarks,
    advanced_benchmarks,
);
