//! Metrics collection infrastructure for LSP experiment
//!
//! Tracks latency, request counts, and accuracy metrics to validate
//! the Semantic Bridge concept.

use std::time::{Duration, Instant};

/// Collected metrics from an LSP session
#[derive(Debug, Clone, Default)]
pub struct LspMetrics {
    /// Time to spawn process and receive initialized notification
    pub startup_latency: Option<Duration>,
    /// Time from documentSymbol request to response
    pub first_symbol_latency: Option<Duration>,
    /// Total requests sent
    pub request_count: u32,
    /// Total responses received
    pub response_count: u32,
    /// Peak memory usage (if available)
    pub peak_memory_bytes: Option<u64>,
}

impl LspMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Format metrics as a human-readable report
    pub fn report(&self) -> String {
        let mut lines = vec!["=== LSP Metrics Report ===".to_string()];

        if let Some(startup) = self.startup_latency {
            lines.push(format!("Startup Latency: {:?}", startup));
        }

        if let Some(symbol) = self.first_symbol_latency {
            lines.push(format!("First Symbol Latency: {:?}", symbol));
        }

        lines.push(format!("Requests: {}", self.request_count));
        lines.push(format!("Responses: {}", self.response_count));

        if let Some(mem) = self.peak_memory_bytes {
            lines.push(format!("Peak Memory: {} MB", mem / 1_000_000));
        }

        lines.join("\n")
    }
}

/// Metrics collector with timing utilities
pub struct MetricsCollector {
    metrics: LspMetrics,
    current_timer: Option<Instant>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: LspMetrics::new(),
            current_timer: None,
        }
    }

    /// Start a timing measurement
    pub fn start_timer(&mut self) {
        self.current_timer = Some(Instant::now());
    }

    /// Record startup latency from current timer
    pub fn record_startup(&mut self) {
        if let Some(start) = self.current_timer.take() {
            self.metrics.startup_latency = Some(start.elapsed());
        }
    }

    /// Record first symbol latency from current timer
    pub fn record_first_symbol(&mut self) {
        if let Some(start) = self.current_timer.take() {
            self.metrics.first_symbol_latency = Some(start.elapsed());
        }
    }

    /// Increment request count
    pub fn record_request(&mut self) {
        self.metrics.request_count += 1;
    }

    /// Increment response count
    pub fn record_response(&mut self) {
        self.metrics.response_count += 1;
    }

    /// Set peak memory usage
    pub fn set_peak_memory(&mut self, bytes: u64) {
        self.metrics.peak_memory_bytes = Some(bytes);
    }

    /// Get the collected metrics
    pub fn finish(self) -> LspMetrics {
        self.metrics
    }

    /// Get a reference to current metrics
    pub fn metrics(&self) -> &LspMetrics {
        &self.metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol extraction metrics for accuracy comparison
#[derive(Debug, Clone, Default)]
pub struct SymbolMetrics {
    /// Symbols found by regex
    pub regex_count: usize,
    /// Symbols found by LSP
    pub lsp_count: usize,
    /// Symbols found by both (true positives)
    pub matched_count: usize,
    /// Regex extraction time
    pub regex_duration: Duration,
    /// LSP extraction time
    pub lsp_duration: Duration,
}

impl SymbolMetrics {
    /// Calculate precision (LSP symbols that match regex / total LSP symbols)
    pub fn precision(&self) -> f64 {
        if self.lsp_count == 0 {
            0.0
        } else {
            self.matched_count as f64 / self.lsp_count as f64
        }
    }

    /// Calculate recall (matched symbols / total regex symbols)
    pub fn recall(&self) -> f64 {
        if self.regex_count == 0 {
            0.0
        } else {
            self.matched_count as f64 / self.regex_count as f64
        }
    }

    /// Calculate F1 score
    pub fn f1_score(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }

    /// Speed ratio (how many times faster is regex)
    pub fn speed_ratio(&self) -> f64 {
        let regex_us = self.regex_duration.as_micros() as f64;
        let lsp_us = self.lsp_duration.as_micros() as f64;
        if regex_us == 0.0 {
            0.0
        } else {
            lsp_us / regex_us
        }
    }

    /// Format as report
    pub fn report(&self) -> String {
        format!(
            "=== Symbol Extraction Comparison ===\n\
             Regex symbols: {}\n\
             LSP symbols: {}\n\
             Matched: {}\n\
             Precision: {:.1}%\n\
             Recall: {:.1}%\n\
             F1 Score: {:.3}\n\
             Regex time: {:?}\n\
             LSP time: {:?}\n\
             Speed ratio: LSP is {:.0}x slower",
            self.regex_count,
            self.lsp_count,
            self.matched_count,
            self.precision() * 100.0,
            self.recall() * 100.0,
            self.f1_score(),
            self.regex_duration,
            self.lsp_duration,
            self.speed_ratio()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();

        collector.start_timer();
        std::thread::sleep(Duration::from_millis(10));
        collector.record_startup();

        collector.record_request();
        collector.record_request();
        collector.record_response();

        let metrics = collector.finish();

        assert!(metrics.startup_latency.is_some());
        assert!(metrics.startup_latency.unwrap() >= Duration::from_millis(10));
        assert_eq!(metrics.request_count, 2);
        assert_eq!(metrics.response_count, 1);
    }

    #[test]
    fn test_symbol_metrics_precision_recall() {
        let metrics = SymbolMetrics {
            regex_count: 10,
            lsp_count: 8,
            matched_count: 7,
            regex_duration: Duration::from_micros(100),
            lsp_duration: Duration::from_millis(500),
        };

        // Precision = 7/8 = 0.875
        assert!((metrics.precision() - 0.875).abs() < 0.001);
        // Recall = 7/10 = 0.7
        assert!((metrics.recall() - 0.7).abs() < 0.001);
        // Speed ratio = 500000/100 = 5000x
        assert!((metrics.speed_ratio() - 5000.0).abs() < 1.0);
    }
}
