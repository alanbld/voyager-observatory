# Voyager Observatory User Guide

> *"Every codebase is a galaxy. The Voyager helps you navigate it."*

## Introduction

The Voyager Observatory (`vo`) is a context serialization tool designed for AI-assisted development. It transforms your codebase into an optimized context window that AI assistants can understand and navigate efficiently.

This guide covers the core concepts and practical usage of the `vo` command.

---

## Core Concepts

### The Viewfinder

The **Viewfinder** is how you point the telescope at your code. When you run `vo .`, you're aiming the viewfinder at your current directory.

```bash
# Point at current directory
vo .

# Point at a specific project
vo /path/to/project

# Point at a subdirectory
vo src/core/
```

The viewfinder automatically:
- Detects project boundaries (git roots, package.json, Cargo.toml)
- Respects .gitignore patterns
- Identifies binary files and excludes them
- Estimates token counts for each file

### Spectral Filters (Lenses)

**Lenses** are spectral filters that highlight different aspects of your codebase. Each lens adjusts file priorities and filtering to surface what matters for your task.

| Lens | Purpose | Highlights |
|------|---------|------------|
| `architecture` | System design | Entry points, configs, core modules |
| `security` | Security review | Auth, crypto, input validation |
| `debug` | Bug hunting | Tests, error handlers, logs |
| `minimal` | Quick overview | READMEs, main files only |
| `onboarding` | New developer | Getting started guides, examples |

```bash
# Apply the architecture lens
vo . --lens architecture

# Security-focused view
vo . --lens security

# Minimal context for quick questions
vo . --lens minimal
```

### Magnification (Zoom)

**Zoom** lets you magnify specific symbols, functions, or files. Instead of sending the entire codebase, you can focus on exactly what you need.

```bash
# Zoom into a specific function
vo . --zoom "function=calculate_total"

# Zoom into a class
vo . --zoom "class=UserService"

# Zoom into a file with line range
vo . --zoom "file=src/lib.rs:100-200"
```

Zoom is particularly powerful for:
- Deep-diving into specific implementations
- Debugging a particular function
- Understanding a single module

### The Observer's Journal

The **Journal** is how the Voyager learns your preferences. It tracks:

- **Bright Stars**: Files you've marked as high-utility
- **Faded Nebulae**: Patterns you consistently ignore
- **Exploration History**: Your navigation patterns

```bash
# Mark a file as important (bright star)
vo --mark src/core/engine.rs --utility 0.95

# View your journal
vo --journal

# Clear the journal
vo --journal-clear
```

The Journal persists in `.pm_encoder/observers_journal.json` and influences future context generation.

---

## Practical Usage

### Basic Commands

```bash
# Generate context for current directory
vo .

# With token budget (100k tokens)
vo . --token-budget 100k

# Stream output (for large codebases)
vo . --stream

# Save to file
vo . > context.txt
```

### Intent-Driven Exploration

The `--explore` flag activates intelligent exploration mode:

```bash
# Explore with business logic intent
vo . --explore business-logic

# Explore for debugging
vo . --explore debugging

# Explore for security audit
vo . --explore security
```

Exploration mode uses semantic analysis to:
- Identify related code clusters
- Suggest navigation paths
- Highlight cross-language connections

### Output Formats

```bash
# Default: Plus/Minus format (compact)
vo .

# XML format (structured)
vo . --format xml

# Markdown format
vo . --format markdown

# Claude-optimized XML
vo . --format claude-xml
```

### Token Budgeting Strategies

When your codebase exceeds the token budget, `vo` uses intelligent strategies:

```bash
# Drop least important files (default)
vo . --token-budget 50k --strategy drop

# Truncate large files
vo . --token-budget 50k --strategy truncate

# Hybrid: truncate first, then drop
vo . --token-budget 50k --strategy hybrid
```

---

## MCP Server Mode

For integration with AI CLI tools (Claude Code, etc.), `vo` can run as an MCP server:

```bash
# Start as MCP server
vo --server /path/to/project
```

### Configuration

Add to your MCP settings (`~/.claude/mcp.json`):

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

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `get_context` | Serialize directory with lens/budget options |
| `zoom` | Symbol-aware zoom into functions/classes |
| `session_list` | List saved zoom sessions |
| `session_create` | Create new zoom session |
| `report_utility` | Report file utility for learning |
| `explore_with_intent` | Intent-driven codebase exploration |

---

## Configuration

### Project Configuration

Create `.pm_encoder_config.json` in your project root:

```json
{
  "default_lens": "architecture",
  "token_budget": "100k",
  "ignore_patterns": ["*.log", "node_modules/**"],
  "priority_files": ["src/lib.rs", "README.md"]
}
```

### Global Configuration

User-wide settings in `~/.pm_encoder/config.json`:

```json
{
  "default_format": "xml",
  "stream_by_default": false,
  "show_token_counts": true
}
```

---

## Tips and Best Practices

### 1. Start with a Lens
Always apply a lens for your specific task. `architecture` is good for general understanding, `debug` for bug hunting.

### 2. Use Token Budgets
Set explicit budgets to ensure your context fits the model's window:
- GPT-4: `--token-budget 128k`
- Claude: `--token-budget 200k`
- Quick questions: `--token-budget 10k`

### 3. Mark Important Files
Use the journal to mark files you frequently need:
```bash
vo --mark src/core/engine.rs --utility 0.95 --note "Core processing logic"
```

### 4. Combine with Zoom
For deep work, combine lenses with zoom:
```bash
vo . --lens debug --zoom "function=handle_error"
```

### 5. Use Explore for Discovery
When you're new to a codebase:
```bash
vo . --explore onboarding
```

---

## Troubleshooting

### "Token budget exceeded"
Your codebase is larger than the budget. Either:
- Increase the budget: `--token-budget 200k`
- Use a more aggressive strategy: `--strategy drop`
- Apply a lens to filter: `--lens minimal`

### "Binary file detected"
Binary files are automatically skipped. If you need to include specific binary paths, use `.pm_encoder_config.json`.

### "No files found"
Check that you're pointing at the right directory and that .gitignore isn't excluding everything.

---

## Quick Reference

```bash
# Basic usage
vo .                              # Current directory
vo /path/to/project               # Specific path

# Lenses
vo . --lens architecture          # System design view
vo . --lens security              # Security review
vo . --lens debug                 # Debugging view
vo . --lens minimal               # Minimal context

# Budgeting
vo . --token-budget 100k          # 100,000 tokens
vo . --strategy hybrid            # Truncate then drop

# Zoom
vo . --zoom "function=main"       # Zoom to function
vo . --zoom "class=Config"        # Zoom to class

# Exploration
vo . --explore business-logic     # Intent-driven explore

# Output
vo . --format xml                 # XML output
vo . --stream                     # Stream mode

# Journal
vo --journal                      # View journal
vo --mark FILE --utility 0.9      # Mark file
```

---

*For legacy Python usage, see [classic/python/README.md](../classic/python/README.md)*
