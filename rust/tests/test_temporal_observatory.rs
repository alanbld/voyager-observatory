//! Temporal Observatory Tests (Chronos Engine)
//!
//! TDD tests for the Chronos Engine using mock Git repositories.
//! Uses celestial terminology: "Observations" (commits), "Observers" (authors),
//! "Stellar Age" (file age), "Volcanic Churn" (recent activity).

#[cfg(feature = "temporal")]
mod temporal_tests {
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    use pm_encoder::core::temporal::{
        ChronosEngine, ChronosState, TemporalCensus,
        AgeClassification, ChurnClassification,
    };
    use pm_encoder::core::temporal::{
        GeologicalAnalyzer, GeologicalActivity,
    };

    // =========================================================================
    // Test Helpers: Mock Repository Creation
    // =========================================================================

    /// Create a mock Git repository with specified files and observations (commits).
    fn create_mock_repository() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path();

        // Initialize git repository
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .expect("Failed to init git repository");

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "observer@galaxy.test"])
            .current_dir(path)
            .output()
            .expect("Failed to configure git email");

        Command::new("git")
            .args(["config", "user.name", "Test Observer"])
            .current_dir(path)
            .output()
            .expect("Failed to configure git user");

        temp_dir
    }

    /// Add a file and create an observation (commit).
    fn add_observation(dir: &PathBuf, filename: &str, content: &str, message: &str) {
        // Write file
        let file_path = dir.join(filename);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&file_path, content).expect("Failed to write file");

        // Stage and commit
        Command::new("git")
            .args(["add", filename])
            .current_dir(dir)
            .output()
            .expect("Failed to add file");

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(dir)
            .output()
            .expect("Failed to commit");
    }

    /// Add multiple observations to a file to simulate churn.
    fn add_multiple_observations(dir: &PathBuf, filename: &str, count: usize) {
        for i in 0..count {
            let content = format!("// Observation {}\nfn main() {{ println!(\"v{}\"); }}", i, i);
            let message = format!("Observation {} on {}", i, filename);
            add_observation(dir, filename, &content, &message);
        }
    }

    // =========================================================================
    // ChronosEngine Initialization Tests
    // =========================================================================

    #[test]
    fn test_chronos_engine_initializes_with_git_repo() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        // Add at least one observation
        add_observation(&path, "main.rs", "fn main() {}", "Initial observation");

        let engine = ChronosEngine::new(temp_dir.path());
        assert!(engine.is_some(), "ChronosEngine should initialize with valid git repo");
    }

    #[test]
    fn test_chronos_engine_returns_none_without_git() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        // No git init - just an empty directory

        let engine = ChronosEngine::new(temp_dir.path());
        assert!(engine.is_none(), "ChronosEngine should return None without .git");
    }

    #[test]
    fn test_static_galaxy_fallback() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Without git, we cannot create an engine
        let engine = ChronosEngine::new(temp_dir.path());
        assert!(engine.is_none(), "Should return None for non-git directory");

        // The Static Galaxy state is used when temporal feature is disabled
        // or when ChronosEngine cannot be created
    }

    // =========================================================================
    // History Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_history_counts_observations() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        // Create 5 observations
        for i in 0..5 {
            add_observation(
                &path,
                &format!("file{}.rs", i),
                &format!("fn func{}() {{}}", i),
                &format!("Observation {}", i),
            );
        }

        let mut engine = ChronosEngine::new(temp_dir.path()).unwrap();
        engine.extract_history().expect("History extraction should succeed");

        let census = engine.build_census();
        assert_eq!(census.total_observations, 5, "Should count all observations");
    }

    #[test]
    fn test_extract_history_identifies_observers() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        add_observation(&path, "main.rs", "fn main() {}", "Initial observation");

        let mut engine = ChronosEngine::new(temp_dir.path()).unwrap();
        engine.extract_history().expect("History extraction should succeed");

        let census = engine.build_census();
        assert!(census.observer_count >= 1, "Should identify at least one observer");
    }

    // =========================================================================
    // Stellar Age Tests
    // =========================================================================

    #[test]
    fn test_stellar_age_detection() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        // Create initial observation
        add_observation(&path, "ancient.rs", "fn ancient() {}", "Big bang");

        let mut engine = ChronosEngine::new(temp_dir.path()).unwrap();
        engine.extract_history().expect("History extraction should succeed");

        let census = engine.build_census();

        // Galaxy age should be at least 0 (created just now)
        assert!(census.galaxy_age_days >= 0, "Galaxy age should be non-negative");
    }

    #[test]
    fn test_age_classification_newborn() {
        // Files created < 30 days ago should be Newborn
        let age = AgeClassification::from_days(15);
        assert!(matches!(age, AgeClassification::Newborn));
    }

    #[test]
    fn test_age_classification_young() {
        // Files created 30-365 days ago should be Young
        let age = AgeClassification::from_days(180);
        assert!(matches!(age, AgeClassification::Young));
    }

    #[test]
    fn test_age_classification_mature() {
        // Files created 1-2 years ago should be Mature
        let age = AgeClassification::from_days(500);
        assert!(matches!(age, AgeClassification::Mature));
    }

    #[test]
    fn test_age_classification_ancient() {
        // Files created > 2 years ago should be Ancient
        let age = AgeClassification::from_days(800);
        assert!(matches!(age, AgeClassification::Ancient));
    }

    // =========================================================================
    // Volcanic Churn Tests
    // =========================================================================

    #[test]
    fn test_churn_classification_dormant() {
        // 0 observations in 90 days = Dormant
        let churn = ChurnClassification::from_counts(0, 0);
        assert!(matches!(churn, ChurnClassification::Dormant));
    }

    #[test]
    fn test_churn_classification_low() {
        // 1-3 observations in 90 days = Low
        let churn = ChurnClassification::from_counts(1, 2);
        assert!(matches!(churn, ChurnClassification::Low));
    }

    #[test]
    fn test_churn_classification_moderate() {
        // 4-10 observations in 90 days = Moderate
        let churn = ChurnClassification::from_counts(3, 7);
        assert!(matches!(churn, ChurnClassification::Moderate));
    }

    #[test]
    fn test_churn_classification_high() {
        // 11-30 observations in 90 days = High
        let churn = ChurnClassification::from_counts(8, 20);
        assert!(matches!(churn, ChurnClassification::High));
    }

    #[test]
    fn test_churn_classification_supernova() {
        // > 30 observations in 30 days = Supernova
        let churn = ChurnClassification::from_counts(35, 50);
        assert!(matches!(churn, ChurnClassification::Supernova));
    }

    // =========================================================================
    // Geological Analysis Tests
    // =========================================================================

    #[test]
    fn test_geological_analyzer_default_thresholds() {
        let analyzer = GeologicalAnalyzer::new();

        assert_eq!(analyzer.tectonic_churn, 10, "Default tectonic churn threshold");
        assert!((analyzer.tectonic_dark_matter - 0.20).abs() < 0.01, "Default dark matter threshold");
        assert_eq!(analyzer.ancient_dormant_days, 730, "Default ancient threshold (2 years)");
        assert_eq!(analyzer.supernova_threshold, 30, "Default supernova threshold");
    }

    #[test]
    fn test_geological_activity_stable() {
        assert_eq!(
            GeologicalActivity::Stable.description(),
            "Stable geology"
        );
    }

    #[test]
    fn test_geological_activity_volcanic() {
        assert_eq!(
            GeologicalActivity::HighVolcanic.indicator(),
            "🔥"
        );
    }

    // =========================================================================
    // Temporal Census Tests
    // =========================================================================

    #[test]
    fn test_temporal_census_state_active() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        add_observation(&path, "main.rs", "fn main() {}", "Initial");

        let mut engine = ChronosEngine::new(temp_dir.path()).unwrap();
        engine.extract_history().expect("History extraction should succeed");

        let state = engine.state();
        assert!(
            matches!(state, ChronosState::Active { .. }),
            "State should be Active after successful extraction"
        );
    }

    #[test]
    fn test_temporal_census_builds_constellations() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        // Create files in different directories (constellations)
        std::fs::create_dir_all(path.join("src")).ok();
        std::fs::create_dir_all(path.join("tests")).ok();

        add_observation(&path, "src/lib.rs", "pub fn lib() {}", "Add lib");
        add_observation(&path, "tests/test.rs", "#[test] fn test() {}", "Add test");

        let mut engine = ChronosEngine::new(temp_dir.path()).unwrap();
        engine.extract_history().expect("History extraction should succeed");

        let census = engine.build_census();

        // Should have file entries
        assert!(!census.files.is_empty(), "Should have file churn data");
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_full_temporal_analysis_workflow() {
        let temp_dir = create_mock_repository();
        let path = temp_dir.path().to_path_buf();

        // Create a project structure
        std::fs::create_dir_all(path.join("src/core")).ok();

        // Add initial files
        add_observation(&path, "src/main.rs", "fn main() { core::run(); }", "Initial main");
        add_observation(&path, "src/core/mod.rs", "pub fn run() {}", "Add core module");

        // Add more observations to main.rs to simulate churn
        for i in 0..5 {
            add_observation(
                &path,
                "src/main.rs",
                &format!("fn main() {{ core::run(); /* v{} */ }}", i),
                &format!("Update main v{}", i),
            );
        }

        // Create and run engine
        let mut engine = ChronosEngine::new(temp_dir.path()).unwrap();
        engine.extract_history().expect("History extraction should succeed");

        let census = engine.build_census();

        // Verify basic metrics
        assert!(census.total_observations >= 7, "Should count all observations");
        assert!(census.observer_count >= 1, "Should have observers");
        assert!(census.galaxy_age_days >= 0, "Should have valid age");

        // Verify file tracking
        assert!(!census.files.is_empty(), "Should track files");
    }

    #[test]
    fn test_chronos_no_jargon_in_state_description() {
        use pm_encoder::core::temporal::temporal_state_description;

        let active_state = ChronosState::Active {
            total_events: 100,
            galaxy_age_days: 365,
            observer_count: 5,
        };

        let description = temporal_state_description(&active_state);

        // Should use celestial terminology, not git jargon
        assert!(!description.contains("git"), "Should not contain 'git'");
        assert!(!description.contains("commit"), "Should not contain 'commit'");
        assert!(!description.contains("blame"), "Should not contain 'blame'");

        assert!(description.contains("Chronos"), "Should use Chronos terminology");
    }

    #[test]
    fn test_static_galaxy_description() {
        use pm_encoder::core::temporal::temporal_state_description;

        let static_state = ChronosState::StaticGalaxy;
        let description = temporal_state_description(&static_state);

        assert!(description.contains("Static Galaxy"), "Should describe static galaxy");
    }
}

// =========================================================================
// Non-Temporal Feature Tests (Fallback Behavior)
// =========================================================================

#[cfg(not(feature = "temporal"))]
mod non_temporal_tests {
    use pm_encoder::core::temporal::{ChronosState, is_temporal_available};

    #[test]
    fn test_temporal_not_available() {
        assert!(!is_temporal_available(), "Temporal should not be available without feature");
    }
}

// =========================================================================
// Common Tests (Both Features)
// =========================================================================

mod common_tests {
    use pm_encoder::core::temporal::is_temporal_available;

    #[test]
    fn test_temporal_availability_function_exists() {
        // This test verifies the function exists and returns a bool
        let _available: bool = is_temporal_available();
    }
}
