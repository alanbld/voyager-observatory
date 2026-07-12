# Voyager Observatory — Open Questions for the Author

*These are the decisions the review could not (and should not) make for you. Each shapes a chunk of the roadmap; answering them before starting Phase 2 of `REVIEW_ROADMAP.md` avoids rework.*

---

## Q1. Which parser is the future?

Four parsing engines coexist (regex skeletonizer on the main path, orphaned `core/syntax/adapter.rs` tree-sitter engine, `voyager-ast`, regex plugins for clustering). The roadmap assumes **voyager-ast wins** — it's the best-tested and already has the IR — with the regex skeletonizer demoted to long-tail fallback. But this is a real architectural choice:

- Was `core/syntax/adapter.rs` (with its `PlaceholderPluginHost` TODO) *meant* to replace voyager-ast, or the reverse?
- Is the 60-language claim (spectrograph + regex skeleton) a hard requirement, or is "excellent for the top ~12 languages, graceful degradation elsewhere" acceptable? That answer determines how much of the regex layer must survive.
- Is WASM support (the `wasm` feature, `crate-type = ["cdylib"]`) still a target? tree-sitter-in-WASM is heavier than regex; if WASM matters, the fallback layer earns its keep.

## Q2. Are lenses and intents one concept or two?

`--lens` (architecture/debug/security/onboarding) and `--explore` (business-logic/debugging/onboarding/security/migration) overlap almost completely in vocabulary and both mean "select a curated subset." The roadmap proposes merging them behind one registry. Push back if there's a distinction we missed — e.g., if lenses were meant to be *filters over serialization* while intents are *interactive exploration sessions* with different output contracts. If they stay separate, the silent-precedence rules between them need to become explicit errors instead.

## Q3. What are the intended root-resolution semantics in a monorepo?

`ProjectManifest::detect` deliberately treats `.git` as "definitive project root," overriding a closer `Cargo.toml` (`manifest.rs:77-81` — the comment says this is intentional). That decision is what breaks zoom from `rust/` and widens symbol search into `classic/` and `experiments/`. Options:

1. Nearest-marker wins (Cargo.toml/package.json/pyproject.toml beat `.git`) — matches user intuition ("I'm in the Rust project").
2. `.git` wins but search is scoped to the path argument the user gave — matches "the repo is the universe" while respecting explicit targets.
3. Explicit `--root` flag; detection is only a default.

The review recommends (2) + (3), but this is a product-semantics call, especially for the MCP server where there is no CWD contract at all.

## Q4. What should "Dark Matter" mean?

Right now it counts tree-sitter parse-error regions (≈0 on healthy code) while the UI sells it as complexity/technical debt. Two honest paths:

- **Rename** to "Unparsed regions" — genuinely useful as a *parser-coverage* signal (it tells the user how much of their context the tool actually understood), and it becomes more meaningful once voyager-ast is on the main path.
- **Reimplement** as real complexity (cyclomatic / block nesting from control-flow nodes) and keep the debt framing.

Doing both (two separate metrics) is also defensible. Which story do you want the census to tell? Related: should test files be exempt from Red Giants, or flagged under a different label ("large test aggregators")?

## Q5. Hand-rolled MCP server or `rmcp` SDK?

The hand-rolled server has the best tests in the repo and zero heavy deps (no tokio). The `rmcp` prototype gets spec compliance (schema generation, version negotiation) for free but drags in an async runtime and currently has an incompatible tool contract. The roadmap hardens the hand-rolled one first (it's what users have), and defers the SDK question. If you foresee needing concurrent request handling, cancellation, or streaming tool results, `rmcp`/tokio is the cheaper long-term path — decide before investing further in the synchronous loop.

## Q6. Who is the primary user: humans at a terminal, or agents over MCP?

The astronomical metaphor, emoji banners, and mission-log presentation are built for humans; the XML/budgeting/zoom machinery is built for agents. Both can coexist, but the *default* posture decides real trade-offs: whether stderr banners exist at all in server mode, whether metric names optimize for charm or greppability, whether docs lead with `vo .` or with mcp.json. The review's read of the market: the agent path is where this tool's differentiation lives (deterministic, budget-honest, sandbox-safe context serving + utility feedback). If you agree, the human-facing CLI becomes a debugging/preview surface for the MCP product — which also answers how much metaphor polish is worth funding.

## Q7. What is `experiments/` / `classic/` policy?

The LSP PoC results (`experiments/lsp_poc` — precision/recall harness comparing regex vs. LSP extraction) suggest you were already evaluating exactly the parser-consolidation question in Q1. What did that experiment conclude? And should `experiments/`, `classic/python` (deprecated reference), and `.llm_archive/` be excluded from vo's own default analysis universe (the review says yes — they poisoned both `--explore` rankings and zoom resolution), or is self-analysis-including-history a deliberate dogfooding choice?

## Q8. `pm_encoder` compatibility: who still depends on it?

The rename cleanup (roadmap 1.8) needs a deprecation policy: are there existing users/scripts on the `pm_encoder` binary name, `.pm_encoder_config.json`, or the `pm_encoder_mcp` tool contract? If none, delete aggressively in one release. If some, ship aliases + warnings for one minor version. Only you know the blast radius.

## Q9. How real is the learning loop meant to be?

`report_utility` accepts `path:score:reason` and the docs frame it as "learning." Today, is anything consuming those scores to change future selection? The review found no evidence of a feedback path into the selection engine (not exhaustively traced). Decide whether it's (a) telemetry to inspect manually, (b) a per-repo prior that reweights selection — the review's recommendation and arguably the product's moat, or (c) aspirational and should be pulled from the tool list until real. Options (a) and (c) are honest; shipping it as-is under a "learning" banner is the same "built but never wired" pattern the review flags everywhere else.

## Q10. Token estimation: how accurate does it need to be?

Everything today is `len/4` heuristics. For budget *enforcement* against real models, errors of ±20% (observed) mean blown context windows. Options: calibrate `len/4` per format and accept ±5-10%; bundle a real tokenizer (e.g. tiktoken-rs class) behind a feature flag for exact counts at some binary-size/speed cost; or let the MCP client supply counts. Depends on how strict you want the "N-token context" promise to be — which is, per the review, the product's core promise.

---

*Answer Q1-Q3 and the Phase 2 consolidation can start without risk of rework; Q4-Q10 can be answered incrementally.*
