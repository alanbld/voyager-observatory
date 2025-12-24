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
}
