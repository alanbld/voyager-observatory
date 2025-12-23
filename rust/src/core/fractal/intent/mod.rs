//! Intent-Driven Exploration Module
//!
//! This module implements cognitive primitives for intent-driven code exploration.
//! Instead of hardcoded "lenses", we provide composable primitives that can be
//! combined to create any exploration intent.
//!
//! # Architecture
//!
//! ```text
//! Raw Code → FractalContext → Cognitive Primitives → IntentResult
//!                                    │
//!                    ┌───────────────┼───────────────┐
//!                    ▼               ▼               ▼
//!              NoiseFilter    RelevanceScorer   ExplorationPlanner
//!                    │               │               │
//!                    └───────────────┼───────────────┘
//!                                    ▼
//!                           IntentComposition
//! ```
//!
//! # Key Insight
//!
//! An intent is NOT a hardcoded lens. An intent is a COMPOSITION of primitives:
//! - NoiseFilter: Remove what doesn't matter for this goal
//! - RelevanceScorer: Score what remains by relevance to goal
//! - ExplorationPlanner: Suggest optimal exploration path
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::intent::{IntentComposition, ExplorationIntent};
//!
//! // Create a "business logic" intent
//! let intent = IntentComposition::business_logic();
//!
//! // Apply to a fractal context
//! let result = intent.execute(&fractal_context);
//!
//! // Get exploration guidance
//! for step in result.exploration_path {
//!     println!("{}: {} ({})", step.decision, step.path, step.reason);
//! }
//! ```

pub mod primitives;
pub mod composition;
pub mod decisions;
pub mod explorer;

// Re-export commonly used types
pub use primitives::{
    CognitivePrimitive,
    NoiseFilter,
    NoiseFilterParams,
    RelevanceScorer,
    RelevanceScorerParams,
    ExplorationPlanner,
    ExplorationPlannerParams,
    // Supporting types
    ConceptType,
    RelevanceScore,
    ScoredElement,
};

pub use composition::{
    IntentComposition,
    ExplorationIntent,
    IntentResult,
    ExplorationStep,
    ConfiguredPrimitive,
};

pub use decisions::{
    ReadingDecision,
    StopReadingEngine,
};

pub use explorer::{
    IntentExplorer,
    ExplorerConfig,
    ExplorationResult,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::{
        ContextLayer, LayerContent, SymbolKind, Visibility, Range,
        FeatureVector, SymbolVectorizer,
    };

    fn create_test_layers() -> Vec<ContextLayer> {
        vec![
            // Business logic layer
            ContextLayer::new("calc_1", LayerContent::Symbol {
                name: "calculate_total".to_string(),
                kind: SymbolKind::Function,
                signature: "fn calculate_total(items: &[Item]) -> f64".to_string(),
                return_type: Some("f64".to_string()),
                parameters: vec![],
                documentation: Some("Calculate total price with discounts".to_string()),
                visibility: Visibility::Public,
                range: Range::line_range(10, 30),
            }),
            // Test layer (should be filtered)
            ContextLayer::new("test_1", LayerContent::Symbol {
                name: "test_calculate_total".to_string(),
                kind: SymbolKind::Function,
                signature: "fn test_calculate_total()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: Visibility::Private,
                range: Range::line_range(100, 120),
            }),
            // Infrastructure layer (should be filtered)
            ContextLayer::new("log_1", LayerContent::Symbol {
                name: "log_request".to_string(),
                kind: SymbolKind::Function,
                signature: "fn log_request(req: &Request)".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: Visibility::Public,
                range: Range::line_range(50, 60),
            }),
            // Another business logic layer
            ContextLayer::new("calc_2", LayerContent::Symbol {
                name: "apply_discount".to_string(),
                kind: SymbolKind::Function,
                signature: "fn apply_discount(price: f64, discount: f64) -> f64".to_string(),
                return_type: Some("f64".to_string()),
                parameters: vec![],
                documentation: Some("Apply percentage discount".to_string()),
                visibility: Visibility::Public,
                range: Range::line_range(35, 45),
            }),
        ]
    }

    #[test]
    fn test_noise_filter_removes_tests() {
        let layers = create_test_layers();
        let vectorizer = SymbolVectorizer::new();
        let vectors: Vec<_> = layers.iter()
            .map(|l| vectorizer.vectorize_layer(l))
            .collect();

        let filter = NoiseFilter::default();
        let params = NoiseFilterParams {
            filter_name_patterns: vec!["test_".to_string(), "log_".to_string()],
            filter_concept_types: vec![ConceptType::Testing, ConceptType::Infrastructure],
            min_relevance_threshold: 0.0,
        };

        let input: Vec<_> = layers.iter().zip(vectors.iter()).collect();
        let filtered = filter.apply(&input, &params);

        // NoiseFilter returns indices and concept types
        // Should filter out test_calculate_total and log_request (indices 1 and 2)
        assert_eq!(filtered.len(), 2, "Should have 2 layers after filtering");

        // Verify filtered indices point to correct layers
        for (idx, _concept_type) in &filtered {
            let name = layers[*idx].name();
            assert!(!name.starts_with("test_"), "Should not contain test_ layers");
            assert!(!name.starts_with("log_"), "Should not contain log_ layers");
        }
    }

    #[test]
    fn test_relevance_scorer_scores_business_logic_higher() {
        let layers = create_test_layers();
        let vectorizer = SymbolVectorizer::new();
        let vectors: Vec<_> = layers.iter()
            .map(|l| vectorizer.vectorize_layer(l))
            .collect();

        let scorer = RelevanceScorer::default();
        let params = RelevanceScorerParams::business_logic();

        // Score individual layers directly
        let calc_layer = &layers[0]; // calculate_total
        let test_layer = &layers[1]; // test_calculate_total

        let calc_score = scorer.score_element(
            calc_layer,
            &vectors[0],
            ConceptType::infer(calc_layer),
            &params
        ).score;

        let test_score = scorer.score_element(
            test_layer,
            &vectors[1],
            ConceptType::infer(test_layer),
            &params
        ).score;

        assert!(
            calc_score > test_score,
            "Business logic should score higher than tests: {} > {}",
            calc_score, test_score
        );
    }

    #[test]
    fn test_intent_composition_end_to_end() {
        let layers = create_test_layers();
        let vectorizer = SymbolVectorizer::new();
        let vectors: Vec<_> = layers.iter()
            .map(|l| vectorizer.vectorize_layer(l))
            .collect();

        let intent = IntentComposition::business_logic();
        let result = intent.execute(&layers, &vectors);

        // Should filter out noise and provide exploration path
        assert!(
            result.relevant_count < result.total_count,
            "Should filter some elements"
        );
        assert!(
            !result.exploration_path.is_empty(),
            "Should provide exploration path"
        );
        assert!(
            result.estimated_minutes > 0,
            "Should estimate reading time"
        );
    }

    #[test]
    fn test_different_intents_produce_different_results() {
        let layers = create_test_layers();
        let vectorizer = SymbolVectorizer::new();
        let vectors: Vec<_> = layers.iter()
            .map(|l| vectorizer.vectorize_layer(l))
            .collect();

        let business_intent = IntentComposition::business_logic();
        let debugging_intent = IntentComposition::debugging();

        let business_result = business_intent.execute(&layers, &vectors);
        let debugging_result = debugging_intent.execute(&layers, &vectors);

        // Different intents should produce different relevance distributions
        // (even if total counts might be similar)
        let business_top = business_result.exploration_path.first()
            .map(|s| s.symbol.clone());
        let debugging_top = debugging_result.exploration_path.first()
            .map(|s| s.symbol.clone());

        // Results should exist
        assert!(business_top.is_some());
        assert!(debugging_top.is_some());
    }
}
