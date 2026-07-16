# pm_encoder (Rust Engine)

**Version:** 0.8.0
**Status:** Production Ready | Feature Parity with Python v1.7.0

This is the Rust implementation of pm_encoder, designed as a high-performance context serializer for LLM workflows.

## Features

### Core Features (100% Parity)
- **Plus/Minus Format Serialization** - Full format support
- **Context Lenses** - architecture, security, debug, onboarding lenses
- **Priority Groups** - File prioritization for token budgeting
- **Token Budgeting** - `--token-budget 100k` with drop/truncate/hybrid strategies
- **Truncation Modes** - simple, smart, structure
- **Truncation Control** - `--truncate-summary`, `--truncate-exclude`
- **Language Analyzers** - Python, Rust, JavaScript, Shell, Generic
- **Init-Prompt** - `--init-prompt` generates CLAUDE.md/GEMINI_INSTRUCTIONS.txt + CONTEXT.txt
- **Streaming Mode** - `--stream` for immediate output

### Not Supported in Rust
- **Python Plugins** - The Rust engine uses compiled analyzers only. For custom language plugins, use the Python reference implementation (`pm_encoder.py`).

## Architecture: Library-First Pattern

```
rust/
├── Cargo.toml              # Package configuration
├── src/
│   ├── lib.rs              # Core library (serialization, config)
│   ├── analyzers/          # Language analyzers
│   │   ├── mod.rs          # Analyzer registry
│   │   ├── generic.rs      # Generic analyzer
│   │   ├── python.rs       # Python analyzer
│   │   ├── rust.rs         # Rust analyzer
│   │   ├── javascript.rs   # JavaScript analyzer
│   │   └── shell.rs        # Shell analyzer
│   ├── budgeting.rs        # Token budgeting and priority resolution
│   ├── lenses.rs           # Context lenses with priority groups
│   ├── init.rs             # Init-prompt generation (Split Brain)
│   └── bin/
│       └── main.rs         # CLI wrapper
```

### The Library: `lib.rs` + modules

- **Purpose:** Pure Rust logic with no CLI dependencies
- **Consumers:** CLI binary, WASM bindings (future), PyO3 bindings (future)
- **Testable:** 175+ unit tests

**Key Exports:**
- `serialize_project(root)` - Basic serialization
- `serialize_project_with_config(root, config)` - Configurable serialization
- `LensManager` - Context lens management
- `apply_token_budget()` - Budget enforcement
- `init::init_prompt()` - Generate AI instruction files

### The Interface: `bin/main.rs`

Thin CLI wrapper that delegates to the library.

## Usage

### Basic Serialization
```bash
pm_encoder /path/to/project
```

### With Context Lens
```bash
pm_encoder /path/to/project --lens architecture
```

### Token Budgeting
```bash
pm_encoder /path/to/project --token-budget 100k --budget-strategy hybrid
```

### Init-Prompt (Split Brain Architecture)
```bash
pm_encoder /path/to/project --init-prompt --init-lens debug --target claude
```

This creates:
- `CLAUDE.md` - Instructions, commands, tree structure (NO code)
- `CONTEXT.txt` - Serialized codebase (separate file)

### Streaming Mode
```bash
pm_encoder /path/to/project --stream
```

## Building & Running

```bash
cd rust

# Build
cargo build --release

# Run
cargo run --release -- /path/to/project

# Run tests
cargo test

# Run with coverage
cargo tarpaulin
```

## Test Coverage

- **Library:** 81%+ coverage
- **Test Vectors:** 29 integration tests
- **Total Tests:** 175+

## Performance

- **TTFB:** ~5ms (vs ~46ms Python)
- **Throughput:** 10-100x faster for large codebases
- **Memory:** Zero-copy where possible

## Comparison with Python

| Feature | Python v1.7.0 | Rust v0.8.0 |
|---------|---------------|-------------|
| Core Serialization | ✅ | ✅ |
| Context Lenses | ✅ | ✅ |
| Priority Groups | ✅ | ✅ |
| Token Budgeting | ✅ | ✅ |
| Truncation Control | ✅ | ✅ |
| Init-Prompt | ✅ | ✅ |
| Language Analyzers | ✅ | ✅ |
| Custom Plugins | ✅ | ❌ |
| Streaming | ✅ | ✅ |

**Note:** For custom language plugins, use the Python implementation.

## License

MIT (same as parent project)
