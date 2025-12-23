//! Intent Composition - Combining Primitives into Intents
//!
//! An intent is a composition of cognitive primitives with specific parameters.
//! This allows flexible, parameterized exploration without hardcoding behaviors.

use serde::{Deserialize, Serialize};

use crate::core::fractal::{ContextLayer, FeatureVector};

use super::primitives::{
    NoiseFilter, NoiseFilterParams,
    RelevanceScorer, RelevanceScorerParams,
    ExplorationPlanner, ExplorationPlannerParams,
    CognitivePrimitive, ConceptType, ScoredElement, PlannedStep,
};

// =============================================================================
// Exploration Intent
// =============================================================================

/// High-level exploration intent - what the user wants to accomplish
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExplorationIntent {
    /// Understand the business rules and core logic
    BusinessLogic,
    /// Debug an issue, understand error flow
    Debugging,
    /// Onboard to a new codebase
    Onboarding,
    /// Review for security vulnerabilities
    SecurityReview,
    /// Assess migration effort
    MigrationAssessment,
}

impl ExplorationIntent {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            ExplorationIntent::BusinessLogic => "Business Logic",
            ExplorationIntent::Debugging => "Debugging",
            ExplorationIntent::Onboarding => "Onboarding",
            ExplorationIntent::SecurityReview => "Security Review",
            ExplorationIntent::MigrationAssessment => "Migration Assessment",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            ExplorationIntent::BusinessLogic =>
                "Focus on calculations, validations, and decision logic",
            ExplorationIntent::Debugging =>
                "Focus on error handling, logging, and state changes",
            ExplorationIntent::Onboarding =>
                "Focus on architecture, entry points, and documentation",
            ExplorationIntent::SecurityReview =>
                "Focus on validation, authentication, and data handling",
            ExplorationIntent::MigrationAssessment =>
                "Focus on external dependencies and platform-specific code",
        }
    }
}

impl std::str::FromStr for ExplorationIntent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "business-logic" | "business_logic" | "businesslogic" => Ok(ExplorationIntent::BusinessLogic),
            "debugging" | "debug" => Ok(ExplorationIntent::Debugging),
            "onboarding" | "onboard" | "learn" => Ok(ExplorationIntent::Onboarding),
            "security" | "security-review" | "security_review" => Ok(ExplorationIntent::SecurityReview),
            "migration" | "migration-assessment" | "migrate" => Ok(ExplorationIntent::MigrationAssessment),
            _ => Err(format!("Unknown intent: '{}'. Valid intents: business-logic, debugging, onboarding, security, migration", s)),
        }
    }
}

// =============================================================================
// Configured Primitive
// =============================================================================

/// A primitive with its configuration
#[derive(Debug, Clone)]
pub struct ConfiguredPrimitive {
    pub name: String,
    pub weight: f32,
}

// =============================================================================
// Exploration Step (Output)
// =============================================================================

/// A step in the exploration path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationStep {
    /// Path to the element (file or ID)
    pub path: String,
    /// Symbol name (if applicable)
    pub symbol: String,
    /// Reading decision: "read", "skim", or "skip"
    pub decision: String,
    /// Reason for this decision
    pub reason: String,
    /// Estimated reading time in minutes
    pub estimated_minutes: u32,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f32,
    /// Concept type
    pub concept_type: String,
}

impl From<PlannedStep> for ExplorationStep {
    fn from(step: PlannedStep) -> Self {
        ExplorationStep {
            path: step.path,
            symbol: step.symbol,
            decision: step.decision,
            reason: step.reason,
            estimated_minutes: step.estimated_minutes,
            relevance_score: step.relevance_score,
            concept_type: "unknown".to_string(), // Will be set during conversion
        }
    }
}

// =============================================================================
// Intent Result
// =============================================================================

/// Result of executing an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// The intent that was executed
    pub intent: ExplorationIntent,
    /// Summary description
    pub summary: String,
    /// Total elements analyzed
    pub total_count: usize,
    /// Relevant elements after filtering
    pub relevant_count: usize,
    /// Estimated total reading time
    pub estimated_minutes: u32,
    /// Exploration path (ordered list of steps)
    pub exploration_path: Vec<ExplorationStep>,
    /// Key insights extracted
    pub key_insights: Vec<String>,
}

// =============================================================================
// Intent Composition
// =============================================================================

/// Composition of cognitive primitives that defines an exploration intent
pub struct IntentComposition {
    pub intent: ExplorationIntent,
    noise_filter: NoiseFilter,
    noise_params: NoiseFilterParams,
    relevance_scorer: RelevanceScorer,
    relevance_params: RelevanceScorerParams,
    exploration_planner: ExplorationPlanner,
    planner_params: ExplorationPlannerParams,
}

impl IntentComposition {
    /// Create a new intent composition with custom parameters
    pub fn new(
        intent: ExplorationIntent,
        noise_params: NoiseFilterParams,
        relevance_params: RelevanceScorerParams,
        planner_params: ExplorationPlannerParams,
    ) -> Self {
        Self {
            intent,
            noise_filter: NoiseFilter::default(),
            noise_params,
            relevance_scorer: RelevanceScorer::default(),
            relevance_params,
            exploration_planner: ExplorationPlanner::default(),
            planner_params,
        }
    }

    /// Preset for business logic exploration
    pub fn business_logic() -> Self {
        Self::new(
            ExplorationIntent::BusinessLogic,
            NoiseFilterParams::business_logic(),
            RelevanceScorerParams::business_logic(),
            ExplorationPlannerParams {
                max_elements: 25,
                time_budget_minutes: 30,
                min_relevance: 0.3,
                group_related: true,
            },
        )
    }

    /// Preset for debugging exploration
    pub fn debugging() -> Self {
        Self::new(
            ExplorationIntent::Debugging,
            NoiseFilterParams::debugging(),
            RelevanceScorerParams::debugging(),
            ExplorationPlannerParams {
                max_elements: 30,
                time_budget_minutes: 45,
                min_relevance: 0.2,
                group_related: true,
            },
        )
    }

    /// Preset for onboarding exploration
    pub fn onboarding() -> Self {
        Self::new(
            ExplorationIntent::Onboarding,
            NoiseFilterParams {
                filter_name_patterns: vec!["mock_".to_string()],
                filter_concept_types: vec![],
                min_relevance_threshold: 0.1,
            },
            RelevanceScorerParams::onboarding(),
            ExplorationPlannerParams {
                max_elements: 20,
                time_budget_minutes: 60,
                min_relevance: 0.2,
                group_related: true,
            },
        )
    }

    /// Preset for security review
    pub fn security_review() -> Self {
        Self::new(
            ExplorationIntent::SecurityReview,
            NoiseFilterParams::security(),
            RelevanceScorerParams::security(),
            ExplorationPlannerParams {
                max_elements: 40,
                time_budget_minutes: 60,
                min_relevance: 0.3,
                group_related: false, // Review each independently
            },
        )
    }

    /// Preset for migration assessment
    pub fn migration_assessment() -> Self {
        Self::new(
            ExplorationIntent::MigrationAssessment,
            NoiseFilterParams {
                filter_name_patterns: vec!["test_".to_string()],
                filter_concept_types: vec![ConceptType::Testing],
                min_relevance_threshold: 0.2,
            },
            RelevanceScorerParams {
                concept_weights: vec![
                    (ConceptType::Infrastructure, 1.0),
                    (ConceptType::Configuration, 0.9),
                    (ConceptType::ErrorHandling, 0.7),
                    (ConceptType::Transformation, 0.6),
                    (ConceptType::Validation, 0.5),
                    (ConceptType::Calculation, 0.4),
                    (ConceptType::Decision, 0.4),
                    (ConceptType::Logging, 0.3),
                    (ConceptType::Testing, 0.1),
                    (ConceptType::Unknown, 0.5),
                ],
                dimension_weights: None,
                documentation_boost: 0.1,
                public_visibility_boost: 0.1,
            },
            ExplorationPlannerParams {
                max_elements: 50,
                time_budget_minutes: 90,
                min_relevance: 0.2,
                group_related: true,
            },
        )
    }

    /// Create composition from intent enum
    pub fn from_intent(intent: ExplorationIntent) -> Self {
        match intent {
            ExplorationIntent::BusinessLogic => Self::business_logic(),
            ExplorationIntent::Debugging => Self::debugging(),
            ExplorationIntent::Onboarding => Self::onboarding(),
            ExplorationIntent::SecurityReview => Self::security_review(),
            ExplorationIntent::MigrationAssessment => Self::migration_assessment(),
        }
    }

    /// Execute the intent composition on a fractal context
    pub fn execute(&self, layers: &[ContextLayer], vectors: &[FeatureVector]) -> IntentResult {
        // Prepare input as tuples
        let elements: Vec<_> = layers.iter().zip(vectors.iter()).collect();
        let total_count = elements.len();

        // Step 1: Apply noise filter
        let filtered = self.noise_filter.apply(&elements, &self.noise_params);

        // Step 2: Score remaining elements
        let scored = self.score_filtered(&elements, &filtered);
        let relevant_count = scored.len();

        // Step 3: Plan exploration
        let planned = self.exploration_planner.apply(&scored, &self.planner_params);

        // Convert to output format
        let exploration_path: Vec<ExplorationStep> = planned
            .into_iter()
            .enumerate()
            .map(|(i, step)| {
                let concept_type = if i < scored.len() {
                    format!("{:?}", scored[step.element_idx].concept_type)
                } else {
                    "Unknown".to_string()
                };

                ExplorationStep {
                    path: step.path,
                    symbol: step.symbol,
                    decision: step.decision,
                    reason: step.reason,
                    estimated_minutes: step.estimated_minutes,
                    relevance_score: step.relevance_score,
                    concept_type,
                }
            })
            .collect();

        // Calculate total estimated time
        let estimated_minutes: u32 = exploration_path
            .iter()
            .filter(|s| s.decision == "read" || s.decision == "skim")
            .map(|s| s.estimated_minutes)
            .sum();

        // Generate key insights
        let key_insights = self.generate_insights(&scored, &exploration_path);

        // Generate summary
        let summary = format!(
            "Found {} relevant elements (filtered {} of {} total). Estimated reading time: {} minutes.",
            relevant_count,
            total_count - relevant_count,
            total_count,
            estimated_minutes
        );

        IntentResult {
            intent: self.intent,
            summary,
            total_count,
            relevant_count,
            estimated_minutes,
            exploration_path,
            key_insights,
        }
    }

    /// Score filtered elements (internal helper)
    fn score_filtered<'a>(
        &self,
        elements: &'a [(&'a ContextLayer, &'a FeatureVector)],
        filtered: &[(usize, ConceptType)],
    ) -> Vec<ScoredElement<'a>> {
        filtered
            .iter()
            .map(|(idx, concept_type)| {
                let (layer, vector) = elements[*idx];
                let relevance = self.relevance_scorer.score_element(
                    layer,
                    vector,
                    *concept_type,
                    &self.relevance_params,
                );

                ScoredElement {
                    layer,
                    vector,
                    score: relevance.score,
                    concept_type: *concept_type,
                    relevance,
                }
            })
            .collect()
    }

    /// Generate key insights from scored elements
    fn generate_insights(&self, scored: &[ScoredElement], path: &[ExplorationStep]) -> Vec<String> {
        let mut insights = Vec::new();

        // Insight 1: Concept type distribution
        let mut type_counts: std::collections::HashMap<ConceptType, usize> = std::collections::HashMap::new();
        for elem in scored {
            *type_counts.entry(elem.concept_type).or_insert(0) += 1;
        }

        let top_type = type_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(t, _)| *t);

        if let Some(t) = top_type {
            insights.push(format!("Dominant concept type: {:?}", t));
        }

        // Insight 2: High-relevance elements
        let high_relevance: Vec<_> = path.iter()
            .filter(|s| s.relevance_score > 0.8)
            .collect();

        if !high_relevance.is_empty() {
            insights.push(format!(
                "Found {} highly relevant elements (>80% score)",
                high_relevance.len()
            ));
        }

        // Insight 3: Recommended starting point
        if let Some(first) = path.first() {
            if first.decision == "read" {
                insights.push(format!(
                    "Recommended starting point: {} ({})",
                    first.symbol,
                    first.concept_type
                ));
            }
        }

        // Insight 4: Time distribution
        let read_time: u32 = path.iter()
            .filter(|s| s.decision == "read")
            .map(|s| s.estimated_minutes)
            .sum();
        let skim_time: u32 = path.iter()
            .filter(|s| s.decision == "skim")
            .map(|s| s.estimated_minutes)
            .sum();

        if read_time > 0 || skim_time > 0 {
            insights.push(format!(
                "Time split: {} min deep reading, {} min skimming",
                read_time, skim_time
            ));
        }

        insights
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_from_string() {
        assert_eq!(
            "business-logic".parse::<ExplorationIntent>().unwrap(),
            ExplorationIntent::BusinessLogic
        );
        assert_eq!(
            "debugging".parse::<ExplorationIntent>().unwrap(),
            ExplorationIntent::Debugging
        );
        assert_eq!(
            "security".parse::<ExplorationIntent>().unwrap(),
            ExplorationIntent::SecurityReview
        );

        assert!("invalid".parse::<ExplorationIntent>().is_err());
    }

    #[test]
    fn test_intent_descriptions() {
        let intent = ExplorationIntent::BusinessLogic;
        assert!(!intent.name().is_empty());
        assert!(!intent.description().is_empty());
    }

    #[test]
    fn test_composition_from_intent() {
        let comp = IntentComposition::from_intent(ExplorationIntent::BusinessLogic);
        assert_eq!(comp.intent, ExplorationIntent::BusinessLogic);

        let comp = IntentComposition::from_intent(ExplorationIntent::Debugging);
        assert_eq!(comp.intent, ExplorationIntent::Debugging);
    }

    #[test]
    fn test_business_logic_preset_configuration() {
        let comp = IntentComposition::business_logic();

        // Should filter tests
        assert!(comp.noise_params.filter_name_patterns.contains(&"test_".to_string()));

        // Should weight calculations high
        let calc_weight = comp.relevance_params.concept_weights.iter()
            .find(|(ct, _)| *ct == ConceptType::Calculation)
            .map(|(_, w)| *w);
        assert!(calc_weight.unwrap_or(0.0) > 0.8);
    }

    #[test]
    fn test_debugging_preset_keeps_logging() {
        let comp = IntentComposition::debugging();

        // Should NOT filter logging for debugging
        assert!(!comp.noise_params.filter_concept_types.contains(&ConceptType::Logging));

        // Should weight error handling high
        let error_weight = comp.relevance_params.concept_weights.iter()
            .find(|(ct, _)| *ct == ConceptType::ErrorHandling)
            .map(|(_, w)| *w);
        assert!(error_weight.unwrap_or(0.0) > 0.8);
    }
}
