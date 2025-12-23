# Intent-Driven Exploration: The "Smart Lens" Vision

> Brainstorming session with DeepSeek - December 2024
>
> This document captures the vision for evolving pm_encoder's Fractal Context Engine
> toward **intent-driven code exploration** - helping developers decide what to read
> and what to skip based on their current goals.

## The Core Insight

What experienced developers do naturally:
1. **Scan** code quickly
2. **Recognize intent** ("this is test code", "this is boilerplate", "this is business logic")
3. **Decide** whether to read deeply or move on
4. **Remember** where things are for later

The fractal approach should **replicate this cognitive process** for the user.

## The "Cognitive Load" Problem

When exploring a new codebase:
- **Bad tools**: Make you read every line (cognitive overload)
- **Good tools**: Show you **what matters for your current goal** (cognitive efficiency)

Target: **"Show me the parts that match my current intent, hide the rest."**

---

## Intent-Driven Lenses

Instead of generic lenses, we need **intent-specific lenses**:

```rust
enum ExplorationIntent {
    // "I want to understand the business logic"
    UnderstandBusinessRules,

    // "I'm looking for bugs"
    FindBugs,

    // "I need to add a feature"
    ModifyFeature(String),  // e.g., "ModifyFeature('payment-processing')"

    // "I'm refactoring"
    Refactor(String),       // e.g., "Refactor('error-handling')"

    // "I'm onboarding"
    LearnCodebase,

    // "I'm debugging a specific issue"
    DebugIssue(String),     // e.g., "DebugIssue('memory-leak')"

    // "I'm writing tests"
    WriteTests,

    // "I'm reviewing security"
    SecurityReview,

    // "I'm estimating migration effort"
    EstimateMigration,
}

struct IntentLens {
    intent: ExplorationIntent,
    focus_areas: Vec<FocusArea>,
    priority_weights: HashMap<SemanticDimension, f32>,
    noise_filters: Vec<NoiseFilter>,
}

impl SemanticLens for IntentLens {
    fn apply(&self, usg: &UniversalSemanticGraph) -> TransformedView {
        // Filter out "noise" for this intent
        let filtered = self.remove_noise(usg);

        // Highlight what matters for this intent
        let highlighted = self.highlight_relevant(&filtered);

        // Re-weight dimensions based on intent
        let reweighted = self.reweight_dimensions(highlighted);

        // Suggest exploration path
        let suggestions = self.suggest_exploration_path(&reweighted);

        TransformedView::new()
            .with_graph(reweighted)
            .with_suggestions(suggestions)
            .with_intent_metadata(self.intent.clone())
    }
}
```

---

## Concrete Examples

### Example 1: "I just want the business logic"

```rust
let business_logic_lens = IntentLens {
    intent: ExplorationIntent::UnderstandBusinessRules,
    focus_areas: vec![
        FocusArea::BusinessCalculations,
        FocusArea::ValidationLogic,
        FocusArea::DecisionPoints,
        FocusArea::CoreAlgorithms,
    ],
    priority_weights: hashmap!{
        SemanticDimension::BusinessLogic => 1.0,
        SemanticDimension::DataTransformation => 0.8,
        SemanticDimension::ErrorHandling => 0.3,  // Less important
        SemanticDimension::Boilerplate => 0.0,    // Hide completely
        SemanticDimension::Testing => 0.0,        // Hide completely
    },
    noise_filters: vec![
        NoiseFilter::ExcludeTestingCode,
        NoiseFilter::ExcludeBoilerplate,
        NoiseFilter::ExcludeInfrastructure,
        NoiseFilter::ExcludeLoggingDebugging,
    ],
};
```

### Example 2: "I'm debugging a memory leak"

```rust
let debug_memory_lens = IntentLens {
    intent: ExplorationIntent::DebugIssue("memory-leak".to_string()),
    focus_areas: vec![
        FocusArea::ResourceAllocation,
        FocusArea::ResourceCleanup,
        FocusArea::CachingPatterns,
        FocusArea::LargeDataStructures,
    ],
    priority_weights: hashmap!{
        SemanticDimension::ResourceManagement => 1.0,
        SemanticDimension::Performance => 0.9,
        SemanticDimension::Caching => 0.8,
        SemanticDimension::ErrorHandling => 0.5,
        SemanticDimension::BusinessLogic => 0.1,  // Not important right now
    },
    noise_filters: vec![
        NoiseFilter::ExcludeTestingCode,
        NoiseFilter::ExcludeDocumentation,
        NoiseFilter::FocusOnRecentChanges,  // If we have git history
    ],
};
```

---

## The "Stop Reading" Decision Interface

```rust
enum StopReadingDecision {
    ReadDeeply { reason: String, expected_value: f32 },
    Skim { key_points: Vec<String>, time_limit: Duration },
    Skip { reason: String, alternative: Option<String> },
    BookmarkForLater { context_needed: Vec<String> },
}

impl CodeExplorer {
    fn should_i_read_this(&self, node: &SemanticNode) -> StopReadingDecision {
        let relevance = self.calculate_relevance(node);
        let complexity = self.estimate_complexity(node);
        let dependencies = self.count_dependencies(node);

        match (relevance, complexity, dependencies) {
            (high, low, _) if high > 0.8 =>
                StopReadingDecision::ReadDeeply {
                    reason: "Core to your current intent".to_string(),
                    expected_value: high,
                },
            (medium, _, _) if medium > 0.5 =>
                StopReadingDecision::Skim {
                    key_points: self.extract_key_points(node),
                    time_limit: Duration::from_secs(60),
                },
            (low, high, _) if low < 0.3 =>
                StopReadingDecision::Skip {
                    reason: "Low relevance, high complexity".to_string(),
                    alternative: self.find_alternative(node),
                },
            // ... other patterns
        }
    }
}
```

---

## Cognitive Compression Pipeline

```
Raw Code (100,000 lines)
    ↓
Semantic Understanding (10,000 concepts)
    ↓
Intent Filtering (1,000 relevant concepts)
    ↓
Exploration Path (100 concepts to examine)
    ↓
Summary (10 key insights)
```

---

## The "Smart Scan" CLI Workflow

```bash
# Command: "Show me the business logic"
pm_encoder explore --intent business-logic --project ./src

# Output:
# ┌─────────────────────────────────────────────────────┐
# │ BUSINESS LOGIC EXPLORATION                          │
# │ Found 247 business logic nodes (87% of codebase)    │
# │                                                     │
# │ ⭐ HIGHLIGHTS:                                      │
# │   • payment_processing.rs:calculate_tax()           │
# │   • inventory.rs:validate_stock_levels()            │
# │   • pricing.rs:apply_discounts()                    │
# │                                                     │
# │ ⚡ RECOMMENDED PATH:                                │
# │   1. Start with payment_processing.rs (core logic)  │
# │   2. Then inventory.rs (data validation)            │
# │   3. Skip: test_*.rs (testing code - hidden)        │
# │   4. Skip: logging.rs (infrastructure - hidden)     │
# │                                                     │
# │ 🎯 ESTIMATED READING TIME: 45 minutes               │
# │ (vs 4 hours if reading everything)                  │
# └─────────────────────────────────────────────────────┘
```

---

## Persona-Based Explanations

```rust
enum Persona {
    ProductManager,      // Focus: business logic, user flows
    QaEngineer,          // Focus: edge cases, test coverage
    NewDeveloper,        // Focus: architecture, key patterns
    SecurityAnalyst,     // Focus: vulnerabilities, data flows
    PerformanceEngineer, // Focus: bottlenecks, optimizations
}
```

Each persona gets:
- Different detail levels
- Different focus areas
- Different explanation styles

---

## Integration with Fractal Context Engine

```
Fractal Navigation (WHERE to look)
    +
Intent Lenses (WHAT to look for)
    +
Smart Bookmarks (HOW to navigate)
    =
Intent-Driven Exploration
```

---

## Proposed Implementation Phases

### Phase 1: Basic Intent Recognition
1. Add `ExplorationIntent` enum
2. Create 2-3 simple intent lenses (business logic, testing, infrastructure)
3. Show simple "relevance scores" in fractal navigation

### Phase 2: Smart Filtering
1. Implement noise filtering based on intent
2. Add "skip this" suggestions
3. Create exploration path recommendations

### Phase 3: Adaptive Exploration
1. Learn from user interactions
2. Adjust recommendations based on what they skip/read
3. Create personalized exploration paths

---

## The Vision Statement

> We're not building a "code analyzer" or "visualizer" — we're building a
> **cognitive augmentation tool** that helps developers **decide what to read
> and what to skip** based on their current goals.

This is the difference between:
- **A map** (shows everything)
- **A GPS with traffic alerts** (shows what matters for your current trip)

We're building the GPS for code exploration.

---

## Key Design Principles

1. Each intent lens is **independent**
2. They all work on the **same semantic graph**
3. **No duplication** across languages
4. Easy to **add new intents**
5. The fractal navigation **stays the same** (WHERE stays, WHAT changes)

---

*Document saved from brainstorming session - ready for implementation planning*
