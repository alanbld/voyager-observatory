# Week 2 Intent Foundation Sprint - Technical Specification

> One-pager for alignment between Claude (implementation) and DeepSeek (vision)
>
> **Status:** SPEC-FIRST, TDD
> **Days:** 5-7 of Week 2
> **Branch:** `experiment/lsp-poc` (continuing)

---

## What We Have (Ground Truth)

```
pm_encoder/rust/src/core/fractal/
├── layers.rs          # ContextLayer, ZoomLevel, LayerContent (25K)
├── context.rs         # FractalContext, RelationshipGraph (23K)
├── builder.rs         # FractalContextBuilder (68K)
├── navigation.rs      # FractalNavigator (35K)
├── relationships/     # CallGraph, CallExtractor (Day 1)
│   ├── mod.rs
│   └── call_graph.rs
└── clustering/        # K-means, DBSCAN, ShellPatterns (Day 2)
    ├── mod.rs         # ClusterEngine, SemanticCluster
    ├── vectorizer.rs  # SymbolVectorizer, FeatureVector (64-dim)
    ├── algorithms.rs  # KMeans, DBSCAN (pure Rust)
    └── shell_patterns.rs  # ShellPatternRecognizer (12 types)
```

### Key Assets Already Built

| Asset | What It Does | Location |
|-------|--------------|----------|
| `FeatureVector` | 64-dim semantic encoding | `clustering/vectorizer.rs` |
| `SemanticCluster` | Groups similar code | `clustering/mod.rs` |
| `ShellPatternType` | 12 pattern categories | `clustering/shell_patterns.rs` |
| `CallGraph` | Function relationships | `relationships/call_graph.rs` |
| `RelationshipGraph` | General node/edge graph | `context.rs` |
| MCP Server | `get_context`, `zoom`, `session_*` | `src/mcp/` |

---

## What We Will Build (Days 5-7)

```
pm_encoder/rust/src/core/fractal/
└── intent/                    # NEW MODULE
    ├── mod.rs                 # IntentLens trait, ExplorationIntent enum
    ├── lenses.rs              # BusinessLogicLens, InfrastructureLens, MigrationLens
    ├── decisions.rs           # ReadingDecision, StopReadingEngine
    └── explorer.rs            # IntentExplorer (orchestrates lens + decisions)
```

---

## Day 5: IntentLens Foundation

### Specification

```rust
// src/core/fractal/intent/mod.rs

/// Developer exploration intent - what are they trying to accomplish?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExplorationIntent {
    /// "I want to understand the business logic"
    BusinessLogic,
    /// "I'm debugging an issue"
    Debugging,
    /// "I'm onboarding to this codebase"
    Onboarding,
    /// "I'm reviewing for security issues"
    SecurityReview,
    /// "I'm estimating migration effort"
    MigrationAssessment,
}

/// A lens transforms how we see the fractal context based on intent
pub trait IntentLens: Send + Sync {
    /// Name of this lens for display
    fn name(&self) -> &str;

    /// Calculate relevance score (0.0 = irrelevant, 1.0 = critical)
    fn relevance(&self, layer: &ContextLayer, vector: &FeatureVector) -> f32;

    /// Dimension weights for this intent (reweights the 64-dim vector)
    fn dimension_weights(&self) -> &[f32; 64];

    /// Which shell pattern types are relevant for this intent?
    fn relevant_patterns(&self) -> &[ShellPatternType];
}

/// Result of applying a lens to a fractal context
pub struct FilteredView<'a> {
    pub intent: ExplorationIntent,
    pub layers: Vec<(&'a ContextLayer, f32)>,  // (layer, relevance_score)
    pub clusters: Vec<&'a SemanticCluster>,
    pub total_layers: usize,
    pub filtered_count: usize,
}
```

### TDD Tests (Write First)

```rust
// tests/intent_lens_tests.rs

#[test]
fn test_business_logic_lens_filters_tests() {
    // Setup: Create context with mixed business logic and test code
    let ctx = create_mixed_context();
    let vectors = vectorize_context(&ctx);

    let lens = BusinessLogicLens::new();
    let view = lens.apply(&ctx, &vectors);

    // Test files should have low relevance
    for (layer, score) in &view.layers {
        if layer.name().starts_with("test_") {
            assert!(score < &0.3, "Test code should have low relevance");
        }
    }

    // Business logic should have high relevance
    let business_layers: Vec<_> = view.layers.iter()
        .filter(|(l, s)| *s > 0.7)
        .collect();
    assert!(!business_layers.is_empty(), "Should find business logic");
}

#[test]
fn test_lens_dimension_weights_sum_to_one() {
    let lens = BusinessLogicLens::new();
    let weights = lens.dimension_weights();
    let sum: f32 = weights.iter().sum();
    assert!((sum - 1.0).abs() < 0.01, "Weights should sum to ~1.0");
}

#[test]
fn test_lens_respects_shell_patterns() {
    let lens = MigrationLens::new();
    let relevant = lens.relevant_patterns();

    // Migration lens should care about deployment and automation
    assert!(relevant.contains(&ShellPatternType::Deployment));
    assert!(relevant.contains(&ShellPatternType::Automation));

    // But not about testing patterns
    assert!(!relevant.contains(&ShellPatternType::Testing));
}
```

---

## Day 6: Reading Decisions Engine

### Specification

```rust
// src/core/fractal/intent/decisions.rs

/// The "Stop Reading" decision for a code element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadingDecision {
    /// Read this carefully - it's core to your goal
    ReadDeeply {
        reason: String,
        estimated_minutes: u32,
        key_points: Vec<String>,
    },
    /// Skim this - get the gist, don't dive deep
    Skim {
        focus_on: Vec<String>,
        time_limit_seconds: u32,
    },
    /// Skip this entirely - not relevant to your goal
    Skip {
        reason: String,
        come_back_if: Option<String>,
    },
    /// Save for later - you'll need context first
    Bookmark {
        prerequisite: String,
        when: String,
    },
}

/// Engine that produces reading decisions based on intent + context
pub struct StopReadingEngine {
    intent: ExplorationIntent,
    relevance_threshold: f32,  // Below this = Skip
    skim_threshold: f32,       // Between this and relevance = Skim
}

impl StopReadingEngine {
    pub fn decide(
        &self,
        layer: &ContextLayer,
        relevance: f32,
        complexity: f32,
        centrality: f32,
    ) -> ReadingDecision {
        // Rule-based heuristics (not ML - keep it simple)
        match (relevance, complexity, centrality) {
            (r, _, c) if r > 0.8 && c > 0.5 => ReadingDecision::ReadDeeply { .. },
            (r, _, _) if r > 0.5 => ReadingDecision::Skim { .. },
            (r, _, _) if r < 0.2 => ReadingDecision::Skip { .. },
            _ => ReadingDecision::Bookmark { .. },
        }
    }
}
```

### TDD Tests

```rust
#[test]
fn test_high_relevance_high_centrality_means_read_deeply() {
    let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
    let layer = create_core_business_layer();

    let decision = engine.decide(&layer, 0.9, 0.5, 0.8);

    assert!(matches!(decision, ReadingDecision::ReadDeeply { .. }));
}

#[test]
fn test_low_relevance_means_skip() {
    let engine = StopReadingEngine::new(ExplorationIntent::BusinessLogic);
    let layer = create_test_helper_layer();

    let decision = engine.decide(&layer, 0.1, 0.2, 0.1);

    assert!(matches!(decision, ReadingDecision::Skip { .. }));
}

#[test]
fn test_medium_relevance_means_skim() {
    let engine = StopReadingEngine::new(ExplorationIntent::Debugging);
    let layer = create_logging_layer();

    let decision = engine.decide(&layer, 0.5, 0.3, 0.4);

    assert!(matches!(decision, ReadingDecision::Skim { .. }));
}
```

---

## Day 7: MCP Integration + CLI

### Specification

```rust
// Extend existing MCP server in src/mcp/tools.rs

/// New MCP tool: Intent-driven exploration
#[derive(Debug, Serialize, Deserialize)]
pub struct ExploreWithIntentParams {
    pub path: String,
    pub intent: String,  // "business-logic", "debugging", "security", "migration"
    #[serde(default)]
    pub detail_level: String,  // "summary" (default), "detailed", "full"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorationResult {
    pub intent: String,
    pub summary: String,
    pub total_elements: usize,
    pub relevant_elements: usize,
    pub estimated_reading_minutes: u32,
    pub recommended_path: Vec<ExplorationStep>,
    pub key_insights: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorationStep {
    pub path: String,
    pub symbol: Option<String>,
    pub decision: String,  // "read", "skim", "skip"
    pub reason: String,
    pub estimated_minutes: u32,
}

// CLI extension
/// pm_encoder explore --intent business-logic ./src
#[derive(clap::Args)]
pub struct ExploreArgs {
    /// Path to explore
    pub path: PathBuf,

    /// Exploration intent
    #[arg(short, long)]
    pub intent: String,

    /// Detail level: summary, detailed, full
    #[arg(short, long, default_value = "summary")]
    pub detail: String,
}
```

### TDD Tests

```rust
#[test]
fn test_explore_with_intent_returns_filtered_results() {
    let result = explore_with_intent(
        "./tests/fixtures/mixed_project",
        "business-logic",
        "summary",
    ).unwrap();

    assert!(result.relevant_elements < result.total_elements);
    assert!(result.estimated_reading_minutes > 0);
    assert!(!result.recommended_path.is_empty());
}

#[test]
fn test_exploration_result_has_valid_decisions() {
    let result = explore_with_intent(
        "./tests/fixtures/mixed_project",
        "debugging",
        "detailed",
    ).unwrap();

    for step in &result.recommended_path {
        assert!(["read", "skim", "skip"].contains(&step.decision.as_str()));
        assert!(!step.reason.is_empty());
    }
}

#[test]
fn test_cli_explore_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "explore", "--intent", "business-logic", "./src"])
        .output()
        .expect("Failed to run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("relevant"));
    assert!(stdout.contains("minutes"));
}
```

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER INTERFACE                            │
│  CLI: pm_encoder explore --intent X                             │
│  MCP: explore_with_intent tool                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     INTENT LAYER (NEW)                          │
│  ┌─────────────┐  ┌──────────────────┐  ┌───────────────────┐  │
│  │ IntentLens  │  │ StopReadingEngine│  │ IntentExplorer    │  │
│  │ (filter)    │  │ (decide)         │  │ (orchestrate)     │  │
│  └─────────────┘  └──────────────────┘  └───────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  EXISTING FRACTAL ENGINE                        │
│  ┌─────────────┐  ┌──────────────────┐  ┌───────────────────┐  │
│  │ Clustering  │  │ Call Graph       │  │ Shell Patterns    │  │
│  │ (K-means)   │  │ (petgraph)       │  │ (12 types)        │  │
│  └─────────────┘  └──────────────────┘  └───────────────────┘  │
│  ┌─────────────┐  ┌──────────────────┐  ┌───────────────────┐  │
│  │ Vectorizer  │  │ FractalContext   │  │ Navigator         │  │
│  │ (64-dim)    │  │ (layers/graph)   │  │ (zoom/pan)        │  │
│  └─────────────┘  └──────────────────┘  └───────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Success Criteria

### Day 5 Complete When:
- [ ] `IntentLens` trait defined and documented
- [ ] `BusinessLogicLens` passes 3+ tests
- [ ] `FilteredView` correctly filters by relevance threshold
- [ ] Tests: 10+ new tests passing

### Day 6 Complete When:
- [ ] `ReadingDecision` enum with all 4 variants
- [ ] `StopReadingEngine` produces correct decisions
- [ ] Heuristics are rule-based (no ML)
- [ ] Tests: 10+ new tests passing

### Day 7 Complete When:
- [ ] `explore_with_intent` MCP tool works
- [ ] CLI `explore` command produces human-readable output
- [ ] Integration test with real project passes
- [ ] Commit: `feat(fractal): implement intent-driven exploration (Week 2 Day 5-7)`

---

## Key Design Decisions

1. **Extend, don't replace**: IntentLens works ON TOP of existing FractalContext
2. **Compose existing assets**: Uses FeatureVector, ShellPatternType, CallGraph
3. **Rule-based first**: No ML in Week 2 - simple heuristics only
4. **MCP-native**: Primary interface is MCP tool for LLM consumption
5. **Human-usable**: CLI produces formatted, actionable output

---

## DeepSeek Alignment Notes

| DeepSeek Vision | Our Implementation | Rationale |
|-----------------|-------------------|-----------|
| Universal Semantic Graph | `FilteredView` on `FractalContext` | USG is a view transformation, not new structure |
| SemanticDimension enum | `ShellPatternType` + weighted `FeatureVector` | Already have 12 discrete + 64 continuous |
| New `src/semantic_graph/` | `src/core/fractal/intent/` | Maintain module coherence |
| Custom SemanticNode | `ContextLayer` + `FeatureVector` tuple | Compose existing types |
| "Cognitive prosthesis" | "Intent-driven exploration" | Same vision, grounded implementation |

---

*Spec version: 1.0 | Date: 2024-12-23 | Ready for implementation*
