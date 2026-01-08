//! Iron Sandbox - Secure Lua Runtime
//!
//! Implements the Iron Sandbox with strict resource limits:
//! - 100ms CPU timeout via instruction counting hook
//! - 10MB memory limit via Lua allocator
//! - Stripped dangerous libraries (io, os, debug, package)

#[cfg(feature = "plugins")]
use mlua::{HookTriggers, Lua, Result as LuaResult, Value};
#[cfg(feature = "plugins")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(feature = "plugins")]
use std::sync::Arc;
#[cfg(feature = "plugins")]
use std::time::Duration;

use super::error::{PluginError, PluginResult};

/// Memory limit for plugin execution (10MB)
pub const MEMORY_LIMIT: usize = 10 * 1024 * 1024;

/// CPU timeout for plugin execution (100ms)
pub const TIMEOUT_MS: u64 = 100;

/// Instruction limit (approximation for timeout)
/// ~100k instructions ≈ 100ms on modern CPUs
pub const INSTRUCTION_LIMIT: u64 = 100_000;

/// The Iron Sandbox - secure Lua execution environment
#[cfg(feature = "plugins")]
pub struct IronSandbox {
    /// The Lua runtime
    lua: Lua,
    /// Execution timeout
    timeout: Duration,
    /// Memory limit in bytes
    memory_limit: usize,
}

#[cfg(feature = "plugins")]
impl IronSandbox {
    /// Create a new Iron Sandbox with default limits
    pub fn new() -> PluginResult<Self> {
        Self::with_limits(Duration::from_millis(TIMEOUT_MS), MEMORY_LIMIT)
    }

    /// Create a sandbox with custom limits
    pub fn with_limits(timeout: Duration, memory_limit: usize) -> PluginResult<Self> {
        let lua = Lua::new();

        // SET MEMORY LIMIT
        lua.set_memory_limit(memory_limit)
            .map_err(|e| PluginError::LuaError(format!("Failed to set memory limit: {}", e)))?;

        // STRIP DANGEROUS LIBRARIES
        Self::strip_dangerous_globals(&lua)?;

        Ok(Self {
            lua,
            timeout,
            memory_limit,
        })
    }

    /// Remove dangerous global functions and libraries
    fn strip_dangerous_globals(lua: &Lua) -> PluginResult<()> {
        let globals = lua.globals();

        // Remove I/O libraries
        globals
            .set("io", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;
        globals
            .set("os", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;

        // Remove debug library (can be used to escape sandbox)
        globals
            .set("debug", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;

        // Remove package/require (prevents loading external modules)
        globals
            .set("package", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;
        globals
            .set("require", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;

        // Remove dynamic code loading functions
        globals
            .set("load", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;
        globals
            .set("loadfile", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;
        globals
            .set("dofile", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;
        globals
            .set("loadstring", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;

        // Remove collectgarbage (can be used to probe memory)
        globals
            .set("collectgarbage", Value::Nil)
            .map_err(|e| PluginError::LuaError(e.to_string()))?;

        Ok(())
    }

    /// Execute Lua code with timeout enforcement via instruction counting
    pub fn execute<F, R>(&self, f: F) -> PluginResult<R>
    where
        F: FnOnce(&Lua) -> LuaResult<R>,
    {
        let instruction_count = Arc::new(AtomicU64::new(0));
        let timed_out = Arc::new(AtomicBool::new(false));
        let instruction_count_clone = Arc::clone(&instruction_count);
        let timed_out_clone = Arc::clone(&timed_out);

        // Set hook to count instructions (fires every 1000 instructions)
        self.lua.set_hook(
            HookTriggers::new().every_nth_instruction(1000),
            move |_lua, _debug| {
                let count = instruction_count_clone.fetch_add(1000, Ordering::Relaxed);
                if count >= INSTRUCTION_LIMIT {
                    timed_out_clone.store(true, Ordering::SeqCst);
                    Err(mlua::Error::RuntimeError(
                        "Instruction limit exceeded".to_string(),
                    ))
                } else {
                    Ok(mlua::VmState::Continue)
                }
            },
        );

        let result = f(&self.lua);

        // Remove hook
        self.lua.remove_hook();

        // Check if we hit the instruction limit
        if timed_out.load(Ordering::SeqCst) {
            return Err(PluginError::TimeoutExceeded);
        }

        result.map_err(PluginError::from)
    }

    /// Execute a Lua script string
    pub fn execute_script(&self, script: &str) -> PluginResult<()> {
        self.execute(|lua| lua.load(script).exec())
    }

    /// Execute a Lua script and return a value
    pub fn execute_script_with_result<T>(&self, script: &str) -> PluginResult<T>
    where
        T: mlua::FromLua,
    {
        self.execute(|lua| lua.load(script).eval::<T>())
    }

    /// Get a reference to the Lua runtime (for setting up globals)
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Get current memory usage
    pub fn memory_used(&self) -> usize {
        self.lua.used_memory()
    }

    /// Get configured timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get configured memory limit
    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }
}

#[cfg(feature = "plugins")]
impl Default for IronSandbox {
    fn default() -> Self {
        Self::new().expect("Failed to create default sandbox")
    }
}

// =============================================================================
// Non-Plugin Fallback
// =============================================================================

/// Stub sandbox when plugins feature is disabled
#[cfg(not(feature = "plugins"))]
pub struct IronSandbox;

#[cfg(not(feature = "plugins"))]
impl IronSandbox {
    pub fn new() -> PluginResult<Self> {
        Err(PluginError::LuaError(
            "Plugins feature not enabled".to_string(),
        ))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(all(test, feature = "plugins"))]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let sandbox = IronSandbox::new();
        assert!(sandbox.is_ok(), "Sandbox should be created successfully");
    }

    #[test]
    fn test_sandbox_basic_execution() {
        let sandbox = IronSandbox::new().unwrap();
        let result: i32 = sandbox.execute_script_with_result("return 1 + 1").unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_sandbox_blocks_io() {
        let sandbox = IronSandbox::new().unwrap();
        let result = sandbox.execute_script("io.open('test.txt', 'r')");

        assert!(result.is_err());
        match result.unwrap_err() {
            PluginError::SandboxViolation(_) | PluginError::LuaError(_) => (),
            e => panic!("Expected sandbox violation, got: {:?}", e),
        }
    }

    #[test]
    fn test_sandbox_blocks_os() {
        let sandbox = IronSandbox::new().unwrap();
        let result = sandbox.execute_script("os.execute('echo hello')");

        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_blocks_debug() {
        let sandbox = IronSandbox::new().unwrap();
        let result = sandbox.execute_script("debug.getinfo(1)");

        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_blocks_require() {
        let sandbox = IronSandbox::new().unwrap();
        let result = sandbox.execute_script("require('os')");

        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_blocks_load() {
        let sandbox = IronSandbox::new().unwrap();
        let result = sandbox.execute_script("load('return 1')()");

        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_timeout() {
        let sandbox = IronSandbox::with_limits(Duration::from_millis(50), MEMORY_LIMIT).unwrap();

        let result = sandbox.execute_script("while true do end");

        assert!(result.is_err());

        match result.unwrap_err() {
            PluginError::TimeoutExceeded => (),
            PluginError::LuaError(msg) if msg.contains("limit") || msg.contains("Timeout") => (),
            e => panic!("Expected timeout/limit error, got: {:?}", e),
        }
    }

    #[test]
    fn test_sandbox_memory_tracking() {
        let sandbox = IronSandbox::new().unwrap();

        // Execute some code that allocates memory
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

        assert!(sandbox.memory_used() > 0, "Should track memory usage");
    }

    #[test]
    fn test_sandbox_allows_safe_operations() {
        let sandbox = IronSandbox::new().unwrap();

        // String operations
        let result: String = sandbox
            .execute_script_with_result(r#"return string.upper("hello")"#)
            .unwrap();
        assert_eq!(result, "HELLO");

        // Math operations
        let result: f64 = sandbox
            .execute_script_with_result("return math.sqrt(16)")
            .unwrap();
        assert_eq!(result, 4.0);

        // Table operations
        let result: i32 = sandbox
            .execute_script_with_result(
                r#"
            local t = {1, 2, 3}
            return #t
        "#,
            )
            .unwrap();
        assert_eq!(result, 3);
    }
}
