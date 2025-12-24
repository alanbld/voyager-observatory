# Fractal Context Engine Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Claude Code + DeepSeek
**Date:** 2025-12-23

## Overview

The Fractal Context Engine provides hierarchical, zoomable context extraction for LLM reasoning. It transforms flat source code into nested semantic layers that can be navigated like a fractal structure.

## Core Concepts

### 1. Zoom Levels

The fractal hierarchy consists of 8 zoom levels, from broadest to most detailed:

| Level | Description | Example Content |
|-------|-------------|-----------------|
| `Project` | Entire repository/workspace | Files, dependencies, build config |
| `Module` | Directory/namespace | Module files, exports |
| `File` | Single source file | Imports, symbols, structure |
| `Symbol` | Function/class/struct | Signature, parameters, docs |
| `Block` | Code block (if/loop/match) | Condition, body, variables |
| `Line` | Individual line | Tokens, comments |
| `Expression` | Sub-expression | Type info, dependencies |
| `Token` | Individual token | Token type, value, position |

### 2. Navigation Operations

| Operation | Description | Use Case |
|-----------|-------------|----------|
| `zoom_in` | Navigate to more detailed layer | File → Symbol → Block |
| `zoom_out` | Navigate to broader context | Block → Symbol → File |
| `pan` | Navigate to related elements at same level | Function → Sibling functions |

### 3. Relationships

Elements track relationships to other elements:

- **Calls**: Functions this element calls
- **CalledBy**: Functions that call this element
- **Similar**: Semantically similar elements
- **DependsOn**: External dependencies
- **Exports**: What this element provides

## Data Structures

### FractalContext

```rust
pub struct FractalContext {
    pub id: String,
    pub current_view: ZoomView,
    pub layers: Vec<ContextLayer>,
    pub relationships: RelationshipGraph,
    pub semantic_clusters: Vec<SemanticCluster>,
    pub metadata: ExtractionMetadata,
}
```

### ZoomLevel

```rust
pub enum ZoomLevel {
    Project,
    Module,
    File,
    Symbol,
    Block,
    Line,
    Expression,
    Token,
}
```

### ContextLayer

```rust
pub struct ContextLayer {
    pub id: String,
    pub level: ZoomLevel,
    pub content: LayerContent,
    pub metadata: LayerMetadata,
    pub children: Vec<ContextLayer>,
    pub parent_id: Option<String>,
    pub sibling_ids: Vec<String>,
    pub confidence: f32,
}
```

### LayerContent

Discriminated union based on zoom level:

```rust
pub enum LayerContent {
    Project { name, files, dependencies, ... },
    Module { name, path, exports, ... },
    File { path, language, imports, symbols, ... },
    Symbol { name, kind, signature, parameters, ... },
    Block { block_type, condition, body, ... },
    Line { number, text, tokens, ... },
    Expression { expr, type_hint, ... },
    Token { token_type, value, position },
}
```

## API

### Builder Pattern

```rust
let context = FractalContextBuilder::for_file("src/main.rs")
    .with_depth(ExtractionDepth::Full)
    .with_relationships(true)
    .with_clustering(true)
    .build()?;
```

### Navigation

```rust
let mut nav = FractalNavigator::new(context);

// Zoom into specific function
nav.zoom_in(ZoomTarget::Symbol("main"))?;

// Zoom out to file level
nav.zoom_out()?;

// Pan to related functions
let siblings = nav.pan(PanDirection::Siblings)?;
```

### Query

```rust
let results = context.query(ContextQuery::FindPattern("async fn"))?;
let rels = context.query(ContextQuery::GetRelationships("main"))?;
```

## Performance Requirements

| Metric | Target |
|--------|--------|
| Context build (1000 LOC) | < 100ms |
| Zoom operation | < 10ms |
| Pan operation | < 50ms |
| Memory (per file) | < 10MB |

## Output Format

JSON output for LLM consumption:

```json
{
  "id": "ctx_abc123",
  "current_view": {
    "level": "symbol",
    "focus": { "name": "process_data", "kind": "function" },
    "range": { "start": 45, "end": 89 }
  },
  "zoom_in": {
    "blocks": [
      { "type": "if", "condition": "data.is_valid()", "line": 52 }
    ]
  },
  "zoom_out": {
    "file": { "path": "src/processor.rs", "symbols": 12 },
    "module": { "name": "processor", "files": 3 }
  },
  "relationships": {
    "calls": ["validate", "transform", "save"],
    "called_by": ["main", "batch_process"],
    "similar": ["process_batch", "process_stream"]
  }
}
```

## Test Requirements

### Unit Tests

1. ZoomLevel transitions (zoom_in/zoom_out)
2. ContextLayer hierarchy building
3. Relationship graph construction
4. Navigation state management

### Integration Tests

1. Full context extraction for Rust files
2. Full context extraction for Shell scripts
3. Cross-file relationship detection
4. Query language functionality

### Performance Tests

1. Build time for various file sizes
2. Navigation latency
3. Memory usage

## Implementation Phases

### Week 1 (Current)
- Core data structures
- Builder pattern
- Basic navigation
- Shell plugin foundation

### Week 2
- Relationship extraction
- Semantic clustering
- Caching layer
- Query language

### Week 3
- ABL plugin
- Cross-file analysis
- Performance optimization

### Week 4
- C# plugin
- Visualization output
- CLI polish
