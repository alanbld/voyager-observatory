//! vo.* Bridge API
//!
//! Exposes curated Observatory capabilities to Lua plugins.
//! All functions are sandboxed and follow the sovereignty model.

#[cfg(feature = "plugins")]
use mlua::{Function, Lua, Result as LuaResult, Table};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::patterns::create_patterns_table;
use crate::core::regex_engine;

/// Current API version for plugin compatibility
pub const API_VERSION: &str = "3.0";

/// Plugin contributions storage (tags, metrics, logs)
#[cfg(feature = "plugins")]
#[derive(Debug, Default)]
pub struct PluginContributions {
    /// Tags contributed by plugins (node_id -> Vec<tag>)
    pub tags: BTreeMap<String, Vec<String>>,
    /// Metric values from plugins (metric_name -> value)
    pub metrics: BTreeMap<String, MetricValue>,
    /// Log entries from plugins
    pub logs: Vec<LogEntry>,
}

/// A metric value with metadata
#[cfg(feature = "plugins")]
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub value: f64,
    pub confidence: f64,
    pub explanation: String,
}

/// A log entry from a plugin
#[cfg(feature = "plugins")]
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub plugin: String,
}

/// Shared contributions across plugins
#[cfg(feature = "plugins")]
pub type SharedContributions = Arc<Mutex<PluginContributions>>;

/// Create the main `vo` global table for plugins
#[cfg(feature = "plugins")]
pub fn create_vo_table(lua: &Lua, contributions: SharedContributions) -> LuaResult<Table> {
    let vo = lua.create_table()?;

    // API version
    vo.set("api_version", API_VERSION)?;

    // Pre-compiled patterns
    vo.set("patterns", create_patterns_table(lua)?)?;

    // Regex bridge
    vo.set("regex", create_regex_function(lua)?)?;

    // Logging bridge
    vo.set("log", create_log_function(lua, contributions.clone())?)?;

    // Tag contribution
    vo.set(
        "contribute_tag",
        create_tag_function(lua, contributions.clone())?,
    )?;

    // Metric registration (stores callback for later use)
    vo.set(
        "register_metric",
        create_metric_function(lua, contributions)?,
    )?;

    // AST proxy (read-only)
    vo.set("ast", create_ast_proxy(lua)?)?;

    Ok(vo)
}

/// Create a simpler vo table without shared contributions (for testing)
#[cfg(feature = "plugins")]
pub fn create_vo_table_simple(lua: &Lua) -> LuaResult<Table> {
    let contributions = Arc::new(Mutex::new(PluginContributions::default()));
    create_vo_table(lua, contributions)
}

/// Create the regex function that returns a matcher
#[cfg(feature = "plugins")]
fn create_regex_function(lua: &Lua) -> LuaResult<Function> {
    lua.create_function(|lua, pattern: String| {
        // Validate the pattern first
        regex_engine::compile(&pattern)
            .map_err(|e| mlua::Error::RuntimeError(format!("Regex error: {}", e)))?;

        // Return a function that matches against text
        // Clone pattern for the closure
        let pattern_clone = pattern.clone();
        lua.create_function(move |_, text: String| {
            let matches = regex_engine::find_all(&pattern_clone, &text)
                .map_err(|e| mlua::Error::RuntimeError(format!("Regex error: {}", e)))?;
            Ok(matches.len())
        })
    })
}

/// Create the log function
#[cfg(feature = "plugins")]
fn create_log_function(lua: &Lua, contributions: SharedContributions) -> LuaResult<Function> {
    lua.create_function(move |_, (level, message): (String, String)| {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        let level = if valid_levels.contains(&level.to_lowercase().as_str()) {
            level.to_lowercase()
        } else {
            "info".to_string()
        };

        // Store log entry
        if let Ok(mut contribs) = contributions.lock() {
            contribs.logs.push(LogEntry {
                level: level.clone(),
                message: message.clone(),
                plugin: "unknown".to_string(), // Will be set by loader
            });
        }

        Ok(())
    })
}

/// Create the tag contribution function
#[cfg(feature = "plugins")]
fn create_tag_function(lua: &Lua, contributions: SharedContributions) -> LuaResult<Function> {
    lua.create_function(move |_, (node_id, tag): (String, String)| {
        // Validate inputs
        if node_id.is_empty() || tag.is_empty() {
            return Err(mlua::Error::RuntimeError(
                "node_id and tag cannot be empty".to_string(),
            ));
        }

        // Store tag (append-only)
        if let Ok(mut contribs) = contributions.lock() {
            contribs
                .tags
                .entry(node_id)
                .or_insert_with(Vec::new)
                .push(tag);
        }

        Ok(())
    })
}

/// Create the metric registration function
#[cfg(feature = "plugins")]
fn create_metric_function(lua: &Lua, contributions: SharedContributions) -> LuaResult<Function> {
    lua.create_function(move |lua, (name, callback): (String, Function)| {
        // Validate metric name
        if name.is_empty() {
            return Err(mlua::Error::RuntimeError(
                "metric name cannot be empty".to_string(),
            ));
        }

        // For now, just execute the callback with an empty AST to get initial value
        // In full implementation, this would be called during census collection
        let ast = lua.create_table()?;
        ast.set("functions", lua.create_table()?)?;
        ast.set("comments", lua.create_table()?)?;

        let result: Table = callback.call(ast)?;

        // Extract metric value
        let value: f64 = result.get("value").unwrap_or(0.0);
        let confidence: f64 = result.get("confidence").unwrap_or(0.5);
        let explanation: String = result.get("explanation").unwrap_or_else(|_| String::new());

        // Store metric
        if let Ok(mut contribs) = contributions.lock() {
            contribs.metrics.insert(
                name,
                MetricValue {
                    value,
                    confidence,
                    explanation,
                },
            );
        }

        Ok(())
    })
}

/// Create read-only AST proxy
#[cfg(feature = "plugins")]
fn create_ast_proxy(lua: &Lua) -> LuaResult<Function> {
    lua.create_function(|lua, path: String| {
        // Return a read-only table representing AST data
        // In full implementation, this would query the actual AST bridge
        let ast = lua.create_table()?;
        ast.set("path", path)?;
        ast.set("functions", lua.create_table()?)?;
        ast.set("comments", lua.create_table()?)?;
        ast.set("imports", lua.create_table()?)?;
        Ok(ast)
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(all(test, feature = "plugins"))]
mod tests {
    use super::*;
    use mlua::Lua;

    fn create_test_env() -> (Lua, SharedContributions) {
        let lua = Lua::new();
        let contributions = Arc::new(Mutex::new(PluginContributions::default()));
        (lua, contributions)
    }

    #[test]
    fn test_vo_table_creation() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions).unwrap();

        // Verify API version
        let version: String = vo.get("api_version").unwrap();
        assert_eq!(version, API_VERSION);
    }

    #[test]
    fn test_vo_patterns_available() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions).unwrap();

        let patterns: Table = vo.get("patterns").unwrap();
        let rust_fn: String = patterns.get("rust_fn").unwrap();
        assert!(!rust_fn.is_empty());
    }

    #[test]
    fn test_vo_regex_bridge() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions).unwrap();
        lua.globals().set("vo", vo).unwrap();

        let result: i32 = lua
            .load(
                r#"
            local pattern = vo.regex("test")
            return pattern("this is a test string with test")
        "#,
            )
            .eval()
            .unwrap();

        assert_eq!(result, 2); // "test" appears twice
    }

    #[test]
    fn test_vo_log_function() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions.clone()).unwrap();
        lua.globals().set("vo", vo).unwrap();

        lua.load(
            r#"
            vo.log("info", "Plugin initialized")
            vo.log("warn", "Something happened")
        "#,
        )
        .exec()
        .unwrap();

        let contribs = contributions.lock().unwrap();
        assert_eq!(contribs.logs.len(), 2);
        assert_eq!(contribs.logs[0].level, "info");
        assert_eq!(contribs.logs[0].message, "Plugin initialized");
    }

    #[test]
    fn test_vo_contribute_tag() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions.clone()).unwrap();
        lua.globals().set("vo", vo).unwrap();

        lua.load(
            r#"
            vo.contribute_tag("src/main.rs:42", "needs-review")
            vo.contribute_tag("src/main.rs:42", "complex")
            vo.contribute_tag("src/lib.rs:10", "todo")
        "#,
        )
        .exec()
        .unwrap();

        let contribs = contributions.lock().unwrap();
        assert_eq!(contribs.tags.len(), 2); // Two unique node IDs
        assert_eq!(contribs.tags.get("src/main.rs:42").unwrap().len(), 2);
    }

    #[test]
    fn test_vo_register_metric() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions.clone()).unwrap();
        lua.globals().set("vo", vo).unwrap();

        lua.load(
            r#"
            vo.register_metric("test_metric", function(ast)
                return {
                    value = 42,
                    confidence = 0.9,
                    explanation = "Test metric"
                }
            end)
        "#,
        )
        .exec()
        .unwrap();

        let contribs = contributions.lock().unwrap();
        let metric = contribs.metrics.get("test_metric").unwrap();
        assert_eq!(metric.value, 42.0);
        assert_eq!(metric.confidence, 0.9);
    }

    #[test]
    fn test_vo_ast_proxy() {
        let (lua, contributions) = create_test_env();
        let vo = create_vo_table(&lua, contributions).unwrap();
        lua.globals().set("vo", vo).unwrap();

        let path: String = lua
            .load(
                r#"
            local ast = vo.ast("src/main.rs")
            return ast.path
        "#,
            )
            .eval()
            .unwrap();

        assert_eq!(path, "src/main.rs");
    }
}
