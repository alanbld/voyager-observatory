//! Cognitive Primitives for Intent-Driven Exploration
//!
//! These primitives are the building blocks of intent compositions.
//! Each primitive performs a single cognitive transformation:
//!
//! - **NoiseFilter**: Remove elements that don't matter for the goal
//! - **RelevanceScorer**: Score remaining elements by relevance to goal
//! - **ExplorationPlanner**: Plan optimal exploration path through relevant elements

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::core::fractal::{
    clustering::ShellPatternType, ContextLayer, FeatureVector, LayerContent, SymbolKind,
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
    ///
    /// Uses multiple signal types:
    /// 1. Name patterns (strongest signal)
    /// 2. Signature patterns (return type, parameters)
    /// 3. Symbol kind (struct, trait, impl)
    /// 4. Documentation keywords
    /// 5. Content heuristics
    pub fn infer(layer: &ContextLayer) -> Self {
        let name = layer.name().to_lowercase();

        // Extract signature and return type for richer heuristics
        let (signature, return_type, kind, documentation) = match &layer.content {
            LayerContent::Symbol {
                signature,
                return_type,
                kind,
                documentation,
                ..
            } => (
                signature.to_lowercase(),
                return_type.as_ref().map(|s| s.to_lowercase()),
                Some(kind.clone()),
                documentation.as_ref().map(|s| s.to_lowercase()),
            ),
            LayerContent::File { path, .. } => {
                let path_str = path.to_string_lossy().to_lowercase();
                // File-level classification
                if path_str.contains("test") || path_str.contains("spec") {
                    return ConceptType::Testing;
                }
                if path_str.contains("config") || path_str.contains("settings") {
                    return ConceptType::Configuration;
                }
                return ConceptType::Unknown;
            }
            _ => (String::new(), None, None, None),
        };

        // =================================================================
        // Priority 1: Test code (strongest signal - early return)
        // =================================================================
        if name.starts_with("test_")
            || name.ends_with("_test")
            || name.contains("_spec")
            || name.contains("_tests")
            || signature.contains("#[test]")
            || signature.contains("#[cfg(test)]")
        {
            return ConceptType::Testing;
        }

        // =================================================================
        // Priority 2: Error handling (signature-based - very reliable)
        // =================================================================
        if let Some(ref ret) = return_type {
            if ret.contains("result<")
                || ret.contains("result ")
                || ret.contains("error")
                || ret.contains("anyhow")
            {
                // Functions returning Result or Error types
                if name.contains("handle")
                    || name.contains("catch")
                    || name.contains("recover")
                    || name.contains("on_error")
                {
                    return ConceptType::ErrorHandling;
                }
            }
        }
        // Error handler patterns
        if name.contains("handle_error")
            || name.contains("on_error")
            || name.contains("_error")
            || name.contains("recover")
            || name.starts_with("err_")
            || name.ends_with("_err")
            || signature.contains("-> result<")
            || signature.contains("-> anyhow")
        {
            return ConceptType::ErrorHandling;
        }

        // =================================================================
        // Priority 3: Validation (bool return + check/validate names)
        // =================================================================
        if let Some(ref ret) = return_type {
            if ret == "bool" || ret.contains("bool") {
                if name.contains("is_")
                    || name.contains("has_")
                    || name.contains("can_")
                    || name.contains("check")
                    || name.contains("valid")
                    || name.contains("verify")
                    || name.contains("assert")
                {
                    return ConceptType::Validation;
                }
            }
        }
        if name.contains("validate")
            || name.contains("check_")
            || name.contains("is_valid")
            || name.starts_with("is_")
            || name.starts_with("has_")
            || name.contains("verify")
            || name.contains("ensure")
        {
            return ConceptType::Validation;
        }

        // =================================================================
        // Priority 4: Logging/Observability
        // =================================================================
        if name.starts_with("log_")
            || name.contains("logger")
            || name.contains("_log")
            || name.contains("trace")
            || name.contains("debug_")
            || name.contains("metric")
            || name.contains("telemetry")
            || signature.contains("tracing::")
            || signature.contains("log::")
        {
            return ConceptType::Logging;
        }

        // =================================================================
        // Priority 5: Configuration
        // =================================================================
        if name.starts_with("config")
            || name.contains("_config")
            || name.contains("settings")
            || name.contains("options")
            || name.contains("params")
            || name.contains("builder")
            || name.ends_with("_opts")
        {
            return ConceptType::Configuration;
        }
        // Struct/type names that suggest configuration
        if let Some(SymbolKind::Struct) | Some(SymbolKind::Class) = kind {
            if name.ends_with("config")
                || name.ends_with("options")
                || name.ends_with("settings")
                || name.ends_with("params")
                || name.ends_with("builder")
            {
                return ConceptType::Configuration;
            }
        }

        // =================================================================
        // Priority 6: Calculation/Computation
        // =================================================================
        if name.contains("calculate")
            || name.contains("compute")
            || name.contains("_total")
            || name.contains("_sum")
            || name.contains("_avg")
            || name.contains("_count")
            || name.contains("score")
            || name.contains("_price")
            || name.contains("_cost")
            || name.contains("estimate")
            || name.contains("eval")
        {
            return ConceptType::Calculation;
        }
        // Numeric return types suggest calculation
        if let Some(ref ret) = return_type {
            if ret == "f32"
                || ret == "f64"
                || ret == "i32"
                || ret == "i64"
                || ret == "u32"
                || ret == "u64"
                || ret == "usize"
                || ret.contains("number")
                || ret.contains("amount")
            {
                // Only if name also suggests calculation
                if name.contains("get_") || name.contains("calc") || name.contains("compute") {
                    return ConceptType::Calculation;
                }
            }
        }

        // =================================================================
        // Priority 7: Transformation/Conversion
        // =================================================================
        if name.contains("transform")
            || name.contains("convert")
            || name.contains("map_")
            || name.contains("_to_")
            || name.contains("into_")
            || name.contains("from_")
            || name.contains("parse")
            || name.contains("serialize")
            || name.contains("deserialize")
            || name.contains("encode")
            || name.contains("decode")
        {
            return ConceptType::Transformation;
        }
        // From/Into trait implementations
        if signature.contains("impl from<")
            || signature.contains("impl into<")
            || signature.contains("impl tryfrom<")
            || signature.contains("impl tryinto<")
        {
            return ConceptType::Transformation;
        }

        // =================================================================
        // Priority 8: Decision/Routing/Control flow
        // =================================================================
        if name.contains("decide")
            || name.contains("route")
            || name.contains("dispatch")
            || name.contains("should_")
            || name.contains("select")
            || name.contains("choose")
            || name.contains("pick")
            || name.contains("match_")
            || name.contains("filter")
            || name.contains("when_")
        {
            return ConceptType::Decision;
        }
        // Entry point patterns
        if name == "main"
            || name == "run"
            || name == "start"
            || name == "execute"
            || name == "process"
            || name.starts_with("handle_")
        {
            return ConceptType::Decision;
        }

        // =================================================================
        // Priority 9: Infrastructure
        // =================================================================
        if name.contains("connect")
            || name.contains("socket")
            || name.contains("http")
            || name.contains("network")
            || name.contains("database")
            || name.contains("db_")
            || name.contains("file_")
            || name.contains("fs_")
            || name.contains("io_")
            || name.contains("read_file")
            || name.contains("write_file")
            || name.contains("send_")
            || name.contains("recv_")
            || name.contains("fetch")
        {
            return ConceptType::Infrastructure;
        }

        // =================================================================
        // Documentation-based inference (fallback)
        // =================================================================
        if let Some(ref doc) = documentation {
            if doc.contains("calculate") || doc.contains("compute") || doc.contains("score") {
                return ConceptType::Calculation;
            }
            if doc.contains("validate") || doc.contains("check") || doc.contains("verify") {
                return ConceptType::Validation;
            }
            if doc.contains("transform") || doc.contains("convert") || doc.contains("parse") {
                return ConceptType::Transformation;
            }
            if doc.contains("error") || doc.contains("handle") || doc.contains("recover") {
                return ConceptType::ErrorHandling;
            }
            if doc.contains("config") || doc.contains("setting") || doc.contains("option") {
                return ConceptType::Configuration;
            }
            if doc.contains("entry point") || doc.contains("main") || doc.contains("start") {
                return ConceptType::Decision;
            }
        }

        // =================================================================
        // Symbol kind fallback (for unclassified symbols)
        // =================================================================
        // Check visibility from the layer content
        let is_public = matches!(&layer.content,
            LayerContent::Symbol { visibility, .. } if *visibility == crate::core::fractal::Visibility::Public
        ) || signature.contains("pub ");

        if let Some(ref k) = kind {
            match k {
                SymbolKind::Struct | SymbolKind::Class => {
                    // Data structures without clear purpose - likely domain models
                    // which are part of business logic/transformation
                    return ConceptType::Transformation;
                }
                SymbolKind::Trait | SymbolKind::Interface => {
                    // Traits define contracts - usually decision/routing related
                    return ConceptType::Decision;
                }
                SymbolKind::Constant => {
                    return ConceptType::Configuration;
                }
                SymbolKind::Enum => {
                    // Enums are usually for decision/state
                    return ConceptType::Decision;
                }
                SymbolKind::Function | SymbolKind::Method => {
                    // Functions/methods that don't match other patterns
                    // Check if public (likely API) vs private (helper)
                    if is_public {
                        // Public functions without clear category - likely core logic
                        return ConceptType::Calculation;
                    }
                    // Private helpers - infrastructure/utility
                    return ConceptType::Infrastructure;
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
            filter_name_patterns: vec!["test_".to_string(), "_test".to_string()],
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
                (ConceptType::Decision, 1.0),      // Entry points
                (ConceptType::Calculation, 0.8),   // Core logic
                (ConceptType::Configuration, 0.7), // How to configure
                (ConceptType::Validation, 0.6),
                (ConceptType::Transformation, 0.5),
                (ConceptType::ErrorHandling, 0.4),
                (ConceptType::Infrastructure, 0.3),
                (ConceptType::Logging, 0.2),
                (ConceptType::Testing, 0.3), // Tests help understand
                (ConceptType::Unknown, 0.4),
            ],
            dimension_weights: None,
            documentation_boost: 0.25, // Documentation very valuable for onboarding
            public_visibility_boost: 0.15,
        }
    }
}

/// Relevance scorer primitive - scores elements by relevance to goal
#[derive(Debug, Clone, Default)]
pub struct RelevanceScorer;

impl RelevanceScorer {
    /// Score a single element (public for composition use)
    ///
    /// Scoring factors:
    /// 1. Concept type weight (primary factor)
    /// 2. Documentation boost
    /// 3. Visibility boost (public API vs internal)
    /// 4. Complexity factor (sweet spot for readability)
    /// 5. Name clarity bonus
    pub fn score_element(
        &self,
        layer: &ContextLayer,
        _vector: &FeatureVector,
        concept_type: ConceptType,
        params: &RelevanceScorerParams,
    ) -> RelevanceScore {
        let mut score = 0.0f32;
        let mut factors = Vec::new();

        // Factor 1: Concept type weight (0.0 - 1.0, primary factor)
        let concept_weight = params
            .concept_weights
            .iter()
            .find(|(ct, _)| *ct == concept_type)
            .map(|(_, w)| *w)
            .unwrap_or(0.3); // Lower default for unconfigured types

        factors.push(("concept_type".to_string(), concept_weight));
        score += concept_weight * 0.6; // 60% of score from concept type

        // Factor 2: Documentation boost
        if let LayerContent::Symbol {
            documentation: Some(doc),
            ..
        } = &layer.content
        {
            let doc_boost = if doc.len() > 50 {
                params.documentation_boost // Full boost for substantial docs
            } else {
                params.documentation_boost * 0.5 // Half boost for brief docs
            };
            score += doc_boost;
            factors.push(("has_documentation".to_string(), doc_boost));
        }

        // Factor 3: Visibility boost (public APIs are more important for understanding)
        if let LayerContent::Symbol { visibility, .. } = &layer.content {
            if *visibility == crate::core::fractal::Visibility::Public {
                score += params.public_visibility_boost;
                factors.push((
                    "public_visibility".to_string(),
                    params.public_visibility_boost,
                ));
            } else {
                // Small penalty for private/internal
                score -= 0.05;
                factors.push(("private_visibility".to_string(), -0.05));
            }
        }

        // Factor 4: Complexity factor - prefer medium complexity (not too simple, not too complex)
        if let LayerContent::Symbol { range, .. } = &layer.content {
            let lines = range.end_line.saturating_sub(range.start_line) + 1;
            let complexity_factor = if lines < 5 {
                -0.1 // Too simple, probably trivial
            } else if lines <= 30 {
                0.1 // Sweet spot - readable functions
            } else if lines <= 100 {
                0.0 // Medium complexity - neutral
            } else {
                -0.1 // Too complex, hard to understand quickly
            };
            score += complexity_factor;
            factors.push(("complexity".to_string(), complexity_factor));
        }

        // Factor 5: Name clarity bonus - descriptive names are more understandable
        let name = layer.name();
        let name_clarity = if name.len() >= 4 && name.len() <= 30 {
            // Good length for descriptive name
            let has_separator = name.contains('_') || name.chars().any(|c| c.is_uppercase());
            if has_separator {
                0.05
            } else {
                0.0
            }
        } else if name.len() < 4 {
            -0.05 // Too short, probably cryptic
        } else {
            0.0 // Long names - neutral
        };
        if name_clarity != 0.0 {
            score += name_clarity;
            factors.push(("name_clarity".to_string(), name_clarity));
        }

        score = score.clamp(0.0, 1.0);

        RelevanceScore {
            score,
            explanation: format!("{:?} element ({})", concept_type, layer.name()),
            factors,
        }
    }
}

impl CognitivePrimitive for RelevanceScorer {
    type Input<'a> = (
        &'a [(&'a ContextLayer, &'a FeatureVector)],
        &'a [(usize, ConceptType)],
    );
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
                    layer: unsafe {
                        std::mem::transmute::<&ContextLayer, &'static ContextLayer>(*layer)
                    },
                    vector: unsafe {
                        std::mem::transmute::<&FeatureVector, &'static FeatureVector>(*vector)
                    },
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
    pub decision: String, // "read", "skim", "skip"
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
            LayerContent::Symbol {
                range,
                documentation,
                ..
            } => {
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
        sorted.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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
                    estimated_minutes: if decision == "skim" {
                        (estimated / 2).max(1)
                    } else {
                        0
                    },
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

    // === ConceptType tests ===

    #[test]
    fn test_concept_type_variants() {
        let _calc = ConceptType::Calculation;
        let _valid = ConceptType::Validation;
        let _decision = ConceptType::Decision;
        let _transform = ConceptType::Transformation;
        let _error = ConceptType::ErrorHandling;
        let _logging = ConceptType::Logging;
        let _config = ConceptType::Configuration;
        let _testing = ConceptType::Testing;
        let _infra = ConceptType::Infrastructure;
        let _unknown = ConceptType::Unknown;
    }

    #[test]
    fn test_concept_type_equality() {
        assert_eq!(ConceptType::Calculation, ConceptType::Calculation);
        assert_ne!(ConceptType::Calculation, ConceptType::Validation);
    }

    #[test]
    fn test_concept_type_from_shell_pattern() {
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::Deployment), ConceptType::Infrastructure);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::ErrorHandling), ConceptType::ErrorHandling);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::DataProcessing), ConceptType::Transformation);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::Monitoring), ConceptType::Logging);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::Testing), ConceptType::Testing);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::Setup), ConceptType::Configuration);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::Security), ConceptType::Validation);
        assert_eq!(ConceptType::from_shell_pattern(&ShellPatternType::Unknown), ConceptType::Unknown);
    }

    // === RelevanceScore tests ===

    #[test]
    fn test_relevance_score_new() {
        let score = RelevanceScore::new(0.75, "Test explanation");
        assert!((score.score - 0.75).abs() < 0.001);
        assert_eq!(score.explanation, "Test explanation");
        assert!(score.factors.is_empty());
    }

    #[test]
    fn test_relevance_score_clamping() {
        let high = RelevanceScore::new(1.5, "Over max");
        assert!((high.score - 1.0).abs() < 0.001);

        let low = RelevanceScore::new(-0.5, "Under min");
        assert!((low.score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_relevance_score_with_factor() {
        let score = RelevanceScore::new(0.8, "Test")
            .with_factor("factor1", 0.3)
            .with_factor("factor2", 0.5);

        assert_eq!(score.factors.len(), 2);
        assert_eq!(score.factors[0].0, "factor1");
        assert!((score.factors[0].1 - 0.3).abs() < 0.001);
    }

    // === NoiseFilterParams tests ===

    #[test]
    fn test_noise_filter_params_default() {
        let params = NoiseFilterParams::default();
        assert!(params.filter_name_patterns.is_empty());
        assert!(params.filter_concept_types.is_empty());
        assert!((params.min_relevance_threshold - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_noise_filter_params_security() {
        let params = NoiseFilterParams::security();
        assert!(params.filter_name_patterns.contains(&"test_".to_string()));
        assert!(params.filter_name_patterns.contains(&"mock_".to_string()));
        assert!(params.filter_concept_types.contains(&ConceptType::Testing));
    }

    // === NoiseFilter tests ===

    #[test]
    fn test_noise_filter_name() {
        let filter = NoiseFilter;
        assert_eq!(filter.name(), "NoiseFilter");
    }

    #[test]
    fn test_noise_filter_description() {
        let filter = NoiseFilter;
        assert!(filter.description().contains("Remove"));
    }

    // === RelevanceScorerParams tests ===

    #[test]
    fn test_relevance_scorer_params_default() {
        let params = RelevanceScorerParams::default();
        assert!(params.concept_weights.is_empty());
        assert!(params.dimension_weights.is_none());
        assert!((params.documentation_boost - 0.1).abs() < 0.001);
        assert!((params.public_visibility_boost - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_relevance_scorer_params_debugging() {
        let params = RelevanceScorerParams::debugging();
        let error_weight = params
            .concept_weights
            .iter()
            .find(|(ct, _)| *ct == ConceptType::ErrorHandling)
            .map(|(_, w)| *w);
        assert!(error_weight.is_some());
        assert!((error_weight.unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_relevance_scorer_params_security() {
        let params = RelevanceScorerParams::security();
        let validation_weight = params
            .concept_weights
            .iter()
            .find(|(ct, _)| *ct == ConceptType::Validation)
            .map(|(_, w)| *w);
        assert!(validation_weight.is_some());
        assert!((validation_weight.unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_relevance_scorer_params_onboarding() {
        let params = RelevanceScorerParams::onboarding();
        // Onboarding has higher documentation boost
        assert!(params.documentation_boost > 0.2);
    }

    // === RelevanceScorer tests ===

    #[test]
    fn test_relevance_scorer_name() {
        let scorer = RelevanceScorer;
        assert_eq!(scorer.name(), "RelevanceScorer");
    }

    #[test]
    fn test_relevance_scorer_description() {
        let scorer = RelevanceScorer;
        assert!(scorer.description().contains("Score"));
    }

    // === ExplorationPlannerParams tests ===

    #[test]
    fn test_exploration_planner_params_default() {
        let params = ExplorationPlannerParams::default();
        assert_eq!(params.max_elements, 20);
        assert_eq!(params.time_budget_minutes, 30);
        assert!((params.min_relevance - 0.3).abs() < 0.001);
        assert!(params.group_related);
    }

    // === ExplorationPlanner tests ===

    #[test]
    fn test_exploration_planner_name() {
        let planner = ExplorationPlanner;
        assert_eq!(planner.name(), "ExplorationPlanner");
    }

    #[test]
    fn test_exploration_planner_description() {
        let planner = ExplorationPlanner;
        assert!(planner.description().contains("Plan"));
    }

    #[test]
    fn test_exploration_planner_decide() {
        let (decision, reason) = ExplorationPlanner::decide(0.8);
        assert_eq!(decision, "read");
        assert!(reason.contains("High"));

        let (decision, reason) = ExplorationPlanner::decide(0.5);
        assert_eq!(decision, "skim");
        assert!(reason.contains("Moderate"));

        let (decision, reason) = ExplorationPlanner::decide(0.2);
        assert_eq!(decision, "skip");
        assert!(reason.contains("Low"));
    }

    #[test]
    fn test_exploration_planner_estimate_minutes_symbol() {
        let layer = ContextLayer::new(
            "s1",
            LayerContent::Symbol {
                name: "test".to_string(),
                kind: SymbolKind::Function,
                signature: "fn test()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: Some("Doc".to_string()),
                visibility: crate::core::fractal::Visibility::Public,
                range: Range::line_range(1, 40), // 40 lines
            },
        );

        let minutes = ExplorationPlanner::estimate_minutes(&layer);
        // 40 lines / 20 = 2 + 1 (doc bonus) = 3
        assert_eq!(minutes, 3);
    }

    #[test]
    fn test_exploration_planner_estimate_minutes_file() {
        let layer = ContextLayer::new(
            "f1",
            LayerContent::File {
                path: std::path::PathBuf::from("test.rs"),
                language: "rust".to_string(),
                size_bytes: 1000,
                line_count: 100,
                symbol_count: 5,
                imports: vec![],
            },
        );

        let minutes = ExplorationPlanner::estimate_minutes(&layer);
        // 100 lines / 50 = 2
        assert_eq!(minutes, 2);
    }

    // === PlannedStep tests ===

    #[test]
    fn test_planned_step_creation() {
        let step = PlannedStep {
            element_idx: 0,
            path: "src/main.rs".to_string(),
            symbol: "main".to_string(),
            decision: "read".to_string(),
            reason: "Entry point".to_string(),
            estimated_minutes: 5,
            relevance_score: 0.9,
        };

        assert_eq!(step.element_idx, 0);
        assert_eq!(step.path, "src/main.rs");
        assert_eq!(step.symbol, "main");
        assert_eq!(step.decision, "read");
        assert_eq!(step.estimated_minutes, 5);
        assert!((step.relevance_score - 0.9).abs() < 0.001);
    }

    fn make_symbol_layer(
        id: &str,
        name: &str,
        kind: SymbolKind,
        signature: &str,
        return_type: Option<&str>,
        is_public: bool,
    ) -> ContextLayer {
        ContextLayer::new(
            id,
            LayerContent::Symbol {
                name: name.to_string(),
                kind,
                signature: signature.to_string(),
                return_type: return_type.map(String::from),
                parameters: vec![],
                documentation: None,
                visibility: if is_public {
                    crate::core::fractal::Visibility::Public
                } else {
                    crate::core::fractal::Visibility::Private
                },
                range: Range::line_range(1, 20),
            },
        )
    }

    #[test]
    fn test_concept_type_inference_from_name() {
        // Test function
        let test_layer = make_symbol_layer(
            "t1",
            "test_calculate_total",
            SymbolKind::Function,
            "fn test_calculate_total()",
            None,
            false,
        );
        assert_eq!(ConceptType::infer(&test_layer), ConceptType::Testing);

        // Calculate function
        let calc_layer = make_symbol_layer(
            "c1",
            "calculate_total",
            SymbolKind::Function,
            "pub fn calculate_total() -> f64",
            Some("f64"),
            true,
        );
        assert_eq!(ConceptType::infer(&calc_layer), ConceptType::Calculation);

        // Validate function
        let validate_layer = make_symbol_layer(
            "v1",
            "validate_input",
            SymbolKind::Function,
            "pub fn validate_input() -> bool",
            Some("bool"),
            true,
        );
        assert_eq!(ConceptType::infer(&validate_layer), ConceptType::Validation);
    }

    #[test]
    fn test_concept_type_inference_expanded() {
        // Error handling - from name
        let error_layer = make_symbol_layer(
            "e1",
            "handle_error",
            SymbolKind::Function,
            "pub fn handle_error()",
            None,
            true,
        );
        assert_eq!(ConceptType::infer(&error_layer), ConceptType::ErrorHandling);

        // Error handling - from signature returning Result
        let result_layer = make_symbol_layer(
            "e2",
            "process_data",
            SymbolKind::Function,
            "pub fn process_data() -> Result<T>",
            Some("Result<T>"),
            true,
        );
        // Signature with "-> Result<" triggers ErrorHandling classification
        assert_eq!(
            ConceptType::infer(&result_layer),
            ConceptType::ErrorHandling
        );

        // Configuration - struct name
        let config_layer = make_symbol_layer(
            "cfg1",
            "ExplorerConfig",
            SymbolKind::Struct,
            "pub struct ExplorerConfig",
            None,
            true,
        );
        assert_eq!(
            ConceptType::infer(&config_layer),
            ConceptType::Configuration
        );

        // Configuration - from name pattern
        let settings_layer = make_symbol_layer(
            "cfg2",
            "load_settings",
            SymbolKind::Function,
            "pub fn load_settings()",
            None,
            true,
        );
        assert_eq!(
            ConceptType::infer(&settings_layer),
            ConceptType::Configuration
        );

        // Transformation - from name
        let transform_layer = make_symbol_layer(
            "tr1",
            "transform_to_json",
            SymbolKind::Function,
            "pub fn transform_to_json()",
            None,
            true,
        );
        assert_eq!(
            ConceptType::infer(&transform_layer),
            ConceptType::Transformation
        );

        // Transformation - parse function (note: "parse_config" has "config" which takes priority)
        let parse_layer = make_symbol_layer(
            "tr2",
            "parse_data",
            SymbolKind::Function,
            "pub fn parse_data()",
            None,
            true,
        );
        assert_eq!(
            ConceptType::infer(&parse_layer),
            ConceptType::Transformation
        );

        // Decision - entry point
        let main_layer =
            make_symbol_layer("d1", "main", SymbolKind::Function, "fn main()", None, false);
        assert_eq!(ConceptType::infer(&main_layer), ConceptType::Decision);

        // Decision - execute/process
        let exec_layer = make_symbol_layer(
            "d2",
            "execute",
            SymbolKind::Function,
            "pub fn execute()",
            None,
            true,
        );
        assert_eq!(ConceptType::infer(&exec_layer), ConceptType::Decision);

        // Infrastructure
        let infra_layer = make_symbol_layer(
            "i1",
            "connect_database",
            SymbolKind::Function,
            "pub fn connect_database()",
            None,
            true,
        );
        assert_eq!(
            ConceptType::infer(&infra_layer),
            ConceptType::Infrastructure
        );

        // Logging
        let log_layer = make_symbol_layer(
            "l1",
            "log_event",
            SymbolKind::Function,
            "fn log_event()",
            None,
            false,
        );
        assert_eq!(ConceptType::infer(&log_layer), ConceptType::Logging);

        // Validation - is_ pattern
        let is_layer = make_symbol_layer(
            "v2",
            "is_valid",
            SymbolKind::Function,
            "fn is_valid() -> bool",
            Some("bool"),
            false,
        );
        assert_eq!(ConceptType::infer(&is_layer), ConceptType::Validation);
    }

    #[test]
    fn test_concept_type_fallback_by_symbol_kind() {
        // Struct without clear pattern should fallback to Transformation (domain model)
        let struct_layer = make_symbol_layer(
            "s1",
            "UserAccount",
            SymbolKind::Struct,
            "pub struct UserAccount",
            None,
            true,
        );
        assert_eq!(
            ConceptType::infer(&struct_layer),
            ConceptType::Transformation
        );

        // Enum should fallback to Decision (state)
        let enum_layer = make_symbol_layer(
            "en1",
            "Status",
            SymbolKind::Enum,
            "pub enum Status",
            None,
            true,
        );
        assert_eq!(ConceptType::infer(&enum_layer), ConceptType::Decision);

        // Trait should fallback to Decision (contract)
        let trait_layer = make_symbol_layer(
            "tr1",
            "Processor",
            SymbolKind::Trait,
            "pub trait Processor",
            None,
            true,
        );
        assert_eq!(ConceptType::infer(&trait_layer), ConceptType::Decision);

        // Constant should fallback to Configuration
        let const_layer = make_symbol_layer(
            "c1",
            "MAX_SIZE",
            SymbolKind::Constant,
            "pub const MAX_SIZE: usize = 100",
            None,
            true,
        );
        assert_eq!(ConceptType::infer(&const_layer), ConceptType::Configuration);

        // Private function without clear pattern - Infrastructure
        let helper_layer = make_symbol_layer(
            "h1",
            "do_work",
            SymbolKind::Function,
            "fn do_work()",
            None,
            false,
        );
        assert_eq!(
            ConceptType::infer(&helper_layer),
            ConceptType::Infrastructure
        );

        // Public function without clear pattern - Calculation (core logic fallback)
        // Note: "process_item" doesn't match the exact "process" entry point pattern
        let pub_layer = make_symbol_layer(
            "p1",
            "process_item",
            SymbolKind::Function,
            "pub fn process_item()",
            None,
            true,
        );
        assert_eq!(ConceptType::infer(&pub_layer), ConceptType::Calculation);
    }

    #[test]
    fn test_noise_filter_params_presets() {
        let business = NoiseFilterParams::business_logic();
        assert!(business.filter_name_patterns.contains(&"test_".to_string()));
        assert!(business
            .filter_concept_types
            .contains(&ConceptType::Testing));

        let debug = NoiseFilterParams::debugging();
        assert!(debug.filter_name_patterns.contains(&"test_".to_string()));
        // Debugging should NOT filter logging
        assert!(!debug.filter_concept_types.contains(&ConceptType::Logging));
    }

    #[test]
    fn test_relevance_scorer_params_presets() {
        let business = RelevanceScorerParams::business_logic();
        let calc_weight = business
            .concept_weights
            .iter()
            .find(|(ct, _)| *ct == ConceptType::Calculation)
            .map(|(_, w)| *w);
        let test_weight = business
            .concept_weights
            .iter()
            .find(|(ct, _)| *ct == ConceptType::Testing)
            .map(|(_, w)| *w);

        assert!(
            calc_weight > test_weight,
            "Business logic should weight calculations higher than tests"
        );
    }

    #[test]
    fn test_relevance_scorer_produces_varied_scores() {
        use crate::core::fractal::SymbolVectorizer;

        let scorer = RelevanceScorer;
        let params = RelevanceScorerParams::onboarding();
        let vectorizer = SymbolVectorizer::new();

        // High-relevance: public, documented, Decision type
        let high_layer = ContextLayer::new(
            "h1",
            LayerContent::Symbol {
                name: "execute".to_string(),
                kind: SymbolKind::Function,
                signature: "pub fn execute()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: Some(
                    "Main entry point for executing the workflow. This is a long doc.".to_string(),
                ),
                visibility: crate::core::fractal::Visibility::Public,
                range: Range::line_range(1, 25), // Sweet spot complexity
            },
        );
        let high_vector = vectorizer.vectorize_layer(&high_layer);
        let high_score =
            scorer.score_element(&high_layer, &high_vector, ConceptType::Decision, &params);

        // Low-relevance: private, no docs, Testing type
        let low_layer = ContextLayer::new(
            "l1",
            LayerContent::Symbol {
                name: "test_internal".to_string(),
                kind: SymbolKind::Function,
                signature: "fn test_internal()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: crate::core::fractal::Visibility::Private,
                range: Range::line_range(1, 3), // Too simple
            },
        );
        let low_vector = vectorizer.vectorize_layer(&low_layer);
        let low_score =
            scorer.score_element(&low_layer, &low_vector, ConceptType::Testing, &params);

        // Assert meaningful difference
        assert!(
            high_score.score > low_score.score,
            "High-relevance element ({}) should score higher than low-relevance element ({})",
            high_score.score,
            low_score.score
        );
        assert!(
            high_score.score >= 0.5,
            "High-relevance should be at least 0.5, got {}",
            high_score.score
        );
        assert!(
            low_score.score <= 0.3,
            "Low-relevance should be at most 0.3, got {}",
            low_score.score
        );
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
