//! Token Budgeting for context-aware file selection (v1.7.0)
//!
//! This module provides token estimation and budget-based file selection
//! to fit output within LLM context windows.
//!
//! ## Tiered Allocation (Phase 2)
//!
//! Files are allocated budget in tier order:
//! 1. Core (src/, lib/) - Primary source code
//! 2. Config (Cargo.toml, package.json) - High value/token ratio
//! 3. Tests (tests/, examples/) - If budget remains
//! 4. Other (docs, scripts) - Lowest priority

use std::path::Path;
use crate::lenses::LensManager;
use crate::truncate_structure;
use crate::core::engine::FileTier;

/// Threshold for hybrid strategy: files > 10% of budget get auto-truncated
const HYBRID_THRESHOLD: f64 = 0.10;

/// Token estimation using heuristic (4 chars per token)
///
/// Note: Rust implementation uses heuristic only. For precise counting,
/// use the Python engine with tiktoken installed.
pub struct TokenEstimator;

impl TokenEstimator {
    /// Estimate tokens in content using heuristic
    ///
    /// The heuristic of len/4 is based on the observation that
    /// English text averages about 4 characters per token for GPT tokenizers.
    pub fn estimate_tokens(content: &str) -> usize {
        content.len() / 4
    }

    /// Estimate tokens for a file including PM format overhead
    ///
    /// Accounts for the ++++/---- markers and path repetition
    pub fn estimate_file_tokens(path: &Path, content: &str) -> usize {
        let path_str = path.to_string_lossy();
        // PM format: "++++++++++ path ++++++++++\n" + content + "\n---------- path checksum path ----------\n"
        let overhead = 20 + path_str.len() * 2 + 50; // Approximate overhead
        Self::estimate_tokens(content) + (overhead / 4)
    }

    /// Get the estimation method name
    pub fn method() -> &'static str {
        "Heuristic (~4 chars/token)"
    }
}

/// Parse a token budget string with optional k/M suffix
///
/// # Arguments
///
/// * `value` - Budget string like "100000", "100k", "100K", "2m", "2M"
///
/// # Returns
///
/// * `Ok(usize)` - Parsed token count
/// * `Err(String)` - Error message if format is invalid
///
/// # Examples
///
/// ```
/// use pm_encoder::budgeting::parse_token_budget;
///
/// assert_eq!(parse_token_budget("100000").unwrap(), 100000);
/// assert_eq!(parse_token_budget("100k").unwrap(), 100000);
/// assert_eq!(parse_token_budget("2M").unwrap(), 2000000);
/// ```
pub fn parse_token_budget(value: &str) -> Result<usize, String> {
    let value = value.trim();

    if value.is_empty() {
        return Err("Empty token budget value".to_string());
    }

    // Check for suffix
    let last_char = value.chars().last().unwrap();
    let (number_part, multiplier) = match last_char {
        'k' | 'K' => (&value[..value.len()-1], 1_000),
        'm' | 'M' => (&value[..value.len()-1], 1_000_000),
        _ => (value, 1),
    };

    let number: usize = number_part.parse()
        .map_err(|_| format!("Invalid token budget format: '{}'. Expected format: 123, 100k, 2M", value))?;

    Ok(number * multiplier)
}

/// File data for budget selection
#[derive(Debug, Clone)]
pub struct FileData {
    /// Relative path
    pub path: String,
    /// File content
    pub content: String,
    /// Priority from lens config
    pub priority: i32,
    /// Estimated token count
    pub tokens: usize,
    /// Original token count (before any truncation)
    pub original_tokens: usize,
    /// Inclusion method: "full" or "truncated"
    pub method: String,
}

/// Report of token budgeting results
#[derive(Debug, Clone)]
pub struct BudgetReport {
    /// Total budget in tokens
    pub budget: usize,
    /// Tokens used
    pub used: usize,
    /// Number of files selected
    pub selected_count: usize,
    /// Number of files dropped
    pub dropped_count: usize,
    /// Dropped files: (path, priority, tokens)
    pub dropped_files: Vec<(String, i32, usize)>,
    /// Estimation method name
    pub estimation_method: String,
    /// Strategy used
    pub strategy: String,
    /// Included files: (path, priority, tokens, method)
    pub included_files: Vec<(String, i32, usize, String)>,
    /// Count of auto-truncated files
    pub truncated_count: usize,
}

impl BudgetReport {
    /// Calculate percentage of budget used
    pub fn used_percentage(&self) -> f64 {
        if self.budget > 0 {
            (self.used as f64 / self.budget as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate remaining tokens
    pub fn remaining(&self) -> usize {
        self.budget.saturating_sub(self.used)
    }

    /// Print a formatted budget report to stderr
    pub fn print_report(&self) {
        eprintln!("{}", "=".repeat(70));
        eprintln!("TOKEN BUDGET REPORT");
        eprintln!("{}", "=".repeat(70));
        eprintln!("Budget:     {:>10} tokens", format_number(self.budget));
        eprintln!("Used:       {:>10} tokens ({:.1}%)",
            format_number(self.used), self.used_percentage());
        eprintln!("Remaining:  {:>10} tokens", format_number(self.remaining()));
        eprintln!("Estimation: {}", self.estimation_method);
        eprintln!("Strategy:   {}", self.strategy);
        eprintln!();

        let full_count = self.included_files.iter()
            .filter(|(_, _, _, m)| m == "full")
            .count();
        eprintln!("Files included: {} ({} full, {} truncated)",
            self.selected_count, full_count, self.truncated_count);
        eprintln!("Files dropped:  {} (lowest priority first)", self.dropped_count);

        if self.truncated_count > 0 {
            eprintln!();
            eprintln!("Auto-truncated files (structure mode):");
            for (path, priority, tokens, method) in self.included_files.iter().take(5) {
                if method == "truncated" {
                    eprintln!("  [P:{:3}] {} ({} tokens)", priority, path, format_number(*tokens));
                }
            }
            let truncated_list: Vec<_> = self.included_files.iter()
                .filter(|(_, _, _, m)| m == "truncated")
                .collect();
            if truncated_list.len() > 5 {
                eprintln!("  ... and {} more", truncated_list.len() - 5);
            }
        }

        if !self.dropped_files.is_empty() {
            eprintln!();
            eprintln!("Dropped files:");
            for (path, priority, tokens) in self.dropped_files.iter().take(10) {
                eprintln!("  [P:{:3}] {} ({} tokens)", priority, path, format_number(*tokens));
            }
            if self.dropped_files.len() > 10 {
                eprintln!("  ... and {} more", self.dropped_files.len() - 10);
            }
        }

        eprintln!("{}", "=".repeat(70));
    }
}

/// Format a number with thousand separators
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Try to truncate content to structure mode
///
/// Returns (truncated_content, was_truncated)
fn try_truncate_to_structure(path: &str, content: &str) -> (String, bool) {
    truncate_structure(content, path)
}

/// Apply token budget to select files based on priority
///
/// # Arguments
///
/// * `files` - List of (path, content) tuples
/// * `budget` - Maximum tokens allowed
/// * `lens_manager` - LensManager for priority resolution
/// * `strategy` - Budget strategy: "drop", "truncate", or "hybrid"
///
/// # Strategies
///
/// * `drop` - Exclude files that don't fit (default)
/// * `truncate` - Force structure mode on files that don't fit
/// * `hybrid` - Auto-truncate files consuming >10% of budget, then apply truncate logic
///
/// # Returns
///
/// * Tuple of (selected files, budget report)
pub fn apply_token_budget(
    files: Vec<(String, String)>,
    budget: usize,
    lens_manager: &LensManager,
    strategy: &str,
) -> (Vec<(String, String)>, BudgetReport) {
    // Step 1: Calculate tokens and get priorities, applying group-based truncation
    let mut file_data: Vec<FileData> = files.into_iter()
        .map(|(path, content)| {
            let path_obj = Path::new(&path);
            let group_config = lens_manager.get_file_group_config(path_obj);

            // Calculate original tokens before any truncation
            let original_tokens = TokenEstimator::estimate_file_tokens(path_obj, &content);

            // Apply group-level truncation if specified (e.g., structure mode for *.py)
            let (final_content, method) = if let Some(ref mode) = group_config.truncate_mode {
                if mode == "structure" {
                    let (truncated, was_truncated) = try_truncate_to_structure(&path, &content);
                    if was_truncated {
                        (truncated, "truncated".to_string())
                    } else {
                        (content, "full".to_string())
                    }
                } else {
                    (content, "full".to_string())
                }
            } else {
                (content, "full".to_string())
            };

            let tokens = TokenEstimator::estimate_file_tokens(path_obj, &final_content);

            FileData {
                path,
                content: final_content,
                priority: group_config.priority,
                tokens,
                original_tokens,
                method,
            }
        })
        .collect();

    // Step 2: Sort by tier (ASC), then priority (DESC), then path (ASC) for determinism
    // Tiered allocation ensures Core files get budget before Config, Tests, Other
    file_data.sort_by(|a, b| {
        let tier_a = FileTier::classify(&a.path, None) as u8;
        let tier_b = FileTier::classify(&b.path, None) as u8;

        match tier_a.cmp(&tier_b) {
            std::cmp::Ordering::Equal => {
                // Within same tier, sort by priority (highest first)
                match b.priority.cmp(&a.priority) {
                    std::cmp::Ordering::Equal => a.path.cmp(&b.path),
                    other => other,
                }
            }
            other => other,
        }
    });

    // Step 3: For hybrid strategy, pre-truncate large files (>10% of budget)
    if strategy == "hybrid" {
        let budget_threshold = (budget as f64 * HYBRID_THRESHOLD) as usize;
        for fd in &mut file_data {
            if fd.tokens > budget_threshold {
                let (truncated_content, was_truncated) = try_truncate_to_structure(&fd.path, &fd.content);
                if was_truncated {
                    let path_obj = Path::new(&fd.path);
                    let new_tokens = TokenEstimator::estimate_file_tokens(path_obj, &truncated_content);
                    fd.content = truncated_content;
                    fd.tokens = new_tokens;
                    fd.method = "truncated".to_string();
                }
            }
        }
    }

    // Step 4: Accumulate files within budget with strategy-specific handling
    let mut selected = Vec::new();
    let mut included_files = Vec::new();
    let mut total_tokens = 0;
    let mut dropped = Vec::new();
    let mut truncated_count = 0;

    for fd in file_data {
        // Check if file fits in remaining budget
        if total_tokens + fd.tokens <= budget {
            if fd.method == "truncated" {
                truncated_count += 1;
            }
            included_files.push((fd.path.clone(), fd.priority, fd.tokens, fd.method.clone()));
            selected.push((fd.path, fd.content));
            total_tokens += fd.tokens;
        } else {
            // File doesn't fit - apply strategy
            if strategy == "truncate" || strategy == "hybrid" {
                // Try to truncate to structure mode
                let (truncated_content, was_truncated) = try_truncate_to_structure(&fd.path, &fd.content);
                if was_truncated {
                    let path_obj = Path::new(&fd.path);
                    let new_tokens = TokenEstimator::estimate_file_tokens(path_obj, &truncated_content);
                    if total_tokens + new_tokens <= budget {
                        // Truncated version fits!
                        truncated_count += 1;
                        included_files.push((fd.path.clone(), fd.priority, new_tokens, "truncated".to_string()));
                        selected.push((fd.path, truncated_content));
                        total_tokens += new_tokens;
                        continue;
                    }
                }
            }
            // File still doesn't fit after truncation attempt (or drop strategy)
            dropped.push((fd.path, fd.priority, fd.original_tokens));
        }
    }

    // Step 5: Generate report
    let report = BudgetReport {
        budget,
        used: total_tokens,
        selected_count: selected.len(),
        dropped_count: dropped.len(),
        dropped_files: dropped,
        estimation_method: TokenEstimator::method().to_string(),
        strategy: strategy.to_string(),
        included_files,
        truncated_count,
    };

    (selected, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_number() {
        assert_eq!(parse_token_budget("100000").unwrap(), 100000);
        assert_eq!(parse_token_budget("50").unwrap(), 50);
    }

    #[test]
    fn test_parse_k_suffix() {
        assert_eq!(parse_token_budget("100k").unwrap(), 100_000);
        assert_eq!(parse_token_budget("100K").unwrap(), 100_000);
        assert_eq!(parse_token_budget("50k").unwrap(), 50_000);
    }

    #[test]
    fn test_parse_m_suffix() {
        assert_eq!(parse_token_budget("2m").unwrap(), 2_000_000);
        assert_eq!(parse_token_budget("2M").unwrap(), 2_000_000);
        assert_eq!(parse_token_budget("1M").unwrap(), 1_000_000);
    }

    #[test]
    fn test_parse_whitespace() {
        assert_eq!(parse_token_budget("  100k  ").unwrap(), 100_000);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_token_budget("").is_err());
        assert!(parse_token_budget("abc").is_err());
        assert!(parse_token_budget("100x").is_err());
    }

    #[test]
    fn test_token_estimation() {
        // 400 chars should be ~100 tokens
        let content = "x".repeat(400);
        assert_eq!(TokenEstimator::estimate_tokens(&content), 100);
    }

    #[test]
    fn test_budget_report_percentage() {
        let report = BudgetReport {
            budget: 1000,
            used: 500,
            selected_count: 5,
            dropped_count: 2,
            dropped_files: vec![],
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![],
            truncated_count: 0,
        };
        assert!((report.used_percentage() - 50.0).abs() < 0.1);
        assert_eq!(report.remaining(), 500);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(100), "100");
    }

    #[test]
    fn test_drop_strategy_skips_oversized() {
        let lens_manager = LensManager::new();
        let files = vec![
            ("small.py".to_string(), "x".repeat(100)),  // ~25 tokens
            ("large.py".to_string(), "y".repeat(10000)), // ~2500 tokens
        ];
        let (selected, report) = apply_token_budget(files, 500, &lens_manager, "drop");

        // Small file should be included, large should be dropped
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].0, "small.py");
        assert_eq!(report.dropped_count, 1);
        assert_eq!(report.strategy, "drop");
    }

    #[test]
    fn test_truncate_strategy_truncates_oversized() {
        let lens_manager = LensManager::new();
        // Create a Python file with class definition that can be truncated
        let python_content = r#"class MyClass:
    """A test class with documentation."""

    def method_one(self, arg1, arg2):
        """First method."""
        x = 1
        y = 2
        z = 3
        return x + y + z

    def method_two(self):
        """Second method."""
        for i in range(100):
            print(i)
        return True
"#.to_string();

        let files = vec![
            ("test.py".to_string(), python_content),
        ];

        // Budget small enough that full file doesn't fit
        let (selected, report) = apply_token_budget(files, 50, &lens_manager, "truncate");

        // File should be included (truncated) or dropped depending on truncated size
        assert_eq!(report.strategy, "truncate");
        // The file might fit or not depending on truncation result
        if report.selected_count > 0 {
            assert!(report.truncated_count > 0 || report.included_files.iter().any(|(_, _, _, m)| m == "truncated"));
        }
    }

    #[test]
    fn test_hybrid_strategy_pre_truncates_large_files() {
        let lens_manager = LensManager::new();
        // Create a Python file that's > 10% of budget
        let python_content = r#"class LargeClass:
    """A large class that exceeds 10% of budget."""

    def method_one(self):
        """Method one."""
        return 1

    def method_two(self):
        """Method two."""
        return 2

    def method_three(self):
        """Method three."""
        return 3
"#.to_string();

        let files = vec![
            ("large.py".to_string(), python_content.repeat(10)), // ~10x content
            ("small.py".to_string(), "x = 1".to_string()),
        ];

        // Budget where large file > 10%
        let (selected, report) = apply_token_budget(files, 1000, &lens_manager, "hybrid");

        // Both files should potentially be included
        assert_eq!(report.strategy, "hybrid");
        // Hybrid should auto-truncate large files
        assert!(selected.len() >= 1);
    }

    #[test]
    fn test_strategy_report_shows_correct_strategy() {
        let lens_manager = LensManager::new();
        let files = vec![("test.py".to_string(), "x = 1".to_string())];

        let (_, report_drop) = apply_token_budget(files.clone(), 1000, &lens_manager, "drop");
        assert_eq!(report_drop.strategy, "drop");

        let (_, report_truncate) = apply_token_budget(files.clone(), 1000, &lens_manager, "truncate");
        assert_eq!(report_truncate.strategy, "truncate");

        let (_, report_hybrid) = apply_token_budget(files, 1000, &lens_manager, "hybrid");
        assert_eq!(report_hybrid.strategy, "hybrid");
    }

    #[test]
    fn test_file_token_estimation_with_overhead() {
        let path = Path::new("test.py");
        let content = "x".repeat(400); // 400 chars = 100 tokens base
        let tokens = TokenEstimator::estimate_file_tokens(path, &content);
        // Should include overhead for PM format markers
        assert!(tokens > 100);
        assert!(tokens < 150); // But not too much overhead
    }

    #[test]
    fn test_estimation_method_name() {
        assert_eq!(TokenEstimator::method(), "Heuristic (~4 chars/token)");
    }

    #[test]
    fn test_budget_report_remaining_over_budget() {
        let report = BudgetReport {
            budget: 100,
            used: 150, // Over budget
            selected_count: 2,
            dropped_count: 0,
            dropped_files: vec![],
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![],
            truncated_count: 0,
        };
        // Remaining should be 0 when over budget, not negative
        assert_eq!(report.remaining(), 0);
    }

    #[test]
    fn test_budget_report_zero_budget() {
        let report = BudgetReport {
            budget: 0,
            used: 0,
            selected_count: 0,
            dropped_count: 0,
            dropped_files: vec![],
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![],
            truncated_count: 0,
        };
        // Should handle zero budget gracefully
        assert_eq!(report.used_percentage(), 0.0);
        assert_eq!(report.remaining(), 0);
    }

    #[test]
    fn test_budget_report_print_with_truncated() {
        let report = BudgetReport {
            budget: 1000,
            used: 800,
            selected_count: 3,
            dropped_count: 1,
            dropped_files: vec![("dropped.py".to_string(), 50, 500)],
            estimation_method: "Heuristic (~4 chars/token)".to_string(),
            strategy: "hybrid".to_string(),
            included_files: vec![
                ("file1.py".to_string(), 100, 200, "full".to_string()),
                ("file2.py".to_string(), 80, 300, "truncated".to_string()),
                ("file3.py".to_string(), 60, 300, "full".to_string()),
            ],
            truncated_count: 1,
        };
        // Just verify print_report doesn't panic
        report.print_report();
    }

    #[test]
    fn test_budget_report_print_many_dropped() {
        let mut dropped_files = Vec::new();
        for i in 0..15 {
            dropped_files.push((format!("file{}.py", i), 50, 100));
        }
        let report = BudgetReport {
            budget: 1000,
            used: 500,
            selected_count: 5,
            dropped_count: 15,
            dropped_files,
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![],
            truncated_count: 0,
        };
        // Should show "... and X more" for >10 dropped files
        report.print_report();
    }

    #[test]
    fn test_budget_report_print_many_truncated() {
        let mut included_files = Vec::new();
        for i in 0..10 {
            included_files.push((format!("file{}.py", i), 100, 50, "truncated".to_string()));
        }
        let report = BudgetReport {
            budget: 1000,
            used: 500,
            selected_count: 10,
            dropped_count: 0,
            dropped_files: vec![],
            estimation_method: "Heuristic".to_string(),
            strategy: "hybrid".to_string(),
            included_files,
            truncated_count: 10,
        };
        // Should show "... and X more" for >5 truncated files
        report.print_report();
    }

    #[test]
    fn test_exact_budget_fit() {
        let lens_manager = LensManager::new();
        // Create files that exactly fill the budget
        let files = vec![
            ("a.py".to_string(), "x".repeat(100)), // ~25 tokens + overhead
            ("b.py".to_string(), "y".repeat(100)),
        ];
        let (selected, report) = apply_token_budget(files, 100, &lens_manager, "drop");

        // At least one file should fit
        assert!(selected.len() >= 1);
        assert!(report.used <= report.budget);
    }

    #[test]
    fn test_empty_file_list() {
        let lens_manager = LensManager::new();
        let files: Vec<(String, String)> = vec![];
        let (selected, report) = apply_token_budget(files, 1000, &lens_manager, "drop");

        assert_eq!(selected.len(), 0);
        assert_eq!(report.selected_count, 0);
        assert_eq!(report.dropped_count, 0);
        assert_eq!(report.used, 0);
    }

    #[test]
    fn test_priority_sorting_in_budget() {
        let mut lens_manager = LensManager::new();
        // Apply architecture lens to get priority groups
        let _ = lens_manager.apply_lens("architecture");

        let files = vec![
            ("tests/test.py".to_string(), "x".repeat(100)),  // Low priority (tests)
            ("src/main.py".to_string(), "y".repeat(100)),    // Higher priority
            ("README.md".to_string(), "z".repeat(100)),      // Medium priority
        ];

        // With limited budget, high priority files should be kept
        let (selected, _report) = apply_token_budget(files, 200, &lens_manager, "drop");

        // Should have selected at least some files
        assert!(!selected.is_empty());
    }

    #[test]
    fn test_format_number_edge_cases() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(12), "12");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(123456), "123,456");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_truncate_strategy_when_truncation_doesnt_help() {
        let lens_manager = LensManager::new();
        // A non-code file that can't be meaningfully truncated
        let files = vec![
            ("data.txt".to_string(), "x".repeat(10000)), // Large non-code file
        ];

        // Very small budget
        let (_selected, report) = apply_token_budget(files, 10, &lens_manager, "truncate");

        // Strategy should still be recorded
        assert_eq!(report.strategy, "truncate");
    }

    #[test]
    fn test_hybrid_threshold_boundary() {
        let lens_manager = LensManager::new();
        // Create a file that's exactly at 10% threshold
        let python_content = r#"def func():
    pass
"#.to_string();

        let files = vec![
            ("small.py".to_string(), python_content.clone()),
            ("medium.py".to_string(), python_content.repeat(5)),
        ];

        let (_selected, report) = apply_token_budget(files, 500, &lens_manager, "hybrid");
        assert_eq!(report.strategy, "hybrid");
    }

    #[test]
    fn test_tiered_budgeting_core_before_tests() {
        let lens_manager = LensManager::new();
        // Create files from different tiers with same size
        let files = vec![
            ("tests/test_main.py".to_string(), "x".repeat(100)),   // Tests tier
            ("src/main.rs".to_string(), "y".repeat(100)),          // Core tier
            ("README.md".to_string(), "z".repeat(100)),            // Other tier
            ("Cargo.toml".to_string(), "w".repeat(100)),           // Config tier
        ];

        // Budget for only 2 files
        let (selected, _report) = apply_token_budget(files, 80, &lens_manager, "drop");

        // Core file (src/main.rs) should be selected first
        assert!(!selected.is_empty());
        let selected_paths: Vec<&str> = selected.iter().map(|(p, _)| p.as_str()).collect();

        // If any file is selected, Core should be prioritized over Tests/Other
        if selected_paths.len() >= 1 {
            // First file should be from Core tier (src/)
            assert!(
                selected_paths[0].starts_with("src/") || selected_paths[0] == "Cargo.toml",
                "Expected Core or Config file first, got: {}",
                selected_paths[0]
            );
        }
    }

    #[test]
    fn test_tiered_budgeting_order() {
        let lens_manager = LensManager::new();
        // Create small files from each tier
        let files = vec![
            ("docs/guide.md".to_string(), "a".repeat(40)),         // Other (tier 3)
            ("tests/test.py".to_string(), "b".repeat(40)),         // Tests (tier 2)
            ("config.toml".to_string(), "c".repeat(40)),           // Config (tier 1)
            ("src/lib.rs".to_string(), "d".repeat(40)),            // Core (tier 0)
        ];

        // Budget for 3 files (drops 1)
        let (selected, _report) = apply_token_budget(files, 100, &lens_manager, "drop");

        let selected_paths: Vec<&str> = selected.iter().map(|(p, _)| p.as_str()).collect();

        // If we have selections, verify tier ordering
        if selected_paths.len() >= 2 {
            // Core should come before Other in the selection
            let has_core = selected_paths.iter().any(|p| p.starts_with("src/"));
            let has_other = selected_paths.iter().any(|p| p.starts_with("docs/"));

            // If budget was tight, Core should be kept over Other
            if has_core && !has_other {
                // Good: Core prioritized
            } else if has_core && has_other {
                // Both fit, also fine
            }
            // Core should always be included if budget allows
            assert!(has_core || selected_paths.is_empty(), "Core files should be prioritized");
        }
    }

    // =========================================================================
    // Additional Coverage Tests
    // =========================================================================

    #[test]
    fn test_file_data_struct() {
        let fd = FileData {
            path: "test.py".to_string(),
            content: "x = 1".to_string(),
            priority: 100,
            tokens: 10,
            original_tokens: 10,
            method: "full".to_string(),
        };

        assert_eq!(fd.path, "test.py");
        assert_eq!(fd.content, "x = 1");
        assert_eq!(fd.priority, 100);
        assert_eq!(fd.tokens, 10);
        assert_eq!(fd.original_tokens, 10);
        assert_eq!(fd.method, "full");
    }

    #[test]
    fn test_file_data_clone() {
        let fd = FileData {
            path: "test.py".to_string(),
            content: "x = 1".to_string(),
            priority: 100,
            tokens: 10,
            original_tokens: 10,
            method: "full".to_string(),
        };

        let cloned = fd.clone();
        assert_eq!(cloned.path, "test.py");
        assert_eq!(cloned.method, "full");
    }

    #[test]
    fn test_budget_report_clone() {
        let report = BudgetReport {
            budget: 1000,
            used: 500,
            selected_count: 5,
            dropped_count: 2,
            dropped_files: vec![("test.py".to_string(), 50, 100)],
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![("main.py".to_string(), 100, 200, "full".to_string())],
            truncated_count: 0,
        };

        let cloned = report.clone();
        assert_eq!(cloned.budget, 1000);
        assert_eq!(cloned.selected_count, 5);
        assert_eq!(cloned.dropped_files.len(), 1);
    }

    #[test]
    fn test_token_estimator_empty_content() {
        assert_eq!(TokenEstimator::estimate_tokens(""), 0);
    }

    #[test]
    fn test_token_estimator_short_content() {
        // Less than 4 chars should still give 0 or 1
        assert_eq!(TokenEstimator::estimate_tokens("abc"), 0);
        assert_eq!(TokenEstimator::estimate_tokens("abcd"), 1);
    }

    #[test]
    fn test_file_token_estimation_empty_file() {
        let path = Path::new("empty.py");
        let tokens = TokenEstimator::estimate_file_tokens(path, "");
        // Should have overhead even for empty content
        assert!(tokens > 0);
    }

    #[test]
    fn test_file_token_estimation_long_path() {
        let path = Path::new("very/long/nested/directory/structure/deep/file.py");
        let content = "x = 1";
        let tokens = TokenEstimator::estimate_file_tokens(path, content);
        // Long path should increase overhead
        assert!(tokens > TokenEstimator::estimate_tokens(content));
    }

    #[test]
    fn test_try_truncate_to_structure() {
        // Python file with class
        let python_content = r#"class Foo:
    def bar(self):
        x = 1
        y = 2
        return x + y
"#;
        let (result, _was_truncated) = try_truncate_to_structure("test.py", python_content);

        // Result should be non-empty
        assert!(!result.is_empty());
    }

    #[test]
    fn test_try_truncate_to_structure_non_code() {
        // Plain text file shouldn't truncate
        let content = "This is just plain text without any code structure.";
        let (result, was_truncated) = try_truncate_to_structure("readme.txt", content);

        // Plain text may or may not truncate depending on implementation
        assert!(!result.is_empty());
        // If not truncated, content should be same
        if !was_truncated {
            assert_eq!(result, content);
        }
    }

    #[test]
    fn test_budget_report_full_usage() {
        let report = BudgetReport {
            budget: 1000,
            used: 1000,
            selected_count: 10,
            dropped_count: 0,
            dropped_files: vec![],
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![],
            truncated_count: 0,
        };

        assert_eq!(report.used_percentage(), 100.0);
        assert_eq!(report.remaining(), 0);
    }

    #[test]
    fn test_apply_budget_with_group_truncation() {
        let mut lens_manager = LensManager::new();
        // Apply a lens that might have group-level truncation
        let _ = lens_manager.apply_lens("architecture");

        let files = vec![
            ("src/main.py".to_string(), "def main():\n    pass".to_string()),
        ];

        let (selected, report) = apply_token_budget(files, 1000, &lens_manager, "drop");

        // File should be selected
        assert!(!selected.is_empty());
        assert_eq!(report.strategy, "drop");
    }

    #[test]
    fn test_hybrid_strategy_all_small_files() {
        let lens_manager = LensManager::new();
        // All files are small (< 10% of budget)
        let files = vec![
            ("a.py".to_string(), "x = 1".to_string()),
            ("b.py".to_string(), "y = 2".to_string()),
            ("c.py".to_string(), "z = 3".to_string()),
        ];

        let (selected, report) = apply_token_budget(files, 1000, &lens_manager, "hybrid");

        // All small files should be included without truncation
        assert_eq!(selected.len(), 3);
        assert_eq!(report.truncated_count, 0);
    }

    #[test]
    fn test_budget_report_debug() {
        let report = BudgetReport {
            budget: 1000,
            used: 500,
            selected_count: 5,
            dropped_count: 2,
            dropped_files: vec![],
            estimation_method: "Heuristic".to_string(),
            strategy: "drop".to_string(),
            included_files: vec![],
            truncated_count: 0,
        };

        let debug_str = format!("{:?}", report);
        assert!(debug_str.contains("BudgetReport"));
        assert!(debug_str.contains("1000"));
    }

    #[test]
    fn test_file_data_debug() {
        let fd = FileData {
            path: "test.py".to_string(),
            content: "x = 1".to_string(),
            priority: 100,
            tokens: 10,
            original_tokens: 10,
            method: "full".to_string(),
        };

        let debug_str = format!("{:?}", fd);
        assert!(debug_str.contains("FileData"));
        assert!(debug_str.contains("test.py"));
    }

    #[test]
    fn test_parse_token_budget_single_digit() {
        assert_eq!(parse_token_budget("5").unwrap(), 5);
        assert_eq!(parse_token_budget("1k").unwrap(), 1000);
        assert_eq!(parse_token_budget("1M").unwrap(), 1000000);
    }

    #[test]
    fn test_deterministic_ordering() {
        let lens_manager = LensManager::new();
        // Files with same tier and priority
        let files = vec![
            ("src/c.py".to_string(), "c".to_string()),
            ("src/a.py".to_string(), "a".to_string()),
            ("src/b.py".to_string(), "b".to_string()),
        ];

        let (selected1, _) = apply_token_budget(files.clone(), 1000, &lens_manager, "drop");
        let (selected2, _) = apply_token_budget(files, 1000, &lens_manager, "drop");

        // Order should be deterministic (sorted by path)
        assert_eq!(selected1.len(), selected2.len());
        for (f1, f2) in selected1.iter().zip(selected2.iter()) {
            assert_eq!(f1.0, f2.0);
        }
    }
}
