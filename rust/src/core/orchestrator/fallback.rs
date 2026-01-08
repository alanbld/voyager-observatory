//! Fallback System Module
//!
//! Provides graceful degradation when semantic analysis fails or times out.
//! The user never sees internal errors - the system silently falls back to
//! simpler analysis methods.

use std::time::Duration;

// =============================================================================
// Analysis Strategy
// =============================================================================

/// Analysis strategy with automatic fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStrategy {
    /// Full semantic analysis with unified substrate
    SemanticDeep,
    /// Quick semantic analysis with timeout
    SemanticQuick,
    /// Heuristic-only analysis (pattern matching)
    Heuristic,
    /// Minimal analysis (just file structure)
    Minimal,
}

impl AnalysisStrategy {
    /// Get the next fallback strategy.
    ///
    /// Returns None if there's no further fallback available.
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::SemanticDeep => Some(Self::SemanticQuick),
            Self::SemanticQuick => Some(Self::Heuristic),
            Self::Heuristic => Some(Self::Minimal),
            Self::Minimal => None,
        }
    }

    /// Get the timeout for this strategy.
    pub fn timeout(&self) -> Duration {
        match self {
            Self::SemanticDeep => Duration::from_secs(30),
            Self::SemanticQuick => Duration::from_millis(500),
            Self::Heuristic => Duration::from_millis(100),
            Self::Minimal => Duration::from_millis(10),
        }
    }

    /// Get a user-friendly description of this strategy.
    pub fn description(&self) -> &'static str {
        match self {
            Self::SemanticDeep => "Deep semantic analysis",
            Self::SemanticQuick => "Quick semantic analysis",
            Self::Heuristic => "Pattern-based analysis",
            Self::Minimal => "Structural analysis",
        }
    }
}

// =============================================================================
// Fallback System
// =============================================================================

/// Manages fallback logic for analysis strategies.
pub struct FallbackSystem {
    /// Maximum number of fallback attempts
    max_attempts: usize,
    /// Whether to log fallback events (for debugging)
    log_fallbacks: bool,
}

impl Default for FallbackSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FallbackSystem {
    /// Create a new fallback system.
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            log_fallbacks: false,
        }
    }

    /// Enable fallback logging (for debugging).
    pub fn with_logging(mut self) -> Self {
        self.log_fallbacks = true;
        self
    }

    /// Execute with automatic fallback.
    ///
    /// Tries the primary strategy, falling back on failure or timeout.
    /// Returns the result and the strategy that was ultimately used.
    pub fn execute_with_fallback<T, E, F>(
        &self,
        initial_strategy: AnalysisStrategy,
        mut executor: F,
    ) -> Result<(T, AnalysisStrategy), FallbackError>
    where
        F: FnMut(AnalysisStrategy) -> Result<T, E>,
        E: std::fmt::Display,
    {
        let mut current_strategy = initial_strategy;
        let mut attempts = 0;

        loop {
            attempts += 1;

            match executor(current_strategy) {
                Ok(result) => {
                    return Ok((result, current_strategy));
                }
                Err(e) => {
                    if self.log_fallbacks {
                        eprintln!(
                            "[FALLBACK] {} failed: {}",
                            current_strategy.description(),
                            e
                        );
                    }

                    if attempts >= self.max_attempts {
                        return Err(FallbackError::MaxAttemptsReached {
                            attempts,
                            last_error: e.to_string(),
                        });
                    }

                    match current_strategy.fallback() {
                        Some(next) => {
                            if self.log_fallbacks {
                                eprintln!("[FALLBACK] Trying: {}", next.description());
                            }
                            current_strategy = next;
                        }
                        None => {
                            return Err(FallbackError::NoMoreFallbacks {
                                last_strategy: current_strategy,
                                last_error: e.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Determine initial strategy based on semantic depth setting.
    pub fn strategy_for_depth(&self, depth: super::SemanticDepth) -> AnalysisStrategy {
        match depth {
            super::SemanticDepth::Deep => AnalysisStrategy::SemanticDeep,
            super::SemanticDepth::Balanced => AnalysisStrategy::SemanticQuick,
            super::SemanticDepth::Quick => AnalysisStrategy::Heuristic,
        }
    }
}

// =============================================================================
// Fallback Error
// =============================================================================

/// Errors from the fallback system.
#[derive(Debug)]
pub enum FallbackError {
    /// Maximum retry attempts reached
    MaxAttemptsReached { attempts: usize, last_error: String },
    /// No more fallback strategies available
    NoMoreFallbacks {
        last_strategy: AnalysisStrategy,
        last_error: String,
    },
}

impl std::fmt::Display for FallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxAttemptsReached {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "Analysis failed after {} attempts: {}",
                    attempts, last_error
                )
            }
            Self::NoMoreFallbacks {
                last_strategy,
                last_error,
            } => {
                write!(
                    f,
                    "All analysis strategies exhausted. Last tried: {}. Error: {}",
                    last_strategy.description(),
                    last_error
                )
            }
        }
    }
}

impl std::error::Error for FallbackError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AnalysisStrategy Tests
    // =========================================================================

    #[test]
    fn test_analysis_strategy_fallback() {
        assert_eq!(
            AnalysisStrategy::SemanticDeep.fallback(),
            Some(AnalysisStrategy::SemanticQuick)
        );
        assert_eq!(
            AnalysisStrategy::SemanticQuick.fallback(),
            Some(AnalysisStrategy::Heuristic)
        );
        assert_eq!(
            AnalysisStrategy::Heuristic.fallback(),
            Some(AnalysisStrategy::Minimal)
        );
        assert_eq!(AnalysisStrategy::Minimal.fallback(), None);
    }

    #[test]
    fn test_analysis_strategy_timeout() {
        assert_eq!(
            AnalysisStrategy::SemanticDeep.timeout(),
            Duration::from_secs(30)
        );
        assert_eq!(
            AnalysisStrategy::SemanticQuick.timeout(),
            Duration::from_millis(500)
        );
        assert_eq!(
            AnalysisStrategy::Heuristic.timeout(),
            Duration::from_millis(100)
        );
        assert_eq!(
            AnalysisStrategy::Minimal.timeout(),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn test_analysis_strategy_description() {
        assert_eq!(
            AnalysisStrategy::SemanticDeep.description(),
            "Deep semantic analysis"
        );
        assert_eq!(
            AnalysisStrategy::SemanticQuick.description(),
            "Quick semantic analysis"
        );
        assert_eq!(
            AnalysisStrategy::Heuristic.description(),
            "Pattern-based analysis"
        );
        assert_eq!(
            AnalysisStrategy::Minimal.description(),
            "Structural analysis"
        );
    }

    #[test]
    fn test_analysis_strategy_equality() {
        assert_eq!(
            AnalysisStrategy::SemanticDeep,
            AnalysisStrategy::SemanticDeep
        );
        assert_ne!(AnalysisStrategy::SemanticDeep, AnalysisStrategy::Minimal);
    }

    #[test]
    fn test_analysis_strategy_clone() {
        let strategy = AnalysisStrategy::SemanticDeep;
        let cloned = strategy;
        assert_eq!(strategy, cloned);
    }

    // =========================================================================
    // FallbackSystem Tests
    // =========================================================================

    #[test]
    fn test_fallback_system_new() {
        let fallback = FallbackSystem::new();
        // Test that it doesn't panic and can be used
        assert!(fallback
            .execute_with_fallback(AnalysisStrategy::Minimal, |_| -> Result<(), &str> {
                Ok(())
            },)
            .is_ok());
    }

    #[test]
    fn test_fallback_system_default() {
        let fallback = FallbackSystem::default();
        assert!(fallback
            .execute_with_fallback(AnalysisStrategy::Minimal, |_| -> Result<(), &str> {
                Ok(())
            },)
            .is_ok());
    }

    #[test]
    fn test_fallback_system_with_logging() {
        let fallback = FallbackSystem::new().with_logging();
        // Just verify it can be created and used
        let result = fallback
            .execute_with_fallback(AnalysisStrategy::Minimal, |_| -> Result<i32, &str> {
                Ok(42)
            });
        assert!(result.is_ok());
    }

    #[test]
    fn test_fallback_system_success() {
        let fallback = FallbackSystem::new();
        let result = fallback.execute_with_fallback(
            AnalysisStrategy::SemanticDeep,
            |_strategy| -> Result<i32, &str> { Ok(42) },
        );

        let (value, strategy) = result.unwrap();
        assert_eq!(value, 42);
        assert_eq!(strategy, AnalysisStrategy::SemanticDeep);
    }

    #[test]
    fn test_fallback_system_falls_back() {
        let fallback = FallbackSystem::new();
        let mut attempts = 0;

        let result = fallback.execute_with_fallback(
            AnalysisStrategy::SemanticDeep,
            |strategy| -> Result<i32, &str> {
                attempts += 1;
                if strategy == AnalysisStrategy::Heuristic {
                    Ok(42)
                } else {
                    Err("not ready")
                }
            },
        );

        let (value, strategy) = result.unwrap();
        assert_eq!(value, 42);
        assert_eq!(strategy, AnalysisStrategy::Heuristic);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_fallback_system_max_attempts() {
        let fallback = FallbackSystem::new();

        let result = fallback.execute_with_fallback(
            AnalysisStrategy::SemanticDeep,
            |_strategy| -> Result<i32, &str> { Err("always fails") },
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_system_no_more_fallbacks() {
        let fallback = FallbackSystem::new();

        // Start from Minimal which has no fallback
        let result = fallback.execute_with_fallback(
            AnalysisStrategy::Minimal,
            |_strategy| -> Result<i32, &str> { Err("fails") },
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            FallbackError::NoMoreFallbacks { last_strategy, .. } => {
                assert_eq!(last_strategy, AnalysisStrategy::Minimal);
            }
            _ => panic!("Expected NoMoreFallbacks error"),
        }
    }

    #[test]
    fn test_fallback_system_strategy_for_depth() {
        let fallback = FallbackSystem::new();

        assert_eq!(
            fallback.strategy_for_depth(super::super::SemanticDepth::Deep),
            AnalysisStrategy::SemanticDeep
        );
        assert_eq!(
            fallback.strategy_for_depth(super::super::SemanticDepth::Balanced),
            AnalysisStrategy::SemanticQuick
        );
        assert_eq!(
            fallback.strategy_for_depth(super::super::SemanticDepth::Quick),
            AnalysisStrategy::Heuristic
        );
    }

    // =========================================================================
    // FallbackError Tests
    // =========================================================================

    #[test]
    fn test_fallback_error_max_attempts_display() {
        let error = FallbackError::MaxAttemptsReached {
            attempts: 3,
            last_error: "test error".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("3 attempts"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_fallback_error_no_more_fallbacks_display() {
        let error = FallbackError::NoMoreFallbacks {
            last_strategy: AnalysisStrategy::Minimal,
            last_error: "test error".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("exhausted"));
        assert!(display.contains("Structural analysis"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_fallback_error_is_error() {
        let error: Box<dyn std::error::Error> = Box::new(FallbackError::MaxAttemptsReached {
            attempts: 1,
            last_error: "test".to_string(),
        });
        // Verify it implements Error trait
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn test_fallback_error_debug() {
        let error = FallbackError::MaxAttemptsReached {
            attempts: 2,
            last_error: "debug test".to_string(),
        };
        let debug = format!("{:?}", error);
        assert!(debug.contains("MaxAttemptsReached"));
        assert!(debug.contains("2"));
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_fallback_with_logging_falls_back() {
        let fallback = FallbackSystem::new().with_logging();
        let mut attempts = 0;

        // This will trigger the logging paths
        let result = fallback.execute_with_fallback(
            AnalysisStrategy::SemanticDeep,
            |strategy| -> Result<i32, &str> {
                attempts += 1;
                if strategy == AnalysisStrategy::Heuristic {
                    Ok(42)
                } else {
                    Err("not ready yet")
                }
            },
        );

        let (value, strategy) = result.unwrap();
        assert_eq!(value, 42);
        assert_eq!(strategy, AnalysisStrategy::Heuristic);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_fallback_with_logging_max_attempts() {
        let fallback = FallbackSystem::new().with_logging();

        let result = fallback
            .execute_with_fallback(AnalysisStrategy::SemanticDeep, |_| -> Result<i32, &str> {
                Err("always fails with logging")
            });

        assert!(result.is_err());
        match result.unwrap_err() {
            FallbackError::MaxAttemptsReached {
                attempts,
                last_error,
            } => {
                assert_eq!(attempts, 3);
                assert!(last_error.contains("always fails"));
            }
            _ => panic!("Expected MaxAttemptsReached error"),
        }
    }

    #[test]
    fn test_fallback_with_logging_no_more_fallbacks() {
        let fallback = FallbackSystem::new().with_logging();

        let result = fallback
            .execute_with_fallback(AnalysisStrategy::Minimal, |_| -> Result<i32, &str> {
                Err("fails with log")
            });

        assert!(result.is_err());
    }

    #[test]
    fn test_fallback_error_no_more_fallbacks_debug() {
        let error = FallbackError::NoMoreFallbacks {
            last_strategy: AnalysisStrategy::Minimal,
            last_error: "debug test".to_string(),
        };
        let debug = format!("{:?}", error);
        assert!(debug.contains("NoMoreFallbacks"));
        assert!(debug.contains("Minimal"));
    }
}
