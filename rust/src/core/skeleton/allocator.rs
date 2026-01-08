//! Adaptive Allocator for Skeleton Protocol v2.2
//!
//! Implements the 3-pass budget allocation strategy:
//! 1. Baseline: Set all to Skeleton
//! 2. Upgrade: Core → Config → Tests (if budget permits)
//! 3. Downgrade: Drop Other → Tests → Config (if over budget)

use super::types::{CompressionLevel, FileAllocation};
use crate::core::FileTier;

/// Adaptive allocator for budget-constrained file compression
#[derive(Debug, Clone)]
pub struct AdaptiveAllocator {
    /// Token budget limit
    budget: usize,
}

impl AdaptiveAllocator {
    /// Create a new allocator with the given token budget
    pub fn new(budget: usize) -> Self {
        Self { budget }
    }

    /// Allocate compression levels to files within the budget
    ///
    /// Uses a 3-pass strategy:
    /// 1. Set all files to Skeleton compression
    /// 2. Upgrade highest-priority files to Full if budget permits
    /// 3. Drop lowest-priority files if still over budget
    pub fn allocate(&self, files: Vec<FileAllocation>) -> Vec<FileAllocation> {
        if files.is_empty() {
            return files;
        }

        let mut allocations = files;

        // Pass 1: Set all to Skeleton
        for file in &mut allocations {
            file.level = CompressionLevel::Skeleton;
        }

        // Calculate baseline cost
        let baseline_cost: usize = allocations.iter().map(|f| f.current_tokens()).sum();

        if baseline_cost <= self.budget {
            // Pass 2: Try to upgrade files (Core first, then Config, then Tests)
            Self::upgrade_pass(self.budget, &mut allocations);
        } else {
            // Pass 3: Downgrade/drop files (Other first, then Tests, then Config)
            Self::downgrade_pass(self.budget, &mut allocations);
        }

        allocations
    }

    /// Upgrade pass: Promote files to Full starting with highest priority
    fn upgrade_pass(budget: usize, allocations: &mut [FileAllocation]) {
        let current: usize = allocations.iter().map(|f| f.current_tokens()).sum();
        let mut remaining_budget = budget.saturating_sub(current);

        // Priority order for upgrading: Core > Config > Tests > Other
        let upgrade_order = [
            FileTier::Core,
            FileTier::Config,
            FileTier::Tests,
            FileTier::Other,
        ];

        for tier in upgrade_order {
            for file in allocations.iter_mut() {
                if file.tier == tier && file.level == CompressionLevel::Skeleton {
                    let upgrade_cost = file.upgrade_cost();
                    if upgrade_cost <= remaining_budget {
                        file.level = CompressionLevel::Full;
                        remaining_budget = remaining_budget.saturating_sub(upgrade_cost);
                    }
                }
            }
        }
    }

    /// Downgrade pass: Drop files starting with lowest priority
    fn downgrade_pass(budget: usize, allocations: &mut [FileAllocation]) {
        // Priority order for dropping: Other > Tests > Config > Core
        let drop_order = [
            FileTier::Other,
            FileTier::Tests,
            FileTier::Config,
            FileTier::Core,
        ];

        for tier in drop_order {
            // Find indices of files in this tier that can be dropped
            let indices: Vec<usize> = allocations
                .iter()
                .enumerate()
                .filter(|(_, f)| f.tier == tier && f.level != CompressionLevel::Drop)
                .map(|(i, _)| i)
                .collect();

            for idx in indices {
                allocations[idx].level = CompressionLevel::Drop;

                // Check if we're now within budget
                let current: usize = allocations.iter().map(|f| f.current_tokens()).sum();
                if current <= budget {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_new() {
        let allocator = AdaptiveAllocator::new(1000);
        assert_eq!(allocator.budget, 1000);
    }

    #[test]
    fn test_allocator_empty_input() {
        let allocator = AdaptiveAllocator::new(100);
        let result = allocator.allocate(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_baseline_all_skeleton() {
        let files = vec![
            FileAllocation::new("src/main.rs", FileTier::Core, 100, 10),
            FileAllocation::new("config.toml", FileTier::Config, 50, 5),
        ];

        let allocator = AdaptiveAllocator::new(15); // Exactly skeleton cost
        let result = allocator.allocate(files);

        assert_eq!(result[0].level, CompressionLevel::Skeleton);
        assert_eq!(result[1].level, CompressionLevel::Skeleton);
    }

    #[test]
    fn test_upgrade_core_first() {
        let files = vec![
            FileAllocation::new("src/main.rs", FileTier::Core, 100, 10),
            FileAllocation::new("config.toml", FileTier::Config, 100, 10),
        ];

        // Budget allows Core to upgrade but not Config
        let allocator = AdaptiveAllocator::new(120);
        let result = allocator.allocate(files);

        assert_eq!(result[0].level, CompressionLevel::Full);
        assert_eq!(result[1].level, CompressionLevel::Skeleton);
    }

    #[test]
    fn test_drop_other_first() {
        let files = vec![
            FileAllocation::new("src/main.rs", FileTier::Core, 100, 10),
            FileAllocation::new("docs/readme.md", FileTier::Other, 100, 10),
        ];

        // Budget only allows one skeleton
        let allocator = AdaptiveAllocator::new(10);
        let result = allocator.allocate(files);

        assert_eq!(result[0].level, CompressionLevel::Skeleton);
        assert_eq!(result[1].level, CompressionLevel::Drop);
    }
}
