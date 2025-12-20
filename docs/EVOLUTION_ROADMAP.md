# pm_encoder Evolution Roadmap

**Date:** December 20, 2025
**Current Version:** Python v1.7.0 | Rust v1.0.0

---

## Last 5 Implemented Features

### 1. Enhanced Claude-XML with Priority Tiers (v1.0.0+)
**Commit:** `98bda8c` | **Date:** Dec 20, 2025

Semantic XML structure optimized for LLM attention:
- **Priority Tier Grouping**: `<priority_tier level="critical|high">` for attention priming
- **Utility Scores**: `utility="0.92"` attribute from Context Store learning
- **Enhanced Zoom Affordances**: Three action types per truncated file
  - `expand`: Full file zoom
  - `structure`: Signature-only view
  - `full`: No truncation mode

```xml
<attention_map>
  <priority_tier level="critical">
    <hotspot path="src/lib.rs" priority="95" utility="0.92" truncated="true" />
  </priority_tier>
  <coldspots>
    <coldspot path="Cargo.lock" priority="50" dropped="true" />
  </coldspots>
</attention_map>
```

---

### 2. Claude-XML Two-Pass Streaming (v1.0.0)
**Commit:** `c0a9a12` | **Date:** Dec 20, 2025

Solved the "streaming paradox" - accurate token counts with O(1) memory:
- **Utilized Attribute**: `utilized="23463"` in root tag from BudgetReport
- **Attention Map Integration**: Hotspots/coldspots from budget calculation
- **CDATA Streaming**: Proper `]]>` escape handling
- **Zoom Actions**: Fractal Protocol affordances for truncated files

---

### 3. Context Store v2 - Learning Layer (v1.0.0)
**Commit:** `9254c66` | **Date:** Dec 18, 2025

EMA-based utility tracking that learns from AI feedback:
- **Utility Scores**: Exponential Moving Average (α=0.3)
- **Priority Blending**: `final = (static × 0.7) + (learned × 100 × 0.3)`
- **CLI Integration**: `--report-utility "path:score:reason"`
- **MCP Tool**: `report_utility` for AI agent feedback loop
- **Privacy Mode**: Optional path hashing for sensitive projects

---

### 4. CLI Hardening & WASM Validation (v1.0.0)
**Commit:** `e08c3c9` | **Date:** Dec 18, 2025

Production-ready CLI with cross-platform support:
- **Argument Validation**: Comprehensive error messages
- **WASM Build**: Browser-compatible WebAssembly package
- **Test Coverage**: 336 tests passing (276 unit + 28 integration + 29 vectors + 3 doc)
- **Performance**: 10-100x faster than Python reference

---

### 5. Token Budgeting & Priority Groups (v1.7.0)
**Commit:** `6691113` | **Date:** Dec 17, 2025

Intelligence layer for context optimization:
- **Token Budget**: `--token-budget 100k` with shorthand notation
- **Budget Strategies**: `drop`, `truncate`, `hybrid`
- **Priority Groups**: Per-lens file priorities (0-100)
- **Budget Report**: Detailed stderr output of allocation decisions

---

## Next 5 Planned Features

### 1. Fractal Protocol v2 - Zoom Orchestration
**Target:** v1.1.0 | **Priority:** Critical

Multi-file zoom coordination for complex investigations:

```
Features:
├── Bidirectional Zoom
│   ├── Expand: Drill into truncated content
│   └── Collapse: Reduce full files to structure
├── Cross-File Navigation
│   ├── Follow imports/dependencies
│   └── Trace call graphs
├── Zoom Sessions
│   ├── Save/restore zoom state
│   └── Named bookmarks
└── AI-Guided Zoom
    ├── Suggest relevant expansions
    └── Auto-collapse low-utility sections
```

**Implementation:**
- `--zoom-session save|load|list`
- `--zoom-follow-imports`
- `--zoom-depth shallow|medium|deep`

---

### 2. Python Backports (Parity Completion)
**Target:** v1.8.0 | **Priority:** High

Bring Rust-only features to Python reference:

| Feature | Rust | Python Target |
|---------|------|---------------|
| Context Store v2 | ✅ | v1.8.0 |
| claude-xml format | ✅ | v1.8.0 |
| Priority tiers | ✅ | v1.8.0 |
| Utility scores | ✅ | v1.8.0 |
| Report utility CLI | ✅ | v1.8.0 |

**Implementation:**
- Port `ContextStore` class with EMA logic
- Add `--format claude-xml` to Python CLI
- Integrate utility feedback in serialization

---

### 3. Real-Time Watch Mode
**Target:** v1.9.0 | **Priority:** Medium

Live context updates for development workflows:

```bash
# Watch mode - regenerate on file changes
pm_encoder . --watch --format claude-xml

# With debounce and selective updates
pm_encoder . --watch --debounce 500ms --incremental
```

**Features:**
- File system watcher (notify/inotify)
- Incremental context updates (diff-based)
- WebSocket streaming for IDE integration
- Selective regeneration (changed files only)

---

### 4. Multi-Agent Context Sharing
**Target:** v2.0.0 | **Priority:** Medium

Shared context infrastructure for AI agent teams:

```
Architecture:
├── Context Server
│   ├── Centralized context store
│   ├── Agent registration
│   └── Conflict resolution
├── Agent Protocols
│   ├── Request context slice
│   ├── Report utility feedback
│   └── Coordinate zoom actions
└── Privacy Controls
    ├── Per-agent access levels
    └── Audit logging
```

**Use Cases:**
- Multiple Claude instances working on same codebase
- Specialized agents (code review, testing, docs)
- Context handoff between agents

---

### 5. IDE Integration - VS Code Extension
**Target:** v2.1.0 | **Priority:** Medium

Native IDE experience for context management:

```
Features:
├── Context Panel
│   ├── Live token budget visualization
│   ├── File priority indicators
│   └── Lens selector dropdown
├── Inline Annotations
│   ├── Utility score badges
│   ├── Truncation indicators
│   └── Zoom affordance links
├── Commands
│   ├── "Generate Context for Selection"
│   ├── "Add to Context Store"
│   └── "Report Utility"
└── Settings
    ├── Default lens
    ├── Token budget
    └── Auto-regenerate on save
```

**Technical:**
- TypeScript extension using WASM core
- LSP integration for semantic analysis
- Workspace-scoped context stores

---

## Evolution Timeline

```
2025 Q4 (Current)
├── v1.0.0 ✅ Context Store v2, Claude-XML, WASM
├── v1.1.0 🔄 Fractal Protocol v2 (Zoom Orchestration)
└── v1.8.0 📋 Python Backports

2026 Q1
├── v1.9.0 📋 Real-Time Watch Mode
└── v2.0.0 📋 Multi-Agent Context Sharing

2026 Q2
└── v2.1.0 📋 VS Code Extension
```

---

## Technical Debt & Maintenance

### Immediate (Before v1.1.0)
- [ ] Fix 6 Clippy warnings (dead code, unused imports)
- [ ] Add XML schema validation (XSD)
- [ ] Improve error messages for zoom failures

### Short-term (Before v2.0.0)
- [ ] Refactor LensManager for better testability
- [ ] Add benchmarks for token counting accuracy
- [ ] Document MCP server protocol

### Long-term
- [ ] Consider async runtime for watch mode
- [ ] Evaluate gRPC for agent communication
- [ ] Performance profiling on 1M+ file repos

---

## Success Metrics

| Metric | Current | v1.1.0 Target | v2.0.0 Target |
|--------|---------|---------------|---------------|
| Rust Tests | 336 | 400 | 500 |
| Python Tests | 146 | 180 | 200 |
| Test Coverage | 81-85% | 90% | 95% |
| TTFB (Stream) | 5ms | 3ms | 2ms |
| Feature Parity | 95% | 98% | 100% |

---

*Generated by Claude Code for pm_encoder project*
