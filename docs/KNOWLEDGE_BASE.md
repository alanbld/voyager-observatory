# pm_encoder Knowledge Base
## Single Source of Truth for AI Context
**Last Updated:** 2025-12-13 | **Format Version:** 1.0

> This document is optimized for AI consumption. It consolidates project state, decisions, and roadmap into a token-efficient reference.

---

## Quick Status

```
Python Engine: v1.3.1 (production, 95% coverage)
Rust Engine:   v0.1.0 (foundation, library-first)
Architecture:  "The Twins" - parallel development
License:       MIT (both engines)
Repository:    Monorepo (Python + Rust)
```

---

## What pm_encoder Does

**One-liner:** Serializes codebases into AI-optimized context using the Plus/Minus format.

**The Problem:** Sharing project context with AI is manual, inconsistent, and wasteful.

**The Solution:** Intent-based serialization with "Context Lenses" that understand *what you're trying to do*, not just *what files to include*.

**Core Innovation:** Structure-aware truncation achieves 70-94% token reduction while preserving semantic understanding.

---

## Architecture: The Twins

```
pm_encoder/
├── pm_encoder.py              # Python Engine (mature)
├── rust/                      # Rust Engine (growing)
│   ├── src/lib.rs            # The Brain (reusable core)
│   └── src/bin/main.rs       # The Interface (CLI)
├── tests/                     # Python tests
├── test_vectors/              # Shared contract
└── .pm_encoder_config.json    # Shared configuration
```

**Key Decision:** Both engines share config format and produce byte-identical output.

### Why Two Engines?

| Engine | Strength | Use Case |
|--------|----------|----------|
| Python | Rapid development, accessibility | Development, prototyping, reference |
| Rust | Performance (10x), WASM-ready | CI/CD, large repos, IDE integration |

**The Contract:** Python generates test vectors. Rust must reproduce them exactly.

---

## Key Features (Python v1.3.1)

### Context Lenses
Intent-based presets that configure serialization automatically:

| Lens | Purpose | Token Reduction |
|------|---------|-----------------|
| `architecture` | System design, big picture | 80-90% |
| `debug` | Recent changes, bug hunting | 60-70% |
| `security` | Auth, dependencies, configs | 70-80% |
| `onboarding` | Project overview, entry points | 85-95% |

**Usage:** `pm_encoder . --lens architecture`

### Truncation Modes

| Mode | Behavior |
|------|----------|
| `simple` | Cut at line N |
| `smart` | Preserve structure boundaries |
| `structure` | Extract signatures only (maximum compression) |

### Language Analyzers
Native support: Python, JavaScript/TypeScript, Rust, Shell, Markdown, JSON, YAML

---

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Plugin Language | Lua (future) | Sandboxable, portable across Python/Rust/WASM |
| Plugin Architecture | Declarative JSON + Lua | 90% declarative, 10% computed |
| Business Model | Open Core | All engines MIT; revenue from enterprise features |
| Rust Timeline | December 2025 (accelerated) | Python validates design; parallel de-risks |
| Monorepo vs Polyrepo | Monorepo | Shared config, test vectors, documentation |
| Rust Structure | Library-first | Enables WASM and PyO3 bindings |

---

## Roadmap (Accelerated)

### Now: Foundation (December 2025)
- [x] Python v1.3.1 with Context Lenses, Structure Mode
- [x] Rust v0.1.0 skeleton (library-first)
- [x] Test vector infrastructure
- [ ] Rust v0.2.0: Core serialization

### Q1 2026: Feature Parity
- [ ] Rust v0.3.0-0.6.0: Analyzers, Lenses, Truncation
- [ ] Lua plugin system (optional dependency)
- [ ] Model-aware lenses
- [ ] VS Code extension preview

### Q2 2026: Production Rust
- [ ] Rust v1.0.0: Full parity with Python
- [ ] WASM module
- [ ] Binary distribution
- [ ] 10x performance validated

### Q3-Q4 2026: Intelligence Layer
- [ ] Context server mode
- [ ] Bidirectional context negotiation
- [ ] Cross-AI session orchestration
- [ ] MCP integration

---

## The Plus/Minus Format

```
++++++++++ path/to/file.ext ++++++++++
[file content]
---------- path/to/file.ext [MD5] path/to/file.ext ----------
```

**Rules:**
- 10 plus signs, 10 minus signs (exactly)
- POSIX paths, relative to project root
- MD5 of UTF-8 content, hex lowercase
- Content ends with newline

---

## Multi-AI Development Protocol

pm_encoder is developed by a coordinated AI team:

| AI | Role | Specialty |
|----|------|-----------|
| Claude.ai (Opus/Sonnet) | Architect, Orchestrator | Strategy, documentation, coordination |
| AI Studio (Gemini) | Analyst, Designer | Performance analysis, feature design, ultrathink |
| Claude Code Server | Implementer | Code generation, testing, deployment |

**Session Format:**
```
Session: YYYY-MM-DD | pm_encoder-{context} | Turn: N
Context: [serialized|partial|minimal]
```

---

## File Recommendations for AI Context

### Keep (Essential)
- `pm_encoder.py` - Production code
- `rust/` - Rust engine
- `tests/` - Test suite
- `docs/BLUEPRINT.md` - Strategic reference
- `docs/THE_TWINS_ARCHITECTURE.md` - Dual-engine design philosophy
- `docs/RUST_GROWTH_STRATEGY.md` - Rust implementation roadmap
- `.pm_encoder_config.json` - Shared config
- `test_vectors/` - Shared contract

### Keep (Narrative/Marketing)
- `docs/pm_encoder_story.html` - Journey documentation
- `docs/THE_TURING_AUDIT.md` - Validation story

### Create (This Session)
- `test_vectors/` - Shared Python/Rust contract
- `Makefile` - Cross-engine orchestration
- `docs/KNOWLEDGE_BASE.md` - This file

---

## Token Budget Guidance

When sharing pm_encoder context with AI:

| Task | Recommended Context |
|------|---------------------|
| Bug fix | Relevant file + test |
| New feature | Main file + BLUEPRINT + backlog |
| Rust development | RUST_GROWTH_STRATEGY + rust/ + test_vectors/ + pm_encoder.py (reference) |
| Architecture discussion | KNOWLEDGE_BASE + THE_TWINS_ARCHITECTURE + BLUEPRINT |
| Documentation | README + story + audit |

**Meta-application:** Use pm_encoder to serialize pm_encoder context:
```bash
pm_encoder . --lens architecture -o context.txt
```

---

## Success Metrics

### Technical
- Test coverage: >95% (Python), >80% (Rust target)
- Output parity: 100% byte-identical between engines
- Performance: Rust 10x faster than Python

### Adoption (18-month targets)
- GitHub stars: 25,000
- Monthly downloads: 500,000
- Community patterns: 150
- Languages supported: 30

---

## The Meta-Tool Paradox

pm_encoder serializes projects for AI consumption, including itself. This creates recursive value:

1. Improve pm_encoder → Better AI context
2. Better AI context → Better AI assistance
3. Better AI assistance → Faster pm_encoder improvement
4. Repeat

**The Manifesto:** "A tool that helps AI understand code better helps AI build better tools to help AI understand code better."

---

**Document maintained by:** Multi-AI Development Team
**Canonical location:** Project knowledge base / Claude.ai memory
**Update frequency:** After each significant release or decision
