//! Core module for pm_encoder Context Kernel
//!
//! This module provides the foundational types and traits for the context serialization engine.
//! It follows a modular architecture for testability and extensibility.
//!
//! # Architecture
//!
//! - `models`: Core data structures (FileEntry, EncoderConfig, ProcessedFile)
//! - `error`: Error types using thiserror
//! - `walker`: Directory traversal with FileWalker trait
//! - `serialization`: Output format serializers
//! - `engine`: Main ContextEngine orchestration
//! - `zoom`: Fractal Protocol zoom actions

pub mod models;
pub mod error;
pub mod walker;
pub mod serialization;
pub mod engine;
pub mod zoom;
pub mod store;
pub mod search;

// Re-export commonly used types
pub use models::{FileEntry, EncoderConfig, ProcessedFile, OutputFormat, Config};
pub use error::{EncoderError, Result};
pub use walker::{FileWalker, DefaultWalker};
pub use engine::ContextEngine;
pub use zoom::{
    ZoomAction, ZoomTarget, ZoomConfig, ZoomDepth,
    // Fractal Protocol v2
    ZoomDirection, ZoomHistory, ZoomHistoryEntry,
    ZoomSession, ZoomSessionStore,
};
pub use store::{ContextStore, FileUtility, DEFAULT_ALPHA};
pub use search::{
    SymbolResolver, SymbolLocation, SymbolType,
    CallGraphAnalyzer, FunctionCall, ZoomSuggestion,
};
