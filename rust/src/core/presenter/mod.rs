//! Intelligent Presenter Module
//!
//! Transforms raw analysis results into delightful, user-friendly output.
//! Uses emojis, progressive disclosure, and semantic transparency.
//!
//! # Design Philosophy
//!
//! - **No jargon by default**: Technical terms hidden unless requested
//! - **Progressive disclosure**: Start simple, reveal details on demand
//! - **Visual hierarchy**: Emojis guide the eye to what matters
//! - **Actionable output**: Always suggest next steps

pub mod emoji_formatter;
pub mod transparency;

pub use emoji_formatter::{EmojiFormatter, Theme};
pub use transparency::SemanticTransparency;

use crate::core::orchestrator::DetailLevel;

// =============================================================================
// Intelligent Presenter
// =============================================================================

/// The intelligent presenter transforms analysis into user-friendly output.
pub struct IntelligentPresenter {
    /// Emoji formatter for visual output
    emoji_formatter: EmojiFormatter,
    /// Semantic transparency for technical details
    transparency: SemanticTransparency,
    /// Current detail level
    detail_level: DetailLevel,
}

impl Default for IntelligentPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentPresenter {
    /// Create a new presenter with default settings.
    pub fn new() -> Self {
        Self {
            emoji_formatter: EmojiFormatter::new(),
            transparency: SemanticTransparency::new(),
            detail_level: DetailLevel::Smart,
        }
    }

    /// Create a presenter with a specific detail level.
    pub fn with_detail_level(mut self, level: DetailLevel) -> Self {
        self.detail_level = level;
        self
    }

    /// Enable semantic transparency (technical details).
    pub fn with_transparency(mut self, enabled: bool) -> Self {
        self.transparency = if enabled {
            SemanticTransparency::new().with_details(true)
        } else {
            SemanticTransparency::new()
        };
        self
    }

    /// Format an exploration summary.
    pub fn format_exploration_summary(
        &self,
        intent: &str,
        file_count: usize,
        language_count: usize,
        analysis_time_ms: u64,
        confidence: f32,
    ) -> String {
        let mut output = String::new();

        // Header with intent
        output.push_str(&format!(
            "{} {} Exploration\n",
            self.emoji_formatter.intent_emoji(intent),
            capitalize_first(intent)
        ));

        // View indicator with confidence
        output.push_str(&format!(
            "{} View: Architecture Lens ({})\n",
            self.emoji_formatter.view_emoji(),
            self.emoji_formatter.confidence_indicator(confidence)
        ));

        // Analysis stats
        let time_str = if analysis_time_ms > 1000 {
            format!("{:.1}s", analysis_time_ms as f64 / 1000.0)
        } else {
            format!("{}ms", analysis_time_ms)
        };

        output.push_str(&format!(
            "{} Analyzed: {} files across {} language{} ({})\n",
            self.emoji_formatter.power_emoji(),
            file_count,
            language_count,
            if language_count == 1 { "" } else { "s" },
            time_str
        ));

        output
    }

    /// Format key insights.
    pub fn format_insights(&self, insights: &[String]) -> String {
        if insights.is_empty() {
            return String::new();
        }

        let mut output = format!("{} Key Insights:\n", self.emoji_formatter.insight_emoji());

        let max_insights = match self.detail_level {
            DetailLevel::Summary => 2,
            DetailLevel::Smart => 3,
            DetailLevel::Detailed => insights.len(),
        };

        for insight in insights.iter().take(max_insights) {
            output.push_str(&format!("  {} {}\n", self.emoji_formatter.bullet(), insight));
        }

        if insights.len() > max_insights {
            output.push_str(&format!(
                "  {} {} more insight{} available with --detail detailed\n",
                self.emoji_formatter.hint_emoji(),
                insights.len() - max_insights,
                if insights.len() - max_insights == 1 { "" } else { "s" }
            ));
        }

        output
    }

    /// Format a starting point recommendation.
    pub fn format_starting_point(&self, symbol: &str, reason: &str) -> String {
        format!(
            "{} Start with: {} - {}\n",
            self.emoji_formatter.navigation_emoji(),
            symbol,
            reason
        )
    }

    /// Format a tip for progressive disclosure.
    pub fn format_tip(&self, tip: &str) -> String {
        format!(
            "{} Tip: {}\n",
            self.emoji_formatter.hint_emoji(),
            tip
        )
    }

    /// Format technical details (only if transparency is enabled).
    pub fn format_technical_details(&self, details: &[(&str, &str)]) -> String {
        self.transparency.format_details(details)
    }

    /// Get the emoji formatter.
    pub fn emoji_formatter(&self) -> &EmojiFormatter {
        &self.emoji_formatter
    }

    /// Get the current detail level.
    pub fn detail_level(&self) -> DetailLevel {
        self.detail_level
    }
}

/// Capitalize the first letter of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presenter_new() {
        let presenter = IntelligentPresenter::new();
        assert_eq!(presenter.detail_level(), DetailLevel::Smart);
    }

    #[test]
    fn test_format_exploration_summary() {
        let presenter = IntelligentPresenter::new();
        let output = presenter.format_exploration_summary(
            "business-logic",
            42,
            3,
            2100,
            0.85,
        );

        assert!(output.contains("Business-logic Exploration"));
        assert!(output.contains("42 files"));
        assert!(output.contains("3 languages"));
        assert!(output.contains("2.1s"));
    }

    #[test]
    fn test_format_insights_limited() {
        let presenter = IntelligentPresenter::new()
            .with_detail_level(DetailLevel::Summary);

        let insights = vec![
            "Insight 1".to_string(),
            "Insight 2".to_string(),
            "Insight 3".to_string(),
            "Insight 4".to_string(),
        ];

        let output = presenter.format_insights(&insights);

        // Summary mode should show only 2 insights
        assert!(output.contains("Insight 1"));
        assert!(output.contains("Insight 2"));
        assert!(!output.contains("Insight 3"));
        assert!(output.contains("2 more insights"));
    }

    #[test]
    fn test_format_starting_point() {
        let presenter = IntelligentPresenter::new();
        let output = presenter.format_starting_point(
            "calculate_total",
            "Core business calculation",
        );

        assert!(output.contains("calculate_total"));
        assert!(output.contains("Core business calculation"));
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }
}
