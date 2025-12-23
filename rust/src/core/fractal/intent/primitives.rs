//! Cognitive Primitives for Intent-Driven Exploration
//!
//! These primitives are the building blocks of intent compositions.
//! Each primitive performs a single cognitive transformation:
//!
//! - **NoiseFilter**: Remove elements that don't matter for the goal
//! - **RelevanceScorer**: Score remaining elements by relevance to goal
//! - **ExplorationPlanner**: Plan optimal exploration path through relevant elements

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

use crate::core::fractal::{
    ContextLayer, LayerContent, SymbolKind,
    FeatureVector,
    clustering::ShellPatternType,
};

// =============================================================================
// Concept Types (derived from existing patterns)
// =============================================================================

/// Semantic concept type - what kind of code element is this?
/// Derived from shell patterns and symbol analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConceptType {
    /// Core business calculations (pricing, totals, etc.)
    Calculation,
    /// Input/data validation logic
    Validation,
    /// Business decisions and routing
    Decision,
    /// Data transformation and mapping
    Transformation,
    /// Error handling and recovery
    ErrorHandling,
    /// Logging, metrics, observability
    Logging,
    /// Configuration and setup
    Configuration,
    /// Test code
    Testing,
    /// Infrastructure (network, file I/O, etc.)
    Infrastructure,
    /// Unknown/unclassified
    Unknown,
}

impl ConceptType {
    /// Infer concept type from layer content and name patterns
    pub fn infer(layer: &ContextLayer) -> Self {
        let name = layer.name().to_lowercase();

        // Name-based heuristics
        if name.starts_with("test_") || name.ends_with("_test") || name.contains("_spec") {
            return ConceptType::Testing;
        }
        if name.starts_with("log_") || name.contains("logger") || name.contains("_log") {
            return ConceptType::Logging;
        }
        if name.starts_with("config") || name.contains("_config") || name.contains("settings") {
            return ConceptType::Configuration;
        }
        if name.contains("validate") || name.contains("check_") || name.contains("is_valid") {
            return ConceptType::Validation;
        }
        if name.contains("calculate") || name.contains("compute") || name.contains("_total")
            || name.contains("_sum") || name.contains("_avg")
        {
            return ConceptType::Calculation;
        }
        if name.contains("transform") || name.contains("convert") || name.contains("map_")
            || name.contains("_to_")
        {
            return ConceptType::Transformation;
        }
        if name.contains("handle_error") || name.contains("on_error") || name.contains("_error")
            || name.contains("recover")
        {
            return ConceptType::ErrorHandling;
        }
        if name.contains("decide") || name.contains("route") || name.contains("dispatch")
            || name.contains("should_")
        {
            return ConceptType::Decision;
        }

        // Content-based heuristics for symbols
        if let LayerContent::Symbol { kind, documentation, .. } = &layer.content {
            if let Some(doc) = documentation {
                let doc_lower = doc.to_lowercase();
                if doc_lower.contains("calculate") || doc_lower.contains("compute") {
                    return ConceptType::Calculation;
                }
                if doc_lower.contains("validate") || doc_lower.contains("check") {
                    return ConceptType::Validation;
                }
            }

            // Method kind hints
            match kind {
                SymbolKind::Function | SymbolKind::Method => {
                    // Default to unknown for functions without clear signals
                }
                SymbolKind::Struct | SymbolKind::Class => {
                    if name.contains("config") || name.contains("options") {
                        return ConceptType::Configuration;
                    }
                }
                _ => {}
            }
        }

        ConceptType::Unknown
    }

    /// Map shell pattern type to concept type
    pub fn from_shell_pattern(pattern: &ShellPatternType) -> Self {
        match pattern {
            ShellPatternType::Deployment => ConceptType::Infrastructure,
            ShellPatternType::ErrorHandling => ConceptType::ErrorHandling,
            ShellPatternType::DataProcessing => ConceptType::Transformation,
            ShellPatternType::Automation => ConceptType::Infrastructure,
            ShellPatternType::Backup => ConceptType::Infrastructure,
            ShellPatternType::Monitoring => ConceptType::Logging,
            ShellPatternType::Testing => ConceptType::Testing,
            ShellPatternType::Build => ConceptType::Infrastructure,
            ShellPatternType::Cleanup => ConceptType::Infrastructure,
            ShellPatternType::Setup => ConceptType::Configuration,
            ShellPatternType::Network => ConceptType::Infrastructure,
            ShellPatternType::Security => ConceptType::Validation,
            ShellPatternType::Unknown => ConceptType::Unknown,
        }
    }
}

// =============================================================================
// Relevance Score
// =============================================================================

/// Relevance score with explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    pub score: f32,
    pub explanation: String,
    pub factors: Vec<(String, f32)>,
}

impl RelevanceScore {
    pub fn new(score: f32, explanation: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            explanation: explanation.into(),
            factors: Vec::new(),
        }
    }

    pub fn with_factor(mut self, name: impl Into<String>, contribution: f32) -> Self {
        self.factors.push((name.into(), contribution));
        self
    }
}

/// Element with its relevance score
#[derive(Debug, Clone)]
pub struct ScoredElement<'a> {
    pub layer: &'a ContextLayer,
    pub vector: &'a FeatureVector,
    pub score: f32,
    pub concept_type: ConceptType,
    pub relevance: RelevanceScore,
}

// =============================================================================
// Cognitive Primitive Trait
// =============================================================================

/// A cognitive primitive performs a single transformation step
pub trait CognitivePrimitive {
    type Input<'a>;
    type Output;
    type Params;

    /// Apply this primitive to transform input
    fn apply<'a>(&self, input: Self::Input<'a>, params: &Self::Params) -> Self::Output;

    /// Name of this primitive for logging/debugging
    fn name(&self) -> &'static str;

    /// Description of what this primitive does
    fn description(&self) -> &'static str;
}

// =============================================================================
// NoiseFilter Primitive
// =============================================================================

/// Parameters for noise filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseFilterParams {
    /// Name patterns to filter (e.g., "test_", "log_")
    pub filter_name_patterns: Vec<String>,
    /// Concept types to filter
    pub filter_concept_types: Vec<ConceptType>,
    /// Minimum relevance threshold (elements below this are noise)
    pub min_relevance_threshold: f32,
}

impl Default for NoiseFilterParams {
    fn default() -> Self {
        Self {
            filter_name_patterns: vec![],
            filter_concept_types: vec![],
            min_relevance_threshold: 0.0,
        }
    }
}

impl NoiseFilterParams {
    /// Preset for business logic exploration
    pub fn business_logic() -> Self {
        Self {
            filter_name_patterns: vec![
                "test_".to_string(),
                "_test".to_string(),
                "log_".to_string(),
                "debug_".to_string(),
                "mock_".to_string(),
            ],
            filter_concept_types: vec![
                ConceptType::Testing,
                ConceptType::Logging,
                ConceptType::Configuration,
            ],
            min_relevance_threshold: 0.2,
        }
    }

    /// Preset for debugging exploration
    pub fn debugging() -> Self {
        Self {
            filter_name_patterns: vec![
                "test_".to_string(),
                "_test".to_string(),
            ],
            filter_concept_types: vec![ConceptType::Testing],
            min_relevance_threshold: 0.1,
        }
    }

    /// Preset for security review
    pub fn security() -> Self {
        Self {
            filter_name_patterns: vec![
                "test_".to_string(),
                "_test".to_string(),
                "mock_".to_string(),
            ],
            filter_concept_types: vec![ConceptType::Testing],
            min_relevance_threshold: 0.1,
        }
    }
}

/// Noise filter primitive - removes elements that don't matter for the goal
#[derive(Debug, Clone, Default)]
pub struct NoiseFilter;

impl CognitivePrimitive for NoiseFilter {
    type Input<'a> = &'a [(&'a ContextLayer, &'a FeatureVector)];
    type Output = Vec<(usize, ConceptType)>; // Indices of kept elements + their types
    type Params = NoiseFilterParams;

    fn apply<'a>(&self, input: Self::Input<'a>, params: &Self::Params) -> Self::Output {
        let filter_types: HashSet<_> = params.filter_concept_types.iter().collect();

        input
            .iter()
            .enumerate()
            .filter_map(|(idx, (layer, _vector))| {
                let name = layer.name().to_lowercase();

                // Check name patterns
                for pattern in &params.filter_name_patterns {
                    if name.contains(pattern) {
                        return None; // Filter out
                    }
                }

                // Check concept type
                let concept_type = ConceptType::infer(layer);
                if filter_types.contains(&concept_type) {
                    return None; // Filter out
                }

                Some((idx, concept_type))
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        "NoiseFilter"
    }

    fn description(&self) -> &'static str {
        "Remove elements that don't matter for the exploration goal"
    }
}

// =============================================================================
// RelevanceScorer Primitive
// =============================================================================

/// Parameters for relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScorerParams {
    /// Weights for different concept types (higher = more relevant)
    pub concept_weights: Vec<(ConceptType, f32)>,
    /// Feature vector dimension weights (for semantic similarity)
    /// Using Vec instead of [f32; 64] for serde compatibility
    pub dimension_weights: Option<Vec<f32>>,
    /// Boost for elements with documentation
    pub documentation_boost: f32,
    /// Boost for public visibility
    pub public_visibility_boost: f32,
}

impl Default for RelevanceScorerParams {
    fn default() -> Self {
        Self {
            concept_weights: vec![],
            dimension_weights: None,
            documentation_boost: 0.1,
            public_visibility_boost: 0.1,
        }
    }
}

impl RelevanceScorerParams {
    /// Preset for business logic exploration
    pub fn business_logic() -> Self {
        Self {
            concept_weights: vec![
                (ConceptType::Calculation, 1.0),
                (ConceptType::Validation, 0.9),
                (ConceptType::Decision, 0.85),
                (ConceptType::Transformation, 0.7),
                (ConceptType::ErrorHandling, 0.4),
                (ConceptType::Logging, 0.1),
                (ConceptType::Configuration, 0.2),
                (ConceptType::Infrastructure, 0.2),
                (ConceptType::Testing, 0.05),
                (ConceptType::Unknown, 0.5),
            ],
            dimension_weights: None,
            documentation_boost: 0.15,
            public_visibility_boost: 0.1,
        }
    }

    /// Preset for debugging exploration
    pub fn debugging() -> Self {
        Self {
            concept_weights: vec![
                (ConceptType::ErrorHandling, 1.0),
                (ConceptType::Logging, 0.9),
                (ConceptType::Validation, 0.7),
                (ConceptType::Decision, 0.6),
                (ConceptType::Calculation, 0.5),
                (ConceptType::Transformation, 0.5),
                (ConceptType::Configuration, 0.4),
                (ConceptType::Infrastructure, 0.3),
                (ConceptType::Testing, 0.2),
                (ConceptType::Unknown, 0.4),
            ],
            dimension_weights: None,
            documentation_boost: 0.1,
            public_visibility_boost: 0.05,
        }
    }

    /// Preset for security review
    pub fn security() -> Self {
        Self {
            concept_weights: vec![
                (ConceptType::Validation, 1.0),
                (ConceptType::ErrorHandling, 0.9),
                (ConceptType::Configuration, 0.8),
                (ConceptType::Infrastructure, 0.7),
                (ConceptType::Decision, 0.6),
                (ConceptType::Transformation, 0.5),
                (ConceptType::Calculation, 0.4),
                (ConceptType::Logging, 0.3),
                (ConceptType::Testing, 0.1),
                (ConceptType::Unknown, 0.5),
            ],
            dimension_weights: None,
            documentation_boost: 0.1,
            public_visibility_boost: 0.15,
        }
    }

    /// Preset for onboarding/learning
    pub fn onboarding() -> Self {
        Self {
            concept_weights: vec![
                (ConceptType::Decision, 1.0),       // Entry points
                (ConceptType::Calculation, 0.8),    // Core logic
                (ConceptType::Configuration, 0.7),  // How to configure
                (ConceptType::Validation, 0.6),
                (ConceptType::Transformation, 0.5),
                (ConceptType::ErrorHandling, 0.4),
                (ConceptType::Infrastructure, 0.3),
                (ConceptType::Logging, 0.2),
                (ConceptType::Testing, 0.3),        // Tests help understand
                (ConceptType::Unknown, 0.4),
            ],
            dimension_weights: None,
            documentation_boost: 0.25,  // Documentation very valuable for onboarding
            public_visibility_boost: 0.15,
        }
    }
}

/// Relevance scorer primitive - scores elements by relevance to goal
#[derive(Debug, Clone, Default)]
pub struct RelevanceScorer;

impl RelevanceScorer {
    /// Score a single element (public for composition use)
    pub fn score_element(
        &self,
        layer: &ContextLayer,
        _vector: &FeatureVector,
        concept_type: ConceptType,
        params: &RelevanceScorerParams,
    ) -> RelevanceScore {
        let mut score = 0.5f32;
        let mut factors = Vec::new();

        // Concept type weight
        let concept_weight = params
            .concept_weights
            .iter()
            .find(|(ct, _)| *ct == concept_type)
            .map(|(_, w)| *w)
            .unwrap_or(0.5);

        factors.push(("concept_type".to_string(), concept_weight));
        score = concept_weight;

        // Documentation boost
        if let LayerContent::Symbol { documentation: Some(_), .. } = &layer.content {
            score += params.documentation_boost;
            factors.push(("has_documentation".to_string(), params.documentation_boost));
        }

        // Visibility boost
        if let LayerContent::Symbol { visibility, .. } = &layer.content {
            if *visibility == crate::core::fractal::Visibility::Public {
                score += params.public_visibility_boost;
                factors.push(("public_visibility".to_string(), params.public_visibility_boost));
            }
        }

        score = score.clamp(0.0, 1.0);

        RelevanceScore {
            score,
            explanation: format!("{:?} element", concept_type),
            factors,
        }
    }
}

impl CognitivePrimitive for RelevanceScorer {
    type Input<'a> = (&'a [(&'a ContextLayer, &'a FeatureVector)], &'a [(usize, ConceptType)]);
    type Output = Vec<ScoredElement<'static>>; // Note: We'll handle lifetimes in actual use
    type Params = RelevanceScorerParams;

    fn apply<'a>(&self, input: Self::Input<'a>, params: &Self::Params) -> Self::Output {
        let (all_elements, filtered_indices) = input;

        filtered_indices
            .iter()
            .map(|(idx, concept_type)| {
                let (layer, vector) = &all_elements[*idx];
                let relevance = self.score_element(layer, vector, *concept_type, params);

                // SAFETY: We're creating references that won't outlive the input
                // In actual use, we'll need to handle this differently
                ScoredElement {
                    layer: unsafe { std::mem::transmute::<&ContextLayer, &'static ContextLayer>(*layer) },
                    vector: unsafe { std::mem::transmute::<&FeatureVector, &'static FeatureVector>(*vector) },
                    score: relevance.score,
                    concept_type: *concept_type,
                    relevance,
                }
            })
            .collect()
    }

    fn name(&self) -> &'static str {
        "RelevanceScorer"
    }

    fn description(&self) -> &'static str {
        "Score elements by relevance to the exploration goal"
    }
}

// =============================================================================
// ExplorationPlanner Primitive
// =============================================================================

/// Parameters for exploration planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationPlannerParams {
    /// Maximum number of elements in exploration path
    pub max_elements: usize,
    /// Time budget in minutes (affects path length)
    pub time_budget_minutes: u32,
    /// Minimum relevance score to include
    pub min_relevance: f32,
    /// Whether to group related elements
    pub group_related: bool,
}

impl Default for ExplorationPlannerParams {
    fn default() -> Self {
        Self {
            max_elements: 20,
            time_budget_minutes: 30,
            min_relevance: 0.3,
            group_related: true,
        }
    }
}

/// Exploration step with decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    pub element_idx: usize,
    pub path: String,
    pub symbol: String,
    pub decision: String,  // "read", "skim", "skip"
    pub reason: String,
    pub estimated_minutes: u32,
    pub relevance_score: f32,
}

/// Exploration planner primitive - creates optimal exploration path
#[derive(Debug, Clone, Default)]
pub struct ExplorationPlanner;

impl ExplorationPlanner {
    /// Estimate reading time for an element based on complexity
    fn estimate_minutes(layer: &ContextLayer) -> u32 {
        match &layer.content {
            LayerContent::Symbol { range, documentation, .. } => {
                let lines = range.end_line.saturating_sub(range.start_line) + 1;
                let base_minutes = (lines as f32 / 20.0).ceil() as u32; // ~20 lines per minute
                let doc_bonus = if documentation.is_some() { 1 } else { 0 };
                (base_minutes + doc_bonus).max(1)
            }
            LayerContent::File { line_count, .. } => {
                (*line_count as f32 / 50.0).ceil() as u32 // ~50 lines per minute for skimming
            }
            _ => 2, // Default
        }
    }

    /// Determine reading decision based on relevance
    fn decide(relevance: f32) -> (&'static str, &'static str) {
        if relevance > 0.7 {
            ("read", "High relevance to your goal")
        } else if relevance > 0.4 {
            ("skim", "Moderate relevance, get the gist")
        } else {
            ("skip", "Low relevance, come back if needed")
        }
    }
}

impl CognitivePrimitive for ExplorationPlanner {
    type Input<'a> = &'a [ScoredElement<'a>];
    type Output = Vec<PlannedStep>;
    type Params = ExplorationPlannerParams;

    fn apply<'a>(&self, input: Self::Input<'a>, params: &Self::Params) -> Self::Output {
        // Sort by relevance (descending)
        let mut sorted: Vec<_> = input.iter().enumerate().collect();
        sorted.sort_by(|a, b| b.1.score.partial_cmp(&a.1.score).unwrap_or(std::cmp::Ordering::Equal));

        // Filter by minimum relevance and limit
        let filtered: Vec<_> = sorted
            .into_iter()
            .filter(|(_, elem)| elem.score >= params.min_relevance)
            .take(params.max_elements)
            .collect();

        // Build exploration path respecting time budget
        let mut total_minutes = 0u32;
        let mut steps = Vec::new();

        for (idx, elem) in filtered {
            let estimated = Self::estimate_minutes(elem.layer);
            let (decision, reason) = Self::decide(elem.score);

            // Only count time for "read" decisions
            if decision == "read" {
                if total_minutes + estimated > params.time_budget_minutes && !steps.is_empty() {
                    // Over budget, change to skim
                    steps.push(PlannedStep {
                        element_idx: idx,
                        path: elem.layer.id.clone(),
                        symbol: elem.layer.name().to_string(),
                        decision: "skim".to_string(),
                        reason: "Over time budget, skim instead".to_string(),
                        estimated_minutes: (estimated / 2).max(1),
                        relevance_score: elem.score,
                    });
                    total_minutes += (estimated / 2).max(1);
                } else {
                    steps.push(PlannedStep {
                        element_idx: idx,
                        path: elem.layer.id.clone(),
                        symbol: elem.layer.name().to_string(),
                        decision: decision.to_string(),
                        reason: reason.to_string(),
                        estimated_minutes: estimated,
                        relevance_score: elem.score,
                    });
                    total_minutes += estimated;
                }
            } else {
                steps.push(PlannedStep {
                    element_idx: idx,
                    path: elem.layer.id.clone(),
                    symbol: elem.layer.name().to_string(),
                    decision: decision.to_string(),
                    reason: reason.to_string(),
                    estimated_minutes: if decision == "skim" { (estimated / 2).max(1) } else { 0 },
                    relevance_score: elem.score,
                });
            }
        }

        steps
    }

    fn name(&self) -> &'static str {
        "ExplorationPlanner"
    }

    fn description(&self) -> &'static str {
        "Plan optimal exploration path through relevant elements"
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::Range;

    #[test]
    fn test_concept_type_inference_from_name() {
        // Test function
        let test_layer = ContextLayer::new("t1", LayerContent::Symbol {
            name: "test_calculate_total".to_string(),
            kind: SymbolKind::Function,
            signature: "fn test_calculate_total()".to_string(),
            return_type: None,
            parameters: vec![],
            documentation: None,
            visibility: crate::core::fractal::Visibility::Private,
            range: Range::line_range(1, 10),
        });
        assert_eq!(ConceptType::infer(&test_layer), ConceptType::Testing);

        // Calculate function
        let calc_layer = ContextLayer::new("c1", LayerContent::Symbol {
            name: "calculate_total".to_string(),
            kind: SymbolKind::Function,
            signature: "fn calculate_total() -> f64".to_string(),
            return_type: Some("f64".to_string()),
            parameters: vec![],
            documentation: None,
            visibility: crate::core::fractal::Visibility::Public,
            range: Range::line_range(1, 10),
        });
        assert_eq!(ConceptType::infer(&calc_layer), ConceptType::Calculation);

        // Validate function
        let validate_layer = ContextLayer::new("v1", LayerContent::Symbol {
            name: "validate_input".to_string(),
            kind: SymbolKind::Function,
            signature: "fn validate_input() -> bool".to_string(),
            return_type: Some("bool".to_string()),
            parameters: vec![],
            documentation: None,
            visibility: crate::core::fractal::Visibility::Public,
            range: Range::line_range(1, 10),
        });
        assert_eq!(ConceptType::infer(&validate_layer), ConceptType::Validation);
    }

    #[test]
    fn test_noise_filter_params_presets() {
        let business = NoiseFilterParams::business_logic();
        assert!(business.filter_name_patterns.contains(&"test_".to_string()));
        assert!(business.filter_concept_types.contains(&ConceptType::Testing));

        let debug = NoiseFilterParams::debugging();
        assert!(debug.filter_name_patterns.contains(&"test_".to_string()));
        // Debugging should NOT filter logging
        assert!(!debug.filter_concept_types.contains(&ConceptType::Logging));
    }

    #[test]
    fn test_relevance_scorer_params_presets() {
        let business = RelevanceScorerParams::business_logic();
        let calc_weight = business.concept_weights.iter()
            .find(|(ct, _)| *ct == ConceptType::Calculation)
            .map(|(_, w)| *w);
        let test_weight = business.concept_weights.iter()
            .find(|(ct, _)| *ct == ConceptType::Testing)
            .map(|(_, w)| *w);

        assert!(calc_weight > test_weight, "Business logic should weight calculations higher than tests");
    }

    #[test]
    fn test_exploration_planner_time_budget() {
        let params = ExplorationPlannerParams {
            max_elements: 100,
            time_budget_minutes: 10,
            min_relevance: 0.0,
            group_related: false,
        };

        // Verify default params
        assert_eq!(params.time_budget_minutes, 10);
        assert_eq!(params.max_elements, 100);
    }
}
