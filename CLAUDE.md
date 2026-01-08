# Voyager Observatory (vo)

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Voyager Observatory (vo)** is a context serialization tool for LLM workflows. It converts codebases into AI-digestible context with intelligent token budgeting and semantic analysis.

### Architecture
- **Rust Engine** - High-performance implementation with MCP server mode
- **voyager-ast** - Tree-sitter based AST extraction for 10+ languages
- **Python Legacy** - Reference implementation (deprecated)

### Key Features
- Token budgeting with drop/truncate/hybrid strategies
- Context lenses (architecture, security, debug, minimal, onboarding)
- Priority groups for intelligent file selection
- Streaming mode for low-latency output
- **MCP Server Mode** - JSON-RPC 2.0 protocol for AI CLI integration
- **Universal Spectrograph** - 60+ language support for syntax detection
- **Semantic Analysis** - Call graphs, clustering, intent-driven exploration

## Quick Start

```bash
# Build
cd rust && cargo build --release

# Basic usage
./target/release/vo . --token-budget 100k --lens architecture

# With streaming
./target/release/vo . --stream --token-budget 50k
```

## MCP Server Mode

The Rust engine supports MCP (Model Context Protocol) for integration with AI CLIs:

```bash
# Run as MCP server (JSON-RPC 2.0 over stdio)
./target/release/vo --server /path/to/project
```

**Available Tools:**
- `get_context` - Serialize directory with lens/budget options
- `zoom` - Symbol-aware zoom into functions/classes/files
- `session_list` - List saved zoom sessions
- `session_create` - Create new zoom session
- `report_utility` - Report file utility for learning
- `explore_with_intent` - Intent-driven codebase exploration

**Configuration:**
```json
// ~/.claude/mcp.json
{
  "mcpServers": {
    "vo": {
      "command": "/path/to/vo",
      "args": ["--server", "/path/to/project"]
    }
  }
}
```

## Commands

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run tests with coverage
cargo tarpaulin --workspace --out Stdout

# Run specific package tests
cargo test --package voyager-ast
```

## Project Structure

```
vo/
├── rust/
│   ├── src/
│   │   ├── bin/vo.rs          # CLI binary
│   │   ├── lib.rs             # Library root
│   │   ├── core/              # Core modules
│   │   │   ├── celestial/     # Navigation, compass, nebula naming
│   │   │   ├── fractal/       # Semantic analysis, clustering
│   │   │   ├── spectrograph/  # Language detection (60+ langs)
│   │   │   └── ...
│   │   └── ...
│   ├── voyager-ast/           # AST extraction subcrate
│   │   ├── src/
│   │   │   ├── adapters/      # Language adapters (Python, TS, Rust, etc.)
│   │   │   ├── ir.rs          # Intermediate representation
│   │   │   ├── registry.rs    # Adapter registry
│   │   │   └── provider.rs    # AST provider interface
│   │   └── Cargo.toml
│   ├── Cargo.toml
│   └── Cargo.lock
├── docs/
├── CLAUDE.md
└── README.md
```

## Test Coverage

Current test coverage for key modules:

| Module | Tests | Coverage |
|--------|-------|----------|
| voyager-ast | 445 | ~62% |
| - ir.rs | - | 100% |
| - error.rs | - | 100% |
| - typescript_adapter.rs | - | 95.5% |
| - python_adapter.rs | - | 87.4% |
| - rust_adapter.rs | - | 78.7% |

## Key Modules

### voyager-ast
Tree-sitter based AST extraction supporting:
- Python, TypeScript/JavaScript, Rust
- Go, Java, C, C++, C#, Ruby
- HTML, CSS, JSON, Bash

### Spectrograph
Universal language detection using spectral signatures for 60+ languages.

### Fractal Analysis
- **Clustering** - Semantic code clustering
- **Relationships** - Call graph analysis
- **Intent Explorer** - Goal-driven code navigation

### Celestial Navigation
- **Compass** - Navigation suggestions
- **Nebula Namer** - Semantic cluster naming

---

*Updated: January 2026*
