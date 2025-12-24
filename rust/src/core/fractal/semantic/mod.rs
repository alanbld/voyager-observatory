//! Multi-Language Semantic Substrate
//!
//! This module provides a unified semantic space for analyzing mixed-language
//! codebases. It enables cross-language concept alignment, feature vector
//! normalization, and polyglot intent-driven exploration.
//!
//! # Architecture
//!
//! The semantic substrate bridges language-specific plugins into a unified space:
//!
//! ```text
//! ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
//! │  ABL Plugin │  │Python Plugin│  │  TS Plugin  │
//! └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
//!        │                │                │
//!        ▼                ▼                ▼
//! ┌─────────────────────────────────────────────────┐
//! │           Unified Semantic Substrate            │
//! │  ┌─────────────────────────────────────────┐   │
//! │  │    Cross-Language Concept Alignment     │   │
//! │  └─────────────────────────────────────────┘   │
//! │  ┌─────────────────────────────────────────┐   │
//! │  │   Normalized 64D Feature Vectors        │   │
//! │  └─────────────────────────────────────────┘   │
//! │  ┌─────────────────────────────────────────┐   │
//! │  │   Cross-Language Relationships          │   │
//! │  └─────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────┘
//!        │
//!        ▼
//! ┌─────────────────────────────────────────────────┐
//! │        Intent-Driven Exploration                │
//! │   (Polyglot exploration across languages)       │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`UnifiedSemanticSubstrate`]: The main data structure holding unified concepts
//! - [`CrossLanguageAligner`]: Finds equivalent concepts across languages
//! - [`FeatureNormalizer`]: Normalizes 64D vectors across languages
//! - [`MultiLanguageProject`]: Represents a mixed-language project
//!
//! # Example
//!
//! ```ignore
//! use pm_encoder::core::fractal::semantic::{
//!     UnifiedSemanticSubstrate, MultiLanguageProject,
//! };
//!
//! // Analyze a mixed-language project
//! let project = MultiLanguageProject::from_path("./my_project")?;
//! let substrate = UnifiedSemanticSubstrate::from_project(&project)?;
//!
//! // Find cross-language equivalents
//! let equivalents = substrate.find_equivalents("calculate_total");
//! ```

pub mod unified_substrate;
pub mod cross_language;
pub mod normalization;
pub mod multi_language;

pub use unified_substrate::{
    UnifiedSemanticSubstrate, UnifiedConcept, ConceptId,
    LanguageSpecificData, UnifiedProperties, UniversalConceptType,
};
pub use cross_language::{
    CrossLanguageAligner, CrossLanguageEquivalent, CrossLanguageRelationship,
    EquivalenceClass,
};
pub use normalization::{
    FeatureNormalizer, NormalizationStrategy, LanguageNormalizationConfig,
};
pub use multi_language::{
    MultiLanguageProject, MultiLanguageExplorer, LanguageBreakdown, ProjectLanguageStats,
    MultiLanguageExplorationResult, CrossLanguageExplorationStep,
    CrossLanguageInsight,
};

// =============================================================================
// Language Enum (shared across semantic module)
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported languages for multi-language analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    ABL,
    Python,
    TypeScript,
    JavaScript,
    Shell,
    Rust,
    Go,
    Java,
    CSharp,
    Ruby,
    Unknown,
}

impl Language {
    /// Get file extensions for this language
    pub fn extensions(&self) -> &[&'static str] {
        match self {
            Language::ABL => &["p", "w", "cls", "i"],
            Language::Python => &["py", "pyw", "pyi"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Language::Shell => &["sh", "bash", "zsh", "ksh"],
            Language::Rust => &["rs"],
            Language::Go => &["go"],
            Language::Java => &["java"],
            Language::CSharp => &["cs"],
            Language::Ruby => &["rb"],
            Language::Unknown => &[],
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        let ext_lower = ext.to_lowercase();
        match ext_lower.as_str() {
            "p" | "w" | "cls" | "i" => Language::ABL,
            "py" | "pyw" | "pyi" => Language::Python,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "sh" | "bash" | "zsh" | "ksh" => Language::Shell,
            "rs" => Language::Rust,
            "go" => Language::Go,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            "rb" => Language::Ruby,
            _ => Language::Unknown,
        }
    }

    /// Get language name for display
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::ABL => "ABL (OpenEdge)",
            Language::Python => "Python",
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::Shell => "Shell",
            Language::Rust => "Rust",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::CSharp => "C#",
            Language::Ruby => "Ruby",
            Language::Unknown => "Unknown",
        }
    }

    /// Check if this language has a plugin implementation
    pub fn has_plugin(&self) -> bool {
        matches!(
            self,
            Language::ABL
                | Language::Python
                | Language::TypeScript
                | Language::JavaScript
                | Language::Shell
        )
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "abl" | "openedge" | "progress" => Ok(Language::ABL),
            "python" | "py" => Ok(Language::Python),
            "typescript" | "ts" => Ok(Language::TypeScript),
            "javascript" | "js" => Ok(Language::JavaScript),
            "shell" | "bash" | "sh" => Ok(Language::Shell),
            "rust" | "rs" => Ok(Language::Rust),
            "go" | "golang" => Ok(Language::Go),
            "java" => Ok(Language::Java),
            "csharp" | "c#" | "cs" => Ok(Language::CSharp),
            "ruby" | "rb" => Ok(Language::Ruby),
            _ => Err(format!("Unknown language: {}", s)),
        }
    }
}

// =============================================================================
// User Context for Multi-Language Exploration
// =============================================================================

use std::collections::HashMap;

/// User context for personalized multi-language exploration
#[derive(Debug, Clone, Default)]
pub struct UserContext {
    /// User's familiarity with each language (0.0 - 1.0)
    pub language_familiarity: HashMap<Language, f32>,
    /// Languages to ignore/exclude from exploration
    pub ignore_languages: Vec<Language>,
    /// Preferred exploration depth per language
    pub depth_preferences: HashMap<Language, u32>,
    /// Maximum time budget in minutes
    pub time_budget_minutes: Option<u32>,
}

impl UserContext {
    /// Create a new user context with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set familiarity for a language
    pub fn with_familiarity(mut self, language: Language, familiarity: f32) -> Self {
        self.language_familiarity
            .insert(language, familiarity.clamp(0.0, 1.0));
        self
    }

    /// Ignore a language
    pub fn ignoring(mut self, language: Language) -> Self {
        if !self.ignore_languages.contains(&language) {
            self.ignore_languages.push(language);
        }
        self
    }

    /// Set time budget
    pub fn with_time_budget(mut self, minutes: u32) -> Self {
        self.time_budget_minutes = Some(minutes);
        self
    }

    /// Get familiarity for a language (default 0.5)
    pub fn get_familiarity(&self, language: Language) -> f32 {
        self.language_familiarity
            .get(&language)
            .copied()
            .unwrap_or(0.5)
    }

    /// Check if a language should be ignored
    pub fn should_ignore(&self, language: Language) -> bool {
        self.ignore_languages.contains(&language)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("p"), Language::ABL);
        assert_eq!(Language::from_extension("sh"), Language::Shell);
        assert_eq!(Language::from_extension("xyz"), Language::Unknown);
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!("python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("TypeScript".parse::<Language>().unwrap(), Language::TypeScript);
        assert_eq!("ABL".parse::<Language>().unwrap(), Language::ABL);
        assert!("invalid".parse::<Language>().is_err());
    }

    #[test]
    fn test_language_extensions() {
        assert!(Language::Python.extensions().contains(&"py"));
        assert!(Language::TypeScript.extensions().contains(&"tsx"));
        assert!(Language::ABL.extensions().contains(&"p"));
    }

    #[test]
    fn test_user_context() {
        let ctx = UserContext::new()
            .with_familiarity(Language::Python, 0.9)
            .with_familiarity(Language::ABL, 0.3)
            .ignoring(Language::Shell)
            .with_time_budget(60);

        assert_eq!(ctx.get_familiarity(Language::Python), 0.9);
        assert_eq!(ctx.get_familiarity(Language::ABL), 0.3);
        assert_eq!(ctx.get_familiarity(Language::TypeScript), 0.5); // default
        assert!(ctx.should_ignore(Language::Shell));
        assert!(!ctx.should_ignore(Language::Python));
        assert_eq!(ctx.time_budget_minutes, Some(60));
    }
}
