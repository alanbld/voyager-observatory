//! voyager-ast: Language-agnostic Structural Indexer
//!
//! This crate provides the "optics layer" for Voyager Observatory, enabling
//! fast, resilient, multi-language structural indexing via Tree-sitter.
//!
//! # Design Philosophy: Telescope, Not Compiler
//!
//! voyager-ast is explicitly designed as an observation instrument:
//! - Best-effort recovery over formal correctness
//! - ~90% structural accuracy is the target, not 100%
//! - Explicit uncertainty via `UnknownNode` markers
//! - Never silently drop content we can't parse
//!
//! # Parsing
//!
//! `AdapterRegistry::parse` is the single entry point: it parses source into
//! a `File` IR and populates every declaration's `body` (control flow, calls,
//! nested declarations) inline, so consumers get full structural data from
//! one parse — no separate indexing/zoom pass is needed.
//!
//! # Example
//!
//! ```rust,ignore
//! use voyager_ast::{AdapterRegistry, LanguageId};
//!
//! let registry = AdapterRegistry::new();
//! let file = registry.parse("fn hello() {}", LanguageId::Rust)?;
//!
//! for decl in &file.declarations {
//!     println!("{}: {:?}", decl.name, decl.kind);
//! }
//! ```

// Pre-existing lint debt as of the Phase 0 safety pass (REVIEW_ROADMAP.md).
// See rust/src/lib.rs for the full rationale; cleanup tracked as Phase 1
// hygiene (N6) in REVIEW_ROADMAP.md.
#![allow(clippy::len_zero)]
#![allow(clippy::unnecessary_get_then_check)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(clippy::clone_on_copy)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod adapters;
pub mod error;
pub mod ir;
mod registry;

// Re-export core types for convenience
pub use ir::{
    // Blocks and control flow
    Block,
    Call,
    // Comments
    Comment,
    CommentKind,
    ControlFlow,
    ControlFlowKind,
    // Declarations
    Declaration,
    DeclarationKind,
    // Core types
    File,
    ImportKind,
    // Imports
    ImportLike,
    // Language
    LanguageId,
    Parameter,
    Region,
    Span,
    // Error recovery
    UnknownNode,
    UnparsedBlock,
    Visibility,
};

pub use adapters::LanguageAdapter;
pub use error::AstError;
pub use registry::AdapterRegistry;

/// Version of the IR schema
pub const IR_VERSION: &str = "v1";

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
