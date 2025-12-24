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

pub mod nebula_namer;
pub mod constellation_mapper;
pub mod compass;

pub use nebula_namer::{NebulaNamer, NebulaName, NamingStrategy};
pub use constellation_mapper::{ConstellationMapper, Nebula, CelestialMap, Star, FileInfo};
pub use compass::{NavigationCompass, NavigationSuggestion, ExplorationHint, SuggestionAction};
