//! Feature Vector Normalization
//!
//! This module provides normalization strategies for 64D feature vectors
//! to make them comparable across different programming languages.
//! Each language has different typical values for metrics like complexity,
//! so normalization is essential for cross-language comparison.

use std::collections::HashMap;

use super::Language;

// =============================================================================
// Normalization Strategy
// =============================================================================

/// Strategy for normalizing feature vectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationStrategy {
    /// Z-score normalization (mean=0, std=1)
    ZScore,
    /// Min-max normalization (0-1 range)
    MinMax,
    /// Language-specific weighted normalization
    LanguageWeighted,
    /// No normalization (raw values)
    None,
}

// =============================================================================
// Language Normalization Config
// =============================================================================

/// Language-specific normalization configuration
#[derive(Debug, Clone)]
pub struct LanguageNormalizationConfig {
    /// Feature index weights (adjust importance per language)
    pub feature_weights: [f32; 64],
    /// Feature index mappings (remap language-specific features to universal)
    pub feature_mappings: HashMap<usize, usize>,
    /// Expected ranges for each feature (for min-max normalization)
    pub expected_ranges: [(f32, f32); 64],
    /// Baseline values (typical values for this language)
    pub baselines: [f32; 64],
}

impl LanguageNormalizationConfig {
    /// Create default config (no adjustments)
    pub fn default_config() -> Self {
        Self {
            feature_weights: [1.0; 64],
            feature_mappings: HashMap::new(),
            expected_ranges: [(0.0, 1.0); 64],
            baselines: [0.0; 64],
        }
    }

    /// Configuration for ABL language
    pub fn abl() -> Self {
        let mut config = Self::default_config();

        // ABL tends to have longer procedures, adjust complexity weighting
        config.feature_weights[0] = 0.8; // Reduce cyclomatic complexity weight
        config.feature_weights[1] = 1.2; // Increase nesting depth importance

        // ABL-specific features (indices 50-54) map to universal positions
        config.feature_mappings.insert(50, 20); // Database operations → universal DB index
        config.feature_mappings.insert(51, 21); // TEMP-TABLE usage → universal data structure index
        config.feature_mappings.insert(52, 22); // Buffer operations → universal buffer index
        config.feature_mappings.insert(53, 23); // Transaction scope → universal transaction index
        config.feature_mappings.insert(54, 24); // Super calls → universal inheritance index

        // ABL-specific expected ranges
        config.expected_ranges[0] = (0.0, 0.6); // ABL cyclomatic complexity typically lower
        config.expected_ranges[1] = (0.0, 0.8); // Nesting can be deeper

        // Baselines for ABL
        config.baselines[0] = 0.3; // Typical complexity
        config.baselines[1] = 0.4; // Typical nesting

        config
    }

    /// Configuration for Python language
    pub fn python() -> Self {
        let mut config = Self::default_config();

        // Python is more concise, adjust accordingly
        config.feature_weights[0] = 1.1; // Slightly increase complexity weight
        config.feature_weights[1] = 0.9; // Reduce nesting weight (indentation-based)

        // Python-specific features (indices 55-59) map to universal positions
        config.feature_mappings.insert(55, 25); // Async patterns → universal async index
        config.feature_mappings.insert(56, 26); // Type hints → universal type index
        config.feature_mappings.insert(57, 27); // Decorators → universal decorator index
        config.feature_mappings.insert(58, 28); // Context managers → universal resource index
        config.feature_mappings.insert(59, 29); // Generator patterns → universal iterator index

        // Python-specific expected ranges
        config.expected_ranges[0] = (0.0, 0.7);
        config.expected_ranges[1] = (0.0, 0.5); // Python typically has less nesting

        // Baselines for Python
        config.baselines[0] = 0.25;
        config.baselines[1] = 0.2;

        config
    }

    /// Configuration for TypeScript language
    pub fn typescript() -> Self {
        let mut config = Self::default_config();

        // TypeScript has rich type information
        config.feature_weights[0] = 0.9;
        config.feature_weights[1] = 0.8;

        // TypeScript-specific features (indices 60-63) map to universal positions
        config.feature_mappings.insert(60, 30); // Type completeness → universal type index
        config.feature_mappings.insert(61, 31); // Async usage → universal async index
        config.feature_mappings.insert(62, 32); // Generic complexity → universal generics index
        config.feature_mappings.insert(63, 33); // Framework patterns → universal framework index

        // TypeScript-specific expected ranges
        config.expected_ranges[0] = (0.0, 0.6);
        config.expected_ranges[1] = (0.0, 0.5);

        // Baselines for TypeScript
        config.baselines[0] = 0.2;
        config.baselines[1] = 0.15;

        config
    }

    /// Configuration for JavaScript (similar to TypeScript but without type features)
    pub fn javascript() -> Self {
        let mut config = Self::typescript();

        // JavaScript has no type system, so type-related features are less important
        config.feature_weights[60] = 0.0; // No type completeness
        config.baselines[60] = 0.0;

        config
    }

    /// Configuration for Shell scripts
    pub fn shell() -> Self {
        let mut config = Self::default_config();

        // Shell scripts have different complexity patterns
        config.feature_weights[0] = 0.7; // Lower complexity weight
        config.feature_weights[1] = 1.0; // Normal nesting

        // Shell typically has higher baseline complexity for simple tasks
        config.baselines[0] = 0.35;
        config.baselines[1] = 0.3;

        config
    }

    /// Get config for a language
    pub fn for_language(language: Language) -> Self {
        match language {
            Language::ABL => Self::abl(),
            Language::Python => Self::python(),
            Language::TypeScript => Self::typescript(),
            Language::JavaScript => Self::javascript(),
            Language::Shell => Self::shell(),
            _ => Self::default_config(),
        }
    }
}

// =============================================================================
// Feature Normalizer
// =============================================================================

/// Normalizer for feature vectors across languages
#[derive(Debug, Clone)]
pub struct FeatureNormalizer {
    strategy: NormalizationStrategy,
    language_configs: HashMap<Language, LanguageNormalizationConfig>,
    /// Global statistics for z-score normalization
    global_means: Option<[f32; 64]>,
    global_stds: Option<[f32; 64]>,
    /// Global min/max for min-max normalization
    global_mins: Option<[f32; 64]>,
    global_maxs: Option<[f32; 64]>,
}

impl FeatureNormalizer {
    /// Create a new normalizer with given strategy
    pub fn new(strategy: NormalizationStrategy) -> Self {
        let mut language_configs = HashMap::new();

        // Pre-populate with known language configs
        language_configs.insert(Language::ABL, LanguageNormalizationConfig::abl());
        language_configs.insert(Language::Python, LanguageNormalizationConfig::python());
        language_configs.insert(
            Language::TypeScript,
            LanguageNormalizationConfig::typescript(),
        );
        language_configs.insert(
            Language::JavaScript,
            LanguageNormalizationConfig::javascript(),
        );
        language_configs.insert(Language::Shell, LanguageNormalizationConfig::shell());

        Self {
            strategy,
            language_configs,
            global_means: None,
            global_stds: None,
            global_mins: None,
            global_maxs: None,
        }
    }

    /// Create with z-score normalization
    pub fn zscore() -> Self {
        Self::new(NormalizationStrategy::ZScore)
    }

    /// Create with min-max normalization
    pub fn minmax() -> Self {
        Self::new(NormalizationStrategy::MinMax)
    }

    /// Create with language-weighted normalization
    pub fn language_weighted() -> Self {
        Self::new(NormalizationStrategy::LanguageWeighted)
    }

    /// Fit the normalizer to a set of vectors
    pub fn fit(&mut self, vectors: &[[f32; 64]]) {
        if vectors.is_empty() {
            return;
        }

        let n = vectors.len() as f32;

        match self.strategy {
            NormalizationStrategy::ZScore => {
                // Calculate means
                let mut means = [0.0f32; 64];
                for vector in vectors {
                    for (i, &v) in vector.iter().enumerate() {
                        means[i] += v;
                    }
                }
                for mean in &mut means {
                    *mean /= n;
                }

                // Calculate standard deviations
                let mut stds = [0.0f32; 64];
                for vector in vectors {
                    for (i, &v) in vector.iter().enumerate() {
                        stds[i] += (v - means[i]).powi(2);
                    }
                }
                for std in &mut stds {
                    *std = (*std / n).sqrt().max(0.001); // Avoid division by zero
                }

                self.global_means = Some(means);
                self.global_stds = Some(stds);
            }
            NormalizationStrategy::MinMax => {
                let mut mins = [f32::MAX; 64];
                let mut maxs = [f32::MIN; 64];

                for vector in vectors {
                    for (i, &v) in vector.iter().enumerate() {
                        mins[i] = mins[i].min(v);
                        maxs[i] = maxs[i].max(v);
                    }
                }

                // Ensure non-zero range
                for i in 0..64 {
                    if (maxs[i] - mins[i]).abs() < 0.001 {
                        maxs[i] = mins[i] + 1.0;
                    }
                }

                self.global_mins = Some(mins);
                self.global_maxs = Some(maxs);
            }
            _ => {}
        }
    }

    /// Normalize a single vector
    pub fn normalize(&self, vector: &[f32; 64], language: Language) -> [f32; 64] {
        match self.strategy {
            NormalizationStrategy::None => *vector,
            NormalizationStrategy::ZScore => self.normalize_zscore(vector),
            NormalizationStrategy::MinMax => self.normalize_minmax(vector),
            NormalizationStrategy::LanguageWeighted => {
                self.normalize_language_weighted(vector, language)
            }
        }
    }

    /// Z-score normalization
    fn normalize_zscore(&self, vector: &[f32; 64]) -> [f32; 64] {
        let means = self.global_means.unwrap_or([0.0; 64]);
        let stds = self.global_stds.unwrap_or([1.0; 64]);

        let mut normalized = [0.0f32; 64];
        for (i, &v) in vector.iter().enumerate() {
            normalized[i] = (v - means[i]) / stds[i];
            // Clip to reasonable range
            normalized[i] = normalized[i].clamp(-3.0, 3.0);
            // Rescale to 0-1
            normalized[i] = (normalized[i] + 3.0) / 6.0;
        }
        normalized
    }

    /// Min-max normalization
    fn normalize_minmax(&self, vector: &[f32; 64]) -> [f32; 64] {
        let mins = self.global_mins.unwrap_or([0.0; 64]);
        let maxs = self.global_maxs.unwrap_or([1.0; 64]);

        let mut normalized = [0.0f32; 64];
        for (i, &v) in vector.iter().enumerate() {
            let range = maxs[i] - mins[i];
            if range > 0.001 {
                normalized[i] = ((v - mins[i]) / range).clamp(0.0, 1.0);
            } else {
                normalized[i] = 0.5;
            }
        }
        normalized
    }

    /// Language-weighted normalization
    fn normalize_language_weighted(&self, vector: &[f32; 64], language: Language) -> [f32; 64] {
        let config = self
            .language_configs
            .get(&language)
            .cloned()
            .unwrap_or_else(LanguageNormalizationConfig::default_config);

        let mut normalized = [0.0f32; 64];

        for i in 0..64 {
            let value = vector[i];
            let weight = config.feature_weights[i];
            let (min, max) = config.expected_ranges[i];
            let baseline = config.baselines[i];

            // Normalize to expected range
            let range = max - min;
            let adjusted = if range > 0.001 {
                ((value - min) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };

            // Apply weight and baseline adjustment
            normalized[i] = (adjusted - baseline).clamp(-1.0, 1.0) * weight;

            // Rescale to 0-1
            normalized[i] = (normalized[i] + 1.0) / 2.0;
        }

        // Apply feature mappings (remap language-specific to universal)
        for (&from, &to) in &config.feature_mappings {
            if from < 64 && to < 64 {
                normalized[to] = normalized[from];
            }
        }

        normalized
    }

    /// Normalize vectors for multiple languages and merge
    pub fn normalize_batch(
        &self,
        vectors_by_language: &HashMap<Language, Vec<[f32; 64]>>,
    ) -> Vec<[f32; 64]> {
        let mut all_normalized = Vec::new();

        for (language, vectors) in vectors_by_language {
            for vector in vectors {
                all_normalized.push(self.normalize(vector, *language));
            }
        }

        all_normalized
    }

    /// Get alignment score between two vectors (0-1, higher = more aligned)
    pub fn alignment_score(&self, a: &[f32; 64], b: &[f32; 64]) -> f32 {
        // Cosine similarity
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            ((dot / (norm_a * norm_b)) + 1.0) / 2.0 // Map from [-1,1] to [0,1]
        } else {
            0.5
        }
    }
}

impl Default for FeatureNormalizer {
    fn default() -> Self {
        Self::language_weighted()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zscore_normalization() {
        let mut normalizer = FeatureNormalizer::zscore();

        let vectors = vec![[0.0f32; 64], [1.0f32; 64], [0.5f32; 64]];

        normalizer.fit(&vectors);

        let normalized = normalizer.normalize(&[0.5f32; 64], Language::Python);

        // Should be close to 0.5 (mean after normalization)
        for &v in &normalized {
            assert!((v - 0.5).abs() < 0.1, "Expected ~0.5, got {}", v);
        }
    }

    #[test]
    fn test_minmax_normalization() {
        let mut normalizer = FeatureNormalizer::minmax();

        let vectors = vec![[0.0f32; 64], [10.0f32; 64]];

        normalizer.fit(&vectors);

        let normalized = normalizer.normalize(&[5.0f32; 64], Language::Python);

        // Should be 0.5 (midpoint)
        for &v in &normalized {
            assert!((v - 0.5).abs() < 0.01, "Expected 0.5, got {}", v);
        }
    }

    #[test]
    fn test_language_weighted_normalization() {
        let normalizer = FeatureNormalizer::language_weighted();

        let vector = [0.5f32; 64];

        let py_normalized = normalizer.normalize(&vector, Language::Python);
        let abl_normalized = normalizer.normalize(&vector, Language::ABL);

        // Different languages should produce different normalizations
        // (due to different weights and baselines)
        let mut differences = 0;
        for (py, abl) in py_normalized.iter().zip(abl_normalized.iter()) {
            if (py - abl).abs() > 0.01 {
                differences += 1;
            }
        }

        assert!(
            differences > 0,
            "Language-weighted should produce different results per language"
        );
    }

    #[test]
    fn test_language_config_for_language() {
        let abl_config = LanguageNormalizationConfig::for_language(Language::ABL);
        let py_config = LanguageNormalizationConfig::for_language(Language::Python);

        // ABL should have reduced complexity weight
        assert!(abl_config.feature_weights[0] < py_config.feature_weights[0]);

        // Each should have its own feature mappings
        assert!(!abl_config.feature_mappings.is_empty());
        assert!(!py_config.feature_mappings.is_empty());
    }

    #[test]
    fn test_alignment_score() {
        let normalizer = FeatureNormalizer::default();

        let a = [1.0f32; 64];
        let b = [1.0f32; 64];
        let c = [-1.0f32; 64];

        // Identical vectors should have score 1.0
        let score_same = normalizer.alignment_score(&a, &b);
        assert!(
            (score_same - 1.0).abs() < 0.01,
            "Identical vectors should have score 1.0"
        );

        // Opposite vectors should have score 0.0
        let score_opposite = normalizer.alignment_score(&a, &c);
        assert!(
            (score_opposite - 0.0).abs() < 0.01,
            "Opposite vectors should have score 0.0"
        );
    }

    #[test]
    fn test_normalize_batch() {
        let normalizer = FeatureNormalizer::language_weighted();

        let mut vectors_by_language = HashMap::new();
        vectors_by_language.insert(Language::Python, vec![[0.5f32; 64]]);
        vectors_by_language.insert(Language::TypeScript, vec![[0.5f32; 64]]);
        vectors_by_language.insert(Language::ABL, vec![[0.5f32; 64]]);

        let normalized = normalizer.normalize_batch(&vectors_by_language);

        assert_eq!(normalized.len(), 3);
    }
}
