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
        format!(
            "{}: {:.2} ({})",
            self.name(),
            result.value,
            result.explanation
        )
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
        MetricResult::confident(count as f64, format!("{} declarations found", count))
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
            format!(
                "Healthy: {:.1} avg params across {} functions",
                avg,
                functions.len()
            )
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
            .filter(|d| matches!(d.visibility, voyager_ast::ir::Visibility::Public))
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
    use voyager_ast::ir::{
        Comment, CommentKind, Declaration, DeclarationKind, LanguageId, Parameter, Span, Visibility,
    };

    fn make_test_file() -> File {
        File {
            path: "test.rs".to_string(),
            language: LanguageId::Rust,
            declarations: vec![
                Declaration::new(
                    "foo".to_string(),
                    DeclarationKind::Function,
                    Span::default(),
                ),
                Declaration::new(
                    "bar".to_string(),
                    DeclarationKind::Function,
                    Span::default(),
                ),
            ],
            imports: vec![],
            comments: vec![],
            unknown_regions: vec![],
            span: Span::default(),
            metadata: Default::default(),
        }
    }

    fn make_file_with_params(param_counts: &[usize]) -> File {
        let declarations = param_counts
            .iter()
            .enumerate()
            .map(|(i, &count)| {
                let mut decl = Declaration::new(
                    format!("func_{}", i),
                    DeclarationKind::Function,
                    Span::default(),
                );
                for j in 0..count {
                    decl.parameters.push(Parameter {
                        name: format!("param_{}", j),
                        type_annotation: None,
                        default_value: None,
                        span: Span::default(),
                    });
                }
                decl
            })
            .collect();

        File {
            path: "test.rs".to_string(),
            language: LanguageId::Rust,
            declarations,
            imports: vec![],
            comments: vec![],
            unknown_regions: vec![],
            span: Span::default(),
            metadata: Default::default(),
        }
    }

    fn make_file_with_visibility(public_count: usize, documented_count: usize) -> File {
        let mut declarations = Vec::new();

        for i in 0..public_count {
            let mut decl = Declaration::new(
                format!("public_{}", i),
                DeclarationKind::Function,
                Span::default(),
            );
            decl.visibility = Visibility::Public;
            if i < documented_count {
                decl.doc_comment = Some(Comment {
                    text: "Documentation".to_string(),
                    kind: CommentKind::Doc,
                    span: Span::default(),
                    attached_to: None,
                });
            }
            declarations.push(decl);
        }

        File {
            path: "test.rs".to_string(),
            language: LanguageId::Rust,
            declarations,
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
    fn test_declaration_count_metric_empty() {
        let file = File {
            path: "empty.rs".to_string(),
            language: LanguageId::Rust,
            declarations: vec![],
            imports: vec![],
            comments: vec![],
            unknown_regions: vec![],
            span: Span::default(),
            metadata: Default::default(),
        };
        let metric = DeclarationCountMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 0.0);
    }

    #[test]
    fn test_declaration_count_name_description() {
        let metric = DeclarationCountMetric;
        assert_eq!(metric.name(), "declaration_count");
        assert!(!metric.description().is_empty());
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
    fn test_metric_registry_default() {
        let registry = MetricRegistry::default();
        assert_eq!(registry.collectors().len(), 0);
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

    #[test]
    fn test_metric_result_confident() {
        let result = MetricResult::confident(42.0, "high confidence");
        assert_eq!(result.value, 42.0);
        assert_eq!(result.confidence, 1.0);
        assert_eq!(result.explanation, "high confidence");
    }

    #[test]
    fn test_metric_result_uncertain() {
        let result = MetricResult::uncertain(10.0, "low confidence");
        assert_eq!(result.value, 10.0);
        assert_eq!(result.confidence, 0.5);
        assert_eq!(result.explanation, "low confidence");
    }

    #[test]
    fn test_format_human() {
        let metric = DeclarationCountMetric;
        let result = MetricResult::confident(5.0, "5 declarations found");
        let formatted = metric.format_human(&result);
        assert!(formatted.contains("declaration_count"));
        assert!(formatted.contains("5.00"));
    }

    #[test]
    fn test_format_machine() {
        let metric = DeclarationCountMetric;
        let result = MetricResult::confident(5.0, "5 declarations found");
        let json = metric.format_machine(&result);
        assert_eq!(json["metric"], "declaration_count");
        assert_eq!(json["value"], 5.0);
        assert_eq!(json["confidence"], 1.0);
    }

    #[test]
    fn test_parameter_complexity_no_functions() {
        let file = File {
            path: "test.rs".to_string(),
            language: LanguageId::Rust,
            declarations: vec![Declaration::new(
                "MyStruct".to_string(),
                DeclarationKind::Struct,
                Span::default(),
            )],
            imports: vec![],
            comments: vec![],
            unknown_regions: vec![],
            span: Span::default(),
            metadata: Default::default(),
        };
        let metric = ParameterComplexityMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 0.0);
        assert_eq!(result.confidence, 0.5); // uncertain
    }

    #[test]
    fn test_parameter_complexity_healthy() {
        let file = make_file_with_params(&[1, 2, 3]); // avg = 2.0
        let metric = ParameterComplexityMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 2.0);
        assert!(result.explanation.contains("Healthy"));
    }

    #[test]
    fn test_parameter_complexity_moderate() {
        let file = make_file_with_params(&[4, 4, 4]); // avg = 4.0
        let metric = ParameterComplexityMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 4.0);
        assert!(result.explanation.contains("Moderate"));
    }

    #[test]
    fn test_parameter_complexity_high() {
        let file = make_file_with_params(&[6, 7, 8]); // avg = 7.0
        let metric = ParameterComplexityMetric;
        let result = metric.analyze(&file);
        assert!(result.value > 5.0);
        assert!(result.explanation.contains("High"));
    }

    #[test]
    fn test_parameter_complexity_name_description() {
        let metric = ParameterComplexityMetric;
        assert_eq!(metric.name(), "avg_parameters");
        assert!(!metric.description().is_empty());
    }

    #[test]
    fn test_doc_coverage_no_public() {
        let file = make_test_file(); // Default visibility is Private
        let metric = DocCoverageMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 100.0); // "No public declarations"
        assert_eq!(result.confidence, 0.5); // uncertain
    }

    #[test]
    fn test_doc_coverage_all_documented() {
        let file = make_file_with_visibility(3, 3); // 3 public, 3 documented
        let metric = DocCoverageMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 100.0);
        assert!(result.explanation.contains("3/3"));
    }

    #[test]
    fn test_doc_coverage_partial() {
        let file = make_file_with_visibility(4, 2); // 4 public, 2 documented
        let metric = DocCoverageMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 50.0);
        assert!(result.explanation.contains("2/4"));
    }

    #[test]
    fn test_doc_coverage_none_documented() {
        let file = make_file_with_visibility(5, 0); // 5 public, 0 documented
        let metric = DocCoverageMetric;
        let result = metric.analyze(&file);
        assert_eq!(result.value, 0.0);
        assert!(result.explanation.contains("0/5"));
    }

    #[test]
    fn test_doc_coverage_name_description() {
        let metric = DocCoverageMetric;
        assert_eq!(metric.name(), "doc_coverage");
        assert!(!metric.description().is_empty());
    }

    #[test]
    fn test_registry_analyze_all_multiple() {
        let mut registry = MetricRegistry::new();
        registry.register(Box::new(DeclarationCountMetric));
        registry.register(Box::new(ParameterComplexityMetric));
        registry.register(Box::new(DocCoverageMetric));

        let file = make_test_file();
        let results = registry.analyze_all(&file);

        assert_eq!(results.len(), 3);
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_metric_result_clone() {
        let result = MetricResult::confident(42.0, "test");
        let cloned = result.clone();

        assert_eq!(cloned.value, 42.0);
        assert_eq!(cloned.confidence, 1.0);
        assert_eq!(cloned.explanation, "test");
    }

    #[test]
    fn test_metric_result_debug() {
        let result = MetricResult::new(10.0, 0.8, "debug test");
        let debug_str = format!("{:?}", result);

        assert!(debug_str.contains("MetricResult"));
        assert!(debug_str.contains("10.0") || debug_str.contains("10"));
    }

    #[test]
    fn test_registry_empty_analyze_all() {
        let registry = MetricRegistry::new();
        let file = make_test_file();
        let results = registry.analyze_all(&file);

        assert!(results.is_empty());
    }

    #[test]
    fn test_registry_find_returns_trait_object() {
        let mut registry = MetricRegistry::new();
        registry.register(Box::new(DeclarationCountMetric));

        if let Some(collector) = registry.find("declaration_count") {
            // Can call trait methods on the returned reference
            assert_eq!(collector.name(), "declaration_count");
            assert!(!collector.description().is_empty());
        } else {
            panic!("Expected to find collector");
        }
    }

    #[test]
    fn test_parameter_complexity_with_methods() {
        // Test that Method kind is also counted
        let mut declarations = vec![];
        let mut method = Declaration::new(
            "method".to_string(),
            DeclarationKind::Method,
            Span::default(),
        );
        method.parameters.push(Parameter {
            name: "self".to_string(),
            type_annotation: None,
            default_value: None,
            span: Span::default(),
        });
        method.parameters.push(Parameter {
            name: "arg".to_string(),
            type_annotation: None,
            default_value: None,
            span: Span::default(),
        });
        declarations.push(method);

        let file = File {
            path: "test.rs".to_string(),
            language: LanguageId::Rust,
            declarations,
            imports: vec![],
            comments: vec![],
            unknown_regions: vec![],
            span: Span::default(),
            metadata: Default::default(),
        };

        let metric = ParameterComplexityMetric;
        let result = metric.analyze(&file);

        // Method has 2 params
        assert_eq!(result.value, 2.0);
    }

    #[test]
    fn test_format_human_for_all_metrics() {
        let file = make_file_with_params(&[2, 3]);

        let metric1 = DeclarationCountMetric;
        let result1 = metric1.analyze(&file);
        let human1 = metric1.format_human(&result1);
        assert!(human1.contains("declaration_count"));

        let metric2 = ParameterComplexityMetric;
        let result2 = metric2.analyze(&file);
        let human2 = metric2.format_human(&result2);
        assert!(human2.contains("avg_parameters"));

        let metric3 = DocCoverageMetric;
        let result3 = metric3.analyze(&file);
        let human3 = metric3.format_human(&result3);
        assert!(human3.contains("doc_coverage"));
    }

    #[test]
    fn test_format_machine_for_all_metrics() {
        let file = make_file_with_params(&[2, 3]);

        let metric1 = ParameterComplexityMetric;
        let result1 = metric1.analyze(&file);
        let json1 = metric1.format_machine(&result1);
        assert_eq!(json1["metric"], "avg_parameters");
        assert!(json1["value"].is_number());
        assert!(json1["confidence"].is_number());
        assert!(json1["explanation"].is_string());
    }

    #[test]
    fn test_metric_result_boundary_confidence() {
        // Test exact boundary values
        let at_zero = MetricResult::new(1.0, 0.0, "zero");
        assert_eq!(at_zero.confidence, 0.0);

        let at_one = MetricResult::new(1.0, 1.0, "one");
        assert_eq!(at_one.confidence, 1.0);

        let mid = MetricResult::new(1.0, 0.5, "mid");
        assert_eq!(mid.confidence, 0.5);
    }

    #[test]
    fn test_collectors_accessor_iteration() {
        let mut registry = MetricRegistry::new();
        registry.register(Box::new(DeclarationCountMetric));
        registry.register(Box::new(ParameterComplexityMetric));

        let names: Vec<&str> = registry.collectors().iter().map(|c| c.name()).collect();

        assert!(names.contains(&"declaration_count"));
        assert!(names.contains(&"avg_parameters"));
    }

    #[test]
    fn test_registry_register_multiple_same_type() {
        let mut registry = MetricRegistry::new();
        // Can register same metric type multiple times (not recommended but allowed)
        registry.register(Box::new(DeclarationCountMetric));
        registry.register(Box::new(DeclarationCountMetric));

        assert_eq!(registry.collectors().len(), 2);
    }
}
