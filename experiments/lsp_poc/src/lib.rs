//! LSP Protocol Proof of Concept
//!
//! This crate validates the "Semantic Bridge" concept by implementing
//! a JSON-RPC 2.0 client that can communicate with Language Server Protocol servers.
//!
//! ## Modules
//!
//! - `protocol`: JSON-RPC 2.0 message types
//! - `metrics`: Latency and accuracy measurement infrastructure
//! - `comparison`: Regex vs LSP symbol extraction comparison

pub mod protocol;
pub mod metrics;
pub mod comparison;
