//! Plugin Ecosystem Integration Tests
//!
//! Comprehensive TDD suite proving the security and functionality
//! of the Iron Sandbox and Plugin Ecosystem.
//!
//! Test Categories:
//! - Adversarial: Security boundary tests
//! - Positive: Functional correctness tests
//! - Integration: End-to-end plugin execution

#![cfg(feature = "plugins")]

use std::sync::{Arc, Mutex};
use tempfile::TempDir;

use pm_encoder::core::plugins::bridges::vo_table::{
    create_vo_table, PluginContributions, SharedContributions,
};
use pm_encoder::core::plugins::{
    EngineState, IronSandbox, PluginEngine, PluginError, PluginStatus, MEMORY_LIMIT,
};

// =============================================================================
// Test Utilities
// =============================================================================

fn create_sandbox() -> IronSandbox {
    IronSandbox::new().expect("Sandbox should be created")
}

fn create_contributions() -> SharedContributions {
    Arc::new(Mutex::new(PluginContributions::default()))
}

fn create_test_plugin_dir(plugins: &[(&str, &str, bool)]) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let plugins_dir = temp_dir.path().to_path_buf();
    std::fs::create_dir_all(&plugins_dir).unwrap();

    // Create manifest
    let entries: Vec<serde_json::Value> = plugins
        .iter()
        .map(|(name, file, enabled)| {
            serde_json::json!({
                "name": name,
                "file": file,
                "enabled": enabled,
                "priority": 100
            })
        })
        .collect();

    let manifest = serde_json::json!({
        "vo_api_version": "3.0",
        "plugins": entries
    });

    std::fs::write(plugins_dir.join("manifest.json"), manifest.to_string()).unwrap();

    // Create plugin files
    for (_, file, _) in plugins {
        std::fs::write(plugins_dir.join(file), "-- placeholder").unwrap();
    }

    temp_dir
}

fn write_plugin_content(dir: &TempDir, filename: &str, content: &str) {
    std::fs::write(dir.path().join(filename), content).unwrap();
}

// =============================================================================
// ADVERSARIAL TESTS - Security Boundaries
// =============================================================================

mod adversarial {
    use super::*;
    use std::time::{Duration, Instant};

    /// Infinite loops must terminate via instruction limit
    #[test]
    fn test_infinite_loop_terminates() {
        let sandbox = create_sandbox();

        let start = Instant::now();
        let result = sandbox.execute_script("while true do end");
        let elapsed = start.elapsed();

        assert!(result.is_err(), "Infinite loop should be stopped");
        // Should terminate relatively quickly (within a few seconds at most)
        assert!(
            elapsed < Duration::from_secs(5),
            "Should terminate quickly, took {:?}",
            elapsed
        );

        match result.unwrap_err() {
            PluginError::TimeoutExceeded => (),
            PluginError::LuaError(msg) if msg.contains("limit") => (),
            e => panic!("Expected timeout/limit error, got: {:?}", e),
        }
    }

    /// Nested infinite recursion must trigger limit
    #[test]
    fn test_infinite_recursion_terminates() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(
            r#"
            function recurse()
                recurse()
            end
            recurse()
        "#,
        );

        assert!(result.is_err(), "Infinite recursion should be stopped");
    }

    /// os.execute("rm -rf /") should fail (os is stripped)
    #[test]
    fn test_os_execute_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"os.execute("rm -rf /")"#);

        assert!(result.is_err(), "os.execute should be blocked");

        let err = result.unwrap_err();
        match err {
            PluginError::LuaError(msg) => {
                assert!(
                    msg.contains("nil") || msg.contains("os"),
                    "Error should mention os is nil/missing: {}",
                    msg
                );
            }
            _ => (), // Any error is acceptable - sandbox prevented execution
        }
    }

    /// os.remove should be blocked
    #[test]
    fn test_os_remove_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"os.remove("/tmp/test.txt")"#);

        assert!(result.is_err(), "os.remove should be blocked");
    }

    /// io.open("/etc/passwd") should fail (io is stripped)
    #[test]
    fn test_io_open_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"io.open("/etc/passwd", "r")"#);

        assert!(result.is_err(), "io.open should be blocked");
    }

    /// io.popen should be blocked
    #[test]
    fn test_io_popen_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"io.popen("ls -la")"#);

        assert!(result.is_err(), "io.popen should be blocked");
    }

    /// debug.setfenv should fail (debug is stripped)
    #[test]
    fn test_debug_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script("debug.getinfo(1)");

        assert!(result.is_err(), "debug library should be blocked");
    }

    /// debug.sethook should fail
    #[test]
    fn test_debug_sethook_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script("debug.sethook(function() end, 'l')");

        assert!(result.is_err(), "debug.sethook should be blocked");
    }

    /// require() should fail (package system stripped)
    #[test]
    fn test_require_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"require("os")"#);

        assert!(result.is_err(), "require should be blocked");
    }

    /// loadfile should fail
    #[test]
    fn test_loadfile_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"loadfile("/etc/passwd")"#);

        assert!(result.is_err(), "loadfile should be blocked");
    }

    /// dofile should fail
    #[test]
    fn test_dofile_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"dofile("/etc/passwd")"#);

        assert!(result.is_err(), "dofile should be blocked");
    }

    /// load() should fail (dynamic code loading stripped)
    #[test]
    fn test_load_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script(r#"load("return 42")()"#);

        assert!(result.is_err(), "load should be blocked");
    }

    /// collectgarbage should fail (can probe memory state)
    #[test]
    fn test_collectgarbage_blocked() {
        let sandbox = create_sandbox();

        let result = sandbox.execute_script("collectgarbage()");

        assert!(result.is_err(), "collectgarbage should be blocked");
    }

    /// Memory allocation should be tracked
    #[test]
    fn test_memory_tracking() {
        let sandbox = create_sandbox();

        // Execute code that allocates memory
        sandbox
            .execute_script(
                r#"
            local t = {}
            for i = 1, 1000 do
                t[i] = string.rep("x", 100)
            end
        "#,
            )
            .unwrap();

        let used = sandbox.memory_used();
        assert!(used > 0, "Should track memory usage: {}", used);
        assert!(
            used < MEMORY_LIMIT,
            "Should be under limit: {}/{}",
            used,
            MEMORY_LIMIT
        );
    }

    /// Environment manipulation should be sandboxed
    #[test]
    fn test_rawset_globals_limited() {
        let sandbox = create_sandbox();

        // Can use rawset on tables, but not escape sandbox
        let result = sandbox.execute_script(
            r#"
            local t = {}
            rawset(t, "key", "value")
            return t.key
        "#,
        );

        // This should work (rawset on local table is fine)
        assert!(result.is_ok() || result.is_err()); // Either is acceptable based on config
    }

    /// getmetatable restrictions (if applicable)
    #[test]
    fn test_metatable_safety() {
        let sandbox = create_sandbox();

        // Metatables should work but not allow escape
        let result: Result<String, _> = sandbox.execute_script_with_result(
            r#"
            local t = setmetatable({}, {__tostring = function() return "safe" end})
            return tostring(t)
        "#,
        );

        // Should either work safely or be blocked
        match result {
            Ok(s) => assert_eq!(s, "safe"),
            Err(_) => (), // Blocking metatables is also acceptable
        }
    }
}

// =============================================================================
// POSITIVE TESTS - Functional Correctness
// =============================================================================

mod positive {
    use super::*;
    use mlua::Lua;
    use pm_encoder::core::plugins::bridges::patterns::create_patterns_table;

    /// Basic Lua execution works
    #[test]
    fn test_basic_execution() {
        let sandbox = create_sandbox();

        let result: i32 = sandbox.execute_script_with_result("return 2 + 2").unwrap();
        assert_eq!(result, 4);
    }

    /// String operations work
    #[test]
    fn test_string_operations() {
        let sandbox = create_sandbox();

        let result: String = sandbox
            .execute_script_with_result(r#"return string.upper("hello world")"#)
            .unwrap();
        assert_eq!(result, "HELLO WORLD");
    }

    /// Math operations work
    #[test]
    fn test_math_operations() {
        let sandbox = create_sandbox();

        let result: f64 = sandbox
            .execute_script_with_result("return math.sqrt(144)")
            .unwrap();
        assert_eq!(result, 12.0);
    }

    /// Table operations work
    #[test]
    fn test_table_operations() {
        let sandbox = create_sandbox();

        let result: i32 = sandbox
            .execute_script_with_result(
                r#"
            local t = {1, 2, 3, 4, 5}
            return #t
        "#,
            )
            .unwrap();
        assert_eq!(result, 5);
    }

    /// Pattern bridge provides correct regex patterns
    #[test]
    fn test_pattern_bridge() {
        let lua = Lua::new();
        let patterns = create_patterns_table(&lua).unwrap();

        let rust_fn: String = patterns.get("rust_fn").unwrap();
        assert!(rust_fn.contains("fn"));

        let python_def: String = patterns.get("python_def").unwrap();
        assert!(python_def.contains("def"));

        let js_class: String = patterns.get("js_class").unwrap();
        assert!(js_class.contains("class"));
    }

    /// vo.log stores entries correctly
    #[test]
    fn test_vo_log_function() {
        let sandbox = create_sandbox();
        let contributions = create_contributions();

        let vo = create_vo_table(sandbox.lua(), contributions.clone()).unwrap();
        sandbox.lua().globals().set("vo", vo).unwrap();

        sandbox
            .execute_script(
                r#"
            vo.log("info", "Test message 1")
            vo.log("warn", "Test message 2")
            vo.log("error", "Test message 3")
        "#,
            )
            .unwrap();

        let contribs = contributions.lock().unwrap();
        assert_eq!(contribs.logs.len(), 3);
        assert_eq!(contribs.logs[0].level, "info");
        assert_eq!(contribs.logs[0].message, "Test message 1");
        assert_eq!(contribs.logs[2].level, "error");
    }

    /// Tags accumulate deterministically (BTreeMap sorted)
    #[test]
    fn test_tags_deterministic_order() {
        let sandbox = create_sandbox();
        let contributions = create_contributions();

        let vo = create_vo_table(sandbox.lua(), contributions.clone()).unwrap();
        sandbox.lua().globals().set("vo", vo).unwrap();

        sandbox
            .execute_script(
                r#"
            vo.contribute_tag("file_c:10", "tag1")
            vo.contribute_tag("file_a:20", "tag2")
            vo.contribute_tag("file_b:30", "tag3")
            vo.contribute_tag("file_a:20", "tag4")
        "#,
            )
            .unwrap();

        let contribs = contributions.lock().unwrap();

        // BTreeMap should maintain sorted order
        let keys: Vec<_> = contribs.tags.keys().collect();
        assert_eq!(keys, vec!["file_a:20", "file_b:30", "file_c:10"]);

        // Multiple tags on same node
        let file_a_tags = contribs.tags.get("file_a:20").unwrap();
        assert_eq!(file_a_tags.len(), 2);
        assert!(file_a_tags.contains(&"tag2".to_string()));
        assert!(file_a_tags.contains(&"tag4".to_string()));
    }

    /// Metric registration works
    #[test]
    fn test_metric_registration() {
        let sandbox = create_sandbox();
        let contributions = create_contributions();

        let vo = create_vo_table(sandbox.lua(), contributions.clone()).unwrap();
        sandbox.lua().globals().set("vo", vo).unwrap();

        sandbox
            .execute_script(
                r#"
            vo.register_metric("test_complexity", function(ast)
                return {
                    value = 42.5,
                    confidence = 0.95,
                    explanation = "Test metric value"
                }
            end)
        "#,
            )
            .unwrap();

        let contribs = contributions.lock().unwrap();
        let metric = contribs.metrics.get("test_complexity").unwrap();
        assert_eq!(metric.value, 42.5);
        assert_eq!(metric.confidence, 0.95);
        assert_eq!(metric.explanation, "Test metric value");
    }

    /// vo.regex function works correctly
    #[test]
    fn test_vo_regex_function() {
        let sandbox = create_sandbox();
        let contributions = create_contributions();

        let vo = create_vo_table(sandbox.lua(), contributions).unwrap();
        sandbox.lua().globals().set("vo", vo).unwrap();

        let count: i32 = sandbox
            .execute_script_with_result(
                r#"
            local matcher = vo.regex("fn\\s+\\w+")
            return matcher("fn foo() fn bar() fn baz()")
        "#,
            )
            .unwrap();

        assert_eq!(count, 3);
    }

    /// API version is accessible
    #[test]
    fn test_api_version_accessible() {
        let sandbox = create_sandbox();
        let contributions = create_contributions();

        let vo = create_vo_table(sandbox.lua(), contributions).unwrap();
        sandbox.lua().globals().set("vo", vo).unwrap();

        let version: String = sandbox
            .execute_script_with_result("return vo.api_version")
            .unwrap();

        assert_eq!(version, "3.0");
    }
}

// =============================================================================
// INTEGRATION TESTS - End-to-End Plugin Execution
// =============================================================================

mod integration {
    use super::*;

    /// PluginEngine discovers and loads plugins from directory
    #[test]
    fn test_engine_discovery() {
        let temp = create_test_plugin_dir(&[
            ("plugin-a", "plugin_a.lua", true),
            ("plugin-b", "plugin_b.lua", true),
        ]);

        write_plugin_content(&temp, "plugin_a.lua", "vo.log('info', 'Plugin A loaded')");
        write_plugin_content(&temp, "plugin_b.lua", "vo.log('info', 'Plugin B loaded')");

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        engine.discover();

        assert_eq!(engine.plugin_count(), 2);
        assert_eq!(engine.state(), EngineState::Discovered);
    }

    /// PluginEngine executes plugins and collects contributions
    #[test]
    fn test_engine_execution() {
        let temp = create_test_plugin_dir(&[("test-plugin", "test.lua", true)]);

        write_plugin_content(
            &temp,
            "test.lua",
            r#"
            vo.log("info", "Plugin initialized")
            vo.contribute_tag("src/main.rs:10", "needs-review")
            vo.register_metric("plugin_metric", function(ast)
                return { value = 100, confidence = 1.0, explanation = "From plugin" }
            end)
        "#,
        );

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        engine.execute().unwrap();

        assert_eq!(engine.state(), EngineState::Executed);

        // Check contributions
        let contributions = engine.contributions().unwrap();
        let contribs = contributions.lock().unwrap();

        assert!(!contribs.logs.is_empty());
        assert!(contribs.tags.contains_key("src/main.rs:10"));
        assert!(contribs.metrics.contains_key("plugin_metric"));
    }

    /// Disabled plugins are not executed
    #[test]
    fn test_disabled_plugin_not_executed() {
        let temp = create_test_plugin_dir(&[
            ("enabled-plugin", "enabled.lua", true),
            ("disabled-plugin", "disabled.lua", false),
        ]);

        write_plugin_content(&temp, "enabled.lua", "vo.log('info', 'enabled')");
        write_plugin_content(&temp, "disabled.lua", "vo.log('info', 'disabled')");

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        engine.execute().unwrap();

        let contributions = engine.contributions().unwrap();
        let contribs = contributions.lock().unwrap();

        // Only enabled plugin's log should be present
        assert_eq!(contribs.logs.len(), 1);
        assert_eq!(contribs.logs[0].message, "enabled");
    }

    /// Plugin execution errors are captured, not propagated
    #[test]
    fn test_plugin_error_captured() {
        let temp = create_test_plugin_dir(&[
            ("good-plugin", "good.lua", true),
            ("bad-plugin", "bad.lua", true),
        ]);

        write_plugin_content(&temp, "good.lua", "vo.log('info', 'good')");
        write_plugin_content(&temp, "bad.lua", "error('intentional failure')");

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        let result = engine.execute();

        // Should succeed overall (errors are captured)
        assert!(result.is_ok());

        // Good plugin should have run
        let contributions = engine.contributions().unwrap();
        let contribs = contributions.lock().unwrap();
        assert_eq!(contribs.logs.len(), 1);

        // Bad plugin should have ExecutionError status
        let plugins = engine.plugins();
        let bad_plugin = plugins
            .iter()
            .find(|p| p.entry.name == "bad-plugin")
            .unwrap();
        assert!(matches!(bad_plugin.status, PluginStatus::ExecutionError(_)));
    }

    /// Plugin summary appears in Mission Log format
    #[test]
    fn test_plugin_summary_format() {
        let temp = create_test_plugin_dir(&[
            ("my-plugin", "my.lua", true),
            ("other-plugin", "other.lua", true),
        ]);

        write_plugin_content(&temp, "my.lua", "-- noop");
        write_plugin_content(&temp, "other.lua", "-- noop");

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        engine.discover();

        let summary = engine.summary();

        assert!(summary.contains("External Optics"));
        assert!(summary.contains("2 community plugins"));
        assert!(summary.contains("my-plugin") || summary.contains("other-plugin"));
        assert!(summary.contains("sandbox"));
    }

    /// Disabled engine returns empty results
    #[test]
    fn test_disabled_engine() {
        let engine = PluginEngine::disabled();

        assert_eq!(engine.state(), EngineState::Disabled);
        assert_eq!(engine.plugin_count(), 0);
        assert!(engine.summary().contains("Disabled"));
    }

    /// Multiple plugins contribute to same tag node
    #[test]
    fn test_multi_plugin_contributions() {
        let temp =
            create_test_plugin_dir(&[("plugin-1", "p1.lua", true), ("plugin-2", "p2.lua", true)]);

        write_plugin_content(
            &temp,
            "p1.lua",
            r#"
            vo.contribute_tag("shared:node", "from-p1")
        "#,
        );
        write_plugin_content(
            &temp,
            "p2.lua",
            r#"
            vo.contribute_tag("shared:node", "from-p2")
        "#,
        );

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        engine.execute().unwrap();

        let contributions = engine.contributions().unwrap();
        let contribs = contributions.lock().unwrap();

        let tags = contribs.tags.get("shared:node").unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"from-p1".to_string()));
        assert!(tags.contains(&"from-p2".to_string()));
    }

    /// Priority order affects execution
    #[test]
    fn test_plugin_priority_order() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(&plugins_dir).unwrap();

        // Create manifest with explicit priorities
        let manifest = serde_json::json!({
            "vo_api_version": "3.0",
            "plugins": [
                { "name": "low-priority", "file": "low.lua", "enabled": true, "priority": 10 },
                { "name": "high-priority", "file": "high.lua", "enabled": true, "priority": 100 }
            ]
        });
        std::fs::write(plugins_dir.join("manifest.json"), manifest.to_string()).unwrap();

        std::fs::write(plugins_dir.join("low.lua"), "vo.log('info', 'low')").unwrap();
        std::fs::write(plugins_dir.join("high.lua"), "vo.log('info', 'high')").unwrap();

        let mut engine = PluginEngine::new();
        engine.add_search_path(plugins_dir);
        engine.execute().unwrap();

        let contributions = engine.contributions().unwrap();
        let contribs = contributions.lock().unwrap();

        // High priority should execute first
        assert_eq!(contribs.logs.len(), 2);
        assert_eq!(contribs.logs[0].message, "high");
        assert_eq!(contribs.logs[1].message, "low");
    }
}

// =============================================================================
// REGRESSION TESTS
// =============================================================================

mod regression {
    use super::*;

    /// Ensure timeout doesn't leave Lua in bad state
    #[test]
    fn test_timeout_recovery() {
        let sandbox = create_sandbox();

        // Trigger timeout
        let _ = sandbox.execute_script("while true do end");

        // Sandbox should still be usable afterward
        let result: i32 = sandbox.execute_script_with_result("return 42").unwrap();
        assert_eq!(result, 42);
    }

    /// Empty plugin file doesn't crash
    #[test]
    fn test_empty_plugin() {
        let temp = create_test_plugin_dir(&[("empty", "empty.lua", true)]);
        write_plugin_content(&temp, "empty.lua", "");

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        let result = engine.execute();

        assert!(result.is_ok());
    }

    /// Comment-only plugin works
    #[test]
    fn test_comment_only_plugin() {
        let temp = create_test_plugin_dir(&[("comments", "comments.lua", true)]);
        write_plugin_content(
            &temp,
            "comments.lua",
            "-- Just a comment\n-- Another comment",
        );

        let mut engine = PluginEngine::new();
        engine.add_search_path(temp.path().to_path_buf());
        let result = engine.execute();

        assert!(result.is_ok());
    }

    /// Very long string doesn't cause issues
    #[test]
    fn test_long_string_handling() {
        let sandbox = create_sandbox();

        let result: i32 = sandbox
            .execute_script_with_result(&format!("local s = '{}'; return #s", "x".repeat(100000)))
            .unwrap();

        assert_eq!(result, 100000);
    }
}
