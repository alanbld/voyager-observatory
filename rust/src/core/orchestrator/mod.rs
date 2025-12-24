//! Smart Orchestrator Module
//!
//! Provides intelligent defaults and silent fallback logic for the Fractal Telescope.
//! This module makes `pm_encoder` "just work" by analyzing input and choosing
//! optimal settings automatically.
//!
//! # Architecture
//!
//! ```text
//! User Input → AutoFocus → SmartDefaults → Analysis Strategy → Result
//!                 │              │                │
//!                 ▼              ▼                ▼
//!            Path Analysis   Lens/Depth      Fallback System
//! ```

pub mod auto_focus;
pub mod fallback;
pub mod journal;
pub mod smart_defaults;

pub use auto_focus::{AutoFocus, InputType};
pub use fallback::{AnalysisStrategy, FallbackSystem};
pub use journal::{ObserversJournal, MarkedStar, ExplorationEntry, FadedNebula};
pub use smart_defaults::{SmartDefaults, SemanticDepth, DetailLevel};

use std::path::Path;
use std::time::Duration;

use crate::core::EncoderConfig;

// =============================================================================
// Smart Orchestrator
// =============================================================================

/// The Smart Orchestrator coordinates analysis with intelligent defaults.
///
/// It analyzes the input path, determines optimal settings, executes analysis
/// with timeout-based fallbacks, and produces user-friendly output.
pub struct SmartOrchestrator {
    /// Auto-focus logic for path analysis
    auto_focus: AutoFocus,
    /// Fallback system for graceful degradation
    fallback: FallbackSystem,
    /// Semantic analysis timeout
    semantic_timeout: Duration,
}

impl Default for SmartOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartOrchestrator {
    /// Create a new orchestrator with default settings.
    pub fn new() -> Self {
        Self {
            auto_focus: AutoFocus::new(),
            fallback: FallbackSystem::new(),
            semantic_timeout: Duration::from_millis(500),
        }
    }

    /// Create an orchestrator with a custom semantic timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.semantic_timeout = timeout;
        self
    }

    /// Analyze a path and return smart defaults.
    ///
    /// This is the main entry point for auto-configuration.
    pub fn analyze_path(&self, path: &Path) -> SmartDefaults {
        self.auto_focus.analyze(path)
    }

    /// Apply smart defaults to an encoder config.
    ///
    /// Only applies defaults for options not explicitly set by the user.
    pub fn apply_defaults(&self, config: &mut EncoderConfig, defaults: &SmartDefaults) {
        // Apply truncation default if not explicitly set
        if config.truncate_lines == 0 {
            config.truncate_lines = defaults.truncate_lines.unwrap_or(0);
        }

        // Apply lens if not set
        if config.active_lens.is_none() {
            config.active_lens = defaults.lens.clone();
        }
    }

    /// Get the fallback system for error handling.
    pub fn fallback(&self) -> &FallbackSystem {
        &self.fallback
    }

    /// Get the semantic timeout.
    pub fn semantic_timeout(&self) -> Duration {
        self.semantic_timeout
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_orchestrator_new() {
        let orchestrator = SmartOrchestrator::new();
        assert_eq!(orchestrator.semantic_timeout(), Duration::from_millis(500));
    }

    #[test]
    fn test_orchestrator_with_timeout() {
        let orchestrator = SmartOrchestrator::new()
            .with_timeout(Duration::from_secs(1));
        assert_eq!(orchestrator.semantic_timeout(), Duration::from_secs(1));
    }

    #[test]
    fn test_analyze_file() {
        // Create a temporary file to test
        let dir = std::env::temp_dir();
        let file = dir.join("test_analyze_file.rs");
        std::fs::write(&file, "fn main() {}").unwrap();

        let orchestrator = SmartOrchestrator::new();
        let defaults = orchestrator.analyze_path(&file);

        // File should get microscope mode (no truncation)
        assert_eq!(defaults.truncate_lines, Some(0));

        // Cleanup
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn test_analyze_directory() {
        let orchestrator = SmartOrchestrator::new();
        let temp = std::env::temp_dir();
        let defaults = orchestrator.analyze_path(&temp);

        // Directory should get wide-angle mode (truncation enabled)
        assert!(defaults.truncate_lines.is_some());
        assert!(defaults.truncate_lines.unwrap() > 0);
    }
}
