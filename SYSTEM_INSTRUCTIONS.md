# System Instructions for Voyager Observatory
**Version: 3.0-VO**
**Generated: 2025-12-26**
**Language: Rust (Voyager Observatory), Python (Legacy Reference)**
**Protocol: AI Collaboration Protocol v3.0 (VO-Native)**

---

## Core Philosophy

Voyager Observatory (VO) is a **context serialization tool for AI collaboration** — it exists to facilitate effective codebase understanding between developers and AI assistants. As such, development must be:

1. **Self-Aware**: Recognize that changes to VO affect how developers collaborate with AI systems
2. **Celestial Metaphors**: Use astronomical terminology for code concepts (stars=functions, nebulae=docs, constellations=modules)
3. **Multi-Modal**: Support both CLI-based LLMs (Claude Code, Cursor) and web-based LLMs (ChatGPT, Claude.ai)
4. **Dogfooded**: Use VO itself to share context during its own development

### The Observatory Paradox

VO serializes projects for AI consumption, including itself. This creates a recursive relationship where:
- We use AI to develop VO
- VO helps us share context with AI
- Changes to VO affect how we use AI for its development

**Implication**: Every modification must consider its impact on the AI collaboration workflow.

---

## VO Quick Reference

### CLI Commands (All Platforms)

```bash
# Survey project health
vo /path/to/project --survey composition

# Get context with lens and budget
vo /path/to/project --lens architecture --token-budget 15k

# Explore with intent (onboarding, debug, security, migration)
vo /path/to/project --explore onboarding

# Auto-focus on single file (Microscope mode)
vo /path/to/project/src/lib.rs

# Evolution analysis (requires git history)
vo /path/to/project --survey evolution
```

### MCP Integration (For Claude Code, Cursor)

When VO is configured as an MCP server:

```json
// ~/.claude/mcp.json or ~/.gemini/settings.json
{
  "mcpServers": {
    "vo": {
      "command": "/path/to/vo",
      "args": ["--server", "/path/to/default/project"]
    }
  }
}
```

**Available MCP Tools:**
| Tool | Purpose |
|------|---------|
| `get_context` | Full project snapshot with lens/budget |
| `zoom` | Deep dive into function/class/file |
| `explore_with_intent` | Guided exploration (onboarding, debug, security) |
| `report_utility` | Mark files as useful for learning |
| `session_list` | List saved zoom sessions |
| `session_create` | Create new zoom session |

### xclip Workflow (For Web-Based LLMs)

When working with ChatGPT, Claude.ai, or Gemini, humans must run commands and paste output:

```bash
# Linux - Copy to clipboard
vo . --lens architecture --token-budget 15k | xclip -selection clipboard

# macOS - Copy to clipboard
vo . --lens architecture --token-budget 15k | pbcopy

# Windows PowerShell - Copy to clipboard
vo . --lens architecture --token-budget 15k | Set-Clipboard

# Save to file for upload
vo . --lens architecture -o context.txt
```

**xclip Tips:**
1. Always use `2>&1` when capturing survey output: `vo . --survey composition 2>&1 | xclip -selection clipboard`
2. Keep budgets reasonable for chat: 5k-15k tokens
3. Use `--skeleton=true` for large codebases (signatures only)

---

## Session Management Protocol

### Session Identification
Each AI response in VO development begins with:
```
Session: 2025-12-26 | vo-a7c3f | Turn: 1
Context: [surveyed|serialized|partial|minimal]
Task: [Precise, concise rephrasing of the user's request]
```

**Response Protocol**:
1. **Rephrase First**: Start by restating the user's question/request in precise, concise terms
2. **Acknowledge Context**: Note what VO context was used (survey, lens, zoom)
3. **Structured Answer**: Provide the response with clear sections

**Components**:
- **Date**: ISO format (YYYY-MM-DD)
- **Hash**: First 5 characters of initial prompt SHA-256
- **Turn**: Sequential number within session (resets each session)
- **Context**: Level of project context provided
  - `surveyed` - Project health check via `--survey`
  - `serialized` - Full VO context via `get_context`
  - `partial` - Specific files/modules only via `zoom`
  - `minimal` - Working from memory/documentation only

### Task Classification

Prefix requests with appropriate tags to set expectations:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `survey:` | Health check request | `survey: Check project composition before refactor` |
| `feature:` | New capability | `feature: Add ODF extraction support` |
| `fix:` | Bug resolution | `fix: Cache invalidation fails on branch switch` |
| `improve:` | Refactoring/optimization | `improve: Reduce memory footprint for large repos` |
| `document:` | Documentation updates | `document: Add MCP configuration guide` |
| `test:` | Test creation/modification | `test: Add integration tests for Chronos Warp` |
| `sync:` | Context update/alignment | `sync: Update context with latest main branch` |
| `explore:` | Codebase exploration | `explore: How does the token budgeting work?` |
| `zoom:` | Deep dive into symbol | `zoom: Understand the Engine::serialize function` |

### Context Generation by Task Type

| Task Type | Recommended Approach | Budget |
|-----------|---------------------|--------|
| Quick question | `vo . --token-budget 5k` | 5k |
| Feature planning | `vo . --lens architecture --token-budget 15k` | 15k |
| Bug investigation | `vo . --lens debug --token-budget 20k` | 20k |
| Security audit | `vo . --lens security --token-budget 30k` | 30k |
| Full review | `vo . --token-budget 50k` | 50k |
| Single file | `vo path/to/file.rs` (auto-focus) | - |

### For CLI-Based LLMs (Claude Code, Cursor)

Before responding to development requests:
1. Run `explore_with_intent(intent="onboarding")` for new projects
2. Use `get_context(lens="architecture")` for design discussions
3. Use `zoom(target="file=...")` for specific file work
4. Use `get_context(lens="debug")` for bug hunting

### For Web-Based LLMs (ChatGPT, Claude.ai, Gemini)

Instruct humans to:
1. Run VO commands and copy output to clipboard
2. Paste into chat as code block
3. Use smaller token budgets (5k-15k) to fit context limits

**Example prompt for web LLMs:**
```
Please run this command and paste the output:
vo /your/project --lens architecture --token-budget 10k | xclip -selection clipboard
```

---

## Celestial Terminology

VO uses astronomical metaphors for code concepts:

| VO Term | Meaning | Code Equivalent |
|---------|---------|-----------------|
| **Star** | Function/Method | `fn decode()` |
| **Nebula** | Documentation | Doc comments, README |
| **Dark Matter** | Complexity/Debt | TODOs, complex code |
| **Constellation** | Directory/Module | `src/`, `crates/` |
| **Galaxy** | Full Project | Entire workspace |
| **Mass** | Lines of Code | LOC count |
| **Stellar Density** | Functions per LOC | Code structure ratio |
| **Chronos Warp** | Persistent cache | `.voyager/cache/` |
| **Warp Engaged** | Cache hit | Instant results |
| **Warp Calibrating** | Cache miss | Full analysis |
| **Temporal Drift** | Code age/decay | Time since last change |

---

## Development Workflow

### Iterative Enhancement Cycle

VO follows a pragmatic, iterative approach:

1. **Survey** - Check project health with `--survey composition`
2. **Identify** - Document the specific context-sharing pain point
3. **Design** - Sketch solution using `--lens architecture`
4. **Implement** - Write code with celestial terminology
5. **Dogfood** - Use VO to serialize the VO project itself
6. **Document** - Update README with examples
7. **Test** - Verify with TDD test vectors

### Development Session Protocol

```bash
# 1. Check health before changes
vo . --survey composition

# 2. Get architecture context
vo . --lens architecture --token-budget 15k

# 3. Implement feature
# ... code ...

# 4. Verify health maintained
vo . --survey composition

# 5. Run tests
cargo test --workspace
```

---

## Code Generation Standards

### Rust Best Practices

VO is written in **Rust** with focus on performance:

- **Typing**: Use Rust's type system for safety
- **Error Handling**: Use `Result<T, E>` with descriptive errors
- **Performance**: Target sub-second for most operations
- **Celestial Names**: Use astronomical metaphors for types and functions
- **Dependencies**: Minimal, well-audited crates only

### Quality Checklist

Before submitting code, verify:

- [ ] **Self-Survey**: `vo . --survey composition` shows healthy metrics
- [ ] **Cross-Platform**: Works on Linux, macOS, Windows
- [ ] **Performance**: <1s for typical projects (10K files)
- [ ] **Celestial Terminology**: New types use astronomical names
- [ ] **MCP Compliance**: Changes don't break MCP protocol
- [ ] **Documentation**: README updated with examples
- [ ] **Tests**: Test vectors added for new features

---

## Research Framework Integration

### The Twins Comparative Study

VO has a Python reference implementation (pm_encoder v1.7) and Rust production engine (VO v2.4). Development contributes to empirical research on language trade-offs.

**Key principle:** Every development session contributes to research on language performance and developer ergonomics.

### Research Context Files

**Essential reading:**
- `research/RESEARCH_FRAMEWORK.md` - Research overview
- `test_vectors/rust_parity/README.md` - Test vector status
- `docs/THE_TWINS_ARCHITECTURE.md` - Twin architecture design

### Research-Aware Commit Messages

Include research context when relevant:

```bash
git commit -m "feat(VO): Add Chronos Warp cache

Performance impact: 10x faster repeat scans
Cache TTL: 24 hours with git HEAD invalidation
Celestial terminology: Warp Engaged/Calibrating/Offline

Research notes:
- Cache hit rate ~95% in typical workflows
- Binary format using bincode for speed
"
```

---

## Session Handoff Protocol

### End-of-Session Summary

When closing a development session, provide:

```markdown
## Session Summary
**Session ID:** 2025-12-26 | vo-a7c3f | Turn: 12
**Duration:** ~2 hours
**Context Mode:** Surveyed + Serialized

### Completed
- Implemented Chronos Warp cache
- Added cache invalidation tests
- Updated README with caching docs

### Decisions Made
1. **Cache Format**: Binary (bincode) for speed over readability
2. **Invalidation**: Git HEAD hash + 24-hour TTL
3. **Location**: `.voyager/cache/chronos/`

### Pending Tasks
- [ ] Windows path testing for cache
- [ ] Performance benchmark with large repos
- [ ] Consider cross-platform cache sharing

### Next Steps
1. User testing on real-world projects
2. Documentation review
3. Release preparation

### Context for Next Session
```bash
vo . --lens debug --token-budget 30k -o handoff_context.txt
```
```

### Cross-Session Continuity

To resume development in a new session:

1. **Survey First**: `vo . --survey composition`
2. **Load Context**: Use `get_context` or read handoff file
3. **Check Git Status**: `git status` and `git log -5 --oneline`
4. **Verify State**: Run `cargo test` to ensure working state
5. **Declare Intent**: Start with context about continuation

---

## Multi-AI Collaboration

### LLM Capability Matrix

| LLM Type | VO Access | Context Workflow |
|----------|-----------|------------------|
| **Claude Code** | MCP direct | Use `get_context`, `zoom`, `explore_with_intent` tools |
| **Cursor** | MCP direct | Configure VO as MCP server |
| **ChatGPT** | Via human | Human runs `vo ... \| pbcopy` and pastes |
| **Claude.ai** | Via human | Human runs `vo ... \| xclip` and pastes |
| **Gemini** | MCP direct | Configure VO as MCP server |

### Web LLM Prompt Templates

**For ChatGPT/Claude.ai users:**

```markdown
I need help with [PROJECT]. Please ask me to run these VO commands:

1. Survey: `vo /path --survey composition 2>&1`
2. Context: `vo /path --lens [architecture|debug|security] --token-budget 10k`
3. Zoom: `vo /path/specific/file.rs`

I'll paste the output after running each command.
```

### Cross-AI Handoff

When switching between AI systems:

```bash
# Generate handoff context
vo . --lens debug --token-budget 30k -o handoff.txt

# Prompt for new AI:
# "Continue development on [PROJECT]. Handoff context attached."
```

---

## Troubleshooting

### VO Not Found
```bash
# Check if vo is in PATH
which vo || echo "vo not in PATH"

# Add to PATH
export PATH="$PATH:/path/to/vo/directory"
```

### MCP Connection Issues
```bash
# Test MCP server manually
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | vo --server .
```

### Context Too Large
```bash
# Reduce budget
vo . --token-budget 5k

# Use skeleton mode
vo . --skeleton=true --token-budget 10k

# Focus on specific directory
vo ./src/core --token-budget 5k
```

---

## Conclusion

Voyager Observatory transforms codebases into AI-digestible context. When developing with VO:

- **Survey first**: Always check project health before major work
- **Use appropriate lenses**: Architecture for design, Debug for bugs
- **Budget appropriately**: Don't send 50k tokens for a typo fix
- **Think celestially**: Stars are functions, nebulae are docs
- **Support all LLMs**: Both CLI-based and web-based workflows

Every change to VO ripples through the AI collaboration ecosystem. Develop with care, test thoroughly, and document clearly.

---

**Protocol**: AI Collaboration Protocol v3.0-VO
**Last Updated**: 2025-12-26
**Maintainer**: Review and update as VO evolves
**Feedback**: Use VO's own output to share context when discussing improvements
