# Project Encoder (`pm_encoder.py`)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.6+](https://img.shields.io/badge/python-3.6+-blue.svg)](https://www.python.org/downloads/)

`pm_encoder.py` is a powerful command-line utility designed to serialize a project's source files into a single, well-structured text file. This is ideal for sharing project context with Large Language Models (LLMs), creating context packages for new developers, or for archival purposes.

The script uses a custom "Plus/Minus" format that is both human-readable and easy for other scripts to parse. It offers robust filtering capabilities through a JSON configuration file and command-line arguments.

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
- **🆕 Intelligent Truncation** (v1.1+): Language-aware file truncation to reduce token usage while preserving critical code structures.
- **🆕 Multi-Language Support** (v1.1+): Built-in analyzers for Python, JavaScript/TypeScript, Shell, Markdown, JSON, and YAML.
- **🆕 Plugin System** (v1.1+): Extensible architecture for community-contributed language analyzers.
- **🆕 Token Optimization** (v1.1+): Detailed statistics on size and token reduction.

## Project Structure

This repository contains **two implementations** of pm_encoder:

### Python Implementation (Current Production)

- **Location:** `pm_encoder.py` (root directory)
- **Version:** 1.3.1
- **Status:** Production-ready with 95% test coverage
- **Best for:** Python ecosystem integration, rapid feature development
- **Dependencies:** None (Python 3.6+ stdlib only)

### Rust Implementation (v2.0 Foundation)

- **Location:** `rust/` directory
- **Version:** 0.1.0 (skeleton)
- **Status:** Architecture foundation
- **Best for:** High-performance, WASM/Python bindings, large codebases
- **Architecture:** Library-first pattern (`lib.rs` + `bin/main.rs`)

The Rust implementation is designed with a **Library-First** architecture:
- `rust/src/lib.rs` - Pure logic, reusable by CLI/WASM/PyO3
- `rust/src/bin/main.rs` - Thin CLI wrapper

See `rust/README.md` for details on the Rust architecture and future WASM/Python binding plans.

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
