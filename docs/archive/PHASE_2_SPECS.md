# Phase 2: Context Infrastructure Specifications
## Protocol Specialization & The Fractal Interface

**Status:** Draft (Awaiting DeepSeek Synthesis)
**Date:** 2025-12-20
**Version:** 2.0.0-alpha

---

## 1. Protocol Adapters (Native LLM Formats)
*Goal: Transition from generic Plus/Minus to model-optimized protocols.*

### 1.1 Claude-Optimized XML (`--format xml`)
*   **Structure:** [Awaiting Spec]
*   **Semantic Headers:** [Awaiting Spec]
*   **Attention ROI:** How we use tags to guide the model's focus.

### 1.2 Markdown Specialization (`--format markdown`)
*   **Structure:** [Awaiting Spec]
*   **Target:** Optimized for GPT-4o and Gemini 1.5 Pro.

---

## 2. The Fractal Protocol (Interactive Zoom)
*Goal: Enable AI Agents to autonomously request deeper context.*

### 2.1 Command Affordances (Hyperlinks)
*   **Syntax:** [Awaiting Spec]
*   **Placement:** Integration within `structure` mode truncation markers.

### 2.2 Micro-Context Arguments
*   **CLI/MCP Parameters:**
    *   `--target-fn <name>`: Extract specific function implementation.
    *   `--target-class <name>`: Extract specific class implementation.
    *   `--depth-level <n>`: Control recursion depth of signatures.

---

## 3. The Reducer Logic (Context Store)
*Goal: Implement the "State-as-Configuration" feedback loop.*

### 3.1 `.pm_context_store.json` Schema
*   **Utility Scores:** (0.0 - 1.0) ranking per file.
*   **Last Accessed:** Timestamp of last AI interaction.
*   **Shadow Links:** Pointers to `.cfd` essence files.

### 3.2 The Feedback Loop (The Reducer)
*   **Mechanism:** How the AI writes back utility data to the store.
*   **Lens Integration:** How the `architecture` lens consumes utility scores to re-rank priority groups.

---

## 📈 Success Metrics (Phase 2)
1.  **Token ROI Factor:** Target > 3.0x improvement over naive packing.
2.  **Agent Autonomy:** Successful "Zoom-In" operations via MCP without human intervention.
3.  **Parity:** 100% byte-level parity maintained for the default format.
