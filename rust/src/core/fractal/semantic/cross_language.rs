//! Cross-Language Concept Alignment
//!
//! This module provides algorithms for finding equivalent concepts across
//! different programming languages. It uses multiple similarity measures
//! (embedding, name, type) to identify cross-language equivalences.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{ConceptId, Language, UnifiedConcept, UnifiedSemanticSubstrate, UniversalConceptType};

// =============================================================================
// Cross-Language Equivalent
// =============================================================================

/// Represents an equivalence between two concepts from different languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLanguageEquivalent {
    /// First concept ID
    pub concept_a_id: ConceptId,
    /// Second concept ID
    pub concept_b_id: ConceptId,
    /// Overall similarity score (0.0 - 1.0)
    pub similarity: f32,
    /// Evidence explaining the match
    pub evidence: Vec<String>,
    /// Confidence level
    pub confidence: MatchConfidence,
}

/// Confidence level for a cross-language match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchConfidence {
    /// High confidence: multiple strong signals
    High,
    /// Medium confidence: good signals but some uncertainty
    Medium,
    /// Low confidence: weak signals, may be false positive
    Low,
}

impl MatchConfidence {
    pub fn from_similarity(sim: f32) -> Self {
        if sim > 0.85 {
            MatchConfidence::High
        } else if sim > 0.7 {
            MatchConfidence::Medium
        } else {
            MatchConfidence::Low
        }
    }
}

// =============================================================================
// Cross-Language Relationship
// =============================================================================

/// Types of relationships between concepts across languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Same concept, different language (e.g., Python validate_order == TS validateOrder)
    Equivalent,
    /// One calls/uses the other (e.g., TypeScript API client calls Python backend)
    Calls,
    /// One depends on the other (e.g., frontend depends on backend types)
    DependsOn,
    /// One extends/inherits from the other (conceptually)
    Extends,
    /// They share a common interface/contract
    SharedContract,
}

/// A relationship between concepts in different languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLanguageRelationship {
    /// Source concept
    pub source_id: ConceptId,
    /// Target concept
    pub target_id: ConceptId,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength of relationship (0.0 - 1.0)
    pub strength: f32,
    /// Description of the relationship
    pub description: String,
}

// =============================================================================
// Equivalence Class
// =============================================================================

/// A group of equivalent concepts across languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceClass {
    /// Canonical name for this equivalence class
    pub canonical_name: String,
    /// Member concept IDs
    pub members: Vec<ConceptId>,
    /// Languages represented
    pub languages: HashSet<Language>,
    /// Average internal similarity
    pub cohesion: f32,
}

impl EquivalenceClass {
    pub fn new(canonical_name: String) -> Self {
        Self {
            canonical_name,
            members: Vec::new(),
            languages: HashSet::new(),
            cohesion: 0.0,
        }
    }

    pub fn add_member(&mut self, id: ConceptId, language: Language) {
        self.members.push(id);
        self.languages.insert(language);
    }

    pub fn is_multi_language(&self) -> bool {
        self.languages.len() > 1
    }
}

// =============================================================================
// Cross-Language Aligner
// =============================================================================

/// Aligns concepts across languages using multiple similarity measures
#[derive(Debug, Clone)]
pub struct CrossLanguageAligner {
    /// Minimum overall similarity to consider a match
    pub similarity_threshold: f32,
    /// Weight for embedding similarity
    pub embedding_weight: f32,
    /// Weight for name similarity
    pub name_weight: f32,
    /// Weight for type similarity
    pub type_weight: f32,
    /// Minimum name similarity to boost embedding matches
    pub name_boost_threshold: f32,
}

impl Default for CrossLanguageAligner {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.65,
            embedding_weight: 0.5,
            name_weight: 0.3,
            type_weight: 0.2,
            name_boost_threshold: 0.7,
        }
    }
}

impl CrossLanguageAligner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure for strict matching (fewer false positives)
    pub fn strict() -> Self {
        Self {
            similarity_threshold: 0.8,
            embedding_weight: 0.4,
            name_weight: 0.4,
            type_weight: 0.2,
            name_boost_threshold: 0.8,
        }
    }

    /// Configure for lenient matching (find more potential matches)
    pub fn lenient() -> Self {
        Self {
            similarity_threshold: 0.5,
            embedding_weight: 0.6,
            name_weight: 0.25,
            type_weight: 0.15,
            name_boost_threshold: 0.5,
        }
    }

    /// Find cross-language equivalents in the substrate
    pub fn find_equivalents(
        &self,
        substrate: &UnifiedSemanticSubstrate,
    ) -> Vec<CrossLanguageEquivalent> {
        let mut equivalents = Vec::new();
        let concepts: Vec<_> = substrate.concepts().collect();

        // Compare all pairs from different languages
        for i in 0..concepts.len() {
            for j in (i + 1)..concepts.len() {
                let concept_a = concepts[i];
                let concept_b = concepts[j];

                // Only compare across different languages
                if concept_a.language() == concept_b.language() {
                    continue;
                }

                // Calculate similarities
                let embedding_sim = self.embedding_similarity(concept_a, concept_b);
                let name_sim = self.name_similarity(&concept_a.name, &concept_b.name);
                let type_sim = self.type_similarity(concept_a, concept_b);

                // Calculate weighted similarity
                let mut total_similarity = self.embedding_weight * embedding_sim
                    + self.name_weight * name_sim
                    + self.type_weight * type_sim;

                // Boost if names are very similar
                if name_sim > self.name_boost_threshold {
                    total_similarity = (total_similarity + 0.15).min(1.0);
                }

                if total_similarity >= self.similarity_threshold {
                    let evidence = vec![
                        format!("Embedding similarity: {:.2}", embedding_sim),
                        format!("Name similarity: {:.2}", name_sim),
                        format!("Type similarity: {:.2}", type_sim),
                    ];

                    equivalents.push(CrossLanguageEquivalent {
                        concept_a_id: concept_a.id.clone(),
                        concept_b_id: concept_b.id.clone(),
                        similarity: total_similarity,
                        evidence,
                        confidence: MatchConfidence::from_similarity(total_similarity),
                    });
                }
            }
        }

        // Sort by similarity (highest first)
        equivalents.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        equivalents
    }

    /// Cluster equivalents into equivalence classes
    pub fn cluster_equivalents(
        &self,
        equivalents: &[CrossLanguageEquivalent],
        substrate: &UnifiedSemanticSubstrate,
    ) -> Vec<EquivalenceClass> {
        // Use union-find to cluster
        let mut parent: HashMap<ConceptId, ConceptId> = HashMap::new();

        fn find(parent: &mut HashMap<ConceptId, ConceptId>, id: &ConceptId) -> ConceptId {
            if !parent.contains_key(id) {
                return id.clone();
            }
            let p = parent.get(id).unwrap().clone();
            if &p == id {
                return id.clone();
            }
            let root = find(parent, &p);
            parent.insert(id.clone(), root.clone());
            root
        }

        fn union(parent: &mut HashMap<ConceptId, ConceptId>, a: &ConceptId, b: &ConceptId) {
            let root_a = find(parent, a);
            let root_b = find(parent, b);
            if root_a != root_b {
                parent.insert(root_b, root_a);
            }
        }

        // Build clusters
        for eq in equivalents {
            parent
                .entry(eq.concept_a_id.clone())
                .or_insert(eq.concept_a_id.clone());
            parent
                .entry(eq.concept_b_id.clone())
                .or_insert(eq.concept_b_id.clone());
            union(&mut parent, &eq.concept_a_id, &eq.concept_b_id);
        }

        // Group by cluster root
        let mut clusters: HashMap<ConceptId, Vec<ConceptId>> = HashMap::new();
        let ids: Vec<_> = parent.keys().cloned().collect();
        for id in ids {
            let root = find(&mut parent, &id);
            clusters.entry(root).or_insert_with(Vec::new).push(id);
        }

        // Convert to equivalence classes
        clusters
            .into_iter()
            .filter(|(_, members)| members.len() > 1)
            .map(|(_, members)| {
                let mut class =
                    EquivalenceClass::new(self.derive_canonical_name(&members, substrate));
                for id in members {
                    if let Some(concept) = substrate.get_concept(&id) {
                        class.add_member(id, concept.language());
                    }
                }
                class.cohesion = self.calculate_cohesion(&class, substrate);
                class
            })
            .collect()
    }

    /// Calculate embedding similarity (cosine)
    fn embedding_similarity(&self, a: &UnifiedConcept, b: &UnifiedConcept) -> f32 {
        a.embedding_similarity(b)
    }

    /// Calculate name similarity using normalized Levenshtein distance
    fn name_similarity(&self, name_a: &str, name_b: &str) -> f32 {
        let normalized_a = self.normalize_name(name_a);
        let normalized_b = self.normalize_name(name_b);

        // Exact match after normalization
        if normalized_a == normalized_b {
            return 1.0;
        }

        // Calculate Levenshtein distance
        let distance = levenshtein_distance(&normalized_a, &normalized_b);
        let max_len = normalized_a.len().max(normalized_b.len());

        if max_len > 0 {
            1.0 - (distance as f32 / max_len as f32)
        } else {
            0.0
        }
    }

    /// Normalize a name for comparison
    fn normalize_name(&self, name: &str) -> String {
        let mut normalized = String::new();

        // Convert camelCase/PascalCase to snake_case and lowercase
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                normalized.push('_');
            }
            normalized.push(c.to_ascii_lowercase());
        }

        // Remove common prefixes/suffixes
        let patterns = [
            "calculate_",
            "calc_",
            "_calc",
            "validate_",
            "check_",
            "_validate",
            "get_",
            "fetch_",
            "_get",
            "process_",
            "handle_",
            "_process",
            "is_",
            "has_",
            "can_",
        ];

        for pattern in &patterns {
            normalized = normalized.replace(pattern, "");
        }

        // Remove double underscores
        while normalized.contains("__") {
            normalized = normalized.replace("__", "_");
        }

        // Trim underscores
        normalized.trim_matches('_').to_string()
    }

    /// Calculate type similarity
    fn type_similarity(&self, a: &UnifiedConcept, b: &UnifiedConcept) -> f32 {
        a.universal_type.similarity_to(&b.universal_type)
    }

    /// Derive a canonical name for an equivalence class
    fn derive_canonical_name(
        &self,
        members: &[ConceptId],
        substrate: &UnifiedSemanticSubstrate,
    ) -> String {
        // Use the most common normalized name
        let mut name_counts: HashMap<String, usize> = HashMap::new();

        for id in members {
            if let Some(concept) = substrate.get_concept(id) {
                let normalized = self.normalize_name(&concept.name);
                *name_counts.entry(normalized).or_insert(0) += 1;
            }
        }

        name_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name)
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Calculate cohesion (internal similarity) of an equivalence class
    fn calculate_cohesion(
        &self,
        class: &EquivalenceClass,
        substrate: &UnifiedSemanticSubstrate,
    ) -> f32 {
        if class.members.len() < 2 {
            return 1.0;
        }

        let concepts: Vec<_> = class
            .members
            .iter()
            .filter_map(|id| substrate.get_concept(id))
            .collect();

        let mut total_sim = 0.0;
        let mut count = 0;

        for i in 0..concepts.len() {
            for j in (i + 1)..concepts.len() {
                total_sim += self.embedding_similarity(concepts[i], concepts[j]);
                count += 1;
            }
        }

        if count > 0 {
            total_sim / count as f32
        } else {
            0.0
        }
    }

    /// Find relationships between concepts across languages
    pub fn find_relationships(
        &self,
        substrate: &UnifiedSemanticSubstrate,
    ) -> Vec<CrossLanguageRelationship> {
        let mut relationships = Vec::new();
        let concepts: Vec<_> = substrate.concepts().collect();

        for i in 0..concepts.len() {
            for j in 0..concepts.len() {
                if i == j {
                    continue;
                }

                let source = concepts[i];
                let target = concepts[j];

                // Skip same-language pairs (handled by regular call graph)
                if source.language() == target.language() {
                    continue;
                }

                // Check for call relationships via dependencies
                if source.properties.dependencies.contains(&target.id) {
                    relationships.push(CrossLanguageRelationship {
                        source_id: source.id.clone(),
                        target_id: target.id.clone(),
                        relationship_type: RelationshipType::Calls,
                        strength: 0.8,
                        description: format!(
                            "{} ({}) calls {} ({})",
                            source.name,
                            source.language(),
                            target.name,
                            target.language()
                        ),
                    });
                }

                // Check for shared contract relationships (same universal type, similar names)
                if source.universal_type == target.universal_type
                    && source.universal_type == UniversalConceptType::DataStructure
                {
                    let name_sim = self.name_similarity(&source.name, &target.name);
                    if name_sim > 0.7 {
                        relationships.push(CrossLanguageRelationship {
                            source_id: source.id.clone(),
                            target_id: target.id.clone(),
                            relationship_type: RelationshipType::SharedContract,
                            strength: name_sim,
                            description: format!(
                                "{} ({}) and {} ({}) share a common contract",
                                source.name,
                                source.language(),
                                target.name,
                                target.language()
                            ),
                        });
                    }
                }
            }
        }

        relationships
    }
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::{ConceptType, Visibility};
    use std::collections::HashMap;

    fn create_test_concept(
        name: &str,
        language: Language,
        concept_type: UniversalConceptType,
        embedding: [f32; 64],
    ) -> UnifiedConcept {
        UnifiedConcept {
            id: ConceptId::new(language, name, "test.file"),
            name: name.to_string(),
            universal_type: concept_type,
            language_specific: super::super::unified_substrate::LanguageSpecificData {
                language,
                original_type: ConceptType::Unknown,
                properties: HashMap::new(),
                file_path: "test.file".to_string(),
                line_range: (1, 10),
            },
            properties: super::super::unified_substrate::UnifiedProperties {
                documentation: None,
                visibility: Visibility::Public,
                complexity_score: 0.0,
                has_tests: false,
                is_async: false,
                is_deprecated: false,
                dependencies: Vec::new(),
                dependents: Vec::new(),
            },
            embedding,
        }
    }

    #[test]
    fn test_name_similarity_exact() {
        let aligner = CrossLanguageAligner::new();
        assert_eq!(
            aligner.name_similarity("calculate_total", "calculate_total"),
            1.0
        );
    }

    #[test]
    fn test_name_similarity_camel_snake() {
        let aligner = CrossLanguageAligner::new();
        // calculateTotal -> calculate_total (after normalization)
        let sim = aligner.name_similarity("calculateTotal", "calculate_total");
        assert!(
            sim > 0.9,
            "camelCase and snake_case should be similar: {}",
            sim
        );
    }

    #[test]
    fn test_name_similarity_different() {
        let aligner = CrossLanguageAligner::new();
        let sim = aligner.name_similarity("calculate_tax", "send_email");
        assert!(
            sim < 0.5,
            "Different names should have low similarity: {}",
            sim
        );
    }

    #[test]
    fn test_normalize_name() {
        let aligner = CrossLanguageAligner::new();
        assert_eq!(aligner.normalize_name("calculateOrderTotal"), "order_total");
        assert_eq!(aligner.normalize_name("validate_email"), "email");
        assert_eq!(aligner.normalize_name("getUser"), "user");
    }

    #[test]
    fn test_find_equivalents() {
        let mut substrate = UnifiedSemanticSubstrate::new();

        // Create similar concepts in different languages
        let embedding_base: [f32; 64] = [0.5; 64];
        let mut embedding_similar = embedding_base;
        embedding_similar[0] = 0.51; // Slight variation

        substrate.add_concept(create_test_concept(
            "calculate_order_total",
            Language::Python,
            UniversalConceptType::Calculation,
            embedding_base,
        ));

        substrate.add_concept(create_test_concept(
            "calculateOrderTotal",
            Language::TypeScript,
            UniversalConceptType::Calculation,
            embedding_similar,
        ));

        substrate.add_concept(create_test_concept(
            "send_email",
            Language::Python,
            UniversalConceptType::Integration,
            [0.1; 64], // Very different embedding
        ));

        let aligner = CrossLanguageAligner::new();
        let equivalents = aligner.find_equivalents(&substrate);

        // Should find the calculate_order_total equivalence
        assert!(
            !equivalents.is_empty(),
            "Should find at least one equivalent"
        );

        let found_calc_equiv = equivalents.iter().any(|eq| {
            let names_match = (eq
                .concept_a_id
                .to_string()
                .contains("calculate_order_total")
                && eq.concept_b_id.to_string().contains("calculateOrderTotal"))
                || (eq.concept_a_id.to_string().contains("calculateOrderTotal")
                    && eq
                        .concept_b_id
                        .to_string()
                        .contains("calculate_order_total"));
            names_match
        });

        assert!(
            found_calc_equiv,
            "Should find calculate_order_total equivalence"
        );
    }

    #[test]
    fn test_cluster_equivalents() {
        let mut substrate = UnifiedSemanticSubstrate::new();

        let embedding: [f32; 64] = [0.5; 64];

        let py_id = ConceptId::new(Language::Python, "calc_total", "test.py");
        let ts_id = ConceptId::new(Language::TypeScript, "calcTotal", "test.ts");
        let abl_id = ConceptId::new(Language::ABL, "calc_total", "test.p");

        substrate.add_concept(UnifiedConcept {
            id: py_id.clone(),
            name: "calc_total".to_string(),
            universal_type: UniversalConceptType::Calculation,
            language_specific: super::super::unified_substrate::LanguageSpecificData {
                language: Language::Python,
                original_type: ConceptType::Unknown,
                properties: HashMap::new(),
                file_path: "test.py".to_string(),
                line_range: (1, 10),
            },
            properties: super::super::unified_substrate::UnifiedProperties::default(),
            embedding,
        });

        substrate.add_concept(UnifiedConcept {
            id: ts_id.clone(),
            name: "calcTotal".to_string(),
            universal_type: UniversalConceptType::Calculation,
            language_specific: super::super::unified_substrate::LanguageSpecificData {
                language: Language::TypeScript,
                original_type: ConceptType::Unknown,
                properties: HashMap::new(),
                file_path: "test.ts".to_string(),
                line_range: (1, 10),
            },
            properties: super::super::unified_substrate::UnifiedProperties::default(),
            embedding,
        });

        substrate.add_concept(UnifiedConcept {
            id: abl_id.clone(),
            name: "calc_total".to_string(),
            universal_type: UniversalConceptType::Calculation,
            language_specific: super::super::unified_substrate::LanguageSpecificData {
                language: Language::ABL,
                original_type: ConceptType::Unknown,
                properties: HashMap::new(),
                file_path: "test.p".to_string(),
                line_range: (1, 10),
            },
            properties: super::super::unified_substrate::UnifiedProperties::default(),
            embedding,
        });

        let equivalents = vec![
            CrossLanguageEquivalent {
                concept_a_id: py_id.clone(),
                concept_b_id: ts_id.clone(),
                similarity: 0.9,
                evidence: vec![],
                confidence: MatchConfidence::High,
            },
            CrossLanguageEquivalent {
                concept_a_id: ts_id.clone(),
                concept_b_id: abl_id.clone(),
                similarity: 0.85,
                evidence: vec![],
                confidence: MatchConfidence::High,
            },
        ];

        let aligner = CrossLanguageAligner::new();
        let classes = aligner.cluster_equivalents(&equivalents, &substrate);

        assert_eq!(classes.len(), 1, "Should form one equivalence class");
        assert_eq!(classes[0].members.len(), 3, "Class should have 3 members");
        assert_eq!(
            classes[0].languages.len(),
            3,
            "Class should span 3 languages"
        );
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }
}
