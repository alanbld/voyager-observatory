//! Celestial Module - Spectral Synthesis
//!
//! This module synthesizes semantic analysis into a "Celestial Map" -
//! grouping files into named Nebulae based on semantic similarity.
//!
//! # Architecture
//!
//! ```text
//! SemanticSubstrate → NebulaNamer → ConstellationMapper → CelestialMap
//!        ↓                  ↓               ↓                   ↓
//!    Concepts          Names           Groupings          Output
//! ```
//!
//! # The Celestial Metaphor
//!
//! - **Stars**: Individual files with brightness based on utility
//! - **Nebulae**: Clusters of semantically similar files
//! - **Constellations**: Higher-level groupings (e.g., all business logic)
//! - **Compass**: Navigation suggestions for exploration

pub mod compass;
pub mod constellation_mapper;
pub mod nebula_namer;

pub use compass::{ExplorationHint, NavigationCompass, NavigationSuggestion, SuggestionAction};
pub use constellation_mapper::{CelestialMap, ConstellationMapper, FileInfo, Nebula, Star};
pub use nebula_namer::{NamingStrategy, NebulaName, NebulaNamer};
