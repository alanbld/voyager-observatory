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

- **Step A** ✅ done (`0d97938`, 2026-07-28) — landed `src/core/scoring/` with `Signal`/`Score`/`Scorer`/`ScoringContext`/`BlendPolicy`/`LinearBlend`/`weighted_sum`. One refinement made during implementation: `Score`/`Signal` are domain-agnostic rather than a hardcoded 0.0..=1.0 scale (see the note below) — `ExclusionSet` (Step D) not built yet, deferred until intents actually need it.
- **Step B** ✅ done (`0d97938`) — `LensManager` implements `Scorer` (`type Subject<'s> = Path`); `get_file_priority` now calls `scoring::score(self, path, &ctx).as_priority()` instead of hand-rolling the blend. `get_static_priority` (the glob-matching itself) is untouched. All 68 existing lenses tests pass unchanged, including the blend-specific ones (`test_priority_blend_high_utility`, `test_priority_blend_low_utility`, `test_frozen_mode_ignores_store`) — confirmed byte-identical, not just structurally similar.
- **Step C** (open): reimplement `score_element` via `signals()`+`weighted_sum`, keeping the same coefficients. primitives.rs tests pin **orderings** not exact floats (primitives.rs:1582–1651), tolerating negligible drift.
- **Step D** (open, first real behavior change; ship with item 3's `BlendPolicy` review): thread `ScoringContext` through the intents call site too so intents gain blend and both surfaces gain a shared exclusion set.

Each step is independently shippable; lenses and intents adopt the layer one at a time. No rewrite of either domain's logic — only the aggregation/blend spine is shared.

**Refinement to the original design (found during Step A implementation)**: `PriorityGroup::priority`'s own doc comment says "arbitrary integers supported," not bounded to 0-100. The original design specified `Score`/`Signal` as a hardcoded 0.0..=1.0 domain with `as_priority() = round(value*100)` — that would silently clamp/lose precision for any out-of-range custom lens priority. `Signal`/`Score` now carry whatever domain the producing `Scorer` uses (lenses: priority-scale, unclamped; intents: 0.0..=1.0), and `as_priority()` is a plain `round()` with no forced `*100`. This only matters for a currently-untested edge case (no built-in lens or test exceeds 0-100), but the fix costs nothing and avoids quietly breaking a documented contract.

### Three concrete fixes (fold in regardless of the abstraction) — ✅ all done (2026-07-28)

1. **Default excludes** `experiments/`, `classic/`, `benches/`, `.llm_archive/` — repo-root-scoped (Q7). **Done** (`520028f`): added `ExplorerConfig::root_ignore_dirs`, matched only against the first path component under the project root, separate from the existing any-depth `ignore_patterns`. Also found and fixed a more severe, previously-unknown bug during live testing (`0e694ed`): vo.rs's `--explore` config construction did `ignore_patterns: cli.exclude.clone()` as a bare struct-literal field, which *replaces* rather than extends — silently discarding even the pre-existing `node_modules/**`/`target/**`/`.git/**` defaults whenever `--exclude` wasn't passed (the common case). Confirmed live: `--explore` on this repo was pulling in tree-sitter/git2 vendored C sources from `target/` as "relevant business logic."
2. **200-file cap** — **Done** (`520028f`), via the "minimum viable" path rather than pre-ranking: `build_context` no longer `break`s at the cap, it keeps walking (cheaply — metadata/extension checks only, no file reads) so it can report an exact `"note: N files skipped by --explore-max-files cap"` instead of silently truncating in filesystem order. True relevance-ordered pre-ranking is still open if wanted later.
3. **Unknown→Calculation fallback** — **Done** (`b3bf6b9`): `ConceptType::infer`'s symbol-kind fallback no longer guesses; unclassified functions/methods fall through to `Unknown` (neutral 0.5 weight) instead of `Calculation` (1.0, the max-weight bucket in `business_logic()`). Verified live against real project source (sensible re-ranking, `Dominant concept type: Unknown` now shows honestly).

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

### ⚠️ CORRECTION (2026-07-27, verified during execution — supersedes the original §2 below)

**The original recommendation to delete the lib.rs `ContextEngine` struct is WRONG. Do not do this.**

The design pass concluded the lib.rs struct had "zero non-test callers" and was safe to delete. That was checked against normal (non-feature-gated) builds only. It is in fact the live entry point for the **WASM kernel**:

- `#[cfg(feature = "wasm")]` module `wasm` (lib.rs:5369) exposes `#[wasm_bindgen] pub fn wasm_serialize(...)` (lib.rs:5414), which calls `ContextEngine::new(config)`/`ContextEngine::with_lens(...)` (lib.rs:5445-5449) and `.generate_context(&files)` (lib.rs:5452) — the exact struct/method chain the design pass proposed deleting.
- `rust/test_wasm_full.mjs` exercises this exact function (`wasmModule.wasm_serialize(...)`) end-to-end against a `wasm-pack`-built package.
- `.github/workflows/voyager-release.yml` has a real CI job (`build-wasm`, line 129) that runs `wasm-pack build --target nodejs --features wasm` and ships the result as a release artifact (`voyager-wasm-nodejs.tar.gz`). This is shipped, released functionality, not dead code.

**Corrected understanding**: this isn't "one dead struct, one real struct" — it's **two structs serving two genuinely different consumers**: `lib.rs::ContextEngine` is the pure, I/O-free library API for embedding (WASM/Node, and per its doc comment potentially PyO3), while `core::engine::ContextEngine` is the I/O-performing engine for the CLI/MCP server. Same name, different module, different audience — a naming/clarity problem (worth a rename to disambiguate, e.g. `PureContextEngine` or moving one under a `wasm`-specific name), not a delete-one-and-keep-the-other problem.

**Consequence for the skeletonizer consolidation below**: `truncate_structure*` (lib.rs) is not just "the CLI's regex skeletonizer" — it is *also* what the WASM kernel uses via `ContextEngine::process_file_content` (lib.rs:371). Any future unification with `core::skeleton::Skeletonizer` must preserve output for the WASM/embedding consumer, not only the CLI/MCP one. This raises the bar for that consolidation from "delete a duplicate" to "prove behavioral parity across two independent consumers first" — treat it as needing its own scoped design/testing pass (e.g. golden-file snapshots of both skeletonizers' output across a fixture corpus) before any merge attempt, not a mechanical refactor.

The `TreeSitterProvider` recommendation below (delete, extend `AstBridge` instead) is unaffected by this correction and still verified independently — it has no wasm or other hidden caller.

### Current state (verified)

The "two ContextEngines" is real, and there are actually **three** structural systems — but see the correction above before acting on any of this:

- **lib.rs:315 `pub struct ContextEngine`** (pure, WASM kernel entry point — see correction above, NOT dead): `process_file_content`/`serialize_processed_file`, wraps free fns `truncate_structure_with_fallback` (lib.rs ~1393–1568).
- **core/engine.rs:188 `ContextEngine`** (I/O; `Box<dyn FileWalker>`+`Box<dyn Serializer>`; `serialize()`/`zoom()`): re-exported as `pm_encoder::core::ContextEngine` (core/mod.rs:45) and used by the **MCP server** (server/mod.rs:558 get_context, :690 zoom) and vo.rs:3338. Its `process_files` hardcodes `priority = 50` ("TODO: Get from lens manager", engine.rs:339) and skeletonizes via `core::skeleton::Skeletonizer` (parser.rs:124).
- Its free functions notwithstanding, the **CLI's main budget serialize path** is a *third* route: `apply_token_budget` + `serialize_entries_claude_xml_with_report`/`serialize_file_with_format` (vo.rs:3579/3588/3645), which skeletonizes via lib.rs `truncate_structure*` — a regex skeletonizer **distinct from** `core::skeleton::Skeletonizer`, and (per the correction above) also shared with the WASM kernel.

So: two regex skeletonizers (lib.rs `truncate_structure`, `core::skeleton::Skeletonizer`) plus the AST one in voyager-ast. The roadmap's "one regex skeletonizer" undercounts.

**voyager-ast dormant machinery**: `TreeSitterProvider` (registry.rs:137, `#[allow(dead_code)]`) implements `AstProvider` with `index_project` (registry.rs:175) and `zoom_into` (registry.rs:243) — complete, but `zoom_into` **re-parses raw** (registry.rs:285–291) instead of reusing `registry.parse`. No callers outside voyager-ast tests, no feature-gated callers either (checked). `AstBridge` (ast_bridge.rs) is the thin wrapper actually used, and **only by survey** (vo.rs:649,687): `analyze_file`/`extract_stars`/`get_file_summary`, no indexing, no zoom. `ParseCache` (ast_cache.rs) is JSON, md5-keyed, `AstFile`-valued, and serves only survey today.

### Recommendation (original — items (a) below is superseded by the correction above; (b)/(c) still stand)

**(a) SUPERSEDED — see the correction at the top of this section.** `lib.rs::ContextEngine` is the live WASM kernel entry point (shipped via CI in `voyager-release.yml`'s `build-wasm` job) — it must **not** be deleted. `core/engine.rs::ContextEngine` remains the real engine for the CLI/MCP path; the two coexist for different consumers (embed vs. CLI/MCP) and should be disambiguated by naming/docs, not merged. The duplication actually worth collapsing is still the **two regex skeletonizers** (`core::skeleton::Skeletonizer` vs. lib.rs `truncate_structure*`) — but per the correction, `truncate_structure*` also backs the WASM kernel, so any unification must preserve output for both the WASM/embedding consumer and the CLI/MCP consumer. Scope that as its own pass with before/after golden-file comparisons across both call paths; do not attempt it as a quick mechanical merge.

**(b) Extend `AstBridge`; do not promote `TreeSitterProvider`.** `AstBridge` is the live, tested, cache-sharing integration point with the right "telescope not compiler" fallback contract. `TreeSitterProvider` duplicates `registry.parse`, ships a parallel `AstProvider`/`PlanetariumModel`/`MicroscopeModel` object graph nothing consumes, and re-parses raw in `zoom_into`. What zoom actually needs (find declaration by id, `extract_body`, context window, source slice) is ~40 lines added to `AstBridge` as `zoom_into(file, symbol) -> Option<ZoomHit>` reusing `registry.parse` + the adapter's `extract_body` — strictly less code than promotion, and it keeps one parse path.

**(c) Delete `TreeSitterProvider`** (and the `AstProvider` trait + orphaned `PlanetariumModel`/`MicroscopeModel`/`IndexOptions`/`ZoomOptions` types, after verifying census uses `AstBridge`+`CelestialCensus`, not these). Counter-argument weighed: `index_project` is a fuller cross-file model and a head-start for roadmap 3.2's call-graph-aware selection. But it is unused/untested-in-production, 3.2 already has `CallGraphAnalyzer` (used in the server zoom menu), and this repo's own kaizen rule says build indexing behind `AstBridge` driven by a real caller rather than revive dead code. Recommendation stands: delete now.

### Migration sequence (revised post-correction)

1. **Collapse regex skeletonizers, carefully**: before touching either implementation, build a golden-file fixture corpus (representative Rust/Python/TS/JS/Go samples) and snapshot both `core::skeleton::Skeletonizer` output and lib.rs `truncate_structure*` output side by side, across both call paths (CLI serialize, WASM `wasm_serialize`). Only once behavioral differences are enumerated and a decision made about which behavior wins where, make `core::skeleton::Skeletonizer` the single implementation and have `truncate_structure*` delegate to it (keeping the free-function signatures so the WASM kernel's call sites don't change).
2. ~~Delete the dead lib.rs `ContextEngine` struct~~ — **do not do this** (see correction). Leave both structs in place; optionally rename for clarity in a separate, low-risk docs/rename-only pass.
3. **Extend `AstBridge`** with `index_file`/`zoom_into` reusing `registry.parse`+`extract_body`, backed by `ParseCache`. Generalize the cache owner from `run_survey` (vo.rs:694) to a shared `AstBridge`-held instance so serialize/survey/zoom share one on-disk cache (kills the double-parse). Add CLI/server zoom tests including the **zoom-from-subdirectory** case (2.3).
4. **Route `core::engine::ContextEngine::serialize`/`zoom`** to prefer `AstBridge` for supported languages (Rust/Python/TS/TSX/JS, registry.rs:340), regex skeletonizer fallback for the rest — 2.2's remaining step. Optionally behind a one-release `ast-serialize` feature flag for rollback, default-on for the 5 langs. While here, fix engine.rs:339's hardcoded `priority = 50` to consult the item-1 scoring layer. This step doesn't touch the WASM kernel (it isn't in `core::engine::ContextEngine`'s call graph), so it's lower-risk than step 1.
5. **Delete `TreeSitterProvider`** + orphaned trait/types once `AstBridge` covers zoom. Independently verified safe (no wasm or other hidden caller) — can be done any time, doesn't depend on steps 1/3/4.

**Tests**: `TreeSitterProvider`/pure-engine tests are deleted with their types; parsing assertions worth keeping migrate to `AstBridge`; new end-to-end zoom tests replace them at the CLI/server boundary. Steps 1 and 4 both warrant feature flags/careful rollout; step 1 especially, since a regression there silently changes real output for both CLI and WASM consumers.

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
