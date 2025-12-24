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

    #[test]
    fn test_formatter_default_theme() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.view_emoji(), "🔭");
    }

    #[test]
    fn test_formatter_plain_theme() {
        let formatter = EmojiFormatter::new().with_theme(Theme::Plain);
        assert_eq!(formatter.view_emoji(), "[VIEW]");
        assert_eq!(formatter.power_emoji(), "[STATS]");
    }

    #[test]
    fn test_intent_emoji() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.intent_emoji("business-logic"), "💼");
        assert_eq!(formatter.intent_emoji("debugging"), "🔍");
        assert_eq!(formatter.intent_emoji("security"), "🔒");
    }

    #[test]
    fn test_confidence_indicator() {
        let formatter = EmojiFormatter::new();

        let high = formatter.confidence_indicator(0.9);
        assert!(high.contains("High Confidence"));

        let medium = formatter.confidence_indicator(0.6);
        assert!(medium.contains("Medium Confidence"));

        let low = formatter.confidence_indicator(0.3);
        assert!(low.contains("Low Confidence"));
    }

    #[test]
    fn test_file_type_emoji() {
        let formatter = EmojiFormatter::new();
        assert_eq!(formatter.file_type_emoji("rs"), "🦀");
        assert_eq!(formatter.file_type_emoji("py"), "🐍");
        assert_eq!(formatter.file_type_emoji("ts"), "📘");
    }
}
