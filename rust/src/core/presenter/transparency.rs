//! Semantic Transparency Module
//!
//! Provides optional technical details for users who want to understand
//! how the analysis works. Hidden by default, shown with --explain-reasoning.

// =============================================================================
// Semantic Transparency
// =============================================================================

/// Provides optional technical explanations.
pub struct SemanticTransparency {
    /// Whether to show technical details
    show_details: bool,
}

impl Default for SemanticTransparency {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticTransparency {
    /// Create a new transparency handler (details hidden by default).
    pub fn new() -> Self {
        Self { show_details: false }
    }

    /// Enable or disable technical details.
    pub fn with_details(mut self, show: bool) -> Self {
        self.show_details = show;
        self
    }

    /// Format technical details section.
    ///
    /// Returns empty string if details are disabled.
    pub fn format_details(&self, details: &[(&str, &str)]) -> String {
        if !self.show_details || details.is_empty() {
            return String::new();
        }

        let mut output = String::from("\n🔬 Technical Optics:\n");

        for (technique, explanation) in details {
            output.push_str(&format!("  • {}: {}\n", technique, explanation));
        }

        output
    }

    /// Format a cross-language equivalence.
    pub fn format_equivalence(
        &self,
        concept_a: &str,
        language_a: &str,
        concept_b: &str,
        language_b: &str,
        similarity: f32,
    ) -> String {
        if !self.show_details {
            return String::new();
        }

        format!(
            "  • {} ({}) ↔ {} ({}) [similarity: {:.2}]\n",
            concept_a, language_a, concept_b, language_b, similarity
        )
    }

    /// Format a relevance score explanation.
    pub fn format_relevance(&self, symbol: &str, score: f32, factors: &[&str]) -> String {
        if !self.show_details {
            return String::new();
        }

        let factors_str = factors.join(", ");
        format!(
            "  • {} scored {:.2} due to: {}\n",
            symbol, score, factors_str
        )
    }

    /// Format a filtering decision explanation.
    pub fn format_filter_decision(&self, symbol: &str, reason: &str, kept: bool) -> String {
        if !self.show_details {
            return String::new();
        }

        let action = if kept { "KEPT" } else { "FILTERED" };
        format!("  • {} [{}]: {}\n", symbol, action, reason)
    }

    /// Whether technical details are enabled.
    pub fn is_enabled(&self) -> bool {
        self.show_details
    }
}

// =============================================================================
// Jargon Filter
// =============================================================================

/// Filters technical jargon from user-facing output.
pub struct JargonFilter;

impl JargonFilter {
    /// List of technical terms to filter from default output.
    const JARGON: &'static [&'static str] = &[
        "substrate",
        "embedding",
        "vector",
        "cosine",
        "similarity",
        "normalization",
        "semantic",
        "alignment",
        "clustering",
        "heuristic",
        "token",
        "entropy",
        "feature",
        "dimension",
        "inference",
    ];

    /// Check if text contains technical jargon.
    pub fn contains_jargon(text: &str) -> bool {
        let lower = text.to_lowercase();
        Self::JARGON.iter().any(|term| lower.contains(term))
    }

    /// Replace technical terms with user-friendly alternatives.
    pub fn simplify(text: &str) -> String {
        let mut result = text.to_string();

        // Replace common technical terms
        let replacements = [
            ("semantic similarity", "relevance"),
            ("cosine similarity", "similarity"),
            ("feature vector", "analysis"),
            ("embedding", "representation"),
            ("token budget", "size limit"),
            ("heuristic analysis", "pattern matching"),
            ("cross-language alignment", "language connections"),
        ];

        for (from, to) in replacements {
            result = result.replace(from, to);
        }

        result
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transparency_disabled_by_default() {
        let transparency = SemanticTransparency::new();
        assert!(!transparency.is_enabled());
    }

    #[test]
    fn test_transparency_enabled() {
        let transparency = SemanticTransparency::new().with_details(true);
        assert!(transparency.is_enabled());
    }

    #[test]
    fn test_format_details_when_disabled() {
        let transparency = SemanticTransparency::new();
        let output = transparency.format_details(&[("Test", "Value")]);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_details_when_enabled() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_details(&[
            ("Technique 1", "Explanation 1"),
            ("Technique 2", "Explanation 2"),
        ]);

        assert!(output.contains("Technical Optics"));
        assert!(output.contains("Technique 1"));
        assert!(output.contains("Explanation 2"));
    }

    #[test]
    fn test_format_equivalence() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_equivalence(
            "calculate_total",
            "Python",
            "CalculateTotal",
            "TypeScript",
            0.87,
        );

        assert!(output.contains("calculate_total"));
        assert!(output.contains("Python"));
        assert!(output.contains("TypeScript"));
        assert!(output.contains("0.87"));
    }

    #[test]
    fn test_jargon_detection() {
        assert!(JargonFilter::contains_jargon("Using semantic similarity"));
        assert!(JargonFilter::contains_jargon("Cosine distance calculation"));
        assert!(!JargonFilter::contains_jargon("Found 5 relevant functions"));
    }

    #[test]
    fn test_jargon_simplify() {
        let input = "Using semantic similarity to find matches";
        let output = JargonFilter::simplify(input);
        assert!(output.contains("relevance"));
    }

    // =========================================================================
    // SemanticTransparency Extended Tests
    // =========================================================================

    #[test]
    fn test_transparency_default() {
        let transparency = SemanticTransparency::default();
        assert!(!transparency.is_enabled());
    }

    #[test]
    fn test_transparency_with_details_toggle() {
        // Enable then disable
        let transparency = SemanticTransparency::new()
            .with_details(true)
            .with_details(false);
        assert!(!transparency.is_enabled());
    }

    #[test]
    fn test_format_details_empty_array() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_details(&[]);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_details_single_item() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_details(&[("Analysis", "Pattern-based matching")]);

        assert!(output.contains("🔬 Technical Optics:"));
        assert!(output.contains("• Analysis: Pattern-based matching"));
    }

    #[test]
    fn test_format_details_multiple_items() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_details(&[
            ("Lens", "Architecture"),
            ("Budget", "100k tokens"),
            ("Strategy", "Drop low-priority"),
        ]);

        assert!(output.contains("Lens: Architecture"));
        assert!(output.contains("Budget: 100k tokens"));
        assert!(output.contains("Strategy: Drop low-priority"));
    }

    #[test]
    fn test_format_equivalence_disabled() {
        let transparency = SemanticTransparency::new(); // disabled
        let output = transparency.format_equivalence(
            "foo",
            "Rust",
            "bar",
            "Python",
            0.9,
        );
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_equivalence_high_similarity() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_equivalence(
            "process_data",
            "Rust",
            "processData",
            "JavaScript",
            0.95,
        );

        assert!(output.contains("process_data"));
        assert!(output.contains("Rust"));
        assert!(output.contains("processData"));
        assert!(output.contains("JavaScript"));
        assert!(output.contains("0.95"));
        assert!(output.contains("↔"));
    }

    #[test]
    fn test_format_equivalence_low_similarity() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_equivalence(
            "validate",
            "Go",
            "check_input",
            "Python",
            0.45,
        );

        assert!(output.contains("similarity: 0.45"));
    }

    #[test]
    fn test_format_relevance_disabled() {
        let transparency = SemanticTransparency::new(); // disabled
        let output = transparency.format_relevance(
            "main",
            0.9,
            &["entry point", "high call count"],
        );
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_relevance_enabled() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_relevance(
            "calculate_total",
            0.87,
            &["matches intent", "high complexity"],
        );

        assert!(output.contains("calculate_total"));
        assert!(output.contains("0.87"));
        assert!(output.contains("matches intent, high complexity"));
    }

    #[test]
    fn test_format_relevance_single_factor() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_relevance(
            "init",
            0.5,
            &["initialization function"],
        );

        assert!(output.contains("init"));
        assert!(output.contains("0.50"));
        assert!(output.contains("initialization function"));
    }

    #[test]
    fn test_format_relevance_empty_factors() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_relevance(
            "unknown",
            0.1,
            &[],
        );

        assert!(output.contains("unknown"));
        assert!(output.contains("0.10"));
        assert!(output.contains("due to:"));
    }

    #[test]
    fn test_format_filter_decision_disabled() {
        let transparency = SemanticTransparency::new(); // disabled
        let output = transparency.format_filter_decision(
            "test_helper",
            "test file",
            false,
        );
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_filter_decision_kept() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_filter_decision(
            "calculate_total",
            "matches business logic intent",
            true,
        );

        assert!(output.contains("calculate_total"));
        assert!(output.contains("[KEPT]"));
        assert!(output.contains("matches business logic intent"));
    }

    #[test]
    fn test_format_filter_decision_filtered() {
        let transparency = SemanticTransparency::new().with_details(true);
        let output = transparency.format_filter_decision(
            "test_helper",
            "test file excluded by lens",
            false,
        );

        assert!(output.contains("test_helper"));
        assert!(output.contains("[FILTERED]"));
        assert!(output.contains("test file excluded by lens"));
    }

    // =========================================================================
    // JargonFilter Extended Tests
    // =========================================================================

    #[test]
    fn test_jargon_detection_all_terms() {
        // Test all jargon terms
        assert!(JargonFilter::contains_jargon("substrate layer"));
        assert!(JargonFilter::contains_jargon("embedding vectors"));
        assert!(JargonFilter::contains_jargon("vector space"));
        assert!(JargonFilter::contains_jargon("cosine distance"));
        assert!(JargonFilter::contains_jargon("similarity score"));
        assert!(JargonFilter::contains_jargon("normalization step"));
        assert!(JargonFilter::contains_jargon("semantic analysis"));
        assert!(JargonFilter::contains_jargon("alignment algorithm"));
        assert!(JargonFilter::contains_jargon("clustering method"));
        assert!(JargonFilter::contains_jargon("heuristic approach"));
        assert!(JargonFilter::contains_jargon("token count"));
        assert!(JargonFilter::contains_jargon("entropy measure"));
        assert!(JargonFilter::contains_jargon("feature extraction"));
        assert!(JargonFilter::contains_jargon("dimension reduction"));
        assert!(JargonFilter::contains_jargon("inference engine"));
    }

    #[test]
    fn test_jargon_detection_case_insensitive() {
        assert!(JargonFilter::contains_jargon("SEMANTIC analysis"));
        assert!(JargonFilter::contains_jargon("Vector Space"));
        assert!(JargonFilter::contains_jargon("EMBEDDING"));
    }

    #[test]
    fn test_jargon_detection_partial_match() {
        // Jargon can be part of larger words
        assert!(JargonFilter::contains_jargon("tokenization"));
        assert!(JargonFilter::contains_jargon("vectorized"));
        assert!(JargonFilter::contains_jargon("embeddings"));
    }

    #[test]
    fn test_no_jargon_clean_text() {
        assert!(!JargonFilter::contains_jargon("Found 10 functions"));
        assert!(!JargonFilter::contains_jargon("Processing files"));
        assert!(!JargonFilter::contains_jargon("Analysis complete"));
        assert!(!JargonFilter::contains_jargon(""));
    }

    #[test]
    fn test_simplify_semantic_similarity() {
        let input = "Using semantic similarity for matching";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "Using relevance for matching");
    }

    #[test]
    fn test_simplify_cosine_similarity() {
        let input = "cosine similarity score: 0.85";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "similarity score: 0.85");
    }

    #[test]
    fn test_simplify_feature_vector() {
        let input = "Generated feature vector for code";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "Generated analysis for code");
    }

    #[test]
    fn test_simplify_embedding() {
        let input = "Code embedding created";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "Code representation created");
    }

    #[test]
    fn test_simplify_token_budget() {
        let input = "Exceeded token budget";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "Exceeded size limit");
    }

    #[test]
    fn test_simplify_heuristic_analysis() {
        let input = "Applied heuristic analysis";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "Applied pattern matching");
    }

    #[test]
    fn test_simplify_cross_language_alignment() {
        let input = "Using cross-language alignment";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, "Using language connections");
    }

    #[test]
    fn test_simplify_no_change() {
        let input = "Found 5 relevant functions in the codebase";
        let output = JargonFilter::simplify(input);
        assert_eq!(output, input);  // No jargon to replace
    }

    #[test]
    fn test_simplify_multiple_replacements() {
        let input = "Used semantic similarity and token budget limits";
        let output = JargonFilter::simplify(input);
        assert!(output.contains("relevance"));
        assert!(output.contains("size limit"));
    }

    #[test]
    fn test_simplify_empty_string() {
        let output = JargonFilter::simplify("");
        assert_eq!(output, "");
    }

    #[test]
    fn test_simplify_preserves_case() {
        // Replacement terms maintain original case from mapping
        let input = "Using semantic similarity";
        let output = JargonFilter::simplify(input);
        assert!(output.contains("relevance"));
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_transparency_workflow() {
        // Simulate a typical workflow
        let transparency = SemanticTransparency::new().with_details(true);

        // Format multiple details
        let details = transparency.format_details(&[
            ("Lens", "Architecture"),
            ("Intent", "Business Logic"),
        ]);
        assert!(!details.is_empty());

        // Format equivalence
        let equiv = transparency.format_equivalence(
            "getData",
            "JavaScript",
            "get_data",
            "Python",
            0.92,
        );
        assert!(equiv.contains("↔"));

        // Format relevance
        let rel = transparency.format_relevance(
            "main",
            0.95,
            &["entry point", "high connectivity"],
        );
        assert!(rel.contains("scored"));

        // Format filter decision
        let filter = transparency.format_filter_decision(
            "test_util",
            "excluded by architecture lens",
            false,
        );
        assert!(filter.contains("FILTERED"));
    }

    #[test]
    fn test_transparency_disabled_workflow() {
        // All methods return empty when disabled
        let transparency = SemanticTransparency::new(); // disabled

        assert!(transparency.format_details(&[("A", "B")]).is_empty());
        assert!(transparency.format_equivalence("a", "x", "b", "y", 0.5).is_empty());
        assert!(transparency.format_relevance("sym", 0.5, &["factor"]).is_empty());
        assert!(transparency.format_filter_decision("sym", "reason", true).is_empty());
    }
}
