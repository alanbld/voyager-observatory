//! Emoji Formatter Module
//!
//! Provides consistent emoji usage across the CLI output.
//! Emojis serve as visual anchors that guide the eye and convey meaning.

// =============================================================================
// Theme
// =============================================================================

/// Visual theme for emoji output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Full emoji support (default)
    Full,
    /// Minimal emojis (for compatibility)
    Minimal,
    /// No emojis (plain text)
    Plain,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Full
    }
}

// =============================================================================
// Emoji Formatter
// =============================================================================

/// Formats output with consistent emoji usage.
pub struct EmojiFormatter {
    theme: Theme,
}

impl Default for EmojiFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl EmojiFormatter {
    /// Create a new emoji formatter with default theme.
    pub fn new() -> Self {
        Self { theme: Theme::Full }
    }

    /// Create a formatter with a specific theme.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    // =========================================================================
    // Core Emojis
    // =========================================================================

    /// Emoji for intent/goal.
    pub fn intent_emoji(&self, intent: &str) -> &'static str {
        match self.theme {
            Theme::Plain => "",
            _ => match intent.to_lowercase().as_str() {
                "business-logic" | "business" => "💼",
                "debugging" | "debug" => "🔍",
                "onboarding" => "🎓",
                "security" => "🔒",
                "migration" => "🔄",
                _ => "🎯",
            },
        }
    }

    /// Emoji for view/lens.
    pub fn view_emoji(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[VIEW]",
            Theme::Minimal => ">>",
            Theme::Full => "🔭",
        }
    }

    /// Emoji for power/analysis.
    pub fn power_emoji(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[STATS]",
            Theme::Minimal => "**",
            Theme::Full => "🔋",
        }
    }

    /// Emoji for insights.
    pub fn insight_emoji(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[INSIGHTS]",
            Theme::Minimal => "*",
            Theme::Full => "💡",
        }
    }

    /// Emoji for navigation/next steps.
    pub fn navigation_emoji(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[START]",
            Theme::Minimal => "->",
            Theme::Full => "🧭",
        }
    }

    /// Emoji for hints/tips.
    pub fn hint_emoji(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[TIP]",
            Theme::Minimal => "i",
            Theme::Full => "💡",
        }
    }

    /// Emoji for technical details.
    pub fn technical_emoji(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[TECH]",
            Theme::Minimal => "#",
            Theme::Full => "🔬",
        }
    }

    /// Bullet point character.
    pub fn bullet(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "-",
            Theme::Minimal => "*",
            Theme::Full => "•",
        }
    }

    // =========================================================================
    // Confidence Indicators
    // =========================================================================

    /// Confidence indicator emoji.
    pub fn confidence_emoji(&self, confidence: f32) -> &'static str {
        match self.theme {
            Theme::Plain => "",
            _ => {
                if confidence > 0.8 {
                    "🔍"  // High confidence - clear view
                } else if confidence > 0.5 {
                    "⚡"  // Medium confidence - quick scan
                } else {
                    "⚠️"  // Low confidence - uncertain
                }
            }
        }
    }

    /// Confidence indicator with text.
    pub fn confidence_indicator(&self, confidence: f32) -> String {
        let emoji = self.confidence_emoji(confidence);
        let label = if confidence > 0.8 {
            "High Confidence"
        } else if confidence > 0.5 {
            "Medium Confidence"
        } else {
            "Low Confidence"
        };

        match self.theme {
            Theme::Plain => label.to_string(),
            _ => format!("{} {}", emoji, label),
        }
    }

    // =========================================================================
    // Voyager Observatory Emojis (DeepSeek Spectrum)
    // =========================================================================

    /// Telescope emoji for primary entry points.
    pub fn telescope(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[START]",
            Theme::Minimal => ">>",
            Theme::Full => "🔭",
        }
    }

    /// Shooting star for recently explored files.
    pub fn shooting_star(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[RECENT]",
            Theme::Minimal => "*",
            Theme::Full => "🌠",
        }
    }

    /// Dizzy star for TODO/FIXME markers.
    pub fn todo_marker(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[TODO]",
            Theme::Minimal => "!",
            Theme::Full => "💫",
        }
    }

    /// Very bright star (utility >= 0.9).
    pub fn very_bright_star(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[****]",
            Theme::Minimal => "****",
            Theme::Full => "🌟",
        }
    }

    /// Bright star (utility >= 0.8).
    pub fn bright_star(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[***]",
            Theme::Minimal => "***",
            Theme::Full => "⭐",
        }
    }

    /// Notable star (utility >= 0.5).
    pub fn notable_star(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[**]",
            Theme::Minimal => "**",
            Theme::Full => "✨",
        }
    }

    /// Galaxy/nebula indicator.
    pub fn galaxy(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[MAP]",
            Theme::Minimal => "@@",
            Theme::Full => "🌌",
        }
    }

    /// Fuel/token budget indicator.
    pub fn fuel(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[FUEL]",
            Theme::Minimal => "##",
            Theme::Full => "🔋",
        }
    }

    /// Gem/point of interest indicator.
    pub fn gem(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[POI]",
            Theme::Minimal => "<>",
            Theme::Full => "💎",
        }
    }

    /// Transmit/teleport indicator.
    pub fn transmit(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[TX]",
            Theme::Minimal => ">>",
            Theme::Full => "📡",
        }
    }

    /// Brightness indicator based on utility.
    pub fn brightness_indicator(&self, utility: f64) -> &'static str {
        if utility >= 0.9 {
            self.very_bright_star()
        } else if utility >= 0.8 {
            self.bright_star()
        } else if utility >= 0.5 {
            self.notable_star()
        } else {
            match self.theme {
                Theme::Plain => "[*]",
                Theme::Minimal => "*",
                Theme::Full => "·",
            }
        }
    }

    // =========================================================================
    // Status Indicators
    // =========================================================================

    /// Success indicator.
    pub fn success(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[OK]",
            Theme::Minimal => "v",
            Theme::Full => "✅",
        }
    }

    /// Warning indicator.
    pub fn warning(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[WARN]",
            Theme::Minimal => "!",
            Theme::Full => "⚠️",
        }
    }

    /// Error indicator.
    pub fn error(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[ERR]",
            Theme::Minimal => "x",
            Theme::Full => "❌",
        }
    }

    /// Info indicator.
    pub fn info(&self) -> &'static str {
        match self.theme {
            Theme::Plain => "[INFO]",
            Theme::Minimal => "i",
            Theme::Full => "ℹ️",
        }
    }

    // =========================================================================
    // File Type Indicators
    // =========================================================================

    /// File type emoji based on extension.
    pub fn file_type_emoji(&self, extension: &str) -> &'static str {
        match self.theme {
            Theme::Plain => "",
            _ => match extension.to_lowercase().as_str() {
                "rs" => "🦀",
                "py" => "🐍",
                "ts" | "tsx" => "📘",
                "js" | "jsx" => "📜",
                "sh" | "bash" => "🐚",
                "md" => "📝",
                "json" | "yaml" | "yml" => "⚙️",
                "html" => "🌐",
                "css" | "scss" => "🎨",
                "sql" => "🗄️",
                _ => "📄",
            },
        }
    }

    /// Language emoji.
    pub fn language_emoji(&self, language: &str) -> &'static str {
        match self.theme {
            Theme::Plain => "",
            _ => match language.to_lowercase().as_str() {
                "rust" => "🦀",
                "python" => "🐍",
                "typescript" | "javascript" => "📘",
                "shell" | "bash" => "🐚",
                "abl" | "progress" => "🏭",
                _ => "💻",
            },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Theme Tests
    // =========================================================================

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme, Theme::Full);
    }

    #[test]
    fn test_theme_variants() {
        assert_eq!(Theme::Full, Theme::Full);
        assert_eq!(Theme::Minimal, Theme::Minimal);
        assert_eq!(Theme::Plain, Theme::Plain);
        assert_ne!(Theme::Full, Theme::Plain);
    }

    // =========================================================================
    // Formatter Creation Tests
    // =========================================================================

    #[test]
    fn test_formatter_new() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.view_emoji(), "🔭");
    }

    #[test]
    fn test_formatter_default() {
        let formatter = EmojiFormatter::default();
        assert_eq!(formatter.view_emoji(), "🔭");
    }

    #[test]
    fn test_formatter_with_theme() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.view_emoji(), "[VIEW]");
    }

    // =========================================================================
    // Intent Emoji Tests
    // =========================================================================

    #[test]
    fn test_intent_emoji_full() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.intent_emoji("business-logic"), "💼");
        assert_eq!(formatter.intent_emoji("business"), "💼");
        assert_eq!(formatter.intent_emoji("debugging"), "🔍");
        assert_eq!(formatter.intent_emoji("debug"), "🔍");
        assert_eq!(formatter.intent_emoji("onboarding"), "🎓");
        assert_eq!(formatter.intent_emoji("security"), "🔒");
        assert_eq!(formatter.intent_emoji("migration"), "🔄");
        assert_eq!(formatter.intent_emoji("unknown"), "🎯");
    }

    #[test]
    fn test_intent_emoji_plain() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.intent_emoji("business-logic"), "");
        assert_eq!(formatter.intent_emoji("debugging"), "");
        assert_eq!(formatter.intent_emoji("unknown"), "");
    }

    // =========================================================================
    // Core Emoji Tests - All Themes
    // =========================================================================

    #[test]
    fn test_view_emoji_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).view_emoji(), "🔭");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).view_emoji(), ">>");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).view_emoji(), "[VIEW]");
    }

    #[test]
    fn test_power_emoji_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).power_emoji(), "🔋");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).power_emoji(), "**");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).power_emoji(), "[STATS]");
    }

    #[test]
    fn test_insight_emoji_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).insight_emoji(), "💡");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).insight_emoji(), "*");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).insight_emoji(), "[INSIGHTS]");
    }

    #[test]
    fn test_navigation_emoji_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).navigation_emoji(), "🧭");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).navigation_emoji(), "->");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).navigation_emoji(), "[START]");
    }

    #[test]
    fn test_hint_emoji_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).hint_emoji(), "💡");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).hint_emoji(), "i");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).hint_emoji(), "[TIP]");
    }

    #[test]
    fn test_technical_emoji_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).technical_emoji(), "🔬");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).technical_emoji(), "#");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).technical_emoji(), "[TECH]");
    }

    #[test]
    fn test_bullet_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).bullet(), "•");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).bullet(), "*");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).bullet(), "-");
    }

    // =========================================================================
    // Confidence Tests
    // =========================================================================

    #[test]
    fn test_confidence_emoji_full() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.confidence_emoji(0.9), "🔍");
        assert_eq!(formatter.confidence_emoji(0.6), "⚡");
        assert_eq!(formatter.confidence_emoji(0.3), "⚠️");
    }

    #[test]
    fn test_confidence_emoji_plain() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.confidence_emoji(0.9), "");
        assert_eq!(formatter.confidence_emoji(0.6), "");
        assert_eq!(formatter.confidence_emoji(0.3), "");
    }

    #[test]
    fn test_confidence_indicator_all_levels() {
        let formatter = EmojiFormatter::new();
        assert!(formatter.confidence_indicator(0.9).contains("High"));
        assert!(formatter.confidence_indicator(0.6).contains("Medium"));
        assert!(formatter.confidence_indicator(0.3).contains("Low"));
    }

    #[test]
    fn test_confidence_indicator_plain() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.confidence_indicator(0.9), "High Confidence");
        assert_eq!(formatter.confidence_indicator(0.6), "Medium Confidence");
        assert_eq!(formatter.confidence_indicator(0.3), "Low Confidence");
    }

    // =========================================================================
    // Voyager Observatory Emoji Tests
    // =========================================================================

    #[test]
    fn test_telescope_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).telescope(), "🔭");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).telescope(), ">>");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).telescope(), "[START]");
    }

    #[test]
    fn test_shooting_star_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).shooting_star(), "🌠");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).shooting_star(), "*");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).shooting_star(), "[RECENT]");
    }

    #[test]
    fn test_todo_marker_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).todo_marker(), "💫");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).todo_marker(), "!");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).todo_marker(), "[TODO]");
    }

    #[test]
    fn test_stars_all_themes() {
        let full = EmojiFormatter::new().with_theme(Theme::Full);
        let minimal = EmojiFormatter::new().with_theme(Theme::Minimal);
        let plain = EmojiFormatter::new().with_theme(Theme::Plain);

        assert_eq!(full.very_bright_star(), "🌟");
        assert_eq!(minimal.very_bright_star(), "****");
        assert_eq!(plain.very_bright_star(), "[****]");

        assert_eq!(full.bright_star(), "⭐");
        assert_eq!(minimal.bright_star(), "***");
        assert_eq!(plain.bright_star(), "[***]");

        assert_eq!(full.notable_star(), "✨");
        assert_eq!(minimal.notable_star(), "**");
        assert_eq!(plain.notable_star(), "[**]");
    }

    #[test]
    fn test_galaxy_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).galaxy(), "🌌");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).galaxy(), "@@");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).galaxy(), "[MAP]");
    }

    #[test]
    fn test_fuel_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).fuel(), "🔋");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).fuel(), "##");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).fuel(), "[FUEL]");
    }

    #[test]
    fn test_gem_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).gem(), "💎");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).gem(), "<>");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).gem(), "[POI]");
    }

    #[test]
    fn test_transmit_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).transmit(), "📡");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).transmit(), ">>");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).transmit(), "[TX]");
    }

    #[test]
    fn test_brightness_indicator_all_levels() {
        let full = EmojiFormatter::new();
        assert_eq!(full.brightness_indicator(0.95), "🌟");
        assert_eq!(full.brightness_indicator(0.85), "⭐");
        assert_eq!(full.brightness_indicator(0.6), "✨");
        assert_eq!(full.brightness_indicator(0.3), "·");
    }

    #[test]
    fn test_brightness_indicator_plain() {
        let plain = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(plain.brightness_indicator(0.95), "[****]");
        assert_eq!(plain.brightness_indicator(0.85), "[***]");
        assert_eq!(plain.brightness_indicator(0.6), "[**]");
        assert_eq!(plain.brightness_indicator(0.3), "[*]");
    }

    #[test]
    fn test_brightness_indicator_minimal() {
        let minimal = EmojiFormatter::new().with_theme(Theme::Minimal);
        assert_eq!(minimal.brightness_indicator(0.3), "*");
    }

    // =========================================================================
    // Status Indicator Tests
    // =========================================================================

    #[test]
    fn test_success_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).success(), "✅");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).success(), "v");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).success(), "[OK]");
    }

    #[test]
    fn test_warning_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).warning(), "⚠️");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).warning(), "!");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).warning(), "[WARN]");
    }

    #[test]
    fn test_error_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).error(), "❌");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).error(), "x");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).error(), "[ERR]");
    }

    #[test]
    fn test_info_all_themes() {
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Full).info(), "ℹ️");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Minimal).info(), "i");
        assert_eq!(EmojiFormatter::new().with_theme(Theme::Plain).info(), "[INFO]");
    }

    // =========================================================================
    // File Type Emoji Tests
    // =========================================================================

    #[test]
    fn test_file_type_emoji_all_extensions() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.file_type_emoji("rs"), "🦀");
        assert_eq!(formatter.file_type_emoji("py"), "🐍");
        assert_eq!(formatter.file_type_emoji("ts"), "📘");
        assert_eq!(formatter.file_type_emoji("tsx"), "📘");
        assert_eq!(formatter.file_type_emoji("js"), "📜");
        assert_eq!(formatter.file_type_emoji("jsx"), "📜");
        assert_eq!(formatter.file_type_emoji("sh"), "🐚");
        assert_eq!(formatter.file_type_emoji("bash"), "🐚");
        assert_eq!(formatter.file_type_emoji("md"), "📝");
        assert_eq!(formatter.file_type_emoji("json"), "⚙️");
        assert_eq!(formatter.file_type_emoji("yaml"), "⚙️");
        assert_eq!(formatter.file_type_emoji("yml"), "⚙️");
        assert_eq!(formatter.file_type_emoji("html"), "🌐");
        assert_eq!(formatter.file_type_emoji("css"), "🎨");
        assert_eq!(formatter.file_type_emoji("scss"), "🎨");
        assert_eq!(formatter.file_type_emoji("sql"), "🗄️");
        assert_eq!(formatter.file_type_emoji("unknown"), "📄");
    }

    #[test]
    fn test_file_type_emoji_plain() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.file_type_emoji("rs"), "");
        assert_eq!(formatter.file_type_emoji("py"), "");
    }

    #[test]
    fn test_file_type_emoji_case_insensitive() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.file_type_emoji("RS"), "🦀");
        assert_eq!(formatter.file_type_emoji("PY"), "🐍");
        assert_eq!(formatter.file_type_emoji("Json"), "⚙️");
    }

    // =========================================================================
    // Language Emoji Tests
    // =========================================================================

    #[test]
    fn test_language_emoji_all() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.language_emoji("rust"), "🦀");
        assert_eq!(formatter.language_emoji("python"), "🐍");
        assert_eq!(formatter.language_emoji("typescript"), "📘");
        assert_eq!(formatter.language_emoji("javascript"), "📘");
        assert_eq!(formatter.language_emoji("shell"), "🐚");
        assert_eq!(formatter.language_emoji("bash"), "🐚");
        assert_eq!(formatter.language_emoji("abl"), "🏭");
        assert_eq!(formatter.language_emoji("progress"), "🏭");
        assert_eq!(formatter.language_emoji("unknown"), "💻");
    }

    #[test]
    fn test_language_emoji_plain() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.language_emoji("rust"), "");
        assert_eq!(formatter.language_emoji("python"), "");
    }

    #[test]
    fn test_language_emoji_case_insensitive() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.language_emoji("RUST"), "🦀");
        assert_eq!(formatter.language_emoji("Python"), "🐍");
    }
}
