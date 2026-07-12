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

**2.1 One token counter** (I1)
- `BudgetReport` becomes the only authority. `print_mission_log` and `print_context_health` take it as a parameter instead of re-deriving `output.len()/4` (`vo.rs:2545`, `presenter/mod.rs:218-274`).
- Calibrate `TokenEstimator`'s per-file overhead against real serializer output (measure once per format, store constants — or better, have the serializer report actual per-file rendered length back into the report).
- Add the missing regression test: serialize with budget N, assert `|rendered_len/4 − report.used| / N < ε`.

**2.2 One parser on the main path** (I6, I3)
- Route the serialize/skeleton path through `voyager-ast` for its supported languages; keep the regex skeletonizer only as fallback for the long tail. `AstBridge` already demonstrates the integration pattern (`vo.rs:673-716`).
- Introduce a per-file parse cache (path + mtime/hash → IR) shared by serialize, survey, and zoom, killing double-parsing.
- This is also the honest fix for the census metrics: with the AST on the main path, implement real block-nesting depth and cyclomatic complexity from control-flow nodes; rename Dark Matter to "Unparsed regions"; retrigger Red Giants on the new complexity metric and exclude/flag test files.

**2.3 One root, one resolver** (C3, I4)
- Single `ProjectRoot` resolved once (manifest detection with explicit precedence rules — see HANDOFF Q3), passed to `SymbolResolver`, `Engine::zoom`, and every MCP tool. Relative paths always join to it; comparisons on canonicalized paths, never string suffixes (`engine.rs:678-707`).
- Scope `SymbolResolver` by language and by the target file/dir the user gave; prefer nearest match; report ambiguity instead of picking the alphabetically-first cross-language hit.
- This plus 0.1 fixes zoom end-to-end. Then add zoom tests that run from a subdirectory CWD — the case that was 100% broken.

**2.4 One selection engine** (C5, I2, I8)
- Lenses and explore-intents are the same operation ("select a curated, prioritized subset") with two implementations. Merge behind one registry; `--explore` becomes sugar for intent-lenses.
- Fix explore's inputs while merging: default-exclude `experiments/**`, `classic/**`, `benches/**`; replace the alphabetical 200-file hard break with relevance-ordered traversal (or at minimum, log "N files skipped by cap"); drop the Unknown→Calculation fallback into the max-weight bucket (`primitives.rs:361-367,599-617`).

**2.5 MCP hardening** (I4)
- Gate non-initialize requests on `initialized` (return `-32002`); make `shutdown` end the loop; validate tool-arg types explicitly (`-32602` with "expected X, got Y"); echo/negotiate `protocolVersion`; add `ping`. All small, all in `server/mod.rs` — its test suite is already the best in the repo, extend it.

## Phase 3 — Selective evolution (only after Phase 2; each item independently shippable)

- **3.1 Wire auto-focus for real**: bare `vo .` runs smart defaults (project-size-aware lens + budget). The modules exist and are tested; after 2.4 there is a single engine to plumb them into.
- **3.2 Selection quality**: use the call graph you already build (post-C6 fix) to weight centrality in lens/intent ranking — fixes I11's "drop lib.rs, keep .claude/settings.local.json" inversion. Never drop a P100 file entirely while smaller low-P files are included; structure-truncate it instead. Exclude `.claude/`, `.mcp.json` by default (they can leak machine-specific paths into shared context).
- **3.3 Invest in `report_utility`**: it's the moat — a feedback loop from the consuming LLM back into selection priors. Persist per-repo utility scores; feed them into the unified selection engine as a prior. Nobody else in the category has this.
- **3.4 Reintroduce `--detail`/`--semantic-depth`** as real controls over the unified engine (depth = parse fallback tier + analysis passes; detail = rendering verbosity) — only now that they can actually control something.
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
Weeks 3–6:   2.1–2.5  (consolidation, v1.2)       ← one counter, one parser, one root, one engine
Afterwards:  3.x as independent increments; 4.x as strategy
```

The single most important line in this document: **stop adding features until every advertised flag does what it says.** The engine underneath deserves it.
