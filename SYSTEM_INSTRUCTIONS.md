# System Instructions for pm_encoder
**Version: 1.0-pm_encoder**
**Generated: 2025-12-12**
**Language: Python 3**
**Protocol: AI Collaboration Protocol v2.0**

---

## Core Philosophy

pm_encoder is a **meta-tool for AI collaboration** — it exists to facilitate effective context sharing between developers and AI assistants. As such, development must be:

1. **Self-Aware**: Recognize that changes to pm_encoder affect how developers collaborate with AI systems
2. **Format-Preserving**: The Plus/Minus format is the core contract; any changes must maintain backward compatibility
3. **Utility-Focused**: Every feature must solve a real context-sharing pain point
4. **Dogfooded**: Use pm_encoder itself to share context during its own development

### The Meta-Tool Paradox

pm_encoder serializes projects for AI consumption, including itself. This creates a recursive relationship where:
- We use AI to develop pm_encoder
- pm_encoder helps us share context with AI
- Changes to pm_encoder affect how we use AI for its development

**Implication**: Every modification must consider its impact on the AI collaboration workflow.

---

## Session Management Protocol

### Session Identification
Each AI response in pm_encoder development begins with:
```
Session: 2025-12-12 | pm_encoder-a7c3f | Turn: 1
Context: [serialized|partial|minimal]
```

**Components**:
- **Date**: ISO format (YYYY-MM-DD)
- **Hash**: First 5 characters of initial prompt SHA-256
- **Turn**: Sequential number within session (resets each session)
- **Context**: Level of project context provided
  - `serialized` - Full pm_encoder context via its own output
  - `partial` - Specific files/modules only
  - `minimal` - Working from memory/documentation only

### Task Classification

Prefix requests with appropriate tags to set expectations:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `format:` | Plus/Minus format changes/fixes | `format: Fix checksum calculation for UTF-8 edge cases` |
| `feature:` | New capability | `feature: Add --depth flag for shallow serialization` |
| `fix:` | Bug resolution | `fix: Binary detection fails on certain file types` |
| `improve:` | Refactoring/optimization | `improve: Reduce memory footprint for large repos` |
| `document:` | Documentation updates | `document: Add migration guide from v1 to v2` |
| `test:` | Test creation/modification | `test: Add integration tests for config handling` |
| `sync:` | Context update/alignment | `sync: Update context with latest main branch changes` |
| `meta:` | Meta-development discussion | `meta: Should we support streaming output?` |

### Context Generation for Sessions

When starting a development session, generate fresh context:

```bash
# Full context (preferred for major changes)
./pm_encoder.py . -o pm_encoder_full_context.txt

# Targeted context (for specific module work)
./pm_encoder.py . --include "pm_encoder.py" "tests/**" "README.md" \
    -o pm_encoder_partial_context.txt

# Review context (for code review)
./pm_encoder.py . --exclude "tests/" "docs/examples/" \
    -o pm_encoder_review_context.txt
```

---

## Development Workflow

### Iterative Enhancement Cycle

pm_encoder follows a pragmatic, iterative approach:

1. **Identify** - Document the specific context-sharing pain point
2. **Design** - Sketch solution maintaining format compatibility
3. **Implement** - Write code with inline documentation
4. **Dogfood** - Use the new feature to serialize pm_encoder itself
5. **Document** - Update README/TUTORIAL with examples
6. **Test** - Verify edge cases and backward compatibility

### Plus/Minus Format Contract

The Plus/Minus format is **sacred** and changes require high consensus:

```
++++++++++ [relative/path/to/file.ext] ++++++++++
[file content, preserved exactly as-is]
---------- [relative/path/to/file.ext] [MD5_checksum] [relative/path/to/file.ext] ----------
```

**Format Rules**:
1. **Delimiters**: Exactly 10 plus signs, 10 minus signs
2. **Paths**: POSIX-style, relative to project root, repeated in footer
3. **Checksum**: MD5 of UTF-8 encoded content, hex lowercase
4. **Newlines**: Content must end with newline; footer adds one if missing
5. **Encoding**: All content UTF-8, fallback to latin-1 if needed

**Backward Compatibility Promise**:
- Any AI system trained on current format must work with future versions
- Format extensions (e.g., metadata) must use optional comment syntax
- Breaking changes require major version bump and migration tool

### Assumption Handling

Given pm_encoder's utility focus, handle assumptions with **mark and proceed**:

```python
# ASSUMPTION: File paths under 4096 chars (PATH_MAX on most systems)
# CONFIDENCE: 95% - extremely rare edge case
# TODO-VERIFY: Test on Windows with \\?\ long path prefix

# ASSUMPTION: Projects under 100K files remain performant
# CONFIDENCE: 80% - may need optimization for monorepos
# PERF-DEBT: Current O(n log n) sort acceptable; consider streaming for massive repos
```

**Confidence Thresholds**:
- **95-100%**: Proceed without comment unless educationally valuable
- **80-94%**: Mark assumption, proceed with implementation
- **50-79%**: Mark assumption, implement with TODO for verification
- **<50%**: Flag for explicit user decision before proceeding

### Documentation Requirements

pm_encoder serves both developers and end-users:

| Change Type | Documentation Required |
|-------------|------------------------|
| Format modification | Update format spec + migration guide |
| New CLI flag | README usage + TUTORIAL example |
| Config option | README config section + JSON schema example |
| Performance change | Benchmark results + scale recommendations |
| Breaking change | CHANGELOG entry + migration path |
| Bug fix | Git commit message + close related issues |

**Style**: Documentation should be **example-driven** and **copy-pasteable**.

---

## Code Generation Standards

### Python Best Practices

pm_encoder targets **Python 3.6+** with minimal dependencies:

- **Typing**: Use type hints for public APIs and complex functions
- **Pathlib**: Prefer `Path` over `os.path` for all file operations
- **Error Handling**: Fail gracefully; never crash on bad input
- **Encoding**: UTF-8 first, latin-1 fallback, skip binary explicitly
- **Performance**: Optimize for common case (1K-10K files, <100MB total)
- **Dependencies**: Standard library only (no external packages)

### Quality Checklist

Before submitting code, verify:

- [ ] **Format Compliance**: Output passes Plus/Minus format validator
- [ ] **Self-Serialization**: `./pm_encoder.py . -o test.txt` succeeds
- [ ] **Cross-Platform**: Works on Linux, macOS, Windows (POSIX paths only in output)
- [ ] **Edge Cases**: Handles empty dirs, symlinks, permission errors
- [ ] **Encoding Safety**: Non-UTF8 files skip gracefully
- [ ] **Performance**: <5s for 10K files on modest hardware
- [ ] **Documentation**: README updated, TUTORIAL example if user-facing
- [ ] **Backward Compat**: Existing encoded outputs still parse correctly

### Deliverable Format

Structure your response as:

```markdown
## Implementation Summary
[Brief description of changes and rationale]

## Files Modified
- `pm_encoder.py` - [Specific changes]
- `README.md` - [Documentation updates]
- `tests/test_*.py` - [New tests added]

## Key Changes
1. [Major modification with technical detail]
2. [Impact on format/API/behavior]
3. [Performance implications if any]

## Testing Performed
- [x] Self-serialization test passed
- [x] Format validation passed
- [x] Tested on sample projects: [list]
- [ ] Windows testing pending (if applicable)

## Breaking Changes
[None] OR [Specific breaking change + migration path]

## Follow-Up Items
- [ ] [Remaining TODOs or improvements identified]
```

---

## Continuous Improvement Protocol

### Session-End Improvement

After each development session, reflect:

```markdown
### Process Improvement
**Observation:** [What friction did we encounter?]
**Suggestion:** [Specific improvement to workflow/tool/docs]
**Benefit:** [Expected impact on future development]
**Effort:** [S/M/L estimation]
```

### Friction Reporting

pm_encoder development should be smooth. Report friction points:

```markdown
**Friction [Confidence: 85%]**: Testing format changes requires manual diff checking
**Impact**: Slows iteration on format-related features
**Suggested Resolution**: Create automated format validator/differ tool
**Priority**: Medium - affects development velocity
```

### Pattern Recognition

Track recurring patterns for abstraction opportunities:

**Threshold**: After 2+ occurrences, propose abstraction

**Example Pattern**:
```python
# Pattern observed 3x: Path filtering with multiple criteria
# Current: Repeated fnmatch logic in multiple functions
# Proposal: PathFilter class with builder pattern
# Reuse potential: 4 current call sites + future features
# Complexity: Low - ~100 LOC, no external deps
```

### Meta-Tool Self-Improvement

Unique to pm_encoder: Use itself to improve itself

**Self-Application Checklist**:
- [ ] After major refactor: Serialize and compare output before/after
- [ ] Before release: Generate context and review with fresh AI session
- [ ] For new features: Dogfood by encoding example projects
- [ ] Documentation: Include pm_encoder's own output as example

---

## Context Requirements

### Essential Context Files

For comprehensive development sessions, include:

```bash
./pm_encoder.py . --include \
    "pm_encoder.py" \
    "README.md" \
    "TUTORIAL.md" \
    "CHANGELOG.md" \
    ".pm_encoder_config.json" \
    "tests/**/*.py" \
    "scripts/*.sh" \
    -o context.txt
```

**Minimal Context** (for quick fixes):
```bash
./pm_encoder.py . --include "pm_encoder.py" "README.md" -o context.txt
```

### State Awareness

Track pm_encoder project state:

- **Current Version**: Check `__version__` in pm_encoder.py or git tags
- **Open Issues**: GitHub issues labeled `bug`, `enhancement`, `format-spec`
- **Roadmap**: See ROADMAP.md or CHANGELOG.md "Planned" section
- **Technical Debt**: Search codebase for `TODO`, `FIXME`, `PERF-DEBT`, `TECH-DEBT`

### Configuration Files

pm_encoder's own configuration:

```json
{
  "ignore_patterns": [
    ".git",
    "__pycache__",
    "*.pyc",
    ".venv",
    "venv",
    ".pytest_cache",
    "*.egg-info",
    "dist",
    "build",
    ".DS_Store",
    "*.swp",
    "*.swo"
  ],
  "include_patterns": []
}
```

**Location**: `.pm_encoder_config.json` in project root

---

## Session Handoff Protocol

### End-of-Session Summary

When closing a development session (or on `sync: end-session`), provide:

```markdown
## Session Summary
**Session ID:** 2025-12-12 | pm_encoder-a7c3f | Turn: 12
**Duration:** ~2 hours
**Context Mode:** Serialized (full project)

### Completed
- Implemented `--depth` flag for shallow serialization
- Added integration tests for new flag
- Updated README and TUTORIAL with examples
- Fixed edge case in path handling for deeply nested dirs

### Decisions Made
1. **Depth Limit**: Defaulted to unlimited (None), user must specify --depth N explicitly
2. **Behavior**: Directories at depth limit are skipped entirely (not listed as empty)
3. **Error Handling**: Log warning if depth truncates expected files

### Pending Tasks
- [ ] Windows path testing for new depth feature
- [ ] Performance benchmark with --depth on large monorepo
- [ ] Consider adding --max-files limit (related feature)

### Next Steps
1. User testing: Try --depth on real-world projects, gather feedback
2. Documentation review: Ensure examples are clear
3. Release preparation: Update CHANGELOG, bump version to 1.2.0

### Context Updates
**Files Changed:**
- `pm_encoder.py` - Added depth tracking and limiting logic
- `tests/test_serialization.py` - New tests for depth feature
- `README.md` - Usage section updated
- `TUTORIAL.md` - Added depth limiting example

**New Context Generated:**
```bash
./pm_encoder.py . -o pm_encoder_v1.2.0-dev.txt
```

### Improvement Opportunities Identified
1. Test suite could benefit from parameterized tests (reduce duplication)
2. Consider adding pre-commit hooks for format validation
3. Documentation examples could be more diverse (not just Python projects)
```

### Cross-Session Continuity

To resume development in a new session:

1. **Load Latest Context**: Use most recent serialized output
2. **Review Session Summary**: Read last session's handoff notes
3. **Check Git Status**: `git status` and `git log -5 --oneline`
4. **Verify State**: Run `./pm_encoder.py . -o /tmp/test.txt` to ensure working state
5. **Declare Intent**: Start with `sync:` command explaining continuation

---

## Special Directives

### Format Specification Changes

Any modification to the Plus/Minus format requires:

1. **Proposal Document**: RFC-style doc explaining change and rationale
2. **Compatibility Analysis**: Impact on existing parsers/consumers
3. **Migration Path**: How existing encoded files remain valid
4. **Version Strategy**: When to bump major vs minor version
5. **Community Input**: If pm_encoder gains users, solicit feedback

### Performance Targets

pm_encoder should be "fast enough to not think about":

| Project Size | Target Time | Notes |
|--------------|-------------|-------|
| <1K files | <1 second | Near-instant feedback |
| 1K-10K files | <5 seconds | Typical mid-size project |
| 10K-50K files | <30 seconds | Large monorepo warning territory |
| >50K files | <5 minutes | Consider pagination/filtering |

**Optimization Priority**: O(n) operations on file list, minimize filesystem calls

### Testing Philosophy

Given the tool's simplicity, balance test coverage with practicality:

- **Unit Tests**: Core functions (filtering, sorting, format generation)
- **Integration Tests**: Full serialization workflows with sample projects
- **Edge Case Tests**: Empty dirs, symlinks, permissions, encoding issues
- **Regression Tests**: Known bugs should have test cases
- **Performance Tests**: Benchmark on synthetic large repos (optional)

**No Testing**: CLI argument parsing (argparse handles), trivial getters/setters

### Dependency Policy

pm_encoder is **standard library only** by design:

**Rationale**:
- Easy installation: Just download pm_encoder.py
- No dependency hell: Works everywhere Python 3.6+ is installed
- Portability: Single file can be copied/shared easily
- Trust: Users can audit ~250 LOC without external deps

**Exception Process**:
If a dependency is absolutely necessary:
1. Justify why standard library cannot solve it
2. Assess dependency health (active maintenance, security record)
3. Consider vendoring (copy into project) for small deps
4. Update installation docs and add `requirements.txt`
5. Requires 2+ maintainer consensus

### User-Facing Language

pm_encoder serves developers of varying experience:

- **Error Messages**: Actionable, suggest fix, include example
- **Help Text**: Examples before flags, common workflows prominent
- **Documentation**: Progressive disclosure (Quick Start → Tutorial → Reference)
- **Defaults**: Safe and intuitive; power users can customize

**Tone**: Friendly but technical. Assume user is competent but may be time-pressured.

---

## Meta-Development Notes

### pm_encoder's Development Meta-Pattern

Developing pm_encoder involves this workflow:

```
1. Work on feature/fix
2. Serialize project with pm_encoder itself
3. Share context with AI for review/next steps
4. AI uses pm_encoder's output to understand current state
5. Iterate on feedback
6. Repeat
```

This creates a **positive feedback loop**: Improvements to pm_encoder improve the experience of developing pm_encoder.

### Questions to Ask When Unsure

When facing design decisions:

1. **Does this help developers share better context with AI?**
2. **Does this maintain format backward compatibility?**
3. **Can this be implemented without external dependencies?**
4. **Will this scale to projects with 10K+ files?**
5. **Is the CLI interface intuitive for first-time users?**
6. **Would I use this feature in my own workflow?**

If answers lean negative, reconsider or simplify.

### Vision Alignment

pm_encoder exists to solve: **"How do I give an AI the full context of my project efficiently?"**

Features should support this by:
- Making context more complete (better filtering, less manual curation)
- Making context more efficient (compression, relevance ranking)
- Making context more maintainable (config files, reproducible commands)

Features that don't serve this vision should be questioned.

---

## Conclusion

pm_encoder is simple by design, powerful by utility. When developing it:

- **Respect the format**: It's the contract with all users and AI systems
- **Use the tool**: Dogfood relentlessly
- **Stay minimal**: Features should earn their complexity
- **Document generously**: Examples over explanations
- **Think recursively**: This tool enables its own development

Every change to pm_encoder ripples through the AI collaboration ecosystem. Develop with care, test thoroughly, and document clearly.

---

**Protocol**: AI Collaboration Protocol v2.0-Universal  
**Last Updated**: 2025-12-12  
**Maintainer**: Review and update as pm_encoder evolves  
**Feedback**: Use pm_encoder's own output to share context when discussing improvements to these instructions

---

## Research Framework Integration

### The Twins Comparative Study

pm_encoder is now part of an active software engineering research project comparing Python (reference) and Rust (parity) implementations using test vector-driven development.

**Key principle:** Every development session contributes to empirical research on language trade-offs and TDD effectiveness.

### Research-Aware Development Workflow

When working on pm_encoder, AI assistants should:

1. **Track Metrics** (After significant changes)
   ```bash
   make track-metrics
   ```
   This captures daily snapshot for research.

2. **Test Vectors First** (For Rust development)
   - Extract behavior from Python tests
   - Create test vector JSON
   - Implement Rust to pass vector
   - Never skip the test vector step

3. **Document Findings** (When discovering insights)
   - Bug taxonomy: Log with appropriate tags
   - Performance observations: Note in commit messages
   - Velocity data: Automatic from snapshots

### Research Context Files

**Essential reading:**
- `research/RESEARCH_FRAMEWORK.md` - KB-optimized overview (uploaded to Claude KB)
- `test_vectors/rust_parity/README.md` - Test vector status

**Full methodology:**
- `research/METHODOLOGY.md` - Complete research design
- `research/README.md` - Project overview

### The Test Vector Contract

**Core principle:** Python defines behavior. Rust must reproduce exactly.

```
Python Test → Extract Vector → Rust Test → Implementation
     ↓              ↓              ↓            ↓
  Validated    Specification   RED phase   GREEN phase
```

**Never write Rust features without test vectors first.** This measures TDD acceleration (hypothesis: 3-4x faster).

### Research Session Protocol

When starting a pm_encoder development session:

```markdown
Session: YYYY-MM-DD | pm_encoder-{context} | Turn: N
Context: serialized

[At session start, note research status:]
Current parity: X% (Y/30 vectors)
Timeline: Z days ahead/behind schedule
Phase: 1|2|3|4

[During development:]
- Document any bugs with taxonomy
- Note implementation time for features
- Track test vector creation vs implementation time

[At session end:]
- Run `make track-metrics` if significant progress
- Update research findings if insights discovered
```

### Multi-AI Research Coordination

The Twins involves multiple AI systems:

| AI | Role | Research Contribution |
|----|------|----------------------|
| Claude.ai (Opus/Sonnet) | Architect | Strategic decisions, methodology validation |
| AI Studio (Gemini) | Analyst | Performance analysis, statistical insights |
| Claude Code Server | Implementer | Code generation, test execution, metrics collection |

**Handoff protocol:** Each AI documents decisions and updates research context.

### Research Milestones

Current targets (as of 2025-12-14):

| Phase | Target Date | Vectors | Parity | Status |
|-------|-------------|---------|--------|--------|
| Phase 1 | Dec 14, 2025 | 9/30 | 30% | ✅ COMPLETE |
| Phase 2 | Dec 28, 2025 | 20/30 | 67% | 🔄 IN PROGRESS |
| Phase 3 | Jan 31, 2026 | 27/30 | 90% | 📋 PLANNED |
| Phase 4 | Mar 31, 2026 | 30/30 | 100% | 📋 PLANNED |

**Current status: 13 days ahead of schedule** ⚡

### Key Research Questions

1. **RQ1:** Do test parity % and coverage % converge to ~95%?
2. **RQ2:** Does TDD accelerate cross-language parity by 3-4x?
3. **RQ3:** What's the LOC ratio at full parity? (Currently: Rust is 19% of Python!)
4. **RQ4:** Do static types reduce test burden vs dynamic types?

### Research-Aware Commit Messages

Include research context when relevant:

```bash
git commit -m "feat: Python analyzer complete

Parity impact: +3.3% (10 → 11 vectors)
Implementation time: 2 hours
Test vector creation: 30 minutes
Ratio: 4x faster than traditional estimate

Research notes:
- Pattern matching simplified implementation
- Type safety caught 2 edge cases at compile time
- Total Rust LOC now 650 (still <25% of Python)
"
```

### Research Ethics

1. **No cherry-picking:** Report all data, including negative results
2. **Full transparency:** All code, data, methodology public
3. **Honest limitations:** Document what we can't conclude
4. **Reproducibility:** Everything versioned and documented

---

**Publication target:** ICSE 2027 (International Conference on Software Engineering)  
**Data license:** CC-BY-4.0  
**Code license:** MIT
