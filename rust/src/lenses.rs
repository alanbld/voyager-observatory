//! Context Lenses for focused project serialization
//!
//! Lenses provide pre-configured views of a project optimized for specific use cases:
//! - architecture: High-level code structure
//! - debug: Recent changes for debugging
//! - security: Security-relevant files
//! - onboarding: Essential files for new contributors
//!
//! # Learning Integration (v2.2.0)
//!
//! LensManager can integrate with ContextStore for adaptive prioritization:
//! - Accepts optional ContextStore for learned priorities
//! - Uses Priority Blend: `final = (static * 0.7) + (learned * 100 * 0.3)`
//! - Respects "frozen" mode by ignoring learned priorities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::core::store::ContextStore;

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
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

/// Manager for context lenses
pub struct LensManager {
    /// Built-in lenses
    built_in: HashMap<String, LensConfig>,
    /// User-defined lenses from config
    custom: HashMap<String, LensConfig>,
    /// Currently active lens
    pub active_lens: Option<String>,
    /// Optional context store for learned priorities (v2.2.0)
    context_store: Option<ContextStore>,
    /// Frozen mode: ignore learned priorities for deterministic output
    frozen: bool,
}

impl LensManager {
    /// Create a new LensManager with built-in lenses
    pub fn new() -> Self {
        let mut built_in = HashMap::new();

        // Architecture lens - high-level code structure
        // v1.7.0: Priority groups for token budgeting (matches Python)
        built_in.insert(
            "architecture".to_string(),
            LensConfig {
                description: "High-level code structure and configuration".to_string(),
                truncate_mode: Some("structure".to_string()),
                truncate: Some(2000),
                exclude: vec![
                    "tests/**".to_string(),
                    "test/**".to_string(),
                    "docs/**".to_string(),
                    "doc/**".to_string(),
                    "htmlcov/**".to_string(),
                    "coverage.xml".to_string(),
                    "*.html".to_string(),
                    "*.css".to_string(),
                    "CONTEXT.txt".to_string(),
                    "*.txt".to_string(),
                    "test_vectors/**".to_string(),
                    "research/**".to_string(),
                    "LLM/**".to_string(),
                    "target/**".to_string(),
                    "dist/**".to_string(),
                    "scripts/**".to_string(),
                    ".github/**".to_string(),
                ],
                include: vec![
                    "*.py".to_string(),
                    "*.js".to_string(),
                    "*.ts".to_string(),
                    "*.rs".to_string(),
                    "*.json".to_string(),
                    "*.toml".to_string(),
                    "*.yaml".to_string(),
                    "*.yml".to_string(),
                    "Dockerfile".to_string(),
                    "Makefile".to_string(),
                    "README.md".to_string(),
                ],
                sort_by: Some("name".to_string()),
                sort_order: Some("asc".to_string()),
                groups: vec![
                    // Core implementation files - highest priority (100)
                    PriorityGroup {
                        pattern: "*.py".to_string(),
                        priority: 100,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "rust/src/**/*.rs".to_string(),
                        priority: 100,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/*.rs".to_string(),
                        priority: 95,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    // Configuration files - high priority (90-80)
                    PriorityGroup {
                        pattern: "Cargo.toml".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "pyproject.toml".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.toml".to_string(),
                        priority: 85,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.json".to_string(),
                        priority: 80,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.yaml".to_string(),
                        priority: 80,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.yml".to_string(),
                        priority: 80,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Build files - medium-high priority (75-70)
                    PriorityGroup {
                        pattern: "Makefile".to_string(),
                        priority: 75,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "Dockerfile".to_string(),
                        priority: 70,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Documentation - medium priority (65)
                    PriorityGroup {
                        pattern: "README.md".to_string(),
                        priority: 65,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // JavaScript/TypeScript - medium priority (60-55)
                    PriorityGroup {
                        pattern: "*.ts".to_string(),
                        priority: 60,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.tsx".to_string(),
                        priority: 60,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.js".to_string(),
                        priority: 55,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.jsx".to_string(),
                        priority: 55,
                        truncate_mode: None,
                        truncate: None,
                    },
                ],
                fallback: Some(FallbackConfig { priority: 50 }),
            },
        );

        // Debug lens - recent changes with full content
        built_in.insert(
            "debug".to_string(),
            LensConfig {
                description: "Recent changes for debugging".to_string(),
                truncate_mode: None,
                truncate: Some(0), // No truncation - full content
                exclude: vec![
                    "*.pyc".to_string(),
                    "__pycache__".to_string(),
                    ".git".to_string(),
                ],
                include: Vec::new(),
                sort_by: Some("mtime".to_string()),
                sort_order: Some("desc".to_string()),
                groups: vec![
                    // Core implementation files - highest priority
                    PriorityGroup {
                        pattern: "*.py".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.rs".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.js".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.ts".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Tests - high priority for debugging
                    PriorityGroup {
                        pattern: "tests/**".to_string(),
                        priority: 85,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "test/**".to_string(),
                        priority: 85,
                        truncate_mode: None,
                        truncate: None,
                    },
                ],
                fallback: Some(FallbackConfig { priority: 50 }),
            },
        );

        // Security lens - focuses on auth, secrets, dependencies, and attack surface
        built_in.insert(
            "security".to_string(),
            LensConfig {
                description: "Security-relevant files (auth, secrets, dependencies, APIs)"
                    .to_string(),
                truncate_mode: None,
                truncate: Some(0),
                exclude: vec![
                    "docs/**".to_string(),
                    "doc/**".to_string(),
                    "*.md".to_string(),
                    "*.txt".to_string(),
                    "htmlcov/**".to_string(),
                    "coverage/**".to_string(),
                ],
                include: vec![
                    "**/*auth*".to_string(),
                    "**/*security*".to_string(),
                    "**/*secret*".to_string(),
                    "**/*credential*".to_string(),
                    "package.json".to_string(),
                    "requirements.txt".to_string(),
                    "Cargo.toml".to_string(),
                    "Dockerfile".to_string(),
                ],
                sort_by: Some("name".to_string()),
                sort_order: None,
                groups: vec![
                    // Auth and secrets - highest priority (Tier 1: Critical)
                    PriorityGroup {
                        pattern: "*auth*".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*secret*".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*credential*".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*password*".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*token*".to_string(),
                        priority: 98,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*security*".to_string(),
                        priority: 95,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*crypto*".to_string(),
                        priority: 95,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*encrypt*".to_string(),
                        priority: 95,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Environment and config - sensitive settings (Tier 2: High)
                    PriorityGroup {
                        pattern: "*.env*".to_string(),
                        priority: 92,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*config*".to_string(),
                        priority: 88,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*settings*".to_string(),
                        priority: 88,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Dependency files - vulnerability analysis (Tier 2: High)
                    PriorityGroup {
                        pattern: "package.json".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "package-lock.json".to_string(),
                        priority: 85,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "requirements.txt".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "Cargo.toml".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "Cargo.lock".to_string(),
                        priority: 85,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "go.mod".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "Gemfile".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // API and server boundaries - attack surface (Tier 3: Medium-High)
                    PriorityGroup {
                        pattern: "**/server/**".to_string(),
                        priority: 80,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/api/**".to_string(),
                        priority: 80,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/routes/**".to_string(),
                        priority: 78,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/handlers/**".to_string(),
                        priority: 78,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/middleware/**".to_string(),
                        priority: 76,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*handler*".to_string(),
                        priority: 75,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*endpoint*".to_string(),
                        priority: 75,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Input processing - injection vectors (Tier 3: Medium-High)
                    PriorityGroup {
                        pattern: "*parse*".to_string(),
                        priority: 72,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*input*".to_string(),
                        priority: 70,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*valid*".to_string(),
                        priority: 70,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*sanitize*".to_string(),
                        priority: 70,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Infrastructure (Tier 4: Medium)
                    PriorityGroup {
                        pattern: "Dockerfile".to_string(),
                        priority: 65,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "docker-compose*.yml".to_string(),
                        priority: 65,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.yaml".to_string(),
                        priority: 55,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.yml".to_string(),
                        priority: 55,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Source code - lower priority but still relevant (Tier 5: Low)
                    PriorityGroup {
                        pattern: "*.rs".to_string(),
                        priority: 45,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.py".to_string(),
                        priority: 45,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.js".to_string(),
                        priority: 40,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.ts".to_string(),
                        priority: 40,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Tests - useful for understanding security assumptions (Tier 5: Low)
                    PriorityGroup {
                        pattern: "tests/**".to_string(),
                        priority: 35,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "test/**".to_string(),
                        priority: 35,
                        truncate_mode: None,
                        truncate: None,
                    },
                ],
                fallback: Some(FallbackConfig { priority: 25 }), // Low default - security lens is selective
            },
        );

        // Onboarding lens - prioritizes documentation and entry points for new contributors
        built_in.insert(
            "onboarding".to_string(),
            LensConfig {
                description: "Essential files for new contributors".to_string(),
                truncate_mode: Some("structure".to_string()),
                truncate: Some(500),
                exclude: vec![
                    "target/**".to_string(),
                    "dist/**".to_string(),
                    "node_modules/**".to_string(),
                    ".git/**".to_string(),
                    "*.lock".to_string(),
                    "*.min.js".to_string(),
                ],
                include: vec![
                    "README.md".to_string(),
                    "CONTRIBUTING.md".to_string(),
                    "LICENSE".to_string(),
                    "CHANGELOG.md".to_string(),
                    "**/main.py".to_string(),
                    "**/index.js".to_string(),
                    "package.json".to_string(),
                    "Cargo.toml".to_string(),
                    "Makefile".to_string(),
                    "Dockerfile".to_string(),
                ],
                sort_by: Some("name".to_string()),
                sort_order: None,
                groups: vec![
                    // Documentation - highest priority for onboarding
                    PriorityGroup {
                        pattern: "README.md".to_string(),
                        priority: 100,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "CONTRIBUTING.md".to_string(),
                        priority: 98,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "CLAUDE.md".to_string(),
                        priority: 97,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "CHANGELOG.md".to_string(),
                        priority: 95,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.md".to_string(),
                        priority: 90,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Project configuration - essential for understanding setup
                    PriorityGroup {
                        pattern: "Cargo.toml".to_string(),
                        priority: 88,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "package.json".to_string(),
                        priority: 88,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "pyproject.toml".to_string(),
                        priority: 88,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "Makefile".to_string(),
                        priority: 85,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "Dockerfile".to_string(),
                        priority: 80,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Entry points - where to start reading code
                    PriorityGroup {
                        pattern: "**/main.rs".to_string(),
                        priority: 75,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/lib.rs".to_string(),
                        priority: 75,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/main.py".to_string(),
                        priority: 75,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/index.js".to_string(),
                        priority: 75,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/index.ts".to_string(),
                        priority: 75,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "**/mod.rs".to_string(),
                        priority: 70,
                        truncate_mode: Some("structure".to_string()),
                        truncate: None,
                    },
                    // Config files - useful context
                    PriorityGroup {
                        pattern: "*.toml".to_string(),
                        priority: 65,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.yaml".to_string(),
                        priority: 60,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.yml".to_string(),
                        priority: 60,
                        truncate_mode: None,
                        truncate: None,
                    },
                    PriorityGroup {
                        pattern: "*.json".to_string(),
                        priority: 55,
                        truncate_mode: None,
                        truncate: None,
                    },
                    // Tests - helpful examples but lower priority
                    PriorityGroup {
                        pattern: "tests/**".to_string(),
                        priority: 40,
                        truncate_mode: Some("structure".to_string()),
                        truncate: Some(200),
                    },
                    PriorityGroup {
                        pattern: "test/**".to_string(),
                        priority: 40,
                        truncate_mode: Some("structure".to_string()),
                        truncate: Some(200),
                    },
                    // Source code - structure only for onboarding
                    PriorityGroup {
                        pattern: "*.rs".to_string(),
                        priority: 35,
                        truncate_mode: Some("structure".to_string()),
                        truncate: Some(300),
                    },
                    PriorityGroup {
                        pattern: "*.py".to_string(),
                        priority: 35,
                        truncate_mode: Some("structure".to_string()),
                        truncate: Some(300),
                    },
                    PriorityGroup {
                        pattern: "*.js".to_string(),
                        priority: 30,
                        truncate_mode: Some("structure".to_string()),
                        truncate: Some(300),
                    },
                    PriorityGroup {
                        pattern: "*.ts".to_string(),
                        priority: 30,
                        truncate_mode: Some("structure".to_string()),
                        truncate: Some(300),
                    },
                ],
                fallback: Some(FallbackConfig { priority: 20 }), // Low priority for implementation details
            },
        );

        Self {
            built_in,
            custom: HashMap::new(),
            active_lens: None,
            context_store: None,
            frozen: false,
        }
    }

    /// Create a new LensManager with a context store for learning (v2.2.0)
    pub fn with_store(store: ContextStore) -> Self {
        let mut manager = Self::new();
        manager.context_store = Some(store);
        manager
    }

    /// Set the context store for learned priorities
    pub fn set_store(&mut self, store: ContextStore) {
        self.context_store = Some(store);
    }

    /// Get a mutable reference to the context store
    pub fn store_mut(&mut self) -> Option<&mut ContextStore> {
        self.context_store.as_mut()
    }

    /// Get a reference to the context store
    pub fn store(&self) -> Option<&ContextStore> {
        self.context_store.as_ref()
    }

    /// Set frozen mode (ignores learned priorities)
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Check if frozen mode is enabled
    pub fn is_frozen(&self) -> bool {
        self.frozen
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
        let lens = self
            .get_lens(name)
            .ok_or_else(|| {
                format!(
                    "Unknown lens '{}'. Available: {}",
                    name,
                    self.available_lenses().join(", ")
                )
            })?
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

    /// Get the matching priority group config for a file (v1.7.0)
    ///
    /// Returns the highest-priority matching group, or a fallback group.
    /// Used by token budgeting to apply per-file truncation settings.
    pub fn get_file_group_config(&self, file_path: &Path) -> PriorityGroup {
        let lens_config = match &self.active_lens {
            Some(name) => self.get_lens(name),
            None => {
                return PriorityGroup {
                    pattern: "*".to_string(),
                    priority: 50,
                    truncate_mode: None,
                    truncate: None,
                }
            }
        };

        let config = match lens_config {
            Some(c) => c,
            None => {
                return PriorityGroup {
                    pattern: "*".to_string(),
                    priority: 50,
                    truncate_mode: None,
                    truncate: None,
                }
            }
        };

        // Backward compatibility: no groups = default group
        if config.groups.is_empty() {
            return PriorityGroup {
                pattern: "*".to_string(),
                priority: 50,
                truncate_mode: None,
                truncate: None,
            };
        }

        // Find ALL groups that match, return the one with HIGHEST priority
        let mut best_match: Option<&PriorityGroup> = None;

        for group in &config.groups {
            if Self::match_pattern(file_path, &group.pattern) {
                match best_match {
                    None => best_match = Some(group),
                    Some(current) if group.priority > current.priority => {
                        best_match = Some(group);
                    }
                    _ => {}
                }
            }
        }

        // Return best match or fallback
        match best_match {
            Some(group) => group.clone(),
            None => {
                let fallback_priority = config.fallback.as_ref().map(|f| f.priority).unwrap_or(50);
                PriorityGroup {
                    pattern: "*".to_string(),
                    priority: fallback_priority,
                    truncate_mode: None,
                    truncate: None,
                }
            }
        }
    }

    /// Get priority for a file based on the active lens configuration (v1.7.0)
    ///
    /// Returns the highest matching priority from groups, or fallback priority.
    /// Default priority is 50 if no groups defined (backward compatible).
    ///
    /// # Learning Integration (v2.2.0)
    ///
    /// When a ContextStore is available and frozen mode is disabled, the priority
    /// is blended with learned utility scores:
    /// `final = (static * 0.7) + (learned * 100 * 0.3)`
    pub fn get_file_priority(&self, file_path: &Path) -> i32 {
        let static_priority = self.get_static_priority(file_path);

        // If frozen or no store, return static priority only
        if self.frozen {
            return static_priority;
        }

        // Blend with learned priorities if store available
        match &self.context_store {
            Some(store) => {
                let path_str = file_path.to_string_lossy();
                store.blend_priority(&path_str, static_priority)
            }
            None => static_priority,
        }
    }

    /// Get static priority from lens configuration only (no learning)
    ///
    /// Used internally and for frozen mode.
    pub fn get_static_priority(&self, file_path: &Path) -> i32 {
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
        highest_priority
            .unwrap_or_else(|| config.fallback.as_ref().map(|f| f.priority).unwrap_or(50))
    }

    /// Match a file path against a glob pattern
    ///
    /// Handles both simple patterns (*.py) and recursive patterns (**/*.rs, tests/**)
    fn match_pattern(file_path: &Path, pattern: &str) -> bool {
        let file_str = file_path.to_string_lossy();
        let file_name = file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Handle ** recursive patterns
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();

            // Handle **/dirname/** pattern (matches files inside dirname anywhere)
            if parts.len() == 3 && parts[0].is_empty() && parts[2].is_empty() {
                let dirname = parts[1].trim_matches('/');
                if !dirname.is_empty() {
                    // Match if path contains /dirname/ or starts with dirname/
                    return file_str.contains(&format!("/{}/", dirname))
                        || file_str.starts_with(&format!("{}/", dirname));
                }
            }

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
                    let remaining_name = Path::new(remaining)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    return Self::simple_match(&remaining_name, suffix)
                        || Self::simple_match(remaining, &format!("*/{}", suffix));
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

        // Handle *middle* patterns (contains check) - e.g., *config*, *auth*
        if pattern.starts_with('*') && pattern.ends_with('*') && pattern.len() > 2 {
            let middle = &pattern[1..pattern.len() - 1];
            // Only handle if middle doesn't contain more wildcards
            if !middle.contains('*') {
                return text.contains(middle);
            }
        }

        // Handle *suffix patterns
        if pattern.starts_with('*') && !pattern[1..].contains('*') {
            return text.ends_with(&pattern[1..]);
        }

        // Handle prefix* patterns
        if pattern.ends_with('*') && !pattern[..pattern.len() - 1].contains('*') {
            return text.starts_with(&pattern[..pattern.len() - 1]);
        }

        // Handle prefix*suffix patterns (single *)
        if let Some(star_pos) = pattern.find('*') {
            if !pattern[star_pos + 1..].contains('*') {
                let prefix = &pattern[..star_pos];
                let suffix = &pattern[star_pos + 1..];
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

        // Create a custom lens with no groups (backward compatibility test)
        let empty_lens = LensConfig {
            description: "Empty groups test".to_string(),
            groups: vec![], // No groups = all files get default priority
            fallback: None,
            ..Default::default()
        };
        manager.custom.insert("empty".to_string(), empty_lens);
        let _ = manager.apply_lens("empty");

        // Without groups, should return default priority 50
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
        assert_eq!(
            manager.get_file_priority(Path::new("docs/unmatched.txt")),
            30
        );
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
        assert!(LensManager::match_pattern(
            Path::new("tests/unit.py"),
            "tests/**"
        ));
        assert!(LensManager::match_pattern(
            Path::new("tests/a/b/c.py"),
            "tests/**"
        ));
        assert!(!LensManager::match_pattern(
            Path::new("src/tests/x.py"),
            "tests/**"
        ));
    }

    #[test]
    fn test_pattern_extension_anywhere() {
        // **/*.rs should match .rs files anywhere
        assert!(LensManager::match_pattern(Path::new("lib.rs"), "**/*.rs"));
        assert!(LensManager::match_pattern(
            Path::new("src/lib.rs"),
            "**/*.rs"
        ));
        assert!(LensManager::match_pattern(
            Path::new("a/b/c/main.rs"),
            "**/*.rs"
        ));
    }

    #[test]
    fn test_pattern_prefix_and_suffix() {
        // src/**/*.py should match .py files under src/
        assert!(LensManager::match_pattern(
            Path::new("src/main.py"),
            "src/**/*.py"
        ));
        assert!(LensManager::match_pattern(
            Path::new("src/utils/helper.py"),
            "src/**/*.py"
        ));
        assert!(!LensManager::match_pattern(
            Path::new("tests/main.py"),
            "src/**/*.py"
        ));
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

    #[test]
    fn test_all_builtin_lenses_have_required_fields() {
        let manager = LensManager::new();
        let lens_names = vec!["architecture", "debug", "security", "onboarding"];

        for name in lens_names {
            let lens = manager.get_lens(name);
            assert!(lens.is_some(), "Lens '{}' should exist", name);
            let lens = lens.unwrap();
            assert!(
                !lens.description.is_empty(),
                "Lens '{}' should have description",
                name
            );
        }
    }

    #[test]
    fn test_architecture_lens_excludes_tests() {
        let manager = LensManager::new();
        let arch_lens = manager.get_lens("architecture").unwrap();

        // Architecture lens should exclude tests
        assert!(arch_lens.exclude.iter().any(|p| p.contains("tests")));
        assert!(arch_lens.exclude.iter().any(|p| p.contains("docs")));
    }

    #[test]
    fn test_architecture_lens_includes_code_files() {
        let manager = LensManager::new();
        let arch_lens = manager.get_lens("architecture").unwrap();

        // Architecture lens should include code files
        assert!(arch_lens.include.iter().any(|p| p.contains(".py")));
        assert!(arch_lens.include.iter().any(|p| p.contains(".rs")));
        assert!(arch_lens.include.iter().any(|p| p.contains(".json")));
    }

    #[test]
    fn test_load_custom_lens() {
        let mut manager = LensManager::new();

        let mut custom_lenses = std::collections::HashMap::new();
        custom_lenses.insert(
            "myproject".to_string(),
            LensConfig {
                description: "My custom project lens".to_string(),
                groups: vec![PriorityGroup {
                    pattern: "*.rs".to_string(),
                    priority: 100,
                    truncate_mode: None,
                    truncate: None,
                }],
                fallback: Some(FallbackConfig { priority: 25 }),
                ..Default::default()
            },
        );

        manager.load_custom(custom_lenses);

        // Custom lens should be available
        assert!(manager.get_lens("myproject").is_some());
        assert!(manager
            .available_lenses()
            .contains(&"myproject".to_string()));
    }

    #[test]
    fn test_custom_lens_overrides_builtin() {
        let mut manager = LensManager::new();

        let mut custom_lenses = std::collections::HashMap::new();
        custom_lenses.insert(
            "architecture".to_string(),
            LensConfig {
                description: "Custom architecture override".to_string(),
                ..Default::default()
            },
        );

        manager.load_custom(custom_lenses);

        // Custom should override built-in
        let lens = manager.get_lens("architecture").unwrap();
        assert_eq!(lens.description, "Custom architecture override");
    }

    #[test]
    fn test_applied_lens_fields() {
        let mut manager = LensManager::new();
        let applied = manager.apply_lens("architecture").unwrap();

        assert_eq!(applied.name, "architecture");
        assert!(!applied.description.is_empty());
        assert!(!applied.ignore_patterns.is_empty());
        assert!(!applied.include_patterns.is_empty());
        assert!(applied.truncate_lines > 0); // Architecture has truncation
    }

    #[test]
    fn test_pattern_exact_filename() {
        // Exact filename patterns
        assert!(LensManager::match_pattern(
            Path::new("Makefile"),
            "Makefile"
        ));
        assert!(LensManager::match_pattern(
            Path::new("README.md"),
            "README.md"
        ));
        assert!(!LensManager::match_pattern(
            Path::new("README.txt"),
            "README.md"
        ));
    }

    #[test]
    fn test_pattern_no_match() {
        // Patterns that shouldn't match
        assert!(!LensManager::match_pattern(Path::new("main.py"), "*.js"));
        assert!(!LensManager::match_pattern(Path::new("lib.rs"), "*.py"));
        assert!(!LensManager::match_pattern(
            Path::new("foo/bar.txt"),
            "baz/**"
        ));
    }

    #[test]
    fn test_simple_match_function() {
        // Test the simple_match helper directly
        assert!(LensManager::simple_match("main.py", "*.py"));
        assert!(LensManager::simple_match("test.rs", "*.rs"));
        assert!(!LensManager::simple_match("main.py", "*.rs"));
        assert!(LensManager::simple_match("Makefile", "Makefile"));
    }

    #[test]
    fn test_priority_fallback_default() {
        let manager = LensManager::new();
        // Without active lens, should return default 50
        assert_eq!(manager.get_file_priority(Path::new("any_file.xyz")), 50);
    }

    #[test]
    fn test_priority_with_custom_groups() {
        let mut manager = LensManager::new();

        let lens = LensConfig {
            description: "Test".to_string(),
            groups: vec![PriorityGroup {
                pattern: "*.py".to_string(),
                priority: 90,
                truncate_mode: Some("structure".to_string()),
                truncate: Some(500),
            }],
            fallback: Some(FallbackConfig { priority: 40 }),
            ..Default::default()
        };

        manager.custom.insert("test".to_string(), lens);
        let _ = manager.apply_lens("test");

        // .py file should get priority 90
        assert_eq!(manager.get_file_priority(Path::new("main.py")), 90);
        // .rs file should get fallback priority 40
        assert_eq!(manager.get_file_priority(Path::new("main.rs")), 40);
    }

    #[test]
    fn test_debug_lens_has_no_truncation() {
        let manager = LensManager::new();
        let debug_lens = manager.get_lens("debug").unwrap();

        // Debug lens should have no truncation (full content)
        assert_eq!(debug_lens.truncate, Some(0));
    }

    #[test]
    fn test_security_lens_focuses_on_sensitive_patterns() {
        let manager = LensManager::new();
        let security_lens = manager.get_lens("security").unwrap();

        // Security lens should include patterns for config/env files
        let includes = &security_lens.include;
        assert!(includes
            .iter()
            .any(|p| p.contains("config") || p.contains(".json") || p.contains(".yaml")));
    }

    #[test]
    fn test_lens_config_default() {
        let default_config = LensConfig::default();
        assert!(default_config.description.is_empty());
        assert!(default_config.exclude.is_empty());
        assert!(default_config.include.is_empty());
        assert!(default_config.groups.is_empty());
        assert!(default_config.fallback.is_none());
    }

    #[test]
    fn test_fallback_config_default_priority() {
        // When using serde default, priority should be 50
        let fallback = FallbackConfig {
            priority: default_priority(),
        };
        assert_eq!(fallback.priority, 50);
    }

    #[test]
    fn test_default_priority_function() {
        assert_eq!(default_priority(), 50);
    }

    // ============================================================
    // Coverage Floor Tests (>85% target)
    // ============================================================

    #[test]
    fn test_apply_lens_with_empty_patterns() {
        // Test apply_lens on a lens with no include/exclude patterns
        let mut manager = LensManager::new();

        // Create a minimal lens with no patterns
        let minimal_lens = LensConfig {
            description: "Minimal test lens".to_string(),
            exclude: vec![],
            include: vec![],
            sort_by: None,
            sort_order: None,
            truncate: None,
            truncate_mode: None,
            groups: vec![],
            fallback: None,
        };

        manager.custom.insert("minimal".to_string(), minimal_lens);
        let applied = manager.apply_lens("minimal").unwrap();

        assert_eq!(applied.name, "minimal");
        assert!(applied.ignore_patterns.is_empty());
        assert!(applied.include_patterns.is_empty());
        assert_eq!(applied.sort_by, "name"); // Default
        assert_eq!(applied.sort_order, "asc"); // Default
        assert_eq!(applied.truncate_lines, 0); // Default
    }

    #[test]
    fn test_apply_lens_nonexistent() {
        // Test apply_lens with non-existent lens name
        let mut manager = LensManager::new();
        let result = manager.apply_lens("nonexistent_lens_xyz");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown lens"));
    }

    #[test]
    fn test_lens_manager_new_empty_state() {
        // Test LensManager::new() initial state
        let manager = LensManager::new();
        assert!(manager.active_lens.is_none());
        assert!(manager.custom.is_empty());
        // Built-in lenses should exist
        assert!(manager.get_lens("architecture").is_some());
        assert!(manager.get_lens("debug").is_some());
        assert!(manager.get_lens("security").is_some());
        assert!(manager.get_lens("onboarding").is_some());
    }

    #[test]
    fn test_print_manifest_architecture() {
        // Test print_manifest doesn't panic
        let manager = LensManager::new();
        // This prints to stderr, just verify it doesn't panic
        manager.print_manifest("architecture");
    }

    #[test]
    fn test_print_manifest_nonexistent() {
        // Test print_manifest with non-existent lens
        let manager = LensManager::new();
        // Should not panic, just does nothing
        manager.print_manifest("nonexistent_lens");
    }

    #[test]
    fn test_print_manifest_debug() {
        // Debug lens has truncate: 0
        let manager = LensManager::new();
        manager.print_manifest("debug");
    }

    #[test]
    fn test_get_file_priority_no_active_lens() {
        // Without active lens, should return default 50
        let manager = LensManager::new();
        assert_eq!(manager.get_file_priority(Path::new("anything.py")), 50);
        assert_eq!(manager.get_file_priority(Path::new("tests/test.py")), 50);
    }

    #[test]
    fn test_get_file_priority_with_multiple_matching_groups() {
        // Test that highest priority wins when multiple groups match
        let mut manager = LensManager::new();

        let lens = LensConfig {
            description: "Multi-match test".to_string(),
            groups: vec![
                PriorityGroup {
                    pattern: "*.py".to_string(),
                    priority: 60,
                    truncate_mode: None,
                    truncate: None,
                },
                PriorityGroup {
                    pattern: "src/**/*.py".to_string(),
                    priority: 90, // Higher priority for src/
                    truncate_mode: None,
                    truncate: None,
                },
            ],
            fallback: Some(FallbackConfig { priority: 30 }),
            ..Default::default()
        };

        manager.custom.insert("multi".to_string(), lens);
        let _ = manager.apply_lens("multi");

        // src/main.py matches both patterns, should get 90 (highest)
        assert_eq!(manager.get_file_priority(Path::new("src/main.py")), 90);

        // root.py only matches *.py, should get 60
        assert_eq!(manager.get_file_priority(Path::new("root.py")), 60);

        // README.md matches nothing, should get fallback 30
        assert_eq!(manager.get_file_priority(Path::new("README.md")), 30);
    }

    #[test]
    fn test_match_pattern_recursive_glob() {
        // Test **/ recursive pattern matching
        assert!(LensManager::match_pattern(
            Path::new("src/lib/utils.py"),
            "src/**/*.py"
        ));
        assert!(LensManager::match_pattern(
            Path::new("tests/unit/test_core.py"),
            "tests/**/*.py"
        ));
        assert!(!LensManager::match_pattern(
            Path::new("docs/readme.md"),
            "tests/**/*.py"
        ));
    }

    #[test]
    fn test_match_pattern_directory_prefix() {
        // Test directory/ prefix patterns
        assert!(LensManager::match_pattern(
            Path::new("tests/test_main.py"),
            "tests/**"
        ));
        assert!(LensManager::match_pattern(
            Path::new("src/module/file.rs"),
            "src/**"
        ));
    }

    #[test]
    fn test_applied_lens_all_fields() {
        // Test all fields of AppliedLens
        let mut manager = LensManager::new();

        let lens = LensConfig {
            description: "Full test".to_string(),
            exclude: vec!["*.log".to_string()],
            include: vec!["*.py".to_string()],
            sort_by: Some("mtime".to_string()),
            sort_order: Some("desc".to_string()),
            truncate: Some(100),
            truncate_mode: Some("smart".to_string()),
            groups: vec![],
            fallback: None,
        };

        manager.custom.insert("full".to_string(), lens);
        let applied = manager.apply_lens("full").unwrap();

        assert_eq!(applied.name, "full");
        assert_eq!(applied.description, "Full test");
        assert_eq!(applied.ignore_patterns, vec!["*.log".to_string()]);
        assert_eq!(applied.include_patterns, vec!["*.py".to_string()]);
        assert_eq!(applied.sort_by, "mtime");
        assert_eq!(applied.sort_order, "desc");
        assert_eq!(applied.truncate_lines, 100);
        assert_eq!(applied.truncate_mode, "smart");
    }

    #[test]
    fn test_load_custom_overwrites_existing() {
        // Test that load_custom properly overwrites
        let mut manager = LensManager::new();

        let mut custom1 = std::collections::HashMap::new();
        custom1.insert(
            "test".to_string(),
            LensConfig {
                description: "First".to_string(),
                ..Default::default()
            },
        );
        manager.load_custom(custom1);

        let mut custom2 = std::collections::HashMap::new();
        custom2.insert(
            "test".to_string(),
            LensConfig {
                description: "Second".to_string(),
                ..Default::default()
            },
        );
        manager.load_custom(custom2);

        let lens = manager.get_lens("test").unwrap();
        assert_eq!(lens.description, "Second");
    }

    #[test]
    fn test_priority_group_with_truncate_overrides() {
        // Test PriorityGroup with truncate_mode and truncate fields
        let group = PriorityGroup {
            pattern: "tests/**".to_string(),
            priority: 20,
            truncate_mode: Some("structure".to_string()),
            truncate: Some(50),
        };

        assert_eq!(group.pattern, "tests/**");
        assert_eq!(group.priority, 20);
        assert_eq!(group.truncate_mode, Some("structure".to_string()));
        assert_eq!(group.truncate, Some(50));
    }

    // ============================================================
    // Phase 1 TDD: Priority Groups in Built-in Lenses
    // ============================================================

    #[test]
    fn test_architecture_lens_has_priority_groups() {
        let manager = LensManager::new();
        let lens = manager.get_lens("architecture").unwrap();
        assert!(
            !lens.groups.is_empty(),
            "Architecture lens should have priority groups"
        );
        assert!(
            lens.groups.len() >= 15,
            "Should have at least 15 group patterns"
        );
    }

    #[test]
    fn test_architecture_lens_python_priority() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("architecture");
        let priority = manager.get_file_priority(Path::new("main.py"));
        assert_eq!(priority, 100, "Python files should have priority 100");
    }

    #[test]
    fn test_architecture_lens_rust_priority() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("architecture");
        let priority = manager.get_file_priority(Path::new("src/lib.rs"));
        assert!(priority >= 95, "Rust files should have priority >= 95");
    }

    #[test]
    fn test_architecture_lens_config_priority() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("architecture");

        // Cargo.toml should have high priority
        let cargo_priority = manager.get_file_priority(Path::new("Cargo.toml"));
        assert_eq!(cargo_priority, 90, "Cargo.toml should have priority 90");

        // Generic JSON should have priority 80
        let json_priority = manager.get_file_priority(Path::new("config.json"));
        assert_eq!(json_priority, 80, "JSON files should have priority 80");
    }

    #[test]
    fn test_architecture_lens_fallback_priority() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("architecture");
        let priority = manager.get_file_priority(Path::new("random.xyz"));
        assert_eq!(
            priority, 50,
            "Unknown files should have fallback priority 50"
        );
    }

    #[test]
    fn test_architecture_lens_javascript_priority() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("architecture");

        let ts_priority = manager.get_file_priority(Path::new("app.ts"));
        assert_eq!(ts_priority, 60, "TypeScript files should have priority 60");

        let js_priority = manager.get_file_priority(Path::new("app.js"));
        assert_eq!(js_priority, 55, "JavaScript files should have priority 55");
    }

    // ============================================================
    // Phase 2: Learning Integration (Context Store v2)
    // ============================================================

    #[test]
    fn test_lens_manager_with_store() {
        let store = ContextStore::new();
        let manager = LensManager::with_store(store);
        assert!(manager.store().is_some());
    }

    #[test]
    fn test_lens_manager_set_store() {
        let mut manager = LensManager::new();
        assert!(manager.store().is_none());

        let store = ContextStore::new();
        manager.set_store(store);
        assert!(manager.store().is_some());
    }

    #[test]
    fn test_frozen_mode_ignores_store() {
        let mut store = ContextStore::new();
        // Train the store to prefer this file highly
        for _ in 0..10 {
            store.report_utility("test.py", 1.0, 0.3);
        }

        let mut manager = LensManager::with_store(store);
        let _ = manager.apply_lens("architecture");

        // Without frozen: should get blended priority
        let priority_normal = manager.get_file_priority(Path::new("test.py"));

        // With frozen: should get static priority (100 for .py)
        manager.set_frozen(true);
        let priority_frozen = manager.get_file_priority(Path::new("test.py"));

        assert_eq!(priority_frozen, 100, "Frozen should return static priority");
        // Normal should be blended: (100 * 0.7) + (1.0 * 100 * 0.3) = 100
        // In this case they're the same because max utility = max priority blend
        assert!(
            priority_normal >= 95,
            "Normal should have high blended priority"
        );
    }

    #[test]
    fn test_priority_blend_high_utility() {
        let mut store = ContextStore::new();
        // Train high utility
        for _ in 0..10 {
            store.report_utility("important.xyz", 1.0, 0.3);
        }

        let mut manager = LensManager::with_store(store);
        let _ = manager.apply_lens("architecture");

        // Fallback is 50 for unknown extension
        // Blended: (50 * 0.7) + (1.0 * 100 * 0.3) = 35 + 30 = 65
        let priority = manager.get_file_priority(Path::new("important.xyz"));
        assert!(
            priority >= 60 && priority <= 70,
            "Expected ~65, got {}",
            priority
        );
    }

    #[test]
    fn test_priority_blend_low_utility() {
        let mut store = ContextStore::new();
        // Train low utility
        for _ in 0..10 {
            store.report_utility("useless.py", 0.0, 0.3);
        }

        let mut manager = LensManager::with_store(store);
        let _ = manager.apply_lens("architecture");

        // Static for .py is 100
        // Blended: (100 * 0.7) + (0.0 * 100 * 0.3) = 70 + 0 = 70
        let priority = manager.get_file_priority(Path::new("useless.py"));
        assert!(
            priority >= 65 && priority <= 75,
            "Expected ~70, got {}",
            priority
        );
    }

    #[test]
    fn test_static_priority_unchanged() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("architecture");

        // Without store, should return static priority
        let static_priority = manager.get_static_priority(Path::new("main.py"));
        let priority = manager.get_file_priority(Path::new("main.py"));

        assert_eq!(static_priority, priority);
        assert_eq!(static_priority, 100);
    }

    #[test]
    fn test_store_mut_access() {
        let mut manager = LensManager::new();
        manager.set_store(ContextStore::new());

        // Report utility via mutable store access
        if let Some(store) = manager.store_mut() {
            store.report_utility("test.rs", 0.9, 0.3);
        }

        // Verify it was recorded
        if let Some(store) = manager.store() {
            let score = store.get_utility_score("test.rs");
            assert!(score > 0.5, "Score should have increased");
        }
    }

    #[test]
    fn test_is_frozen_default() {
        let manager = LensManager::new();
        assert!(!manager.is_frozen());
    }

    #[test]
    fn test_set_frozen() {
        let mut manager = LensManager::new();
        manager.set_frozen(true);
        assert!(manager.is_frozen());
        manager.set_frozen(false);
        assert!(!manager.is_frozen());
    }

    // ============================================================
    // Lens Differentiation Tests (v2.3.0)
    // Verifies that different lenses produce meaningfully different
    // priority orderings for the same files.
    // ============================================================

    #[test]
    fn test_onboarding_lens_has_priority_groups() {
        let manager = LensManager::new();
        let lens = manager.get_lens("onboarding").unwrap();
        assert!(
            !lens.groups.is_empty(),
            "Onboarding lens must have priority groups"
        );
        assert!(
            lens.groups.len() >= 20,
            "Onboarding should have comprehensive patterns"
        );
    }

    #[test]
    fn test_security_lens_has_priority_groups() {
        let manager = LensManager::new();
        let lens = manager.get_lens("security").unwrap();
        assert!(
            !lens.groups.is_empty(),
            "Security lens must have priority groups"
        );
        assert!(
            lens.groups.len() >= 25,
            "Security should have comprehensive patterns"
        );
    }

    #[test]
    fn test_onboarding_prioritizes_documentation() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("onboarding");

        // Documentation should be highest priority
        let readme_priority = manager.get_file_priority(Path::new("README.md"));
        let source_priority = manager.get_file_priority(Path::new("src/main.rs"));
        let test_priority = manager.get_file_priority(Path::new("tests/test.py"));

        assert!(
            readme_priority > source_priority,
            "README ({}) should have higher priority than source ({})",
            readme_priority,
            source_priority
        );
        assert!(
            source_priority > test_priority,
            "Source ({}) should have higher priority than tests ({})",
            source_priority,
            test_priority
        );
    }

    #[test]
    fn test_security_prioritizes_server_over_docs() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("security");

        // Server code should be higher priority than documentation
        let server_priority = manager.get_file_priority(Path::new("src/server/handler.rs"));
        let readme_priority = manager.get_file_priority(Path::new("README.md"));

        assert!(
            server_priority > readme_priority,
            "Server code ({}) should have higher priority than README ({}) for security",
            server_priority,
            readme_priority
        );
    }

    #[test]
    fn test_lens_differentiation_readme() {
        // README.md should have different priorities in different lenses
        let mut onboarding_mgr = LensManager::new();
        let mut security_mgr = LensManager::new();

        let _ = onboarding_mgr.apply_lens("onboarding");
        let _ = security_mgr.apply_lens("security");

        let onboarding_priority = onboarding_mgr.get_file_priority(Path::new("README.md"));
        let security_priority = security_mgr.get_file_priority(Path::new("README.md"));

        // Onboarding should prioritize README much higher than security
        assert!(
            onboarding_priority > security_priority,
            "Onboarding ({}) should prioritize README higher than security ({})",
            onboarding_priority,
            security_priority
        );
    }

    #[test]
    fn test_lens_differentiation_auth_file() {
        // Auth file should have different priorities in different lenses
        let mut onboarding_mgr = LensManager::new();
        let mut security_mgr = LensManager::new();

        let _ = onboarding_mgr.apply_lens("onboarding");
        let _ = security_mgr.apply_lens("security");

        let onboarding_priority = onboarding_mgr.get_file_priority(Path::new("src/auth.rs"));
        let security_priority = security_mgr.get_file_priority(Path::new("src/auth.rs"));

        // Security should prioritize auth files much higher than onboarding
        assert!(
            security_priority > onboarding_priority,
            "Security ({}) should prioritize auth.rs higher than onboarding ({})",
            security_priority,
            onboarding_priority
        );
    }

    #[test]
    fn test_lens_differentiation_cargo_toml() {
        // Cargo.toml should have reasonable priority in both, but different
        let mut onboarding_mgr = LensManager::new();
        let mut security_mgr = LensManager::new();

        let _ = onboarding_mgr.apply_lens("onboarding");
        let _ = security_mgr.apply_lens("security");

        let onboarding_priority = onboarding_mgr.get_file_priority(Path::new("Cargo.toml"));
        let security_priority = security_mgr.get_file_priority(Path::new("Cargo.toml"));

        // Both should have high priority (>=80) but can differ
        assert!(
            onboarding_priority >= 80,
            "Onboarding should value Cargo.toml highly"
        );
        assert!(
            security_priority >= 80,
            "Security should value Cargo.toml highly"
        );
    }

    #[test]
    fn test_onboarding_fallback_is_low() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("onboarding");

        // Unknown files should get low fallback priority
        let priority = manager.get_file_priority(Path::new("random_impl_detail.xyz"));
        assert!(
            priority <= 25,
            "Onboarding fallback ({}) should be low",
            priority
        );
    }

    #[test]
    fn test_security_fallback_is_low() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("security");

        // Unknown files should get low fallback priority
        let priority = manager.get_file_priority(Path::new("random_impl_detail.xyz"));
        assert!(
            priority <= 30,
            "Security fallback ({}) should be low",
            priority
        );
    }

    #[test]
    fn test_security_config_patterns() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("security");

        // Config-related files should get high priority
        let config_priority = manager.get_file_priority(Path::new("src/config.rs"));
        let settings_priority = manager.get_file_priority(Path::new("settings.py"));

        assert!(
            config_priority >= 80,
            "config.rs ({}) should be high priority",
            config_priority
        );
        assert!(
            settings_priority >= 80,
            "settings.py ({}) should be high priority",
            settings_priority
        );
    }

    #[test]
    fn test_security_api_patterns() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("security");

        // API and server files should get high priority
        let api_priority = manager.get_file_priority(Path::new("src/api/routes.rs"));
        let server_priority = manager.get_file_priority(Path::new("server/main.rs"));

        assert!(
            api_priority >= 75,
            "API file ({}) should be high priority",
            api_priority
        );
        assert!(
            server_priority >= 75,
            "Server file ({}) should be high priority",
            server_priority
        );
    }

    #[test]
    fn test_onboarding_entry_points() {
        let mut manager = LensManager::new();
        let _ = manager.apply_lens("onboarding");

        // Entry point files should get high priority
        let main_rs = manager.get_file_priority(Path::new("src/main.rs"));
        let lib_rs = manager.get_file_priority(Path::new("src/lib.rs"));
        let mod_rs = manager.get_file_priority(Path::new("src/core/mod.rs"));

        assert!(
            main_rs >= 70,
            "main.rs ({}) should be high priority",
            main_rs
        );
        assert!(lib_rs >= 70, "lib.rs ({}) should be high priority", lib_rs);
        assert!(
            mod_rs >= 65,
            "mod.rs ({}) should be moderately high priority",
            mod_rs
        );
    }
}
