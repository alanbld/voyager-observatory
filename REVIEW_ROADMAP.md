# Voyager Observatory — Improvement Roadmap

*Ordered by impact vs. effort. Each phase is shippable on its own; do them in order. Finding IDs reference `REVIEW_FINDINGS.md`.*

**Guiding principle:** the goal is one sentence — *"turn a repo into the best possible N-token context for an LLM."* Every change below either makes the existing promise true, or deletes the promise. Nothing here adds new surface area until Phase 4.

---

## Phase 0 — Safety (days; non-negotiable before anyone integrates the MCP server)

| # | Fix | Finding | Hint |
|---|-----|---------|------|
| 0.1 | MCP path containment | C1 | In `server/mod.rs`, resolve every `path` arg: join relative paths onto `self.project_root`, then `fs::canonicalize` both root and target and reject unless `target.starts_with(&root)`. One helper function, three call sites (`:442`, `:525`, `:832`). Add traversal tests (`../`, absolute, symlink). |
| 0.2 | Remove the layout-UB transmute | C2 | `analyzers/generic.rs:202` — return `Vec<&str>` built with `.iter().map(String::as_str).collect()`; change the trait signature if needed. Strengthen the test to dereference elements. |
| 0.3 | Remove the `'static` lifetime transmute | I5 | `intent/primitives.rs:812-818` — give `ScoredElement` a real `'a` parameter or store owned clones. Mechanical but touches signatures. |
| 0.4 | Fix char/byte indexing panic | C6 | `relationships/extractor.rs:690` — use `char_indices()`. Add a non-ASCII fixture test (emoji in comment, accented identifier). |
| 0.5 | Make clippy a CI gate | C6, N6 | It currently *fails to compile* the lib. Once 0.4 lands, `cargo clippy --workspace --all-targets -- -D warnings` (or at least deny-lints) in CI. |
| 0.6 | Pin tree-sitter-c-sharp in voyager-ast too | N4 | Copy the `=0.23.1` pin + comment into `voyager-ast/Cargo.toml:39`, or move grammar deps to `[workspace.dependencies]`. |

## Phase 1 — The honesty release (≈1 week; the tool gets *smaller* and *better*)

Every flag that exists works; every claim in `--help` is true. This is the highest trust-per-effort ratio in the whole roadmap.

| # | Fix | Finding | Hint |
|---|-----|---------|------|
| 1.1 | Delete dead flags `--semantic-depth`, `--detail`, `--explain-reasoning` (or wire them — see 3.4; do not ship them unread) | C4 | They have zero read sites in `vo.rs`. Deleting is a 20-line diff; keep the enums (the orchestrator uses them internally). |
| 1.2 | Remove "auto-focus applies smart defaults" from `--help`, or actually call `AutoFocus` on bare `vo .` | C4 | Zero call sites today (`orchestrator/{auto_focus,smart_defaults}.rs` are tested but orphaned). Wiring it is the better long-term move but is Phase 3 work; the help text lie can die today. |
| 1.3 | Make `--lens` a clap `ValueEnum`, validated in every mode | C5, N2 | Use `--format` as the template. Kills the `minimal` drift (C4), the garbage-string acceptance, and the exit-code inconsistency in one change. Remove `minimal` from help or register a real minimal lens. |
| 1.4 | Warn on ignored flags instead of silent precedence | I8 | Early-return branches for `--survey`/`--explore`/`--zoom` (`vo.rs:2688,2826`): one `eprintln!("note: --lens is ignored in {mode} mode")` each. Same for `--by` outside `composition`. |
| 1.5 | Delete `pm_encoder_mcp` (or move to `examples/legacy/` with a README) | I7 | It's feature-gated, undocumented, contract-incompatible, and pre-rename. Pure confusion risk. |
| 1.6 | Delete or quarantine the orphaned `core/syntax/adapter.rs` engine | I6, N5 | 2,266 LOC + 8 tautological tests for code nothing calls. If Phase 2's consolidation will reuse it, mark it `#[cfg(feature = "experimental")]`; otherwise delete — voyager-ast already covers the same ground better. |
| 1.7 | Remove unused `anyhow`; fix duplicated explore error text; fix "Healthy Healthy Density" string | N3, I10 | Trivial. |
| 1.8 | Finish the `vo` rename | N1 | Priority order: (a) `ZOOM_AFFORDANCE` strings in LLM-facing output (`lib.rs:1122,1235,1313`) — these actively mislead the downstream model; (b) MCP `serverInfo.name`; (c) XML `package` attr; (d) config filenames (accept old names for one release, warn, then drop). |
| 1.9 | Default token budget per lens | I9 | `--lens onboarding` without a budget currently emits 2.4M tokens. Give each lens a sane default (e.g. onboarding 50k) and print "using default budget Nk (override with --token-budget)". Priority ranking already exists; it just needs to gate. |

## Phase 2 — One source of truth (2-4 weeks; the structural kaizen)

Three consolidations. Each removes a whole class of contradiction.

**2.1 One token counter** (I1) — ✅ done (`c10af3b`); one follow-up open
- `BudgetReport` becomes the only authority. `print_mission_log` and `print_context_health` take it as a parameter instead of re-deriving `output.len()/4` (`vo.rs:2545`, `presenter/mod.rs:218-274`). **Done**: `BudgetReport::recalibrate()` sets `used` from the real rendered output; `serialize_entries_claude_xml_with_report` renders twice when the pre-serialization estimate drifts, so the `utilized` attribute embedded in the file agrees with stderr too.
- Add the missing regression test: serialize with budget N, assert `|rendered_len/4 − report.used| / N < ε`. **Done** — `test_budget_report_used_matches_rendered_claude_xml`.
- **Open** (this is DECISIONS Q10's actual root-cause fix, not yet done): `TokenEstimator::estimate_file_tokens`'s per-file overhead is one flat formula regardless of output format, so the *pre-serialization selection estimate* still undercounts real Claude-XML overhead (attrs, CDATA, `zoom_actions`). The 2.1 fix above makes the *reported* number honest after the fact; it does not stop a borderline file from being selected when real XML rendering would have put it over budget. Calibrate the estimator per format (see Q10) so budget *enforcement*, not just reporting, is accurate.

**2.2 One parser on the main path** (I6, I3) — direction confirmed, DECISIONS Q1/Q4 (High confidence); census/complexity half **done**; remainder needed a design pass, now complete (see `REVIEW_DESIGN_PROPOSALS.md` §2)
- Route the serialize/skeleton path through `voyager-ast` for its supported languages; keep the regex skeletonizer only as fallback for the long tail. `AstBridge` already demonstrates the integration pattern (`vo.rs:673-716`). Q1 settled this with evidence (`experiments/lsp_poc/comparison_results.csv`): regex/tree-sitter ≈1.0 precision at 50-75× rust-analyzer's speed. Only 5 of 26 `LanguageId` variants have real adapters today — frame docs/help honestly as "excellent top ~5-12 languages, graceful fallback elsewhere," not a 60-language claim. Watch the tree-sitter-C-grammar-vs-`wasm32` risk if the `wasm` feature still matters.
  - **Correction found during execution (2026-07)**: this isn't "one regex skeletonizer as fallback" — there are currently *two* independent regex skeletonizers (`lib.rs`'s `truncate_structure*` family and `core::skeleton::Skeletonizer`), living behind *two* differently-scoped `ContextEngine` structs (`lib.rs:315`, dead/callerless; `core/engine.rs:188`, the real one). A dormant, fuller AST indexing/zoom system (`voyager_ast::TreeSitterProvider`/`AstProvider`) also exists, unused outside its own tests. `REVIEW_DESIGN_PROPOSALS.md` §2 has the full design: collapse to one regex skeletonizer, delete the dead `ContextEngine` struct (keep its free fns), extend `AstBridge` (not `TreeSitterProvider`, which should be deleted) with zoom/index support, then route `core::engine::ContextEngine` through it — 5-step migration sequence provided.
- Introduce a per-file parse cache (path + mtime/hash → IR) shared by serialize, survey, and zoom, killing double-parsing. **Partially done** (`b8eac44`): `ParseCache`/`ParseCacheManager` (`src/core/ast_cache.rs`) is disk-persisted, content-hash (md5) keyed, and wired into `--survey`. Scoped to survey only for now because serialize/zoom didn't route through `AdapterRegistry::parse` at the time — the design above extends it to all three once the routing above lands (cache stays body-free per the design's recommendation; zoom re-parses its one target file on demand).
- This is also the honest fix for the census metrics: with the AST on the main path, implement real block-nesting depth and cyclomatic complexity from control-flow nodes; rename Dark Matter to "Unparsed regions"; retrigger Red Giants on the new complexity metric and exclude/flag test files. Q4 found the cheap path: all three main adapters already extract control-flow into IR `Block`/`ControlFlow` types, but `extract_body` has zero non-test callers — the missing wire is attaching `Block` to `Declaration` in the production parse path and having census count decision points from that. Do both the rename and the real metric (not one or the other); complexity only covers the ~3 adapter languages that have it, so label the rest honestly rather than showing 0. **Complexity/nesting half done** (`a9591f5`): real McCabe complexity + control-flow nesting wired via `Declaration.body: Option<Block>`, Red Giants now exclude test files. Dark Matter → "Unparsed regions" rename still open (deferred: 7-file blast radius including `temporal/geological.rs`'s `dark_matter_ratio` consumer, needs its own pass).

**2.3 One root, one resolver** (C3, I4) — target corrected by DECISIONS Q3 (High confidence on mechanism)
- **Correction, not just an amendment**: `ProjectManifest::detect` (the "`.git` overrides `Cargo.toml`" function the original review blamed) has **zero callers — it's dead code**. The CLI already resolves roots via `find_project_root`, which is already nearest-marker-wins. Don't spend time rewriting `ProjectManifest::detect`'s precedence rules; the bug isn't there.
- The real zoom-breaking bug is `SymbolResolver::find_symbol` walking the *entire* tree unscoped by language or by the path the user gave — that's how a Rust lookup resolved into deprecated Python. Fix: scope `SymbolResolver` by language and by the target file/dir; prefer nearest match; report ambiguity instead of picking the alphabetically-first cross-language hit.
- Decided semantics (Q3): nearest-marker-wins stays the default; add an explicit `--root` override; scope symbol search to the target subpath and language. MCP's explicit-root + containment model (post-0.1) is the template — apply the same pattern to `Engine::zoom` and every MCP tool, joining relative paths onto one resolved root and comparing canonicalized paths, never string suffixes (`engine.rs:678-707`).
- This plus 0.1 fixes zoom end-to-end. Then add zoom tests that run from a subdirectory CWD — the case that was 100% broken.

**2.4 One selection engine** (C5, I2, I8) — **AMENDED by DECISIONS Q2 (High confidence); design pass complete, see `REVIEW_DESIGN_PROPOSALS.md` §1 (concrete `Signal`/`Score`/`Scorer`/`ScoringContext` shape + 4-step migration); ready for execution**
- ~~Lenses and explore-intents are the same operation... Merge behind one registry~~ — **superseded.** Q2: **do not merge.** A lens modifies serialized file *content*; an intent produces a terminal *report* (relevance %, read/skim/skip, no content) and `--explore` returns before `EncoderConfig` is even built. MCP already exposes them as separate tools with different JSON shapes. Keep both surfaces; **unify the scoring layer underneath** instead. Intent docs should frame intents as the successor to lenses to explain the vocabulary overlap, rather than collapsing the two.
- **Why this needs a design pass, not straight execution**: a code survey (2026-07) found *zero* shared code between the two scoring subsystems today — lenses (`src/lenses.rs`, ~900-950 LOC of scoring logic) are a static glob-pattern → integer-priority lookup over file *paths*; intents (`src/core/fractal/intent/`, ~1800-2000 LOC) are a weighted semantic classifier over parsed *symbol metadata* (name/signature/doc/visibility → concept buckets → float relevance). No common `Scorer` trait, no shared config schema, different input domains and score representations entirely. "Unify the scoring layer underneath" is a from-scratch abstraction-design task (~2700-3000 LOC of incompatible logic to reconcile), not a mechanical extract-and-move refactor. Get a design proposal (candidate: a shared `Signal`/`ScoringContext` model that both glob-based priority and symbol-level concept scoring plug into) reviewed before a session starts moving code.
- Fix explore's inputs regardless of merge/no-merge: default-exclude `experiments/**`, `classic/**`, `benches/**`, `.llm_archive/` (also confirmed independently by Q7, scoped to repo root not any-depth); replace the alphabetical 200-file hard break with relevance-ordered traversal (or at minimum, log "N files skipped by cap"); drop the Unknown→Calculation fallback into the max-weight bucket (`primitives.rs:361-367,599-617`).

**2.5 MCP hardening** (I4) — direction confirmed, DECISIONS Q5 (High confidence)
- Gate non-initialize requests on `initialized` (return `-32002`); make `shutdown` end the loop; validate tool-arg types explicitly (`-32602` with "expected X, got Y"); echo/negotiate `protocolVersion`; add `ping`. All small, all in `server/mod.rs` — its test suite is already the best in the repo, extend it. Q5 confirmed keep-hand-rolled-harden-it (52 tests, no tokio) over adopting `rmcp` (0 tests, incompatible contract) — the `rmcp` prototype itself was already deleted (roadmap 1.5). Revisit only if the MCP spec starts moving fast on schema/version/streaming.

## Phase 3 — Selective evolution (only after Phase 2; each item independently shippable)

- **3.1 Wire auto-focus for real**: bare `vo .` runs smart defaults (project-size-aware lens + budget). The modules exist and are tested; per Q2's amendment to 2.4 there is no single merged engine, so plumb this into the unified *scoring layer* (not a unified lens/intent surface) once that lands.
- **3.2 Selection quality**: use the call graph you already build (post-C6 fix) to weight centrality in lens/intent ranking — fixes I11's "drop lib.rs, keep .claude/settings.local.json" inversion. Never drop a P100 file entirely while smaller low-P files are included; structure-truncate it instead. Exclude `.claude/`, `.mcp.json` by default (they can leak machine-specific paths into shared context).
- **3.3 Invest in `report_utility`**: it's the moat — a feedback loop from the consuming LLM back into selection priors. DECISIONS Q9 (High confidence, exhaustive trace): confirmed **write-only today** — `blend_priority` only runs when `LensManager` has a `ContextStore`, and production code never constructs one with a store; the read-back path is "~one line from working" (`with_store(ContextStore::load_from_file(...))`). **Decision: wire it.** Design pass complete, see `REVIEW_DESIGN_PROPOSALS.md` §3: time-decay toward neutral (half-life ~30 days) + confidence-weighted blend (low-observation files stay near static priority) so early low scores don't permanently bury a file; also fixes a live oversight where even *unseen* files get perturbed by the store's default value today. `--frozen` must bypass decay entirely to stay deterministic. Ready for execution.
- **3.4 Reintroduce `--detail`/`--semantic-depth`** as real controls over the unified scoring layer (depth = parse fallback tier + analysis passes; detail = rendering verbosity) — only now that they can actually control something.
- **3.5 Metaphor policy**: keep the flavor, add the meaning — every metric a user acts on gets a plain-language name alongside ("Dark Matter (unparsed regions)"), the Rosetta legend appears in every output mode, and placeholder "Unknown" values are either resolved or the line is suppressed (I10: hemispheres, dominant concept, Big Bang date — the last one likely a bug in `stellar_drift.rs` date extraction since age-in-days is populated).
- **3.6 God-file decomposition**: `lib.rs` (5,292) and `vo.rs` (3,743) into command modules — do it *after* 2.x so you're not moving code you're about to delete.

## Phase 4 — Future-proofing bets

- **MCP-first product posture.** The CLI is becoming the secondary interface; agent platforms are the primary consumer. The durable niche is: deterministic (`--frozen`), honestly-budgeted, sandbox-safe context serving with zoom + utility feedback. A tool that does only that, correctly, beats the current sprawling surface.
- **Consider adopting `rmcp`** for the server once the tool contract stabilizes (the abandoned prototype proved it works and gets spec compliance for free) — but only after the hand-rolled server's behavior is fully tested, so parity is verifiable. Keeping the dependency-free hand-rolled server is also defensible; pick one (HANDOFF Q5).
- **Process kaizen** (root cause of the meta-finding): a feature is "done" only when a production entry point calls it and an end-to-end test exercises it through the CLI/server; `--help` text is generated from the same registry that validates input (a flag that isn't wired can't be documented); CI gates: clippy `-D warnings`, a budget-accounting invariant test, a zoom-from-subdirectory test, an MCP traversal test.

---

## Suggested sequencing at a glance

```
Week 1:      0.1–0.6  (safety)                    ← unblocks safe MCP adoption
Week 2:      1.1–1.9  (honesty release, v1.1)     ← every flag works; smaller surface
Weeks 3–6:   2.1–2.5  (consolidation, v1.2)       ← one counter, one parser, one root, one scoring layer (two surfaces)
Afterwards:  3.x as independent increments; 4.x as strategy
```

The single most important line in this document: **stop adding features until every advertised flag does what it says.** The engine underneath deserves it.

*Amendment history: 2026-07-25 — folded `REVIEW_DECISIONS.md` (Q1-Q10) into 2.1-2.5 and their Phase 3 references directly, so this document and the decisions record don't drift out of sync. Notably: 2.4 no longer proposes merging lenses/intents (Q2), 2.3's target shifted from `ProjectManifest::detect` (dead code) to unscoped `SymbolResolver` (Q3), and 2.1's calibration sub-task remains open pending roadmap 2.1's follow-up.*

*2026-07-27 — 2.1's calibration follow-up shipped (`a304092`). 2.2's census/complexity half shipped (`a9591f5`) plus a survey-scoped parse cache (`b8eac44`); the "route serialize through voyager-ast" half surfaced a sharper problem than originally scoped (two `ContextEngine` structs, two regex skeletonizers, a dormant unused AST indexing system) — see `REVIEW_DESIGN_PROPOSALS.md`. A dedicated design pass produced concrete, ready-to-execute proposals for 2.2's remainder, 2.4, and 3.3 (all three were flagged as needing design before code); nothing in that doc has been implemented yet.*
