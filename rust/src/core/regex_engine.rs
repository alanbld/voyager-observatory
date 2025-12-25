//! Centralized Regex Engine Bridge
//!
//! Provides a unified, FFI-friendly wrapper around the Rust `regex` crate.
//! Designed for easy integration with Lua scripting and other foreign interfaces.
//!
//! # Design Goals
//!
//! 1. **FFI-Friendly**: Simple signatures, no complex lifetimes, owned types
//! 2. **Performance**: Lazy compilation with caching for repeated patterns
//! 3. **Safety**: All operations return Results with clear error types
//! 4. **Thread-Safe**: All types are Send + Sync for MCP server usage
//!
//! # Example
//!
//! ```rust
//! use pm_encoder::core::regex_engine::{RegexEngine, MatchRange};
//!
//! let engine = RegexEngine::new();
//!
//! // Compile a pattern (cached)
//! let regex = engine.compile(r"(\w+)@(\w+)\.com").unwrap();
//!
//! // Find all matches
//! let text = "Contact: alice@example.com and bob@test.com";
//! let matches = engine.find_iter(&regex, text);
//!
//! // Get named captures
//! let regex2 = engine.compile(r"(?P<user>\w+)@(?P<domain>\w+)\.com").unwrap();
//! if let Some(caps) = engine.match_captures(&regex2, text) {
//!     assert_eq!(caps.get("user"), Some(&"alice".to_string()));
//! }
//! ```

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use regex::{Captures, Regex};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during regex operations.
#[derive(Debug, Clone)]
pub struct RegexError {
    /// The pattern that caused the error
    pub pattern: String,
    /// Human-readable error description
    pub message: String,
}

impl RegexError {
    pub fn new(pattern: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RegexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regex error for '{}': {}", self.pattern, self.message)
    }
}

impl std::error::Error for RegexError {}

// =============================================================================
// Core Types
// =============================================================================

/// A compiled regular expression.
///
/// This is an opaque handle type designed for FFI safety.
/// Internally wraps `regex::Regex` with Arc for cheap cloning.
#[derive(Clone)]
pub struct CompiledRegex {
    inner: Arc<Regex>,
    /// Original pattern string for debugging/serialization
    pattern: String,
}

impl CompiledRegex {
    /// Get the original pattern string.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Check if the pattern matches anywhere in the text.
    pub fn is_match(&self, text: &str) -> bool {
        self.inner.is_match(text)
    }

    // -------------------------------------------------------------------------
    // Convenience methods for internal plugin use
    // Note: These return iterators with lifetimes, not FFI-safe
    // -------------------------------------------------------------------------

    /// Get an iterator over all captures.
    ///
    /// This is a convenience method for internal plugin use.
    /// For FFI-safe usage, use `RegexEngine::captures_iter()`.
    pub fn captures_iter<'r, 't>(&'r self, text: &'t str) -> regex::CaptureMatches<'r, 't> {
        self.inner.captures_iter(text)
    }

    /// Get the first capture.
    pub fn captures<'t>(&self, text: &'t str) -> Option<regex::Captures<'t>> {
        self.inner.captures(text)
    }

    /// Find all matches.
    pub fn find_iter<'r, 't>(&'r self, text: &'t str) -> regex::Matches<'r, 't> {
        self.inner.find_iter(text)
    }

    /// Find the first match.
    pub fn find<'t>(&self, text: &'t str) -> Option<regex::Match<'t>> {
        self.inner.find(text)
    }

    /// Replace all occurrences.
    pub fn replace_all<'t>(&self, text: &'t str, rep: &str) -> std::borrow::Cow<'t, str> {
        self.inner.replace_all(text, rep)
    }

    /// Get capture names.
    pub fn capture_names(&self) -> regex::CaptureNames<'_> {
        self.inner.capture_names()
    }
}

impl std::fmt::Debug for CompiledRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledRegex")
            .field("pattern", &self.pattern)
            .finish()
    }
}

/// A match range within text.
///
/// Simple struct for FFI compatibility - no lifetimes, just indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
}

impl MatchRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Length of the match in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the match is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Extract the matched text from the original string.
    pub fn extract<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

/// A complete match with capture groups.
///
/// FFI-friendly structure containing all capture information.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The full match range
    pub range: MatchRange,
    /// Named captures (if any)
    pub named: BTreeMap<String, String>,
    /// Indexed captures (group 0 = full match)
    pub indexed: Vec<Option<String>>,
}

impl MatchResult {
    /// Get a named capture group.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.named.get(name).map(|s| s.as_str())
    }

    /// Get an indexed capture group.
    pub fn group(&self, index: usize) -> Option<&str> {
        self.indexed.get(index).and_then(|o| o.as_deref())
    }
}

// =============================================================================
// Regex Engine
// =============================================================================

/// Thread-safe regex engine with compilation cache.
///
/// The engine maintains a cache of compiled patterns to avoid
/// recompilation overhead for frequently-used patterns.
pub struct RegexEngine {
    cache: RwLock<BTreeMap<String, CompiledRegex>>,
    /// Maximum number of cached patterns (0 = unlimited)
    max_cache_size: usize,
}

impl Default for RegexEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexEngine {
    /// Create a new regex engine with default settings.
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(BTreeMap::new()),
            max_cache_size: 1000,
        }
    }

    /// Create a regex engine with a specific cache size limit.
    pub fn with_cache_size(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(BTreeMap::new()),
            max_cache_size: max_size,
        }
    }

    /// Compile a regex pattern.
    ///
    /// Patterns are cached, so repeated compilation of the same pattern
    /// is cheap (just an Arc clone).
    ///
    /// # FFI Safety
    ///
    /// Returns owned types only - no lifetimes to worry about.
    pub fn compile(&self, pattern: &str) -> Result<CompiledRegex, RegexError> {
        // Check cache first (read lock)
        {
            let cache = self.cache.read().unwrap();
            if let Some(compiled) = cache.get(pattern) {
                return Ok(compiled.clone());
            }
        }

        // Compile new regex
        let regex = Regex::new(pattern).map_err(|e| RegexError::new(pattern, e.to_string()))?;

        let compiled = CompiledRegex {
            inner: Arc::new(regex),
            pattern: pattern.to_string(),
        };

        // Insert into cache (write lock)
        {
            let mut cache = self.cache.write().unwrap();

            // Evict if at capacity
            if self.max_cache_size > 0 && cache.len() >= self.max_cache_size {
                // Simple eviction: remove first entry (oldest by insertion order in BTreeMap)
                if let Some(key) = cache.keys().next().cloned() {
                    cache.remove(&key);
                }
            }

            cache.insert(pattern.to_string(), compiled.clone());
        }

        Ok(compiled)
    }

    /// Compile without caching.
    ///
    /// Useful for one-off patterns or when cache pollution is a concern.
    pub fn compile_uncached(&self, pattern: &str) -> Result<CompiledRegex, RegexError> {
        let regex = Regex::new(pattern).map_err(|e| RegexError::new(pattern, e.to_string()))?;

        Ok(CompiledRegex {
            inner: Arc::new(regex),
            pattern: pattern.to_string(),
        })
    }

    /// Check if a pattern matches anywhere in the text.
    pub fn is_match(&self, regex: &CompiledRegex, text: &str) -> bool {
        regex.inner.is_match(text)
    }

    /// Find all non-overlapping matches.
    ///
    /// # FFI Safety
    ///
    /// Returns a Vec of simple structs with no lifetimes.
    pub fn find_iter(&self, regex: &CompiledRegex, text: &str) -> Vec<MatchRange> {
        regex
            .inner
            .find_iter(text)
            .map(|m| MatchRange::new(m.start(), m.end()))
            .collect()
    }

    /// Find the first match.
    pub fn find(&self, regex: &CompiledRegex, text: &str) -> Option<MatchRange> {
        regex
            .inner
            .find(text)
            .map(|m| MatchRange::new(m.start(), m.end()))
    }

    /// Get named captures from the first match.
    ///
    /// # FFI Safety
    ///
    /// Returns a BTreeMap of owned Strings - no lifetimes.
    pub fn match_captures(
        &self,
        regex: &CompiledRegex,
        text: &str,
    ) -> Option<BTreeMap<String, String>> {
        let caps = regex.inner.captures(text)?;
        Some(self.extract_named_captures(&regex.inner, &caps))
    }

    /// Get complete match result with all captures.
    pub fn match_full(&self, regex: &CompiledRegex, text: &str) -> Option<MatchResult> {
        let caps = regex.inner.captures(text)?;
        let full_match = caps.get(0)?;

        Some(MatchResult {
            range: MatchRange::new(full_match.start(), full_match.end()),
            named: self.extract_named_captures(&regex.inner, &caps),
            indexed: self.extract_indexed_captures(&caps),
        })
    }

    /// Get all matches with their captures.
    pub fn captures_iter(&self, regex: &CompiledRegex, text: &str) -> Vec<MatchResult> {
        regex
            .inner
            .captures_iter(text)
            .filter_map(|caps| {
                let full_match = caps.get(0)?;
                Some(MatchResult {
                    range: MatchRange::new(full_match.start(), full_match.end()),
                    named: self.extract_named_captures(&regex.inner, &caps),
                    indexed: self.extract_indexed_captures(&caps),
                })
            })
            .collect()
    }

    /// Replace all matches with a replacement string.
    ///
    /// The replacement string can use `$1`, `$2`, etc. for capture groups,
    /// or `$name` for named captures.
    ///
    /// # FFI Safety
    ///
    /// Returns an owned String.
    pub fn replace_all(&self, regex: &CompiledRegex, text: &str, replacement: &str) -> String {
        regex.inner.replace_all(text, replacement).into_owned()
    }

    /// Replace the first match.
    pub fn replace(&self, regex: &CompiledRegex, text: &str, replacement: &str) -> String {
        regex.inner.replace(text, replacement).into_owned()
    }

    /// Split text by a regex pattern.
    pub fn split(&self, regex: &CompiledRegex, text: &str) -> Vec<String> {
        regex.inner.split(text).map(|s| s.to_string()).collect()
    }

    /// Count the number of matches.
    pub fn count(&self, regex: &CompiledRegex, text: &str) -> usize {
        regex.inner.find_iter(text).count()
    }

    /// Clear the compilation cache.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Get current cache size.
    pub fn cache_size(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }

    // -------------------------------------------------------------------------
    // Internal Helpers
    // -------------------------------------------------------------------------

    fn extract_named_captures(&self, regex: &Regex, caps: &Captures) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();

        for name in regex.capture_names().flatten() {
            if let Some(m) = caps.name(name) {
                result.insert(name.to_string(), m.as_str().to_string());
            }
        }

        result
    }

    fn extract_indexed_captures(&self, caps: &Captures) -> Vec<Option<String>> {
        caps.iter()
            .map(|m| m.map(|m| m.as_str().to_string()))
            .collect()
    }
}

// =============================================================================
// Convenience Functions (FFI Entry Points)
// =============================================================================

/// Global engine instance for simple usage.
///
/// Thread-safe via internal locking.
static GLOBAL_ENGINE: std::sync::OnceLock<RegexEngine> = std::sync::OnceLock::new();

/// Get the global regex engine.
pub fn global_engine() -> &'static RegexEngine {
    GLOBAL_ENGINE.get_or_init(RegexEngine::new)
}

/// Compile a pattern using the global engine.
pub fn compile(pattern: &str) -> Result<CompiledRegex, RegexError> {
    global_engine().compile(pattern)
}

/// Quick match check using the global engine.
pub fn is_match(pattern: &str, text: &str) -> Result<bool, RegexError> {
    let regex = global_engine().compile(pattern)?;
    Ok(regex.is_match(text))
}

/// Quick find_iter using the global engine.
pub fn find_all(pattern: &str, text: &str) -> Result<Vec<MatchRange>, RegexError> {
    let regex = global_engine().compile(pattern)?;
    Ok(global_engine().find_iter(&regex, text))
}

/// Quick replace_all using the global engine.
pub fn replace_all(pattern: &str, text: &str, replacement: &str) -> Result<String, RegexError> {
    let regex = global_engine().compile(pattern)?;
    Ok(global_engine().replace_all(&regex, text, replacement))
}

// =============================================================================
// Precompiled Pattern Sets (For Plugin Use)
// =============================================================================

/// A set of precompiled patterns for efficient batch matching.
///
/// Useful for plugins that need to match many patterns against the same text.
#[derive(Clone)]
pub struct PatternSet {
    patterns: Vec<(String, CompiledRegex)>,
}

impl PatternSet {
    /// Create a new empty pattern set.
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Add a named pattern to the set.
    pub fn add(&mut self, name: &str, pattern: &str) -> Result<(), RegexError> {
        let compiled = global_engine().compile(pattern)?;
        self.patterns.push((name.to_string(), compiled));
        Ok(())
    }

    /// Check which patterns match the text.
    pub fn match_all(&self, text: &str) -> Vec<&str> {
        self.patterns
            .iter()
            .filter(|(_, regex)| regex.is_match(text))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get the first matching pattern name.
    pub fn first_match(&self, text: &str) -> Option<&str> {
        self.patterns
            .iter()
            .find(|(_, regex)| regex.is_match(text))
            .map(|(name, _)| name.as_str())
    }

    /// Count matches for each pattern.
    pub fn count_matches(&self, text: &str) -> BTreeMap<String, usize> {
        self.patterns
            .iter()
            .map(|(name, regex)| (name.clone(), global_engine().count(regex, text)))
            .collect()
    }
}

impl Default for PatternSet {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_compile() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"\d+").unwrap();
        assert!(engine.is_match(&regex, "abc123"));
        assert!(!engine.is_match(&regex, "abcdef"));
    }

    #[test]
    fn test_invalid_pattern() {
        let engine = RegexEngine::new();
        let result = engine.compile(r"[invalid");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.pattern.contains("invalid"));
    }

    #[test]
    fn test_cache_hit() {
        let engine = RegexEngine::new();

        // First compile
        let _r1 = engine.compile(r"\d+").unwrap();
        assert_eq!(engine.cache_size(), 1);

        // Second compile - should hit cache
        let _r2 = engine.compile(r"\d+").unwrap();
        assert_eq!(engine.cache_size(), 1); // Still 1
    }

    #[test]
    fn test_find_iter() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"\d+").unwrap();

        let matches = engine.find_iter(&regex, "a1b23c456");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0], MatchRange::new(1, 2));
        assert_eq!(matches[1], MatchRange::new(3, 5));
        assert_eq!(matches[2], MatchRange::new(6, 9));
    }

    #[test]
    fn test_match_range_extract() {
        let text = "hello world";
        let range = MatchRange::new(6, 11);
        assert_eq!(range.extract(text), "world");
        assert_eq!(range.len(), 5);
    }

    #[test]
    fn test_named_captures() {
        let engine = RegexEngine::new();
        let regex = engine
            .compile(r"(?P<first>\w+)\s+(?P<last>\w+)")
            .unwrap();

        let caps = engine.match_captures(&regex, "John Doe").unwrap();
        assert_eq!(caps.get("first"), Some(&"John".to_string()));
        assert_eq!(caps.get("last"), Some(&"Doe".to_string()));
    }

    #[test]
    fn test_indexed_captures() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"(\w+)@(\w+)\.(\w+)").unwrap();

        let result = engine.match_full(&regex, "user@example.com").unwrap();
        assert_eq!(result.group(0), Some("user@example.com"));
        assert_eq!(result.group(1), Some("user"));
        assert_eq!(result.group(2), Some("example"));
        assert_eq!(result.group(3), Some("com"));
    }

    #[test]
    fn test_replace_all() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"\d+").unwrap();

        let result = engine.replace_all(&regex, "a1b2c3", "X");
        assert_eq!(result, "aXbXcX");
    }

    #[test]
    fn test_replace_with_captures() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"(\w+)@(\w+)").unwrap();

        let result = engine.replace_all(&regex, "user@domain", "[$1 at $2]");
        assert_eq!(result, "[user at domain]");
    }

    #[test]
    fn test_split() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"\s+").unwrap();

        let parts = engine.split(&regex, "a b  c   d");
        assert_eq!(parts, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_count() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"\d").unwrap();

        assert_eq!(engine.count(&regex, "a1b2c3d4e5"), 5);
    }

    #[test]
    fn test_captures_iter() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"(?P<word>\w+)").unwrap();

        let results = engine.captures_iter(&regex, "hello world");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get("word"), Some("hello"));
        assert_eq!(results[1].get("word"), Some("world"));
    }

    #[test]
    fn test_global_engine() {
        // Ensure global engine works
        let regex = compile(r"\w+").unwrap();
        assert!(is_match(r"\d+", "test123").unwrap());
        assert_eq!(
            find_all(r"\d+", "a1b2c3").unwrap(),
            vec![
                MatchRange::new(1, 2),
                MatchRange::new(3, 4),
                MatchRange::new(5, 6)
            ]
        );
        assert_eq!(replace_all(r"\d", "a1b2", "X").unwrap(), "aXbX");

        // Verify it's the same global instance
        let size = global_engine().cache_size();
        assert!(size > 0);

        drop(regex);
    }

    #[test]
    fn test_pattern_set() {
        let mut set = PatternSet::new();
        set.add("digits", r"\d+").unwrap();
        set.add("words", r"\w+").unwrap();
        set.add("email", r"\w+@\w+\.\w+").unwrap();

        let text = "Contact: user@example.com or call 555-1234";

        let matches = set.match_all(text);
        assert!(matches.contains(&"digits"));
        assert!(matches.contains(&"words"));
        assert!(matches.contains(&"email"));

        let counts = set.count_matches(text);
        assert!(counts.get("digits").unwrap() > &0);
    }

    #[test]
    fn test_cache_eviction() {
        let engine = RegexEngine::with_cache_size(3);

        engine.compile(r"a").unwrap();
        engine.compile(r"b").unwrap();
        engine.compile(r"c").unwrap();
        assert_eq!(engine.cache_size(), 3);

        // Adding a 4th should evict one
        engine.compile(r"d").unwrap();
        assert_eq!(engine.cache_size(), 3);
    }

    #[test]
    fn test_uncached_compile() {
        let engine = RegexEngine::new();

        let regex = engine.compile_uncached(r"test").unwrap();
        assert!(engine.is_match(&regex, "testing"));
        assert_eq!(engine.cache_size(), 0); // Not cached
    }

    #[test]
    fn test_clear_cache() {
        let engine = RegexEngine::new();

        engine.compile(r"a").unwrap();
        engine.compile(r"b").unwrap();
        assert_eq!(engine.cache_size(), 2);

        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let engine = std::sync::Arc::new(RegexEngine::new());
        let mut handles = vec![];

        for i in 0..10 {
            let engine = engine.clone();
            handles.push(thread::spawn(move || {
                let pattern = format!(r"test{}", i);
                let regex = engine.compile(&pattern).unwrap();
                engine.is_match(&regex, &format!("test{}", i))
            }));
        }

        for handle in handles {
            assert!(handle.join().unwrap());
        }
    }

    // -------------------------------------------------------------------------
    // Performance sanity tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_cached_compile_is_fast() {
        let engine = RegexEngine::new();

        // First compile (cold)
        let _ = engine.compile(r"(?P<name>\w+)@(?P<domain>\w+)\.com");

        // Subsequent compiles should be instant (cache hit)
        let start = std::time::Instant::now();
        for _ in 0..10000 {
            let _ = engine.compile(r"(?P<name>\w+)@(?P<domain>\w+)\.com");
        }
        let elapsed = start.elapsed();

        // 10000 cache hits should be well under 100ms
        assert!(
            elapsed.as_millis() < 100,
            "Cache hits too slow: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_find_iter_performance() {
        let engine = RegexEngine::new();
        let regex = engine.compile(r"\w+").unwrap();

        // Generate some text
        let text = "word ".repeat(1000);

        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = engine.find_iter(&regex, &text);
        }
        let elapsed = start.elapsed();

        // 100 iterations finding 1000 words each should be fast
        assert!(
            elapsed.as_millis() < 500,
            "find_iter too slow: {:?}",
            elapsed
        );
    }
}
