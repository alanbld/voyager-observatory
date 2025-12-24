# Voyager Observatory (VO)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.0.0-orange.svg)](rust/)

**Stop sending raw files to your AI. Start sending a contextual galaxy.**

```bash
# Point the telescope at your codebase
vo .

# Output: A structured constellation of your code, ready for AI comprehension
```

---

## The Fractal Telescope

Every codebase is a galaxy of interconnected systems. Most tools dump raw files into AI context windows like throwing stars into a bag. The Voyager Observatory is different—it's a telescope that reveals the structure, relationships, and meaning in your code.

### The Viewfinder

Point the telescope at any directory. The viewfinder automatically detects project boundaries, respects your gitignore, and calculates token costs.

```bash
# Point at current directory
vo .

# Point at a specific project
vo /path/to/project
```

### Spectral Filters (Lenses)

Different tasks require different views. Lenses filter your codebase to highlight what matters:

```bash
# Architecture lens - system design, entry points, configs
vo . --lens architecture

# Security lens - auth, crypto, input validation
vo . --lens security

# Debug lens - tests, error handlers, logs
vo . --lens debug

# Minimal lens - just the essentials
vo . --lens minimal
```

### Magnification (Zoom)

When you need to focus on a specific star, zoom in:

```bash
# Zoom into a function
vo . --zoom "function=calculate_total"

# Zoom into a class
vo . --zoom "class=UserService"

# Zoom into specific lines
vo . --zoom "file=src/lib.rs:100-200"
```

### The Observer's Journal

The telescope learns. Mark files as important, and it remembers. Repeatedly ignore a pattern, and it fades into the background.

```bash
# Mark a bright star
vo --mark src/core/engine.rs --utility 0.95

# View your observation history
vo --journal
```

---

## Quick Start

### Installation

```bash
# From source
cd rust && cargo build --release

# Install globally
cargo install --path rust

# Verify
vo --version
```

### Basic Usage

```bash
# Generate context (default: plus/minus format)
vo .

# With token budget
vo . --token-budget 100k

# Stream large codebases
vo . --stream

# Save to file
vo . > context.txt
```

### Intent-Driven Exploration

```bash
# Explore for business logic
vo . --explore business-logic

# Explore for debugging
vo . --explore debugging

# Explore for onboarding
vo . --explore onboarding
```

---

## MCP Server Mode

Integrate with AI CLI tools like Claude Code:

```bash
# Start as MCP server
vo --server /path/to/project
```

Configure in `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "pm_encoder": {
      "command": "/path/to/vo",
      "args": ["--server", "/path/to/project"]
    }
  }
}
```

**Available Tools:**
- `get_context` - Serialize with lens/budget options
- `zoom` - Symbol-aware magnification
- `explore_with_intent` - Guided codebase exploration
- `report_utility` - Train the telescope

---

## Output Formats

```bash
vo . --format plusminus    # Compact (default)
vo . --format xml          # Structured XML
vo . --format markdown     # Markdown
vo . --format claude-xml   # Optimized for Claude
```

---

## Token Budgeting

When your galaxy exceeds the viewport:

```bash
# Drop least important files
vo . --token-budget 50k --strategy drop

# Truncate large files
vo . --token-budget 50k --strategy truncate

# Hybrid: truncate first, then drop
vo . --token-budget 50k --strategy hybrid
```

---

## Why Voyager?

| Feature | Voyager Observatory | repomix | files-to-prompt |
|---------|---------------------|---------|-----------------|
| **Token budgeting** | Drop/truncate/hybrid | No | No |
| **Semantic analysis** | Fractal clustering | No | No |
| **Intent exploration** | 5 built-in intents | No | No |
| **Learning journal** | Persists preferences | No | No |
| **Performance** | Rust (10x faster) | Node.js | Python |
| **MCP server** | Built-in | No | No |

---

## Documentation

- **[Voyager Guide](docs/VOYAGER_GUIDE.md)** - Complete user manual
- **[Plugin Guide](PLUGIN_GUIDE.md)** - Extend language support

---

## Legacy Python Version

The original Python implementation is preserved for users who need it:

```bash
cd classic/python
./pm_encoder.py . --token-budget 100k
```

See [classic/python/README.md](classic/python/README.md) for details.

---

## Project Structure

```
pm_encoder/
├── rust/                    # Voyager Observatory (vo) - Rust engine
│   ├── src/
│   │   ├── bin/vo.rs        # Main binary
│   │   ├── core/            # Core modules
│   │   └── lib.rs           # Library exports
│   └── Cargo.toml
├── classic/
│   └── python/              # Legacy Python implementation
│       ├── pm_encoder.py
│       └── tests/
├── docs/
│   ├── VOYAGER_GUIDE.md     # User manual
│   └── archive/             # Historical specs
└── test_vectors/            # Cross-implementation tests
```

---

## The Twins Architecture

Voyager Observatory is built on "The Twins Architecture"—two engines, one vision:

- **Python (v1.7.0 LTS)**: Reference implementation, feature prototyping
- **Rust (v1.0.0)**: High-performance engine, production deployments

Both produce identical output for the same input, verified by differential testing.

---

## License

MIT License - See [LICENSE](LICENSE)

---

*"The engine is tested. The optics are clean. Now, let the world see the stars."*
