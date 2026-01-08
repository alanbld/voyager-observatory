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

pub mod ast_bridge;
pub mod celestial;
pub mod census;
pub mod engine;
pub mod error;
pub mod fractal;
pub mod manifest;
pub mod metrics;
pub mod models;
pub mod orchestrator;
pub mod plugin;
pub mod plugins;
pub mod presenter;
pub mod regex_engine;
pub mod search;
pub mod serialization;
pub mod skeleton;
pub mod spectrograph;
pub mod store;
pub mod syntax;
pub mod temporal;
pub mod walker;
pub mod zoom;

// Re-export commonly used types
pub use engine::{BudgetStats, ContextEngine, FileTier};
pub use error::{EncoderError, Result};
pub use manifest::{ProjectManifest, ProjectType};
pub use models::{
    CompressionLevel, Config, EncoderConfig, FileEntry, OutputFormat, ProcessedFile, SkeletonMode,
};
pub use search::{
    CallGraphAnalyzer,
    FunctionCall,
    RelatedContext,
    SymbolLocation,
    SymbolResolver,
    SymbolType,
    UsageFinder,
    // Phase 2: Reverse call graph
    UsageLocation,
    ZoomSuggestion,
};
pub use store::{ContextStore, FileUtility, DEFAULT_ALPHA};
pub use walker::{DefaultWalker, FileWalker, SmartWalkConfig, SmartWalker, WalkEntry};
pub use zoom::{
    ZoomAction,
    ZoomConfig,
    ZoomDepth,
    // Fractal Protocol v2
    ZoomDirection,
    ZoomHistory,
    ZoomHistoryEntry,
    ZoomSession,
    ZoomSessionStore,
    ZoomTarget,
};

// Phase 2 Week 2: Intent-Driven Exploration
pub use fractal::{
    ConceptType, ExplorationIntent, ExplorationResult, ExplorationStep, ExplorerConfig,
    IntentComposition, IntentExplorer, IntentResult, ReadingDecision, StopReadingEngine,
};

// Phase 2 Week 3: Fractal Telescope UX
pub use orchestrator::{
    AnalysisStrategy,
    AutoFocus,
    DetailLevel,
    ExplorationEntry,
    FadedNebula,
    FallbackSystem,
    InputType,
    MarkedStar,
    // Observer's Journal
    ObserversJournal,
    SemanticDepth,
    SmartDefaults,
    SmartOrchestrator,
};
pub use presenter::{
    // Drift Info (v1.1.0)
    DriftInfo,
    EmojiFormatter,
    IntelligentPresenter,
    SemanticTransparency,
    Theme,
};

// Phase 3: Spectral Synthesis (Celestial Navigation)
pub use celestial::{
    CelestialMap, ConstellationMapper, ExplorationHint, FileInfo, NamingStrategy,
    NavigationCompass, NavigationSuggestion, Nebula, NebulaName, NebulaNamer, Star,
    SuggestionAction,
};

// Phase 1A: Core Syntax Infrastructure (Tree-sitter)
pub use syntax::{
    Import, ImportKind, Language as SyntaxLanguage, Location, NormalizedAst, ProviderStats, Span,
    Symbol, SymbolKind, SymbolVisibility, SyntaxError, SyntaxProvider, SyntaxRegistry,
    TreeSitterAdapter,
};

// voyager-ast integration (Structural Optics)
pub use ast_bridge::{AstBridge, FileSummary, Star as AstStar, StarKind, StarSummary};

// Phase 0 Hardening: Centralized Regex Engine
pub use regex_engine::{
    compile, find_all, global_engine, is_match, replace_all, CompiledRegex, MatchRange,
    MatchResult, PatternSet, RegexEngine, RegexError,
};

// Phase 1C: Celestial Census (Code Health Metrics)
pub use census::{
    build_census_registry,
    CelestialCensus,
    CensusMetrics,
    ConstellationCensus,
    DarkMatterMetric,
    DarkMatterMetrics,
    DerivedMetrics,
    GalaxyCensus,
    HealthRating,
    HealthScoreMetric,
    NebulaRatioMetric,
    NebulaeCountMetric,
    NebulaeMetrics,
    // Universal Spectrograph fallback
    PatternFallbackAnalyzer,
    StarCountMetric,
    StarMetrics,
    StellarDensityMetric,
};

// Universal Spectrograph (80+ Language Patterns)
pub use spectrograph::{Hemisphere, SpectralSignature, StellarLibrary, STELLAR_LIBRARY};

// Phase 2: Temporal (Chronos Engine)
pub use temporal::{
    is_temporal_available,
    temporal_state_description,
    AgeClassification,
    AncientStar,
    CachedGalaxyStats,
    CachedObservation,
    // Chronos Warp (v1.2.0)
    ChronosCache,
    ChronosCacheManager,
    ChronosEngine,
    ChronosMetrics,
    ChronosState,
    ChurnClassification,
    ConstellationChurn,
    ConstellationEvolution,
    FileChurn,
    GeologicalActivity,
    GeologicalAnalyzer,
    GeologicalSummary,
    NewStar,
    Observer,
    ObserverImpact,
    StellarAge,
    // Stellar Drift (v1.1.0)
    StellarDriftAnalyzer,
    StellarDriftReport,
    Supernova,
    TectonicShift,
    TemporalCensus,
    VolcanicChurn,
    WarpStatus,
    // Shallow Chronos (v1.1.0)
    DEFAULT_COMMIT_DEPTH,
    FULL_COMMIT_DEPTH,
};

// Phase 3: Plugin Ecosystem (Iron Sandbox)
pub use plugins::{
    is_plugins_available, plugins_feature_description, EngineState, LoadedPlugin, PluginEngine,
    PluginEntry, PluginError, PluginLoader, PluginManifest, PluginResult, PluginStatus,
    CURRENT_API_VERSION, MEMORY_LIMIT, TIMEOUT_MS,
};

#[cfg(feature = "plugins")]
pub use plugins::{
    create_vo_table, create_vo_table_simple, IronSandbox, LogEntry, MetricValue,
    PluginContributions, SharedContributions,
};
