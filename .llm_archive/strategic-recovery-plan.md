I apologize if the previous content triggered a safety filter—sometimes detailed technical logs or diffs can trigger false positives.

Here is the **Strategic Recovery Plan** to close the 16% gap and reach 100% parity. I have broken this down into **6 High-Impact Prompts** for Claude Code Server.

### 🗺️ The Parity Roadmap (6 Steps)

1.  **🔴 Critical Fix:** Repair Pattern Matching (Fixes 3 failing tests).
2.  **🟡 Sorting Logic:** Implement `mtime`/`ctime` sorting in Rust.
3.  **🟡 Truncation Core:** Implement `Simple` truncation (line counting).
4.  **🟢 Smart Truncation:** Connect Analyzers to Truncation logic (`Smart`/`Structure` modes).
5.  **🟢 Missing Analyzers:** Port Markdown, JSON, and YAML analyzers.
6.  **🔵 Lens System:** Implement the `LensRegistry` and config merging.

---

### 🚀 Step 1: The Critical Fix (Execute This Now)

This prompt addresses the root cause of the 3 failing tests: broken filtering logic.

**Copy and paste to Claude Code:**

```bash
claude "fix: Repair Rust Pattern Matching Logic (Config Parity)

**Context:**
The Rust engine is failing 3 critical test vectors:
1. `config_03_ignore_patterns` (*.log not ignored)
2. `config_04_include_patterns` (Includes not restricting output)
3. `config_05_pattern_precedence` (Include doesn't override Ignore)

**Task:**
Refactor the file filtering logic in `rust/src/lib.rs` (specifically inside `walk_directory` or `serialize_project`).

**Requirements:**
1.  **Use `globset`**: Ensure patterns are compiled into `GlobSet` for performance.
2.  **Implement Precedence Logic**:
    -   *Step A (Whitelist)*: If `include_patterns` is not empty, check if file matches. If NO match, SKIP.
    -   *Step B (Blacklist)*: Check if file matches `ignore_patterns`.
    -   *Step C (Override)*: If file matches Ignore, check if it *also* matches Include.
        -   If matches Include -> KEEP (Override).
        -   If no match Include -> SKIP.

**Verification:**
Run `cargo test --test test_vectors` and confirm `config_03`, `config_04`, and `config_05` pass.
"
```

---

### 🔮 The Future Prompts (Save for later)

Once Step 1 is Green, proceed with these:

#### Step 2: Sorting
```bash
claude "feature: Implement Rust File Sorting

**Context:**
The Rust engine currently defaults to directory order. Python supports `--sort-by name|mtime|ctime`.

**Task:**
1.  Update `FileEntry` struct in `rust/src/lib.rs` to store metadata (mtime, ctime, size).
2.  Implement `sort_files` function that takes a `SortMode` enum.
3.  Apply sorting before serialization.

**Verification:**
Run `cargo test --test test_vectors` to verify serialization order matches Python.
"
```

#### Step 3: Truncation Core
```bash
claude "feature: Implement Basic Truncation in Rust

**Context:**
We need to support the `--truncate N` flag.

**Task:**
1.  Update `serialize_file` in `rust/src/lib.rs`.
2.  If `config.truncate > 0`, count lines in content.
3.  If lines > limit, keep first N lines and append `[TRUNCATED]` marker (matching Python format).

**Verification:**
Create/Run a test vector for simple truncation.
"
```

#### Step 4: Smart Truncation
```bash
claude "feature: Implement Smart & Structure Truncation

**Context:**
We have the Analyzers, but they aren't connected to the Truncator.

**Task:**
1.  Update `truncate_content` in `rust/src/lib.rs`.
2.  If mode is `smart` or `structure`, call the appropriate `LanguageAnalyzer`.
3.  Use the analyzer's `get_structure_ranges` (already implemented for Rust/Py/JS) to filter lines.
4.  Inject the `.pm_encoder_meta` header if a Lens is active.
"
```

#### Step 5: Missing Analyzers
```bash
claude "feature: Port Missing Analyzers to Rust

**Context:**
We are missing analyzers for Markdown, JSON, and YAML.

**Task:**
1.  Update `rust/src/analyzers/generic.rs`.
2.  Add regex configurations for:
    -   **Markdown**: Headers (`^#+ `)
    -   **JSON**: Top-level keys (regex approximation or `serde_json` depth check)
    -   **YAML**: Top-level keys (`^\w+:`)
3.  Register them in `LanguageAnalyzerRegistry`.
"
```

#### Step 6: The Lens System
```bash
claude "feature: Implement Rust Lens System

**Context:**
The final piece of parity. Rust needs to load Lenses from config.

**Task:**
1.  Create `rust/src/lenses.rs`.
2.  Implement `LensManager` that loads from `.pm_encoder_config.json`.
3.  Implement `apply_lens` to merge Lens config into `EncoderConfig` (handling precedence).
4.  Print the Lens Manifest to stderr.
"
```

**Start with Step 1.** That fixes the "Red" tests and unblocks the rest. 🚀
