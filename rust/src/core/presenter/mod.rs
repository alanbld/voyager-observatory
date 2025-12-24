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

    // =========================================================================
    // Voyager Mission Log Format
    // =========================================================================

    /// Format a complete Voyager Mission Log summary.
    ///
    /// This creates the immersive "Observatory" experience with:
    /// - Telescope pointing at project
    /// - Two hemispheres detection (top languages)
    /// - Spectral filter (lens) status
    /// - Fuel gauge (token budget)
    /// - Points of interest
    /// - Transmission status
    pub fn format_mission_log(
        &self,
        project_name: &str,
        hemispheres: (&str, Option<&str>),
        lens: &str,
        confidence: f32,
        tokens_used: usize,
        token_budget: usize,
        poi_count: usize,
        nebula_name: Option<&str>,
    ) -> String {
        let mut output = String::new();

        // Line 1: Observatory pointing
        output.push_str(&format!(
            "{} Observatory pointed at {}.\n",
            self.emoji_formatter.telescope(),
            project_name
        ));

        // Line 2: Two hemispheres
        let hemisphere_str = match hemispheres.1 {
            Some(lang2) => format!("{} | {}", hemispheres.0, lang2),
            None => hemispheres.0.to_string(),
        };
        output.push_str(&format!(
            "{} Two hemispheres detected: {}.\n",
            self.emoji_formatter.notable_star(),
            hemisphere_str
        ));

        // Line 3: Spectral filter
        let confidence_label = if confidence > 0.8 {
            "High Confidence"
        } else if confidence > 0.5 {
            "Medium Confidence"
        } else {
            "Low Confidence"
        };
        output.push_str(&format!(
            "{} Spectral Filter '{}' applied ({}).\n",
            self.emoji_formatter.view_emoji(),
            capitalize_first(lens),
            confidence_label
        ));

        // Line 4: Fuel gauge
        let fuel_pct = if token_budget > 0 {
            (tokens_used as f64 / token_budget as f64 * 100.0) as usize
        } else {
            0
        };
        output.push_str(&format!(
            "{} Fuel: {} / {} tokens ({}%).\n",
            self.emoji_formatter.fuel(),
            format_number(tokens_used),
            format_number(token_budget),
            fuel_pct
        ));

        // Line 5: Points of interest
        if poi_count > 0 {
            let nebula_str = nebula_name.unwrap_or("primary cluster");
            output.push_str(&format!(
                "{} {} Points of Interest identified in the '{}'.\n",
                self.emoji_formatter.gem(),
                poi_count,
                nebula_str
            ));
        }

        // Line 6: Transmission
        output.push_str(&format!(
            "{} Teleporting context sample to LLM base...\n",
            self.emoji_formatter.transmit()
        ));

        output
    }

    /// Detect the two hemispheres (top 2 languages) from a language distribution.
    pub fn detect_hemispheres(languages: &[(String, usize)]) -> (String, Option<String>) {
        let mut sorted: Vec<_> = languages.to_vec();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let primary = sorted.first()
            .map(|(lang, _)| format_language_name(lang))
            .unwrap_or_else(|| "Unknown".to_string());

        let secondary = sorted.get(1)
            .map(|(lang, _)| format_language_name(lang));

        (primary, secondary)
    }
}

/// Format a language name for display.
fn format_language_name(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "rust" => "Logic: Rust".to_string(),
        "python" => "Logic: Python".to_string(),
        "typescript" => "Interface: TypeScript".to_string(),
        "javascript" => "Interface: JavaScript".to_string(),
        "html" | "css" => "Presentation: Web".to_string(),
        "shell" | "bash" => "Automation: Shell".to_string(),
        "go" => "Logic: Go".to_string(),
        "java" => "Logic: Java".to_string(),
        "c" | "cpp" => "Systems: C/C++".to_string(),
        "sql" => "Data: SQL".to_string(),
        "markdown" => "Docs: Markdown".to_string(),
        "json" | "yaml" | "toml" => "Config: Structured".to_string(),
        _ => format!("Code: {}", capitalize_first(lang)),
    }
}

/// Format a number with thousand separators.
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
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

    // =========================================================================
    // Voyager Mission Log Tests (Stage 3)
    // =========================================================================

    #[test]
    fn test_mission_log_contains_telescope_emoji() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "my_project",
            ("Logic: Rust", Some("Interface: TypeScript")),
            "architecture",
            0.85,
            50_000,
            100_000,
            15,
            Some("Core Engine"),
        );

        // Verify telescope emoji at start
        assert!(log.contains("🔭"), "Mission log should contain telescope emoji");
        assert!(log.contains("Observatory pointed at my_project"));
    }

    #[test]
    fn test_mission_log_contains_hemispheres() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Rust", Some("Interface: TypeScript")),
            "debug",
            0.7,
            25_000,
            50_000,
            10,
            None,
        );

        assert!(log.contains("✨"), "Mission log should contain notable star emoji");
        assert!(log.contains("Two hemispheres detected"));
        assert!(log.contains("Logic: Rust | Interface: TypeScript"));
    }

    #[test]
    fn test_mission_log_contains_spectral_filter() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Python", None),
            "security",
            0.9,
            10_000,
            20_000,
            5,
            None,
        );

        assert!(log.contains("🔭"), "Mission log should contain view emoji");
        assert!(log.contains("Spectral Filter 'Security' applied"));
        assert!(log.contains("High Confidence"));
    }

    #[test]
    fn test_mission_log_fuel_gauge() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Go", None),
            "minimal",
            0.5,
            50_000,
            100_000,
            3,
            None,
        );

        assert!(log.contains("🔋"), "Mission log should contain fuel emoji");
        assert!(log.contains("Fuel:"));
        assert!(log.contains("50,000"));
        assert!(log.contains("100,000"));
        assert!(log.contains("50%"));
    }

    #[test]
    fn test_mission_log_points_of_interest() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Java", None),
            "architecture",
            0.8,
            75_000,
            100_000,
            12,
            Some("Service Layer"),
        );

        assert!(log.contains("💎"), "Mission log should contain gem emoji");
        assert!(log.contains("12 Points of Interest"));
        assert!(log.contains("'Service Layer'"));
    }

    #[test]
    fn test_mission_log_transmission() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Automation: Shell", None),
            "auto",
            0.6,
            5_000,
            10_000,
            2,
            None,
        );

        assert!(log.contains("📡"), "Mission log should contain transmit emoji");
        assert!(log.contains("Teleporting context sample to LLM base"));
    }

    #[test]
    fn test_detect_hemispheres_single_language() {
        let languages = vec![("rust".to_string(), 50)];
        let (primary, secondary) = IntelligentPresenter::detect_hemispheres(&languages);

        assert_eq!(primary, "Logic: Rust");
        assert!(secondary.is_none());
    }

    #[test]
    fn test_detect_hemispheres_multiple_languages() {
        let languages = vec![
            ("typescript".to_string(), 30),
            ("python".to_string(), 25),
            ("shell".to_string(), 10),
        ];
        let (primary, secondary) = IntelligentPresenter::detect_hemispheres(&languages);

        assert_eq!(primary, "Interface: TypeScript");
        assert_eq!(secondary, Some("Logic: Python".to_string()));
    }

    #[test]
    fn test_detect_hemispheres_empty() {
        let languages: Vec<(String, usize)> = vec![];
        let (primary, secondary) = IntelligentPresenter::detect_hemispheres(&languages);

        assert_eq!(primary, "Unknown");
        assert!(secondary.is_none());
    }

    #[test]
    fn test_format_language_name_categories() {
        // Logic languages
        assert_eq!(format_language_name("rust"), "Logic: Rust");
        assert_eq!(format_language_name("python"), "Logic: Python");
        assert_eq!(format_language_name("go"), "Logic: Go");
        assert_eq!(format_language_name("java"), "Logic: Java");

        // Interface languages
        assert_eq!(format_language_name("typescript"), "Interface: TypeScript");
        assert_eq!(format_language_name("javascript"), "Interface: JavaScript");

        // Presentation
        assert_eq!(format_language_name("html"), "Presentation: Web");
        assert_eq!(format_language_name("css"), "Presentation: Web");

        // Automation
        assert_eq!(format_language_name("shell"), "Automation: Shell");
        assert_eq!(format_language_name("bash"), "Automation: Shell");

        // Systems
        assert_eq!(format_language_name("c"), "Systems: C/C++");
        assert_eq!(format_language_name("cpp"), "Systems: C/C++");

        // Data
        assert_eq!(format_language_name("sql"), "Data: SQL");

        // Config
        assert_eq!(format_language_name("json"), "Config: Structured");
        assert_eq!(format_language_name("yaml"), "Config: Structured");
        assert_eq!(format_language_name("toml"), "Config: Structured");

        // Docs
        assert_eq!(format_language_name("markdown"), "Docs: Markdown");

        // Unknown
        assert_eq!(format_language_name("cobol"), "Code: Cobol");
    }

    #[test]
    fn test_format_number_with_separators() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(10_000), "10,000");
        assert_eq!(format_number(100_000), "100,000");
        assert_eq!(format_number(1_000_000), "1,000,000");
    }

    #[test]
    fn test_mission_log_no_jargon() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Rust", None),
            "architecture",
            0.9,
            50_000,
            100_000,
            10,
            Some("Core"),
        );

        // Verify no technical jargon in default output
        assert!(!log.contains("Substrate"));
        assert!(!log.contains("EMA"));
        assert!(!log.contains("vectorize"));
        assert!(!log.contains("semantic"));
    }
}
