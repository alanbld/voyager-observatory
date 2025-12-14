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

## Current Status (Dec 14, 2025)

### Parity Metrics
- **Test Vectors:** 9/10 passing (90% active parity)
- **Python Coverage:** 95%
- **Rust Coverage:** TBD (cargo tarpaulin setup pending)
- **Timeline:** 13 days ahead of original schedule

### Milestones Completed
- ✅ v0.1.0 - Foundation (Rust)
- ✅ v0.2.0 - Core Serialization (Rust)
- ✅ v0.3.0 - Config System (Rust, 80% parity)
- ✅ v0.4.0 - Serialization Vectors (100% parity)

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

**Last Updated:** December 14, 2025
**Researchers:** Multi-AI Development Team (Opus Architect, Sonnet Orchestrator, Claude Code)
