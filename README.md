# Voyager Observatory

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust v1.0.0](https://img.shields.io/badge/rust-v1.0.0-orange.svg)](rust/)
[![Tests](https://img.shields.io/badge/tests-1237%20passing-brightgreen.svg)]()

**The Fractal Telescope for Code**

```bash
vo .
```

*Point the telescope at your codebase. See the stars.*

---

## The Story

> *In the vast darkness of context windows, codebases drift like uncharted galaxies. Raw files tumble through token limits—truncated, compressed, lost. Meaning dissolves into noise.*
>
> *But there is another way.*
>
> *The Voyager Observatory was built by astronomers who understood that the universe is not chaos—it is structure. Every function is a star. Every module is a constellation. Every codebase is a galaxy waiting to be mapped.*
>
> *Point the telescope. Adjust the lens. And suddenly, the chaos resolves into clarity.*

---

## The Viewfinder

Every observation begins with pointing the telescope:

```bash
# Point at current directory
vo .

# Point at a specific galaxy
vo /path/to/project

# Output: A structured map of your code, ready for AI comprehension
```

The Viewfinder automatically detects project boundaries, respects your `.gitignore`, and calculates token costs.

---

## Spectral Filters (Lenses)

Different missions require different views. Each lens reveals a different layer of the codebase:

```bash
# Architecture Lens - system design, entry points, configurations
vo . --lens architecture

# Security Lens - authentication, cryptography, input validation
vo . --lens security

# Debug Lens - tests, error handlers, logging
vo . --lens debug

# Minimal Lens - just the essentials
vo . --lens minimal

# Onboarding Lens - best for newcomers
vo . --lens onboarding
```

---

## Magnification (Zoom)

When you need to focus on a specific star, zoom in:

```bash
# Zoom into a function
vo . --zoom "function=calculate_total"

# Zoom into a class
vo . --zoom "class=UserService"

# Zoom into specific lines
vo . --zoom "file=src/lib.rs:100-200"
```

**The Fractal Principle**: Zoom in, and new detail emerges. Zoom out, and patterns appear. Context flows at every level.

---

## The Observer's Journal

The telescope learns from every observation. Mark files as important, and it remembers. Repeatedly ignore a pattern, and it fades into the background.

```bash
# Mark a bright star
vo --mark src/core/engine.rs --utility 0.95

# View your observation history
vo --journal
```

---

## Intent-Driven Exploration

Let the telescope guide your exploration:

```bash
# Explore for business logic
vo . --explore business-logic

# Explore for debugging
vo . --explore debugging

# Explore for onboarding new developers
vo . --explore onboarding

# Explore for security audit
vo . --explore security

# Explore for migration planning
vo . --explore migration
```

---

## Celestial Census

Survey the health of your galaxy:

```bash
vo . --survey

# Output:
# Galaxy Census
# ├── Stars (Functions): 342
# ├── Nebulae (Modules): 28
# ├── Dark Matter (Tests): 156
# ├── Stellar Age: 18 months
# ├── Volcanic Churn: Medium
# └── Health Rating: ★★★★☆
```

---

## External Optics (Community Plugins)

Extend the telescope with community lenses. Plugins are Lua scripts that run in a secure sandbox.

### Installing Plugins

Create a `manifest.json` in `.vo/plugins/` or `~/.config/vo/plugins/`:

```json
{
  "vo_api_version": "3.0",
  "plugins": [
    {
      "name": "complexity-analyzer",
      "file": "complexity.lua",
      "enabled": true,
      "priority": 100
    }
  ]
}
```

### Writing Your First Plugin

```lua
-- complexity.lua
-- A simple cyclomatic complexity estimator

vo.log("info", "Complexity analyzer loaded")

-- Contribute tags to nodes
vo.contribute_tag("src/complex.rs:42", "high-complexity")

-- Register custom metrics
vo.register_metric("avg_complexity", function(ast)
    -- Analyze the AST and return a metric
    return {
        value = 4.2,
        confidence = 0.85,
        explanation = "Average cyclomatic complexity"
    }
end)
```

### The vo.* API

| Function | Description |
|----------|-------------|
| `vo.api_version` | Current API version ("3.0") |
| `vo.patterns.*` | Pre-compiled regex patterns for 8+ languages |
| `vo.regex(pattern)` | Returns a safe matcher function |
| `vo.log(level, message)` | Log messages (trace/debug/info/warn/error) |
| `vo.contribute_tag(node, tag)` | Attach tags to code nodes |
| `vo.register_metric(name, fn)` | Register custom metrics |
| `vo.ast(path)` | Read-only access to parsed AST |

### Security Guarantees

Plugins run in the **Iron Sandbox**:
- 100ms CPU timeout (instruction limit)
- 10MB memory ceiling
- No filesystem access (`io` stripped)
- No shell execution (`os` stripped)
- No dynamic code loading (`load`, `require` stripped)
- No debugging escape (`debug` stripped)

Plugins can only **append** to the context—they cannot modify or delete core data.

---

## Token Budgeting

When your galaxy exceeds the viewport:

```bash
# Drop least important files
vo . --token-budget 50k --strategy drop

# Truncate large files (preserve structure)
vo . --token-budget 50k --strategy truncate

# Hybrid: truncate first, then drop
vo . --token-budget 50k --strategy hybrid
```

---

## Output Formats

```bash
vo . --format plusminus    # Compact diff-like format (default)
vo . --format xml          # Structured XML
vo . --format markdown     # Markdown documentation
vo . --format claude-xml   # Optimized for Claude
```

---

## MCP Server Mode

Integrate with AI CLI tools like Claude Code, Cursor, or Gemini CLI.

### Adding VO as MCP Server

Use the `claude mcp add` command to register VO:

```bash
# Project scope (recommended) - stored in .mcp.json, shareable with team
cd /path/to/your/project
claude mcp add vo /path/to/vo --scope project -- --server .

# User scope - available across all projects on your machine
claude mcp add vo /path/to/vo --scope user -- --server /default/project

# Remove from a scope
claude mcp remove vo
```

### Configuration Scopes

| Scope | Location | Use Case |
|-------|----------|----------|
| **Project** | `.mcp.json` (project root) | Team sharing, version control |
| **User** | `~/.claude.json` | Personal cross-project tools |
| **Local** | `.claude/settings.local.json` | Private project-specific |

### Project Scope (Recommended)

Creates `.mcp.json` in your project root (check into git):

```json
{
  "mcpServers": {
    "vo": {
      "command": "/path/to/vo",
      "args": ["--server", "."]
    }
  }
}
```

Using `"."` as server path makes VO serve the **current working directory**, so the same config works for any project.

### User Scope

Stored in `~/.claude.json` - available everywhere but requires absolute path:

```json
{
  "mcpServers": {
    "vo": {
      "command": "/path/to/vo",
      "args": ["--server", "/home/user/default/project"]
    }
  }
}
```

### Available MCP Tools

| Tool | Purpose |
|------|---------|
| `get_context` | Serialize with lens/budget options |
| `zoom` | Symbol-aware magnification |
| `explore_with_intent` | Guided codebase exploration |
| `report_utility` | Train the telescope |
| `session_list` | List saved zoom sessions |
| `session_create` | Create new zoom session |

### Troubleshooting MCP

```bash
# Check running MCP servers
ps aux | grep "vo --server"

# Kill stale servers (after config changes)
pkill -f "vo --server"

# Test MCP server manually
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | vo --server .

# Reset project approval choices
claude mcp reset-project-choices
```

**Important**: After changing MCP config, restart Claude Code to spawn fresh servers.

---

## Installation

### From Source

```bash
cd rust && cargo build --release --features plugins

# The binary is at: target/release/vo
```

### Install Globally

```bash
cargo install --path rust --features plugins

# Verify
vo --version
```

### With Plugins Disabled

```bash
cargo build --release  # No --features plugins
```

---

## Quick Start

```bash
# Point the telescope
vo .

# Apply a lens
vo . --lens architecture

# Zoom into a function
vo . --zoom "function=main"

# Set a token budget
vo . --token-budget 100k

# Stream large codebases
vo . --stream

# Save to file
vo . > context.txt
```

---

## Why Voyager?

| Feature | Voyager Observatory | repomix | files-to-prompt |
|---------|---------------------|---------|-----------------|
| **AST parsing** | Tree-sitter (25+ languages) | No | No |
| **Token budgeting** | Drop/truncate/hybrid | No | No |
| **Semantic analysis** | Fractal clustering | No | No |
| **Intent exploration** | 5 built-in intents | No | No |
| **Learning journal** | Persists preferences | No | No |
| **Community plugins** | Secure Lua sandbox | No | No |
| **Celestial Census** | Code health metrics | No | No |
| **Performance** | Rust (10x faster) | Node.js | Python |
| **MCP server** | Built-in | No | No |

---

## Documentation

- **[Voyager Guide](docs/VOYAGER_GUIDE.md)** - Complete user manual
- **[Plugin Architecture](rust/docs/arch/PLUGIN_ARCHITECTURE.md)** - Plugin system design
- **[Plugin Guide](PLUGIN_GUIDE.md)** - Language plugin development

---

## The Twins Architecture

Voyager Observatory evolved from a dual-engine architecture:

- **Python (v1.7.0 LTS)**: Reference implementation in `classic/python/`
- **Rust (v1.0.0)**: High-performance engine, the production core

Both produce identical output for the same input, verified by differential testing with 1,237+ tests.

---

## Project Structure

```
voyager-observatory/
├── rust/                    # Voyager Observatory (vo) - Rust engine
│   ├── src/
│   │   ├── bin/vo.rs        # Main binary
│   │   ├── core/            # Core modules
│   │   │   ├── plugins/     # Iron Sandbox & Plugin System
│   │   │   ├── fractal/     # Fractal Context Engine
│   │   │   ├── celestial/   # Celestial Navigation
│   │   │   └── temporal/    # Chronos Engine (git history)
│   │   └── lib.rs           # Library exports
│   └── Cargo.toml
├── classic/
│   └── python/              # Legacy Python implementation (LTS)
├── docs/
│   └── VOYAGER_GUIDE.md     # User manual
└── test_vectors/            # Cross-implementation tests
```

---

## License

MIT License - See [LICENSE](LICENSE)

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

<p align="center">
<i>"The engine is tested. The optics are clean. The sandbox is secure.<br>
Now, let the world see the stars."</i>
</p>
