//! Benchmark: Regex vs LSP Symbol Extraction
//!
//! Measures the performance gap between regex-based parsing
//! and LSP-based symbol extraction.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lsp_poc::comparison::RegexExtractor;

/// Sample Rust code for benchmarking (realistic complexity)
const SAMPLE_CODE_SMALL: &str = r#"
pub fn hello() -> &'static str {
    "Hello, world!"
}

pub struct Config {
    name: String,
}
"#;

const SAMPLE_CODE_MEDIUM: &str = r#"
//! A medium-sized module for benchmarking

use std::collections::HashMap;
use std::sync::Arc;

pub const VERSION: &str = "1.0.0";
pub const MAX_ITEMS: usize = 1000;

/// Configuration for the service
#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            port: 8080,
            timeout_ms: 5000,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new("default")
    }
}

#[derive(Debug)]
pub enum Status {
    Active,
    Inactive,
    Pending { reason: String },
}

pub trait Handler: Send + Sync {
    fn handle(&self, request: &str) -> Result<String, Error>;
    fn name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

pub struct Service<H: Handler> {
    config: Config,
    handler: Arc<H>,
    cache: HashMap<String, String>,
}

impl<H: Handler> Service<H> {
    pub fn new(config: Config, handler: H) -> Self {
        Self {
            config,
            handler: Arc::new(handler),
            cache: HashMap::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        println!("Starting {} on port {}", self.config.name, self.config.port);
        Ok(())
    }

    pub fn process(&mut self, key: &str, data: &str) -> Result<String, Error> {
        if let Some(cached) = self.cache.get(key) {
            return Ok(cached.clone());
        }

        let result = self.handler.handle(data)?;
        self.cache.insert(key.to_string(), result.clone());
        Ok(result)
    }
}

pub mod utils {
    pub fn sanitize(input: &str) -> String {
        input.trim().to_lowercase()
    }

    pub fn validate(input: &str) -> bool {
        !input.is_empty() && input.len() < 1000
    }
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
    }
}
"#;

/// Large code sample (multiple modules worth)
fn generate_large_code(multiplier: usize) -> String {
    let mut code = String::with_capacity(SAMPLE_CODE_MEDIUM.len() * multiplier);
    for i in 0..multiplier {
        code.push_str(&format!("\nmod module_{} {{\n", i));
        code.push_str(SAMPLE_CODE_MEDIUM);
        code.push_str("\n}\n");
    }
    code
}

fn bench_regex_extraction(c: &mut Criterion) {
    let extractor = RegexExtractor::new();

    let mut group = c.benchmark_group("regex_extraction");

    // Small code
    group.bench_function("small_50_chars", |b| {
        b.iter(|| extractor.extract(black_box(SAMPLE_CODE_SMALL)))
    });

    // Medium code
    group.bench_function("medium_2k_chars", |b| {
        b.iter(|| extractor.extract(black_box(SAMPLE_CODE_MEDIUM)))
    });

    // Large code
    let large_code = generate_large_code(5);
    group.bench_function("large_10k_chars", |b| {
        b.iter(|| extractor.extract(black_box(&large_code)))
    });

    // Very large code
    let very_large_code = generate_large_code(20);
    group.bench_function("very_large_40k_chars", |b| {
        b.iter(|| extractor.extract(black_box(&very_large_code)))
    });

    group.finish();
}

fn bench_regex_scaling(c: &mut Criterion) {
    let extractor = RegexExtractor::new();

    let mut group = c.benchmark_group("regex_scaling");

    for size in [1, 2, 5, 10, 20].iter() {
        let code = generate_large_code(*size);
        let chars = code.len();

        group.bench_with_input(
            BenchmarkId::new("modules", size),
            &code,
            |b, code| {
                b.iter(|| extractor.extract(black_box(code)))
            },
        );

        // Report chars processed
        eprintln!("Size {} modules = {} chars", size, chars);
    }

    group.finish();
}

fn bench_symbol_count(c: &mut Criterion) {
    let extractor = RegexExtractor::new();

    // Count symbols in medium code
    let symbols = extractor.extract(SAMPLE_CODE_MEDIUM);
    eprintln!("\nMedium code symbol count: {}", symbols.len());
    for s in &symbols {
        eprintln!("  - {} {} (line {})", s.kind, s.name, s.line);
    }

    c.bench_function("symbol_extraction_and_count", |b| {
        b.iter(|| {
            let symbols = extractor.extract(black_box(SAMPLE_CODE_MEDIUM));
            black_box(symbols.len())
        })
    });
}

criterion_group!(
    benches,
    bench_regex_extraction,
    bench_regex_scaling,
    bench_symbol_count
);
criterion_main!(benches);
