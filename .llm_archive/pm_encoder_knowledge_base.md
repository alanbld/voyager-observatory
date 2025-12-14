# pm_encoder Knowledge Base
## Single Source of Truth for AI Context
**Last Updated:** 2025-12-13 | **Format Version:** 1.0

> This document is optimized for AI consumption. It consolidates project state, decisions, and roadmap into a token-efficient reference.

---

## Quick Status

```
Python Engine: v1.3.1 (production, 95% coverage)
Rust Engine:   v0.1.0 (skeleton, library-first)
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
├── test_vectors/              # Shared contract (planned)
└── .pm_encoder_config.json    # Shared configuration
```

**Key Decision:** Both engines share config format and produce byte-identical output.

### Why Two Engines?

| Engine | Strength | Use Case |
|--------|----------|----------|
| Python | Rapid development, accessibility | "Just download one file" |
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

Each analyzer understands language-specific structures (classes, functions, imports) for intelligent truncation.

---

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Plugin Language | Lua (future) | Sandboxable, portable across Python/Rust/WASM |
| Plugin Architecture | Declarative JSON + Lua escape hatch | 90% declarative, 10% computed |
| Business Model | Open Core | All engines MIT; revenue from enterprise workflow features |
| Rust Timeline | December 2025 (accelerated from Q2 2026) | Python validates design; parallel development de-risks |
| Monorepo vs Polyrepo | Monorepo | Shared config, test vectors, documentation |
| Rust Structure | Library-first (`lib.rs` + `bin/main.rs`) | Enables WASM and PyO3 bindings |

---

## Roadmap (Accelerated)

### Now: Foundation (December 2025)
- [x] Python v1.3.1 with Context Lenses, Structure Mode
- [x] Rust v0.1.0 skeleton (library-first)
- [ ] Test vector infrastructure
- [ ] Rust v0.2.0: Core serialization (Plus/Minus format)

### Q1 2026: Feature Parity
- [ ] Rust v0.3.0-0.6.0: Analyzers, Lenses, Truncation
- [ ] Lua plugin system (optional dependency)
- [ ] Model-aware lenses (`--lens claude-opus-architecture`)
- [ ] VS Code extension preview

### Q2 2026: Production Rust
- [ ] Rust v1.0.0: Full parity with Python
- [ ] WASM module (compile from lib.rs)
- [ ] Binary distribution (cargo install, homebrew)
- [ ] 10x performance validated

### Q3-Q4 2026: Intelligence Layer
- [ ] Context server mode (long-running, file watching)
- [ ] Bidirectional context negotiation with AI
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

**Extensions (v2.0 planned):**
- `.pm_encoder_meta` header with project summary
- `[pm_encoder: structure]` inline markers
- `[pm_encoder: truncated N lines]` annotations

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

**Handoff Protocol:** Each AI documents decisions, creates artifacts, updates backlog.

---

## File Recommendations for Project

### Keep (Essential)
- `pm_encoder.py` - Production code
- `rust/` - Rust engine
- `tests/` - Test suite
- `docs/BLUEPRINT.md` - Strategic reference
- `.pm_encoder_config.json` - Shared config

### Keep (Narrative/Marketing)
- `docs/pm_encoder_story.html` - Journey documentation
- `docs/pm_encoder_roadmap.html` - Visual roadmap

### Consolidate/Update
- `pm_encoder_backlog.md` - Merge Dec 12 and Dec 13 versions, add Rust track

### Create (Missing)
- `test_vectors/` - Shared Python/Rust contract
- `Makefile` - Cross-engine orchestration
- `CONTRIBUTING.md` - Community guidelines

---

## Token Budget Guidance

When sharing pm_encoder context with AI:

| Task | Recommended Context |
|------|---------------------|
| Bug fix | `pm_encoder.py` + relevant test file |
| New feature | `pm_encoder.py` + `BLUEPRINT.md` + backlog |
| Rust development | `rust/` + test vectors + `pm_encoder.py` (reference) |
| Architecture discussion | This knowledge base + `BLUEPRINT.md` |
| Documentation | `README.md` + story + roadmap |

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

### Business (if commercialized)
- Pro subscribers: 5,000
- Enterprise customers: 20
- MRR: $150K

---

## The Meta-Tool Paradox

pm_encoder serializes projects for AI consumption, including itself. This creates recursive value:

1. Improve pm_encoder → Better AI context
2. Better AI context → Better AI assistance
3. Better AI assistance → Faster pm_encoder improvement
4. Repeat

**The Manifesto:** "A tool that helps AI understand code better helps AI build better tools to help AI understand code better."

---

## Quick Reference

```bash
# Python (production)
python3 pm_encoder.py . --lens architecture -o context.txt

# Rust (skeleton - not yet functional)
cd rust && cargo run -- .

# Run tests
python -m pytest tests/
cd rust && cargo test

# Generate context for AI session
pm_encoder . --lens debug --truncate 50 -o session_context.txt
```

---

**Document maintained by:** Multi-AI Development Team
**Canonical location:** Project knowledge base / Claude.ai memory
**Update frequency:** After each significant release or decision
