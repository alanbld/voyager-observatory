//! Reading Decisions - The "Stop Reading" Engine
//!
//! This module provides the decision-making logic for whether a developer
//! should read, skim, or skip a code element based on their exploration intent.

use serde::{Deserialize, Serialize};

use crate::core::fractal::ContextLayer;

use super::primitives::ConceptType;
use super::composition::ExplorationIntent;

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
        matches!(self, ReadingDecision::ReadDeeply { .. } | ReadingDecision::Skim { .. })
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
        if complexity > self.complexity_threshold && relevance > self.read_deeply_threshold && centrality < 0.3 {
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
                format!("Core {} logic ({:.0}% relevant to {})",
                    "calculation", relevance * 100.0, intent_name)
            }
            ConceptType::Validation => {
                format!("Key {} rules ({:.0}% relevant to {})",
                    "validation", relevance * 100.0, intent_name)
            }
            ConceptType::Decision => {
                format!("Important {} point ({:.0}% relevant to {})",
                    "decision", relevance * 100.0, intent_name)
            }
            ConceptType::ErrorHandling => {
                format!("Critical {} path ({:.0}% relevant to {})",
                    "error handling", relevance * 100.0, intent_name)
            }
            _ => {
                format!("High relevance to {} ({:.0}%)", intent_name, relevance * 100.0)
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
                format!("Low relevance to {} ({:.0}%)", intent_name, relevance * 100.0)
            }
        }
    }

    /// Suggest when to come back to skipped element
    fn suggest_comeback_reason(&self, layer: &ContextLayer, _relevance: f32) -> Option<String> {
        let concept_type = ConceptType::infer(layer);

        match concept_type {
            ConceptType::Testing => {
                Some("you need to verify expected behavior".to_string())
            }
            ConceptType::Configuration => {
                Some("you encounter configuration-related issues".to_string())
            }
            ConceptType::Logging => {
                Some("you need to understand logging behavior".to_string())
            }
            ConceptType::ErrorHandling => {
                Some("you encounter related errors".to_string())
            }
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
        if let crate::core::fractal::LayerContent::Symbol { documentation: Some(_), .. } = &layer.content {
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
    use crate::core::fractal::{LayerContent, SymbolKind, Visibility, Range};

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
        assert_eq!(ReadingDecision::ReadDeeply {
            reason: "".to_string(),
            estimated_minutes: 0,
            key_points: vec![],
        }.label(), "READ");

        assert_eq!(ReadingDecision::Skim {
            focus_on: vec![],
            time_limit_seconds: 0,
        }.label(), "SKIM");

        assert_eq!(ReadingDecision::Skip {
            reason: "".to_string(),
            come_back_if: None,
        }.label(), "SKIP");
    }

    #[test]
    fn test_different_intents_produce_different_thresholds() {
        let business = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
        let debugging = StopReadingEngine::new(ExplorationIntent::Debugging);

        // Business logic is more selective
        assert!(business.skip_threshold >= debugging.skip_threshold);
    }
}
