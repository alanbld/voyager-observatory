//! Core metrics analysis traits and interfaces
//!
//! This module provides the foundation for AST-based code metrics collection.
//! Metrics can analyze parsed code structure to extract quantitative measures
//! like complexity, coupling, cohesion, and more.

use voyager_ast::ir::File;

/// Result of metric analysis
#[derive(Debug, Clone)]
pub struct MetricResult {
    /// The computed metric value
    pub value: f64,

    /// Confidence in the measurement (0.0 - 1.0)
    pub confidence: f64,

    /// Human-readable explanation (Observatory language)
    pub explanation: String,
}

impl MetricResult {
    /// Create a new metric result
    pub fn new(value: f64, confidence: f64, explanation: impl Into<String>) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            explanation: explanation.into(),
        }
    }

    /// Create a high-confidence result
    pub fn confident(value: f64, explanation: impl Into<String>) -> Self {
        Self::new(value, 1.0, explanation)
    }

    /// Create a result with unknown/low confidence
    pub fn uncertain(value: f64, explanation: impl Into<String>) -> Self {
        Self::new(value, 0.5, explanation)
    }
}

/// Trait for collecting metrics from AST
///
/// Implement this trait to create custom code metrics that analyze
/// the parsed AST structure.
pub trait MetricCollector: Send + Sync {
    /// Unique name for this metric
    fn name(&self) -> &str;

    /// Short description of what this metric measures
    fn description(&self) -> &str;

    /// Analyze AST and produce metric
    fn analyze(&self, file: &File) -> MetricResult;

    /// Format for human consumption (Observatory language)
    fn format_human(&self, result: &MetricResult) -> String {
        format!("{}: {:.2} ({})", self.name(), result.value, result.explanation)
    }

    /// Optional: Format for machine consumption
    fn format_machine(&self, result: &MetricResult) -> serde_json::Value {
        serde_json::json!({
            "metric": self.name(),
            "value": result.value,
            "confidence": result.confidence,
            "explanation": result.explanation
        })
    }
}

/// Registry for metric collectors
///
/// Manages a collection of metrics that can be applied to parsed files.
pub struct MetricRegistry {
    collectors: Vec<Box<dyn MetricCollector>>,
}

impl MetricRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            collectors: Vec::new(),
        }
    }

    /// Register a metric collector
    pub fn register(&mut self, collector: Box<dyn MetricCollector>) {
        self.collectors.push(collector);
    }

    /// Get all registered collectors
    pub fn collectors(&self) -> &[Box<dyn MetricCollector>] {
        &self.collectors
    }

    /// Analyze a file with all registered metrics
    pub fn analyze_all(&self, file: &File) -> Vec<(&str, MetricResult)> {
        self.collectors
            .iter()
            .map(|c| (c.name(), c.analyze(file)))
            .collect()
    }

    /// Find a collector by name
    pub fn find(&self, name: &str) -> Option<&dyn MetricCollector> {
        self.collectors
            .iter()
            .find(|c| c.name() == name)
            .map(|c| c.as_ref())
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in Metrics (Phase 3 placeholders)
// ============================================================================

/// Counts the total number of declarations in a file
pub struct DeclarationCountMetric;

impl MetricCollector for DeclarationCountMetric {
    fn name(&self) -> &str {
        "declaration_count"
    }

    fn description(&self) -> &str {
        "Total number of declarations (functions, classes, etc.)"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let count = file.declarations.len();
        MetricResult::confident(
            count as f64,
            format!("{} declarations found", count),
        )
    }
}

/// Measures the average number of parameters per function
pub struct ParameterComplexityMetric;

impl MetricCollector for ParameterComplexityMetric {
    fn name(&self) -> &str {
        "avg_parameters"
    }

    fn description(&self) -> &str {
        "Average number of parameters per function/method"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let functions: Vec<_> = file
            .declarations
            .iter()
            .filter(|d| {
                matches!(
                    d.kind,
                    voyager_ast::ir::DeclarationKind::Function
                        | voyager_ast::ir::DeclarationKind::Method
                )
            })
            .collect();

        if functions.is_empty() {
            return MetricResult::uncertain(0.0, "No functions found");
        }

        let total_params: usize = functions.iter().map(|f| f.parameters.len()).sum();
        let avg = total_params as f64 / functions.len() as f64;

        let explanation = if avg <= 3.0 {
            format!("Healthy: {:.1} avg params across {} functions", avg, functions.len())
        } else if avg <= 5.0 {
            format!("Moderate: {:.1} avg params - consider simplifying", avg)
        } else {
            format!("High: {:.1} avg params - functions may be too complex", avg)
        };

        MetricResult::confident(avg, explanation)
    }
}

/// Measures documentation coverage
pub struct DocCoverageMetric;

impl MetricCollector for DocCoverageMetric {
    fn name(&self) -> &str {
        "doc_coverage"
    }

    fn description(&self) -> &str {
        "Percentage of public declarations with documentation"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let public_decls: Vec<_> = file
            .declarations
            .iter()
            .filter(|d| {
                matches!(
                    d.visibility,
                    voyager_ast::ir::Visibility::Public
                )
            })
            .collect();

        if public_decls.is_empty() {
            return MetricResult::uncertain(100.0, "No public declarations");
        }

        let documented = public_decls
            .iter()
            .filter(|d| d.doc_comment.is_some())
            .count();

        let coverage = (documented as f64 / public_decls.len() as f64) * 100.0;

        let explanation = format!(
            "{}/{} public items documented ({:.0}%)",
            documented,
            public_decls.len(),
            coverage
        );

        MetricResult::confident(coverage, explanation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voyager_ast::ir::{Declaration, DeclarationKind, LanguageId, Span};

    fn make_test_file() -> File {
        File {
            path: "test.rs".to_string(),
            language: LanguageId::Rust,
            declarations: vec![
                Declaration::new("foo".to_string(), DeclarationKind::Function, Span::default()),
                Declaration::new("bar".to_string(), DeclarationKind::Function, Span::default()),
            ],
            imports: vec![],
            comments: vec![],
            unknown_regions: vec![],
            span: Span::default(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_declaration_count_metric() {
        let file = make_test_file();
        let metric = DeclarationCountMetric;
        let result = metric.analyze(&file);

        assert_eq!(result.value, 2.0);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_metric_registry() {
        let mut registry = MetricRegistry::new();
        registry.register(Box::new(DeclarationCountMetric));
        registry.register(Box::new(ParameterComplexityMetric));

        assert_eq!(registry.collectors().len(), 2);
        assert!(registry.find("declaration_count").is_some());
        assert!(registry.find("nonexistent").is_none());
    }

    #[test]
    fn test_analyze_all() {
        let mut registry = MetricRegistry::new();
        registry.register(Box::new(DeclarationCountMetric));

        let file = make_test_file();
        let results = registry.analyze_all(&file);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "declaration_count");
        assert_eq!(results[0].1.value, 2.0);
    }

    #[test]
    fn test_metric_result_clamping() {
        let result = MetricResult::new(50.0, 1.5, "test");
        assert_eq!(result.confidence, 1.0);

        let result = MetricResult::new(50.0, -0.5, "test");
        assert_eq!(result.confidence, 0.0);
    }
}
