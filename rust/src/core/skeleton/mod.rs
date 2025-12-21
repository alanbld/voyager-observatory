//! Skeleton Protocol v2.2 - Adaptive Skeletonization
//!
//! This module provides intelligent code compression by extracting signatures
//! and stripping implementation details while staying within token budgets.
//!
//! ## Key Components
//!
//! - [`Skeletonizer`]: Extracts signatures from code files
//! - [`AdaptiveAllocator`]: Budget-aware compression level allocation
//! - [`CompressionLevel`]: Full, Skeleton, or Drop
//! - [`Language`]: Supported languages for parsing
//!
//! ## Example
//!
//! ```rust,ignore
//! use pm_encoder::core::skeleton::{Skeletonizer, Language, AdaptiveAllocator, FileAllocation};
//! use pm_encoder::core::FileTier;
//!
//! // Skeletonize a Rust file
//! let skeletonizer = Skeletonizer::new();
//! let result = skeletonizer.skeletonize(rust_code, Language::Rust);
//!
//! // Allocate compression levels within budget
//! let allocator = AdaptiveAllocator::new(10000);
//! let files = vec![
//!     FileAllocation::new("src/main.rs", FileTier::Core, 500, 50),
//!     FileAllocation::new("tests/test.rs", FileTier::Tests, 300, 30),
//! ];
//! let allocated = allocator.allocate(files);
//! ```

mod allocator;
mod parser;
mod types;

#[cfg(test)]
mod tests;

pub use allocator::AdaptiveAllocator;
pub use parser::Skeletonizer;
pub use types::{CompressionLevel, FileAllocation, Language, SkeletonResult};
