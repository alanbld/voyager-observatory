//! Multi-Language Project Analysis
//!
//! This module provides types and functions for analyzing projects that
//! contain multiple programming languages, generating unified exploration
//! paths that cross language boundaries.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    ConceptId, CrossLanguageAligner, EquivalenceClass,
    FeatureNormalizer, Language, UnifiedConcept,
    UnifiedSemanticSubstrate, UniversalConceptType, UserContext,
};
use crate::plugins::PluginRegistry;

// =============================================================================
// Language Breakdown
// =============================================================================

/// Statistics about a language's contribution to the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLanguageStats {
    /// Number of files
    pub file_count: usize,
    /// Total lines of code
    pub line_count: usize,
    /// Number of concepts extracted
    pub concept_count: usize,
    /// Concept type distribution
    pub type_distribution: HashMap<String, usize>,
    /// Key files in this language
    pub key_files: Vec<String>,
}

/// Breakdown of languages in a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageBreakdown {
    /// Stats per language
    pub languages: HashMap<Language, ProjectLanguageStats>,
    /// Primary language (most concepts)
    pub primary_language: Option<Language>,
    /// Total files across all languages
    pub total_files: usize,
    /// Total concepts across all languages
    pub total_concepts: usize,
}

impl LanguageBreakdown {
    pub fn new() -> Self {
        Self {
            languages: HashMap::new(),
            primary_language: None,
            total_files: 0,
            total_concepts: 0,
        }
    }

    pub fn add_language(&mut self, language: Language, stats: ProjectLanguageStats) {
        self.total_files += stats.file_count;
        self.total_concepts += stats.concept_count;

        // Update primary language
        if let Some(primary) = self.primary_language {
            if let Some(primary_stats) = self.languages.get(&primary) {
                if stats.concept_count > primary_stats.concept_count {
                    self.primary_language = Some(language);
                }
            }
        } else {
            self.primary_language = Some(language);
        }

        self.languages.insert(language, stats);
    }

    /// Get languages sorted by concept count
    pub fn sorted_languages(&self) -> Vec<(Language, &ProjectLanguageStats)> {
        let mut langs: Vec<_> = self.languages.iter().map(|(l, s)| (*l, s)).collect();
        langs.sort_by(|a, b| b.1.concept_count.cmp(&a.1.concept_count));
        langs
    }
}

impl Default for LanguageBreakdown {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Cross-Language Insight
// =============================================================================

/// An insight about cross-language patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLanguageInsight {
    /// Insight type
    pub insight_type: InsightType,
    /// Description
    pub description: String,
    /// Related concepts
    pub related_concepts: Vec<ConceptId>,
    /// Languages involved
    pub languages: Vec<Language>,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
}

/// Types of cross-language insights
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightType {
    /// Same concept exists in multiple languages
    SharedConcept,
    /// One language calls into another
    CrossLanguageCall,
    /// Data flows between languages
    DataFlow,
    /// Shared interface/contract
    SharedContract,
    /// Duplicated logic (potential for refactoring)
    DuplicatedLogic,
    /// Missing validation in one language
    ValidationGap,
}

// =============================================================================
// Cross-Language Exploration Step
// =============================================================================

/// A step in a cross-language exploration path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLanguageExplorationStep {
    /// Concept being explored
    pub concept_id: ConceptId,
    /// Concept name
    pub name: String,
    /// Source language
    pub language: Language,
    /// Universal concept type
    pub universal_type: UniversalConceptType,
    /// File path
    pub file_path: String,
    /// Line range
    pub line_range: (usize, usize),
    /// Relevance score for this step
    pub relevance_score: f32,
    /// Reading decision
    pub decision: ReadingDecision,
    /// Equivalent concepts in other languages
    pub equivalents: Vec<EquivalentReference>,
    /// Estimated reading time in minutes
    pub estimated_time_minutes: u32,
    /// Language-specific tips
    pub language_tips: Vec<String>,
}

/// Reference to an equivalent concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalentReference {
    /// Concept ID
    pub concept_id: ConceptId,
    /// Language
    pub language: Language,
    /// Name
    pub name: String,
    /// Similarity score
    pub similarity: f32,
}

/// Reading decision for a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadingDecision {
    /// Read this code deeply
    ReadDeeply {
        reason: String,
        focus_points: Vec<String>,
    },
    /// Skim this code
    Skim {
        reason: String,
        key_patterns: Vec<String>,
        time_limit_minutes: u32,
    },
    /// Read with additional context
    ReadWithContext {
        reason: String,
        language_context: String,
        prerequisites: Vec<String>,
    },
    /// Skip this code
    Skip {
        reason: String,
        alternative: Option<String>,
    },
}

// =============================================================================
// Multi-Language Exploration Result
// =============================================================================

/// Result of multi-language exploration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLanguageExplorationResult {
    /// Intent that was explored
    pub intent: String,
    /// Project summary
    pub project_summary: String,
    /// Language breakdown
    pub language_breakdown: LanguageBreakdown,
    /// Cross-language insights
    pub cross_language_insights: Vec<CrossLanguageInsight>,
    /// Exploration path (ordered steps)
    pub exploration_path: Vec<CrossLanguageExplorationStep>,
    /// Total estimated time in minutes
    pub estimated_time_minutes: u32,
    /// Equivalence classes found
    pub equivalence_classes: Vec<EquivalenceClass>,
}

impl MultiLanguageExplorationResult {
    /// Get languages represented in exploration path
    pub fn path_languages(&self) -> Vec<Language> {
        let mut languages: Vec<_> = self
            .exploration_path
            .iter()
            .map(|s| s.language)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        languages.sort_by_key(|l| format!("{:?}", l));
        languages
    }

    /// Get steps for a specific language
    pub fn steps_for_language(&self, language: Language) -> Vec<&CrossLanguageExplorationStep> {
        self.exploration_path
            .iter()
            .filter(|s| s.language == language)
            .collect()
    }

    /// Generate a formatted summary
    pub fn format_summary(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("🎯 MULTI-LANGUAGE EXPLORATION: {}\n\n", self.intent));
        output.push_str(&format!("{}\n\n", self.project_summary));

        // Language breakdown
        output.push_str("📊 Language Breakdown:\n");
        for (lang, stats) in self.language_breakdown.sorted_languages() {
            output.push_str(&format!(
                "  • {}: {} concepts ({} files)\n",
                lang.display_name(),
                stats.concept_count,
                stats.file_count
            ));
        }
        output.push('\n');

        // Cross-language insights
        if !self.cross_language_insights.is_empty() {
            output.push_str("🔗 Cross-Language Insights:\n");
            for insight in &self.cross_language_insights {
                output.push_str(&format!("  • {}\n", insight.description));
            }
            output.push('\n');
        }

        // Exploration path
        output.push_str("🧭 Recommended Path:\n");
        for (i, step) in self.exploration_path.iter().take(10).enumerate() {
            let decision_emoji = match &step.decision {
                ReadingDecision::ReadDeeply { .. } => "📖",
                ReadingDecision::Skim { .. } => "👀",
                ReadingDecision::ReadWithContext { .. } => "📚",
                ReadingDecision::Skip { .. } => "⏭️",
            };
            output.push_str(&format!(
                "  {}. {} [{}] {} ({})\n",
                i + 1,
                decision_emoji,
                step.language,
                step.name,
                format!("{:?}", step.universal_type)
            ));
        }
        if self.exploration_path.len() > 10 {
            output.push_str(&format!(
                "  ... and {} more steps\n",
                self.exploration_path.len() - 10
            ));
        }
        output.push('\n');

        output.push_str(&format!(
            "⏱️ Estimated time: {} minutes\n",
            self.estimated_time_minutes
        ));

        output
    }
}

// =============================================================================
// Multi-Language Project
// =============================================================================

/// Represents a project with multiple programming languages
#[derive(Debug, Clone)]
pub struct MultiLanguageProject {
    /// Project root path
    pub root_path: PathBuf,
    /// Files organized by language
    pub files_by_language: HashMap<Language, Vec<PathBuf>>,
    /// Total file count
    pub total_files: usize,
}

impl MultiLanguageProject {
    /// Create a new multi-language project
    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            files_by_language: HashMap::new(),
            total_files: 0,
        }
    }

    /// Detect project structure from a path
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let mut project = Self::new(path.to_path_buf());

        // Walk the directory
        fn visit_dir(
            dir: &Path,
            files_by_language: &mut HashMap<Language, Vec<PathBuf>>,
        ) -> std::io::Result<usize> {
            let mut count = 0;

            if dir.is_dir() {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    // Skip hidden directories and common non-source directories
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with('.')
                            || name == "node_modules"
                            || name == "__pycache__"
                            || name == "target"
                            || name == "dist"
                            || name == "build"
                        {
                            continue;
                        }
                    }

                    if path.is_dir() {
                        count += visit_dir(&path, files_by_language)?;
                    } else if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let language = Language::from_extension(ext);
                            if language != Language::Unknown && language.has_plugin() {
                                files_by_language
                                    .entry(language)
                                    .or_insert_with(Vec::new)
                                    .push(path);
                                count += 1;
                            }
                        }
                    }
                }
            }

            Ok(count)
        }

        project.total_files = visit_dir(path, &mut project.files_by_language)?;

        Ok(project)
    }

    /// Get languages in the project
    pub fn languages(&self) -> Vec<Language> {
        self.files_by_language.keys().copied().collect()
    }

    /// Check if this is a multi-language project
    pub fn is_multi_language(&self) -> bool {
        self.files_by_language.len() > 1
    }

    /// Get files for a specific language
    pub fn files_for_language(&self, language: Language) -> Vec<&PathBuf> {
        self.files_by_language
            .get(&language)
            .map(|files| files.iter().collect())
            .unwrap_or_default()
    }
}

// =============================================================================
// Multi-Language Explorer
// =============================================================================

/// Explores multi-language projects with unified semantics
pub struct MultiLanguageExplorer {
    plugin_registry: PluginRegistry,
    aligner: CrossLanguageAligner,
    normalizer: FeatureNormalizer,
}

impl MultiLanguageExplorer {
    /// Create a new explorer
    pub fn new() -> Self {
        Self {
            plugin_registry: PluginRegistry::with_defaults(),
            aligner: CrossLanguageAligner::default(),
            normalizer: FeatureNormalizer::language_weighted(),
        }
    }

    /// Analyze a multi-language project
    pub fn analyze_project(
        &self,
        project: &MultiLanguageProject,
    ) -> Result<UnifiedSemanticSubstrate, String> {
        let mut substrate = UnifiedSemanticSubstrate::new();

        for (language, files) in &project.files_by_language {
            // Find plugin for this language
            let plugin = match self.plugin_registry.find_by_language(&format!("{:?}", language).to_lowercase()) {
                Some(p) => p,
                None => continue,
            };

            for file_path in files {
                // Read file content
                let content = match std::fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Extract symbols using plugin
                let symbols = match plugin.extract_symbols(&content) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let file_str = file_path.to_string_lossy().to_string();

                // Convert to unified concepts
                for symbol in symbols {
                    let concept_type = plugin.infer_concept_type(&symbol, &content);
                    let universal_type = UniversalConceptType::from_concept_type(concept_type);

                    // Get language features and create embedding
                    let features = plugin.language_features(&symbol, &content);
                    let mut embedding = [0.0f32; 64];

                    // Set base features
                    embedding[0] = symbol.parameters.len() as f32 * 0.1; // Param count
                    embedding[1] = if symbol.documentation.is_some() { 0.5 } else { 0.0 };

                    // Apply language-specific features
                    for (idx, value) in features {
                        if idx < 64 {
                            embedding[idx] = value;
                        }
                    }

                    // Normalize the embedding
                    let normalized_embedding = self.normalizer.normalize(&embedding, *language);

                    let concept = UnifiedConcept {
                        id: ConceptId::new(*language, &symbol.name, &file_str),
                        name: symbol.name.clone(),
                        universal_type,
                        language_specific: super::unified_substrate::LanguageSpecificData {
                            language: *language,
                            original_type: concept_type,
                            properties: HashMap::from([
                                ("kind".to_string(), format!("{:?}", symbol.kind)),
                                ("signature".to_string(), symbol.signature.clone()),
                            ]),
                            file_path: file_str.clone(),
                            line_range: (symbol.range.start_line, symbol.range.end_line),
                        },
                        properties: super::unified_substrate::UnifiedProperties {
                            documentation: symbol.documentation.clone(),
                            visibility: symbol.visibility,
                            complexity_score: 0.0,
                            has_tests: false,
                            is_async: symbol.signature.contains("async"),
                            is_deprecated: false,
                            dependencies: Vec::new(),
                            dependents: Vec::new(),
                        },
                        embedding: normalized_embedding,
                    };

                    substrate.add_concept(concept);
                }
            }
        }

        // Find cross-language equivalents
        let equivalents = self.aligner.find_equivalents(&substrate);
        for eq in &equivalents {
            substrate.register_equivalence(&eq.concept_a_id, &eq.concept_b_id);
        }

        Ok(substrate)
    }

    /// Explore a project with a given intent
    pub fn explore(
        &self,
        project: &MultiLanguageProject,
        intent: &str,
        context: &UserContext,
    ) -> Result<MultiLanguageExplorationResult, String> {
        // Analyze the project
        let substrate = self.analyze_project(project)?;

        // Calculate language breakdown
        let language_breakdown = self.calculate_language_breakdown(&substrate, project);

        // Find cross-language equivalents and cluster them
        let equivalents = self.aligner.find_equivalents(&substrate);
        let equivalence_classes = self.aligner.cluster_equivalents(&equivalents, &substrate);

        // Generate cross-language insights
        let insights = self.generate_insights(&substrate, &equivalence_classes);

        // Score concepts for the intent
        let scored = substrate.score_for_intent(intent, context);

        // Generate exploration path
        let path = self.generate_exploration_path(&scored, &substrate, context, intent);

        // Calculate total time
        let estimated_time: u32 = path.iter().map(|s| s.estimated_time_minutes).sum();

        Ok(MultiLanguageExplorationResult {
            intent: intent.to_string(),
            project_summary: self.generate_project_summary(project, &language_breakdown),
            language_breakdown,
            cross_language_insights: insights,
            exploration_path: path,
            estimated_time_minutes: estimated_time,
            equivalence_classes,
        })
    }

    /// Calculate language breakdown from substrate
    fn calculate_language_breakdown(
        &self,
        substrate: &UnifiedSemanticSubstrate,
        project: &MultiLanguageProject,
    ) -> LanguageBreakdown {
        let mut breakdown = LanguageBreakdown::new();

        for language in substrate.languages() {
            let concepts = substrate.concepts_for_language(language);
            let mut type_dist: HashMap<String, usize> = HashMap::new();

            for concept in &concepts {
                let type_name = format!("{:?}", concept.universal_type);
                *type_dist.entry(type_name).or_insert(0) += 1;
            }

            let file_count = project
                .files_by_language
                .get(&language)
                .map(|f| f.len())
                .unwrap_or(0);

            let key_files: Vec<String> = project
                .files_by_language
                .get(&language)
                .map(|files| {
                    files
                        .iter()
                        .take(3)
                        .filter_map(|p| p.file_name()?.to_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            breakdown.add_language(
                language,
                ProjectLanguageStats {
                    file_count,
                    line_count: 0, // Would require reading files
                    concept_count: concepts.len(),
                    type_distribution: type_dist,
                    key_files,
                },
            );
        }

        breakdown
    }

    /// Generate cross-language insights
    fn generate_insights(
        &self,
        substrate: &UnifiedSemanticSubstrate,
        equivalence_classes: &[EquivalenceClass],
    ) -> Vec<CrossLanguageInsight> {
        let mut insights = Vec::new();

        // Insight: Shared concepts across languages
        for class in equivalence_classes {
            if class.is_multi_language() {
                insights.push(CrossLanguageInsight {
                    insight_type: InsightType::SharedConcept,
                    description: format!(
                        "'{}' found in {} languages: {}",
                        class.canonical_name,
                        class.languages.len(),
                        class
                            .languages
                            .iter()
                            .map(|l| l.display_name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    related_concepts: class.members.clone(),
                    languages: class.languages.iter().copied().collect(),
                    importance: 0.8,
                });
            }
        }

        // Insight: Validation patterns
        let validation_concepts = substrate.concepts_of_type(UniversalConceptType::Validation);
        let validation_languages: std::collections::HashSet<_> =
            validation_concepts.iter().map(|c| c.language()).collect();

        if validation_languages.len() > 1 {
            insights.push(CrossLanguageInsight {
                insight_type: InsightType::ValidationGap,
                description: format!(
                    "Validation logic spread across {} languages - verify consistency",
                    validation_languages.len()
                ),
                related_concepts: validation_concepts.iter().map(|c| c.id.clone()).collect(),
                languages: validation_languages.into_iter().collect(),
                importance: 0.7,
            });
        }

        insights
    }

    /// Generate exploration path
    fn generate_exploration_path(
        &self,
        scored: &[(&UnifiedConcept, f32)],
        substrate: &UnifiedSemanticSubstrate,
        context: &UserContext,
        _intent: &str,
    ) -> Vec<CrossLanguageExplorationStep> {
        let mut path = Vec::new();
        let mut language_balance: HashMap<Language, usize> = HashMap::new();
        let max_per_language = 8;

        // Sort by score
        let mut sorted_scored: Vec<_> = scored.to_vec();
        sorted_scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (concept, score) in sorted_scored.iter().take(30) {
            let language = concept.language();

            // Balance languages
            let count = language_balance.entry(language).or_insert(0);
            if *count >= max_per_language {
                continue;
            }
            *count += 1;

            // Find equivalents
            let equivalents: Vec<_> = substrate
                .find_equivalents(&concept.id)
                .iter()
                .map(|eq| EquivalentReference {
                    concept_id: eq.id.clone(),
                    language: eq.language(),
                    name: eq.name.clone(),
                    similarity: concept.embedding_similarity(eq),
                })
                .collect();

            // Determine reading decision
            let familiarity = context.get_familiarity(language);
            let decision = if *score > 0.8 {
                ReadingDecision::ReadDeeply {
                    reason: format!(
                        "Highly relevant {:?} concept",
                        concept.universal_type
                    ),
                    focus_points: vec![
                        "Core logic".to_string(),
                        "Dependencies".to_string(),
                    ],
                }
            } else if *score > 0.5 && familiarity < 0.5 {
                ReadingDecision::ReadWithContext {
                    reason: "Moderately relevant but unfamiliar language".to_string(),
                    language_context: format!(
                        "This is {} code - {}",
                        language.display_name(),
                        self.get_language_tip(language)
                    ),
                    prerequisites: vec![],
                }
            } else if *score > 0.3 {
                ReadingDecision::Skim {
                    reason: "Worth understanding the pattern".to_string(),
                    key_patterns: vec!["Entry point".to_string()],
                    time_limit_minutes: 3,
                }
            } else {
                ReadingDecision::Skip {
                    reason: "Low relevance to current intent".to_string(),
                    alternative: None,
                }
            };

            // Estimate time
            let base_time = match &decision {
                ReadingDecision::ReadDeeply { .. } => 5,
                ReadingDecision::ReadWithContext { .. } => 4,
                ReadingDecision::Skim { time_limit_minutes, .. } => *time_limit_minutes,
                ReadingDecision::Skip { .. } => 0,
            };

            path.push(CrossLanguageExplorationStep {
                concept_id: concept.id.clone(),
                name: concept.name.clone(),
                language,
                universal_type: concept.universal_type,
                file_path: concept.language_specific.file_path.clone(),
                line_range: concept.language_specific.line_range,
                relevance_score: *score,
                decision,
                equivalents,
                estimated_time_minutes: base_time,
                language_tips: self.get_language_tips(language),
            });
        }

        path
    }

    /// Generate project summary
    fn generate_project_summary(
        &self,
        project: &MultiLanguageProject,
        breakdown: &LanguageBreakdown,
    ) -> String {
        let langs: Vec<_> = breakdown
            .sorted_languages()
            .iter()
            .map(|(l, _)| l.display_name())
            .collect();

        format!(
            "Multi-language project with {} files across {} languages ({})",
            project.total_files,
            breakdown.languages.len(),
            langs.join(", ")
        )
    }

    /// Get a tip for working with a language
    fn get_language_tip(&self, language: Language) -> &'static str {
        match language {
            Language::ABL => "Procedural with strong database integration",
            Language::Python => "Dynamic typing, indentation-based blocks",
            Language::TypeScript => "Type-safe JavaScript with interfaces",
            Language::JavaScript => "Dynamic, event-driven patterns",
            Language::Shell => "Script-based automation and pipelines",
            _ => "Standard programming patterns apply",
        }
    }

    /// Get tips for working with a language
    fn get_language_tips(&self, language: Language) -> Vec<String> {
        match language {
            Language::ABL => vec![
                "Look for TEMP-TABLE definitions for data structures".to_string(),
                "PROCEDURE/FUNCTION blocks contain business logic".to_string(),
            ],
            Language::Python => vec![
                "Check decorators for behavior modifications".to_string(),
                "Type hints indicate expected data types".to_string(),
            ],
            Language::TypeScript => vec![
                "Interfaces define data contracts".to_string(),
                "Check for async/await patterns".to_string(),
            ],
            Language::JavaScript => vec![
                "Check for callbacks and Promise patterns".to_string(),
                "Event handlers are common entry points".to_string(),
            ],
            Language::Shell => vec![
                "Look for exported functions and variables".to_string(),
                "Check sourced files for dependencies".to_string(),
            ],
            _ => vec!["Standard programming patterns apply".to_string()],
        }
    }
}

impl Default for MultiLanguageExplorer {
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
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project() -> (TempDir, MultiLanguageProject) {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create Python file
        fs::create_dir_all(root.join("backend")).unwrap();
        fs::write(
            root.join("backend/order.py"),
            r#"
def calculate_total(items: list) -> float:
    """Calculate order total."""
    return sum(item.price for item in items)

def validate_order(order: dict) -> bool:
    """Validate order data."""
    return order.get("items") is not None
"#,
        )
        .unwrap();

        // Create TypeScript file
        fs::create_dir_all(root.join("frontend")).unwrap();
        fs::write(
            root.join("frontend/order.ts"),
            r#"
export function calculateTotal(items: OrderItem[]): number {
    return items.reduce((sum, item) => sum + item.price, 0);
}

export function validateOrder(order: Order): boolean {
    return order.items !== undefined && order.items.length > 0;
}
"#,
        )
        .unwrap();

        let project = MultiLanguageProject::from_path(root).unwrap();
        (temp, project)
    }

    #[test]
    fn test_multi_language_project_detection() {
        let (temp, project) = create_test_project();

        assert!(project.is_multi_language());
        assert!(project.languages().contains(&Language::Python));
        assert!(project.languages().contains(&Language::TypeScript));
        assert_eq!(project.total_files, 2);

        drop(temp); // Keep temp alive until test ends
    }

    #[test]
    fn test_language_breakdown() {
        let mut breakdown = LanguageBreakdown::new();

        breakdown.add_language(
            Language::Python,
            ProjectLanguageStats {
                file_count: 10,
                line_count: 500,
                concept_count: 25,
                type_distribution: HashMap::new(),
                key_files: vec![],
            },
        );

        breakdown.add_language(
            Language::TypeScript,
            ProjectLanguageStats {
                file_count: 15,
                line_count: 800,
                concept_count: 30,
                type_distribution: HashMap::new(),
                key_files: vec![],
            },
        );

        assert_eq!(breakdown.total_files, 25);
        assert_eq!(breakdown.total_concepts, 55);
        assert_eq!(breakdown.primary_language, Some(Language::TypeScript));
    }

    #[test]
    fn test_multi_language_explorer_analyze() {
        let (temp, project) = create_test_project();

        let explorer = MultiLanguageExplorer::new();
        let substrate = explorer.analyze_project(&project).unwrap();

        // Should have concepts from both languages
        let languages = substrate.languages();
        assert!(languages.len() >= 2, "Should detect multiple languages");

        // Should have concepts
        assert!(substrate.concept_count() > 0, "Should extract concepts");

        drop(temp);
    }

    #[test]
    fn test_multi_language_exploration() {
        let (temp, project) = create_test_project();

        let explorer = MultiLanguageExplorer::new();
        let context = UserContext::new()
            .with_familiarity(Language::Python, 0.9)
            .with_familiarity(Language::TypeScript, 0.7);

        let result = explorer.explore(&project, "business-logic", &context).unwrap();

        // Should have results
        assert!(!result.exploration_path.is_empty(), "Should have exploration steps");
        assert!(result.language_breakdown.languages.len() >= 2, "Should analyze multiple languages");

        // Summary should mention both languages
        let summary = result.format_summary();
        assert!(summary.contains("Python") || summary.contains("TypeScript"));

        drop(temp);
    }

    #[test]
    fn test_exploration_result_formatting() {
        let result = MultiLanguageExplorationResult {
            intent: "business-logic".to_string(),
            project_summary: "Test project".to_string(),
            language_breakdown: LanguageBreakdown::default(),
            cross_language_insights: vec![],
            exploration_path: vec![],
            estimated_time_minutes: 30,
            equivalence_classes: vec![],
        };

        let summary = result.format_summary();
        assert!(summary.contains("MULTI-LANGUAGE EXPLORATION"));
        assert!(summary.contains("business-logic"));
    }
}
