//! Smart Defaults Module
//!
//! Defines the default settings that the orchestrator can apply.
//! These are the "auto-tuned" settings that make the tool "just work".

use serde::{Deserialize, Serialize};

// =============================================================================
// Semantic Depth
// =============================================================================

/// Semantic analysis depth level.
///
/// Controls how deep the semantic analysis goes:
/// - Quick: Pattern matching only (fastest)
/// - Balanced: Pattern matching + light semantic analysis
/// - Deep: Full cross-language semantic substrate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SemanticDepth {
    /// Fast pattern matching only (10ms)
    Quick,
    /// Balanced analysis with timeout (500ms)
    #[default]
    Balanced,
    /// Full semantic analysis (no timeout)
    Deep,
}

impl SemanticDepth {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "quick" | "fast" | "q" => Some(Self::Quick),
            "balanced" | "normal" | "b" => Some(Self::Balanced),
            "deep" | "full" | "d" => Some(Self::Deep),
            _ => None,
        }
    }

    /// Get timeout in milliseconds for this depth.
    pub fn timeout_ms(&self) -> u64 {
        match self {
            Self::Quick => 10,
            Self::Balanced => 500,
            Self::Deep => 30000, // 30 seconds for deep analysis
        }
    }
}

impl std::fmt::Display for SemanticDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quick => write!(f, "quick"),
            Self::Balanced => write!(f, "balanced"),
            Self::Deep => write!(f, "deep"),
        }
    }
}

impl std::str::FromStr for SemanticDepth {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| {
            format!(
                "Invalid semantic depth: '{}'. Use: quick, balanced, or deep",
                s
            )
        })
    }
}

// =============================================================================
// Detail Level
// =============================================================================

/// Output detail level.
///
/// Controls how much information is shown:
/// - Summary: Just key insights and recommendations
/// - Smart: Progressive disclosure (expand on request)
/// - Detailed: Full technical details
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DetailLevel {
    /// Minimal output with just key insights
    Summary,
    /// Smart progressive disclosure (default)
    #[default]
    Smart,
    /// Full technical details
    Detailed,
}

impl DetailLevel {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "summary" | "minimal" | "s" => Some(Self::Summary),
            "smart" | "normal" | "auto" => Some(Self::Smart),
            "detailed" | "full" | "verbose" | "d" => Some(Self::Detailed),
            _ => None,
        }
    }
}

impl std::fmt::Display for DetailLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Summary => write!(f, "summary"),
            Self::Smart => write!(f, "smart"),
            Self::Detailed => write!(f, "detailed"),
        }
    }
}

impl std::str::FromStr for DetailLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| {
            format!(
                "Invalid detail level: '{}'. Use: summary, smart, or detailed",
                s
            )
        })
    }
}

// =============================================================================
// Smart Defaults
// =============================================================================

/// Smart defaults determined by auto-focus.
#[derive(Debug, Clone)]
pub struct SmartDefaults {
    /// Truncation setting (None = use CLI default, Some(0) = no truncation)
    pub truncate_lines: Option<usize>,
    /// Lens to apply (None = user must specify)
    pub lens: Option<String>,
    /// Semantic analysis depth
    pub semantic_depth: SemanticDepth,
    /// Output detail level
    pub detail_level: DetailLevel,
    /// Estimated output tokens (for budget hints)
    pub estimated_tokens: Option<usize>,
}

impl Default for SmartDefaults {
    fn default() -> Self {
        Self {
            truncate_lines: Some(100),
            lens: Some("architecture".to_string()),
            semantic_depth: SemanticDepth::Balanced,
            detail_level: DetailLevel::Smart,
            estimated_tokens: None,
        }
    }
}

impl SmartDefaults {
    /// Create defaults for a single file (microscope mode).
    pub fn for_file() -> Self {
        Self {
            truncate_lines: Some(0), // No truncation
            lens: Some("architecture".to_string()),
            semantic_depth: SemanticDepth::Deep,
            detail_level: DetailLevel::Detailed,
            estimated_tokens: None,
        }
    }

    /// Create defaults for a directory (wide-angle mode).
    pub fn for_directory() -> Self {
        Self {
            truncate_lines: Some(100),
            lens: Some("architecture".to_string()),
            semantic_depth: SemanticDepth::Balanced,
            detail_level: DetailLevel::Smart,
            estimated_tokens: None,
        }
    }

    /// Create defaults for a large project.
    pub fn for_large_project() -> Self {
        Self {
            truncate_lines: Some(50),
            lens: Some("architecture".to_string()),
            semantic_depth: SemanticDepth::Quick,
            detail_level: DetailLevel::Summary,
            estimated_tokens: None,
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
    // SemanticDepth Tests
    // =========================================================================

    #[test]
    fn test_semantic_depth_parse() {
        assert_eq!(SemanticDepth::parse("quick"), Some(SemanticDepth::Quick));
        assert_eq!(
            SemanticDepth::parse("balanced"),
            Some(SemanticDepth::Balanced)
        );
        assert_eq!(SemanticDepth::parse("deep"), Some(SemanticDepth::Deep));
        assert_eq!(SemanticDepth::parse("invalid"), None);
    }

    #[test]
    fn test_semantic_depth_parse_aliases() {
        // Quick aliases
        assert_eq!(SemanticDepth::parse("fast"), Some(SemanticDepth::Quick));
        assert_eq!(SemanticDepth::parse("q"), Some(SemanticDepth::Quick));
        // Balanced aliases
        assert_eq!(
            SemanticDepth::parse("normal"),
            Some(SemanticDepth::Balanced)
        );
        assert_eq!(SemanticDepth::parse("b"), Some(SemanticDepth::Balanced));
        // Deep aliases
        assert_eq!(SemanticDepth::parse("full"), Some(SemanticDepth::Deep));
        assert_eq!(SemanticDepth::parse("d"), Some(SemanticDepth::Deep));
    }

    #[test]
    fn test_semantic_depth_parse_case_insensitive() {
        assert_eq!(SemanticDepth::parse("QUICK"), Some(SemanticDepth::Quick));
        assert_eq!(
            SemanticDepth::parse("Balanced"),
            Some(SemanticDepth::Balanced)
        );
        assert_eq!(SemanticDepth::parse("DEEP"), Some(SemanticDepth::Deep));
    }

    #[test]
    fn test_semantic_depth_timeout() {
        assert_eq!(SemanticDepth::Quick.timeout_ms(), 10);
        assert_eq!(SemanticDepth::Balanced.timeout_ms(), 500);
        assert_eq!(SemanticDepth::Deep.timeout_ms(), 30000);
    }

    #[test]
    fn test_semantic_depth_default() {
        let depth = SemanticDepth::default();
        assert_eq!(depth, SemanticDepth::Balanced);
    }

    #[test]
    fn test_semantic_depth_display() {
        assert_eq!(format!("{}", SemanticDepth::Quick), "quick");
        assert_eq!(format!("{}", SemanticDepth::Balanced), "balanced");
        assert_eq!(format!("{}", SemanticDepth::Deep), "deep");
    }

    #[test]
    fn test_semantic_depth_from_str() {
        assert_eq!(
            "quick".parse::<SemanticDepth>().unwrap(),
            SemanticDepth::Quick
        );
        assert_eq!(
            "balanced".parse::<SemanticDepth>().unwrap(),
            SemanticDepth::Balanced
        );
        assert_eq!(
            "deep".parse::<SemanticDepth>().unwrap(),
            SemanticDepth::Deep
        );
    }

    #[test]
    fn test_semantic_depth_from_str_error() {
        let result = "invalid".parse::<SemanticDepth>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid semantic depth"));
    }

    #[test]
    fn test_semantic_depth_clone_copy() {
        let depth = SemanticDepth::Deep;
        let cloned = depth;
        assert_eq!(depth, cloned);
    }

    #[test]
    fn test_semantic_depth_serialization() {
        let depth = SemanticDepth::Balanced;
        let json = serde_json::to_string(&depth).unwrap();
        let parsed: SemanticDepth = serde_json::from_str(&json).unwrap();
        assert_eq!(depth, parsed);
    }

    // =========================================================================
    // DetailLevel Tests
    // =========================================================================

    #[test]
    fn test_detail_level_parse() {
        assert_eq!(DetailLevel::parse("summary"), Some(DetailLevel::Summary));
        assert_eq!(DetailLevel::parse("smart"), Some(DetailLevel::Smart));
        assert_eq!(DetailLevel::parse("detailed"), Some(DetailLevel::Detailed));
        assert_eq!(DetailLevel::parse("invalid"), None);
    }

    #[test]
    fn test_detail_level_parse_aliases() {
        // Summary aliases
        assert_eq!(DetailLevel::parse("minimal"), Some(DetailLevel::Summary));
        assert_eq!(DetailLevel::parse("s"), Some(DetailLevel::Summary));
        // Smart aliases
        assert_eq!(DetailLevel::parse("normal"), Some(DetailLevel::Smart));
        assert_eq!(DetailLevel::parse("auto"), Some(DetailLevel::Smart));
        // Detailed aliases
        assert_eq!(DetailLevel::parse("full"), Some(DetailLevel::Detailed));
        assert_eq!(DetailLevel::parse("verbose"), Some(DetailLevel::Detailed));
        assert_eq!(DetailLevel::parse("d"), Some(DetailLevel::Detailed));
    }

    #[test]
    fn test_detail_level_parse_case_insensitive() {
        assert_eq!(DetailLevel::parse("SUMMARY"), Some(DetailLevel::Summary));
        assert_eq!(DetailLevel::parse("Smart"), Some(DetailLevel::Smart));
        assert_eq!(DetailLevel::parse("DETAILED"), Some(DetailLevel::Detailed));
    }

    #[test]
    fn test_detail_level_default() {
        let level = DetailLevel::default();
        assert_eq!(level, DetailLevel::Smart);
    }

    #[test]
    fn test_detail_level_display() {
        assert_eq!(format!("{}", DetailLevel::Summary), "summary");
        assert_eq!(format!("{}", DetailLevel::Smart), "smart");
        assert_eq!(format!("{}", DetailLevel::Detailed), "detailed");
    }

    #[test]
    fn test_detail_level_from_str() {
        assert_eq!(
            "summary".parse::<DetailLevel>().unwrap(),
            DetailLevel::Summary
        );
        assert_eq!("smart".parse::<DetailLevel>().unwrap(), DetailLevel::Smart);
        assert_eq!(
            "detailed".parse::<DetailLevel>().unwrap(),
            DetailLevel::Detailed
        );
    }

    #[test]
    fn test_detail_level_from_str_error() {
        let result = "invalid".parse::<DetailLevel>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid detail level"));
    }

    #[test]
    fn test_detail_level_serialization() {
        let level = DetailLevel::Smart;
        let json = serde_json::to_string(&level).unwrap();
        let parsed: DetailLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }

    // =========================================================================
    // SmartDefaults Tests
    // =========================================================================

    #[test]
    fn test_smart_defaults_default() {
        let defaults = SmartDefaults::default();
        assert_eq!(defaults.truncate_lines, Some(100));
        assert_eq!(defaults.lens, Some("architecture".to_string()));
        assert_eq!(defaults.semantic_depth, SemanticDepth::Balanced);
        assert_eq!(defaults.detail_level, DetailLevel::Smart);
        assert!(defaults.estimated_tokens.is_none());
    }

    #[test]
    fn test_smart_defaults_for_file() {
        let defaults = SmartDefaults::for_file();
        assert_eq!(defaults.truncate_lines, Some(0)); // No truncation
        assert_eq!(defaults.semantic_depth, SemanticDepth::Deep);
        assert_eq!(defaults.detail_level, DetailLevel::Detailed);
    }

    #[test]
    fn test_smart_defaults_for_directory() {
        let defaults = SmartDefaults::for_directory();
        assert_eq!(defaults.truncate_lines, Some(100));
        assert_eq!(defaults.semantic_depth, SemanticDepth::Balanced);
        assert_eq!(defaults.detail_level, DetailLevel::Smart);
    }

    #[test]
    fn test_smart_defaults_for_large_project() {
        let defaults = SmartDefaults::for_large_project();
        assert_eq!(defaults.truncate_lines, Some(50));
        assert_eq!(defaults.semantic_depth, SemanticDepth::Quick);
        assert_eq!(defaults.detail_level, DetailLevel::Summary);
    }

    #[test]
    fn test_smart_defaults_clone() {
        let defaults = SmartDefaults::for_file();
        let cloned = defaults.clone();
        assert_eq!(cloned.truncate_lines, defaults.truncate_lines);
        assert_eq!(cloned.semantic_depth, defaults.semantic_depth);
    }
}
