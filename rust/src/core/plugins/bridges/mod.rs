//! Plugin Bridges
//!
//! Provides the `vo.*` API bridge for Lua plugins to interact
//! with the Voyager Observatory core functionality.

#[cfg(feature = "plugins")]
pub mod patterns;
#[cfg(feature = "plugins")]
pub mod vo_table;

#[cfg(feature = "plugins")]
pub use patterns::create_patterns_table;
#[cfg(feature = "plugins")]
pub use vo_table::create_vo_table;
