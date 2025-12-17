# CFD Integration Decision Record
## Multi-AI Consensus: Claude.ai + AI Studio (Gemini)
**Date:** 2025-12-17
**Status:** APPROVED - Ready for Implementation
**Participants:** Claude.ai (Architect), AI Studio/Gemini (Performance Analyst)

---

## Executive Decision

**Adopt CFD as the "Policy Layer" blueprint while keeping pm_encoder as the "Mechanism Layer."**

### The Core Insight

| Aspect | pm_encoder | CFD |
|--------|------------|-----|
| Philosophy | Stateless, Deterministic | Stateful, Adaptive |
| Strength | Implementation | Architecture |
| Role | Mechanism (the "how") | Policy (the "what") |

**Integration Strategy:** Re-implement CFD's best structural ideas using pm_encoder's TDD methodology. Do NOT import CFD's codebase or statefulness.

---

## Approved Integrations

### ✅ Integrate Immediately (v1.6.0)

#### 1. Priority Groups (Enhanced Lenses)

**What:** Add numeric priority field to lens configuration.

**Schema Change:**
```json
{
  "lenses": {
    "architecture": {
      "groups": [
        { "pattern": "src/core/**", "priority": 100, "truncate": "structure" },
        { "pattern": "src/utils/**", "priority": 50, "truncate": "smart" },
        { "pattern": "tests/**", "priority": 0, "truncate": "simple" }
      ]
    }
  }
}
```

**Why:** Enables "Token Budgeting" - the killer feature for large context windows.

**Effort:** Small
**Risk:** Low
**Test Vectors Required:** Yes - `budgeting` category

#### 2. Protocol Adapter Interface (Python)

**What:** Define abstract `RepoSource` and `OutputSink` interfaces.

**Code Pattern:**
```python
from abc import ABC, abstractmethod

class RepoSource(ABC):
    @abstractmethod
    def walk_files(self) -> Iterator[Path]:
        pass
    
    @abstractmethod
    def read_file(self, path: Path) -> str:
        pass

class OutputSink(ABC):
    @abstractmethod
    def write_chunk(self, chunk: str) -> None:
        pass
    
    @abstractmethod
    def finalize(self) -> None:
        pass
```

**Why:** Prepares streaming architecture for MCP without coupling to specific delivery.

**Effort:** Medium
**Risk:** Low

---

### ✅ Integrate at Rust v1.0.0 (April 2026)

#### 3. Protocol Adapter Trait (Rust)

**What:** Expose trait-based API in `lib.rs` for swappable drivers.

**Code Pattern:**
```rust
pub trait ContextRequest {
    fn get_root(&self) -> PathBuf;
    fn get_lens(&self) -> &Lens;
    fn get_token_budget(&self) -> Option<usize>;
}

pub trait ContextResponse {
    fn stream_chunk(&mut self, chunk: &str);
    fn report_meta(&mut self, meta: &Metadata);
}
```

**Why:** Critical for "Library-First" architecture enabling CLI/MCP/WASM drivers.

**Effort:** Medium
**Risk:** Low

#### 4. Shadow File Hook

**What:** Detect `.cfd` or `.pm_encoder_shadow` files during analysis (namespace reservation).

**Why:** Reserves the pattern for future shadow-lite implementation.

**Effort:** Small
**Risk:** Low

---

### ⏳ Defer to v2.0+ (Context Server)

#### 5. EMA Utility Scoring

**Reason for Deferral:**
- Requires persistent state (database/history)
- Breaks deterministic behavior
- Belongs in daemon mode, not CLI

**Implementation Note:** When implemented, must be opt-in (`--adaptive` flag).

#### 6. Progressive Revelation

**Reason for Deferral:**
- Interaction pattern, not serialization pattern
- Requires conversational loop (MCP)
- Fits v3.0 bidirectional context negotiation

---

### ❌ Do Not Integrate

#### Vector Engine / Semantic Search

**Reason:** 
- Heavy dependency (FAISS/Torch)
- Breaks "Zero Dependency" promise
- Should be external tool that *feeds* pm_encoder

#### Full Memory Architecture

**Reason:**
- Domain of the Agent (Cline/Claude), not the Context Tool
- pm_encoder should feed memory, not *be* memory

---

## The Statefulness Principle

**Critical Warning from AI Studio:**

> "If running the same command twice produces different output (because of EMA learning), debugging becomes a nightmare."

**Decision:** pm_encoder core remains **stateless and deterministic**.

**Adaptive features (when added):**
1. Must be explicit opt-in (`--adaptive`)
2. State managed by optional "Brain" plugin
3. Core produces same output for same input

**Implementation Pattern:**
```
┌──────────────────────────────────────────────┐
│           pm_encoder Core (Stateless)        │
│  Same input → Same output (always)           │
└─────────────────────┬────────────────────────┘
                      │
                      │ Optional
                      ▼
┌──────────────────────────────────────────────┐
│         Brain Plugin (Stateful)              │
│  • Tracks file utility over sessions         │
│  • Updates .pm_encoder_config.json           │
│  • Provides --adaptive behavior              │
└──────────────────────────────────────────────┘
```

---

## Updated Roadmap Integration

### v1.6.0 (Python - January 2026)
- [ ] **NEW:** Priority Groups in lens schema
- [ ] **NEW:** Protocol Adapter interfaces (RepoSource, OutputSink)
- [ ] **NEW:** Token budgeting logic
- [ ] **NEW:** Test vector category: `budgeting`
- [ ] Streaming output mode

### v1.7.0 (Python - February 2026)
- [ ] **NEW:** Model-aware lens presets
- [ ] **NEW:** Meta header in output format
- [ ] AI guidance generation

### Rust v1.0.0 (April 2026)
- [ ] **NEW:** Protocol Adapter traits in lib.rs
- [ ] **NEW:** Shadow file detection hook
- [ ] Full feature parity
- [ ] Binary distribution

### v2.0.0 (Context Server - Q3 2026)
- [ ] **NEW:** Daemon mode with file watching
- [ ] **NEW:** Brain plugin with EMA learning
- [ ] **NEW:** Persistent state management
- [ ] MCP server integration

### v3.0.0 (Intelligence Layer - Q4 2026)
- [ ] **NEW:** Progressive revelation protocol
- [ ] **NEW:** Bidirectional context negotiation
- [ ] Cross-session learning
- [ ] Model-aware optimization

---

## Test Vector Requirements

### New Category: `budgeting`

**Purpose:** Validate priority-based file selection under token constraints.

**Test Vector Schema:**
```json
{
  "test_name": "priority_budget_cut",
  "input": {
    "files": [
      { "path": "core/main.py", "tokens": 200, "priority": 100 },
      { "path": "utils/helpers.py", "tokens": 150, "priority": 50 },
      { "path": "tests/test_main.py", "tokens": 300, "priority": 0 }
    ],
    "budget": 300
  },
  "expected": {
    "included": ["core/main.py", "utils/helpers.py"],
    "excluded": ["tests/test_main.py"],
    "total_tokens": 350,
    "budget_exceeded": true,
    "strategy": "cut_lowest_priority"
  }
}
```

---

## Terminology Unification

| CFD Term | pm_encoder Term | Decision |
|----------|-----------------|----------|
| Context Personas | Lenses | Keep **Lenses** |
| Priority Groups | Lens Groups | Keep **Groups** |
| Shadow Files | Metadata Files | Keep **Shadows** (reserved) |
| Utility Score | File Priority | Keep **Priority** |
| Progressive Revelation | Truncation Levels | Keep **Truncation** |

---

## Key Questions Answered

### 1. Is CFD integration worth the complexity?

**Answer:** Only the *structural* parts (Adapters, Priorities). The *cognitive* parts (Learning, Vectors) are too heavy for the core tool.

### 2. What's the minimum viable intelligence layer?

**Answer:** **Priority Groups**. If users can rank files, the tool can intelligently fit context into any token window.

### 3. How do we avoid CFD's implementation gap trap?

**Answer:** By enforcing **Statelessness** in the core. Do not build a database. Build a config file that *can be updated* by an external brain.

---

## Multi-AI Consensus Statement

**Claude.ai (Architect):** The cross-session analysis revealed CFD's strength in architectural vision but weakness in implementation. pm_encoder should learn from CFD's concepts while avoiding its traps.

**AI Studio (Analyst):** CFD provides the blueprint for v2.0's API. Re-implement the best ideas using pm_encoder's rigorous TDD methodology. The Protocol Adapter pattern and Priority Groups deliver 80% of the value with 20% of the complexity.

**Consensus:** 
- ✅ Protocol Independence → Adopt as core architecture
- ✅ Priority Groups → Adopt in v1.6.0
- ⏳ EMA Learning → Defer to v2.0 plugin
- ❌ Vector Engine → Reject (external tool)
- ❌ Full Memory → Reject (agent domain)

---

## Action Items

### Immediate (This Week)
1. [ ] Update `pm_encoder_roadmap_2025_2026.md` with CFD integration milestones
2. [ ] Create GitHub issue for Priority Groups feature
3. [ ] Design token budgeting algorithm
4. [ ] Add `CFD_KNOWLEDGE_BASE.md` to project documentation

### Sprint 7 (Dec 22-28)
1. [ ] Implement Priority field in lens schema (Python)
2. [ ] Create `budgeting` test vector category
3. [ ] Design RepoSource/OutputSink interfaces
4. [ ] **NEW:** Begin Streaming Pipeline refactor (prerequisite for v1.6.0)

### Q1 2026
1. [ ] Implement Protocol Adapter pattern in Python
2. [ ] Port Priority Groups to Rust
3. [ ] **NEW:** Implement Context Store reading (v1.7.0)
4. [ ] **NEW:** Design Context Hydration pattern

---

## AI Studio Round 2: Ecosystem Architecture

### The "State-as-Configuration" Pattern

**The Statefulness Paradox - SOLVED:**

```
┌──────────────────────────────────────────────────────────┐
│  pm_encoder (Stateless Execution)                        │
│  • READS the store                                       │
│  • Same input → Same output (always)                     │
│  • Does not modify state                                 │
└──────────────────────────┬───────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────┐
│  Context Store (.pm_context_store.json)                  │
│  • Passive Knowledge file                                │
│  • Contains: utility scores, shadows, tags               │
│  • Updated by external process                           │
└──────────────────────────┬───────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────┐
│  Learner / Coach (Stateful)                              │
│  • Separate command: `pm_coach learn --feedback "..."`   │
│  • WRITES to the store                                   │
│  • AI agent or human feedback                            │
└──────────────────────────────────────────────────────────┘
```

**The Insight:** "This is the React/Redux pattern applied to CLI tools."
- `pm_encoder` = View (Renderer)
- Store = State
- AI/Human = Reducer

### The AgiLLM Three-Layer Stack

| Layer | Component | Role | Responsibility |
|-------|-----------|------|----------------|
| 3 | utf8dok | Sensor | Binary → Semantic text |
| 2 | CFD | Policy | Memory, Budgeting, Sessions |
| 1 | pm_encoder | Mechanism | Serialization, Truncation |

### Context Store Schema

```json
{
  "version": "1.0",
  "files": {
    "src/lib.rs": {
      "utility_score": 0.95,
      "shadow_path": ".shadow/src/lib.rs.md",
      "tags": ["core", "critical"],
      "summary": "Main library entry point."
    },
    "tests/legacy_test.py": {
      "utility_score": 0.1,
      "tags": ["deprecated", "noise"]
    }
  }
}
```

### Context Hydration Pattern

```rust
struct FileContext {
    path: PathBuf,
    content: String,
    // Injected from Store:
    shadow_summary: Option<String>,
    utility_score: f32,
    last_accessed: u64,
    user_notes: Option<String>,
}
```

**Context Decorators (Abstraction):**
1. `FileSystemDecorator` - size, mtime
2. `GitDecorator` - commit, author
3. `StoreDecorator` - utility scores, shadows

### New Methodologies

#### The Iron Mold Protocol
> "Do not port Molten code."

```
Molten (Python)  →  Solid (Python)  →  Iron (Rust)
  Experimental       Stable API        Performance
```

#### The Annealing Protocol
> "Solve deadlocks by cycling Temperature."

- High Temp (1.2): Visionary brainstorming
- Low Temp (0.15): Pragmatic engineering

---

## Updated Roadmap with AI Studio Input

### v1.6.0 (Streaming Pipeline - January 2026)
**Prerequisite for all future scalability**

- [ ] Refactor Python core to Generators (`yield`)
- [ ] Priority Groups in lens schema
- [ ] Protocol Adapter interfaces
- [ ] Token budgeting logic
- [ ] Test vector category: `budgeting`

### v1.7.0 (Context Store - February 2026)
**Connect CFD (OS) to pm_encoder (Kernel)**

- [ ] Read `.pm_context_store.json`
- [ ] Context Hydration with FileContext struct
- [ ] Virtual Files (Session Context)
- [ ] Utility Score soft filtering
- [ ] Context Decorator pattern

### Rust v1.0.0 (April 2026)
- [ ] Protocol Adapter traits
- [ ] Iterator-based streaming (match Python)
- [ ] Store reading parity
- [ ] Shadow file detection hook

### v2.0.0 (Context Server - Q3 2026)
- [ ] Daemon mode
- [ ] `pm_coach` learner command
- [ ] Store writing (external process)
- [ ] MCP server integration

---

## Final Consensus Statement

### Multi-AI Agreement

**Claude.ai (Architect):**
> "CFD's structural patterns (Adapters, Priorities) align with pm_encoder's philosophy. The State-as-Configuration pattern elegantly preserves determinism while enabling intelligence."

**AI Studio (Strategist):**
> "pm_encoder is the Kernel. CFD is the OS. Keep them cleanly separated. The Store file is the bridge. Do not build a database—build a config file that can be updated by an external brain."

### The Governing Principle

> **"pm_encoder is the View. The Store is the State. The AI is the Reducer."**

This ensures:
- ✅ Deterministic, debuggable execution
- ✅ Infinite intelligence via passive files
- ✅ Clean mechanism/policy separation
- ✅ Future-proof for MCP/WASM delivery

---

**Document Status:** Living decision record
**Review Cycle:** Monthly or at each milestone
**Owner:** pm_encoder Multi-AI Development Team
**Contributors:** Claude.ai, AI Studio/Gemini
