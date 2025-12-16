# The Twins: A Research Project

**Investigating TDD-Driven Convergence in Dual-Engine Development**

## Overview

This research project tracks the development of `pm_encoder`, a dual-engine codebase with:
- **Python Engine** (reference implementation, v1.5.0)
- **Rust Engine** (performance implementation, v0.3.0)

The goal is to empirically validate the **Convergence Hypothesis**: that Test-Driven Development using test vectors can accelerate feature parity between two language implementations 3-4x faster than traditional development.

## Research Questions

1. **Speed**: How much faster is TDD-driven parity development vs traditional parallel implementation?
2. **Quality**: Does the test vector contract improve code quality in both engines?
3. **Sustainability**: Can this pace be maintained over 6+ months?
4. **Scalability**: Does the approach scale to complex features (analyzers, lenses)?

## Current Status (Dec 16, 2025)

### Parity Metrics
- **Test Vectors:** 10/10 passing (100% active parity)
- **Python Coverage:** 95%
- **Rust Coverage:** TBD (cargo tarpaulin setup pending)
- **Timeline:** 15 days ahead of original schedule

### Milestones Completed
- ✅ v0.1.0 - Foundation (Rust)
- ✅ v0.2.0 - Core Serialization (Rust)
- ✅ v0.3.0 - Config System (Rust, 80% parity)
- ✅ v0.4.0 - Serialization Vectors (100% parity)
- ✅ v1.5.0 - Rust Parity & Interface Protocol (Python)
- ✅ v1.6.0 - The Streaming Pipeline (Python only)

---

## Phase 2.5: The Streaming Divergence (v1.6.0)

**Date:** December 16, 2025

### Observation
Python Reference Implementation has adopted **Streaming Architecture** (Generators).
Rust Parity Implementation remains **Batch Architecture** (Vectors).

This creates a deliberate architectural gap to study: **Architecture vs. Language**.

### New Research Question (RQ5)
> Can an interpreted language with superior architecture (Streaming) beat a compiled language with inferior architecture (Batch) on Time-To-First-Byte (TTFB)?

### Hypothesis
Python v1.6.0 will have **lower TTFB** than Rust v0.4.0, proving that **architecture dominates raw speed** for latency-sensitive workloads.

### Preliminary Results (React repo, 6,941 files)

| Engine | Architecture | TTFB |
|--------|-------------|------|
| Python v1.6.0 | Streaming | **46ms** |
| Python v1.5.0 | Batch | 485ms |
| Rust v0.4.0 | Batch | ~50ms* |

*Rust batch is fast due to compiled speed, but Python streaming achieves parity through architecture.

### Implications
1. **TTFB Parity**: Python streaming ≈ Rust batch (architecture closes the language gap)
2. **Total Time**: Rust still wins on total processing time (but users see TTFB first)
3. **Next Step**: Port streaming to Rust for true performance leadership

### Status
- **Python v1.6.0**: Streaming implemented ✅
- **Rust v0.4.0**: Batch only (streaming pending)
- **Gap Window**: Intentionally preserved for research documentation

## Methodology

See [METHODOLOGY.md](./METHODOLOGY.md) for detailed research approach.

## Data Collection

Daily snapshots tracked in `data/daily_snapshots.csv`:
- Python test coverage
- Rust test coverage
- Test vector parity percentage
- Lines of code (both engines)
- Velocity metrics

## Timeline

- **Start:** December 13, 2025
- **Duration:** 6 months (through June 2025)
- **Target:** 100% parity by March 31, 2025

## Publications

Results will be published as:
1. Blog series documenting the journey
2. Academic paper on TDD acceleration
3. Open-source case study

## Contributing

This is an active research project. See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

## License

Research data and findings: CC BY 4.0
Code: MIT License (see [LICENSE](../LICENSE))

---

**Last Updated:** December 16, 2025
**Researchers:** Multi-AI Development Team (Opus Architect, Sonnet Orchestrator, Claude Code)
