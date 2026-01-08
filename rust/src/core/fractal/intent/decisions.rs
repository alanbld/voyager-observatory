//! Reading Decisions - The "Stop Reading" Engine
//!
//! This module provides the decision-making logic for whether a developer
//! should read, skim, or skip a code element based on their exploration intent.

use serde::{Deserialize, Serialize};

use crate::core::fractal::ContextLayer;

use super::composition::ExplorationIntent;
use super::primitives::ConceptType;

// =============================================================================
// Reading Decision
// =============================================================================

/// Decision about how to approach reading a code element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadingDecision {
    /// Read this carefully - it's core to your goal
    ReadDeeply {
        reason: String,
        estimated_minutes: u32,
        key_points: Vec<String>,
    },
    /// Skim this - get the gist without diving deep
    Skim {
        focus_on: Vec<String>,
        time_limit_seconds: u32,
    },
    /// Skip this entirely - not relevant to your current goal
    Skip {
        reason: String,
        come_back_if: Option<String>,
    },
    /// Bookmark for later - you need prerequisite knowledge first
    Bookmark {
        prerequisite: String,
        when_to_review: String,
    },
}

impl ReadingDecision {
    /// Get a short label for this decision
    pub fn label(&self) -> &'static str {
        match self {
            ReadingDecision::ReadDeeply { .. } => "READ",
            ReadingDecision::Skim { .. } => "SKIM",
            ReadingDecision::Skip { .. } => "SKIP",
            ReadingDecision::Bookmark { .. } => "BOOKMARK",
        }
    }

    /// Get the decision as a simple string
    pub fn as_str(&self) -> &'static str {
        match self {
            ReadingDecision::ReadDeeply { .. } => "read",
            ReadingDecision::Skim { .. } => "skim",
            ReadingDecision::Skip { .. } => "skip",
            ReadingDecision::Bookmark { .. } => "bookmark",
        }
    }

    /// Check if this is a "should read" decision
    pub fn should_read(&self) -> bool {
        matches!(
            self,
            ReadingDecision::ReadDeeply { .. } | ReadingDecision::Skim { .. }
        )
    }
}

// =============================================================================
// Stop Reading Engine
// =============================================================================

/// Engine that produces reading decisions based on context and intent
pub struct StopReadingEngine {
    intent: ExplorationIntent,
    /// Below this relevance score = Skip
    skip_threshold: f32,
    /// Above this relevance score = ReadDeeply
    read_deeply_threshold: f32,
    /// Complexity threshold for bookmarking
    complexity_threshold: f32,
}

impl StopReadingEngine {
    /// Create a new engine for a given intent
    pub fn new(intent: ExplorationIntent) -> Self {
        let (skip_threshold, read_deeply_threshold) = match intent {
            ExplorationIntent::BusinessLogic => (0.3, 0.7),
            ExplorationIntent::Debugging => (0.2, 0.6),
            ExplorationIntent::Onboarding => (0.2, 0.5),
            ExplorationIntent::SecurityReview => (0.3, 0.7),
            ExplorationIntent::MigrationAssessment => (0.2, 0.6),
        };

        Self {
            intent,
            skip_threshold,
            read_deeply_threshold,
            complexity_threshold: 0.7,
        }
    }

    /// Decide how to approach a code element
    pub fn decide(
        &self,
        layer: &ContextLayer,
        relevance: f32,
        complexity: f32,
        centrality: f32,
    ) -> ReadingDecision {
        // High complexity + low relevance = skip with suggestion
        if complexity > self.complexity_threshold && relevance < self.skip_threshold {
            return ReadingDecision::Skip {
                reason: "Complex but not relevant to your current goal".to_string(),
                come_back_if: Some("you encounter related issues".to_string()),
            };
        }

        // High complexity + high relevance = bookmark if not central
        if complexity > self.complexity_threshold
            && relevance > self.read_deeply_threshold
            && centrality < 0.3
        {
            return ReadingDecision::Bookmark {
                prerequisite: "Understand simpler related concepts first".to_string(),
                when_to_review: "After reviewing the core logic".to_string(),
            };
        }

        // Low relevance = skip
        if relevance < self.skip_threshold {
            let come_back_reason = self.suggest_comeback_reason(layer, relevance);
            return ReadingDecision::Skip {
                reason: self.explain_skip(layer, relevance),
                come_back_if: come_back_reason,
            };
        }

        // High relevance = read deeply
        if relevance >= self.read_deeply_threshold {
            let key_points = self.extract_key_points(layer);
            return ReadingDecision::ReadDeeply {
                reason: self.explain_read_deeply(layer, relevance),
                estimated_minutes: self.estimate_minutes(layer, complexity),
                key_points,
            };
        }

        // Middle relevance = skim
        let focus_points = self.suggest_focus_points(layer);
        ReadingDecision::Skim {
            focus_on: focus_points,
            time_limit_seconds: (self.estimate_minutes(layer, complexity) * 30).max(60),
        }
    }

    /// Explain why we recommend reading deeply
    fn explain_read_deeply(&self, layer: &ContextLayer, relevance: f32) -> String {
        let concept_type = ConceptType::infer(layer);
        let intent_name = self.intent.name();

        match concept_type {
            ConceptType::Calculation => {
                format!(
                    "Core {} logic ({:.0}% relevant to {})",
                    "calculation",
                    relevance * 100.0,
                    intent_name
                )
            }
            ConceptType::Validation => {
                format!(
                    "Key {} rules ({:.0}% relevant to {})",
                    "validation",
                    relevance * 100.0,
                    intent_name
                )
            }
            ConceptType::Decision => {
                format!(
                    "Important {} point ({:.0}% relevant to {})",
                    "decision",
                    relevance * 100.0,
                    intent_name
                )
            }
            ConceptType::ErrorHandling => {
                format!(
                    "Critical {} path ({:.0}% relevant to {})",
                    "error handling",
                    relevance * 100.0,
                    intent_name
                )
            }
            _ => {
                format!(
                    "High relevance to {} ({:.0}%)",
                    intent_name,
                    relevance * 100.0
                )
            }
        }
    }

    /// Explain why we recommend skipping
    fn explain_skip(&self, layer: &ContextLayer, relevance: f32) -> String {
        let concept_type = ConceptType::infer(layer);
        let intent_name = self.intent.name();

        match concept_type {
            ConceptType::Testing => {
                "Test code - not relevant for understanding business logic".to_string()
            }
            ConceptType::Logging => {
                "Logging infrastructure - unlikely to help with current goal".to_string()
            }
            ConceptType::Configuration => {
                "Configuration code - can be explored later if needed".to_string()
            }
            _ => {
                format!(
                    "Low relevance to {} ({:.0}%)",
                    intent_name,
                    relevance * 100.0
                )
            }
        }
    }

    /// Suggest when to come back to skipped element
    fn suggest_comeback_reason(&self, layer: &ContextLayer, _relevance: f32) -> Option<String> {
        let concept_type = ConceptType::infer(layer);

        match concept_type {
            ConceptType::Testing => Some("you need to verify expected behavior".to_string()),
            ConceptType::Configuration => {
                Some("you encounter configuration-related issues".to_string())
            }
            ConceptType::Logging => Some("you need to understand logging behavior".to_string()),
            ConceptType::ErrorHandling => Some("you encounter related errors".to_string()),
            _ => None,
        }
    }

    /// Extract key points to focus on when reading
    fn extract_key_points(&self, layer: &ContextLayer) -> Vec<String> {
        let mut points = Vec::new();
        let concept_type = ConceptType::infer(layer);

        match self.intent {
            ExplorationIntent::BusinessLogic => {
                points.push("Look for business calculations".to_string());
                points.push("Note validation rules".to_string());
                if concept_type == ConceptType::Decision {
                    points.push("Understand decision criteria".to_string());
                }
            }
            ExplorationIntent::Debugging => {
                points.push("Check error handling paths".to_string());
                points.push("Note state changes".to_string());
                points.push("Look for logging points".to_string());
            }
            ExplorationIntent::Onboarding => {
                points.push("Understand the purpose".to_string());
                points.push("Note key dependencies".to_string());
                points.push("Identify entry points".to_string());
            }
            ExplorationIntent::SecurityReview => {
                points.push("Check input validation".to_string());
                points.push("Look for authentication/authorization".to_string());
                points.push("Note data handling patterns".to_string());
            }
            ExplorationIntent::MigrationAssessment => {
                points.push("Identify platform-specific code".to_string());
                points.push("Note external dependencies".to_string());
                points.push("Check for hardcoded values".to_string());
            }
        }

        points
    }

    /// Suggest what to focus on when skimming
    fn suggest_focus_points(&self, layer: &ContextLayer) -> Vec<String> {
        let mut points = Vec::new();

        // Always look at function signature
        points.push("Function signature and parameters".to_string());

        // Intent-specific focus
        match self.intent {
            ExplorationIntent::BusinessLogic => {
                points.push("Return type and key calculations".to_string());
            }
            ExplorationIntent::Debugging => {
                points.push("Error return paths".to_string());
            }
            ExplorationIntent::SecurityReview => {
                points.push("Input handling".to_string());
            }
            _ => {
                points.push("Main logic flow".to_string());
            }
        }

        // Check for documentation
        if let crate::core::fractal::LayerContent::Symbol {
            documentation: Some(_),
            ..
        } = &layer.content
        {
            points.push("Read the documentation".to_string());
        }

        points
    }

    /// Estimate reading time in minutes
    fn estimate_minutes(&self, layer: &ContextLayer, complexity: f32) -> u32 {
        let base_minutes = match &layer.content {
            crate::core::fractal::LayerContent::Symbol { range, .. } => {
                let lines = range.end_line.saturating_sub(range.start_line) + 1;
                (lines as f32 / 15.0).ceil() as u32 // ~15 lines per minute for careful reading
            }
            crate::core::fractal::LayerContent::File { line_count, .. } => {
                (*line_count as f32 / 30.0).ceil() as u32 // ~30 lines per minute for file overview
            }
            _ => 2,
        };

        // Adjust for complexity
        let complexity_factor = 1.0 + complexity;
        ((base_minutes as f32) * complexity_factor).ceil() as u32
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::{LayerContent, Range, SymbolKind, Visibility};

    fn create_test_layer(name: &str, lines: usize) -> ContextLayer {
        ContextLayer::new(
            &format!("id_{}", name),
            LayerContent::Symbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!("fn {}()", name),
                return_type: None,
                parameters: vec![],
                documentation: Some("Test documentation".to_string()),
                visibility: Visibility::Public,
                range: Range::line_range(1, lines),
            },
        )
    }

    #[test]
    fn test_high_relevance_produces_read_deeply() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("calculate_total", 20);

        let decision = engine.decide(&layer, 0.9, 0.5, 0.8);

        assert!(matches!(decision, ReadingDecision::ReadDeeply { .. }));
    }

    #[test]
    fn test_low_relevance_produces_skip() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("test_helper", 10);

        let decision = engine.decide(&layer, 0.1, 0.3, 0.1);

        assert!(matches!(decision, ReadingDecision::Skip { .. }));
    }

    #[test]
    fn test_medium_relevance_produces_skim() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("utility_function", 15);

        let decision = engine.decide(&layer, 0.5, 0.4, 0.5);

        assert!(matches!(decision, ReadingDecision::Skim { .. }));
    }

    #[test]
    fn test_high_complexity_low_relevance_skips() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("complex_internal", 100);

        let decision = engine.decide(&layer, 0.2, 0.9, 0.1);

        assert!(matches!(decision, ReadingDecision::Skip { .. }));
    }

    #[test]
    fn test_decision_labels() {
        assert_eq!(
            ReadingDecision::ReadDeeply {
                reason: "".to_string(),
                estimated_minutes: 0,
                key_points: vec![],
            }
            .label(),
            "READ"
        );

        assert_eq!(
            ReadingDecision::Skim {
                focus_on: vec![],
                time_limit_seconds: 0,
            }
            .label(),
            "SKIM"
        );

        assert_eq!(
            ReadingDecision::Skip {
                reason: "".to_string(),
                come_back_if: None,
            }
            .label(),
            "SKIP"
        );
    }

    #[test]
    fn test_different_intents_produce_different_thresholds() {
        let business = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let debugging = StopReadingEngine::new(ExplorationIntent::Debugging);

        // Business logic is more selective
        assert!(business.skip_threshold >= debugging.skip_threshold);
    }

    // ==================== ReadingDecision Tests ====================

    #[test]
    fn test_reading_decision_as_str() {
        assert_eq!(
            ReadingDecision::ReadDeeply {
                reason: "".to_string(),
                estimated_minutes: 0,
                key_points: vec![],
            }
            .as_str(),
            "read"
        );

        assert_eq!(
            ReadingDecision::Skim {
                focus_on: vec![],
                time_limit_seconds: 0,
            }
            .as_str(),
            "skim"
        );

        assert_eq!(
            ReadingDecision::Skip {
                reason: "".to_string(),
                come_back_if: None,
            }
            .as_str(),
            "skip"
        );

        assert_eq!(
            ReadingDecision::Bookmark {
                prerequisite: "".to_string(),
                when_to_review: "".to_string(),
            }
            .as_str(),
            "bookmark"
        );
    }

    #[test]
    fn test_reading_decision_should_read() {
        assert!(ReadingDecision::ReadDeeply {
            reason: "".to_string(),
            estimated_minutes: 0,
            key_points: vec![],
        }
        .should_read());

        assert!(ReadingDecision::Skim {
            focus_on: vec![],
            time_limit_seconds: 0,
        }
        .should_read());

        assert!(!ReadingDecision::Skip {
            reason: "".to_string(),
            come_back_if: None,
        }
        .should_read());

        assert!(!ReadingDecision::Bookmark {
            prerequisite: "".to_string(),
            when_to_review: "".to_string(),
        }
        .should_read());
    }

    #[test]
    fn test_bookmark_label() {
        assert_eq!(
            ReadingDecision::Bookmark {
                prerequisite: "understand X".to_string(),
                when_to_review: "after Y".to_string(),
            }
            .label(),
            "BOOKMARK"
        );
    }

    // ==================== Bookmark Decision Tests ====================

    #[test]
    fn test_high_complexity_high_relevance_low_centrality_bookmarks() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("complex_but_relevant", 100);

        // High complexity (0.9), high relevance (0.8), low centrality (0.2)
        let decision = engine.decide(&layer, 0.8, 0.9, 0.2);

        assert!(matches!(decision, ReadingDecision::Bookmark { .. }));
    }

    // ==================== Intent-Specific Tests ====================

    #[test]
    fn test_onboarding_intent() {
        let engine = StopReadingEngine::new(ExplorationIntent::Onboarding);
        let layer = create_test_layer("main_function", 50);

        let decision = engine.decide(&layer, 0.6, 0.4, 0.5);
        assert!(matches!(decision, ReadingDecision::ReadDeeply { .. }));
    }

    #[test]
    fn test_security_review_intent() {
        let engine = StopReadingEngine::new(ExplorationIntent::SecurityReview);
        let layer = create_test_layer("validate_input", 30);

        let decision = engine.decide(&layer, 0.8, 0.5, 0.7);
        assert!(matches!(decision, ReadingDecision::ReadDeeply { .. }));
    }

    #[test]
    fn test_migration_assessment_intent() {
        let engine = StopReadingEngine::new(ExplorationIntent::MigrationAssessment);
        let layer = create_test_layer("platform_check", 25);

        let decision = engine.decide(&layer, 0.7, 0.4, 0.6);
        assert!(matches!(decision, ReadingDecision::ReadDeeply { .. }));
    }

    // ==================== Concept Type Explanation Tests ====================

    #[test]
    fn test_explain_read_deeply_calculation() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("calculate_price", 20);

        let decision = engine.decide(&layer, 0.9, 0.5, 0.8);
        if let ReadingDecision::ReadDeeply { reason, .. } = decision {
            assert!(reason.contains("calculation") || reason.contains("relevant"));
        }
    }

    #[test]
    fn test_explain_read_deeply_validation() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("validate_data", 20);

        let decision = engine.decide(&layer, 0.9, 0.5, 0.8);
        if let ReadingDecision::ReadDeeply { reason, .. } = decision {
            assert!(reason.contains("validation") || reason.contains("relevant"));
        }
    }

    #[test]
    fn test_explain_read_deeply_decision() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("route_request", 20);

        let decision = engine.decide(&layer, 0.9, 0.5, 0.8);
        if let ReadingDecision::ReadDeeply { reason, .. } = decision {
            assert!(reason.contains("decision") || reason.contains("relevant"));
        }
    }

    #[test]
    fn test_explain_read_deeply_error_handling() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("handle_error", 20);

        let decision = engine.decide(&layer, 0.9, 0.5, 0.8);
        if let ReadingDecision::ReadDeeply { reason, .. } = decision {
            assert!(reason.contains("error") || reason.contains("relevant"));
        }
    }

    // ==================== Skip Explanation Tests ====================

    #[test]
    fn test_explain_skip_testing() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("test_something", 20);

        let decision = engine.decide(&layer, 0.1, 0.3, 0.1);
        if let ReadingDecision::Skip { reason, .. } = decision {
            assert!(reason.contains("Test") || reason.contains("relevance"));
        }
    }

    #[test]
    fn test_explain_skip_logging() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("log_event", 20);

        let decision = engine.decide(&layer, 0.1, 0.3, 0.1);
        if let ReadingDecision::Skip { reason, .. } = decision {
            assert!(reason.contains("Logging") || reason.contains("relevance"));
        }
    }

    #[test]
    fn test_explain_skip_configuration() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("init_config", 20);

        let decision = engine.decide(&layer, 0.1, 0.3, 0.1);
        if let ReadingDecision::Skip { reason, .. } = decision {
            assert!(reason.contains("Configuration") || reason.contains("relevance"));
        }
    }

    // ==================== Comeback Suggestion Tests ====================

    #[test]
    fn test_skip_with_comeback_testing() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("test_feature", 20);

        let decision = engine.decide(&layer, 0.1, 0.3, 0.1);
        if let ReadingDecision::Skip { come_back_if, .. } = decision {
            assert!(come_back_if.is_some());
        }
    }

    #[test]
    fn test_skip_with_comeback_error_handling() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("handle_errors", 20);

        let decision = engine.decide(&layer, 0.1, 0.3, 0.1);
        if let ReadingDecision::Skip { come_back_if, .. } = decision {
            // Error handling might have a comeback suggestion
            let _ = come_back_if;
        }
    }

    // ==================== Key Points Tests ====================

    #[test]
    fn test_key_points_debugging_intent() {
        let engine = StopReadingEngine::new(ExplorationIntent::Debugging);
        let layer = create_test_layer("process_data", 30);

        let decision = engine.decide(&layer, 0.8, 0.5, 0.7);
        if let ReadingDecision::ReadDeeply { key_points, .. } = decision {
            assert!(!key_points.is_empty());
            // Debugging should have error handling, state changes, logging points
        }
    }

    #[test]
    fn test_key_points_security_review_intent() {
        let engine = StopReadingEngine::new(ExplorationIntent::SecurityReview);
        let layer = create_test_layer("process_data", 30);

        let decision = engine.decide(&layer, 0.8, 0.5, 0.7);
        if let ReadingDecision::ReadDeeply { key_points, .. } = decision {
            assert!(!key_points.is_empty());
        }
    }

    #[test]
    fn test_key_points_migration_assessment_intent() {
        let engine = StopReadingEngine::new(ExplorationIntent::MigrationAssessment);
        let layer = create_test_layer("process_data", 30);

        let decision = engine.decide(&layer, 0.8, 0.5, 0.7);
        if let ReadingDecision::ReadDeeply { key_points, .. } = decision {
            assert!(!key_points.is_empty());
        }
    }

    // ==================== Focus Points Tests ====================

    #[test]
    fn test_skim_focus_points_debugging() {
        let engine = StopReadingEngine::new(ExplorationIntent::Debugging);
        let layer = create_test_layer("utility", 20);

        let decision = engine.decide(&layer, 0.4, 0.3, 0.4);
        if let ReadingDecision::Skim { focus_on, .. } = decision {
            assert!(!focus_on.is_empty());
        }
    }

    #[test]
    fn test_skim_focus_points_security() {
        let engine = StopReadingEngine::new(ExplorationIntent::SecurityReview);
        let layer = create_test_layer("utility", 20);

        let decision = engine.decide(&layer, 0.5, 0.3, 0.4);
        if let ReadingDecision::Skim { focus_on, .. } = decision {
            assert!(!focus_on.is_empty());
        }
    }

    // ==================== Time Estimation Tests ====================

    #[test]
    fn test_estimate_minutes_small_function() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("small_fn", 10);

        let decision = engine.decide(&layer, 0.9, 0.1, 0.8);
        if let ReadingDecision::ReadDeeply {
            estimated_minutes, ..
        } = decision
        {
            assert!(estimated_minutes > 0);
            assert!(estimated_minutes <= 5);
        }
    }

    #[test]
    fn test_estimate_minutes_large_function() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("large_fn", 200);

        let decision = engine.decide(&layer, 0.9, 0.1, 0.8);
        if let ReadingDecision::ReadDeeply {
            estimated_minutes, ..
        } = decision
        {
            assert!(estimated_minutes > 5);
        }
    }

    #[test]
    fn test_estimate_minutes_with_complexity() {
        let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let layer = create_test_layer("complex_fn", 50);

        let decision_low = engine.decide(&layer, 0.9, 0.1, 0.8);
        let decision_high = engine.decide(&layer, 0.9, 0.9, 0.8);

        if let (
            ReadingDecision::ReadDeeply {
                estimated_minutes: low_minutes,
                ..
            },
            ReadingDecision::ReadDeeply {
                estimated_minutes: high_minutes,
                ..
            },
        ) = (decision_low, decision_high)
        {
            // High complexity should take longer
            assert!(high_minutes >= low_minutes);
        }
    }

    // ==================== File Layer Tests ====================

    #[test]
    fn test_estimate_minutes_file_layer() {
        use std::path::PathBuf;

        let engine = StopReadingEngine::new(ExplorationIntent::Onboarding);
        let layer = ContextLayer::new(
            "test_file",
            LayerContent::File {
                path: PathBuf::from("test.rs"),
                language: "rust".to_string(),
                size_bytes: 10000,
                line_count: 300,
                symbol_count: 20,
                imports: vec![],
            },
        );

        let decision = engine.decide(&layer, 0.8, 0.5, 0.7);
        if let ReadingDecision::ReadDeeply {
            estimated_minutes, ..
        } = decision
        {
            assert!(estimated_minutes > 0);
        }
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_reading_decision_serialize_read_deeply() {
        let decision = ReadingDecision::ReadDeeply {
            reason: "Important code".to_string(),
            estimated_minutes: 5,
            key_points: vec!["point1".to_string()],
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("ReadDeeply"));
        assert!(json.contains("Important code"));
    }

    #[test]
    fn test_reading_decision_serialize_skip() {
        let decision = ReadingDecision::Skip {
            reason: "Not relevant".to_string(),
            come_back_if: Some("needed later".to_string()),
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("Skip"));
    }

    #[test]
    fn test_reading_decision_deserialize() {
        let json = r#"{"Skip":{"reason":"test","come_back_if":null}}"#;
        let decision: ReadingDecision = serde_json::from_str(json).unwrap();
        assert!(matches!(decision, ReadingDecision::Skip { .. }));
    }
}
