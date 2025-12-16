# CFD System Architecture - Deep Dive

**Created:** December 16, 2025
**Purpose:** Document the relationship between CFD and pm_encoder
**Status:** Reference Architecture

---

## What is CFD?

**CFD = Context Fractal Distributed**

The CFD Protocol Suite implements the Context Fractal Distributed protocol for LLM context management, achieving massive token reduction while preserving semantic integrity through adaptive learning and vector-based retrieval.

> "CFD Protocol is evolving into the world's first Human-AI Collaborative Context Intelligence Platform"

---

## Executive Summary

**CFD is NOT just "Shadow Files"** - it's a comprehensive **Human-AI Collaborative Context Intelligence Platform** with multiple interconnected layers.

**pm_encoder is CFD's serialization backend** - the bridge between machine intelligence and human readability.

---

## The Five Layers of CFD

```
┌─────────────────────────────────────────────────────────────────┐
│                    CFD Unified Platform                         │
├─────────────────────────────────────────────────────────────────┤
│  Layer 5: Multi-Agent Orchestration                             │
│  ├── Agent Coordination Protocol                                │
│  ├── Analyst + Coder agents                                     │
│  └── Parallel development streams                               │
├─────────────────────────────────────────────────────────────────┤
│  Layer 4: Collaboration Layer                                   │
│  ├── Context Personas (debug/feature modes)                     │
│  ├── Co-Creation, Co-Planning, Co-Tasking                       │
│  └── Real-time guidance & interrupts                            │
├─────────────────────────────────────────────────────────────────┤
│  Layer 3: Learning Layer                                        │
│  ├── EMA (Exponential Moving Average) learning                  │
│  ├── Pattern extraction from shadows                            │
│  └── Adaptive context optimization                              │
├─────────────────────────────────────────────────────────────────┤
│  Layer 2: Memory Layer                                          │
│  ├── Four-tier memory architecture                              │
│  ├── Progressive Context Revelation                             │
│  └── Self-editing memory blocks                                 │
├─────────────────────────────────────────────────────────────────┤
│  Layer 1: Shadow Layer (Foundation)                             │
│  ├── .cfd shadow files                                          │
│  ├── Human-authored code essence                                │
│  └── 95% initial token reduction                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Map

| Directory | Purpose | Key Files |
|-----------|---------|-----------|
| `cfd_protocol/` | Core Python package | Main protocol implementation |
| `cfd_protocol/vector_engine/` | **Semantic Search** | `faiss_index.py`, `embedding_generator.py`, `cosine_similarity.py` |
| `cfd_protocol/cfd_protocol/` | Protocol core | `adaptive_learning_v21.py`, `serializer_hybrid.py`, `error_recovery.py` |
| `cfd_protocol/session/` | Session management | Session state persistence |
| `cfd-rust-core/` | **Performance Layer** | Rust implementation with PyO3 bindings |
| `tools/` | CLI tools | `cfd_shadow_cli.py`, `cfd_cli.py` |

---

## The Vector Engine Role

The Vector Engine provides **semantic similarity search**:

```python
# cfd_protocol/vector_engine/
├── embedding_generator.py  # Convert code → vectors
├── faiss_index.py          # Fast similarity search (FAISS)
└── simple_cosine_similarity.py  # Fallback without FAISS
```

**Purpose:** Enable semantic context detection - find relevant files by meaning, not just keywords.

---

## Memory Architecture (Four Tiers)

Inspired by Letta (formerly MemGPT):

| Tier | Name | Purpose | Persistence |
|------|------|---------|-------------|
| 1 | Working Memory | Current task context | Session |
| 2 | Short-term | Recent interactions | Hours |
| 3 | Long-term | Learned patterns | Permanent |
| 4 | Archival | Full history | Compressed |

---

## Context Personas

Dynamic configuration profiles for different task types:

| Persona | Use Case | Priority Focus |
|---------|----------|----------------|
| `cfd_debugging_mode.yaml` | Bug hunting | Suspect files + trait interfaces |
| `cfd_feature_mode.yaml` | New features | Architecture docs + exemplars |

**Key Insight:** Personas are **pm_encoder lenses on steroids** - they include user-configurable target files.

---

## The Rust Core

```toml
# cfd-rust-core/Cargo.toml
[dependencies]
pyo3 = "0.25.1"      # Python bindings
tiktoken-rs = "0.7"  # Token counting
walkdir = "2"        # File traversal
```

**Modules:**
- `learning/engine.rs` - Learning algorithms
- `context/resolver.rs` - Context resolution
- `protocol/traits.rs` - Core interfaces

**Output:** `rust_bindings.cpython-311-x86_64-linux-gnu.so` (38MB compiled)

---

## Where pm_encoder Fits

```
┌─────────────────────────────────────────────────────────────────┐
│                         CFD Platform                            │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ Shadow Layer │───▶│ Memory Layer │───▶│ Output Layer │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                   │               │
│         ▼                   ▼                   ▼               │
│  ┌──────────────────────────────────────────────────────┐      │
│  │              pm_encoder (SERIALIZATION BACKEND)       │      │
│  │  • Plus/Minus format                                  │      │
│  │  • MD5 checksums                                      │      │
│  │  • Human-readable output                              │      │
│  │  • Structure mode truncation                          │      │
│  └──────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

**pm_encoder is a SERIALIZATION BACKEND** - not a core engine, not a plugin, but the **output formatter** that makes CFD's intelligent context selection human-readable.

---

## Integration Points for pm_encoder v1.7.0

| CFD Feature | pm_encoder Enhancement |
|-------------|------------------------|
| Shadow Files (.cfd) | New `--lens shadow` that reads .cfd metadata |
| Context Personas | Pre-configured lens profiles with user targets |
| Progressive Revelation | Three-stage output mode |
| Vector Engine | Semantic file selection (future) |
| Memory Tiers | Context state persistence |

---

## The AgiLLM Ecosystem

CFD is part of the larger **AgiLLM (Agile LLM Orchestration)** framework:

```
AgiLLM Ecosystem
├── CFD Protocol (Intelligence Layer)
│   ├── Shadow Layer
│   ├── Memory Layer
│   ├── Learning Layer
│   └── Multi-Agent Orchestration
│
├── pm_encoder (Serialization Layer)
│   ├── Context Lenses
│   ├── Structure Mode
│   └── Plus/Minus Format
│
└── Integration Points
    ├── CFD-pm_encoder Hybrid Protocol
    ├── Progressive Context Revelation
    └── Human-Readable Intelligence
```

---

## Key Documents

| Document | Location | Purpose |
|----------|----------|---------|
| Unified Vision Roadmap | `CFD/docs/sprints/CFD_Unified_Vision_Roadmap.md` | Master roadmap |
| Shadow File Spec | `pm_encoder/docs/research/cfd/SHADOW_SPEC.md` | ADR-015 |
| Hybrid Protocol | `pm_encoder/docs/research/cfd/HYBRID_PROTOCOL.md` | Integration design |
| Reference CLI | `pm_encoder/docs/research/cfd/reference_implementation.py` | Shadow CLI |

---

## Conclusion

**CFD = Intelligence + Memory + Learning + Collaboration + Multi-Agent**

**pm_encoder = Human-Readable Serialization Backend**

The relationship is:
- **CFD decides WHAT** to include (intelligence)
- **pm_encoder decides HOW** to format it (serialization)

For v1.7.0, we should implement:
1. **Shadow Lens**: Read `.cfd` files to enhance context
2. **Persona Mode**: User-configurable target patterns
3. **Progressive Output**: Shadow-first → Selective → Full

---

**Last Updated:** December 16, 2025
**Source:** CFD Project Analysis (~/projects/CFD)
**Researchers:** Multi-AI Development Team
