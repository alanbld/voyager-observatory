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

pub mod cross_language;
pub mod multi_language;
pub mod normalization;
pub mod unified_substrate;

pub use cross_language::{
    CrossLanguageAligner, CrossLanguageEquivalent, CrossLanguageRelationship, EquivalenceClass,
};
pub use multi_language::{
    CrossLanguageExplorationStep, CrossLanguageInsight, LanguageBreakdown,
    MultiLanguageExplorationResult, MultiLanguageExplorer, MultiLanguageProject,
    ProjectLanguageStats,
};
pub use normalization::{FeatureNormalizer, LanguageNormalizationConfig, NormalizationStrategy};
pub use unified_substrate::{
    ConceptId, LanguageSpecificData, UnifiedConcept, UnifiedProperties, UnifiedSemanticSubstrate,
    UniversalConceptType,
};

// =============================================================================
// Language Enum (shared across semantic module)
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported languages for multi-language analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
        assert_eq!(
            "TypeScript".parse::<Language>().unwrap(),
            Language::TypeScript
        );
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

    // =========================================================================
    // Language Enum Comprehensive Tests
    // =========================================================================

    #[test]
    fn test_language_from_extension_all_abl() {
        assert_eq!(Language::from_extension("p"), Language::ABL);
        assert_eq!(Language::from_extension("w"), Language::ABL);
        assert_eq!(Language::from_extension("cls"), Language::ABL);
        assert_eq!(Language::from_extension("i"), Language::ABL);
    }

    #[test]
    fn test_language_from_extension_all_python() {
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("pyw"), Language::Python);
        assert_eq!(Language::from_extension("pyi"), Language::Python);
    }

    #[test]
    fn test_language_from_extension_all_typescript() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
    }

    #[test]
    fn test_language_from_extension_all_javascript() {
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
        assert_eq!(Language::from_extension("jsx"), Language::JavaScript);
        assert_eq!(Language::from_extension("mjs"), Language::JavaScript);
        assert_eq!(Language::from_extension("cjs"), Language::JavaScript);
    }

    #[test]
    fn test_language_from_extension_all_shell() {
        assert_eq!(Language::from_extension("sh"), Language::Shell);
        assert_eq!(Language::from_extension("bash"), Language::Shell);
        assert_eq!(Language::from_extension("zsh"), Language::Shell);
        assert_eq!(Language::from_extension("ksh"), Language::Shell);
    }

    #[test]
    fn test_language_from_extension_other_languages() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("java"), Language::Java);
        assert_eq!(Language::from_extension("cs"), Language::CSharp);
        assert_eq!(Language::from_extension("rb"), Language::Ruby);
    }

    #[test]
    fn test_language_from_extension_case_insensitive() {
        assert_eq!(Language::from_extension("PY"), Language::Python);
        assert_eq!(Language::from_extension("Ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("RS"), Language::Rust);
    }

    #[test]
    fn test_language_display_name_all() {
        assert_eq!(Language::ABL.display_name(), "ABL (OpenEdge)");
        assert_eq!(Language::Python.display_name(), "Python");
        assert_eq!(Language::TypeScript.display_name(), "TypeScript");
        assert_eq!(Language::JavaScript.display_name(), "JavaScript");
        assert_eq!(Language::Shell.display_name(), "Shell");
        assert_eq!(Language::Rust.display_name(), "Rust");
        assert_eq!(Language::Go.display_name(), "Go");
        assert_eq!(Language::Java.display_name(), "Java");
        assert_eq!(Language::CSharp.display_name(), "C#");
        assert_eq!(Language::Ruby.display_name(), "Ruby");
        assert_eq!(Language::Unknown.display_name(), "Unknown");
    }

    #[test]
    fn test_language_has_plugin() {
        assert!(Language::ABL.has_plugin());
        assert!(Language::Python.has_plugin());
        assert!(Language::TypeScript.has_plugin());
        assert!(Language::JavaScript.has_plugin());
        assert!(Language::Shell.has_plugin());
        assert!(!Language::Rust.has_plugin());
        assert!(!Language::Go.has_plugin());
        assert!(!Language::Java.has_plugin());
        assert!(!Language::CSharp.has_plugin());
        assert!(!Language::Ruby.has_plugin());
        assert!(!Language::Unknown.has_plugin());
    }

    #[test]
    fn test_language_extensions_all_variants() {
        assert!(!Language::ABL.extensions().is_empty());
        assert!(!Language::Python.extensions().is_empty());
        assert!(!Language::TypeScript.extensions().is_empty());
        assert!(!Language::JavaScript.extensions().is_empty());
        assert!(!Language::Shell.extensions().is_empty());
        assert!(!Language::Rust.extensions().is_empty());
        assert!(!Language::Go.extensions().is_empty());
        assert!(!Language::Java.extensions().is_empty());
        assert!(!Language::CSharp.extensions().is_empty());
        assert!(!Language::Ruby.extensions().is_empty());
        assert!(Language::Unknown.extensions().is_empty());
    }

    #[test]
    fn test_language_display_trait() {
        assert_eq!(format!("{}", Language::Python), "Python");
        assert_eq!(format!("{}", Language::ABL), "ABL (OpenEdge)");
        assert_eq!(format!("{}", Language::CSharp), "C#");
    }

    #[test]
    fn test_language_from_str_all_aliases() {
        // ABL aliases
        assert_eq!("abl".parse::<Language>().unwrap(), Language::ABL);
        assert_eq!("openedge".parse::<Language>().unwrap(), Language::ABL);
        assert_eq!("progress".parse::<Language>().unwrap(), Language::ABL);

        // Python aliases
        assert_eq!("python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("py".parse::<Language>().unwrap(), Language::Python);

        // TypeScript aliases
        assert_eq!(
            "typescript".parse::<Language>().unwrap(),
            Language::TypeScript
        );
        assert_eq!("ts".parse::<Language>().unwrap(), Language::TypeScript);

        // JavaScript aliases
        assert_eq!(
            "javascript".parse::<Language>().unwrap(),
            Language::JavaScript
        );
        assert_eq!("js".parse::<Language>().unwrap(), Language::JavaScript);

        // Shell aliases
        assert_eq!("shell".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("bash".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("sh".parse::<Language>().unwrap(), Language::Shell);

        // Rust aliases
        assert_eq!("rust".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("rs".parse::<Language>().unwrap(), Language::Rust);

        // Go aliases
        assert_eq!("go".parse::<Language>().unwrap(), Language::Go);
        assert_eq!("golang".parse::<Language>().unwrap(), Language::Go);

        // Java
        assert_eq!("java".parse::<Language>().unwrap(), Language::Java);

        // CSharp aliases
        assert_eq!("csharp".parse::<Language>().unwrap(), Language::CSharp);
        assert_eq!("c#".parse::<Language>().unwrap(), Language::CSharp);
        assert_eq!("cs".parse::<Language>().unwrap(), Language::CSharp);

        // Ruby aliases
        assert_eq!("ruby".parse::<Language>().unwrap(), Language::Ruby);
        assert_eq!("rb".parse::<Language>().unwrap(), Language::Ruby);
    }

    #[test]
    fn test_language_from_str_case_insensitive() {
        assert_eq!("PYTHON".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("Python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("RUST".parse::<Language>().unwrap(), Language::Rust);
    }

    #[test]
    fn test_language_from_str_error() {
        let result = "unknown_language".parse::<Language>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown language"));
    }

    #[test]
    fn test_language_clone() {
        let lang = Language::Python;
        let cloned = lang.clone();
        assert_eq!(lang, cloned);
    }

    #[test]
    fn test_language_copy() {
        let lang = Language::Rust;
        let copied: Language = lang;
        assert_eq!(lang, copied);
    }

    #[test]
    fn test_language_ordering() {
        // PartialOrd/Ord should work
        assert!(Language::ABL < Language::Python);
        let mut langs = vec![Language::Python, Language::ABL, Language::Rust];
        langs.sort();
        assert_eq!(langs[0], Language::ABL);
    }

    #[test]
    fn test_language_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Language::Python);
        set.insert(Language::Python);
        assert_eq!(set.len(), 1);
    }

    // =========================================================================
    // UserContext Comprehensive Tests
    // =========================================================================

    #[test]
    fn test_user_context_default() {
        let ctx = UserContext::default();
        assert!(ctx.language_familiarity.is_empty());
        assert!(ctx.ignore_languages.is_empty());
        assert!(ctx.depth_preferences.is_empty());
        assert!(ctx.time_budget_minutes.is_none());
    }

    #[test]
    fn test_user_context_new() {
        let ctx = UserContext::new();
        assert!(ctx.language_familiarity.is_empty());
    }

    #[test]
    fn test_user_context_familiarity_clamping() {
        let ctx = UserContext::new()
            .with_familiarity(Language::Python, 1.5) // should clamp to 1.0
            .with_familiarity(Language::Rust, -0.5); // should clamp to 0.0

        assert_eq!(ctx.get_familiarity(Language::Python), 1.0);
        assert_eq!(ctx.get_familiarity(Language::Rust), 0.0);
    }

    #[test]
    fn test_user_context_ignoring_no_duplicates() {
        let ctx = UserContext::new()
            .ignoring(Language::Shell)
            .ignoring(Language::Shell); // duplicate

        assert_eq!(ctx.ignore_languages.len(), 1);
    }

    #[test]
    fn test_user_context_get_familiarity_default() {
        let ctx = UserContext::new();
        assert_eq!(ctx.get_familiarity(Language::Unknown), 0.5);
    }

    #[test]
    fn test_user_context_should_ignore_false() {
        let ctx = UserContext::new();
        assert!(!ctx.should_ignore(Language::Python));
    }

    #[test]
    fn test_user_context_builder_chain() {
        let ctx = UserContext::new()
            .with_familiarity(Language::Python, 0.8)
            .with_familiarity(Language::Rust, 0.9)
            .ignoring(Language::ABL)
            .ignoring(Language::Go)
            .with_time_budget(120);

        assert_eq!(ctx.get_familiarity(Language::Python), 0.8);
        assert_eq!(ctx.get_familiarity(Language::Rust), 0.9);
        assert!(ctx.should_ignore(Language::ABL));
        assert!(ctx.should_ignore(Language::Go));
        assert_eq!(ctx.time_budget_minutes, Some(120));
    }

    #[test]
    fn test_user_context_override_familiarity() {
        let ctx = UserContext::new()
            .with_familiarity(Language::Python, 0.5)
            .with_familiarity(Language::Python, 0.9);

        assert_eq!(ctx.get_familiarity(Language::Python), 0.9);
    }

    #[test]
    fn test_user_context_clone() {
        let ctx = UserContext::new()
            .with_familiarity(Language::Python, 0.8)
            .ignoring(Language::Shell);
        let cloned = ctx.clone();

        assert_eq!(cloned.get_familiarity(Language::Python), 0.8);
        assert!(cloned.should_ignore(Language::Shell));
    }

    #[test]
    fn test_user_context_debug() {
        let ctx = UserContext::new().with_familiarity(Language::Python, 0.5);
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("UserContext"));
    }
}
