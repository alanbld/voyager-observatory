# Voyager Observatory — Design Proposals (Phase 2/3 items requiring a design pass)

*Produced 2026-07-27 by a dedicated Opus 4.8 design session, scoped to the three roadmap items explicitly flagged as needing design work before execution (2.4, the 2.2 blocker below, 3.3). Read alongside `REVIEW_ROADMAP.md` and `REVIEW_DECISIONS.md`. Nothing here has been implemented — these are proposals for a human to approve before a code session starts.*

---

## 1. PRIMARY — Roadmap 2.4: unified scoring layer

### Current state (verified)

Two subsystems with **zero shared code**, confirming the 2026-07 code survey cited in roadmap 2.4:

- **Lenses** (`src/lenses.rs`): `LensManager::get_static_priority(&self, &Path) -> i32` (lenses.rs:1013) walks `PriorityGroup` globs and returns the highest matching integer 0–100; `get_file_priority` (lenses.rs:992) optionally blends via the store. Input domain = **file path strings**; output = **i32**. Consumed by `apply_token_budget` (vo.rs:3553) to order/drop files. Matching is a hand-rolled glob (`match_pattern`, lenses.rs:1052).
- **Intents** (`src/core/fractal/intent/`): a pipeline `NoiseFilter → RelevanceScorer → ExplorationPlanner` over `ContextLayer` (parsed symbol metadata). `ConceptType::infer(&ContextLayer)` (primitives.rs:56) classifies into 10 buckets from name/signature/return-type/doc/kind; `RelevanceScorer::score_element` (primitives.rs:696) produces **f32 0–1** as `concept_weight*0.6` plus additive doc/visibility/complexity/name factors, weighted per-intent by `RelevanceScorerParams` presets (primitives.rs:599–681). Input domain = **symbol metadata**; output = **f32 + read/skim/skip**. Terminal report; `--explore` returns before `EncoderConfig` is built (vo.rs:2856–2884).

They do **not** share an input type, and forcing one (a `Path`-or-`ContextLayer` union subject) would be a lossy abstraction. What they genuinely share is: (a) combine weighted signals into a ranking score, (b) a per-profile weight table, (c) the desire to fold in the learning store, (d) the same exclusion set. The abstraction should unify **signal aggregation + blend + exclusions**, not classification and not the subject type.

### Proposed shape (new module `src/core/scoring/`)

```rust
/// One factor's contribution. `value` is normalized 0.0..=1.0; `weight` is relative.
#[derive(Debug, Clone)]
pub struct Signal { pub name: &'static str, pub value: f32, pub weight: f32 }

/// Canonical internal score. f32 0..=1 is the one true representation.
#[derive(Debug, Clone)]
pub struct Score { pub value: f32, pub signals: Vec<Signal>, pub blended: bool }
impl Score {
    /// The ONLY place f32 -> lens i32 priority happens; preserves the XML contract.
    pub fn as_priority(&self) -> i32 { (self.value * 100.0).round() as i32 }
}

/// Cross-cutting config shared by every subsystem.
pub struct ScoringContext<'a> {
    pub exclusions: &'a ExclusionSet,     // repo-root-scoped defaults
    pub store: Option<&'a ContextStore>,  // None => static only
    pub frozen: bool,                     // ignore store; deterministic
    pub blend: BlendPolicy,               // decay/exploration — see item 3
}

/// Each subsystem implements this over ITS OWN subject type. No shared subject.
pub trait Scorer {
    type Subject<'s>;
    fn utility_key(&self, s: &Self::Subject<'_>) -> Option<String>; // usually a path
    fn signals(&self, s: &Self::Subject<'_>) -> Vec<Signal>;        // no store, no norm
}

/// Shared aggregation + blend spine (the actual unification).
pub fn score<S: Scorer>(scorer: &S, subject: &S::Subject<'_>, ctx: &ScoringContext<'_>) -> Score {
    let signals = scorer.signals(subject);
    let raw = weighted_sum(&signals);                    // shared normalization
    let (value, blended) = match (ctx.frozen, ctx.store, scorer.utility_key(subject)) {
        (false, Some(store), Some(key)) => (ctx.blend.apply(raw, store, &key), true),
        _ => (raw, false),
    };
    Score { value: value.clamp(0.0, 1.0), signals, blended }
}
```

- Lenses: `impl Scorer for LensManager { type Subject<'s> = &'s Path; }` — `signals()` returns one glob-match `Signal { value: matched_priority/100.0 }`; `utility_key` = path string. `get_file_priority` becomes `score(self, &path, ctx).as_priority()`.
- Intents: `impl Scorer` over a small `ScoredSubject<'s> { layer, vector, concept_type }`; `signals()` emits the existing five factors (primitives.rs:706–781) as explicit `Signal`s, turning `RelevanceScorerParams` into a weight table feeding the same `weighted_sum`. Intents thereby gain store-blending **for free** — extending the moat to `--explore`, which has none today.

Canonical representation is f32 0–1; lenses convert at the edge (`as_priority`), so the integer `<file priority=...>` attribute and stderr values are unchanged.

### Migration plan (incremental, not flag-day)

- **Step A** (additive, zero behavior change): land `src/core/scoring/` with `Signal`/`Score`/`Scorer`/`ScoringContext`/`weighted_sum` and `ExclusionSet`. No callers.
- **Step B**: reimplement `get_static_priority`/`get_file_priority` on `score()` with a single glob signal. The lenses.rs tests pin **exact integers** (e.g. lenses.rs:1270–1282, 1369) — a perfect byte-identical regression net. Pure refactor.
- **Step C**: reimplement `score_element` via `signals()`+`weighted_sum`, keeping the same coefficients. primitives.rs tests pin **orderings** not exact floats (primitives.rs:1582–1651), tolerating negligible drift.
- **Step D** (first behavior change; ship with item 3's `BlendPolicy` review): thread `ScoringContext` through both call sites so intents gain blend and both gain the shared exclusion set.

Each step is independently shippable; lenses and intents adopt the layer one at a time. No rewrite of either domain's logic — only the aggregation/blend/exclusion spine is shared.

### Three concrete fixes (fold in regardless of the abstraction)

1. **Default excludes** `experiments/`, `classic/`, `benches/`, `.llm_archive/` — repo-root-scoped (Q7). Live bug: `IntentExplorer::should_ignore` (explorer.rs:431) strips wildcards to a core string and does `path_str.contains(core)` — that's the any-depth match Q7 explicitly warns against (would hide a legitimately-named nested dir). `ExclusionSet` must anchor to root: strip the `project_root` prefix, then match the **first path component**. Apply in both `IntentExplorer` (explorer.rs:425) and the lens serialization walker (vo.rs:3530).
2. **200-file cap** (`ExplorerConfig::max_files=200`, explorer.rs:56; `build_context` breaks in filesystem/alpha walk order, explorer.rs:380). True relevance ordering needs scoring, which needs parsing — the expensive thing the cap bounds. Pragmatic fix: pre-rank candidates on a **cheap path proxy** (lens static priority + is-not-test) before the parse/score pass, then take top-N. Minimum viable if that's too much for one session: keep the cap but always `eprintln!("note: {N} files skipped by --explore-max-files cap")`.
3. **Unknown→Calculation fallback** (primitives.rs:361–367): a public function matching no heuristic is classified `Calculation`, which is the **max-weight** bucket (1.0) in `business_logic()` (primitives.rs:601). So "we don't know what this is" silently scores as top business logic. Fix: return `ConceptType::Unknown` for unclassified Function/Method (drop the `is_public → Calculation` / `private → Infrastructure` guesses). `Unknown` already carries a neutral 0.5 in every preset (primitives.rs:611, 632, 653, 674), so unknowns become neutral instead of maximal. Keep the struct/enum/trait/const kind fallbacks — those are defensible.

### Must NOT change
- Two separate surfaces (Q2): `--explore` still returns a terminal report before `EncoderConfig`; `--lens` still modifies content. No merged registry.
- MCP JSON shapes: `explore_with_intent` (exploration_path/relevance/decision/concept) and `get_context` (`priority` attr) unchanged.
- The i32 `priority` in Claude-XML and stderr — same integer values per lens (Step B is byte-identical).
- Public presets `RelevanceScorerParams::{business_logic,debugging,security,onboarding}` and `NoiseFilterParams::*` — become weight tables, keep names/semantics.

### Open for a human
- Whether lenses should ever consume `ConceptType` classification (symbol-aware lenses) or stay purely path-based. Recommend path-based — determinism/speed are features; the shared layer unifies aggregation, not classification.
- `RelevanceScorerParams::dimension_weights` (the 64-dim FeatureVector cosine path, `None` everywhere, primitives.rs:579) — wire as a Signal or delete as another callerless capability.

---

## 2. SECONDARY — ContextEngine/skeletonizer duplication (blocking 2.2's remainder)

### Current state (verified)

The "two ContextEngines" is real, and there are actually **three** structural systems:

- **lib.rs:315 `pub struct ContextEngine`** (pure): `process_file_content`/`serialize_processed_file`, wraps free fns `truncate_structure_with_fallback` (lib.rs ~1393–1568). The **struct has zero non-test callers** — only lib.rs tests use it (lib.rs:4576+, 5445). It is *not* what `pm_encoder::core::ContextEngine` resolves to.
- **core/engine.rs:188 `ContextEngine`** (I/O; `Box<dyn FileWalker>`+`Box<dyn Serializer>`; `serialize()`/`zoom()`): re-exported as `pm_encoder::core::ContextEngine` (core/mod.rs:45) and used by the **MCP server** (server/mod.rs:558 get_context, :690 zoom) and vo.rs:3338. Its `process_files` hardcodes `priority = 50` ("TODO: Get from lens manager", engine.rs:339) and skeletonizes via `core::skeleton::Skeletonizer` (parser.rs:124).
- Its free functions notwithstanding, the **CLI's main budget serialize path** is a *third* route: `apply_token_budget` + `serialize_entries_claude_xml_with_report`/`serialize_file_with_format` (vo.rs:3579/3588/3645), which skeletonizes via lib.rs `truncate_structure*` — a regex skeletonizer **distinct from** `core::skeleton::Skeletonizer`.

So: two regex skeletonizers (lib.rs `truncate_structure`, `core::skeleton::Skeletonizer`) plus the AST one in voyager-ast. The roadmap's "one regex skeletonizer" undercounts.

**voyager-ast dormant machinery**: `TreeSitterProvider` (registry.rs:137, `#[allow(dead_code)]`) implements `AstProvider` with `index_project` (registry.rs:175) and `zoom_into` (registry.rs:243) — complete, but `zoom_into` **re-parses raw** (registry.rs:285–291) instead of reusing `registry.parse`. No callers outside voyager-ast tests. `AstBridge` (ast_bridge.rs) is the thin wrapper actually used, and **only by survey** (vo.rs:649,687): `analyze_file`/`extract_stars`/`get_file_summary`, no indexing, no zoom. `ParseCache` (ast_cache.rs) is JSON, md5-keyed, `AstFile`-valued, and serves only survey today.

### Recommendation

**(a) The real engine is `core/engine.rs`.** Delete the lib.rs pure `ContextEngine` **struct** (callerless — the same Q1/Q4/Q9 disease) but **keep its free functions** (`truncate_structure_with_fallback`, the `serialize_*` family) — those are the live CLI path, merely mis-wrapped in a dead struct. Do **not** "repurpose as WASM-only": no `wasm` cfg referencing it was found (flag for human confirmation before deleting); the free functions are already pure and WASM-ready without the struct, so keeping a struct "for WASM" is speculative generality. The duplication worth actually collapsing is the **two regex skeletonizers** — make `core::skeleton::Skeletonizer` the single regex implementation and route the CLI structure-truncation through it (or extract lib.rs's `truncate_structure` into the skeleton module and delete the other). One regex skeletonizer is the precondition for 2.2's "regex only as fallback."

**(b) Extend `AstBridge`; do not promote `TreeSitterProvider`.** `AstBridge` is the live, tested, cache-sharing integration point with the right "telescope not compiler" fallback contract. `TreeSitterProvider` duplicates `registry.parse`, ships a parallel `AstProvider`/`PlanetariumModel`/`MicroscopeModel` object graph nothing consumes, and re-parses raw in `zoom_into`. What zoom actually needs (find declaration by id, `extract_body`, context window, source slice) is ~40 lines added to `AstBridge` as `zoom_into(file, symbol) -> Option<ZoomHit>` reusing `registry.parse` + the adapter's `extract_body` — strictly less code than promotion, and it keeps one parse path.

**(c) Delete `TreeSitterProvider`** (and the `AstProvider` trait + orphaned `PlanetariumModel`/`MicroscopeModel`/`IndexOptions`/`ZoomOptions` types, after verifying census uses `AstBridge`+`CelestialCensus`, not these). Counter-argument weighed: `index_project` is a fuller cross-file model and a head-start for roadmap 3.2's call-graph-aware selection. But it is unused/untested-in-production, 3.2 already has `CallGraphAnalyzer` (used in the server zoom menu), and this repo's own kaizen rule says build indexing behind `AstBridge` driven by a real caller rather than revive dead code. Recommendation stands: delete now.

### Migration sequence

1. **Collapse regex skeletonizers** onto `core::skeleton::Skeletonizer`; route the CLI structure path through it. Existing skeleton + truncation tests hold behavior. Ship (no AST yet).
2. **Delete the dead lib.rs `ContextEngine` struct** (keep free fns). Test-only fallout.
3. **Extend `AstBridge`** with `index_file`/`zoom_into` reusing `registry.parse`+`extract_body`, backed by `ParseCache`. Generalize the cache owner from `run_survey` (vo.rs:694) to a shared `AstBridge`-held instance so serialize/survey/zoom share one on-disk cache (kills the double-parse). Add CLI/server zoom tests including the **zoom-from-subdirectory** case (2.3).
4. **Route `core::engine::ContextEngine::serialize`/`zoom`** to prefer `AstBridge` for supported languages (Rust/Python/TS/TSX/JS, registry.rs:340), regex skeletonizer fallback for the rest — 2.2's remaining step. Optionally behind a one-release `ast-serialize` feature flag for rollback, default-on for the 5 langs. While here, fix engine.rs:339's hardcoded `priority = 50` to consult the item-1 scoring layer.
5. **Delete `TreeSitterProvider`** + orphaned trait/types once `AstBridge` covers zoom.

**Tests**: `TreeSitterProvider`/pure-engine tests are deleted with their types; parsing assertions worth keeping migrate to `AstBridge`; new end-to-end zoom tests replace them at the CLI/server boundary. Only step 4 warrants a feature flag.

**Cache extension**: `ParseCache` is already suitable. Tradeoff to flag: it stores declaration skeletons without bodies (survey doesn't need them); zoom needs bodies via `extract_body` off the tree, which isn't cached. Recommend keeping the cache **body-free** and having zoom re-parse the single target file on demand (one file, cheap) rather than bloating the cache with bodies survey ignores.

### Open for a human
- Confirm no `wasm` feature references the lib.rs `ContextEngine` struct before deletion.
- Confirm `PlanetariumModel`/`MicroscopeModel` have no consumer outside `TreeSitterProvider` tests before deleting them.

---

## 3. TERTIARY — report_utility decay/exploration (roadmap 3.3)

### Current state (verified)

`FileUtility::update` is EMA (store.rs:69): `score = α·session + (1-α)·score`, α=0.3. `blend_priority` (store.rs:200): `final = static·0.7 + learned·100·0.3`. `LensManager::get_file_priority` (lenses.rs:1001) calls it only when `store` is `Some` and not frozen. The **write** path works (server:844–849, vo.rs:2824–2837 load→update→save to `.pm_encoder/context_store.json`); the **read** path is dead — production `LensManager::new()` (vo.rs:3505) never gets a store (Q9). Burial risk: one early `0.0` decays a file toward 0, the blended priority drops, the file stops being selected, so it never earns utility again → **permanent burial**.

### Proposal (localized to store.rs + the `BlendPolicy` from item 1)

**1. Time-decay of the learned signal toward neutral (recency):**
```rust
pub const NEUTRAL: f64 = 0.5;
pub const HALF_LIFE_DAYS: f64 = 30.0;
impl FileUtility {
    pub fn effective_score(&self, now: DateTime<Utc>) -> f64 {
        let decay = 0.5f64.powf(self.age_days(now) / HALF_LIFE_DAYS); // 1 fresh → 0 stale
        NEUTRAL + (self.score - NEUTRAL) * decay
    }
}
```
A file scored 0.0 thirty days ago reads back as 0.25, ~0.375 at 60d, asymptotically 0.5 — it un-buries itself instead of staying pinned at 0.

**2. Confidence-weighted blend (cold-start / exploration):** few observations shouldn't override static priority.
```rust
fn confidence(n: u32) -> f64 { let n = n as f64; n / (n + K) }   // K≈3
pub const MAX_LEARNED_WEIGHT: f64 = 0.4;

pub fn blend_priority_v2(&self, path: &str, static_priority: i32, now: DateTime<Utc>) -> i32 {
    let Some(u) = self.get_utility(path) else { return static_priority }; // unseen => pure static
    let learned = u.effective_score(now);
    let c = confidence(u.access_count) * MAX_LEARNED_WEIGHT;
    (static_priority as f64 * (1.0 - c) + learned * 100.0 * c).round() as i32
}
```
This also fixes a current oddity: today an **unseen** file gets `0.7·static + 0.5·100·0.3 = 0.7·static + 15` (store.rs:200 with default 0.5) — the learning layer perturbs files it has never observed, pulling everything toward 65. v2 returns **pure static** for unseen files (c=0), preserving lens determinism until real evidence exists.

**3. Exploration floor (optional):** item 1 already guarantees no permanent burial. For a stronger guarantee add `last_offered` and a periodic bonus (`if days_since_offered > EXPLORE_INTERVAL { += EXPLORE_BONUS }`). Recommend shipping **1+2 first** (pure functions of existing fields) and treating 3 as optional, because it requires the selection path to write the store on **every** serialize (write-amplification + a `--frozen` determinism problem), not just on `report_utility`.

### Where it plugs in
- Replace `ContextStore::blend_priority` (store.rs:200) with the `now`-taking v2; keep the old signature as a thin `now = Utc::now()` wrapper for the existing test call sites (store.rs:475–516).
- This **is** item 1's `BlendPolicy::apply(raw_static, store, key) -> f32` — items 1 and 3 meet here; the decay/blend is the single shared read-back path for both lenses and (newly) intents.
- **Wire the read side** (the whole point): at vo.rs:3505 replace `LensManager::new()` with load-store-if-exists + `with_store`, honoring `--frozen` (`set_frozen`) and a new `--no-learning` escape hatch; mirror in server get_context.
- **Determinism guard**: `effective_score` depends on wall-clock `now`, so `--frozen` MUST bypass decay entirely (static only) to stay reproducible. State this explicitly in the flag docs.

Defaults: NEUTRAL 0.5, HALF_LIFE_DAYS 30, K 3, MAX_LEARNED_WEIGHT 0.4 (vs today's flat 0.3).

### Open for a human
- Exact constants (30-day half-life, K=3, cap 0.4) — need real telemetry; propose as defaults.
- Whether to persist `last_offered` (item 3) given frozen-determinism and write-amplification cost.
- Store-dir naming: `.pm_encoder/context_store.json` (store.rs:258) vs the cache's `.voyager/…` (ast_cache.rs:25) — the incomplete rename (roadmap 1.8); pick one before wiring so users don't accumulate two dotdirs.
- Utility store is per-**file**; intents score per-**symbol**. Simplest is to have intents blend at file granularity against the same per-file store; flag whether that's acceptable.

---

## Critical files for implementation
- `rust/src/lenses.rs`
- `rust/src/core/fractal/intent/primitives.rs`
- `rust/src/core/engine.rs`
- `rust/src/core/ast_bridge.rs`
- `rust/src/core/store.rs`
