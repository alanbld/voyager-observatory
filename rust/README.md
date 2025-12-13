# pm_encoder (Rust Engine)

**Version:** 0.1.0
**Status:** Foundation (v2.0 Architecture)

This is the Rust implementation of pm_encoder, designed as a high-performance context serializer for LLM workflows.

## Architecture: Library-First Pattern

This crate is intentionally structured to separate **logic** from **interface**:

```
rust/
├── Cargo.toml          # Package configuration (library + binary)
├── src/
│   ├── lib.rs          # 🧠 The Brain (core logic)
│   └── bin/
│       └── main.rs     # 🖥️ The Interface (CLI wrapper)
```

### The Brain: `lib.rs`

- **Purpose:** Pure Rust logic with no CLI dependencies
- **Consumers:** CLI binary, WASM bindings, PyO3 Python bindings
- **Testable:** Unit tests run against the library directly
- **Reusable:** Can be embedded in any Rust project

**Key Functions:**
- `version()` - Returns library version
- `serialize_project(root: &str)` - Core serialization logic
- `serialize_project_with_config(root: &str, config: &EncoderConfig)` - Configurable serialization

### The Interface: `bin/main.rs`

- **Purpose:** Thin CLI wrapper around the library
- **Responsibilities:** Argument parsing, error formatting, exit codes
- **Philosophy:** Minimal logic, maximum delegation to `lib.rs`

This separation ensures:
1. **Testability** - Library logic can be unit tested without spawning processes
2. **Reusability** - Same logic works for CLI, WASM, and Python
3. **Modularity** - CLI can be swapped/extended without touching core logic

## Future Bindings

### WASM (JavaScript/Browser)

The library can be compiled to WebAssembly:

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn serialize_wasm(root: &str) -> String {
    pm_encoder::serialize_project(root).unwrap_or_else(|e| e)
}
```

### Python (PyO3)

The library can be wrapped for Python:

```rust
use pyo3::prelude::*;

#[pyfunction]
fn serialize(root: &str) -> PyResult<String> {
    pm_encoder::serialize_project(root)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
}
```

## Building & Running

### Build the Library

```bash
cd rust
cargo build --lib
```

### Build the CLI Binary

```bash
cd rust
cargo build --bin pm_encoder
```

### Run the CLI

```bash
cd rust
cargo run -- /path/to/project
```

### Run Tests

```bash
cd rust
cargo test
```

## Design Principles

1. **Zero Dependencies** (for now) - Keep the skeleton minimal
2. **Library-First** - Core logic lives in `lib.rs`, not `main.rs`
3. **Interface Agnostic** - Same logic works for CLI, WASM, Python
4. **Testability** - Library functions are pure and testable
5. **Modularity** - Easy to add new interfaces without changing core logic

## Current Status

**Implemented:**
- ✅ Library skeleton (`lib.rs`)
- ✅ CLI wrapper (`bin/main.rs`)
- ✅ Basic configuration struct
- ✅ Version management
- ✅ Unit tests

**Next Steps:**
- Directory traversal
- File filtering (ignore patterns)
- Plus/Minus format generation
- Language analyzers (Rust ports from Python)
- Truncation modes (simple, smart, structure)

## Why Rust?

The Python implementation (pm_encoder.py) is excellent for:
- Rapid development
- Python ecosystem integration
- Prototyping features

The Rust implementation will provide:
- **10-100x performance** for large codebases
- **WASM compatibility** for browser-based tools
- **Python bindings** (best of both worlds)
- **Memory safety** without garbage collection

## License

MIT (same as parent project)
