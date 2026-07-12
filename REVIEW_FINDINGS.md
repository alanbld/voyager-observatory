# Voyager Observatory — Technical Review Findings

*Multi-agent review, July 2026. Four investigation domains (UX/CLI, Semantic Analysis, MCP Server, Rust Code Quality) plus a self-analysis phase running `vo` on its own codebase. All findings below were empirically verified (command + output, or file:line code trace) unless marked otherwise.*

**Verdict in one line:** the core engine (walk → skeletonize → budget → render, plus MCP zoom/session plumbing) is real, fast, and crash-free — but a large fraction of the advertised surface is dead, mislabeled, or broken, and there is one confirmed security vulnerability. The dominant systemic pattern is **"built but never wired"**: at least six subsystems exist, compile, and pass tests, yet are never invoked from any production entry point.

---

## Critical

### C1. MCP server path traversal / sandbox escape (Security)
The `path` argument of `get_context`, `zoom`, and `explore_with_intent` accepts arbitrary absolute or `../`-relative paths with **no containment check** against the `--server <root>` argument. Live-confirmed: a client request with `path: "../secret_outside"` returned the full contents of a file outside the declared project root; an absolute path did the same.
- Evidence: `rust/src/server/mod.rs:442-446` (get_context), `:525-529` (zoom), `:832-836` (explore_with_intent). Zero hits for canonicalization or `starts_with(project_root)` in `server/mod.rs`, `core/engine.rs`, `core/zoom.rs`.
- Anyone wiring this into an AI CLI today exposes their filesystem to whatever the model requests.
- Fix: canonicalize `project_root` and every resolved target; reject targets not prefixed by the root.

### C2. Undefined behavior: `&[String]` → `&[&str]` transmute (Memory safety)
`unsafe { std::mem::transmute(self.config.extensions.as_slice()) }` reinterprets a slice of 24-byte `String`s as 16-byte `&str`s — wrong element stride, so any read past index 0 is memory corruption. The safety comment addresses lifetimes, not the actual layout mismatch. Not on the current dispatch path, but it is a public trait method (`LanguageAnalyzer::supported_extensions`) reachable by any future caller. Its own unit test only checks `!is_empty()` and structurally cannot detect the corruption.
- Evidence: `rust/src/analyzers/generic.rs:202`; test at `:445-451`.
- Fix: return owned `Vec<&str>` via `.iter().map(String::as_str).collect()`, or store `&'static str` in config.

### C3. `--zoom fn=/class=` is broken in almost all realistic invocations
Two independent root computations disagree: `ProjectManifest::detect` walks up to `.git` and **overrides** closer markers (`rust/Cargo.toml`), silently widening symbol search to the whole monorepo, while `Engine::zoom` walks files relative to the **raw CLI path argument**. The resolved symbol path and the walk root are then incompatible, producing `Invalid zoom target` errors. Compounding it, `SymbolResolver` does unscoped cross-language regex search — `--zoom fn=apply_token_budget` targeting `rust/src/budgeting.rs` resolved to the **deprecated Python implementation** (`classic/python/pm_encoder.py:1664`) before crashing. The only working form found: run from the exact `.git` root with `.` as target (span accuracy is then correct).
- Evidence: `rust/src/core/manifest.rs:55-84` (".git is definitive root" override), `rust/src/bin/vo.rs:3332` + `rust/src/core/engine.rs:266-292,678-707` (suffix-match `find_file` that can never match), `rust/src/core/search.rs` (no language/path scoping).
- Zoom is a flagship MCP tool; this breaks it for CLI and server alike.
- Fix: thread the single resolved manifest root through both resolver and engine; compare canonicalized paths, not string suffixes; scope symbol search by language/target file.

### C4. Three advertised CLI controls are dead code; a fourth advertised feature is never invoked
`--semantic-depth`, `--detail`, and `--explain-reasoning` are parsed by clap and **never read** (zero usage sites for `cli.semantic_depth`, `cli.detail`, `cli.explain_reasoning`). Byte-identical outputs confirmed (same MD5) across `quick`/`balanced`/`deep`. The `--help` promise "auto-focus applies smart defaults" is also never invoked: `core/orchestrator/{auto_focus,smart_defaults}.rs` exist and pass tests but have zero call sites in any binary. `--lens minimal` is listed in help but not registered in `LensManager`.
- Evidence: `rust/src/bin/vo.rs:113,212,216` (declared, never read); `rust/src/core/mod.rs:88,98` (re-exported, uncalled); `rust/src/bin/vo.rs:69` vs `rust/src/lenses.rs:115,263,322,598` (registry: architecture, debug, security, onboarding only).
- Fix: wire them or delete them. Shipping controls that don't move the output destroys trust in the ones that do.

### C5. `--lens` is unvalidated and has zero effect outside `--token-budget` mode
Without a budget, the lens string is stored unchecked (`config.active_lens = cli.lens.clone()`), the `apply_lens` result is explicitly discarded (`let _ =`), and the value is echoed into XML metadata while file selection is **byte-identical** across lens values (verified by diff). Only the `--token-budget` branch validates.
- Evidence: `rust/src/bin/vo.rs:2947`, `rust/src/lib.rs:2195-2196,2458-2487`, validation only at `vo.rs:3459-3477`. `vo . --lens totallybogus` → exit 0, `<context lens="totallybogus">`.
- Fix: make `--lens` a clap `ValueEnum` (like `--format`/`--survey` already are) so bad values fail everywhere at parse time; make lenses affect selection in non-budget mode (see roadmap) or say they don't.

### C6. Non-ASCII input can panic the call-graph extractor; clippy cannot compile the lib
`content[s..=i]` slices with `i` taken from `chars().enumerate()` — a character index used as a byte offset. Any multi-byte UTF-8 character before a matched brace pair panics or mis-slices. This trips the deny-by-default lint `clippy::char_indices_as_byte_indices`, so `cargo clippy --release --lib` **fails to compile** the crate.
- Evidence: `rust/src/core/fractal/relationships/extractor.rs:690` (index from `:655`).
- The Relationships module gained 30 tests recently (`d6910ee`) — none use non-ASCII fixtures.
- Fix: use `char_indices()`; add a non-ASCII regression fixture; gate CI on clippy.

---

## Important

### I1. Three independent token counters disagree in a single run
- Path A — the budgeter (`rust/src/budgeting.rs:117-223`): per-file estimates (`len/4` + flat ~17.5-char/file overhead guess) accumulated at selection time; by construction lands ≤100%. Reported "Used: 79,993 / 80,000 (100.0%)".
- Path B — the "Fuel" banner: `output.len() / 4` on the final rendered string (`rust/src/bin/vo.rs:2545` → `rust/src/core/presenter/mod.rs:262-273`). Reported "94,633 / 80,000 (118%)" for the same run.
- Path C — "Context Health" recomputes `output.len()/4` again with a different overhead formula (`presenter/mod.rs:218-240`).
- Root cause of the 100%-vs-118% contradiction: Path A's flat overhead model systematically undercounts real serialization overhead (markers, TOC, checksums, zoom affordances). No test compares `BudgetReport.used` against actual output length.
- Fix: single source of truth — the presenter consumes `BudgetReport`; calibrate the per-file overhead against real serializer output; add the regression test.

### I2. `--explore` intent ranking is starved and fallback-biased — top "business logic" is a side experiment
Three compounding causes, all verified:
1. Default ignores (`explorer.rs:58-64`) exclude only `node_modules/target/.git/locks/min.js` — `experiments/`, `classic/` (deprecated Python), `.llm_archive/` are all scanned as production code.
2. `--explore-max-files` defaults to 200 and traversal is plain alphabetical `walkdir` with a hard break — `experiments/` sorts before `rust/`, so the budget is spent before the real engine is ever reached (`explorer.rs:369-422`; raising to 5000 surfaces `estimate_tokens`, `apply_token_budget` and flips "Dominant concept type" from Unknown to Calculation).
3. `ConceptType::infer`'s final fallback routes any unmatched public function into `Calculation` (`primitives.rs:361-367`) — which `business_logic()` weights at 1.0, the maximum (`primitives.rs:599-617`). Well-documented benchmark helpers (`precision`, `recall`, `f1_score`) therefore top the ranking at 72-75% relevance.
- Fix: exclude `experiments/**`, `classic/**`, `benches/**` by default; make traversal relevance-ordered or remove the silent cap (at minimum, log what was skipped); give the fallback bucket a low weight.

### I3. Census metrics measure something other than what their labels claim
- **Dark Matter** = tree-sitter parse-error regions (`census.rs:328-329` ← `voyager-ast/src/registry.rs:101` `extract_errors`), not complexity/debt. On any codebase that compiles it is ~always 0 — as observed across all 123.8k LOC.
- **Red Giants** ("high complexity") trigger: `>500 lines && (dark_matter_ratio>0.05 || nebula_ratio<0.1)` (`census.rs:721-724`). Since dark matter is always ~0, this degenerates to "long and under-documented" — which is why 5-6 **test files** top the list. No complexity computation backs the label.
- **Max Nesting Depth** recurses only `Declaration.children` (`census.rs:344-360`); Rust adapters populate children only for Impl/Trait/Struct/Enum (`rust_adapter.rs:179-188`), so control flow is invisible and the metric is structurally capped near 2-3.
- The health survey simultaneously prints "No High Dark Matter regions - code is well-parsed" and a "high complexity" Red Giants list — two subsystems contradicting each other with the same vocabulary.
- Fix: rename Dark Matter to "Unparsed regions" (that reading is honest and useful), implement real cyclomatic/block-nesting metrics from the tree-sitter AST, exclude test files from Red Giants or say "large & undocumented".

### I4. MCP server: CWD confusion, no lifecycle enforcement, no argument validation
- A **relative** `path` override is resolved against the server process's CWD, not the `--server <root>` argument (`server/mod.rs:442-446` — never joined to `project_root`). The documented `~/.claude/mcp.json` has no `cwd` field, so real-world behavior depends on whatever CWD the MCP client chooses. Verified: same request returned 3.1MB of the wrong tree from one CWD and the correct file from another.
- `tools/list`/`tools/call` succeed **before** `initialize`; the `self.initialized` flag (`server/mod.rs:148,230,256`) is write-only — grep confirms it gates nothing. `shutdown` returns `{}` but does not stop the request loop.
- No tool-argument type validation: `lens: 123` is silently ignored (request succeeds, no lens applied); `utility: "x"` is misreported as "Missing 'utility' parameter" (`server/mod.rs:448,768-784`).
- `initialize` hardcodes `protocolVersion: "2024-11-05"` and ignores the client's requested version (`server/mod.rs:255,262`). No `ping` handler.
- What works (verified): correct JSON-RPC framing, -32700/-32600/-32601/-32602 errors, notifications unanswered, clean stdout discipline (banners go to stderr), no crash on malformed/2MB/unicode input, clean exit on stdin EOF.

### I5. Second unsound transmute: lifetime laundering to `'static` on the intent-scoring path
`transmute::<&ContextLayer, &'static ContextLayer>` (×2) with the author's own comment: "we'll need to handle this differently." On the `explore_with_intent` path — an exposed MCP tool. Any `ScoredElement<'static>` outliving its source is a use-after-free.
- Evidence: `rust/src/core/fractal/intent/primitives.rs:812-818`.
- Fix: real lifetime parameter or owned data; no transmute.

### I6. Four parallel parsing engines; the flagship path uses the crudest one
1. `core/skeleton/parser.rs` (1,159 LOC, regex) — the one wired into the main serialize/budget path (`engine.rs:415,428,470`).
2. `core/syntax/adapter.rs` (2,266 LOC, real tree-sitter) — **orphaned**: only consumer is a `PlaceholderPluginHost` with a literal TODO "Connect to TreeSitterAdapter" (`core/plugin/mod.rs:158,338`); kept alive by a re-export.
3. `voyager-ast` subcrate (~11.4k LOC, tree-sitter, best-tested code in the repo) — used **only** by `--survey` via `AstBridge` (`vo.rs:673-716`).
4. `src/plugins/{typescript,python,abl,shell}.rs` (~6,800 LOC, hand-rolled regex) — used only by semantic clustering.
No shared cache; a file analyzed by both `--survey` and serialization is parsed twice by unrelated engines. The product's context quality is capped by engine #1, the weakest.

### I7. Two incompatible MCP server implementations ship in the same crate
The hand-rolled `vo --server` (documented, default-built) and the rmcp-based `pm_encoder_mcp` (feature-gated, builds fine) expose **different tool contracts** (`files:[{path,content}]` upload vs. filesystem walk; different parameter names; missing sessions/explore tools). `pm_encoder_mcp` predates the Voyager rename (15 `pm_encoder` references, 0 `voyager`), is referenced only by a stale status doc, and is effectively abandoned.
- Fix: delete it or mark it legacy explicitly; one server, one contract.

### I8. Silent winner-takes-all flag precedence
Verified by diff: `--lens` is silently ignored by `--survey`, `--zoom`, and `--explore` (early-return branches at `vo.rs:2688,2826` execute before the lens is even stored); `--by` is silently ignored by `--survey health/evolution` (only `composition` respects it); `--survey composition --by sector` prints "using constellation" as fallback but renders a flat leaf-directory list that matches neither. No mode emits a "flag X is ignored in this mode" note.

### I9. The documented onboarding command emits the entire repository
`vo . --lens onboarding --format claude-xml` → 153,563 lines, "Fuel: 2,476,633 / 2,476,633 (100%)". The attention map computes sensible priority tiers but they never gate inclusion without `--token-budget`. A newcomer's first command produces an unusable multi-megabyte dump.
- Fix: default token budget per lens, or make every doc example pair lenses with a budget.

### I10. Placeholder values leak into user-facing output in at least four places
"Two hemispheres detected: Unknown" (architecture banner), "Dominant concept type: Unknown" (explore — mode-count dominated by zero-symbol File-fallback layers, `explorer.rs:515-527`, `composition.rs:406-422`), "Big Bang (First Commit): Unknown" while Galaxy Age is populated in the same report (`vo.rs:1526-1529`; the date and the age derive inconsistently), and `--explore` invalid-intent errors print the valid-intents list twice.

### I11. Architecture-lens selection is size-greedy in the wrong direction, and pulls in config noise
Under an 80k budget it dropped `lib.rs` (43.7k tokens) and `lenses.rs` (20.6k) — arguably the two most architecturally central files — entirely rather than structure-truncating them, while including `.claude/settings.local.json` and `.mcp.json` at P80 (the latter leaking a foreign machine's absolute path into the context). Breadth-of-small-files beats even-skeleton-inclusion of large central files.

---

## Nice-to-have

- **N1. `pm_encoder` branding is systemic**: XML `package` attr, MCP `serverInfo.name` (`server/mod.rs:267`, `mcp_server.rs:373`), config filenames (`.pm_encoder_config.json`, `init.rs:161-194`), compatibility binaries, and — worst — `ZOOM_AFFORDANCE: pm_encoder --zoom ...` strings injected into LLM-facing output (`lib.rs:1122,1235,1313`), literally instructing the downstream model to run the wrong binary name.
- **N2. Exit-code split is defensible but undocumented**: clap enum errors → 2, all business-logic errors → 1; `--lens` errors take the "1" path only because it isn't a ValueEnum (see C5).
- **N3. Error-handling style is threefold**: unused `anyhow` dependency (0 references — dead weight), `thiserror` in 9 files, and 75 occurrences of stringly `Result<_, String>` dominating.
- **N4. tree-sitter-c-sharp pin lives only in the root `Cargo.toml`** (`=0.23.1`, ABI-15 comment); `voyager-ast/Cargo.toml:39` says `"0.23"` unpinned — safe today only via workspace unification.
- **N5. Test-quality gaps behind the big number** (3,273 tests): `assert!(x.len() >= 0)` tautologies ×8 in the orphaned syntax adapter, plus in `test_vectors.rs:1452` and `intent/explorer.rs:905`; the golden-vector budget harness has populated-but-never-read expectation structs — the comparison logic was scaffolded, never wired. Budgeting has 43 tests but none guard the Path A/B divergence (I1). Bright spot: `server/mod.rs`'s 47 tests genuinely assert JSON-RPC shapes.
- **N6. Hygiene**: 51 build warnings, 57 clippy warnings (+1 hard error, C6), 18 `#[allow(dead_code)]`, 38 TODO/FIXME. God files: `lib.rs` 5,292 LOC, `vo.rs` 3,743, `presenter/mod.rs` 2,283. Production `unwrap()`s are mostly safe `lazy_static!` regex compiles; the MCP request parser correctly matches instead of unwrapping.
- **N7. Metaphor without a legend outside `--survey`**: the Rosetta-Stone line exists only in survey output; architecture/explore banners use Stars/Nebulae/hemispheres/Fuel unexplained.

---

## What works (verified, worth protecting)

- No crashes or panics across every command, malformed input, 2MB argument, and unicode content thrown at CLI and server.
- `claude-xml` output is well-formed (xmllint-clean), CDATA correct.
- `--stream` is genuine incremental streaming (`lib.rs:2509-2549`: per-file write + flush), not buffered batch.
- Surveys and explore are deterministic across runs; serialization is fast (0.57s for an 80k-token context over 124k LOC).
- JSON-RPC framing, error codes, and stdout discipline in server mode are solid.
- `voyager-ast` is the best-engineered, best-tested module in the workspace.
- `report_utility` (LLM-to-selector feedback loop) is a genuinely novel design idea.

## The meta-finding: "built but never wired"

Six independent instances of complete, often-tested subsystems with no production caller: auto-focus/smart-defaults orchestrator, the 2,266-line tree-sitter `syntax/adapter.rs` (placeholder host), `--semantic-depth`/`--detail`/`--explain-reasoning` plumbing, the write-only MCP `initialized` flag, the golden-vector comparison harness, and the `pm_encoder_mcp` binary. The failure mode is consistently the **last mile of integration**, not implementation quality. Any process change that requires an end-to-end wiring test before a feature is considered "done" (and before it appears in `--help`) would have prevented most of the Critical findings.
