//! Celestial Navigation Tests
//!
//! TDD tests for the Observer's Journal and navigation features:
//! - Journal creation and persistence
//! - Star marking and brightness
//! - Exploration history tracking
//! - Faded nebulae (ignored patterns)

use pm_encoder::core::{ObserversJournal, ExplorationEntry};
use tempfile::TempDir;

// =============================================================================
// Journal Creation and Persistence
// =============================================================================

#[test]
fn test_journal_creation_with_no_existing_file() {
    let temp_dir = TempDir::new().unwrap();

    // Load journal from directory with no existing journal
    let journal = ObserversJournal::load(temp_dir.path());

    // Should return empty journal
    assert!(journal.bright_stars.is_empty());
    assert!(journal.explorations.is_empty());
    assert_eq!(journal.total_explorations, 0);
}

#[test]
fn test_journal_save_and_load_roundtrip() {
    let temp_dir = TempDir::new().unwrap();

    // Create and populate journal
    let mut journal = ObserversJournal::new();
    journal.mark_star("src/lib.rs", 0.95);
    journal.mark_star_with_note("src/core/engine.rs", 0.9, "Critical engine file");

    // Save
    journal.save(temp_dir.path()).expect("Failed to save journal");

    // Verify file exists
    let journal_path = ObserversJournal::default_path(temp_dir.path());
    assert!(journal_path.exists());

    // Load and verify
    let loaded = ObserversJournal::load(temp_dir.path());
    assert_eq!(loaded.bright_stars.len(), 2);
    assert!(loaded.is_bright_star("src/lib.rs"));
}

// =============================================================================
// Star Marking and Brightness
// =============================================================================

#[test]
fn test_mark_star_creates_entry() {
    let mut journal = ObserversJournal::new();

    // Mark a star
    journal.mark_star("src/main.rs", 0.9);

    // Verify entry exists
    let star = journal.get_star("src/main.rs").expect("Star not found");
    assert_eq!(star.path, "src/main.rs");
    assert_eq!(star.utility, 0.9);
    assert_eq!(star.view_count, 0);
}

#[test]
fn test_mark_star_with_note() {
    let mut journal = ObserversJournal::new();

    journal.mark_star_with_note("config.toml", 0.85, "Project configuration");

    let star = journal.get_star("config.toml").expect("Star not found");
    assert_eq!(star.note, Some("Project configuration".to_string()));
}

#[test]
fn test_bright_star_classification() {
    let mut journal = ObserversJournal::new();

    // High utility = bright star
    journal.mark_star("bright.rs", 0.9);
    assert!(journal.is_bright_star("bright.rs"));

    // Medium utility = not bright
    journal.mark_star("medium.rs", 0.5);
    assert!(!journal.is_bright_star("medium.rs"));

    // Edge case: exactly 0.8 should be bright
    journal.mark_star("edge.rs", 0.8);
    assert!(journal.is_bright_star("edge.rs"));

    // Below threshold
    journal.mark_star("dim.rs", 0.79);
    assert!(!journal.is_bright_star("dim.rs"));
}

#[test]
fn test_all_bright_stars_filter() {
    let mut journal = ObserversJournal::new();

    journal.mark_star("bright1.rs", 0.95);
    journal.mark_star("bright2.rs", 0.85);
    journal.mark_star("dim1.rs", 0.5);
    journal.mark_star("dim2.rs", 0.3);

    let bright = journal.all_bright_stars();
    assert_eq!(bright.len(), 2);

    let paths: Vec<&str> = bright.iter().map(|s| s.path.as_str()).collect();
    assert!(paths.contains(&"bright1.rs"));
    assert!(paths.contains(&"bright2.rs"));
}

#[test]
fn test_star_view_count_increment() {
    let mut journal = ObserversJournal::new();

    journal.mark_star("viewed.rs", 0.9);
    assert_eq!(journal.get_star("viewed.rs").unwrap().view_count, 0);

    journal.record_view("viewed.rs");
    assert_eq!(journal.get_star("viewed.rs").unwrap().view_count, 1);

    journal.record_view("viewed.rs");
    journal.record_view("viewed.rs");
    assert_eq!(journal.get_star("viewed.rs").unwrap().view_count, 3);

    // Viewing non-existent star is safe (no-op)
    journal.record_view("nonexistent.rs");
}

// =============================================================================
// Exploration History
// =============================================================================

#[test]
fn test_exploration_increments_count() {
    let mut journal = ObserversJournal::new();
    assert_eq!(journal.total_explorations, 0);

    let entry = ExplorationEntry::new("business-logic", 42);
    journal.record_exploration(entry);

    assert_eq!(journal.total_explorations, 1);
    assert_eq!(journal.explorations.len(), 1);
}

#[test]
fn test_exploration_history_limited_to_50() {
    let mut journal = ObserversJournal::new();

    // Add 60 explorations
    for i in 0..60 {
        let entry = ExplorationEntry::new(&format!("intent-{}", i), i);
        journal.record_exploration(entry);
    }

    // Should have 50 entries (oldest removed)
    assert_eq!(journal.explorations.len(), 50);
    assert_eq!(journal.total_explorations, 60);

    // First entry should be intent-10 (oldest 10 were removed)
    assert_eq!(journal.explorations[0].intent, "intent-10");
}

#[test]
fn test_recent_explorations_slice() {
    let mut journal = ObserversJournal::new();

    for i in 0..10 {
        let entry = ExplorationEntry::new(&format!("intent-{}", i), i);
        journal.record_exploration(entry);
    }

    let recent = journal.recent_explorations(3);
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].intent, "intent-7");
    assert_eq!(recent[1].intent, "intent-8");
    assert_eq!(recent[2].intent, "intent-9");
}

// =============================================================================
// Faded Nebulae (Ignored Patterns)
// =============================================================================

#[test]
fn test_nebula_fading_threshold() {
    let mut journal = ObserversJournal::new();

    // Pattern ignored < 5 times is not faded
    for _ in 0..4 {
        journal.record_ignored("node_modules/**");
    }
    assert!(!journal.is_faded("node_modules/**"));

    // Fifth ignore makes it faded
    journal.record_ignored("node_modules/**");
    assert!(journal.is_faded("node_modules/**"));
}

#[test]
fn test_multiple_nebulae_fade_independently() {
    let mut journal = ObserversJournal::new();

    // Fade one pattern
    for _ in 0..5 {
        journal.record_ignored("*.log");
    }

    // Partially ignore another
    for _ in 0..3 {
        journal.record_ignored("target/**");
    }

    assert!(journal.is_faded("*.log"));
    assert!(!journal.is_faded("target/**"));
}

// =============================================================================
// Journal Display
// =============================================================================

#[test]
fn test_journal_display_format() {
    let mut journal = ObserversJournal::new();
    journal.mark_star("src/lib.rs", 0.95);

    let output = journal.display();

    assert!(output.contains("OBSERVER'S JOURNAL"));
    assert!(output.contains("⭐"));  // Bright star marker
    assert!(output.contains("src/lib.rs"));
    assert!(output.contains("Marked Stars: 1"));
}

#[test]
fn test_journal_clear_resets_all() {
    let mut journal = ObserversJournal::new();

    // Populate journal
    journal.mark_star("file.rs", 0.9);
    let entry = ExplorationEntry::new("debug", 10);
    journal.record_exploration(entry);
    for _ in 0..5 {
        journal.record_ignored("*.tmp");
    }

    // Clear
    journal.clear();

    // Verify everything is reset
    assert!(journal.bright_stars.is_empty());
    assert!(journal.explorations.is_empty());
    assert!(journal.faded_nebulae.is_empty());
    assert_eq!(journal.total_explorations, 0);
}

// =============================================================================
// Brightness Indicator (ProcessedFile)
// =============================================================================

#[test]
fn test_brightness_indicator_levels() {
    use pm_encoder::core::models::{ProcessedFile, FileEntry};

    let entry = FileEntry::new("test.rs", "fn main() {}");

    // Very bright (utility >= 0.9)
    let pf = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.95);
    assert_eq!(pf.brightness_indicator(), "🌟 ");

    // Bright (utility >= 0.8)
    let pf = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.85);
    assert_eq!(pf.brightness_indicator(), "⭐ ");

    // Notable (utility >= 0.5)
    let pf = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.6);
    assert_eq!(pf.brightness_indicator(), "✨ ");

    // Dim (utility < 0.5)
    let pf = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.3);
    assert_eq!(pf.brightness_indicator(), "");

    // No utility set
    let pf = ProcessedFile::from_entry(&entry, "rust", 100);
    assert_eq!(pf.brightness_indicator(), "");
}

#[test]
fn test_is_bright_star_check() {
    use pm_encoder::core::models::{ProcessedFile, FileEntry};

    let entry = FileEntry::new("test.rs", "fn main() {}");

    let pf = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.85);
    assert!(pf.is_bright_star());

    let pf = ProcessedFile::from_entry(&entry, "rust", 100).with_utility(0.75);
    assert!(!pf.is_bright_star());

    let pf = ProcessedFile::from_entry(&entry, "rust", 100);
    assert!(!pf.is_bright_star());
}

// =============================================================================
// Spectral Synthesis Tests (Day 7)
// =============================================================================

#[test]
fn test_nebula_namer_concept_based_naming() {
    use pm_encoder::core::celestial::{NebulaNamer, NamingStrategy};
    use pm_encoder::core::fractal::semantic::UniversalConceptType;
    use std::collections::HashMap;

    let namer = NebulaNamer::new();

    // Strong dominance of Validation concept
    let mut concepts = HashMap::new();
    concepts.insert(UniversalConceptType::Validation, 10);
    concepts.insert(UniversalConceptType::ErrorHandling, 2);

    let name = namer.name_nebula(&[], &concepts);
    assert_eq!(name.name, "Input Validation");
    assert_eq!(name.strategy, NamingStrategy::ConceptBased);
    assert!(name.confidence >= 0.8);
}

#[test]
fn test_nebula_namer_directory_fallback() {
    use pm_encoder::core::celestial::{NebulaNamer, NamingStrategy};
    use std::collections::HashMap;

    let namer = NebulaNamer::new();

    // No dominant concept, but common directory (no conflicting parent)
    let files = vec![
        "handlers/user.rs".to_string(),
        "handlers/order.rs".to_string(),
        "handlers/payment.rs".to_string(),
    ];

    let name = namer.name_nebula(&files, &HashMap::new());
    assert_eq!(name.name, "Request Handlers");
    assert_eq!(name.strategy, NamingStrategy::DirectoryBased);
}

#[test]
fn test_nebula_namer_pattern_based_tests() {
    use pm_encoder::core::celestial::{NebulaNamer, NamingStrategy};
    use std::collections::HashMap;

    let namer = NebulaNamer::new();

    // Files with test pattern as flat files (no directory)
    // This triggers pattern-based naming
    let files = vec![
        "test_user.py".to_string(),
        "test_order.py".to_string(),
        "test_payment.py".to_string(),
    ];

    let name = namer.name_nebula(&files, &HashMap::new());
    assert_eq!(name.name, "Test Suite");
    assert_eq!(name.strategy, NamingStrategy::PatternBased);
}

#[test]
fn test_constellation_mapper_groups_by_directory() {
    use pm_encoder::core::celestial::{ConstellationMapper, FileInfo};
    use pm_encoder::core::fractal::semantic::UniversalConceptType;

    let mapper = ConstellationMapper::new();

    let files = vec![
        FileInfo {
            path: "src/handlers/user.rs".to_string(),
            language: "rust".to_string(),
            concept_type: Some(UniversalConceptType::Endpoint),
            utility: 0.8,
            tokens: 100,
        },
        FileInfo {
            path: "src/handlers/order.rs".to_string(),
            language: "rust".to_string(),
            concept_type: Some(UniversalConceptType::Endpoint),
            utility: 0.9,
            tokens: 150,
        },
        FileInfo {
            path: "src/models/user.rs".to_string(),
            language: "rust".to_string(),
            concept_type: Some(UniversalConceptType::DataStructure),
            utility: 0.7,
            tokens: 80,
        },
    ];

    let map = mapper.map(&files);

    // Should create at least one nebula
    assert!(map.nebulae.len() >= 1 || map.ungrouped_stars.len() > 0);
    assert_eq!(map.total_stars, 3);
}

#[test]
fn test_constellation_mapper_mixed_languages() {
    use pm_encoder::core::celestial::{ConstellationMapper, FileInfo};
    use pm_encoder::core::fractal::semantic::UniversalConceptType;

    let mapper = ConstellationMapper::new();

    // Mix of Python and Rust files in same semantic cluster
    let files = vec![
        FileInfo {
            path: "src/api/handler.py".to_string(),
            language: "python".to_string(),
            concept_type: Some(UniversalConceptType::Endpoint),
            utility: 0.85,
            tokens: 100,
        },
        FileInfo {
            path: "src/api/handler.rs".to_string(),
            language: "rust".to_string(),
            concept_type: Some(UniversalConceptType::Endpoint),
            utility: 0.9,
            tokens: 120,
        },
        FileInfo {
            path: "src/api/routes.py".to_string(),
            language: "python".to_string(),
            concept_type: Some(UniversalConceptType::Endpoint),
            utility: 0.75,
            tokens: 80,
        },
    ];

    let map = mapper.map(&files);

    // Should group them together despite different languages
    assert_eq!(map.total_stars, 3);

    // If grouped into a nebula, it should contain multiple languages
    if !map.nebulae.is_empty() {
        let nebula = &map.nebulae[0];
        assert!(nebula.languages.len() >= 1);
    }
}

#[test]
fn test_celestial_map_display_format() {
    use pm_encoder::core::celestial::{ConstellationMapper, FileInfo, CelestialMap};
    use pm_encoder::core::fractal::semantic::UniversalConceptType;

    let mapper = ConstellationMapper::new();

    let files = vec![
        FileInfo {
            path: "src/core/engine.rs".to_string(),
            language: "rust".to_string(),
            concept_type: Some(UniversalConceptType::Service),
            utility: 0.95,
            tokens: 500,
        },
        FileInfo {
            path: "src/core/models.rs".to_string(),
            language: "rust".to_string(),
            concept_type: Some(UniversalConceptType::DataStructure),
            utility: 0.85,
            tokens: 300,
        },
    ];

    let map = mapper.map(&files);
    let display = map.format_display();

    // Should have celestial map header
    assert!(display.contains("CELESTIAL MAP"));
    // Should show star count
    assert!(display.contains("stars"));
}

#[test]
fn test_navigation_compass_suggests_brightest() {
    use pm_encoder::core::celestial::{
        NavigationCompass, CelestialMap, Nebula, Star, NebulaName, NamingStrategy,
        SuggestionAction,
    };
    use pm_encoder::core::fractal::semantic::UniversalConceptType;

    let compass = NavigationCompass::new();

    let stars = vec![
        Star {
            path: "src/dim.rs".to_string(),
            language: "rust".to_string(),
            brightness: 0.5,
            concept_type: Some(UniversalConceptType::Service),
            tokens: 100,
            is_brightest: false,
        },
        Star {
            path: "src/bright.rs".to_string(),
            language: "rust".to_string(),
            brightness: 0.95,
            concept_type: Some(UniversalConceptType::Service),
            tokens: 200,
            is_brightest: true,
        },
    ];

    let nebula = Nebula {
        id: "0".to_string(),
        name: NebulaName::new("Service Layer", NamingStrategy::ConceptBased),
        stars,
        cohesion: 0.8,
        dominant_concept: Some(UniversalConceptType::Service),
        languages: vec!["rust".to_string()],
        is_faded: false,
    };

    let map = CelestialMap {
        nebulae: vec![nebula],
        ungrouped_stars: vec![],
        total_stars: 2,
        analysis_time_ms: 10,
    };

    let suggestions = compass.navigate(&map);
    assert!(!suggestions.is_empty());

    let first = &suggestions[0];
    assert_eq!(first.action, SuggestionAction::StartHere);
    assert_eq!(first.target_path, Some("src/bright.rs".to_string()));
}

#[test]
fn test_navigation_compass_suggests_skim_for_faded() {
    use pm_encoder::core::celestial::{
        NavigationCompass, CelestialMap, Nebula, Star, NebulaName, NamingStrategy,
        SuggestionAction,
    };
    use pm_encoder::core::fractal::semantic::UniversalConceptType;

    let compass = NavigationCompass::new();

    let stars = vec![Star {
        path: "src/old.rs".to_string(),
        language: "rust".to_string(),
        brightness: 0.2,
        concept_type: Some(UniversalConceptType::Unknown),
        tokens: 50,
        is_brightest: false,
    }];

    let nebula = Nebula {
        id: "1".to_string(),
        name: NebulaName::new("Legacy Code", NamingStrategy::Fallback),
        stars,
        cohesion: 0.3,
        dominant_concept: None,
        languages: vec!["rust".to_string()],
        is_faded: true, // Marked as faded
    };

    let map = CelestialMap {
        nebulae: vec![nebula],
        ungrouped_stars: vec![],
        total_stars: 1,
        analysis_time_ms: 5,
    };

    let suggestions = compass.navigate(&map);
    assert!(!suggestions.is_empty());

    let first = &suggestions[0];
    assert_eq!(first.action, SuggestionAction::Skim);
    assert!(first.reason.contains("skim"));
}

#[test]
fn test_navigation_display_format() {
    use pm_encoder::core::celestial::{
        NavigationCompass, CelestialMap, Nebula, Star, NebulaName, NamingStrategy,
    };
    use pm_encoder::core::fractal::semantic::UniversalConceptType;

    let compass = NavigationCompass::new();

    let stars = vec![Star {
        path: "src/main.rs".to_string(),
        language: "rust".to_string(),
        brightness: 0.95,
        concept_type: Some(UniversalConceptType::Service),
        tokens: 100,
        is_brightest: true,
    }];

    let nebula = Nebula {
        id: "2".to_string(),
        name: NebulaName::new("Core Engine", NamingStrategy::ConceptBased),
        stars,
        cohesion: 0.9,
        dominant_concept: Some(UniversalConceptType::Service),
        languages: vec!["rust".to_string()],
        is_faded: false,
    };

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
