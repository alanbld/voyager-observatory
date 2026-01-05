//! Plugin Engine - Core Plugin System
//!
//! Orchestrates plugin discovery, loading, and execution.
//! Provides the main interface for the plugin ecosystem.

use std::path::PathBuf;

use super::error::PluginResult;
use super::loader::{PluginLoader, LoadedPlugin, PluginStatus, CURRENT_API_VERSION};

#[cfg(feature = "plugins")]
use super::bridges::vo_table::SharedContributions;

/// Plugin engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// Engine not initialized
    Uninitialized,
    /// Plugins discovered but not executed
    Discovered,
    /// Plugins executed
    Executed,
    /// Engine disabled (feature not compiled or --no-plugins)
    Disabled,
}

/// The Plugin Engine - manages the plugin lifecycle
pub struct PluginEngine {
    /// Plugin loader
    loader: PluginLoader,
    /// Current engine state
    state: EngineState,
    /// Plugin contributions (after execution)
    #[cfg(feature = "plugins")]
    contributions: Option<SharedContributions>,
}

impl PluginEngine {
    /// Create a new plugin engine
    pub fn new() -> Self {
        Self {
            loader: PluginLoader::new(),
            state: EngineState::Uninitialized,
            #[cfg(feature = "plugins")]
            contributions: None,
        }
    }

    /// Create a disabled engine (for --no-plugins mode)
    pub fn disabled() -> Self {
        Self {
            loader: PluginLoader::with_paths(vec![]),
            state: EngineState::Disabled,
            #[cfg(feature = "plugins")]
            contributions: None,
        }
    }

    /// Add a custom plugin search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.loader.add_path(path);
    }

    /// Get current engine state
    pub fn state(&self) -> EngineState {
        self.state
    }

    /// Check if plugins feature is available
    pub fn is_available() -> bool {
        cfg!(feature = "plugins")
    }

    /// Get API version
    pub fn api_version() -> &'static str {
        CURRENT_API_VERSION
    }

    /// Discover plugins from configured paths
    pub fn discover(&mut self) -> &[LoadedPlugin] {
        if self.state == EngineState::Disabled {
            return &[];
        }

        self.loader.discover();
        self.state = EngineState::Discovered;
        self.loader.plugins()
    }

    /// Execute all discovered plugins
    #[cfg(feature = "plugins")]
    pub fn execute(&mut self) -> PluginResult<()> {
        if self.state == EngineState::Disabled {
            return Ok(());
        }

        if self.state == EngineState::Uninitialized {
            self.discover();
        }

        self.contributions = Some(self.loader.execute_all()?);
        self.state = EngineState::Executed;
        Ok(())
    }

    /// Execute (no-op when plugins feature is disabled)
    #[cfg(not(feature = "plugins"))]
    pub fn execute(&mut self) -> PluginResult<()> {
        self.state = EngineState::Disabled;
        Ok(())
    }

    /// Get discovered plugins
    pub fn plugins(&self) -> &[LoadedPlugin] {
        self.loader.plugins()
    }

    /// Get enabled plugin names
    pub fn plugin_names(&self) -> Vec<&str> {
        self.loader.plugin_names()
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.loader.enabled_plugins().len()
    }

    /// Get contributions from executed plugins
    #[cfg(feature = "plugins")]
    pub fn contributions(&self) -> Option<&SharedContributions> {
        self.contributions.as_ref()
    }

    /// Get metric value by name
    #[cfg(feature = "plugins")]
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.contributions.as_ref().and_then(|c| {
            c.lock().ok().and_then(|contribs| {
                contribs.metrics.get(name).map(|m| m.value)
            })
        })
    }

    /// Get tags for a node
    #[cfg(feature = "plugins")]
    pub fn get_tags(&self, node_id: &str) -> Vec<String> {
        self.contributions
            .as_ref()
            .and_then(|c| {
                c.lock().ok().map(|contribs| {
                    contribs.tags.get(node_id).cloned().unwrap_or_default()
                })
            })
            .unwrap_or_default()
    }

    /// Get all log entries
    #[cfg(feature = "plugins")]
    pub fn get_logs(&self) -> Vec<super::bridges::vo_table::LogEntry> {
        self.contributions
            .as_ref()
            .and_then(|c| {
                c.lock().ok().map(|contribs| contribs.logs.clone())
            })
            .unwrap_or_default()
    }

    /// Generate summary for Mission Log
    pub fn summary(&self) -> String {
        if self.state == EngineState::Disabled {
            return String::from("🔌 External optics: Disabled");
        }

        let plugin_count = self.plugin_count();
        if plugin_count == 0 {
            return String::from("🔌 No external optics detected.");
        }

        let mut output = format!("🔌 External Optics: {} community plugin{} loaded\n",
            plugin_count,
            if plugin_count == 1 { "" } else { "s" }
        );

        for name in self.plugin_names() {
            output.push_str(&format!("   ├─ {}\n", name));
        }

        output.push_str("🛡️ Plugin sandbox: Active (10MB memory, 100ms timeout)\n");
        output
    }

    /// Generate status for --plugins list command
    pub fn list_status(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Plugin API Version: {}\n", CURRENT_API_VERSION));
        output.push_str(&format!("Feature Status: {}\n",
            if Self::is_available() { "Enabled" } else { "Disabled" }
        ));
        output.push_str("\nSearch Paths:\n");

        for path in self.loader.search_paths() {
            let status = if path.exists() { "✓" } else { "✗" };
            output.push_str(&format!("  {} {}\n", status, path.display()));
        }

        output.push_str(&format!("\nDiscovered Plugins: {}\n", self.loader.plugins().len()));

        for plugin in self.loader.plugins() {
            let status_icon = match &plugin.status {
                PluginStatus::Loaded => "✓",
                PluginStatus::Executed => "✓",
                PluginStatus::Disabled => "○",
                PluginStatus::LoadError(_) => "✗",
                PluginStatus::ExecutionError(_) => "✗",
            };

            output.push_str(&format!("  {} {} (priority: {})\n",
                status_icon,
                plugin.entry.name,
                plugin.entry.priority
            ));

            if let PluginStatus::LoadError(e) | PluginStatus::ExecutionError(e) = &plugin.status {
                output.push_str(&format!("      Error: {}\n", e));
            }
        }

        output
    }
}

impl Default for PluginEngine {
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

    // ==================== Engine Creation Tests ====================

    #[test]
    fn test_engine_creation() {
        let engine = PluginEngine::new();
        assert_eq!(engine.state(), EngineState::Uninitialized);
    }

    #[test]
    fn test_engine_default() {
        let engine = PluginEngine::default();
        assert_eq!(engine.state(), EngineState::Uninitialized);
    }

    #[test]
    fn test_engine_disabled() {
        let engine = PluginEngine::disabled();
        assert_eq!(engine.state(), EngineState::Disabled);
    }

    // ==================== State Tests ====================

    #[test]
    fn test_engine_state_variants() {
        // Test all state variants exist
        let _uninitialized = EngineState::Uninitialized;
        let _discovered = EngineState::Discovered;
        let _executed = EngineState::Executed;
        let _disabled = EngineState::Disabled;
    }

    #[test]
    fn test_engine_state_equality() {
        assert_eq!(EngineState::Uninitialized, EngineState::Uninitialized);
        assert_ne!(EngineState::Uninitialized, EngineState::Disabled);
    }

    #[test]
    fn test_engine_state_debug() {
        let state = EngineState::Uninitialized;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Uninitialized"));
    }

    #[test]
    fn test_engine_state_clone() {
        let state = EngineState::Discovered;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_engine_state_copy() {
        let state = EngineState::Executed;
        let copied = state;
        assert_eq!(state, copied);
    }

    // ==================== API Version Tests ====================

    #[test]
    fn test_api_version() {
        assert_eq!(PluginEngine::api_version(), "3.0");
    }

    #[test]
    fn test_api_version_is_static() {
        let version = PluginEngine::api_version();
        assert!(!version.is_empty());
    }

    // ==================== Availability Tests ====================

    #[test]
    fn test_is_available() {
        let _available = PluginEngine::is_available();
        // Just verify it returns a bool without panicking
    }

    // ==================== Discovery Tests ====================

    #[test]
    fn test_discover_empty() {
        let mut engine = PluginEngine::new();
        let plugins = engine.discover();
        // May or may not find plugins depending on system
        let _ = plugins;
        assert_eq!(engine.state(), EngineState::Discovered);
    }

    #[test]
    fn test_discover_disabled_engine() {
        let mut engine = PluginEngine::disabled();
        let plugins = engine.discover();
        assert!(plugins.is_empty());
        assert_eq!(engine.state(), EngineState::Disabled);
    }

    // ==================== Plugin Access Tests ====================

    #[test]
    fn test_plugins_empty() {
        let engine = PluginEngine::new();
        let plugins = engine.plugins();
        // No plugins discovered yet
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_plugin_names_empty() {
        let engine = PluginEngine::new();
        let names = engine.plugin_names();
        assert!(names.is_empty());
    }

    #[test]
    fn test_plugin_count_empty() {
        let engine = PluginEngine::new();
        assert_eq!(engine.plugin_count(), 0);
    }

    #[test]
    fn test_plugin_count_after_discover() {
        let mut engine = PluginEngine::new();
        engine.discover();
        // Count may be 0 or more depending on installed plugins
        let _ = engine.plugin_count();
    }

    // ==================== Search Path Tests ====================

    #[test]
    fn test_add_search_path() {
        let mut engine = PluginEngine::new();
        engine.add_search_path(PathBuf::from("/tmp/test_plugins"));
        // Just verify it doesn't panic
    }

    #[test]
    fn test_add_multiple_search_paths() {
        let mut engine = PluginEngine::new();
        engine.add_search_path(PathBuf::from("/tmp/path1"));
        engine.add_search_path(PathBuf::from("/tmp/path2"));
        engine.add_search_path(PathBuf::from("/tmp/path3"));
        // Verify no panics
    }

    // ==================== Execute Tests ====================

    #[test]
    fn test_execute_disabled() {
        let mut engine = PluginEngine::disabled();
        let result = engine.execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_empty() {
        let mut engine = PluginEngine::new();
        let result = engine.execute();
        assert!(result.is_ok());
    }

    // ==================== Summary Tests ====================

    #[test]
    fn test_summary_no_plugins() {
        let mut engine = PluginEngine::new();
        engine.discover();
        let summary = engine.summary();
        assert!(summary.contains("No external optics") || summary.contains("External Optics"));
    }

    #[test]
    fn test_summary_disabled() {
        let engine = PluginEngine::disabled();
        let summary = engine.summary();
        assert!(summary.contains("Disabled"));
    }

    #[test]
    fn test_summary_contains_icon() {
        let engine = PluginEngine::disabled();
        let summary = engine.summary();
        assert!(summary.contains("🔌"));
    }

    // ==================== List Status Tests ====================

    #[test]
    fn test_list_status() {
        let engine = PluginEngine::new();
        let status = engine.list_status();

        assert!(status.contains("Plugin API Version"));
        assert!(status.contains("Feature Status"));
        assert!(status.contains("Search Paths"));
        assert!(status.contains("Discovered Plugins"));
    }

    #[test]
    fn test_list_status_shows_api_version() {
        let engine = PluginEngine::new();
        let status = engine.list_status();
        assert!(status.contains("3.0"));
    }

    #[test]
    fn test_list_status_shows_feature_status() {
        let engine = PluginEngine::new();
        let status = engine.list_status();
        // Should show either Enabled or Disabled
        assert!(status.contains("Enabled") || status.contains("Disabled"));
    }

    #[test]
    fn test_list_status_after_discover() {
        let mut engine = PluginEngine::new();
        engine.discover();
        let status = engine.list_status();
        assert!(status.contains("Discovered Plugins"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_lifecycle() {
        let mut engine = PluginEngine::new();

        // Initial state
        assert_eq!(engine.state(), EngineState::Uninitialized);
        assert_eq!(engine.plugin_count(), 0);

        // Discover
        engine.discover();
        assert_eq!(engine.state(), EngineState::Discovered);

        // Execute
        let _ = engine.execute();

        // Summary
        let summary = engine.summary();
        assert!(!summary.is_empty());

        // List status
        let status = engine.list_status();
        assert!(!status.is_empty());
    }

    #[test]
    fn test_disabled_lifecycle() {
        let mut engine = PluginEngine::disabled();

        // Initial state
        assert_eq!(engine.state(), EngineState::Disabled);

        // Discover returns empty
        let plugins = engine.discover();
        assert!(plugins.is_empty());
        assert_eq!(engine.state(), EngineState::Disabled);

        // Execute succeeds but does nothing
        let result = engine.execute();
        assert!(result.is_ok());

        // Summary shows disabled
        let summary = engine.summary();
        assert!(summary.contains("Disabled"));
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn test_engine_with_plugins() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        // Create manifest
        let manifest = serde_json::json!({
            "vo_api_version": "3.0",
            "plugins": [{
                "name": "test-plugin",
                "file": "test.lua",
                "enabled": true,
                "priority": 100
            }]
        });
        std::fs::write(plugins_dir.join("manifest.json"), manifest.to_string()).unwrap();
        std::fs::write(plugins_dir.join("test.lua"), "vo.log('info', 'Hello!')").unwrap();

        let mut engine = PluginEngine::new();
        engine.add_search_path(plugins_dir);
        engine.discover();

        assert_eq!(engine.plugin_count(), 1);
        assert_eq!(engine.state(), EngineState::Discovered);
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn test_contributions_none_before_execute() {
        let engine = PluginEngine::new();
        assert!(engine.contributions().is_none());
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn test_get_metric_none() {
        let engine = PluginEngine::new();
        assert!(engine.get_metric("nonexistent").is_none());
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn test_get_tags_empty() {
        let engine = PluginEngine::new();
        let tags = engine.get_tags("some_node");
        assert!(tags.is_empty());
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn test_get_logs_empty() {
        let engine = PluginEngine::new();
        let logs = engine.get_logs();
        assert!(logs.is_empty());
    }
}
