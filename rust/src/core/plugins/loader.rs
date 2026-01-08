//! Plugin Loader - Discovery and Manifest System
//!
//! Discovers and loads plugins from standard paths:
//! 1. `.vo/plugins/` - Project-local plugins
//! 2. `~/.config/vo/plugins/` - User-global plugins

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(feature = "plugins")]
use super::bridges::vo_table::{create_vo_table, PluginContributions, SharedContributions};
use super::error::{PluginError, PluginResult};
#[cfg(feature = "plugins")]
use super::sandbox::IronSandbox;
#[cfg(feature = "plugins")]
use std::sync::{Arc, Mutex};

/// Current API version for compatibility checking
pub const CURRENT_API_VERSION: &str = "3.0";

/// Plugin manifest file name
pub const MANIFEST_FILE: &str = "manifest.json";

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Required API version
    pub vo_api_version: String,
    /// List of plugins in this manifest
    pub plugins: Vec<PluginEntry>,
}

/// Individual plugin entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Plugin name (for display and logging)
    pub name: String,
    /// Lua file path (relative to manifest directory)
    pub file: String,
    /// Whether plugin is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Priority for execution order (higher = first)
    #[serde(default)]
    pub priority: i32,
    /// Optional description
    #[serde(default)]
    pub description: String,
    /// Optional author
    #[serde(default)]
    pub author: String,
    /// Optional version
    #[serde(default)]
    pub version: String,
}

fn default_enabled() -> bool {
    true
}

/// Loaded plugin with runtime information
#[derive(Debug)]
pub struct LoadedPlugin {
    /// Plugin entry from manifest
    pub entry: PluginEntry,
    /// Full path to the Lua file
    pub path: PathBuf,
    /// Source code (loaded once)
    pub source: String,
    /// Load status
    pub status: PluginStatus,
}

/// Plugin load/execution status
#[derive(Debug, Clone)]
pub enum PluginStatus {
    /// Plugin loaded successfully
    Loaded,
    /// Plugin executed successfully
    Executed,
    /// Plugin failed to load
    LoadError(String),
    /// Plugin failed during execution
    ExecutionError(String),
    /// Plugin disabled in manifest
    Disabled,
}

/// Plugin loader and discovery system
pub struct PluginLoader {
    /// Search paths for plugins
    search_paths: Vec<PathBuf>,
    /// Discovered plugins
    plugins: Vec<LoadedPlugin>,
}

impl PluginLoader {
    /// Create a new plugin loader with default search paths
    pub fn new() -> Self {
        let mut paths = Vec::new();

        // Local project plugins
        if let Ok(cwd) = std::env::current_dir() {
            paths.push(cwd.join(".vo/plugins"));
        }

        // User-global plugins
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("vo/plugins"));
        }

        // Legacy path support
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config/vo/plugins"));
        }

        Self {
            search_paths: paths,
            plugins: Vec::new(),
        }
    }

    /// Create a loader with custom search paths
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths: paths,
            plugins: Vec::new(),
        }
    }

    /// Add a search path
    pub fn add_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Get configured search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Discover all plugins from search paths
    pub fn discover(&mut self) -> Vec<&LoadedPlugin> {
        self.plugins.clear();

        for path in &self.search_paths.clone() {
            if let Ok(plugins) = self.discover_in_path(path) {
                self.plugins.extend(plugins);
            }
        }

        // Sort by priority (higher first)
        self.plugins
            .sort_by(|a, b| b.entry.priority.cmp(&a.entry.priority));

        self.plugins.iter().collect()
    }

    /// Discover plugins in a specific path
    fn discover_in_path(&self, path: &Path) -> PluginResult<Vec<LoadedPlugin>> {
        let manifest_path = path.join(MANIFEST_FILE);

        if !manifest_path.exists() {
            return Ok(Vec::new());
        }

        // Read and parse manifest
        let contents = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = serde_json::from_str(&contents)
            .map_err(|e| PluginError::InvalidManifest(e.to_string()))?;

        // Check API version
        if manifest.vo_api_version != CURRENT_API_VERSION {
            return Err(PluginError::ApiVersionMismatch {
                expected: CURRENT_API_VERSION.to_string(),
                actual: manifest.vo_api_version,
            });
        }

        // Load each plugin
        let mut loaded = Vec::new();
        for entry in manifest.plugins {
            let plugin_path = path.join(&entry.file);

            if !entry.enabled {
                loaded.push(LoadedPlugin {
                    entry,
                    path: plugin_path,
                    source: String::new(),
                    status: PluginStatus::Disabled,
                });
                continue;
            }

            if !plugin_path.exists() {
                loaded.push(LoadedPlugin {
                    entry: entry.clone(),
                    path: plugin_path.clone(),
                    source: String::new(),
                    status: PluginStatus::LoadError(format!("File not found: {:?}", plugin_path)),
                });
                continue;
            }

            match std::fs::read_to_string(&plugin_path) {
                Ok(source) => {
                    loaded.push(LoadedPlugin {
                        entry,
                        path: plugin_path,
                        source,
                        status: PluginStatus::Loaded,
                    });
                }
                Err(e) => {
                    loaded.push(LoadedPlugin {
                        entry,
                        path: plugin_path,
                        source: String::new(),
                        status: PluginStatus::LoadError(e.to_string()),
                    });
                }
            }
        }

        Ok(loaded)
    }

    /// Get all discovered plugins
    pub fn plugins(&self) -> &[LoadedPlugin] {
        &self.plugins
    }

    /// Get enabled plugins
    pub fn enabled_plugins(&self) -> Vec<&LoadedPlugin> {
        self.plugins
            .iter()
            .filter(|p| matches!(p.status, PluginStatus::Loaded | PluginStatus::Executed))
            .collect()
    }

    /// Get plugin names
    pub fn plugin_names(&self) -> Vec<&str> {
        self.enabled_plugins()
            .iter()
            .map(|p| p.entry.name.as_str())
            .collect()
    }

    /// Execute all loaded plugins in a sandbox
    #[cfg(feature = "plugins")]
    pub fn execute_all(&mut self) -> PluginResult<SharedContributions> {
        let contributions = Arc::new(Mutex::new(PluginContributions::default()));

        for i in 0..self.plugins.len() {
            if !matches!(self.plugins[i].status, PluginStatus::Loaded) {
                continue;
            }

            // Execute the plugin (can't borrow self while iterating)
            match Self::execute_single_plugin(&self.plugins[i].source, contributions.clone()) {
                Ok(_) => {
                    self.plugins[i].status = PluginStatus::Executed;
                }
                Err(e) => {
                    self.plugins[i].status = PluginStatus::ExecutionError(e.to_string());
                }
            }
        }

        Ok(contributions)
    }

    /// Execute a single plugin script in a sandbox
    #[cfg(feature = "plugins")]
    fn execute_single_plugin(source: &str, contributions: SharedContributions) -> PluginResult<()> {
        let sandbox = IronSandbox::new()?;

        // Set up the vo global
        let vo = create_vo_table(sandbox.lua(), contributions)?;
        sandbox
            .lua()
            .globals()
            .set("vo", vo)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;

        // Execute the plugin
        sandbox.execute_script(source)?;

        Ok(())
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manifest(dir: &Path, plugins: &[(&str, &str, bool)]) {
        std::fs::create_dir_all(dir).unwrap();

        let entries: Vec<PluginEntry> = plugins
            .iter()
            .map(|(name, file, enabled)| PluginEntry {
                name: name.to_string(),
                file: file.to_string(),
                enabled: *enabled,
                priority: 0,
                description: String::new(),
                author: String::new(),
                version: String::new(),
            })
            .collect();

        let manifest = PluginManifest {
            vo_api_version: CURRENT_API_VERSION.to_string(),
            plugins: entries,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(dir.join(MANIFEST_FILE), manifest_json).unwrap();
    }

    #[test]
    fn test_loader_creation() {
        let loader = PluginLoader::new();
        assert!(!loader.search_paths().is_empty());
    }

    #[test]
    fn test_discover_empty_path() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = PluginLoader::with_paths(vec![temp_dir.path().to_path_buf()]);

        let plugins = loader.discover();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_with_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        // Create manifest with one plugin
        create_test_manifest(&plugins_dir, &[("test-plugin", "test.lua", true)]);

        // Create the plugin file
        std::fs::write(plugins_dir.join("test.lua"), "-- Test plugin").unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        let plugins = loader.discover();

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].entry.name, "test-plugin");
        assert!(matches!(plugins[0].status, PluginStatus::Loaded));
    }

    #[test]
    fn test_discover_disabled_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        create_test_manifest(&plugins_dir, &[("disabled-plugin", "disabled.lua", false)]);

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        let plugins = loader.discover();

        assert_eq!(plugins.len(), 1);
        assert!(matches!(plugins[0].status, PluginStatus::Disabled));
    }

    #[test]
    fn test_discover_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        // Create manifest but don't create the plugin file
        create_test_manifest(&plugins_dir, &[("missing-plugin", "missing.lua", true)]);

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        let plugins = loader.discover();

        assert_eq!(plugins.len(), 1);
        assert!(matches!(plugins[0].status, PluginStatus::LoadError(_)));
    }

    #[test]
    fn test_api_version_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        // Create manifest with wrong API version
        let manifest = serde_json::json!({
            "vo_api_version": "1.0",
            "plugins": []
        });
        std::fs::write(plugins_dir.join(MANIFEST_FILE), manifest.to_string()).unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        let plugins = loader.discover();

        // Should not load any plugins due to version mismatch
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_priority_sorting() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        let manifest = PluginManifest {
            vo_api_version: CURRENT_API_VERSION.to_string(),
            plugins: vec![
                PluginEntry {
                    name: "low-priority".to_string(),
                    file: "low.lua".to_string(),
                    enabled: true,
                    priority: 10,
                    description: String::new(),
                    author: String::new(),
                    version: String::new(),
                },
                PluginEntry {
                    name: "high-priority".to_string(),
                    file: "high.lua".to_string(),
                    enabled: true,
                    priority: 100,
                    description: String::new(),
                    author: String::new(),
                    version: String::new(),
                },
            ],
        };

        std::fs::write(
            plugins_dir.join(MANIFEST_FILE),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        std::fs::write(plugins_dir.join("low.lua"), "-- low").unwrap();
        std::fs::write(plugins_dir.join("high.lua"), "-- high").unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        let plugins = loader.discover();

        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].entry.name, "high-priority"); // Higher priority first
        assert_eq!(plugins[1].entry.name, "low-priority");
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn test_execute_simple_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        create_test_manifest(&plugins_dir, &[("simple", "simple.lua", true)]);

        std::fs::write(
            plugins_dir.join("simple.lua"),
            r#"
                vo.log("info", "Plugin loaded")
                vo.contribute_tag("test:1", "simple-tag")
            "#,
        )
        .unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        loader.discover();

        let contributions = loader.execute_all().unwrap();
        let contribs = contributions.lock().unwrap();

        assert!(!contribs.logs.is_empty());
        assert!(contribs.tags.contains_key("test:1"));
    }

    // =========================================================================
    // Additional Coverage Tests
    // =========================================================================

    #[test]
    fn test_default_enabled_function() {
        // Test default_enabled is true
        assert!(default_enabled());
    }

    #[test]
    fn test_loader_default_trait() {
        let loader = PluginLoader::default();
        // Should have search paths set up
        assert!(!loader.search_paths().is_empty());
    }

    #[test]
    fn test_add_path() {
        let mut loader = PluginLoader::with_paths(vec![]);
        assert!(loader.search_paths().is_empty());

        let new_path = PathBuf::from("/test/path");
        loader.add_path(new_path.clone());

        assert_eq!(loader.search_paths().len(), 1);
        assert_eq!(loader.search_paths()[0], new_path);
    }

    #[test]
    fn test_add_path_no_duplicate() {
        let mut loader = PluginLoader::with_paths(vec![]);
        let path = PathBuf::from("/test/path");

        loader.add_path(path.clone());
        loader.add_path(path.clone()); // Add same path again

        // Should not duplicate
        assert_eq!(loader.search_paths().len(), 1);
    }

    #[test]
    fn test_enabled_plugins_filters_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        // Create manifest with one enabled and one disabled
        create_test_manifest(
            &plugins_dir,
            &[
                ("enabled-plugin", "enabled.lua", true),
                ("disabled-plugin", "disabled.lua", false),
            ],
        );

        std::fs::write(plugins_dir.join("enabled.lua"), "-- enabled").unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        loader.discover();

        let enabled = loader.enabled_plugins();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].entry.name, "enabled-plugin");
    }

    #[test]
    fn test_plugin_names() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        create_test_manifest(
            &plugins_dir,
            &[
                ("plugin-a", "a.lua", true),
                ("plugin-b", "b.lua", true),
                ("plugin-c", "c.lua", false),
            ],
        );

        std::fs::write(plugins_dir.join("a.lua"), "-- a").unwrap();
        std::fs::write(plugins_dir.join("b.lua"), "-- b").unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        loader.discover();

        let names = loader.plugin_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"plugin-a"));
        assert!(names.contains(&"plugin-b"));
        assert!(!names.contains(&"plugin-c")); // Disabled
    }

    #[test]
    fn test_plugins_accessor() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");

        create_test_manifest(&plugins_dir, &[("test", "test.lua", true)]);
        std::fs::write(plugins_dir.join("test.lua"), "-- test").unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        loader.discover();

        let all_plugins = loader.plugins();
        assert_eq!(all_plugins.len(), 1);
    }

    #[test]
    fn test_plugin_entry_serde_defaults() {
        let json = r#"{"name": "test", "file": "test.lua"}"#;
        let entry: PluginEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.name, "test");
        assert_eq!(entry.file, "test.lua");
        assert!(entry.enabled); // default true
        assert_eq!(entry.priority, 0); // default 0
        assert!(entry.description.is_empty()); // default empty
        assert!(entry.author.is_empty()); // default empty
        assert!(entry.version.is_empty()); // default empty
    }

    #[test]
    fn test_plugin_manifest_serialization() {
        let manifest = PluginManifest {
            vo_api_version: "3.0".to_string(),
            plugins: vec![PluginEntry {
                name: "test".to_string(),
                file: "test.lua".to_string(),
                enabled: true,
                priority: 10,
                description: "A test plugin".to_string(),
                author: "Test Author".to_string(),
                version: "1.0.0".to_string(),
            }],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: PluginManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.vo_api_version, "3.0");
        assert_eq!(parsed.plugins.len(), 1);
        assert_eq!(parsed.plugins[0].name, "test");
    }

    #[test]
    fn test_plugin_status_variants() {
        let loaded = PluginStatus::Loaded;
        let executed = PluginStatus::Executed;
        let load_err = PluginStatus::LoadError("error".to_string());
        let exec_err = PluginStatus::ExecutionError("error".to_string());
        let disabled = PluginStatus::Disabled;

        // Just verify they can be created and matched
        assert!(matches!(loaded, PluginStatus::Loaded));
        assert!(matches!(executed, PluginStatus::Executed));
        assert!(matches!(load_err, PluginStatus::LoadError(_)));
        assert!(matches!(exec_err, PluginStatus::ExecutionError(_)));
        assert!(matches!(disabled, PluginStatus::Disabled));
    }

    #[test]
    fn test_loaded_plugin_fields() {
        let plugin = LoadedPlugin {
            entry: PluginEntry {
                name: "test".to_string(),
                file: "test.lua".to_string(),
                enabled: true,
                priority: 0,
                description: String::new(),
                author: String::new(),
                version: String::new(),
            },
            path: PathBuf::from("/path/to/test.lua"),
            source: "-- lua code".to_string(),
            status: PluginStatus::Loaded,
        };

        assert_eq!(plugin.entry.name, "test");
        assert_eq!(plugin.path, PathBuf::from("/path/to/test.lua"));
        assert_eq!(plugin.source, "-- lua code");
    }

    #[test]
    fn test_discover_invalid_json_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        // Write invalid JSON
        std::fs::write(plugins_dir.join(MANIFEST_FILE), "{ invalid json }").unwrap();

        let mut loader = PluginLoader::with_paths(vec![plugins_dir]);
        let plugins = loader.discover();

        // Should not load any plugins
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_constants() {
        assert_eq!(CURRENT_API_VERSION, "3.0");
        assert_eq!(MANIFEST_FILE, "manifest.json");
    }
}
