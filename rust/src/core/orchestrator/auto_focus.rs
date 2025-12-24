//! Auto-Focus Module
//!
//! Analyzes input paths and determines optimal default settings.
//! Like a camera's autofocus, this module adjusts the "lens" based on what
//! the user is looking at.

use std::path::Path;

use super::smart_defaults::{SmartDefaults, SemanticDepth, DetailLevel};

// =============================================================================
// Input Type Detection
// =============================================================================

/// Type of input being analyzed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// A single file (microscope mode)
    SingleFile,
    /// A directory (wide-angle mode)
    Directory,
    /// A small project (< 50 files)
    SmallProject,
    /// A large project (> 500 files)
    LargeProject,
    /// A monorepo (multiple distinct projects)
    Monorepo,
}

impl InputType {
    /// Detect input type from a path.
    pub fn detect(path: &Path) -> Self {
        if path.is_file() {
            return Self::SingleFile;
        }

        if !path.is_dir() {
            // Path doesn't exist yet, assume directory
            return Self::Directory;
        }

        // Count files to determine project size
        let file_count = count_files_quick(path, 1000);

        // Check for monorepo indicators
        let is_monorepo = path.join("packages").is_dir()
            || path.join("apps").is_dir()
            || path.join("services").is_dir()
            || (path.join("lerna.json").exists() || path.join("pnpm-workspace.yaml").exists());

        if is_monorepo {
            Self::Monorepo
        } else if file_count > 500 {
            Self::LargeProject
        } else if file_count < 50 {
            Self::SmallProject
        } else {
            Self::Directory
        }
    }
}

/// Quick file count (stops at limit for performance).
fn count_files_quick(path: &Path, limit: usize) -> usize {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if count >= limit {
                return count;
            }

            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                // Skip hidden and common ignore directories
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with('.')
                        && name != "node_modules"
                        && name != "target"
                        && name != "__pycache__"
                        && name != "venv"
                        && name != ".git"
                    {
                        count += count_files_quick(&path, limit - count);
                    }
                }
            }
        }
    }

    count
}

// =============================================================================
// Auto-Focus Logic
// =============================================================================

/// Auto-focus system for intelligent default selection.
pub struct AutoFocus {
    /// Default truncation for directories
    default_directory_truncate: usize,
    /// Default truncation for large projects
    default_large_project_truncate: usize,
}

impl Default for AutoFocus {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoFocus {
    /// Create a new auto-focus system.
    pub fn new() -> Self {
        Self {
            default_directory_truncate: 100,
            default_large_project_truncate: 50,
        }
    }

    /// Analyze a path and return smart defaults.
    pub fn analyze(&self, path: &Path) -> SmartDefaults {
        let input_type = InputType::detect(path);
        self.defaults_for_type(input_type)
    }

    /// Get defaults for a specific input type.
    pub fn defaults_for_type(&self, input_type: InputType) -> SmartDefaults {
        match input_type {
            InputType::SingleFile => SmartDefaults {
                // Microscope mode: show everything
                truncate_lines: Some(0),  // No truncation
                lens: Some("architecture".to_string()),
                semantic_depth: SemanticDepth::Deep,
                detail_level: DetailLevel::Detailed,
                estimated_tokens: None,
            },

            InputType::SmallProject => SmartDefaults {
                // Compact project: moderate truncation
                truncate_lines: Some(200),
                lens: Some("architecture".to_string()),
                semantic_depth: SemanticDepth::Balanced,
                detail_level: DetailLevel::Smart,
                estimated_tokens: None,
            },

            InputType::Directory => SmartDefaults {
                // Standard directory: default truncation
                truncate_lines: Some(self.default_directory_truncate),
                lens: Some("architecture".to_string()),
                semantic_depth: SemanticDepth::Balanced,
                detail_level: DetailLevel::Smart,
                estimated_tokens: None,
            },

            InputType::LargeProject => SmartDefaults {
                // Large project: aggressive truncation
                truncate_lines: Some(self.default_large_project_truncate),
                lens: Some("architecture".to_string()),
                semantic_depth: SemanticDepth::Quick,
                detail_level: DetailLevel::Summary,
                estimated_tokens: None,
            },

            InputType::Monorepo => SmartDefaults {
                // Monorepo: very aggressive, skeleton mode recommended
                truncate_lines: Some(30),
                lens: Some("architecture".to_string()),
                semantic_depth: SemanticDepth::Quick,
                detail_level: DetailLevel::Summary,
                estimated_tokens: None,
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_input_type_single_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "fn main() {}").unwrap();

        assert_eq!(InputType::detect(&file), InputType::SingleFile);
    }

    #[test]
    fn test_input_type_directory() {
        let dir = tempdir().unwrap();

        // Create a few files
        for i in 0..10 {
            fs::write(dir.path().join(format!("file{}.rs", i)), "// content").unwrap();
        }

        let input_type = InputType::detect(dir.path());
        assert_eq!(input_type, InputType::SmallProject);
    }

    #[test]
    fn test_auto_focus_file() {
        let auto_focus = AutoFocus::new();
        let defaults = auto_focus.defaults_for_type(InputType::SingleFile);

        // Files should have no truncation
        assert_eq!(defaults.truncate_lines, Some(0));
        assert_eq!(defaults.semantic_depth, SemanticDepth::Deep);
    }

    #[test]
    fn test_auto_focus_directory() {
        let auto_focus = AutoFocus::new();
        let defaults = auto_focus.defaults_for_type(InputType::Directory);

        // Directories should have truncation
        assert_eq!(defaults.truncate_lines, Some(100));
        assert_eq!(defaults.semantic_depth, SemanticDepth::Balanced);
    }

    #[test]
    fn test_auto_focus_large_project() {
        let auto_focus = AutoFocus::new();
        let defaults = auto_focus.defaults_for_type(InputType::LargeProject);

        // Large projects should have aggressive truncation
        assert_eq!(defaults.truncate_lines, Some(50));
        assert_eq!(defaults.semantic_depth, SemanticDepth::Quick);
    }
}
