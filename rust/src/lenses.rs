//! Context Lenses for focused project serialization
//!
//! Lenses provide pre-configured views of a project optimized for specific use cases:
//! - architecture: High-level code structure
//! - debug: Recent changes for debugging
//! - security: Security-relevant files
//! - onboarding: Essential files for new contributors

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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
}
