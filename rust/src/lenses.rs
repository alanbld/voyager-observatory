//! Context Lenses for focused project serialization
//!
//! Lenses provide pre-configured views of a project optimized for specific use cases:
//! - architecture: High-level code structure
//! - debug: Recent changes for debugging
//! - security: Security-relevant files
//! - onboarding: Essential files for new contributors

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Priority group for file ranking (v1.7.0)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PriorityGroup {
    /// Glob pattern to match files (e.g., "*.py", "src/**/*.rs", "tests/**")
    pub pattern: String,

    /// Priority value (higher = more important)
    /// Standard range: 0-100, but arbitrary integers supported
    pub priority: i32,

    /// Optional truncation mode override for this group
    #[serde(default)]
    pub truncate_mode: Option<String>,

    /// Optional truncation limit override for this group
    #[serde(default)]
    pub truncate: Option<usize>,
}

/// Fallback configuration for files that don't match any group
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FallbackConfig {
    /// Default priority for unmatched files (default: 50)
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    50
}

/// Lens configuration that can override EncoderConfig settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LensConfig {
    /// Human-readable description of the lens
    #[serde(default)]
    pub description: String,

    /// Truncation mode: "simple", "smart", "structure"
    #[serde(default)]
    pub truncate_mode: Option<String>,

    /// Maximum lines per file (0 = no truncation)
    #[serde(default)]
    pub truncate: Option<usize>,

    /// Patterns to exclude
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Patterns to include
    #[serde(default)]
    pub include: Vec<String>,

    /// Sort by: "name", "mtime", "ctime"
    #[serde(default)]
    pub sort_by: Option<String>,

    /// Sort order: "asc", "desc"
    #[serde(default)]
    pub sort_order: Option<String>,

    /// Priority groups for file ranking (v1.7.0)
    #[serde(default)]
    pub groups: Vec<PriorityGroup>,

    /// Fallback config for files matching no groups (v1.7.0)
    #[serde(default)]
    pub fallback: Option<FallbackConfig>,
}

impl Default for LensConfig {
    fn default() -> Self {
        Self {
            description: String::new(),
            truncate_mode: None,
            truncate: None,
            exclude: Vec::new(),
            include: Vec::new(),
            sort_by: None,
            sort_order: None,
            groups: Vec::new(),
            fallback: None,
        }
    }
}

/// Manager for context lenses
pub struct LensManager {
    /// Built-in lenses
    built_in: HashMap<String, LensConfig>,
    /// User-defined lenses from config
    custom: HashMap<String, LensConfig>,
    /// Currently active lens
    pub active_lens: Option<String>,
}

impl LensManager {
    /// Create a new LensManager with built-in lenses
    pub fn new() -> Self {
        let mut built_in = HashMap::new();

        // Architecture lens - high-level code structure
        built_in.insert("architecture".to_string(), LensConfig {
            description: "High-level code structure and configuration".to_string(),
            truncate_mode: Some("structure".to_string()),
            truncate: Some(2000),
            exclude: vec![
                "tests/**".to_string(), "test/**".to_string(),
                "docs/**".to_string(), "doc/**".to_string(),
                "htmlcov/**".to_string(), "coverage.xml".to_string(),
                "*.html".to_string(), "*.css".to_string(),
                "CONTEXT.txt".to_string(), "*.txt".to_string(),
                "test_vectors/**".to_string(),
                "research/**".to_string(), "LLM/**".to_string(),
                "target/**".to_string(), "dist/**".to_string(),
                "scripts/**".to_string(),
                ".github/**".to_string(),
            ],
            include: vec![
                "*.py".to_string(), "*.js".to_string(), "*.ts".to_string(),
                "*.rs".to_string(), "*.json".to_string(), "*.toml".to_string(),
                "*.yaml".to_string(), "*.yml".to_string(),
                "Dockerfile".to_string(), "Makefile".to_string(), "README.md".to_string(),
            ],
            sort_by: Some("name".to_string()),
            sort_order: Some("asc".to_string()),
            groups: Vec::new(),  // v1.7.0: no default groups for built-in lenses
            fallback: None,
        });

        // Debug lens - recent changes
        built_in.insert("debug".to_string(), LensConfig {
            description: "Recent changes for debugging".to_string(),
            truncate_mode: None,
            truncate: Some(0),
            exclude: vec![
                "*.pyc".to_string(), "__pycache__".to_string(), ".git".to_string(),
            ],
            include: Vec::new(),
            sort_by: Some("mtime".to_string()),
            sort_order: Some("desc".to_string()),
            groups: Vec::new(),
            fallback: None,
        });

        // Security lens
        built_in.insert("security".to_string(), LensConfig {
            description: "Security-relevant files (auth, secrets, dependencies)".to_string(),
            truncate_mode: None,
            truncate: Some(0),
            exclude: vec![
                "tests/**".to_string(), "test/**".to_string(), "docs/**".to_string(),
            ],
            include: vec![
                "**/*auth*".to_string(), "**/*security*".to_string(),
                "**/*secret*".to_string(), "**/*credential*".to_string(),
                "package.json".to_string(), "requirements.txt".to_string(),
                "Cargo.toml".to_string(), "Dockerfile".to_string(),
            ],
            sort_by: Some("name".to_string()),
            sort_order: None,
            groups: Vec::new(),
            fallback: None,
        });

        // Onboarding lens
        built_in.insert("onboarding".to_string(), LensConfig {
            description: "Essential files for new contributors".to_string(),
            truncate_mode: None,
            truncate: Some(0),
            exclude: Vec::new(),
            include: vec![
                "README.md".to_string(), "CONTRIBUTING.md".to_string(),
                "LICENSE".to_string(), "CHANGELOG.md".to_string(),
                "**/main.py".to_string(), "**/index.js".to_string(),
                "package.json".to_string(), "Cargo.toml".to_string(),
                "Makefile".to_string(), "Dockerfile".to_string(),
            ],
            sort_by: Some("name".to_string()),
            sort_order: None,
            groups: Vec::new(),
            fallback: None,
        });

        Self {
            built_in,
            custom: HashMap::new(),
            active_lens: None,
        }
    }

    /// Load custom lenses from config
    pub fn load_custom(&mut self, lenses: HashMap<String, LensConfig>) {
        self.custom = lenses;
    }

    /// Get a lens by name (checks custom first, then built-in)
    pub fn get_lens(&self, name: &str) -> Option<&LensConfig> {
        self.custom.get(name).or_else(|| self.built_in.get(name))
    }

    /// Get list of available lens names
    pub fn available_lenses(&self) -> Vec<String> {
        let mut lenses: Vec<String> = self.built_in.keys().cloned().collect();
        lenses.extend(self.custom.keys().cloned());
        lenses.sort();
        lenses.dedup();
        lenses
    }

    /// Apply a lens and return merged configuration values
    ///
    /// Returns: (ignore_patterns, include_patterns, sort_by, sort_order, truncate_lines, truncate_mode)
    pub fn apply_lens(&mut self, name: &str) -> Result<AppliedLens, String> {
        let lens = self.get_lens(name)
            .ok_or_else(|| format!(
                "Unknown lens '{}'. Available: {}",
                name,
                self.available_lenses().join(", ")
            ))?
            .clone();

        self.active_lens = Some(name.to_string());

        Ok(AppliedLens {
            name: name.to_string(),
            description: lens.description.clone(),
            ignore_patterns: lens.exclude.clone(),
            include_patterns: lens.include.clone(),
            sort_by: lens.sort_by.unwrap_or_else(|| "name".to_string()),
            sort_order: lens.sort_order.unwrap_or_else(|| "asc".to_string()),
            truncate_lines: lens.truncate.unwrap_or(0),
            truncate_mode: lens.truncate_mode.unwrap_or_else(|| "simple".to_string()),
        })
    }

    /// Print lens manifest to stderr
    pub fn print_manifest(&self, lens_name: &str) {
        if let Some(lens) = self.get_lens(lens_name) {
            eprintln!("╔════════════════════════════════════════════════════════════════╗");
            eprintln!("║ CONTEXT LENS: {:<48} ║", lens_name);
            eprintln!("╠════════════════════════════════════════════════════════════════╣");
            eprintln!("║ {:<62} ║", lens.description);
            eprintln!("╠════════════════════════════════════════════════════════════════╣");

            if let Some(ref mode) = lens.truncate_mode {
                eprintln!("║ Truncation Mode: {:<45} ║", mode);
            }
            if let Some(limit) = lens.truncate {
                if limit > 0 {
                    eprintln!("║ Truncation Limit: {:<44} ║", format!("{} lines", limit));
                }
            }
            if let Some(ref sort) = lens.sort_by {
                eprintln!("║ Sort By: {:<53} ║", sort);
            }
            if !lens.include.is_empty() {
                eprintln!("║ Include Patterns: {:<44} ║", lens.include.len());
            }
            if !lens.exclude.is_empty() {
                eprintln!("║ Exclude Patterns: {:<44} ║", lens.exclude.len());
            }

            eprintln!("╚════════════════════════════════════════════════════════════════╝");
        }
    }

    /// Get priority for a file based on the active lens configuration (v1.7.0)
    ///
    /// Returns the highest matching priority from groups, or fallback priority.
    /// Default priority is 50 if no groups defined (backward compatible).
    pub fn get_file_priority(&self, file_path: &Path) -> i32 {
        let lens_config = match &self.active_lens {
            Some(name) => self.get_lens(name),
            None => return 50, // No active lens = default priority
        };

        let config = match lens_config {
            Some(c) => c,
            None => return 50,
        };

        // Backward compatibility: no groups = all files equal priority
        if config.groups.is_empty() {
            return 50;
        }

        // Find ALL groups that match, return HIGHEST priority
        let mut highest_priority: Option<i32> = None;

        for group in &config.groups {
            if Self::match_pattern(file_path, &group.pattern) {
                match highest_priority {
                    None => highest_priority = Some(group.priority),
                    Some(current) if group.priority > current => {
                        highest_priority = Some(group.priority);
                    }
                    _ => {}
                }
            }
        }

        // Return highest match or fallback priority
        highest_priority.unwrap_or_else(|| {
            config.fallback.as_ref()
                .map(|f| f.priority)
                .unwrap_or(50)
        })
    }

    /// Match a file path against a glob pattern
    ///
    /// Handles both simple patterns (*.py) and recursive patterns (**/*.rs, tests/**)
    fn match_pattern(file_path: &Path, pattern: &str) -> bool {
        let file_str = file_path.to_string_lossy();
        let file_name = file_path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Handle ** recursive patterns
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');

                // Case 1: "tests/**" - prefix only (directory match)
                if suffix.is_empty() {
                    if prefix.is_empty() {
                        return true; // "**" matches everything
                    }
                    return file_str.starts_with(&format!("{}/", prefix))
                        || file_str.as_ref() == prefix;
                }

                // Case 2: "**/*.rs" - suffix only (extension anywhere)
                if prefix.is_empty() {
                    return Self::simple_match(&file_name, suffix)
                        || Self::simple_match(&file_str, &format!("*/{}", suffix));
                }

                // Case 3: "src/**/*.py" - both prefix and suffix
                if file_str.starts_with(&format!("{}/", prefix)) {
                    let remaining = &file_str[prefix.len() + 1..];
                    let remaining_name = Path::new(&*remaining)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    return Self::simple_match(&remaining_name, suffix)
                        || Self::simple_match(&remaining.to_string(), &format!("*/{}", suffix));
                }
                return false;
            }
        }

        // Simple pattern - try matching against full path and file name
        Self::simple_match(&file_str, pattern) || Self::simple_match(&file_name, pattern)
    }

    /// Simple glob matching with * wildcard
    fn simple_match(text: &str, pattern: &str) -> bool {
        // Handle exact match
        if !pattern.contains('*') {
            return text == pattern;
        }

        // Handle *.ext patterns
        if pattern.starts_with("*.") {
            let ext = &pattern[1..]; // ".ext"
            return text.ends_with(ext);
        }

        // Handle *suffix patterns
        if pattern.starts_with('*') && !pattern[1..].contains('*') {
            return text.ends_with(&pattern[1..]);
        }

        // Handle prefix* patterns
        if pattern.ends_with('*') && !pattern[..pattern.len()-1].contains('*') {
            return text.starts_with(&pattern[..pattern.len()-1]);
        }

        // Handle prefix*suffix patterns (single *)
        if let Some(star_pos) = pattern.find('*') {
            if !pattern[star_pos+1..].contains('*') {
                let prefix = &pattern[..star_pos];
                let suffix = &pattern[star_pos+1..];
                return text.starts_with(prefix) && text.ends_with(suffix);
            }
        }

        // Fallback: exact match (no complex glob support)
        text == pattern
    }
}

impl Default for LensManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of applying a lens
#[derive(Debug, Clone)]
pub struct AppliedLens {
    pub name: String,
    pub description: String,
    pub ignore_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub sort_by: String,
    pub sort_order: String,
    pub truncate_lines: usize,
    pub truncate_mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lens_manager_new() {
        let manager = LensManager::new();
        assert!(manager.get_lens("architecture").is_some());
        assert!(manager.get_lens("debug").is_some());
        assert!(manager.get_lens("security").is_some());
        assert!(manager.get_lens("onboarding").is_some());
    }

    #[test]
    fn test_apply_lens() {
        let mut manager = LensManager::new();
        let applied = manager.apply_lens("architecture").unwrap();

        assert_eq!(applied.name, "architecture");
        assert_eq!(applied.truncate_mode, "structure");
        assert_eq!(applied.sort_by, "name");
        assert!(!applied.include_patterns.is_empty());
    }

    #[test]
    fn test_unknown_lens() {
        let mut manager = LensManager::new();
        let result = manager.apply_lens("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_available_lenses() {
        let manager = LensManager::new();
        let lenses = manager.available_lenses();
        assert!(lenses.contains(&"architecture".to_string()));
        assert!(lenses.contains(&"debug".to_string()));
    }

    // Priority Groups tests (v1.7.0)

    #[test]
    fn test_priority_no_active_lens() {
        let manager = LensManager::new();
        // No active lens = default priority 50
        assert_eq!(manager.get_file_priority(Path::new("src/main.py")), 50);
    }

    #[test]
    fn test_priority_no_groups() {
        let mut manager = LensManager::new();
        // Apply a lens without groups (backward compatibility)
        let _ = manager.apply_lens("debug");
        assert_eq!(manager.get_file_priority(Path::new("any_file.py")), 50);
    }

    #[test]
    fn test_priority_with_groups() {
        let mut manager = LensManager::new();

        // Create a custom lens with groups
        let lens = LensConfig {
            description: "Test lens".to_string(),
            groups: vec![
                PriorityGroup {
                    pattern: "*.py".to_string(),
                    priority: 100,
                    truncate_mode: None,
                    truncate: None,
                },
                PriorityGroup {
                    pattern: "tests/**".to_string(),
                    priority: 10,
                    truncate_mode: None,
                    truncate: None,
                },
            ],
            fallback: Some(FallbackConfig { priority: 30 }),
            ..Default::default()
        };

        manager.custom.insert("test".to_string(), lens);
        let _ = manager.apply_lens("test");

        // *.py matches -> priority 100
        assert_eq!(manager.get_file_priority(Path::new("main.py")), 100);

        // tests/foo.py -> matches both, highest wins (100)
        assert_eq!(manager.get_file_priority(Path::new("tests/foo.py")), 100);

        // tests/data.json -> matches tests/**, priority 10
        assert_eq!(manager.get_file_priority(Path::new("tests/data.json")), 10);

        // unmatched.txt -> fallback priority 30
        assert_eq!(manager.get_file_priority(Path::new("docs/unmatched.txt")), 30);
    }

    #[test]
    fn test_pattern_simple_extension() {
        assert!(LensManager::match_pattern(Path::new("main.py"), "*.py"));
        assert!(LensManager::match_pattern(Path::new("src/lib.rs"), "*.rs"));
        assert!(!LensManager::match_pattern(Path::new("main.py"), "*.rs"));
    }

    #[test]
    fn test_pattern_directory_recursive() {
        // tests/** should match anything under tests/
        assert!(LensManager::match_pattern(Path::new("tests/unit.py"), "tests/**"));
        assert!(LensManager::match_pattern(Path::new("tests/a/b/c.py"), "tests/**"));
        assert!(!LensManager::match_pattern(Path::new("src/tests/x.py"), "tests/**"));
    }

    #[test]
    fn test_pattern_extension_anywhere() {
        // **/*.rs should match .rs files anywhere
        assert!(LensManager::match_pattern(Path::new("lib.rs"), "**/*.rs"));
        assert!(LensManager::match_pattern(Path::new("src/lib.rs"), "**/*.rs"));
        assert!(LensManager::match_pattern(Path::new("a/b/c/main.rs"), "**/*.rs"));
    }

    #[test]
    fn test_pattern_prefix_and_suffix() {
        // src/**/*.py should match .py files under src/
        assert!(LensManager::match_pattern(Path::new("src/main.py"), "src/**/*.py"));
        assert!(LensManager::match_pattern(Path::new("src/utils/helper.py"), "src/**/*.py"));
        assert!(!LensManager::match_pattern(Path::new("tests/main.py"), "src/**/*.py"));
    }

    #[test]
    fn test_highest_priority_wins() {
        let mut manager = LensManager::new();

        let lens = LensConfig {
            description: "Test".to_string(),
            groups: vec![
                PriorityGroup {
                    pattern: "*.py".to_string(),
                    priority: 80,
                    truncate_mode: None,
                    truncate: None,
                },
                PriorityGroup {
                    pattern: "src/**".to_string(),
                    priority: 60,
                    truncate_mode: None,
                    truncate: None,
                },
            ],
            fallback: Some(FallbackConfig { priority: 50 }),
            ..Default::default()
        };

        manager.custom.insert("test".to_string(), lens);
        let _ = manager.apply_lens("test");

        // src/main.py matches both: *.py (80) and src/** (60)
        // Should return 80 (highest)
        assert_eq!(manager.get_file_priority(Path::new("src/main.py")), 80);
    }
}
