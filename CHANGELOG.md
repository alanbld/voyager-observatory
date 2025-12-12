# Changelog

All notable changes to pm_encoder will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.1] - 2025-12-12

### Fixed - Critical Logic Bugs
- **Structure mode triggering**: Fixed bug where structure mode wouldn't trigger unless `--truncate N` was specified
  - Changed condition from `if truncate_lines > 0:` to `if truncate_lines > 0 or truncate_mode == 'structure':`
  - Now `--truncate-mode structure` works correctly without requiring numeric line limit
  - **Impact**: Users can now use `--lens architecture` or `--truncate-mode structure` without setting `--truncate`
- **Lens precedence mapping**: Fixed bug in `apply_lens()` where lens "include"/"exclude" keys weren't properly mapped to "include_patterns"/"ignore_patterns"
  - Lens "include" now correctly overrides base "include_patterns"
  - Lens "exclude" now correctly extends base "ignore_patterns"
  - **Impact**: Custom and built-in lenses now work as documented

### Added - Comprehensive Test Suite
- **tests/test_pm_encoder.py**: 9 comprehensive tests using standard library only (unittest, tempfile, shutil)
  - `test_structure_mode_trigger`: Verifies structure mode works with truncate=0 (the bug fix)
  - `test_lens_precedence`: Verifies layered precedence (CLI > Lens > Config > Defaults)
  - `test_python_structure`: Verifies Python signature extraction (keeps signatures, removes bodies)
  - `test_js_structure`: Verifies JavaScript structure extraction
  - `test_json_fallback`: Verifies JSON files fall back to smart mode (no structure support)
  - `test_meta_injection`: Verifies .pm_encoder_meta file is injected when using lenses
  - `test_ignore_patterns`: Verifies .git and __pycache__ are properly ignored
  - `test_all_lenses_exist`: Verifies all 4 built-in lenses exist
  - `test_architecture_lens_has_safety_limit`: Verifies architecture lens has truncate: 2000 safety limit
- **Test Results**: ✅ 9/9 tests passing, 0 failures, 0 errors

### Changed
- **architecture lens**: Added `truncate: 2000` safety limit for non-code files
- **Version bumped** to 1.2.1

### Why This Release Matters
v1.2.1 fixes critical bugs that prevented structure mode from working as designed in v1.2.0. The comprehensive test suite ensures these bugs won't regress and validates all core functionality. This is a **recommended upgrade** for all v1.2.0 users.

## [1.2.0] - 2025-12-12

### Added - Context Lenses System
- **Context Lenses**: Pre-configured serialization profiles for specific use cases
- **Built-in lenses** (4 total):
  - `architecture`: High-level structure with structure mode (signatures only)
  - `debug`: Recent changes with full files, sorted by mtime DESC
  - `security`: Security-critical code with smart truncation (300 lines)
  - `onboarding`: Balanced overview for new developers (400 lines)
- **LensManager class**: Handles lens application with layered precedence
- **Lens manifest printing**: Transparent stderr output showing active lens configuration
- **Meta-aware output**: Injects `.pm_encoder_meta` file to document lens and filtering for LLMs
- **Custom lens support**: Define project-specific lenses in `.pm_encoder_config.json`

### Added - Structure Mode Truncation
- **Structure mode**: New truncation mode showing only signatures (imports, class/function declarations)
- **Language-specific structure extraction** for:
  - Python: Imports, class definitions, function signatures, decorators
  - JavaScript/TypeScript: Imports/exports, classes, functions, arrow functions, interfaces
  - Shell: Shebang, function declarations, source statements
- **Graceful fallback**: Unsupported languages automatically fall back to smart mode
- **Structure mode markers**: Clear indicators showing what was included/excluded

### Added - CLI Options
- `--lens NAME`: Apply a context lens (architecture|debug|security|onboarding|custom)
- `--truncate-mode structure`: New truncation mode option (in addition to simple|smart)

### Changed
- **load_config()**: Now returns custom lenses from config file (3rd return value)
- **serialize()**: Accepts optional `lens_manager` parameter for lens integration
- **Layered precedence**: CLI flags > Lens settings > Config file > Defaults
- **Version bumped** to 1.2.0

### Performance
- **Optimized analyzers**: All analyzers now use `analyze_lines()` to eliminate redundant string splitting
- **~50% reduction** in string allocation overhead from v1.1.0

### Documentation
- **README.md**: Added comprehensive Context Lenses section with examples
- **TUTORIAL.md**: Added 4 new lens examples (Examples 8-11) and Workflow 6
- **CHANGELOG.md**: This entry

### Use Cases Unlocked
- **Architecture exploration**: Get codebase overview without implementation details (80%+ reduction)
- **Rapid debugging**: Immediately focus on recently modified files
- **Security audits**: Automated filtering for security-relevant code
- **Team onboarding**: Pre-configured balanced context for new developers
- **Custom workflows**: Define project-specific lenses for common tasks

### Technical Details
- Zero new external dependencies (still 100% standard library)
- Python 3.6+ compatibility maintained
- Backward compatible: existing v1.1.0 workflows unchanged
- Lens system purely additive (no breaking changes)

## [1.1.0] - 2025-12-12

### Added - Intelligent Truncation System
- **Language-aware truncation**: Smart truncation that understands code structure across multiple languages
- **Built-in language analyzers** for:
  - Python (`.py`, `.pyw`): Classes, functions, imports, `__main__` blocks, docstrings, markers
  - JavaScript/TypeScript (`.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`, `.cjs`): Classes, functions, imports, exports, JSDoc
  - Shell (`.sh`, `.bash`, `.zsh`, `.fish`): Functions, sourced files, shebang detection
  - Markdown (`.md`, `.markdown`): Headers, code blocks, links, structure-aware truncation
  - JSON (`.json`): Structural analysis with key/depth detection
  - YAML (`.yaml`, `.yml`): Key structure preservation
- **Truncation modes**:
  - `simple`: Fast truncation keeping first N lines
  - `smart`: Language-aware truncation preserving critical code sections
- **Detailed truncation summaries** showing:
  - Language and file category (application/library/test/config)
  - Detected classes, functions, imports
  - Entry points and markers (TODO/FIXME)
  - Instructions for retrieving full content
- **Truncation statistics** with `--truncate-stats` flag showing:
  - Files analyzed vs truncated
  - Line and size reduction percentages
  - Per-language breakdown
  - Estimated token count reduction

### Added - Plugin System
- **Extensible language analyzer architecture** allowing community contributions
- **Plugin template generator**: `--create-plugin LANGUAGE` command
- **AI prompt generator**: `--plugin-prompt LANGUAGE` for getting AI assistance
- **Plugin loading system** from `~/.pm_encoder/plugins/` or custom directory
- **Example Rust analyzer** in `examples/plugins/rust_analyzer.py`
- **Comprehensive plugin development guide** (`PLUGIN_GUIDE.md`)

### Added - CLI Options
- `--truncate N`: Truncate files exceeding N lines (default: 0 = no truncation)
- `--truncate-mode {simple|smart}`: Choose truncation strategy (default: simple)
- `--truncate-summary`: Include analysis summary in truncation markers (default: true)
- `--no-truncate-summary`: Disable truncation summaries
- `--truncate-exclude PATTERN [PATTERN ...]`: Exclude files from truncation by glob pattern
- `--truncate-stats`: Show detailed truncation statistics report
- `--language-plugins DIR`: Specify custom language analyzer plugins directory
- `--create-plugin LANGUAGE`: Generate plugin template for a language
- `--plugin-prompt LANGUAGE`: Generate AI prompt for creating a plugin

### Changed
- **Enhanced Plus/Minus format**: Truncated files show `[TRUNCATED: N lines]` in headers and `[TRUNCATED:N→M]` in footers
- **Version bumped** to 1.1.0
- **Performance optimized**: Language analysis adds <100ms overhead per file

### Documentation
- **README.md**: Added language support matrix and truncation examples
- **TUTORIAL.md**: New "Token Optimization" section with practical truncation workflows
- **PLUGIN_GUIDE.md**: Complete guide for creating custom language analyzers
- **Examples**: Added `examples/plugins/` with Rust analyzer sample

### Technical Details
- Zero new external dependencies (still 100% standard library)
- Python 3.6+ compatibility maintained
- Backward compatible: existing workflows unchanged without truncation flags
- Regex-based analyzers (no AST parsing) for speed and portability

### Use Cases Unlocked
- **LLM context optimization**: Reduce large codebases to fit token limits
- **Cost reduction**: Lower API costs for token-based LLM services
- **Faster processing**: Smaller context = faster LLM responses
- **Better code understanding**: Summaries help AI grasp project structure
- **Multi-language projects**: Single tool handles polyglot repositories

## [1.0.0] - 2025-12-12

### Added
- Initial public release of pm_encoder
- Plus/Minus format serialization with MD5 checksums
- JSON configuration file support (`.pm_encoder_config.json`)
- CLI flags for filtering: `--include`, `--exclude`
- Sorting options: `--sort-by` (name, mtime, ctime) and `--sort-order` (asc, desc)
- Binary file detection (null-byte heuristic)
- Large file skipping (>5MB)
- UTF-8 encoding with latin-1 fallback
- POSIX-style paths in output for cross-platform compatibility
- Directory pruning for efficient traversal
- Standard output by default with `-o` option for file output

### Technical Details
- Python 3.6+ compatibility
- Zero external dependencies (standard library only)
- Single-file distribution (`pm_encoder.py`)

[1.0.0]: https://github.com/alanbld/pm_encoder/releases/tag/v1.0.0
