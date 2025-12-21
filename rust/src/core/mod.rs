//! Core module for pm_encoder Context Kernel
//!
//! This module provides the foundational types and traits for the context serialization engine.
//! It follows a modular architecture for testability and extensibility.
//!
//! # Architecture
//!
//! - `models`: Core data structures (FileEntry, EncoderConfig, ProcessedFile)
//! - `error`: Error types using thiserror
//! - `walker`: Directory traversal with FileWalker trait + SmartWalker
//! - `manifest`: Project boundary detection
//! - `serialization`: Output format serializers
//! - `engine`: Main ContextEngine orchestration
//! - `zoom`: Fractal Protocol zoom actions

pub mod models;
pub mod error;
pub mod walker;
pub mod manifest;
pub mod serialization;
pub mod engine;
pub mod zoom;
pub mod store;
pub mod search;
pub mod skeleton;

// Re-export commonly used types
pub use models::{FileEntry, EncoderConfig, ProcessedFile, OutputFormat, Config, SkeletonMode, CompressionLevel};
pub use error::{EncoderError, Result};
pub use walker::{FileWalker, DefaultWalker, SmartWalker, SmartWalkConfig, WalkEntry};
pub use manifest::{ProjectManifest, ProjectType};
pub use engine::{ContextEngine, FileTier, BudgetStats};
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
    // Phase 2: Reverse call graph
    UsageLocation, UsageFinder, RelatedContext,
};
