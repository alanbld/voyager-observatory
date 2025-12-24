# pm_encoder (Classic Python Edition)

> **LTS/Legacy Version** - The original Python implementation of pm_encoder.

## Overview

This is the classic Python implementation of pm_encoder, preserved for users who prefer the original tool or need Python-specific functionality. For the latest features and best performance, see the [Voyager Observatory (vo)](../../README.md) Rust implementation.

## Quick Start

```bash
# From this directory
./pm_encoder.py . --token-budget 100k

# With a lens
./pm_encoder.py /path/to/project --lens architecture

# Streaming mode
./pm_encoder.py . --stream
```

## Features

- **Token Budgeting**: Intelligent file selection with drop/truncate/hybrid strategies
- **Context Lenses**: Architecture, Security, Debug, Minimal views
- **Priority Groups**: Ordered file selection based on importance
- **Streaming Mode**: Low-latency output for large codebases
- **Plugin System**: Extensible language analysis

## Requirements

- Python 3.8+
- No external dependencies (standard library only)

## Running Tests

```bash
cd classic/python
python -m pytest tests/ -v
```

## Version

- **Version**: 1.7.0 (LTS)
- **Status**: Maintenance mode - critical fixes only

## Migration to Voyager Observatory

The Rust-based Voyager Observatory (`vo`) offers:

- 10x faster performance
- MCP server mode for AI CLI integration
- Fractal semantic analysis
- Observer's Journal for learning preferences

See the [main README](../../README.md) for migration guidance.

---

*Preserved as part of The Twins Architecture - "Two engines, one vision."*
