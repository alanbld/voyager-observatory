# Rust Growth Strategy
## Fast-Track to Feature Parity with Python v1.3.1

**Created:** December 13, 2025
**Target:** Rust v1.0.0 by Q2 2026
**Strategy:** Test-Driven, Incremental, Validated

---

## Executive Summary

**Current State:**
- Python v1.3.1: Production-ready, 95% coverage, 7 analyzers, lens system
- Rust v0.1.0: Foundation established, library-first architecture

**The Goal:** Achieve byte-identical feature parity in 6 months (26 weeks)

**The Approach:** 8 milestone releases, each validated against Python test vectors

**Success Metric:** 100% test vector pass rate + 10x performance improvement

---

## Feature Gap Analysis

### Python v1.3.1 Feature Matrix

| Category | Features | Complexity | Priority |
|----------|----------|------------|----------|
| **Core Serialization** | Plus/Minus format, MD5 checksums, file traversal | Medium | P0 |
| **Configuration** | JSON config, CLI args, include/exclude patterns | Medium | P0 |
| **File Handling** | Binary detection, large file skip, encoding fallback | Low | P0 |
| **Language Analyzers** | 7 analyzers (Py, JS, Rust, Shell, MD, JSON, YAML) | High | P1 |
| **Truncation** | 3 modes (simple, smart, structure) | High | P1 |
| **Context Lenses** | 4 built-in lenses (architecture, debug, security, onboarding) | Medium | P2 |
| **Plugin System** | Template generation, custom analyzers | High | P2 |
| **Sorting** | By name/mtime/ctime, asc/desc | Low | P1 |
| **Statistics** | Token counts, truncation stats | Low | P2 |

### Complexity Assessment

**Low Complexity (1-2 weeks each):**
- File handling (binary detection, size limits)
- Sorting options
- Statistics generation

**Medium Complexity (2-3 weeks each):**
- Core serialization (Plus/Minus format)
- Configuration system (JSON + CLI)
- Context Lenses

**High Complexity (3-4 weeks each):**
- Language analyzers (7 separate implementations)
- Truncation modes (especially "structure")
- Plugin system

---

## The 8-Milestone Roadmap

### Phase 1: Foundation (Weeks 1-8)

#### v0.2.0 - Core Serialization (Weeks 1-2)
**Target Date:** December 20, 2025

**Deliverables:**
```rust
// lib.rs additions
pub struct FileEntry {
    pub path: String,
    pub content: String,
    pub md5: String,
}

pub fn walk_directory(root: &str, include: &[&str], exclude: &[&str]) -> Vec<FileEntry>;
pub fn serialize_file(entry: &FileEntry) -> String;  // Plus/Minus format
pub fn calculate_md5(content: &str) -> String;
```

**Test Vectors:**
- `basic_serialization.json` ✅ (already exists)
- `binary_detection.json` (create)
- `large_file_skip.json` (create)

**Success Criteria:**
- [ ] Directory traversal works
- [ ] Plus/Minus format matches Python exactly
- [ ] MD5 checksums match Python
- [ ] Pass 3 test vectors

**Dependencies:** Zero (stdlib only)

---

#### v0.3.0 - Configuration System (Weeks 3-4)
**Target Date:** January 3, 2026

**Deliverables:**
```rust
// lib.rs additions
pub struct EncoderConfig {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub sort_by: SortMode,
    pub sort_order: SortOrder,
    pub truncate: usize,
    pub truncate_mode: TruncateMode,
}

pub fn load_config(path: &str) -> Result<EncoderConfig, String>;
pub fn merge_cli_args(config: EncoderConfig, args: CliArgs) -> EncoderConfig;
```

**Test Vectors:**
- `config_file_loading.json` (create)
- `cli_override.json` (create)
- `pattern_matching.json` (create)

**Success Criteria:**
- [ ] JSON config file parsing
- [ ] CLI argument parsing (use `clap` or hand-roll for zero deps)
- [ ] Include/exclude glob pattern matching
- [ ] Pass 3 test vectors

**Dependencies:** Consider `glob` crate for pattern matching (or implement manually)

---

#### v0.4.0 - Sorting & File Handling (Weeks 5-6)
**Target Date:** January 17, 2026

**Deliverables:**
```rust
// lib.rs additions
pub enum SortMode { Name, Mtime, Ctime }
pub enum SortOrder { Asc, Desc }

pub fn sort_files(files: &mut Vec<FileEntry>, mode: SortMode, order: SortOrder);
pub fn is_binary(content: &[u8]) -> bool;
pub fn is_too_large(size: u64, limit: u64) -> bool;
```

**Test Vectors:**
- `sorting_by_mtime.json` (create)
- `sorting_by_name.json` (create)
- `binary_file_skip.json` (create)

**Success Criteria:**
- [ ] Sort by name/mtime/ctime
- [ ] Ascending/descending order
- [ ] Binary file detection (null byte check)
- [ ] Large file skipping (>5MB default)
- [ ] Pass 3 test vectors

**Dependencies:** Zero

---

#### v0.5.0 - Test Parity Validation (Weeks 7-8)
**Target Date:** January 31, 2026

**Focus:** Cross-validation, not new features

**Deliverables:**
- Automated test vector runner
- Byte-diff validation
- Performance baseline benchmarks

**Test Vectors:**
- Run ALL Python test vectors created so far (~10 vectors)
- Generate Python reference outputs
- Compare Rust outputs byte-by-byte

**Success Criteria:**
- [ ] 100% test vector pass rate
- [ ] Zero byte differences in output
- [ ] Performance measured (baseline for future comparison)
- [ ] CI/CD integration (`make test-cross` automated)

**Dependencies:** Zero (uses existing test infrastructure)

---

### Phase 2: Intelligence (Weeks 9-18)

#### v0.6.0 - Python Analyzer (Weeks 9-11)
**Target Date:** February 21, 2026

**Deliverables:**
```rust
// lib.rs additions
pub trait LanguageAnalyzer {
    fn analyze(&self, content: &str) -> AnalysisResult;
    fn get_truncate_ranges(&self, content: &str, max_lines: usize) -> Vec<Range>;
}

pub struct PythonAnalyzer;
impl LanguageAnalyzer for PythonAnalyzer {
    // Detect classes, functions, imports
    // Structure mode: extract signatures only
}
```

**Test Vectors:**
- `python_analyzer.json` ✅ (already exists)
- `python_class_detection.json` (create)
- `python_function_extraction.json` (create)
- `python_structure_mode.json` (create)

**Success Criteria:**
- [ ] Detect Python classes
- [ ] Detect Python functions (def, async def)
- [ ] Extract imports
- [ ] Structure mode: signature-only output
- [ ] Pass 4 test vectors

**Dependencies:** Consider `tree-sitter-python` or regex-based (choose based on complexity/size)

---

#### v0.7.0 - JavaScript/TypeScript Analyzer (Weeks 12-14)
**Target Date:** March 14, 2026

**Deliverables:**
```rust
pub struct JavaScriptAnalyzer;
impl LanguageAnalyzer for JavaScriptAnalyzer {
    // Detect classes, functions, exports, imports
    // Handle both JS and TS
}
```

**Test Vectors:**
- `javascript_analyzer.json` (create)
- `typescript_analyzer.json` (create)
- `jsx_tsx_analyzer.json` (create)

**Success Criteria:**
- [ ] Detect classes, functions, arrow functions
- [ ] Detect exports/imports (ES6 + CommonJS)
- [ ] Handle JSX/TSX syntax
- [ ] Pass 3 test vectors

**Dependencies:** Possibly `tree-sitter-javascript` or regex

---

#### v0.8.0 - Remaining Analyzers (Weeks 15-18)
**Target Date:** April 11, 2026

**Deliverables:**
```rust
pub struct RustAnalyzer;    // Can analyze itself! 🦀
pub struct ShellAnalyzer;   // Bash/sh function detection
pub struct MarkdownAnalyzer; // Section extraction
pub struct JSONAnalyzer;    // Structure validation
pub struct YAMLAnalyzer;    // Structure validation
```

**Test Vectors:** 2-3 vectors per analyzer (10-15 total)

**Success Criteria:**
- [ ] All 7 analyzers implemented
- [ ] Rust analyzer can process pm_encoder's own Rust code
- [ ] Pass 15 test vectors
- [ ] Analyzer registry system working

**Dependencies:** Possibly `tree-sitter` for multiple languages

---

### Phase 3: Advanced Features (Weeks 19-24)

#### v0.9.0 - Truncation Modes (Weeks 19-21)
**Target Date:** May 2, 2026

**Deliverables:**
```rust
pub enum TruncateMode {
    Simple,     // Cut at line N
    Smart,      // Preserve structure boundaries
    Structure,  // Signatures only
}

pub fn truncate_file(
    content: &str,
    max_lines: usize,
    mode: TruncateMode,
    analyzer: &dyn LanguageAnalyzer,
) -> String;
```

**Test Vectors:**
- `truncate_simple.json` (create)
- `truncate_smart.json` (create)
- `truncate_structure.json` (create)
- Cross-analyzer truncation tests (5-7 vectors)

**Success Criteria:**
- [ ] Simple mode works (line-based cut)
- [ ] Smart mode preserves function/class boundaries
- [ ] Structure mode extracts signatures only
- [ ] All analyzers support all modes
- [ ] Pass 10 test vectors

---

#### v0.10.0 - Context Lenses (Weeks 22-24)
**Target Date:** May 23, 2026

**Deliverables:**
```rust
pub struct Lens {
    pub name: String,
    pub description: String,
    pub config: EncoderConfig,
}

pub struct LensRegistry {
    built_in: HashMap<String, Lens>,
    custom: HashMap<String, Lens>,
}

impl LensRegistry {
    pub fn apply_lens(&self, name: &str, base_config: EncoderConfig) -> EncoderConfig;
}
```

**Built-in Lenses:**
- `architecture` - High-level structure
- `debug` - Recent changes
- `security` - Auth, secrets, dependencies
- `onboarding` - Essential files for new contributors

**Test Vectors:**
- `lens_architecture.json` (create)
- `lens_debug.json` (create)
- `lens_security.json` (create)
- `lens_onboarding.json` (create)

**Success Criteria:**
- [ ] 4 built-in lenses match Python behavior
- [ ] Custom lens loading from config
- [ ] Lens precedence: CLI > Lens > Config > Default
- [ ] Pass 4 test vectors

---

### Phase 4: Production Ready (Weeks 25-26)

#### v1.0.0 - Production Release (Weeks 25-26)
**Target Date:** June 6, 2026

**Focus:** Polish, performance, distribution

**Deliverables:**
1. **Performance Validation**
   - Benchmark against Python
   - Target: 10x faster on large repos (10k+ files)
   - Memory profiling and optimization

2. **Binary Distribution**
   ```bash
   # Users can install via cargo
   cargo install pm_encoder

   # Or download pre-built binaries
   # Linux (x86_64, aarch64)
   # macOS (x86_64, aarch64)
   # Windows (x86_64)
   ```

3. **Documentation**
   - CLI help text complete
   - `rust/README.md` updated
   - Performance benchmarks published

4. **Final Validation**
   - Run Python's entire test suite via test vectors
   - 100% pass rate required
   - Zero byte differences in output

**Success Criteria:**
- [ ] All test vectors passing (50+ vectors)
- [ ] 10x performance improvement validated
- [ ] Binary releases for 6 platforms
- [ ] `cargo install pm_encoder` works
- [ ] Full documentation complete

---

## Development Principles

### 1. Test-Driven Development

**Every feature follows this cycle:**

```
1. Python generates test vector (expected behavior)
2. Rust implements feature
3. Rust runs test vector → fails
4. Fix implementation → iterate
5. Test passes → move to next feature
```

**No feature is "done" until its test vector passes.**

### 2. Zero Regressions

**Rule:** Never break a passing test vector.

- Run full test suite before each commit
- CI/CD blocks merges if test vectors fail
- Use `make test-cross` constantly

### 3. Incremental Complexity

**Dependency graph:**

```
v0.2.0 (Core) → Foundation for everything
    ↓
v0.3.0 (Config) → Required by v0.10.0 (Lenses)
    ↓
v0.4.0 (Sorting) → Independent, can parallelize
    ↓
v0.5.0 (Validation) → CHECKPOINT: Everything works
    ↓
v0.6.0-0.8.0 (Analyzers) → Can parallelize different languages
    ↓
v0.9.0 (Truncation) → Requires analyzers
    ↓
v0.10.0 (Lenses) → Requires config + truncation
    ↓
v1.0.0 (Production) → Integration + polish
```

**Strategy:** Knock out v0.2.0-0.4.0 sequentially (foundation), then parallelize analyzer development.

### 4. Performance From Day 1

**Track performance at each milestone:**

```bash
# Benchmark against Python
hyperfine \
  './pm_encoder.py large_repo/' \
  'cargo run --release -- large_repo/'
```

**Target:** Each version should be faster than previous.

**Final target:** 10x faster than Python on large repos (>10k files)

### 5. Dependency Minimalism

**Allowed dependencies:**
- **Pattern matching:** `glob` crate (small, well-tested)
- **JSON parsing:** `serde_json` (essential for config)
- **CLI parsing:** `clap` (optional, can hand-roll)
- **Language parsing:** `tree-sitter` + language grammars (optional, worth it for correctness)

**Principle:** Only add dependencies that provide clear value > cost.

**Alternative:** Implement manually if dependency is large or complex.

---

## Risk Mitigation

### Risk 1: Tree-sitter Dependency Size

**Problem:** `tree-sitter` + language grammars could bloat binary size

**Mitigation:**
- **Plan A:** Use `tree-sitter` with feature flags (each language is optional)
- **Plan B:** Implement regex-based analyzers first, upgrade to tree-sitter later
- **Plan C:** Make analyzers pluggable, ship minimal binary + language packs

**Decision point:** v0.6.0 (first analyzer)

### Risk 2: Byte-Identical Output

**Problem:** Subtle differences in MD5, whitespace, newlines

**Mitigation:**
- Test vectors include exact output hashes
- Byte-diff tool in CI/CD
- Python's output is canonical (Rust must match exactly)

**Validation:** v0.5.0 checkpoint

### Risk 3: Performance Doesn't Hit 10x

**Problem:** Rust might not be 10x faster (I/O bound, not CPU bound)

**Mitigation:**
- Profile early (v0.5.0)
- Optimize hot paths (MD5, file reading, pattern matching)
- Consider parallel file processing
- Set realistic targets based on profiling

**Validation:** v1.0.0 final benchmarks

### Risk 4: Feature Creep

**Problem:** Python adds features while Rust is catching up

**Mitigation:**
- Feature freeze Python at v1.3.1 for Rust parity period
- New Python features go into v1.4.0+ (after Rust v1.0.0)
- Both engines evolve together after parity

**Governance:** Document in THE_TWINS_ARCHITECTURE.md

---

## Resource Optimization

### Parallel Development Opportunities

**Can work in parallel:**
- Analyzers (v0.6.0-0.8.0): Each language is independent
- Test vector generation: Can create vectors ahead of implementation
- Documentation: Can write while coding

**Must be sequential:**
- Foundation (v0.2.0-0.4.0): Each builds on previous
- Truncation (v0.9.0): Requires analyzers
- Lenses (v0.10.0): Requires truncation + config

### Test Vector Generation Strategy

**Create vectors ahead of implementation:**

```bash
# Week 1: Create all test vectors for v0.2.0-0.5.0
# Weeks 2-8: Implement features to pass those vectors
# Week 9: Create analyzer test vectors
# Weeks 10-18: Implement analyzers
```

**Benefit:** Clear targets, no waiting for spec

### Multi-AI Collaboration

**Divide work by AI specialty:**
- **Claude Code Server:** Implementation (write Rust code)
- **AI Studio/Gemini:** Analysis (performance profiling, architectural validation)
- **Claude.ai Opus:** Orchestration (test vector generation, documentation)

**Coordination:** Weekly sync via session summaries

---

## Success Metrics Dashboard

### Technical Metrics

| Metric | Target | Tracking |
|--------|--------|----------|
| Test Vector Pass Rate | 100% | `make test-cross` |
| Performance vs Python | 10x faster | `hyperfine` benchmarks |
| Binary Size | <10MB | `cargo build --release && ls -lh` |
| Test Coverage | >80% | `cargo tarpaulin` |
| Feature Parity | 100% | Feature matrix checklist |

### Milestone Metrics

| Milestone | Target Date | Test Vectors | Features |
|-----------|-------------|--------------|----------|
| v0.2.0 | Dec 20, 2025 | 3 passing | Core serialization |
| v0.3.0 | Jan 3, 2026 | 6 passing | + Configuration |
| v0.4.0 | Jan 17, 2026 | 9 passing | + Sorting |
| v0.5.0 | Jan 31, 2026 | 10 passing | Validation checkpoint |
| v0.6.0 | Feb 21, 2026 | 14 passing | + Python analyzer |
| v0.7.0 | Mar 14, 2026 | 17 passing | + JS/TS analyzer |
| v0.8.0 | Apr 11, 2026 | 32 passing | + All 7 analyzers |
| v0.9.0 | May 2, 2026 | 42 passing | + Truncation modes |
| v0.10.0 | May 23, 2026 | 46 passing | + Context lenses |
| v1.0.0 | Jun 6, 2026 | 50+ passing | Production ready |

---

## Next Steps (Immediate Actions)

### Week 1 (Dec 13-20, 2025)

**Day 1-2: Test Vector Creation**
```bash
# Create test vectors for v0.2.0
cd test_vectors/
# - directory_traversal.json
# - plus_minus_format.json
# - md5_checksum.json
# - binary_detection.json
# - large_file_skip.json
```

**Day 3-5: Core Implementation**
```rust
// rust/src/lib.rs
// Implement:
// - walk_directory()
// - serialize_file()
// - calculate_md5()
// - is_binary()
// - is_too_large()
```

**Day 6-7: Test & Validate**
```bash
make test-rust           # Unit tests pass
make test-cross          # Test vectors pass
cargo build --release    # Clean build
```

**Deliverable:** Rust v0.2.0 tagged and released

---

## Conclusion

**The Vision:** Rust v1.0.0 in 6 months with 100% feature parity

**The Path:** 8 carefully planned milestones, each validated by test vectors

**The Philosophy:** Test-driven, incremental, no regressions

**The Outcome:** Two production-ready engines, one vision, infinite possibilities

🐍 + 🦀 = 🚀

---

**Last Updated:** December 13, 2025
**Status:** Strategy Active
**Next Milestone:** v0.2.0 (Core Serialization) - December 20, 2025
