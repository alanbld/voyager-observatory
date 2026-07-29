//! Unified scoring layer (roadmap 2.4)
//!
//! Lenses (file path -> integer priority) and intents (symbol metadata ->
//! float relevance) are deliberately kept as separate user-facing surfaces
//! (Q2 in REVIEW_DECISIONS.md: a lens modifies serialized content, an intent
//! produces a terminal report). What they can share is the *aggregation and
//! blend* spine underneath: combine weighted [`Signal`]s into a [`Score`],
//! then optionally blend with a [`ContextStore`]'s learned utility via a
//! [`BlendPolicy`].
//!
//! Each subsystem keeps its own subject type and its own signal-generation
//! logic via the [`Scorer`] trait — there is no shared "subject" model, only
//! a shared way to combine whatever signals a subsystem produces. Scores
//! stay in whatever domain the `Scorer` naturally produces: lenses' single
//! signal is already priority-scale (0-100, and per `PriorityGroup`'s own
//! doc comment, arbitrary integers are allowed), while intents' signals are
//! 0.0..=1.0 relevance. Nothing here forces a universal 0.0..=1.0 domain —
//! doing so would silently clamp/lose precision for lens priorities outside
//! that range.

use crate::core::store::ContextStore;

/// One factor's contribution to a [`Score`]. The domain of `value` is
/// whatever the producing [`Scorer`] uses (0.0..=1.0 for intents,
/// priority-scale for lenses) — this type does not normalize or clamp.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub name: &'static str,
    pub value: f32,
    pub weight: f32,
}

impl Signal {
    pub fn new(name: &'static str, value: f32, weight: f32) -> Self {
        Self {
            name,
            value,
            weight,
        }
    }
}

/// Combine signals into a single value via a weighted average. Signals with
/// non-positive total weight score as 0.0 (nothing to justify a score). Use
/// this when signals compete for one channel and should be averaged
/// together (e.g. lenses' single glob-priority signal).
pub fn weighted_sum(signals: &[Signal]) -> f32 {
    let total_weight: f32 = signals.iter().map(|s| s.weight).sum();
    if total_weight <= 0.0 {
        return 0.0;
    }
    signals.iter().map(|s| s.value * s.weight).sum::<f32>() / total_weight
}

/// Combine signals via a plain sum of `value * weight` — NOT an average.
/// Use this when signals are independent, stacking bonuses/penalties (each
/// `weight` a fixed per-factor coefficient, `value` how much of it to
/// award) rather than competing votes — e.g. intents' relevance scoring,
/// which sums a concept-type weight, documentation/visibility boosts, and
/// complexity/name-clarity adjustments that can be positive or negative.
pub fn additive_sum(signals: &[Signal]) -> f32 {
    signals.iter().map(|s| s.value * s.weight).sum()
}

/// The result of scoring a subject: the aggregated value, the signals that
/// produced it (kept for explainability), and whether learned-utility
/// blending was applied.
#[derive(Debug, Clone)]
pub struct Score {
    pub value: f32,
    pub signals: Vec<Signal>,
    pub blended: bool,
}

impl Score {
    /// Round to an integer. Meaningful for priority-scale scorers (lenses);
    /// 0.0..=1.0 relevance scorers (intents) should use `value` directly.
    pub fn as_priority(&self) -> i32 {
        self.value.round() as i32
    }
}

/// How a raw (static) score combines with a `ContextStore`'s learned
/// utility for a given key.
pub trait BlendPolicy {
    fn apply(&self, raw: f32, store: &ContextStore, key: &str) -> f32;
}

/// The blend formula `ContextStore::blend_priority` already uses:
/// `raw * 0.7 + learned * 100.0 * 0.3`. Exists so lenses can move onto the
/// shared scoring layer without changing behavior; roadmap 3.3's
/// decay/confidence-weighted policy will provide an alternative
/// `BlendPolicy` impl without changing this trait's shape.
#[derive(Debug, Clone, Copy, Default)]
pub struct LinearBlend;

impl BlendPolicy for LinearBlend {
    fn apply(&self, raw: f32, store: &ContextStore, key: &str) -> f32 {
        let learned = store.get_utility_score(key) as f32;
        raw * 0.7 + learned * 100.0 * 0.3
    }
}

/// Cross-cutting context threaded through every [`score`] call.
pub struct ScoringContext<'a> {
    pub store: Option<&'a ContextStore>,
    pub frozen: bool,
    pub blend: &'a dyn BlendPolicy,
}

impl<'a> ScoringContext<'a> {
    /// No store, frozen — every score is its raw, unblended value.
    pub fn static_only() -> Self {
        Self {
            store: None,
            frozen: true,
            blend: &LinearBlend,
        }
    }
}

/// Implemented by each scoring subsystem over its own subject type.
/// `signals` must be context-free (no store/frozen access) — [`score`]
/// handles aggregation and the optional learned-utility blend.
pub trait Scorer {
    type Subject<'s>: ?Sized;

    /// Key used to look up learned utility (usually a file path). `None`
    /// disables blending for this subject even when a store is present.
    fn utility_key(&self, subject: &Self::Subject<'_>) -> Option<String>;

    /// Context-free signals for this subject.
    fn signals(&self, subject: &Self::Subject<'_>) -> Vec<Signal>;
}

/// Score a subject: aggregate its `Scorer`'s signals, then blend with the
/// `ScoringContext`'s store unless frozen, storeless, or the scorer opts
/// this subject out of blending (`utility_key` returns `None`).
pub fn score<S: Scorer>(scorer: &S, subject: &S::Subject<'_>, ctx: &ScoringContext<'_>) -> Score {
    let signals = scorer.signals(subject);
    let raw = weighted_sum(&signals);

    let (value, blended) = match (ctx.frozen, ctx.store, scorer.utility_key(subject)) {
        (false, Some(store), Some(key)) => (ctx.blend.apply(raw, store, &key), true),
        _ => (raw, false),
    };

    Score {
        value,
        signals,
        blended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_new() {
        let s = Signal::new("test", 0.5, 2.0);
        assert_eq!(s.name, "test");
        assert_eq!(s.value, 0.5);
        assert_eq!(s.weight, 2.0);
    }

    #[test]
    fn test_weighted_sum_empty() {
        assert_eq!(weighted_sum(&[]), 0.0);
    }

    #[test]
    fn test_weighted_sum_single_signal() {
        let signals = vec![Signal::new("a", 75.0, 1.0)];
        assert_eq!(weighted_sum(&signals), 75.0);
    }

    #[test]
    fn test_weighted_sum_multiple_signals() {
        let signals = vec![Signal::new("a", 1.0, 1.0), Signal::new("b", 0.0, 1.0)];
        assert_eq!(weighted_sum(&signals), 0.5);
    }

    #[test]
    fn test_weighted_sum_respects_weight() {
        let signals = vec![Signal::new("a", 1.0, 3.0), Signal::new("b", 0.0, 1.0)];
        // (1.0*3.0 + 0.0*1.0) / 4.0 = 0.75
        assert_eq!(weighted_sum(&signals), 0.75);
    }

    #[test]
    fn test_weighted_sum_zero_total_weight() {
        let signals = vec![Signal::new("a", 1.0, 0.0)];
        assert_eq!(weighted_sum(&signals), 0.0);
    }

    #[test]
    fn test_additive_sum_empty() {
        assert_eq!(additive_sum(&[]), 0.0);
    }

    #[test]
    fn test_additive_sum_stacks_independent_terms() {
        // Unlike weighted_sum, more signals should not dilute the total —
        // each contributes its own value*weight independently.
        let signals = vec![
            Signal::new("concept_type", 1.0, 0.6),
            Signal::new("has_documentation", 1.0, 0.15),
            Signal::new("public_visibility", 1.0, 0.1),
        ];
        assert!((additive_sum(&signals) - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_additive_sum_allows_negative_terms() {
        let signals = vec![
            Signal::new("concept_type", 0.5, 0.6),
            Signal::new("private_visibility", -0.05, 1.0),
            Signal::new("complexity", -0.1, 1.0),
        ];
        assert!((additive_sum(&signals) - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn test_score_as_priority_rounds() {
        let score = Score {
            value: 74.6,
            signals: vec![],
            blended: false,
        };
        assert_eq!(score.as_priority(), 75);
    }

    #[test]
    fn test_score_as_priority_preserves_out_of_range_values() {
        // Lens priorities are documented as "arbitrary integers", not
        // bounded to 0..=100 — the scoring layer must not clamp this away.
        let score = Score {
            value: 150.0,
            signals: vec![],
            blended: false,
        };
        assert_eq!(score.as_priority(), 150);

        let negative = Score {
            value: -10.0,
            signals: vec![],
            blended: false,
        };
        assert_eq!(negative.as_priority(), -10);
    }

    #[test]
    fn test_linear_blend_matches_context_store_formula() {
        let mut store = ContextStore::new();
        store.report_utility("src/main.rs", 0.8, 0.3);

        let blend = LinearBlend;
        let via_blend_policy = blend.apply(90.0, &store, "src/main.rs");
        let via_store_directly = store.blend_priority("src/main.rs", 90) as f32;

        assert_eq!(
            via_blend_policy.round() as i32,
            via_store_directly.round() as i32
        );
    }

    #[test]
    fn test_scoring_context_static_only_has_no_store() {
        let ctx = ScoringContext::static_only();
        assert!(ctx.store.is_none());
        assert!(ctx.frozen);
    }

    // A minimal Scorer used only to exercise the generic `score()` function
    // in isolation from any real production Scorer.
    struct FixedScorer(f32);

    impl Scorer for FixedScorer {
        type Subject<'s> = str;

        fn utility_key(&self, subject: &str) -> Option<String> {
            Some(subject.to_string())
        }

        fn signals(&self, _subject: &str) -> Vec<Signal> {
            vec![Signal::new("fixed", self.0, 1.0)]
        }
    }

    #[test]
    fn test_score_unblended_without_store() {
        let scorer = FixedScorer(42.0);
        let ctx = ScoringContext::static_only();

        let result = score(&scorer, "any/path.rs", &ctx);
        assert_eq!(result.value, 42.0);
        assert!(!result.blended);
        assert_eq!(result.signals.len(), 1);
    }

    #[test]
    fn test_score_blended_with_store() {
        let mut store = ContextStore::new();
        store.report_utility("any/path.rs", 1.0, 1.0);

        let scorer = FixedScorer(50.0);
        let blend = LinearBlend;
        let ctx = ScoringContext {
            store: Some(&store),
            frozen: false,
            blend: &blend,
        };

        let result = score(&scorer, "any/path.rs", &ctx);
        // 50.0*0.7 + 1.0*100.0*0.3 = 65.0
        assert_eq!(result.value, 65.0);
        assert!(result.blended);
    }

    #[test]
    fn test_score_frozen_ignores_store() {
        let mut store = ContextStore::new();
        store.report_utility("any/path.rs", 1.0, 1.0);

        let scorer = FixedScorer(50.0);
        let blend = LinearBlend;
        let ctx = ScoringContext {
            store: Some(&store),
            frozen: true,
            blend: &blend,
        };

        let result = score(&scorer, "any/path.rs", &ctx);
        assert_eq!(result.value, 50.0);
        assert!(!result.blended);
    }

    #[test]
    fn test_score_no_store_present() {
        let scorer = FixedScorer(33.0);
        let ctx = ScoringContext {
            store: None,
            frozen: false,
            blend: &LinearBlend,
        };

        let result = score(&scorer, "any/path.rs", &ctx);
        assert_eq!(result.value, 33.0);
        assert!(!result.blended);
    }
}
