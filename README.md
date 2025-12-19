# pm_encoder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.6+](https://img.shields.io/badge/python-3.6+-blue.svg)](https://www.python.org/downloads/)
[![Rust](https://img.shields.io/badge/rust-v0.9.1-orange.svg)](rust/)

**Convert your entire codebase into a single text file, intelligently budgeted for LLM context windows.**

```bash
# Fit a 500-file project into Claude's context
./pm_encoder.py ./my-project --token-budget 100k --budget-strategy hybrid

# Output: One file, priority-sorted, intelligently truncated, ready to paste
```

---

## Why pm_encoder?

There are other tools in this space ([repomix](https://github.com/yamadashy/repomix), [files-to-prompt](https://github.com/simonw/files-to-prompt)). Here's when to use pm_encoder:

| Feature | pm_encoder | repomix | files-to-prompt |
|---------|-----------|---------|-----------------|
| **Token budgeting** | ✅ `--token-budget 100k` | ❌ | ❌ |
| **Budget strategies** | ✅ drop/truncate/hybrid | ❌ | ❌ |
| **Priority groups** | ✅ Explicit numeric | ⚠️ Git frequency | ❌ |
| **Deterministic output** | ✅ Reproducible | ⚠️ Varies | ✅ |
| **Zero dependencies** | ✅ Python stdlib | ❌ Node.js | ❌ pip install |
| **Dual engine** | ✅ Python + Rust | ❌ | ❌ |
| **File checksums** | ✅ MD5 per file | ❌ | ❌ |

### pm_encoder is Pipeline-First

```
repomix       = UX-first      → Interactive chat, human review
files-to-prompt = Simple-first  → Quick concatenation
pm_encoder    = Pipeline-first → CI/CD, agents, automation, reproducibility
```

**Best for:**
- CI pipelines with strict token limits
- Deterministic, reproducible prompts for regression testing
- Agent workflows with hard budget constraints
- Automated refactors and code mods
- Eval harnesses comparing LLM outputs

**Not ideal for:**
- Quick one-off pastes (use files-to-prompt)
- Interactive exploration with Web UI (use repomix.com)

---

## Quick Start

```bash
# Basic: serialize entire project
./pm_encoder.py . > context.txt

# With token budget (fits in Claude's context)
./pm_encoder.py . --token-budget 100k -o context.txt

# Smart curation: hybrid strategy + architecture lens
./pm_encoder.py . --token-budget 100k --budget-strategy hybrid --lens architecture

# Generate CLAUDE.md + CONTEXT.txt for AI IDE
./pm_encoder.py . --init-prompt --target claude

# Use Rust engine (10x faster)
./rust/target/release/pm_encoder . --token-budget 100k
```

---

## Features

- **Plus/Minus Format**: A clean, readable format that wraps each file's content with clear start and end markers.
- **Data Integrity**: The closing marker for each file includes an MD5 checksum to verify content integrity.
- **Binary File Detection**: Automatically detects and skips binary files (e.g., images, executables, archives) to keep the output clean and text-focused.
- **Configuration File**: Uses a `.pm_encoder_config.json` file to define default include and exclude patterns.
- **CLI Overrides**: Command-line flags (`--include`, `--exclude`) allow for flexible, on-the-fly filtering that can override or extend the configuration file settings.
- **Robust Filtering**: Uses glob patterns for powerful matching of files and directories.
- **Directory Pruning**: Efficiently skips entire directories (like `target/` or `node_modules/`) that match ignore patterns.
- **Large File Skipping**: Avoids including files over a certain size (default: 5MB) to keep the output manageable.
- **Standard I/O**: Writes to standard output by default, allowing it to be piped to other commands (e.g., clipboards).
- **🆕 AI IDE Integration** (v1.4+): One-command generation of CLAUDE.md / GEMINI_INSTRUCTIONS.txt with directory tree and statistics.
- **🆕 Multi-AI Support** (v1.4+): Generate optimized context for Claude Code, Google AI Studio, and other AI development tools.
- **🆕 Smart Command Detection** (v1.4+): Auto-detect project commands (npm, cargo, make, pip) from project files.
- **🆕 Intelligent Truncation** (v1.1+): Language-aware file truncation to reduce token usage while preserving critical code structures.
- **🆕 Multi-Language Support** (v1.1+): Built-in analyzers for Python, JavaScript/TypeScript, Shell, Markdown, JSON, and YAML.
- **🆕 Plugin System** (v1.1+): Extensible architecture for community-contributed language analyzers.
- **🆕 Token Optimization** (v1.1+): Detailed statistics on size and token reduction.

## The Twins Architecture

This repository contains **two implementations** with 100% byte-level parity:

| Engine | Version | Status | Performance | Best For |
|--------|---------|--------|-------------|----------|
| **Python** | v1.7.0 | Production | ~46ms TTFB | Feature prototyping, plugins |
| **Rust** | v0.9.1 | Production | ~5ms TTFB | Large codebases, CI pipelines |

Both engines produce **identical output** for the same input. Verified by 213 tests and differential fuzzing via `pm_coach`.

```bash
# Python (reference implementation)
./pm_encoder.py . --token-budget 100k

# Rust (10x faster, same output)
./rust/target/release/pm_encoder . --token-budget 100k
```

### Library-First Architecture (WASM-Ready)

The Rust engine uses a pure-function core for future WASM compilation:
- `rust/src/lib.rs` - Pure logic, no I/O (WASM compatible)
- `rust/src/bin/main.rs` - Thin CLI wrapper with filesystem access

See [docs/STRATEGIC_VISION_2026.md](docs/STRATEGIC_VISION_2026.md) for the roadmap to browser/IDE integration.

## Documentation

For AI-assisted development and comprehensive project context:
- **[Knowledge Base](docs/KNOWLEDGE_BASE.md)** - Single source of truth for AI sessions (architecture, decisions, roadmap)
- **[The Twins Architecture](docs/THE_TWINS_ARCHITECTURE.md)** - Dual-engine design philosophy and roadmap
- **[Rust Growth Strategy](docs/RUST_GROWTH_STRATEGY.md)** - Fast-track plan to feature parity (v0.1.0 → v1.0.0)
- **[Blueprint](docs/BLUEPRINT.md)** - Strategic vision and feature planning
- **[The Turing Audit](docs/THE_TURING_AUDIT.md)** - Multi-AI development story
- **[Testing Guide](TESTING.md)** - Test infrastructure and coverage details

## The Plus/Minus Format

The script outputs files in the following format:

```
++++++++++ path/to/your/file.rs ++++++++++
// The full and COMPLETE content of the file goes here.
// Every line must be included.
---------- path/to/your/file.rs <md5_checksum> path/to/your/file.rs ----------
```

## Prerequisites

- Python 3.6+

## Installation

### Option 1: Quick Download (Single File)

Download the script directly:

```bash
# Download to current directory
curl -O https://raw.githubusercontent.com/alanbld/pm_encoder/main/pm_encoder.py
chmod +x pm_encoder.py

# Or download to a scripts/ directory
mkdir -p scripts
curl -o scripts/pm_encoder.py https://raw.githubusercontent.com/alanbld/pm_encoder/main/pm_encoder.py
chmod +x scripts/pm_encoder.py
```

### Option 2: Git Clone (Full Repository)

Clone the repository for full access to examples and documentation:

```bash
git clone https://github.com/alanbld/pm_encoder.git
cd pm_encoder
chmod +x pm_encoder.py
```

### Option 3: Copy to Your Project

1. Place the `pm_encoder.py` script in a `scripts/` directory within your project.
2. Make the script executable:
   ```bash
   chmod +x scripts/pm_encoder.py
   ```

### Configuration (Optional)

Create a `.pm_encoder_config.json` file in your project's root directory to define default filters. See the **Configuration** section below for an example, or copy from `examples/.pm_encoder_config.json`.

## Usage

The script is run from the command line.

```
usage: pm_encoder.py [-h] [--version] [-o OUTPUT] [-c CONFIG] [--include [INCLUDE ...]]
                     [--exclude [EXCLUDE ...]] [--sort-by {name,mtime,ctime}]
                     [--sort-order {asc,desc}] project_root

Serialize project files into the Plus/Minus format.

positional arguments:
  project_root          The root directory of the project to serialize.

options:
  -h, --help            show this help message and exit
  --version             show program's version number and exit
  -o OUTPUT, --output OUTPUT
                        Output file path. Defaults to standard output.
  -c CONFIG, --config CONFIG
                        Path to a JSON configuration file for ignore/include patterns.
                        Defaults to ./.pm_encoder_config.json
  --include [INCLUDE ...]
                        One or more glob patterns for files to include. Overrides config includes.
  --exclude [EXCLUDE ...]
                        One or more glob patterns for files/dirs to exclude. Adds to config excludes.
  --sort-by {name,mtime,ctime}
                        Sort files by 'name' (default), 'mtime' (modification time),
                        or 'ctime' (creation time).
  --sort-order {asc,desc}
                        Sort order: 'asc' (ascending, default) or 'desc' (descending).
```

---

## Examples

### 1. Basic Usage (Using Config File)

Serialize the current project (`.`) and save the output to `context.txt`. This will use the filters defined in `.pm_encoder_config.json`.

```bash
./scripts/pm_encoder.py . -o context.txt
```

### 2. Piping to Clipboard

Serialize the project and pipe the output directly to your system's clipboard.

```bash
# On macOS
./scripts/pm_encoder.py . | pbcopy

# On Linux (requires xclip)
./scripts/pm_encoder.py . | xclip -selection clipboard
```

### 3. Packaging Only Specific Files (`--include`)

To package only the Rust source files from the `api_test_tool_core` crate and the main `Cargo.toml`, you can override the include patterns from the command line.

```bash
./scripts/pm_encoder.py . \
  --include "api_test_tool_core/src/**/*.rs" "Cargo.toml" \
  -o core_crate_context.txt
```
*Note: The `**` glob pattern allows for recursive matching.*

### 4. Temporarily Excluding Files (`--exclude`)

To serialize the project according to the config file but also exclude all Markdown files (`*.md`) and the `docs/` directory for this run:

```bash
./scripts/pm_encoder.py . --exclude "*.md" "docs" -o no_docs_context.txt
```

### 5. Combining Filters

Create a package containing only Python scripts (`*.py`) and shell scripts (`*.sh`), while also ensuring that the `.venv` directory is ignored.

```bash
./scripts/pm_encoder.py . \
  --include "*.py" "*.sh" \
  --exclude ".venv" \
  -o scripts_only.txt
```

### 6. Token Optimization with Truncation (v1.1+)

When sharing large projects with LLMs, you may hit token limits. Use intelligent truncation to reduce file sizes while preserving the most important code:

```bash
# Smart truncation (500 lines per file, language-aware)
./pm_encoder.py . --truncate 500 --truncate-mode smart -o context.txt

# Show truncation statistics
./pm_encoder.py . --truncate 300 --truncate-stats

# Exclude certain files from truncation
./pm_encoder.py . --truncate 500 --truncate-exclude "README.md" "LICENSE"

# Simple truncation (just keep first N lines)
./pm_encoder.py . --truncate 200 --truncate-mode simple -o quick.txt
```

**Smart truncation** analyzes each file's language and preserves:
- Import statements and dependencies
- Class and function signatures
- Entry points (main functions, exports)
- Critical code sections
- Documentation headers

**Example truncation output:**
```
++++++++++ src/database/handler.py [TRUNCATED: 873 lines] ++++++++++
[First 250 lines showing imports, class definitions, key functions]

... [400 lines omitted] ...

[Last 50 lines showing main entry point]

======================================================================
TRUNCATED at line 500/873 (42% reduction)
Language: Python
Category: Application Module
Classes (5): DatabaseHandler, MigrationRunner, SchemaValidator
Functions (23): apply_migration, rollback, validate_schema, ...
Key imports: psycopg2, sqlalchemy, pandas

To get full content: --include "src/database/handler.py" --truncate 0
======================================================================
---------- src/database/handler.py [TRUNCATED:873→300] a7b3c9d2... ----------
```

### 7. Claude Code / AI IDE Integration (v1.4.0+)

Generate instant AI CLI integration files with directory tree and statistics:

```bash
# Generate CLAUDE.md + CONTEXT.txt (default: architecture lens)
./pm_encoder.py . --init-prompt

# Use debug lens (smaller, faster)
./pm_encoder.py . --init-prompt --init-lens debug

# Generate for Google AI Studio / Gemini
./pm_encoder.py . --init-prompt --target gemini

# Try different optimization lenses
./pm_encoder.py . --init-prompt --init-lens security    # Security focus
./pm_encoder.py . --init-prompt --init-lens onboarding  # New developer focus
```

**What gets generated:**

1. **CLAUDE.md** or **GEMINI_INSTRUCTIONS.txt** (~1 KB) - Clean instructions with:
   - Project directory tree (3 levels deep)
   - File count and context size statistics
   - Auto-detected commands (npm, cargo, make, pip)
   - Reference to CONTEXT.txt

2. **CONTEXT.txt** (varies) - Full serialized codebase

**Benefits:**
- ⚡ **Instant setup** - 2 seconds vs 30-60s manual /init
- 🆓 **Zero cost** - No API calls, works offline
- 📊 **Better context** - Lens-optimized, consistent quality
- 🔄 **Regeneratable** - One command to update

**Example generated CLAUDE.md:**
```markdown
# my_project

## Project Structure
```
my_project/
├── src/
│   ├── main.rs
│   └── lib.rs
├── tests/
│   └── integration_test.rs
├── Cargo.toml
└── README.md
```

**Statistics:**
- Files: 15
- Context size: 42,350 bytes (41.3 KB)

## Commands
- `cargo build`
- `cargo test`

For complete codebase, see `CONTEXT.txt`.
```

---

## Language Support (v1.1+)

pm_encoder includes built-in intelligent truncation for multiple programming languages:

| Language | Extensions | Detected Features |
|----------|-----------|-------------------|
| **Python** | `.py`, `.pyw` | Classes, functions, imports, `__main__` blocks, docstrings, TODO/FIXME markers |
| **JavaScript/TypeScript** | `.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs` | Classes, functions, arrow functions, imports, exports, JSDoc |
| **Shell** | `.sh`, `.bash`, `.zsh`, `.fish` | Functions, sourced files, shebang |
| **Markdown** | `.md`, `.markdown` | Headers, code blocks, links, table of contents |
| **JSON** | `.json` | Keys, nested structure depth, value types |
| **YAML** | `.yaml`, `.yml` | Keys, nested structure |

### Extending Language Support

Create custom language analyzers for additional languages:

```bash
# Generate a plugin template
./pm_encoder.py --create-plugin Rust > rust_analyzer.py

# Or get AI assistance
./pm_encoder.py --plugin-prompt Kotlin > kotlin_prompt.txt
```

See [PLUGIN_GUIDE.md](PLUGIN_GUIDE.md) for complete plugin development documentation.

### Community Plugins

Example plugins available in `examples/plugins/`:
- **Rust** (`rust_analyzer.py`): Structs, traits, functions, use statements

To use community plugins:
```bash
mkdir -p ~/.pm_encoder/plugins/
cp examples/plugins/rust_analyzer.py ~/.pm_encoder/plugins/
./pm_encoder.py . --truncate 500 --language-plugins ~/.pm_encoder/plugins/
```

---

## Context Lenses (v1.2.0)

Context Lenses provide pre-configured serialization profiles optimized for specific use cases. Each lens configures filters, sorting, and truncation strategies to produce focused, relevant context.

### Built-in Lenses

| Lens | Description | Truncation | Sort | Best For |
|------|-------------|------------|------|----------|
| **architecture** | High-level structure, interfaces, configuration | Structure mode | Name (ASC) | Understanding codebase organization, API surface |
| **debug** | Recent changes for debugging | None (full files) | Mtime (DESC) | Bug investigation, recent modifications |
| **security** | Security-sensitive code for review | Smart (300 lines) | Name (ASC) | Security audits, vulnerability scanning |
| **onboarding** | New developer introduction | Smart (400 lines) | Name (ASC) | Team onboarding, documentation |

### Using Context Lenses

```bash
# Architecture view - signatures only
./pm_encoder.py . --lens architecture -o architecture.txt

# Debug view - recent changes with full content
./pm_encoder.py . --lens debug -o recent_changes.txt

# Security review - focused on security-critical code
./pm_encoder.py . --lens security -o security_review.txt

# Onboarding - balanced overview for new developers
./pm_encoder.py . --lens onboarding -o onboarding.txt
```

### Lens Transparency

When using a lens, pm_encoder injects a `.pm_encoder_meta` file at the start of output:

```
++++++++++ .pm_encoder_meta ++++++++++
Context generated with lens: "architecture"
Focus: High-level structure, interfaces, configuration

Implementation details truncated using structure mode
Output shows only:
  - Import/export statements
  - Class and function signatures
  - Type definitions and interfaces
  - Module-level documentation

Generated: 2025-12-12T22:38:43.850133
pm_encoder version: 1.2.0
---------- .pm_encoder_meta ... ----------
```

This ensures LLMs understand how the context was filtered.

### Structure Mode Truncation

Structure mode (used by `architecture` lens) shows only signatures:

**Original file (100 lines):**
```python
import os
from pathlib import Path

class FileProcessor:
    def __init__(self, root_dir):
        self.root = root_dir
        self.cache = {}

    def process_file(self, file_path):
        # 50 lines of implementation
        ...
        return result
```

**Structure mode output (~10 lines):**
```python
import os
from pathlib import Path

class FileProcessor:
    def __init__(self, root_dir):
    def process_file(self, file_path):

======================================================================
STRUCTURE MODE: Showing only signatures
Included: imports, class/function signatures, type definitions
Excluded: function bodies, implementation details
======================================================================
```

### Custom Lenses

Define custom lenses in `.pm_encoder_config.json`:

```json
{
  "lenses": {
    "frontend": {
      "description": "Frontend components and styles",
      "include": ["src/components/**/*.tsx", "src/styles/**/*.css"],
      "exclude": ["*.test.tsx"],
      "truncate_mode": "smart",
      "truncate": 400,
      "sort_by": "name"
    },
    "api": {
      "description": "Backend API endpoints",
      "include": ["api/**/*.py", "models/**/*.py"],
      "exclude": ["tests/**"],
      "truncate_mode": "structure",
      "sort_by": "name"
    }
  }
}
```

Then use them:
```bash
./pm_encoder.py . --lens frontend -o frontend_context.txt
./pm_encoder.py . --lens api -o api_structure.txt
```

### Lens Precedence

Configuration is merged with layered precedence (highest to lowest):
1. **CLI flags** (e.g., `--include`, `--exclude`)
2. **Lens settings** (from `--lens`)
3. **Config file** (`.pm_encoder_config.json`)
4. **Defaults** (built-in patterns)

Example:
```bash
# Lens sets structure mode, but CLI overrides to smart
./pm_encoder.py . --lens architecture --truncate-mode smart
```

---

## 🧠 Context Economics (v1.7.0+)

Don't guess file limits. Set a **Token Budget**, and let pm_encoder curate the context for you.

### Token Budgeting

Limit the output size to fit your LLM's context window (e.g., 8k, 100k, 2M).

```bash
# Limit output to 100,000 tokens (drops low-priority files first)
./pm_encoder.py . --token-budget 100k
```

### Budget Strategies

Decide what happens when files don't fit:

*   **`drop`** (Default): Skips the file entirely.
*   **`truncate`**: Forces the file into Structure Mode (signatures only).
*   **`hybrid`** (Recommended): Smart curation.
    *   If a file takes up >10% of the budget, it auto-truncates to Structure Mode.
    *   If it fits, it keeps full content.
    *   If it still doesn't fit, it drops.

```bash
# Maximize information density
./pm_encoder.py . --token-budget 50k --budget-strategy hybrid
```

### Priority Groups

Define what matters most in `.pm_encoder_config.json`. Higher priority files are added first.

```json
"lenses": {
  "architecture": {
    "groups": [
      { "pattern": "src/core/**", "priority": 100 },
      { "pattern": "src/utils/**", "priority": 50 },
      { "pattern": "tests/**", "priority": 10 }
    ]
  }
}
```

---

## Configuration

You can control the default behavior of the script by placing a `.pm_encoder_config.json` file in your project's root directory.

- `ignore_patterns`: A list of glob patterns. Any file or directory matching these patterns will be completely ignored. This is useful for build artifacts, virtual environments, and version control folders.
- `include_patterns`: A list of glob patterns. If this list is not empty, **only** files matching these patterns will be included in the output.

### Example `.pm_encoder_config.json`

```json
{
  "ignore_patterns": [
    ".git",
    ".idea",
    ".vscode",
    "target",
    "build",
    "dist",
    ".venv",
    "__pycache__",
    "*.pyc",
    "*.log",
    "*.swp",
    "*.bak",
    "*.tmp",
    "node_modules",
    "recordings.db*",
    "compare_output"
  ],
  "include_patterns": [
    "*.rs",
    "*.toml",
    "*.md",
    "*.py",
    "*.sh",
    "*.json",
    "*.xml",
    "*.txt",
    "Dockerfile",
    "LICENSE"
  ]
}
```
