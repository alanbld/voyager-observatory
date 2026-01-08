//! Plugin Ecosystem Module
//!
//! Implements the Three-Layer Sovereignty Model for community plugins.
//! Plugins run in a secure Lua sandbox (Iron Sandbox) with strict limits:
//! - 100ms CPU timeout
//! - 10MB memory limit
//! - Stripped dangerous libraries (io, os, debug, package)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │          LAYER 3: LUA PLUGINS           │
//! │  • Append-only contributions            │
//! │  • vo.* API bridge                      │
//! ├─────────────────────────────────────────┤
//! │         LAYER 2: SYNTAX PROVIDERS       │
//! │  • Tree-sitter parsers                  │
//! │  • Regex engine                         │
//! ├─────────────────────────────────────────┤
//! │          LAYER 1: FRACTAL CORE          │
//! │  • Normalized AST                       │
//! │  • Immutable data structures            │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Feature Gate
//!
//! This module requires the `plugins` feature:
//! ```toml
//! [dependencies]
//! voyager-observatory = { features = ["plugins"] }
//! ```

pub mod engine;
pub mod error;
pub mod loader;
pub mod sandbox;

#[cfg(feature = "plugins")]
pub mod bridges;

// Re-exports
pub use engine::{EngineState, PluginEngine};
pub use error::{PluginError, PluginResult};
pub use loader::{
    LoadedPlugin, PluginEntry, PluginLoader, PluginManifest, PluginStatus, CURRENT_API_VERSION,
};
pub use sandbox::{MEMORY_LIMIT, TIMEOUT_MS};

#[cfg(feature = "plugins")]
pub use sandbox::IronSandbox;

#[cfg(feature = "plugins")]
pub use bridges::vo_table::{
    create_vo_table, create_vo_table_simple, LogEntry, MetricValue, PluginContributions,
    SharedContributions, API_VERSION,
};

/// Check if plugin feature is available at runtime
pub fn is_plugins_available() -> bool {
    cfg!(feature = "plugins")
}

/// Get plugin feature description
pub fn plugins_feature_description() -> &'static str {
    if cfg!(feature = "plugins") {
        "Plugin ecosystem enabled (Iron Sandbox active)"
    } else {
        "Plugin ecosystem disabled (compile with --features plugins)"
    }
}
