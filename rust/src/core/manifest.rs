//! Project boundary detection and classification.
//!
//! This module detects project roots by looking for manifest files
//! (Cargo.toml, package.json, etc.) and classifies project types.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Detected project type based on manifest files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectType {
    /// Rust project (Cargo.toml)
    Rust,
    /// Node.js project (package.json)
    Node,
    /// Python project (pyproject.toml, setup.py, requirements.txt)
    Python,
    /// Go project (go.mod)
    Go,
    /// Multiple project types detected
    Mixed,
    /// No markers found
    Unknown,
}

/// Project boundary information.
#[derive(Debug, Clone)]
pub struct ProjectManifest {
    /// Detected project root (where markers are found).
    pub root: PathBuf,

    /// Type of project based on manifest files.
    pub project_type: ProjectType,

    /// Manifest files found (Cargo.toml, package.json, etc.).
    pub manifest_files: Vec<PathBuf>,

    /// Whether this is a workspace/monorepo.
    pub is_workspace: bool,
}

impl ProjectManifest {
    /// Marker files that indicate project root.
    const MARKERS: &'static [(&'static str, ProjectType)] = &[
        ("Cargo.toml", ProjectType::Rust),
        ("package.json", ProjectType::Node),
        ("pyproject.toml", ProjectType::Python),
        ("setup.py", ProjectType::Python),
        ("go.mod", ProjectType::Go),
        (".git", ProjectType::Unknown), // Git root as fallback
    ];

    /// Detect project manifest starting from given path.
    /// Walks up directory tree looking for marker files.
    pub fn detect(start_path: &Path) -> Self {
        let start = if start_path.is_file() {
            start_path.parent().unwrap_or(start_path)
        } else {
            start_path
        };

        let canonical = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());

        let mut current = Some(canonical.as_path());
        let mut found_markers: Vec<(PathBuf, ProjectType)> = Vec::new();
        let mut root = canonical.clone();

        while let Some(dir) = current {
            for (marker, project_type) in Self::MARKERS {
                let marker_path = dir.join(marker);
                if marker_path.exists() {
                    found_markers.push((marker_path, project_type.clone()));
                    root = dir.to_path_buf();
                }
            }

            // Stop at .git (definitive project root)
            if dir.join(".git").exists() {
                root = dir.to_path_buf();
                break;
            }

            current = dir.parent();
        }

        // Determine project type
        let project_type = Self::determine_type(&found_markers);

        // Check for workspace patterns
        let is_workspace = Self::detect_workspace(&root, &project_type);

        Self {
            root,
            project_type,
            manifest_files: found_markers.into_iter().map(|(p, _)| p).collect(),
            is_workspace,
        }
    }

    /// Determine project type from found markers.
    fn determine_type(markers: &[(PathBuf, ProjectType)]) -> ProjectType {
        let types: HashSet<_> = markers
            .iter()
            .filter(|(_, t)| *t != ProjectType::Unknown)
            .map(|(_, t)| t.clone())
            .collect();

        match types.len() {
            0 => ProjectType::Unknown,
            1 => types.into_iter().next().unwrap(),
            _ => ProjectType::Mixed,
        }
    }

    /// Detect if this is a workspace/monorepo.
    fn detect_workspace(root: &Path, project_type: &ProjectType) -> bool {
        match project_type {
            ProjectType::Rust => {
                // Check Cargo.toml for [workspace]
                let cargo_toml = root.join("Cargo.toml");
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    content.contains("[workspace]")
                } else {
                    false
                }
            }
            ProjectType::Node => {
                // Check package.json for "workspaces"
                let package_json = root.join("package.json");
                if let Ok(content) = std::fs::read_to_string(&package_json) {
                    content.contains("\"workspaces\"")
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Get the project root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Check if the project is a workspace/monorepo.
    pub fn is_workspace(&self) -> bool {
        self.is_workspace
    }

    /// Get the project type.
    pub fn project_type(&self) -> &ProjectType {
        &self.project_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();

        let manifest = ProjectManifest::detect(&tmp.path().join("src"));

        assert_eq!(manifest.project_type, ProjectType::Rust);
        assert_eq!(manifest.root, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn test_detect_node_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.project_type, ProjectType::Node);
    }

    #[test]
    fn test_detect_python_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "[project]").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.project_type, ProjectType::Python);
    }

    #[test]
    fn test_detect_go_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.project_type, ProjectType::Go);
    }

    #[test]
    fn test_detect_mixed_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.project_type, ProjectType::Mixed);
    }

    #[test]
    fn test_detect_rust_workspace() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]",
        )
        .unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert!(manifest.is_workspace);
        assert_eq!(manifest.project_type, ProjectType::Rust);
    }

    #[test]
    fn test_detect_node_workspace() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
        .unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert!(manifest.is_workspace);
        assert_eq!(manifest.project_type, ProjectType::Node);
    }

    #[test]
    fn test_fallback_to_current_dir() {
        let tmp = TempDir::new().unwrap();
        // No markers - should use start path as root

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.project_type, ProjectType::Unknown);
    }

    #[test]
    fn test_detect_from_nested_directory() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src/nested/deep")).unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("src/nested/deep/file.rs"), "code").unwrap();

        // Start from deep nested directory
        let manifest = ProjectManifest::detect(&tmp.path().join("src/nested/deep"));

        // Root should be detected at Cargo.toml level
        assert_eq!(manifest.root, tmp.path().canonicalize().unwrap());
        assert_eq!(manifest.project_type, ProjectType::Rust);
    }

    #[test]
    fn test_git_stops_traversal() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let manifest = ProjectManifest::detect(&tmp.path().join("subdir"));

        // Should stop at .git
        assert_eq!(manifest.root, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn test_manifest_files_collected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.manifest_files.len(), 2);
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_detect_from_file_path() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        // Pass a file path (not directory)
        let manifest = ProjectManifest::detect(&file_path);

        assert_eq!(manifest.project_type, ProjectType::Rust);
    }

    #[test]
    fn test_accessor_methods() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[workspace]\nmembers = []",
        )
        .unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        // Test root() accessor
        assert_eq!(manifest.root(), tmp.path().canonicalize().unwrap());

        // Test is_workspace() accessor
        assert!(manifest.is_workspace());

        // Test project_type() accessor
        assert_eq!(manifest.project_type(), &ProjectType::Rust);
    }

    #[test]
    fn test_rust_non_workspace() {
        let tmp = TempDir::new().unwrap();
        // Cargo.toml without [workspace]
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"single\"").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert!(!manifest.is_workspace());
        assert_eq!(manifest.project_type, ProjectType::Rust);
    }

    #[test]
    fn test_node_non_workspace() {
        let tmp = TempDir::new().unwrap();
        // package.json without workspaces
        fs::write(tmp.path().join("package.json"), r#"{"name": "single"}"#).unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert!(!manifest.is_workspace());
        assert_eq!(manifest.project_type, ProjectType::Node);
    }

    #[test]
    fn test_workspace_detection_go() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        // Go doesn't have workspace detection, should be false
        assert!(!manifest.is_workspace());
    }

    #[test]
    fn test_workspace_detection_python() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "[project]").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        // Python doesn't have workspace detection in this implementation
        assert!(!manifest.is_workspace());
    }

    #[test]
    fn test_workspace_detection_unknown() {
        let tmp = TempDir::new().unwrap();
        // No manifest files

        let manifest = ProjectManifest::detect(tmp.path());

        assert!(!manifest.is_workspace());
        assert_eq!(manifest.project_type, ProjectType::Unknown);
    }

    #[test]
    fn test_setup_py_detection() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("setup.py"), "from setuptools import setup").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        assert_eq!(manifest.project_type, ProjectType::Python);
    }

    #[test]
    fn test_project_type_equality() {
        assert_eq!(ProjectType::Rust, ProjectType::Rust);
        assert_eq!(ProjectType::Node, ProjectType::Node);
        assert_eq!(ProjectType::Python, ProjectType::Python);
        assert_eq!(ProjectType::Go, ProjectType::Go);
        assert_eq!(ProjectType::Mixed, ProjectType::Mixed);
        assert_eq!(ProjectType::Unknown, ProjectType::Unknown);
        assert_ne!(ProjectType::Rust, ProjectType::Python);
    }

    #[test]
    fn test_manifest_clone() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());
        let cloned = manifest.clone();

        assert_eq!(manifest.root, cloned.root);
        assert_eq!(manifest.project_type, cloned.project_type);
    }

    #[test]
    fn test_project_type_debug() {
        let debug_str = format!("{:?}", ProjectType::Rust);
        assert!(debug_str.contains("Rust"));

        let debug_str = format!("{:?}", ProjectType::Mixed);
        assert!(debug_str.contains("Mixed"));
    }

    #[test]
    fn test_manifest_debug() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());
        let debug_str = format!("{:?}", manifest);

        assert!(debug_str.contains("ProjectManifest"));
        assert!(debug_str.contains("Rust"));
    }

    #[test]
    fn test_project_type_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ProjectType::Rust);
        set.insert(ProjectType::Python);
        set.insert(ProjectType::Rust); // duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ProjectType::Rust));
        assert!(set.contains(&ProjectType::Python));
    }

    #[test]
    fn test_project_type_clone() {
        let original = ProjectType::Go;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_manifest_files_accessor() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        // Should have 3 manifest files
        assert_eq!(manifest.manifest_files.len(), 3);
    }

    #[test]
    fn test_markers_constant() {
        // Verify the MARKERS constant is accessible and correct
        assert!(ProjectManifest::MARKERS.iter().any(|(m, _)| *m == "Cargo.toml"));
        assert!(ProjectManifest::MARKERS.iter().any(|(m, _)| *m == "package.json"));
        assert!(ProjectManifest::MARKERS.iter().any(|(m, _)| *m == "go.mod"));
        assert!(ProjectManifest::MARKERS.iter().any(|(m, _)| *m == ".git"));
    }

    #[test]
    fn test_workspace_detection_mixed() {
        let tmp = TempDir::new().unwrap();
        // Mixed project with both Rust workspace and Node workspace
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[workspace]\nmembers = []",
        ).unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        ).unwrap();

        let manifest = ProjectManifest::detect(tmp.path());

        // Mixed type, but workspace detection runs for first matching type
        assert_eq!(manifest.project_type, ProjectType::Mixed);
        // Mixed doesn't trigger workspace detection
        assert!(!manifest.is_workspace());
    }
}
