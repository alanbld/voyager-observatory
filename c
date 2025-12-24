This commits the navigation engine—a huge leap in usability. The tool now "understands" the codebase structure, not just file paths.

### 1. Git Checkpoint

```bash
cd rust

# Stage new search module and updated engine/lib
git add src/core/search.rs src/core/engine.rs src/lib.rs

# Commit
git commit -m "feat(zoom): implement cross-file symbol resolution engine

Enables semantic zooming (e.g., --zoom function=name) without file paths.

Key components:
- SymbolResolver: Scans codebase for function/class/struct definitions
- Regex Engines: Language-specific patterns for Rust, Python, TS, Go
- Block Detection: Smart brace/indentation counting for line ranges
- CLI Integration: Auto-resolves symbolic names to file coordinates

Verification:
- 364 tests passing (+6 new search tests)
- Verified with --zoom function=find_symbol smoke tests"
```

---

### 2. Strategy for Fractal v2 Step 3

**Current Status:**
*   ✅ **State**: Sessions work (we remember where we are).
*   ✅ **Navigation**: We can find specific functions (we can go where we want).

**Missing Link**: **Discovery.**
Currently, the user has to *know* the function name to zoom into it. The "Fractal" promise is that looking at `main()` should *reveal* that `init_logger()` and `parse_args()` are the next logical steps, without the user guessing.

**Step 3: Call Graph Analysis & XML Affordances**
We need to parse the *body* of the focused function to identify:
1.  **Outgoing Calls**: What functions does this code call?
2.  **Affordance Injection**: Inject `<zoom_suggestion>` tags into the Claude-XML output so the LLM sees them as clickable menu options.

### Prompt for Step 3

```markdown
Session: 2025-12-20 | pm_encoder-fractal-v2-graph | Turn: 1
Context: architecture lens

feature: Implement Call Graph Analysis for Zoom Suggestions

**Goal:** When zooming into a function, identify the *other* functions it calls and present them as suggested next steps.

## Current State
- We can find a function's body using `SymbolResolver`.
- We can render that body using `ContextEngine`.

## Requirements

### 1. `CallGraphAnalyzer`
Extend `src/core/search.rs` or `src/core/analysis.rs`:
- Input: The source code of the currently zoomed function.
- Operation: Regex scan for function calls (e.g., `my_func(...)`, `Class.method()`).
- Output: A list of `potential_targets` (names of functions called).

### 2. Validation (Filter)
- Just because `scan()` looks like a function call doesn't mean it's in our codebase (it could be `std::println`).
- Use the existing `SymbolResolver` to quick-check if these potential targets exist in the project.
- *Performance constraint*: Limit checks to top 10 unique calls to avoid IO thrashing.

### 3. XML Output Enrichment
Modify `src/core/xml.rs` (Claude-XML formatting):
- When rendering a zoomed context, append a `<related_context>` or `<zoom_menu>` block.
- Format:
  ```xml
  <zoom_menu>
    <option target="function=init_logger">Definition of init_logger</option>
    <option target="function=parse_args">Definition of parse_args</option>
  </zoom_menu>
  ```
- This gives the LLM (and user) a "menu" of where to go next.

## Constraints
- Keep it regex-based for now (no full AST/LSP) for speed.
- Handle common patterns: `fn()`, `object.method()`, `Module::fn()`.
- Ignore keywords (if, while, for).
```

### Execution

Run the commit command above, then feed this prompt to your coding agent. This completes the "Fractal" loop: **Zoom In -> Discover -> Zoom Deeper**.
