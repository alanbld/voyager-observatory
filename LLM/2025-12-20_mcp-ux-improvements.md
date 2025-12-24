# MCP Server UX Improvements - Challenge Prompt

**Context:** pm_encoder is a context serialization tool for LLM workflows. It has an MCP (Model Context Protocol) server mode that exposes tools like `get_context`, `zoom`, `session_list`, `session_create`, and `report_utility`.

**Testing performed by:** Claude Opus 4.5 acting as an LLM consumer of the MCP tools.

---

## Critical Findings from LLM Testing

### Issue 1: Virtual Environment Pollution

When using the MCP tools, `.venv/` contents leak into results:

```
# Asked for security-relevant code
get_context(lens="security", token_budget="5k")

# Got: .venv/lib/python3.9/site-packages/requests/auth.py
# Instead of: actual project source code
```

The zoom tool also suggests `.venv/` paths in its menu:
```xml
<zoom_menu>
  <option target="function=walk" path=".venv/lib/python3.9/site-packages/coverage/bytecode.py:78-79">
</zoom_menu>
```

**Impact:**
- Wastes precious token budget on irrelevant dependencies
- Confuses the LLM about what's "project code" vs "third-party"
- Zoom suggestions are useless when they point to dependencies

### Issue 2: No Intelligent Boundary Detection

The current implementation treats all files equally. It doesn't understand:
- Project boundaries (what's "mine" vs "installed")
- Code relevance (entry points vs deep utilities)
- Dependency graphs (what calls what)

### Issue 3: Zoom Lacks Context

When zooming into a function, only the function body is shown. No surrounding context, no callers, no call graph.

---

## Current Codebase Structure

```
pm_encoder/
├── rust/src/
│   ├── lib.rs              # Core library
│   ├── core/
│   │   ├── engine.rs       # Context engine
│   │   ├── walker.rs       # File walker (DefaultWalker)
│   │   └── zoom.rs         # Zoom implementation
│   ├── server/
│   │   └── mod.rs          # MCP server implementation
│   └── lenses/
│       └── mod.rs          # Lens configurations
├── pm_encoder.py           # Python reference implementation
└── .pm_encoder_config.json # Default config (if exists)
```

Key files to examine:
- `rust/src/core/walker.rs` - The `DefaultWalker` that traverses directories
- `rust/src/server/mod.rs` - MCP tool implementations
- `rust/src/lenses/mod.rs` - Lens priority configurations

---

## Your Challenge

I need you to propose **bold, innovative solutions** that go beyond simple fixes. Consider:

### Tier 1: Immediate Fixes
1. Default exclusion patterns for `.venv/`, `node_modules/`, `__pycache__/`, `.git/`
2. Respect `.gitignore` during file walking
3. Add a `--project-root` boundary concept

### Tier 2: Intelligent Enhancements
4. **Dependency Graph Awareness**: Distinguish "my code" from "installed packages" by analyzing import statements and file locations
5. **Smart Zoom Context**: When zooming to a function, include:
   - N lines of context above/below
   - List of callers (who calls this?)
   - List of callees (what does this call?)
6. **Relevance Scoring**: Files closer to entry points score higher than deep utilities

### Tier 3: Bold Innovations
7. **Semantic Project Boundaries**: Auto-detect project roots by looking for `Cargo.toml`, `package.json`, `pyproject.toml`, `setup.py`, etc. Everything outside these boundaries is "external"
8. **LLM-Optimized Summaries**: For excluded files, provide a one-line summary instead of nothing (e.g., "requests.auth: HTTP authentication handlers (BasicAuth, DigestAuth)")
9. **Adaptive Token Budgeting**: If the LLM keeps zooming into the same area, automatically expand that region's budget in future `get_context` calls
10. **Call Graph Zoom**: New zoom mode that shows the full call chain: `zoom(target="callgraph=handle_request", depth=2)`

---

## Constraints

- The Rust implementation is the primary target (Python is reference only)
- MCP protocol must remain compatible (JSON-RPC 2.0 over stdio)
- Performance matters: should handle 100k+ file repositories
- Solutions should be implementable incrementally

---

## Deliverables Expected

1. **Architecture proposal**: How would you restructure the walker/zoom to support project boundaries?
2. **Implementation sketch**: Pseudocode or Rust snippets for at least 2 Tier 2+ features
3. **Edge cases**: What happens with monorepos? Workspaces? Symlinks?
4. **Trade-offs**: What are the costs (performance, complexity) of each approach?

---

## Bonus Challenge

The `report_utility` tool lets LLMs report how useful a file was. Currently it just stores an EMA score:

```
Utility reported for 'rust/src/lib.rs': 0.90 → 0.62
```

**How could this data be used to make future `get_context` calls smarter?**

Ideas to explore:
- Prioritize high-utility files in budget allocation
- Auto-exclude files that are consistently rated low
- Build a "project fingerprint" of what matters for this codebase
- Cross-session learning: if multiple LLMs rate the same files highly, trust that signal

---

*Challenge issued by Claude Opus 4.5 after real-world testing of the MCP server UX.*
