//! Voyager Observatory - Plugin Ecosystem (Phase 2 Reserved)
//!
//! This module defines the architecture for the Phase 2 Lua plugin ecosystem.
//! It serves as a reservation and design document, establishing the contracts
//! that will enable external extensions to the Voyager Observatory.
//!
//! # Vision: The Plugin Nebula
//!
//! Voyager Observatory's power comes from its ability to understand code structure.
//! But no single tool can understand every domain-specific pattern, framework quirk,
//! or custom convention. The Plugin Nebula solves this by allowing the community
//! to extend the telescope's vision.
//!
//! ## Why Lua?
//!
//! Lua was chosen for plugins because:
//! - **Embeddable**: Small runtime, easy to sandbox
//! - **Fast**: LuaJIT is one of the fastest scripting languages
//! - **Simple**: Easy to learn, hard to abuse
//! - **Proven**: Used by Neovim, Redis, game engines
//!
//! ## Plugin Types
//!
//! ### 1. Syntax Enhancers
//!
//! Extend or modify how the Tree-sitter AST is processed:
//!
//! ```lua
//! -- Example: Django model analyzer
//! voyager.register_syntax_hook({
//!     name = "django-models",
//!     language = "python",
//!     pattern = "class.*\\(models\\.Model\\)",
//!
//!     on_class = function(symbol, ast)
//!         -- Extract Django-specific metadata
//!         symbol.metadata["django_model"] = true
//!         symbol.metadata["fields"] = extract_model_fields(ast)
//!         return symbol
//!     end
//! })
//! ```
//!
//! ### 2. Lens Plugins
//!
//! Create custom spectral filters:
//!
//! ```lua
//! -- Example: Security audit lens
//! voyager.register_lens({
//!     name = "security-audit",
//!     description = "Highlight security-sensitive code",
//!
//!     priority_rules = {
//!         { pattern = "auth/**", priority = 100 },
//!         { pattern = "crypto/**", priority = 95 },
//!         { pattern = "**/password*", priority = 90 },
//!     },
//!
//!     symbol_filter = function(symbol)
//!         -- Include if touches security-related patterns
//!         return symbol.name:match("auth") or
//!                symbol.name:match("token") or
//!                symbol.decorators:contains("@requires_auth")
//!     end
//! })
//! ```
//!
//! ### 3. Format Plugins
//!
//! Custom output formatters:
//!
//! ```lua
//! -- Example: Markdown with diagrams
//! voyager.register_format({
//!     name = "mermaid-md",
//!     extension = ".mmd.md",
//!
//!     render = function(context)
//!         local output = {}
//!
//!         -- Generate Mermaid class diagram
//!         table.insert(output, "```mermaid")
//!         table.insert(output, "classDiagram")
//!         for _, symbol in ipairs(context.types) do
//!             table.insert(output, format_class(symbol))
//!         end
//!         table.insert(output, "```")
//!
//!         return table.concat(output, "\n")
//!     end
//! })
//! ```
//!
//! ### 4. Explorer Plugins
//!
//! Custom exploration intents:
//!
//! ```lua
//! -- Example: API documentation explorer
//! voyager.register_intent({
//!     name = "api-docs",
//!     description = "Navigate API documentation patterns",
//!
//!     entry_points = function(context)
//!         return context.symbols:filter(function(s)
//!             return s.decorators:match("@api_endpoint") or
//!                    s.name:match("^handle_")
//!         end)
//!     end,
//!
//!     related_symbols = function(symbol)
//!         -- Find related DTOs, validators, etc.
//!         return find_api_dependencies(symbol)
//!     end
//! })
//! ```
//!
//! ## Plugin Discovery
//!
//! Plugins are discovered from:
//! 1. `~/.voyager/plugins/` - User plugins
//! 2. `.voyager/plugins/` - Project plugins
//! 3. `voyager.plugins` in `Cargo.toml` - Declared dependencies
//!
//! ## Security Model
//!
//! Plugins run in a sandboxed Lua environment with:
//! - No filesystem access (beyond provided APIs)
//! - No network access
//! - CPU time limits
//! - Memory limits
//! - Allowlist-based API access
//!
//! ## Integration with SyntaxProvider
//!
//! The `SyntaxProvider` trait includes a reserved method:
//!
//! ```rust,ignore
//! fn apply_plugin_hook(&mut self, hook: PluginHook) -> Result<(), SyntaxError>;
//! ```
//!
//! In Phase 2, this will be implemented to:
//! 1. Load the Lua script from `hook.lua_source`
//! 2. Register callbacks for AST traversal
//! 3. Execute callbacks during symbol extraction
//! 4. Merge plugin-provided metadata into symbols
//!
//! ## Roadmap
//!
//! ### Phase 2A: Foundation
//! - [ ] Embed mlua runtime
//! - [ ] Implement sandboxing
//! - [ ] Create voyager.* API surface
//! - [ ] Plugin discovery and loading
//!
//! ### Phase 2B: Syntax Hooks
//! - [ ] Connect to TreeSitterAdapter
//! - [ ] Symbol transformation pipeline
//! - [ ] Metadata injection
//!
//! ### Phase 2C: Ecosystem
//! - [ ] Plugin registry (voyager.dev)
//! - [ ] Version management
//! - [ ] Documentation generator
//!
//! ## Current Status
//!
//! **This module is a design reservation.** No Lua runtime is included in Phase 1A.
//! The types and traits defined here establish the contract for Phase 2 development.

use std::collections::HashMap;

/// Plugin manifest describing a Voyager Observatory plugin
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// Unique plugin identifier (e.g., "voyager-django")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Plugin author
    pub author: Option<String>,

    /// Short description
    pub description: String,

    /// Minimum Voyager Observatory version required
    pub min_voyager_version: String,

    /// Plugin entry point (Lua file)
    pub entry_point: String,

    /// Required permissions
    pub permissions: Vec<PluginPermission>,

    /// Supported languages (for syntax plugins)
    pub languages: Vec<String>,
}

/// Permissions a plugin may request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginPermission {
    /// Read files in the project directory
    ReadProjectFiles,

    /// Modify symbol metadata
    ModifySymbols,

    /// Register custom lenses
    RegisterLens,

    /// Register custom formats
    RegisterFormat,

    /// Register exploration intents
    RegisterIntent,

    /// Access the Observer's Journal
    AccessJournal,
}

/// Plugin execution context provided to Lua scripts
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Current project root
    pub project_root: String,

    /// Active lens (if any)
    pub active_lens: Option<String>,

    /// Available symbols
    pub symbol_count: usize,

    /// Language being processed
    pub language: Option<String>,

    /// Plugin-specific configuration
    pub config: HashMap<String, String>,
}

/// Result of plugin execution
#[derive(Debug, Clone)]
pub struct PluginResult {
    /// Whether the plugin executed successfully
    pub success: bool,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Number of symbols modified
    pub symbols_modified: usize,

    /// Any warnings produced
    pub warnings: Vec<String>,

    /// Error message (if success is false)
    pub error: Option<String>,
}

/// Trait for plugin host implementations (Phase 2)
///
/// This trait will be implemented by the Lua runtime wrapper in Phase 2.
/// For now, it serves as a contract for the expected interface.
pub trait PluginHost: Send + Sync {
    /// Load a plugin from its manifest
    fn load_plugin(&mut self, manifest: &PluginManifest) -> Result<(), PluginError>;

    /// Execute a plugin with the given context
    fn execute(&self, plugin_id: &str, context: PluginContext)
        -> Result<PluginResult, PluginError>;

    /// Unload a plugin
    fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError>;

    /// Get all loaded plugins
    fn loaded_plugins(&self) -> Vec<String>;

    /// Check if a plugin is loaded
    fn is_loaded(&self, plugin_id: &str) -> bool;
}

/// Errors that can occur during plugin operations
#[derive(Debug, Clone)]
pub enum PluginError {
    /// Plugin not found
    NotFound(String),

    /// Failed to load plugin
    LoadError(String),

    /// Plugin execution failed
    ExecutionError(String),

    /// Permission denied
    PermissionDenied(String),

    /// Plugin version incompatible
    VersionMismatch { required: String, found: String },

    /// Sandbox violation
    SandboxViolation(String),

    /// Timeout exceeded
    Timeout { limit_ms: u64 },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::NotFound(id) => write!(f, "Plugin not found: {}", id),
            PluginError::LoadError(msg) => write!(f, "Failed to load plugin: {}", msg),
            PluginError::ExecutionError(msg) => write!(f, "Plugin execution failed: {}", msg),
            PluginError::PermissionDenied(perm) => write!(f, "Permission denied: {}", perm),
            PluginError::VersionMismatch { required, found } => {
                write!(
                    f,
                    "Version mismatch: requires {}, found {}",
                    required, found
                )
            }
            PluginError::SandboxViolation(msg) => write!(f, "Sandbox violation: {}", msg),
            PluginError::Timeout { limit_ms } => write!(f, "Plugin timed out after {}ms", limit_ms),
        }
    }
}

impl std::error::Error for PluginError {}

/// Placeholder plugin host for Phase 1A
///
/// This implementation returns `NotImplemented` for all operations.
/// It will be replaced with a real Lua-based implementation in Phase 2.
pub struct PlaceholderPluginHost;

impl PluginHost for PlaceholderPluginHost {
    fn load_plugin(&mut self, _manifest: &PluginManifest) -> Result<(), PluginError> {
        Err(PluginError::LoadError(
            "Plugin system not yet implemented (Phase 2)".to_string(),
        ))
    }

    fn execute(
        &self,
        plugin_id: &str,
        _context: PluginContext,
    ) -> Result<PluginResult, PluginError> {
        Err(PluginError::NotFound(format!(
            "Plugin '{}' not available (Phase 2)",
            plugin_id
        )))
    }

    fn unload_plugin(&mut self, _plugin_id: &str) -> Result<(), PluginError> {
        Ok(()) // No-op for placeholder
    }

    fn loaded_plugins(&self) -> Vec<String> {
        Vec::new()
    }

    fn is_loaded(&self, _plugin_id: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest_creation() {
        let manifest = PluginManifest {
            id: "voyager-django".to_string(),
            name: "Django Model Analyzer".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Voyager Community".to_string()),
            description: "Extract Django model metadata".to_string(),
            min_voyager_version: "2.0.0".to_string(),
            entry_point: "init.lua".to_string(),
            permissions: vec![PluginPermission::ModifySymbols],
            languages: vec!["python".to_string()],
        };

        assert_eq!(manifest.id, "voyager-django");
        assert!(manifest
            .permissions
            .contains(&PluginPermission::ModifySymbols));
    }

    #[test]
    fn test_placeholder_host_returns_not_implemented() {
        let mut host = PlaceholderPluginHost;

        let manifest = PluginManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: "Test".to_string(),
            min_voyager_version: "2.0.0".to_string(),
            entry_point: "init.lua".to_string(),
            permissions: vec![],
            languages: vec![],
        };

        let result = host.load_plugin(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_error_display() {
        let error = PluginError::NotFound("my-plugin".to_string());
        assert!(error.to_string().contains("my-plugin"));

        let error = PluginError::Timeout { limit_ms: 5000 };
        assert!(error.to_string().contains("5000"));
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_plugin_context_creation() {
        let mut config = HashMap::new();
        config.insert("key".to_string(), "value".to_string());

        let context = PluginContext {
            project_root: "/project".to_string(),
            active_lens: Some("security".to_string()),
            symbol_count: 42,
            language: Some("rust".to_string()),
            config,
        };

        assert_eq!(context.project_root, "/project");
        assert_eq!(context.active_lens, Some("security".to_string()));
        assert_eq!(context.symbol_count, 42);
        assert_eq!(context.language, Some("rust".to_string()));
        assert_eq!(context.config.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_plugin_result_creation() {
        let result = PluginResult {
            success: true,
            execution_time_ms: 100,
            symbols_modified: 5,
            warnings: vec!["warn1".to_string()],
            error: None,
        };

        assert!(result.success);
        assert_eq!(result.execution_time_ms, 100);
        assert_eq!(result.symbols_modified, 5);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_plugin_permissions_equality() {
        assert_eq!(
            PluginPermission::ReadProjectFiles,
            PluginPermission::ReadProjectFiles
        );
        assert_ne!(
            PluginPermission::ReadProjectFiles,
            PluginPermission::ModifySymbols
        );
        assert_eq!(
            PluginPermission::RegisterLens,
            PluginPermission::RegisterLens
        );
        assert_eq!(
            PluginPermission::RegisterFormat,
            PluginPermission::RegisterFormat
        );
        assert_eq!(
            PluginPermission::RegisterIntent,
            PluginPermission::RegisterIntent
        );
        assert_eq!(
            PluginPermission::AccessJournal,
            PluginPermission::AccessJournal
        );
    }

    #[test]
    fn test_plugin_error_all_variants() {
        // NotFound
        let e = PluginError::NotFound("plugin".to_string());
        assert!(e.to_string().contains("not found"));

        // LoadError
        let e = PluginError::LoadError("load failed".to_string());
        assert!(e.to_string().contains("load"));

        // ExecutionError
        let e = PluginError::ExecutionError("execution failed".to_string());
        assert!(e.to_string().contains("execution"));

        // PermissionDenied
        let e = PluginError::PermissionDenied("filesystem".to_string());
        assert!(e.to_string().contains("Permission denied"));

        // VersionMismatch
        let e = PluginError::VersionMismatch {
            required: "2.0.0".to_string(),
            found: "1.0.0".to_string(),
        };
        assert!(e.to_string().contains("2.0.0"));
        assert!(e.to_string().contains("1.0.0"));

        // SandboxViolation
        let e = PluginError::SandboxViolation("forbidden".to_string());
        assert!(e.to_string().contains("Sandbox"));

        // Timeout
        let e = PluginError::Timeout { limit_ms: 1000 };
        assert!(e.to_string().contains("1000"));
    }

    #[test]
    fn test_plugin_error_is_error_trait() {
        let error: Box<dyn std::error::Error> = Box::new(PluginError::NotFound("test".to_string()));
        assert!(error.to_string().contains("test"));
    }

    #[test]
    fn test_placeholder_host_execute() {
        let host = PlaceholderPluginHost;
        let context = PluginContext {
            project_root: "/test".to_string(),
            active_lens: None,
            symbol_count: 0,
            language: None,
            config: HashMap::new(),
        };

        let result = host.execute("any-plugin", context);
        assert!(result.is_err());
        match result {
            Err(PluginError::NotFound(id)) => assert!(id.contains("any-plugin")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_placeholder_host_unload() {
        let mut host = PlaceholderPluginHost;
        // unload_plugin is a no-op, should return Ok
        assert!(host.unload_plugin("any").is_ok());
    }

    #[test]
    fn test_placeholder_host_loaded_plugins() {
        let host = PlaceholderPluginHost;
        let plugins = host.loaded_plugins();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_placeholder_host_is_loaded() {
        let host = PlaceholderPluginHost;
        assert!(!host.is_loaded("any-plugin"));
    }
}
