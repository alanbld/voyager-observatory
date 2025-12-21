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
}
