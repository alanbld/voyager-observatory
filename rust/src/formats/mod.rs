//! Output format modules for pm_encoder
//!
//! This module provides streaming formatters for various output formats.
//! All formatters use the `std::io::Write` trait for WASM compatibility.

pub mod xml_writer;

pub use xml_writer::{XmlWriter, XmlConfig, XmlError, AttentionEntry, escape_cdata};
