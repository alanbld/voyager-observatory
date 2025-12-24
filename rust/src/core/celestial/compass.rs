//! Navigation Compass Module
//!
//! Provides navigation suggestions for LLMs exploring the codebase.
//! Uses Nebula brightness and utility data to guide exploration.
//!
//! # Suggestion Types
//!
//! 1. **Brightest Star**: Best entry point for each Nebula
//! 2. **Exploration Hints**: Based on concept distribution
//! 3. **Skim Suggestions**: For low-utility (faded) Nebulae

use super::{Nebula, CelestialMap, Star};

// =============================================================================
// Navigation Suggestion
// =============================================================================

/// A navigation suggestion for LLM exploration.
#[derive(Debug, Clone)]
pub struct NavigationSuggestion {
    /// The nebula this suggestion relates to
    pub nebula_name: String,
    /// The suggested action
    pub action: SuggestionAction,
    /// Target file path (if applicable)
    pub target_path: Option<String>,
    /// Reason for the suggestion
    pub reason: String,
    /// Priority (higher = more important)
    pub priority: u8,
}

impl NavigationSuggestion {
    /// Create a new navigation suggestion.
    pub fn new(
        nebula_name: impl Into<String>,
        action: SuggestionAction,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            nebula_name: nebula_name.into(),
            action,
            target_path: None,
            reason: reason.into(),
            priority: 5,
        }
    }

    /// Set the target path.
    pub fn with_target(mut self, path: impl Into<String>) -> Self {
        self.target_path = Some(path.into());
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Format as display string.
    pub fn display(&self) -> String {
        let icon = match self.action {
            SuggestionAction::StartHere => "🌟",
            SuggestionAction::Explore => "🔍",
            SuggestionAction::Skim => "📖",
            SuggestionAction::Skip => "⏭️",
            SuggestionAction::DeepDive => "🏊",
        };

        let target = self.target_path.as_ref()
            .map(|p| format!(" → {}", p))
            .unwrap_or_default();

        format!("{} {} [{}]{}: {}",
            icon,
            self.action.verb(),
            self.nebula_name,
            target,
            self.reason
        )
    }
}

/// Type of suggested action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionAction {
    /// Start exploration here (brightest star)
    StartHere,
    /// Explore in detail
    Explore,
    /// Skim over (low utility)
    Skim,
    /// Skip entirely
    Skip,
    /// Deep dive (high complexity/importance)
    DeepDive,
}

impl SuggestionAction {
    /// Get the action verb for display.
    pub fn verb(&self) -> &'static str {
        match self {
            SuggestionAction::StartHere => "START",
            SuggestionAction::Explore => "EXPLORE",
            SuggestionAction::Skim => "SKIM",
            SuggestionAction::Skip => "SKIP",
            SuggestionAction::DeepDive => "DEEP DIVE",
        }
    }
}

// =============================================================================
// Exploration Hint
// =============================================================================

/// A hint about what to look for during exploration.
#[derive(Debug, Clone)]
pub struct ExplorationHint {
    /// The hint text
    pub hint: String,
    /// Category of hint
    pub category: HintCategory,
}

impl ExplorationHint {
    /// Create a new exploration hint.
    pub fn new(hint: impl Into<String>, category: HintCategory) -> Self {
        Self {
            hint: hint.into(),
            category,
        }
    }
}

/// Category of exploration hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintCategory {
    /// Architecture-related
    Architecture,
    /// Entry points
    EntryPoints,
    /// Data flow
    DataFlow,
    /// Error handling
    ErrorHandling,
    /// Testing
    Testing,
}

// =============================================================================
// Navigation Compass
// =============================================================================

/// The Navigation Compass generates exploration suggestions.
pub struct NavigationCompass {
    /// Maximum suggestions to generate
    max_suggestions: usize,
    /// Whether to include faded nebulae
    include_faded: bool,
}

impl Default for NavigationCompass {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationCompass {
    /// Create a new navigation compass.
    pub fn new() -> Self {
        Self {
            max_suggestions: 10,
            include_faded: true,
        }
    }

    /// Set maximum suggestions.
    pub fn with_max_suggestions(mut self, max: usize) -> Self {
        self.max_suggestions = max;
        self
    }

    /// Set whether to include faded nebulae.
    pub fn with_faded(mut self, include: bool) -> Self {
        self.include_faded = include;
        self
    }

    /// Generate navigation suggestions from a celestial map.
    pub fn navigate(&self, map: &CelestialMap) -> Vec<NavigationSuggestion> {
        let mut suggestions = Vec::new();

        // Process each nebula
        for nebula in &map.nebulae {
            if let Some(suggestion) = self.suggest_for_nebula(nebula) {
                suggestions.push(suggestion);
            }
        }

        // Handle ungrouped stars if significant
        if !map.ungrouped_stars.is_empty() {
            if let Some(brightest) = self.find_brightest(&map.ungrouped_stars) {
                suggestions.push(NavigationSuggestion::new(
                    "Ungrouped",
                    SuggestionAction::Explore,
                    "Standalone file with high utility",
                )
                .with_target(&brightest.path)
                .with_priority(3));
            }
        }

        // Sort by priority (descending)
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Limit to max suggestions
        suggestions.truncate(self.max_suggestions);

        suggestions
    }

    /// Generate suggestion for a single nebula.
    fn suggest_for_nebula(&self, nebula: &Nebula) -> Option<NavigationSuggestion> {
        // Handle faded nebulae
        if nebula.is_faded {
            if !self.include_faded {
                return None;
            }
            return Some(NavigationSuggestion::new(
                &nebula.name.name,
                SuggestionAction::Skim,
                "Low utility cluster - skim for context only",
            )
            .with_priority(2));
        }

        // Find brightest star
        let brightest = self.find_brightest(&nebula.stars)?;

        // Determine action based on nebula characteristics
        let (action, reason, priority) = if brightest.brightness >= 0.9 {
            (
                SuggestionAction::StartHere,
                format!("Brightest star in {} - excellent entry point", nebula.name.name),
                9,
            )
        } else if nebula.cohesion >= 0.8 {
            (
                SuggestionAction::DeepDive,
                format!("Highly cohesive {} cluster - deep analysis recommended", nebula.name.name),
                7,
            )
        } else if brightest.brightness >= 0.7 {
            (
                SuggestionAction::Explore,
                format!("Good entry point for {}", nebula.name.name),
                6,
            )
        } else {
            (
                SuggestionAction::Explore,
                format!("Entry point for {}", nebula.name.name),
                4,
            )
        };

        Some(NavigationSuggestion::new(&nebula.name.name, action, reason)
            .with_target(&brightest.path)
            .with_priority(priority))
    }

    /// Find the brightest star in a list.
    fn find_brightest<'a>(&self, stars: &'a [Star]) -> Option<&'a Star> {
        stars.iter()
            .filter(|s| s.brightness > 0.0)
            .max_by(|a, b| a.brightness.partial_cmp(&b.brightness).unwrap())
    }

    /// Generate exploration hints from a celestial map.
    pub fn generate_hints(&self, map: &CelestialMap) -> Vec<ExplorationHint> {
        let mut hints = Vec::new();

        // Check for entry points
        let entry_nebulae: Vec<_> = map.nebulae.iter()
            .filter(|n| {
                let name_lower = n.name.name.to_lowercase();
                name_lower.contains("api") ||
                name_lower.contains("handler") ||
                name_lower.contains("endpoint") ||
                name_lower.contains("cli")
            })
            .collect();

        if !entry_nebulae.is_empty() {
            let names: Vec<_> = entry_nebulae.iter()
                .map(|n| n.name.name.as_str())
                .collect();
            hints.push(ExplorationHint::new(
                format!("Entry points found: {}", names.join(", ")),
                HintCategory::EntryPoints,
            ));
        }

        // Check for data flow
        let has_models = map.nebulae.iter().any(|n| {
            let name_lower = n.name.name.to_lowercase();
            name_lower.contains("model") || name_lower.contains("data")
        });

        let has_services = map.nebulae.iter().any(|n| {
            let name_lower = n.name.name.to_lowercase();
            name_lower.contains("service")
        });

        if has_models && has_services {
            hints.push(ExplorationHint::new(
                "Follow data flow: Models → Services → Handlers",
                HintCategory::DataFlow,
            ));
        }

        // Check for test coverage
        let test_nebulae: Vec<_> = map.nebulae.iter()
            .filter(|n| n.name.name.to_lowercase().contains("test"))
            .collect();

        if !test_nebulae.is_empty() {
            let star_count: usize = test_nebulae.iter().map(|n| n.stars.len()).sum();
            hints.push(ExplorationHint::new(
                format!("Test suite found: {} test files", star_count),
                HintCategory::Testing,
            ));
        }

        hints
    }

    /// Format navigation block for display.
    pub fn format_display(&self, map: &CelestialMap) -> String {
        let suggestions = self.navigate(map);
        let hints = self.generate_hints(map);

        if suggestions.is_empty() && hints.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("\n");
        output.push_str("================================================================================\n");
        output.push_str("                         NAVIGATION SUGGESTIONS\n");
        output.push_str("================================================================================\n\n");

        // Navigation suggestions
        if !suggestions.is_empty() {
            output.push_str("RECOMMENDED EXPLORATION PATH:\n\n");

            for (i, suggestion) in suggestions.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, suggestion.display()));
            }
            output.push('\n');
        }

        // Exploration hints
        if !hints.is_empty() {
            output.push_str("EXPLORATION HINTS:\n\n");

            for hint in &hints {
                let icon = match hint.category {
                    HintCategory::Architecture => "🏛️",
                    HintCategory::EntryPoints => "🚪",
                    HintCategory::DataFlow => "🔄",
                    HintCategory::ErrorHandling => "⚠️",
                    HintCategory::Testing => "🧪",
                };
                output.push_str(&format!("  {} {}\n", icon, hint.hint));
            }
            output.push('\n');
        }

        // Summary
        let bright_count = suggestions.iter()
            .filter(|s| s.action == SuggestionAction::StartHere)
            .count();
        let skim_count = suggestions.iter()
            .filter(|s| s.action == SuggestionAction::Skim)
            .count();

        output.push_str(&format!(
            "Summary: {} bright entry points, {} areas to skim\n",
            bright_count, skim_count
        ));

        output
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::celestial::{NebulaName, NamingStrategy};
    use crate::core::fractal::semantic::UniversalConceptType;

    fn create_test_star(path: &str, brightness: f64) -> Star {
        Star {
            path: path.to_string(),
            language: "rust".to_string(),
            brightness,
            concept_type: Some(UniversalConceptType::Service),
            tokens: 100,
            is_brightest: brightness >= 0.9,
        }
    }

    fn create_test_nebula(name: &str, stars: Vec<Star>, is_faded: bool) -> Nebula {
        Nebula {
            id: "0".to_string(),
            name: NebulaName::new(name, NamingStrategy::ConceptBased),
            stars,
            cohesion: 0.8,
            dominant_concept: Some(UniversalConceptType::Service),
            languages: vec!["rust".to_string()],
            is_faded,
        }
    }

    #[test]
    fn test_compass_suggests_brightest_star() {
        let compass = NavigationCompass::new();

        let stars = vec![
            create_test_star("src/dim.rs", 0.5),
            create_test_star("src/bright.rs", 0.95),
            create_test_star("src/medium.rs", 0.7),
        ];

        let nebula = create_test_nebula("Service Layer", stars, false);
        let map = CelestialMap {
            nebulae: vec![nebula],
            ungrouped_stars: vec![],
            total_stars: 3,
            analysis_time_ms: 10,
        };

        let suggestions = compass.navigate(&map);
        assert!(!suggestions.is_empty());

        let first = &suggestions[0];
        assert_eq!(first.action, SuggestionAction::StartHere);
        assert_eq!(first.target_path, Some("src/bright.rs".to_string()));
    }

    #[test]
    fn test_compass_suggests_skim_for_faded() {
        let compass = NavigationCompass::new();

        let stars = vec![create_test_star("src/old.rs", 0.2)];
        let nebula = create_test_nebula("Legacy Code", stars, true);

        let map = CelestialMap {
            nebulae: vec![nebula],
            ungrouped_stars: vec![],
            total_stars: 1,
            analysis_time_ms: 10,
        };

        let suggestions = compass.navigate(&map);
        assert!(!suggestions.is_empty());

        let first = &suggestions[0];
        assert_eq!(first.action, SuggestionAction::Skim);
    }

    #[test]
    fn test_compass_excludes_faded_when_disabled() {
        let compass = NavigationCompass::new().with_faded(false);

        let stars = vec![create_test_star("src/old.rs", 0.2)];
        let nebula = create_test_nebula("Legacy Code", stars, true);

        let map = CelestialMap {
            nebulae: vec![nebula],
            ungrouped_stars: vec![],
            total_stars: 1,
            analysis_time_ms: 10,
        };

        let suggestions = compass.navigate(&map);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_compass_limits_suggestions() {
        let compass = NavigationCompass::new().with_max_suggestions(2);

        let nebulae: Vec<_> = (0..5)
            .map(|i| {
                let stars = vec![create_test_star(&format!("src/file{}.rs", i), 0.8)];
                create_test_nebula(&format!("Nebula {}", i), stars, false)
            })
            .collect();

        let map = CelestialMap {
            nebulae,
            ungrouped_stars: vec![],
            total_stars: 5,
            analysis_time_ms: 10,
        };

        let suggestions = compass.navigate(&map);
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn test_suggestion_display_format() {
        let suggestion = NavigationSuggestion::new(
            "Service Layer",
            SuggestionAction::StartHere,
            "Best entry point",
        )
        .with_target("src/main.rs");

        let display = suggestion.display();
        assert!(display.contains("🌟"));
        assert!(display.contains("START"));
        assert!(display.contains("Service Layer"));
        assert!(display.contains("src/main.rs"));
    }

    #[test]
    fn test_compass_generates_hints() {
        let compass = NavigationCompass::new();

        let api_stars = vec![create_test_star("src/api/handler.rs", 0.9)];
        let test_stars = vec![create_test_star("tests/test_api.rs", 0.7)];

        let map = CelestialMap {
            nebulae: vec![
                create_test_nebula("API Handlers", api_stars, false),
                create_test_nebula("Test Suite", test_stars, false),
            ],
            ungrouped_stars: vec![],
            total_stars: 2,
            analysis_time_ms: 10,
        };

        let hints = compass.generate_hints(&map);
        assert!(!hints.is_empty());

        // Should find entry points and tests
        let categories: Vec<_> = hints.iter().map(|h| h.category).collect();
        assert!(categories.contains(&HintCategory::EntryPoints));
        assert!(categories.contains(&HintCategory::Testing));
    }

    #[test]
    fn test_compass_format_display() {
        let compass = NavigationCompass::new();

        let stars = vec![create_test_star("src/main.rs", 0.95)];
        let nebula = create_test_nebula("Core Engine", stars, false);

        let map = CelestialMap {
            nebulae: vec![nebula],
            ungrouped_stars: vec![],
            total_stars: 1,
            analysis_time_ms: 10,
        };

        let output = compass.format_display(&map);
        assert!(output.contains("NAVIGATION SUGGESTIONS"));
        assert!(output.contains("RECOMMENDED EXPLORATION PATH"));
    }
}
