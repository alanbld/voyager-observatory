# The AgiLLM Manifesto
## Toward an Intuitive Operating System for AI Collaboration

**Version:** 1.0 | **Date:** Dec 16, 2025
**Status:** Vision Document

---

## 1. The Core Philosophy: "The iPhone for LLMs"

Current tools require complex prompting. The AgiLLM ecosystem aims for **Intuitive Context**:

* The context provided to an LLM should contain **Affordances**—clear, self-describing paths to get more information.
* The LLM should not need to be "taught" the tool; the tool's output should guide the LLM.

---

## 2. The Architecture

### Layer 1: The Engine (`pm_encoder`)

* **Role:** Mechanism. Stateless, deterministic, high-performance.
* **Responsibility:** I/O, Parsing, Truncation, Token Counting.
* **Key Feature:** **Context Lenses** (Views of the code).
* **Status:** Python v1.6.0 (Prod), Rust v0.5.0 (100% Parity).

### Layer 2: The Operating System (`CFD`)

* **Role:** Policy. Stateful, adaptive, intelligent.
* **Responsibility:** Memory Management, Token Budgeting, Session Handoffs.
* **Key Feature:** **Shadow Intelligence** (Human intent linked to code).
* **Status:** Prototype (Python + Rust Core).

### The Relationship

```
┌─────────────────────────────────────────────────────────────┐
│                    AgiLLM Ecosystem                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │            CFD (The Operating System)                │   │
│  │  • Memory Management    • Token Budgeting           │   │
│  │  • Session Handoffs     • Shadow Intelligence       │   │
│  │  • Adaptive Learning    • Multi-Agent Orchestration │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │            pm_encoder (The Engine)                   │   │
│  │  • Plus/Minus Format    • MD5 Checksums             │   │
│  │  • Context Lenses       • Structure Mode            │   │
│  │  • Streaming Output     • Language Analyzers        │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              LLM Context Window                      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**CFD decides WHAT. pm_encoder decides HOW.**

---

## 3. The Roadmap: Bridging the Gap

### Step 1: Token Awareness (The Currency)

* **Feature:** Add `--max-tokens` to `pm_encoder`.
* **Why:** LLMs operate on token limits, not line counts. To manage a budget, we must count the cost.

### Step 2: The Fractal Protocol (The Interface)

* **Feature:** Interactive Zoom.
* **Concept:**
  1. **Macro:** Directory Tree + High-level READMEs.
  2. **Meso:** Class Signatures + Docstrings (Structure Mode).
  3. **Micro:** Full Function Implementation.
* **Mechanism:** `pm_encoder` output includes "Hyperlinks" (commands) to zoom in.
  * *Example Output:* `[Function body truncated. To see logic: pm_encoder src/lib.rs --target-fn process_data]`

### Step 3: The Session Handshake (The Memory)

* **Feature:** Context Fingerprinting.
* **Concept:** When a session ends, `CFD` generates a compressed summary + a hash of the codebase state. The next session resumes exactly where the last left off.

---

## 4. Research Status (The Twins)

* **Hypothesis Proven:** Test-Driven Parity Development (TDPD) accelerates Rust porting by 3-4x.
* **Discovery:** Rust code is 50% less complex (CCN) than Python reference.
* **Next Research:** Can `pm_coach` (Differential Fuzzing) automatically discover edge cases in the wild?

### The Twins: Python + Rust

| Metric | Python v1.6.0 | Rust v0.5.0 |
|--------|---------------|-------------|
| Test Parity | Reference | 100% (25/25) |
| Streaming | Generators | Iterators |
| TTFB | 88ms | 5ms |
| Avg CCN | 6.72 | 3.33 |

---

## 5. The Vision

```
Today:    pm_encoder serializes code for LLMs
Tomorrow: CFD manages context intelligently
Future:   AgiLLM enables seamless human-AI pair programming
```

### Success Metrics

* **Token Reduction:** 75%+ (achieved: 61.9%, targeting 95% with shadows)
* **Productivity Gain:** 10x (measured by context switching overhead)
* **Parity:** 100% between Python and Rust engines

---

**"We are not just building a CLI. We are building the interface through which AIs experience the world of code."**

---

*Last Updated: December 16, 2025*
*Authors: Multi-AI Development Team (Claude Opus, Claude Sonnet, Gemini Pro)*
