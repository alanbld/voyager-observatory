//! Observer's Journal Module
//!
//! The Journal is the persistent memory of the Observatory. It records:
//! - **Bright Stars**: Files marked as important by the user
//! - **Exploration History**: Constellations (intents) explored
//! - **Faded Nebulae**: Files consistently ignored or truncated
//!
//! # Usage
//!
//! ```bash
//! vo --mark src/core/engine.rs    # Mark a star as important
//! vo --journal                     # View exploration history
//! vo --journal-clear              # Start fresh
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// =============================================================================
// Journal Entry Types
// =============================================================================

/// A star (file) marked as important in the journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkedStar {
    /// Path to the file
    pub path: String,
    /// Utility score (0.0 - 1.0)
    pub utility: f64,
    /// When the star was marked
    pub marked_at: String,
    /// Optional note from the user
    pub note: Option<String>,
    /// Number of times this star has been viewed
    pub view_count: u32,
}

impl MarkedStar {
    /// Create a new marked star.
    pub fn new(path: &str, utility: f64) -> Self {
        Self {
            path: path.to_string(),
            utility,
            marked_at: current_timestamp(),
            note: None,
            view_count: 0,
        }
    }

    /// Check if this is a "bright" star (high utility).
    pub fn is_bright(&self) -> bool {
        self.utility >= 0.8
    }

    /// Increment view count.
    pub fn viewed(&mut self) {
        self.view_count += 1;
    }
}

/// An exploration session recorded in the journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationEntry {
    /// The intent that was explored
    pub intent: String,
    /// When the exploration occurred
    pub explored_at: String,
    /// Number of files analyzed
    pub files_analyzed: usize,
    /// Key insights discovered
    pub key_insights: Vec<String>,
    /// Starting point recommended
    pub starting_point: Option<String>,
}

impl ExplorationEntry {
    /// Create a new exploration entry.
    pub fn new(intent: &str, files: usize) -> Self {
        Self {
            intent: intent.to_string(),
            explored_at: current_timestamp(),
            files_analyzed: files,
            key_insights: Vec::new(),
            starting_point: None,
        }
    }
}

/// A nebula that has faded (consistently ignored).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FadedNebula {
    /// Path pattern that was ignored
    pub pattern: String,
    /// Number of times ignored
    pub ignore_count: u32,
    /// Last time it was ignored
    pub last_ignored: String,
}

// =============================================================================
// The Observer's Journal
// =============================================================================

/// The Observer's Journal - persistent memory of the Observatory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObserversJournal {
    /// Version for future migrations
    pub version: String,
    /// Bright stars (marked files)
    pub bright_stars: HashMap<String, MarkedStar>,
    /// Exploration history
    pub explorations: Vec<ExplorationEntry>,
    /// Faded nebulae (ignored patterns)
    pub faded_nebulae: HashMap<String, FadedNebula>,
    /// Total explorations count
    pub total_explorations: u64,
    /// Journal creation date
    pub created_at: String,
    /// Last updated
    pub updated_at: String,
}

impl ObserversJournal {
    /// Create a new empty journal.
    pub fn new() -> Self {
        let now = current_timestamp();
        Self {
            version: "1.0.0".to_string(),
            bright_stars: HashMap::new(),
            explorations: Vec::new(),
            faded_nebulae: HashMap::new(),
            total_explorations: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Load journal from the default path for a project.
    pub fn load(project_root: &Path) -> Self {
        let path = Self::default_path(project_root);
        Self::load_from_file(&path)
    }

    /// Load journal from a specific file.
    pub fn load_from_file(path: &Path) -> Self {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    serde_json::from_str(&contents).unwrap_or_else(|_| Self::new())
                }
                Err(_) => Self::new(),
            }
        } else {
            Self::new()
        }
    }

    /// Save journal to the default path.
    pub fn save(&self, project_root: &Path) -> std::io::Result<()> {
        let path = Self::default_path(project_root);
        self.save_to_file(&path)
    }

    /// Save journal to a specific file.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)
    }

    /// Get the default journal path for a project.
    pub fn default_path(project_root: &Path) -> PathBuf {
        project_root.join(".pm_encoder").join("journal.json")
    }

    // =========================================================================
    // Star Operations
    // =========================================================================

    /// Mark a star (file) as important.
    pub fn mark_star(&mut self, path: &str, utility: f64) {
        let star = MarkedStar::new(path, utility);
        self.bright_stars.insert(path.to_string(), star);
        self.updated_at = current_timestamp();
    }

    /// Mark a star with a note.
    pub fn mark_star_with_note(&mut self, path: &str, utility: f64, note: &str) {
        let mut star = MarkedStar::new(path, utility);
        star.note = Some(note.to_string());
        self.bright_stars.insert(path.to_string(), star);
        self.updated_at = current_timestamp();
    }

    /// Get a marked star.
    pub fn get_star(&self, path: &str) -> Option<&MarkedStar> {
        self.bright_stars.get(path)
    }

    /// Check if a path is a bright star.
    pub fn is_bright_star(&self, path: &str) -> bool {
        self.bright_stars
            .get(path)
            .map(|s| s.is_bright())
            .unwrap_or(false)
    }

    /// Get all bright stars (utility >= 0.8).
    pub fn all_bright_stars(&self) -> Vec<&MarkedStar> {
        self.bright_stars
            .values()
            .filter(|s| s.is_bright())
            .collect()
    }

    /// Record a star view.
    pub fn record_view(&mut self, path: &str) {
        if let Some(star) = self.bright_stars.get_mut(path) {
            star.viewed();
            self.updated_at = current_timestamp();
        }
    }

    // =========================================================================
    // Exploration Operations
    // =========================================================================

    /// Record an exploration session.
    pub fn record_exploration(&mut self, entry: ExplorationEntry) {
        self.explorations.push(entry);
        self.total_explorations += 1;
        self.updated_at = current_timestamp();

        // Keep only last 50 explorations
        if self.explorations.len() > 50 {
            self.explorations.remove(0);
        }
    }

    /// Get recent explorations.
    pub fn recent_explorations(&self, count: usize) -> &[ExplorationEntry] {
        let start = self.explorations.len().saturating_sub(count);
        &self.explorations[start..]
    }

    // =========================================================================
    // Faded Nebulae Operations
    // =========================================================================

    /// Record a file/pattern as ignored.
    pub fn record_ignored(&mut self, pattern: &str) {
        let entry = self.faded_nebulae.entry(pattern.to_string()).or_insert(FadedNebula {
            pattern: pattern.to_string(),
            ignore_count: 0,
            last_ignored: current_timestamp(),
        });
        entry.ignore_count += 1;
        entry.last_ignored = current_timestamp();
        self.updated_at = current_timestamp();
    }

    /// Check if a pattern is a faded nebula (ignored many times).
    pub fn is_faded(&self, pattern: &str) -> bool {
        self.faded_nebulae
            .get(pattern)
            .map(|n| n.ignore_count >= 5)
            .unwrap_or(false)
    }

    // =========================================================================
    // Display
    // =========================================================================

    /// Format the journal for display.
    pub fn display(&self) -> String {
        let mut output = String::new();

        output.push_str("📓 OBSERVER'S JOURNAL\n");
        output.push_str("═══════════════════════════════════════════════════════════\n\n");

        // Stats
        output.push_str(&format!("🌌 Total Explorations: {}\n", self.total_explorations));
        output.push_str(&format!("⭐ Marked Stars: {}\n", self.bright_stars.len()));
        output.push_str(&format!("🌫️  Faded Nebulae: {}\n\n", self.faded_nebulae.len()));

        // Bright Stars
        if !self.bright_stars.is_empty() {
            output.push_str("⭐ BRIGHT STARS (Your Important Files)\n");
            output.push_str("───────────────────────────────────────────────────────────\n");

            let mut stars: Vec<_> = self.bright_stars.values().collect();
            stars.sort_by(|a, b| b.utility.partial_cmp(&a.utility).unwrap());

            for star in stars.iter().take(10) {
                let brightness = if star.utility >= 0.9 {
                    "🌟"
                } else if star.utility >= 0.8 {
                    "⭐"
                } else {
                    "✨"
                };

                output.push_str(&format!(
                    "  {} {} (utility: {:.0}%, views: {})\n",
                    brightness,
                    star.path,
                    star.utility * 100.0,
                    star.view_count
                ));

                if let Some(note) = &star.note {
                    output.push_str(&format!("      📝 {}\n", note));
                }
            }

            if stars.len() > 10 {
                output.push_str(&format!("  ... and {} more\n", stars.len() - 10));
            }
            output.push('\n');
        }

        // Recent Explorations
        if !self.explorations.is_empty() {
            output.push_str("🔭 RECENT EXPLORATIONS\n");
            output.push_str("───────────────────────────────────────────────────────────\n");

            for entry in self.recent_explorations(5).iter().rev() {
                output.push_str(&format!(
                    "  {} {} ({} files analyzed)\n",
                    entry.explored_at.split('T').next().unwrap_or(&entry.explored_at),
                    entry.intent,
                    entry.files_analyzed
                ));

                if let Some(start) = &entry.starting_point {
                    output.push_str(&format!("      🧭 Started at: {}\n", start));
                }
            }
            output.push('\n');
        }

        // Faded Nebulae
        let faded: Vec<_> = self.faded_nebulae.values().filter(|n| n.ignore_count >= 5).collect();
        if !faded.is_empty() {
            output.push_str("🌫️  FADED NEBULAE (Consistently Ignored)\n");
            output.push_str("───────────────────────────────────────────────────────────\n");

            for nebula in faded.iter().take(5) {
                output.push_str(&format!(
                    "  {} (ignored {} times)\n",
                    nebula.pattern,
                    nebula.ignore_count
                ));
            }
            output.push('\n');
        }

        output.push_str("───────────────────────────────────────────────────────────\n");
        output.push_str(&format!("Journal created: {}\n", self.created_at.split('T').next().unwrap_or(&self.created_at)));
        output.push_str(&format!("Last updated: {}\n", self.updated_at.split('T').next().unwrap_or(&self.updated_at)));

        output
    }

    /// Clear the journal.
    pub fn clear(&mut self) {
        self.bright_stars.clear();
        self.explorations.clear();
        self.faded_nebulae.clear();
        self.total_explorations = 0;
        self.updated_at = current_timestamp();
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Get current timestamp in ISO format.
fn current_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();
    let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)
        .unwrap_or_else(|| chrono::Utc::now());

    datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_new_journal() {
        let journal = ObserversJournal::new();
        assert_eq!(journal.version, "1.0.0");
        assert!(journal.bright_stars.is_empty());
        assert!(journal.explorations.is_empty());
    }

    #[test]
    fn test_mark_star() {
        let mut journal = ObserversJournal::new();
        journal.mark_star("src/lib.rs", 0.95);

        assert!(journal.is_bright_star("src/lib.rs"));
        assert!(!journal.is_bright_star("other.rs"));
    }

    #[test]
    fn test_mark_star_with_note() {
        let mut journal = ObserversJournal::new();
        journal.mark_star_with_note("src/core.rs", 0.9, "The heart of the engine");

        let star = journal.get_star("src/core.rs").unwrap();
        assert_eq!(star.note, Some("The heart of the engine".to_string()));
    }

    #[test]
    fn test_all_bright_stars() {
        let mut journal = ObserversJournal::new();
        journal.mark_star("bright1.rs", 0.9);
        journal.mark_star("bright2.rs", 0.85);
        journal.mark_star("dim.rs", 0.5);

        let bright = journal.all_bright_stars();
        assert_eq!(bright.len(), 2);
    }

    #[test]
    fn test_record_exploration() {
        let mut journal = ObserversJournal::new();

        let entry = ExplorationEntry::new("business-logic", 42);
        journal.record_exploration(entry);

        assert_eq!(journal.total_explorations, 1);
        assert_eq!(journal.explorations.len(), 1);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let mut journal = ObserversJournal::new();
        journal.mark_star("test.rs", 0.9);

        journal.save(dir.path()).unwrap();

        let loaded = ObserversJournal::load(dir.path());
        assert!(loaded.is_bright_star("test.rs"));
    }

    #[test]
    fn test_faded_nebula() {
        let mut journal = ObserversJournal::new();

        for _ in 0..5 {
            journal.record_ignored("node_modules/**");
        }

        assert!(journal.is_faded("node_modules/**"));
        assert!(!journal.is_faded("src/**"));
    }

    #[test]
    fn test_display() {
        let mut journal = ObserversJournal::new();
        journal.mark_star("src/lib.rs", 0.95);

        let output = journal.display();
        assert!(output.contains("OBSERVER'S JOURNAL"));
        assert!(output.contains("src/lib.rs"));
    }
}
