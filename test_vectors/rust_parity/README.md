# Rust Parity Test Vectors

This directory contains test vectors that ensure Python and Rust engines produce identical output.

## Purpose

The Python engine (v1.5.0+) is the **reference implementation**. These test vectors capture its behavior so the Rust engine can replicate it exactly.

## The Contract

```
Python generates test vectors (expected behavior)
        ↓
Rust must reproduce expected output exactly
        ↓
Byte-identical output = parity achieved ✅
```

## Current Status

| Category | Vectors | Rust Passing | Parity |
|----------|---------|--------------|--------|
| Config | 5 | 4 | 80% |
| Serialization | 0 | 0 | - |
| Analyzer | 0 | 0 | - |
| Truncation | 0 | 0 | - |
| Lens | 0 | 0 | - |

**Note:** Config parity is 80% (4/5). Test `config_02_cli_override` requires CLI argument parsing (planned for v0.4.0).

## Usage

### Running Rust Tests

```bash
cd rust
cargo test test_vectors
```

### Creating New Vectors

```bash
# Generate from Python test
python scripts/generate_test_vector.py test_name > test_vectors/rust_parity/category_##_name.json
```

### Validating Vectors

```bash
# Ensure Python output matches vector
python scripts/validate_test_vector.py test_vectors/rust_parity/vector.json
```

## Roadmap

- [x] Infrastructure created
- [x] Config vectors (5) - **80% passing (v0.3.0)**
- [ ] Serialization vectors (5)
- [ ] Analyzer vectors (10)
- [ ] Truncation vectors (5)
- [ ] Lens vectors (5)

**Target: 30 vectors by end of December 2025**
