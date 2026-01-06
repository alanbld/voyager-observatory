//! Types for Skeleton Protocol v2.2
//!
//! Defines compression levels, language detection, and result structures.

use crate::core::FileTier;

/// Compression level for file content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum CompressionLevel {
    /// L0: Full content preserved
    #[default]
    Full,
    /// L2: Signatures only (bodies stripped)
    Skeleton,
    /// L3: File excluded from output
    Drop,
}


/// Supported programming languages for skeletonization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
}

impl Language {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "go" => Some(Language::Go),
            _ => None,
        }
    }

    /// Check if language uses brace-based blocks
    pub fn uses_braces(&self) -> bool {
        matches!(
            self,
            Language::Rust | Language::TypeScript | Language::JavaScript | Language::Go
        )
    }

    /// Check if language uses indentation-based blocks
    pub fn uses_indentation(&self) -> bool {
        matches!(self, Language::Python)
    }
}

/// Result of skeletonizing a file
#[derive(Debug, Clone)]
pub struct SkeletonResult {
    /// The skeletonized content
    pub content: String,
    /// Original token count (estimated)
    pub original_tokens: usize,
    /// Skeleton token count (estimated)
    pub skeleton_tokens: usize,
    /// Compression ratio (0.0 to 1.0, higher = more compression)
    pub compression_ratio: f32,
    /// List of preserved symbol names
    pub preserved_symbols: Vec<String>,
}

impl Default for SkeletonResult {
    fn default() -> Self {
        Self {
            content: String::new(),
            original_tokens: 0,
            skeleton_tokens: 0,
            compression_ratio: 0.0,
            preserved_symbols: Vec::new(),
        }
    }
}

impl SkeletonResult {
    /// Create a new skeleton result
    pub fn new(
        content: String,
        original_tokens: usize,
        skeleton_tokens: usize,
        preserved_symbols: Vec<String>,
    ) -> Self {
        let compression_ratio = if original_tokens > 0 {
            1.0 - (skeleton_tokens as f32 / original_tokens as f32)
        } else {
            0.0
        };

        Self {
            content,
            original_tokens,
            skeleton_tokens,
            compression_ratio,
            preserved_symbols,
        }
    }
}

/// File allocation result from the adaptive allocator
#[derive(Debug, Clone)]
pub struct FileAllocation {
    /// File path
    pub path: String,
    /// File tier (Core, Config, Tests, Other)
    pub tier: FileTier,
    /// Full content token cost
    pub full_tokens: usize,
    /// Skeleton content token cost
    pub skeleton_tokens: usize,
    /// Assigned compression level
    pub level: CompressionLevel,
}

impl FileAllocation {
    /// Create a new file allocation
    pub fn new(path: &str, tier: FileTier, full_tokens: usize, skeleton_tokens: usize) -> Self {
        Self {
            path: path.to_string(),
            tier,
            full_tokens,
            skeleton_tokens,
            level: CompressionLevel::Skeleton, // Default to skeleton
        }
    }

    /// Get the token cost for the current compression level
    pub fn current_tokens(&self) -> usize {
        match self.level {
            CompressionLevel::Full => self.full_tokens,
            CompressionLevel::Skeleton => self.skeleton_tokens,
            CompressionLevel::Drop => 0,
        }
    }

    /// Calculate upgrade cost (skeleton -> full)
    pub fn upgrade_cost(&self) -> usize {
        if self.level == CompressionLevel::Skeleton {
            self.full_tokens - self.skeleton_tokens
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("txt"), None);
        assert_eq!(Language::from_extension("RS"), Some(Language::Rust)); // case insensitive
    }

    #[test]
    fn test_compression_ratio_calculation() {
        let result = SkeletonResult::new(
            "fn main();".to_string(),
            100,
            10,
            vec!["main".to_string()],
        );
        assert_eq!(result.compression_ratio, 0.9);
    }

    #[test]
    fn test_file_allocation_tokens() {
        let mut alloc = FileAllocation::new("test.rs", FileTier::Core, 100, 10);

        assert_eq!(alloc.current_tokens(), 10); // Default is Skeleton
        assert_eq!(alloc.upgrade_cost(), 90);

        alloc.level = CompressionLevel::Full;
        assert_eq!(alloc.current_tokens(), 100);
        assert_eq!(alloc.upgrade_cost(), 0);

        alloc.level = CompressionLevel::Drop;
        assert_eq!(alloc.current_tokens(), 0);
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_language_uses_braces() {
        assert!(Language::Rust.uses_braces());
        assert!(Language::TypeScript.uses_braces());
        assert!(Language::JavaScript.uses_braces());
        assert!(Language::Go.uses_braces());
        assert!(!Language::Python.uses_braces());
    }

    #[test]
    fn test_language_uses_indentation() {
        assert!(Language::Python.uses_indentation());
        assert!(!Language::Rust.uses_indentation());
        assert!(!Language::TypeScript.uses_indentation());
        assert!(!Language::JavaScript.uses_indentation());
        assert!(!Language::Go.uses_indentation());
    }

    #[test]
    fn test_compression_ratio_zero_original() {
        // When original_tokens is 0, compression_ratio should be 0.0
        let result = SkeletonResult::new(
            "".to_string(),
            0,  // original_tokens = 0
            0,
            vec![],
        );
        assert_eq!(result.compression_ratio, 0.0);
    }

    #[test]
    fn test_skeleton_result_default() {
        let result = SkeletonResult::default();
        assert!(result.content.is_empty());
        assert_eq!(result.original_tokens, 0);
        assert_eq!(result.skeleton_tokens, 0);
        assert_eq!(result.compression_ratio, 0.0);
        assert!(result.preserved_symbols.is_empty());
    }

    #[test]
    fn test_compression_level_default() {
        let level: CompressionLevel = Default::default();
        assert_eq!(level, CompressionLevel::Full);
    }

    #[test]
    fn test_language_from_extension_all_js_variants() {
        assert_eq!(Language::from_extension("jsx"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("mjs"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("cjs"), Some(Language::JavaScript));
    }

    #[test]
    fn test_file_allocation_upgrade_cost_non_skeleton() {
        let mut alloc = FileAllocation::new("test.rs", FileTier::Core, 100, 10);

        // Full level has 0 upgrade cost
        alloc.level = CompressionLevel::Full;
        assert_eq!(alloc.upgrade_cost(), 0);

        // Drop level also has 0 upgrade cost
        alloc.level = CompressionLevel::Drop;
        assert_eq!(alloc.upgrade_cost(), 0);
    }

    #[test]
    fn test_compression_level_equality() {
        assert_eq!(CompressionLevel::Full, CompressionLevel::Full);
        assert_eq!(CompressionLevel::Skeleton, CompressionLevel::Skeleton);
        assert_eq!(CompressionLevel::Drop, CompressionLevel::Drop);
        assert_ne!(CompressionLevel::Full, CompressionLevel::Skeleton);
    }

    #[test]
    fn test_language_equality() {
        assert_eq!(Language::Rust, Language::Rust);
        assert_eq!(Language::Python, Language::Python);
        assert_ne!(Language::Rust, Language::Python);
    }

    #[test]
    fn test_skeleton_result_with_symbols() {
        let result = SkeletonResult::new(
            "fn foo(); fn bar();".to_string(),
            200,
            20,
            vec!["foo".to_string(), "bar".to_string()],
        );
        assert_eq!(result.preserved_symbols.len(), 2);
        assert!(result.preserved_symbols.contains(&"foo".to_string()));
        assert!(result.preserved_symbols.contains(&"bar".to_string()));
    }

    #[test]
    fn test_file_allocation_fields() {
        let alloc = FileAllocation::new("src/lib.rs", FileTier::Core, 500, 50);
        assert_eq!(alloc.path, "src/lib.rs");
        assert_eq!(alloc.tier, FileTier::Core);
        assert_eq!(alloc.full_tokens, 500);
        assert_eq!(alloc.skeleton_tokens, 50);
        assert_eq!(alloc.level, CompressionLevel::Skeleton); // default
    }
}
