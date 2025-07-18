# Project Encoder (`pm_encoder.py`)

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

1.  Place the `pm_encoder.py` script in a `scripts/` directory within your project.
2.  Make the script executable:
    ```bash
    chmod +x scripts/pm_encoder.py
    ```
3.  (Optional) Create a `.pm_encoder_config.json` file in your project's root directory to define default filters. See the **Configuration** section below for an example.

## Usage

The script is run from the command line.

```
usage: pm_encoder.py [-h] [-o OUTPUT] [-c CONFIG] [--include [INCLUDE ...]] [--exclude [EXCLUDE ...]] project_root

Serialize project files into the Plus/Minus format.

positional arguments:
  project_root          The root directory of the project to serialize.

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
                        Output file path. Defaults to standard output.
  -c CONFIG, --config CONFIG
                        Path to a JSON configuration file for ignore/include patterns.
                        Defaults to ./.pm_encoder_config.json
  --include [INCLUDE ...]
                        One or more glob patterns for files to include. Overrides config includes.
  --exclude [EXCLUDE ...]
                        One or more glob patterns for files/dirs to exclude. Adds to config excludes.
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
