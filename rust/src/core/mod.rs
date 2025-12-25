//! Core module for Voyager Observatory Context Kernel
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
//! - `fractal`: Fractal Context Engine for hierarchical, zoomable context
//! - `syntax`: Tree-sitter based AST parsing (Phase 1A)
//! - `plugin`: Plugin ecosystem reservation (Phase 2)
//! - `ast_bridge`: Bridge to voyager-ast structural optics
//! - `metrics`: AST-based code metrics collection (Phase 3 foundation)

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
pub mod fractal;
pub mod orchestrator;
pub mod presenter;
pub mod celestial;
pub mod syntax;
pub mod plugin;
pub mod ast_bridge;
pub mod metrics;
pub mod regex_engine;

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

// Phase 2 Week 2: Intent-Driven Exploration
pub use fractal::{
    IntentExplorer, ExplorerConfig, ExplorationResult,
    ExplorationIntent, IntentComposition, IntentResult,
    ExplorationStep, ReadingDecision, StopReadingEngine,
    ConceptType,
};

// Phase 2 Week 3: Fractal Telescope UX
pub use orchestrator::{
    SmartOrchestrator, AutoFocus, InputType,
    SmartDefaults, SemanticDepth, DetailLevel,
    AnalysisStrategy, FallbackSystem,
    // Observer's Journal
    ObserversJournal, MarkedStar, ExplorationEntry, FadedNebula,
};
pub use presenter::{
    IntelligentPresenter, EmojiFormatter, Theme,
    SemanticTransparency,
};

// Phase 3: Spectral Synthesis (Celestial Navigation)
pub use celestial::{
    NebulaNamer, NebulaName, NamingStrategy,
    ConstellationMapper, Nebula, CelestialMap, Star, FileInfo,
    NavigationCompass, NavigationSuggestion, ExplorationHint, SuggestionAction,
};

// Phase 1A: Core Syntax Infrastructure (Tree-sitter)
pub use syntax::{
    SyntaxRegistry, SyntaxProvider, TreeSitterAdapter,
    NormalizedAst, Symbol, SymbolKind, SymbolVisibility,
    Import, ImportKind, Location, Span, Language as SyntaxLanguage,
    SyntaxError, ProviderStats,
};

// voyager-ast integration (Structural Optics)
pub use ast_bridge::{
    AstBridge, Star as AstStar, StarKind, FileSummary, StarSummary,
};

// Phase 0 Hardening: Centralized Regex Engine
pub use regex_engine::{
    RegexEngine, CompiledRegex, RegexError, MatchRange, MatchResult,
    PatternSet, compile, is_match, find_all, replace_all, global_engine,
};
