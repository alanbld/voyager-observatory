# pm_encoder: Strategic Vision 2026

## The Universal Context Engine

> *From CLI tool to ecosystem platform*

---

## Executive Summary

pm_encoder began as a Python script for serializing codebases into AI-digestible context. With the Rust engine achieving byte-level parity (v0.9.1), we now have the foundation for something larger: **a universal context engine** that runs anywhere—CLI, browser, IDE, or server.

This document outlines the strategic path from v1.x to v3.0, establishing pm_encoder as the standard for codebase-to-AI context transformation.

---

## 1. The Business Case: Zero-Friction & Privacy

### The Problem
- **Friction**: Users must install Python/Rust, clone repos, run commands
- **Privacy Concern**: Sending code to external services for processing
- **Integration Gap**: No native IDE support, no real-time updates

### The Solution: WASM Engine

**Killer Feature: Zero Data Egress**

```
┌─────────────────────────────────────────────────────────────┐
│                     USER'S BROWSER                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐  │
│  │  Drag/Drop  │───▶│ WASM Engine │───▶│ CONTEXT.txt     │  │
│  │  Files      │    │ (pm_encoder)│    │ (Download)      │  │
│  └─────────────┘    └─────────────┘    └─────────────────┘  │
│                                                             │
│         ⚠️  NO DATA LEAVES THE BROWSER  ⚠️                  │
└─────────────────────────────────────────────────────────────┘
```

- **Enterprises**: Security teams approve instantly (no external API calls)
- **Developers**: Zero install, works on any device with a browser
- **Privacy**: Source code never touches our servers

### VS Code Extension (Powered by WASM)

```
┌────────────────────────────────────────────────────────────┐
│  VS Code                                                   │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Status Bar: [pm_encoder: 847 files | 1.2MB context] │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  Commands:                                                 │
│  - "pm_encoder: Generate Context" → Clipboard/File        │
│  - "pm_encoder: Preview Context" → Side Panel             │
│  - "pm_encoder: Copy for Claude" → Optimized for Claude   │
│                                                            │
│  Settings:                                                 │
│  - Lens: [architecture ▼]                                  │
│  - Token Budget: [100k]                                    │
│  - Auto-update on save: [✓]                                │
└────────────────────────────────────────────────────────────┘
```

---

## 2. The Research Extension: Platform Parity

### Hypothesis

> The WASM build must match the CLI binary **byte-for-byte** on identical inputs.

### Why This Matters

1. **Trust**: Users must trust that browser output equals CLI output
2. **Testing**: Single test suite validates all platforms
3. **Debugging**: Reproduce issues across environments

### Methodology: Test Vector Reuse

```
┌─────────────────────────────────────────────────────────────┐
│                    TEST VECTOR SUITE                        │
│                                                             │
│  test_vectors/*.json                                        │
│  ├── basic_serialization.json                               │
│  ├── binary_detection.json                                  │
│  ├── truncation_modes.json                                  │
│  └── ...                                                    │
│                                                             │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────────┐   │
│  │ Python CLI  │   │  Rust CLI   │   │   Rust WASM     │   │
│  │   v1.7.0    │   │   v2.0.0    │   │    v2.1.0       │   │
│  └──────┬──────┘   └──────┬──────┘   └───────┬─────────┘   │
│         │                 │                  │              │
│         └────────────────▼──────────────────┘              │
│                    BYTE-FOR-BYTE                            │
│                      IDENTICAL                              │
└─────────────────────────────────────────────────────────────┘
```

### WASM Test Harness

```rust
// tests/wasm_parity.rs
#[wasm_bindgen_test]
fn test_vector_parity() {
    let vectors = load_test_vectors("test_vectors/");
    for vector in vectors {
        let wasm_output = pm_encoder_wasm::serialize(&vector.input);
        let expected = vector.expected_output;
        assert_eq!(wasm_output, expected, "Vector: {}", vector.name);
    }
}
```

---

## 3. The Roadmap

### Version Timeline

```
2024 Q4          2025 Q1          2025 Q2          2025 Q3+
   │                │                │                │
   ▼                ▼                ▼                ▼
┌──────┐        ┌──────┐        ┌──────┐        ┌──────┐
│v1.7.0│        │v2.0.0│        │v2.1.0│        │v3.0.0│
│Python│───────▶│ Rust │───────▶│ WASM │───────▶│Semant│
│ Ref  │        │ CLI  │        │Engine│        │  ic  │
└──────┘        └──────┘        └──────┘        └──────┘
   │                │                │                │
   │                │                │                │
Reference      Production       Universal       Next-Gen
& Prototyping   Workhorse        Client         Chunking
```

### v1.x (Python) - The Reference & Prototyping Lab

**Role**: Rapid feature prototyping, plugin development

| Feature | Status |
|---------|--------|
| Core serialization | ✅ Complete |
| Context Lenses | ✅ Complete |
| Token Budgeting | ✅ Complete |
| Priority Groups | ✅ Complete |
| Plugin System | ✅ Complete |
| Language Analyzers | ✅ Complete |

**Future**: Python remains the innovation testbed. New features prototype here first.

---

### v2.0 (Rust CLI) - The High-Performance Workhorse

**Role**: Production deployment, performance-critical use cases

| Feature | Status |
|---------|--------|
| Core serialization | ✅ Byte-parity (v0.9.1) |
| Context Lenses | ✅ Complete |
| Token Budgeting | ✅ Complete |
| Priority Groups | ✅ Complete |
| Init-Prompt | ✅ Complete |
| Library-First Architecture | 🔄 In Progress |

**v2.0 Release Criteria**:
1. ✅ Byte-level parity with Python
2. 🔄 Library-First refactor (ContextEngine)
3. ⬜ Publish to crates.io
4. ⬜ GitHub Releases with binaries

---

### v2.1 (WASM) - The Universal Client

**Role**: Browser, IDE, embedded contexts

| Feature | Target |
|---------|--------|
| WASM compilation | wasm32-unknown-unknown |
| JavaScript bindings | wasm-bindgen |
| Web demo | pm-encoder.dev |
| VS Code extension | Marketplace |
| npm package | @pm-encoder/wasm |

**Architecture Requirements**:
```rust
// Pure functions only - no std::fs, no network
pub fn process_content(
    files: &[(String, String)],  // (path, content) pairs
    config: &EngineConfig,
) -> Result<String, EngineError>
```

---

### v2.2 (Live Server) - The LSP-Style Daemon

**Role**: Real-time context for IDE integration

```bash
$ pm_encoder serve --port 8080 --watch .
[INFO] Watching 1,247 files
[INFO] WebSocket server: ws://localhost:8080
[INFO] REST API: http://localhost:8080/api
```

| Endpoint | Description |
|----------|-------------|
| `GET /context` | Full serialized context |
| `GET /context?lens=security` | Lens-filtered context |
| `WS /stream` | Real-time updates on file change |
| `POST /files` | Ad-hoc file processing |

**Performance Target**: <10ms incremental update latency

---

### v3.0 (Semantic) - AST Chunking with Tree-sitter

**Role**: Next-generation context that understands code structure

**Current Limitation**: Line-based truncation breaks semantic units

```python
# BEFORE: Line truncation breaks mid-function
def calculate_tax(income, deductions):
    taxable = income - deductions
    if taxable <= 10000:
        rate = 0.1
# --- TRUNCATED ---
```

**Future: Semantic Chunking**

```python
# AFTER: Complete semantic units preserved
def calculate_tax(income, deductions):
    """Calculate tax based on income and deductions."""
    # [Implementation: 15 lines, complexity: 3]
    ...

def apply_credits(tax, credits):
    """Apply tax credits to calculated tax."""
    # [Implementation: 8 lines, complexity: 2]
    ...
```

**Tree-sitter Integration**:
```rust
use tree_sitter::{Parser, Language};

pub fn semantic_chunk(content: &str, lang: Language) -> Vec<SemanticUnit> {
    let tree = parser.parse(content, None).unwrap();
    extract_units(tree.root_node())
}
```

---

## 4. Architecture: Library-First Pattern

### The Constraint

> **WASM cannot do I/O.** Therefore, the core engine must be I/O-free.

### Current Architecture (I/O-Coupled)

```
┌─────────────────────────────────────────┐
│              lib.rs                     │
│  ┌─────────────────────────────────┐    │
│  │  serialize_project(path)        │    │
│  │    └── walk_directory(path)  ◀──┼──── fs::read_dir
│  │         └── read_file(path) ◀───┼──── fs::read
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

### Target Architecture (Library-First)

```
┌─────────────────────────────────────────────────────────────┐
│                        lib.rs                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  ContextEngine                                        │  │
│  │    - config: EngineConfig                             │  │
│  │    - lens_manager: LensManager                        │  │
│  │                                                       │  │
│  │  + process_file(&path, &content) -> ProcessedFile    │◀─── PURE
│  │  + serialize_files(&[ProcessedFile]) -> String       │◀─── PURE
│  │  + generate_context(&[(path, content)]) -> String    │◀─── PURE
│  └───────────────────────────────────────────────────────┘  │
│                            ▲                                │
│                            │                                │
│  ┌─────────────────────────┴─────────────────────────────┐  │
│  │  I/O Adapters (CLI only)                              │  │
│  │    - walk_directory() -> Vec<(PathBuf, String)>       │  │
│  │    - read_file() -> String                            │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────┐    ┌─────────────────────┐
│      CLI Binary     │    │    WASM Module      │
│  (with I/O adapter) │    │  (pure functions)   │
└─────────────────────┘    └─────────────────────┘
```

---

## 5. Success Metrics

### v2.0 (Rust CLI)
- [ ] 100% test vector pass rate
- [ ] <100ms for 1000-file project
- [ ] crates.io publication
- [ ] 10 GitHub stars

### v2.1 (WASM)
- [ ] Byte-identical to CLI
- [ ] <500KB WASM bundle size
- [ ] VS Code extension published
- [ ] 100 weekly active users

### v2.2 (Live Server)
- [ ] <10ms incremental update
- [ ] WebSocket stability (24hr test)
- [ ] IDE integration docs

### v3.0 (Semantic)
- [ ] Tree-sitter for 5+ languages
- [ ] 30% better context quality (user study)
- [ ] Semantic diff support

---

## 6. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| WASM bundle too large | Slow load times | Aggressive dead code elimination, lazy loading |
| Tree-sitter complexity | Delayed v3.0 | Start with Python/JS only |
| Browser compatibility | Limited reach | Target evergreen browsers only |
| Maintenance burden | Developer burnout | Shared test vectors, automated CI |

---

## Appendix: Key Files

| File | Purpose |
|------|---------|
| `rust/src/lib.rs` | Core engine (ContextEngine) |
| `rust/src/wasm.rs` | WASM bindings (future) |
| `rust/src/bin/main.rs` | CLI binary |
| `test_vectors/*.json` | Cross-platform test suite |
| `docs/STRATEGIC_VISION_2026.md` | This document |

---

*Document Version: 1.0*
*Last Updated: 2024-12*
*Authors: pm_encoder Core Team*
