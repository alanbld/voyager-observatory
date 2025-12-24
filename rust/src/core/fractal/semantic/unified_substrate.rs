//! Unified Semantic Substrate
//!
//! The core data structure for multi-language semantic analysis.
//! Provides a language-agnostic representation of concepts that enables
//! cross-language exploration and comparison.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{Language, UserContext};
use crate::core::fractal::{ConceptType, ContextLayer, FeatureVector, Visibility};

// =============================================================================
// Core Types
// =============================================================================

/// Unique identifier for a concept in the unified substrate
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConceptId(String);

impl ConceptId {
    pub fn new(language: Language, name: &str, file: &str) -> Self {
        Self(format!("{}:{}:{}", language, file, name))
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract language from concept ID
    pub fn language(&self) -> Option<Language> {
        self.0.split(':').next().and_then(|s| s.parse().ok())
    }

    /// Extract file path from concept ID
    pub fn file_path(&self) -> Option<&str> {
        let parts: Vec<_> = self.0.splitn(3, ':').collect();
        parts.get(1).copied()
    }

    /// Extract symbol name from concept ID
    pub fn symbol_name(&self) -> Option<&str> {
        let parts: Vec<_> = self.0.splitn(3, ':').collect();
        parts.get(2).copied()
    }
}

impl std::fmt::Display for ConceptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Universal concept types (language-agnostic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UniversalConceptType {
    /// Mathematical or business calculations
    Calculation,
    /// Data validation and constraints
    Validation,
    /// Data transformation (mapping, filtering, reducing)
    Transformation,
    /// Branching and control flow decisions
    Decision,
    /// Data structures (classes, interfaces, records)
    DataStructure,
    /// Service or business logic components
    Service,
    /// API endpoints or route handlers
    Endpoint,
    /// Database operations (CRUD)
    DatabaseOperation,
    /// External system integration
    Integration,
    /// Error handling and recovery
    ErrorHandling,
    /// Infrastructure and utilities
    Infrastructure,
    /// Configuration management
    Configuration,
    /// Logging and monitoring
    Observability,
    /// Testing code
    Testing,
    /// Unknown or unclassified
    Unknown,
}

impl UniversalConceptType {
    /// Map from language-specific ConceptType to universal type
    pub fn from_concept_type(ct: ConceptType) -> Self {
        match ct {
            ConceptType::Calculation => UniversalConceptType::Calculation,
            ConceptType::Validation => UniversalConceptType::Validation,
            ConceptType::Transformation => UniversalConceptType::Transformation,
            ConceptType::Decision => UniversalConceptType::Decision,
            ConceptType::ErrorHandling => UniversalConceptType::ErrorHandling,
            ConceptType::Infrastructure => UniversalConceptType::Infrastructure,
            ConceptType::Configuration => UniversalConceptType::Configuration,
            ConceptType::Logging => UniversalConceptType::Observability,
            ConceptType::Testing => UniversalConceptType::Testing,
            ConceptType::Unknown => UniversalConceptType::Unknown,
        }
    }

    /// Get semantic similarity to another type
    pub fn similarity_to(&self, other: &Self) -> f32 {
        if self == other {
            return 1.0;
        }

        // Define semantic relationships between types
        match (self, other) {
            // Closely related
            (UniversalConceptType::Calculation, UniversalConceptType::Transformation)
            | (UniversalConceptType::Transformation, UniversalConceptType::Calculation) => 0.6,

            (UniversalConceptType::Validation, UniversalConceptType::ErrorHandling)
            | (UniversalConceptType::ErrorHandling, UniversalConceptType::Validation) => 0.5,

            (UniversalConceptType::Service, UniversalConceptType::Endpoint)
            | (UniversalConceptType::Endpoint, UniversalConceptType::Service) => 0.7,

            (UniversalConceptType::DataStructure, UniversalConceptType::DatabaseOperation)
            | (UniversalConceptType::DatabaseOperation, UniversalConceptType::DataStructure) => 0.5,

            (UniversalConceptType::Configuration, UniversalConceptType::Infrastructure)
            | (UniversalConceptType::Infrastructure, UniversalConceptType::Configuration) => 0.6,

            (UniversalConceptType::Observability, UniversalConceptType::ErrorHandling)
            | (UniversalConceptType::ErrorHandling, UniversalConceptType::Observability) => 0.4,

            // Testing is moderately related to validation
            (UniversalConceptType::Testing, UniversalConceptType::Validation)
            | (UniversalConceptType::Validation, UniversalConceptType::Testing) => 0.3,

            // Unknown has low similarity to everything
            (UniversalConceptType::Unknown, _) | (_, UniversalConceptType::Unknown) => 0.1,

            // Default: low similarity
            _ => 0.2,
        }
    }
}

// =============================================================================
// Language-Specific Data
// =============================================================================

/// Language-specific metadata preserved from original analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSpecificData {
    /// Source language
    pub language: Language,
    /// Original concept type from language plugin
    pub original_type: ConceptType,
    /// Language-specific properties (e.g., decorators, visibility)
    pub properties: HashMap<String, String>,
    /// Original file path
    pub file_path: String,
    /// Line range in source file
    pub line_range: (usize, usize),
}

/// Unified properties extracted from all languages
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedProperties {
    /// Human-readable documentation
    pub documentation: Option<String>,
    /// Visibility (public/private)
    pub visibility: Visibility,
    /// Complexity score (0.0 - 1.0)
    pub complexity_score: f32,
    /// Whether it has tests
    pub has_tests: bool,
    /// Whether it's async/concurrent
    pub is_async: bool,
    /// Whether it's deprecated
    pub is_deprecated: bool,
    /// Dependencies (other concept IDs)
    pub dependencies: Vec<ConceptId>,
    /// Dependents (concepts that depend on this)
    pub dependents: Vec<ConceptId>,
}

// =============================================================================
// Unified Concept
// =============================================================================

/// A concept in the unified semantic space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedConcept {
    /// Unique identifier
    pub id: ConceptId,
    /// Symbol name
    pub name: String,
    /// Universal (language-agnostic) concept type
    pub universal_type: UniversalConceptType,
    /// Language-specific data
    pub language_specific: LanguageSpecificData,
    /// Unified properties
    pub properties: UnifiedProperties,
    /// Normalized 64D feature vector (stored as Vec for serde compatibility)
    #[serde(with = "embedding_serde")]
    pub embedding: [f32; 64],
}

/// Custom serde for [f32; 64] arrays
mod embedding_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(arr: &[f32; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        arr.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[f32; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<f32>::deserialize(deserializer)?;
        if vec.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "expected 64 elements, got {}",
                vec.len()
            )));
        }
        let mut arr = [0.0f32; 64];
        arr.copy_from_slice(&vec);
        Ok(arr)
    }
}

impl UnifiedConcept {
    /// Get the source language
    pub fn language(&self) -> Language {
        self.language_specific.language
    }

    /// Get complexity score
    pub fn complexity_score(&self) -> f32 {
        self.properties.complexity_score
    }

    /// Calculate embedding similarity to another concept
    pub fn embedding_similarity(&self, other: &Self) -> f32 {
        let dot_product: f32 = self
            .embedding
            .iter()
            .zip(other.embedding.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            dot_product / (norm_a * norm_b)
        } else {
            0.0
        }
    }
}

// =============================================================================
// Language Contribution
// =============================================================================

/// Track what a language contributes to the substrate
#[derive(Debug, Clone, Default)]
pub struct LanguageContribution {
    /// Concept IDs from this language
    pub concepts: Vec<ConceptId>,
    /// Total files analyzed
    pub file_count: usize,
    /// Total lines of code
    pub line_count: usize,
    /// Concept type distribution
    pub type_distribution: HashMap<UniversalConceptType, usize>,
}

impl LanguageContribution {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_concept(&mut self, id: ConceptId, concept_type: UniversalConceptType) {
        self.concepts.push(id);
        *self.type_distribution.entry(concept_type).or_insert(0) += 1;
    }
}

// =============================================================================
// Unified Semantic Substrate
// =============================================================================

/// The unified semantic substrate for multi-language analysis
#[derive(Debug, Clone)]
pub struct UnifiedSemanticSubstrate {
    /// All unified concepts
    concepts: HashMap<ConceptId, UnifiedConcept>,
    /// Language contributions
    language_contributions: HashMap<Language, LanguageContribution>,
    /// Cross-language equivalences (concept_id -> equivalent concept_ids)
    equivalences: HashMap<ConceptId, Vec<ConceptId>>,
}

impl UnifiedSemanticSubstrate {
    /// Create a new empty substrate
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
            language_contributions: HashMap::new(),
            equivalences: HashMap::new(),
        }
    }

    /// Add a concept to the substrate
    pub fn add_concept(&mut self, concept: UnifiedConcept) {
        let language = concept.language();
        let concept_type = concept.universal_type;
        let id = concept.id.clone();

        // Update language contribution
        self.language_contributions
            .entry(language)
            .or_insert_with(LanguageContribution::new)
            .add_concept(id.clone(), concept_type);

        self.concepts.insert(id, concept);
    }

    /// Get a concept by ID
    pub fn get_concept(&self, id: &ConceptId) -> Option<&UnifiedConcept> {
        self.concepts.get(id)
    }

    /// Get all concepts
    pub fn concepts(&self) -> impl Iterator<Item = &UnifiedConcept> {
        self.concepts.values()
    }

    /// Get concepts for a specific language
    pub fn concepts_for_language(&self, language: Language) -> Vec<&UnifiedConcept> {
        self.concepts
            .values()
            .filter(|c| c.language() == language)
            .collect()
    }

    /// Get concepts of a specific universal type
    pub fn concepts_of_type(&self, concept_type: UniversalConceptType) -> Vec<&UnifiedConcept> {
        self.concepts
            .values()
            .filter(|c| c.universal_type == concept_type)
            .collect()
    }

    /// Get language contributions
    pub fn language_contributions(&self) -> &HashMap<Language, LanguageContribution> {
        &self.language_contributions
    }

    /// Get languages present in the substrate
    pub fn languages(&self) -> Vec<Language> {
        self.language_contributions.keys().copied().collect()
    }

    /// Total number of concepts
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Register cross-language equivalence
    pub fn register_equivalence(&mut self, concept_a: &ConceptId, concept_b: &ConceptId) {
        self.equivalences
            .entry(concept_a.clone())
            .or_insert_with(Vec::new)
            .push(concept_b.clone());

        self.equivalences
            .entry(concept_b.clone())
            .or_insert_with(Vec::new)
            .push(concept_a.clone());
    }

    /// Get equivalents for a concept
    pub fn find_equivalents(&self, id: &ConceptId) -> Vec<&UnifiedConcept> {
        self.equivalences
            .get(id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|eq_id| self.concepts.get(eq_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find concepts by name pattern
    pub fn find_by_name(&self, pattern: &str) -> Vec<&UnifiedConcept> {
        let pattern_lower = pattern.to_lowercase();
        self.concepts
            .values()
            .filter(|c| c.name.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    /// Get concept distribution across languages
    pub fn get_language_breakdown(&self) -> HashMap<Language, usize> {
        self.language_contributions
            .iter()
            .map(|(lang, contrib)| (*lang, contrib.concepts.len()))
            .collect()
    }

    /// Get universal type distribution
    pub fn get_type_distribution(&self) -> HashMap<UniversalConceptType, usize> {
        let mut dist = HashMap::new();
        for concept in self.concepts.values() {
            *dist.entry(concept.universal_type).or_insert(0) += 1;
        }
        dist
    }

    /// Filter concepts based on user context
    pub fn filter_by_context(&self, context: &UserContext) -> Vec<&UnifiedConcept> {
        self.concepts
            .values()
            .filter(|c| !context.should_ignore(c.language()))
            .collect()
    }

    /// Score concepts for a given intent
    pub fn score_for_intent(
        &self,
        intent: &str,
        context: &UserContext,
    ) -> Vec<(&UnifiedConcept, f32)> {
        let relevant_types = match intent.to_lowercase().as_str() {
            "business-logic" | "business_logic" => vec![
                (UniversalConceptType::Calculation, 1.0),
                (UniversalConceptType::Validation, 0.9),
                (UniversalConceptType::Decision, 0.8),
                (UniversalConceptType::Transformation, 0.7),
                (UniversalConceptType::Service, 0.6),
            ],
            "debugging" | "debug" => vec![
                (UniversalConceptType::ErrorHandling, 1.0),
                (UniversalConceptType::Observability, 0.9),
                (UniversalConceptType::Validation, 0.7),
                (UniversalConceptType::Decision, 0.6),
                (UniversalConceptType::Integration, 0.5),
            ],
            "security" | "security-review" => vec![
                (UniversalConceptType::Validation, 1.0),
                (UniversalConceptType::ErrorHandling, 0.9),
                (UniversalConceptType::Endpoint, 0.8),
                (UniversalConceptType::DatabaseOperation, 0.7),
                (UniversalConceptType::Configuration, 0.6),
            ],
            "onboarding" => vec![
                (UniversalConceptType::Service, 1.0),
                (UniversalConceptType::Endpoint, 0.9),
                (UniversalConceptType::DataStructure, 0.8),
                (UniversalConceptType::Configuration, 0.7),
                (UniversalConceptType::Infrastructure, 0.6),
            ],
            "migration" => vec![
                (UniversalConceptType::Infrastructure, 1.0),
                (UniversalConceptType::Configuration, 0.9),
                (UniversalConceptType::DatabaseOperation, 0.8),
                (UniversalConceptType::Integration, 0.7),
                (UniversalConceptType::ErrorHandling, 0.5),
            ],
            _ => vec![
                (UniversalConceptType::Service, 0.5),
                (UniversalConceptType::DataStructure, 0.5),
            ],
        };

        let type_weights: HashMap<_, _> = relevant_types.into_iter().collect();

        self.concepts
            .values()
            .filter(|c| !context.should_ignore(c.language()))
            .map(|c| {
                let type_score = type_weights
                    .get(&c.universal_type)
                    .copied()
                    .unwrap_or(0.1);
                let familiarity = context.get_familiarity(c.language());
                let score = type_score * (0.5 + 0.5 * familiarity);
                (c, score)
            })
            .collect()
    }

    /// Build substrate from context layers and feature vectors
    pub fn from_layers(
        layers: &[ContextLayer],
        vectors: &[FeatureVector],
        language: Language,
        file_path: &str,
    ) -> Self {
        let mut substrate = Self::new();

        for (layer, vector) in layers.iter().zip(vectors.iter()) {
            if let crate::core::fractal::LayerContent::Symbol {
                name,
                kind,
                signature,
                documentation,
                visibility,
                range,
                ..
            } = &layer.content
            {
                let concept_type = ConceptType::infer(layer);
                let universal_type = UniversalConceptType::from_concept_type(concept_type);

                // Convert Vec<f32> to [f32; 64]
                let mut embedding = [0.0f32; 64];
                for (i, v) in vector.values.iter().take(64).enumerate() {
                    embedding[i] = *v;
                }

                let concept = UnifiedConcept {
                    id: ConceptId::new(language, name, file_path),
                    name: name.clone(),
                    universal_type,
                    language_specific: LanguageSpecificData {
                        language,
                        original_type: concept_type,
                        properties: HashMap::from([
                            ("kind".to_string(), format!("{:?}", kind)),
                            ("signature".to_string(), signature.clone()),
                        ]),
                        file_path: file_path.to_string(),
                        line_range: (range.start_line, range.end_line),
                    },
                    properties: UnifiedProperties {
                        documentation: documentation.clone(),
                        visibility: *visibility,
                        complexity_score: 0.0, // Computed separately if needed
                        has_tests: false,
                        is_async: signature.contains("async"),
                        is_deprecated: signature.contains("@deprecated")
                            || signature.contains("# deprecated"),
                        dependencies: Vec::new(),
                        dependents: Vec::new(),
                    },
                    embedding,
                };

                substrate.add_concept(concept);
            }
        }

        substrate
    }

    /// Merge another substrate into this one
    pub fn merge(&mut self, other: UnifiedSemanticSubstrate) {
        for (id, concept) in other.concepts {
            self.add_concept(concept);
        }

        for (concept_a, equivalents) in other.equivalences {
            for concept_b in equivalents {
                self.register_equivalence(&concept_a, &concept_b);
            }
        }
    }
}

impl Default for UnifiedSemanticSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_concept(
        name: &str,
        language: Language,
        concept_type: UniversalConceptType,
    ) -> UnifiedConcept {
        UnifiedConcept {
            id: ConceptId::new(language, name, "test.file"),
            name: name.to_string(),
            universal_type: concept_type,
            language_specific: LanguageSpecificData {
                language,
                original_type: ConceptType::Unknown,
                properties: HashMap::new(),
                file_path: "test.file".to_string(),
                line_range: (1, 10),
            },
            properties: UnifiedProperties::default(),
            embedding: [0.0; 64],
        }
    }

    #[test]
    fn test_concept_id() {
        let id = ConceptId::new(Language::Python, "calculate_total", "order.py");
        assert_eq!(id.language(), Some(Language::Python));
        assert_eq!(id.file_path(), Some("order.py"));
        assert_eq!(id.symbol_name(), Some("calculate_total"));
    }

    #[test]
    fn test_substrate_add_concept() {
        let mut substrate = UnifiedSemanticSubstrate::new();

        let concept = create_test_concept(
            "calculate_tax",
            Language::Python,
            UniversalConceptType::Calculation,
        );
        substrate.add_concept(concept);

        assert_eq!(substrate.concept_count(), 1);
        assert_eq!(substrate.languages(), vec![Language::Python]);
    }

    #[test]
    fn test_substrate_multi_language() {
        let mut substrate = UnifiedSemanticSubstrate::new();

        substrate.add_concept(create_test_concept(
            "calculate_tax",
            Language::Python,
            UniversalConceptType::Calculation,
        ));
        substrate.add_concept(create_test_concept(
            "calculateTax",
            Language::TypeScript,
            UniversalConceptType::Calculation,
        ));
        substrate.add_concept(create_test_concept(
            "calculate_tax",
            Language::ABL,
            UniversalConceptType::Calculation,
        ));

        assert_eq!(substrate.concept_count(), 3);
        assert_eq!(substrate.languages().len(), 3);
    }

    #[test]
    fn test_substrate_equivalence() {
        let mut substrate = UnifiedSemanticSubstrate::new();

        let py_concept = create_test_concept(
            "calculate_tax",
            Language::Python,
            UniversalConceptType::Calculation,
        );
        let ts_concept = create_test_concept(
            "calculateTax",
            Language::TypeScript,
            UniversalConceptType::Calculation,
        );

        let py_id = py_concept.id.clone();
        let ts_id = ts_concept.id.clone();

        substrate.add_concept(py_concept);
        substrate.add_concept(ts_concept);
        substrate.register_equivalence(&py_id, &ts_id);

        let equivalents = substrate.find_equivalents(&py_id);
        assert_eq!(equivalents.len(), 1);
        assert_eq!(equivalents[0].name, "calculateTax");
    }

    #[test]
    fn test_universal_type_similarity() {
        assert_eq!(
            UniversalConceptType::Calculation.similarity_to(&UniversalConceptType::Calculation),
            1.0
        );
        assert!(
            UniversalConceptType::Calculation.similarity_to(&UniversalConceptType::Transformation)
                > 0.5
        );
        assert!(
            UniversalConceptType::Calculation.similarity_to(&UniversalConceptType::Testing) < 0.3
        );
    }

    #[test]
    fn test_score_for_intent() {
        let mut substrate = UnifiedSemanticSubstrate::new();

        substrate.add_concept(create_test_concept(
            "validate_order",
            Language::Python,
            UniversalConceptType::Validation,
        ));
        substrate.add_concept(create_test_concept(
            "log_error",
            Language::Python,
            UniversalConceptType::Observability,
        ));

        let context = UserContext::new().with_familiarity(Language::Python, 0.9);

        let scores = substrate.score_for_intent("business-logic", &context);

        // Validation should score higher than Observability for business-logic
        let validation_score = scores
            .iter()
            .find(|(c, _)| c.name == "validate_order")
            .map(|(_, s)| *s)
            .unwrap();
        let logging_score = scores
            .iter()
            .find(|(c, _)| c.name == "log_error")
            .map(|(_, s)| *s)
            .unwrap();

        assert!(
            validation_score > logging_score,
            "Validation should score higher for business-logic intent"
        );
    }
}
