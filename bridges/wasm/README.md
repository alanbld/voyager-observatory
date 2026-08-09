# Voyager Observatory - WASM Bridge

> *"The Observatory can now run in a vacuum."*

This package provides the Voyager Observatory context serialization engine compiled to WebAssembly. It enables "Planetarium Scans" in any JavaScript environment—Node.js, browsers, VS Code extensions, and more.

## Installation

```bash
npm install @voyager-observatory/wasm
```

## Quick Start

```javascript
const { VoyagerObservatory } = require('@voyager-observatory/wasm');

const vo = new VoyagerObservatory();

// Scan files with architecture lens
const context = vo.scan([
    { path: 'src/main.rs', content: 'fn main() { println!("Hello"); }' },
    { path: 'README.md', content: '# My Project' }
], { lens: 'architecture' });

console.log(context);
```

## API Reference

### `VoyagerObservatory`

#### Properties

- `version` - Library version string
- `lenses` - Available spectral filters: `['architecture', 'debug', 'security', 'onboarding']`

#### Methods

##### `scan(files, config?)`

Perform a Planetarium Scan on the provided files.

```javascript
const context = vo.scan(files, {
    lens: 'architecture',      // Spectral filter
    token_budget: 50000,       // Max tokens
    budget_strategy: 'hybrid', // 'drop', 'truncate', or 'hybrid'
    truncate_lines: 100,       // Max lines per file
    truncate_mode: 'smart'     // 'head', 'tail', or 'smart'
});
```

##### `quickScan(files)`

Minimal lens scan for rapid context generation.

##### `architectureScan(files, tokenBudget?)`

Architecture-focused scan highlighting system design.

##### `securityScan(files, tokenBudget?)`

Security-focused scan highlighting auth, crypto, validation.

##### `debugScan(files, tokenBudget?)`

Debug-focused scan highlighting tests, error handlers, logs.

## Use Cases

### VS Code Extension

```javascript
// In your VS Code extension
const { VoyagerObservatory } = require('@voyager-observatory/wasm');

async function generateContext(document) {
    const vo = new VoyagerObservatory();
    const files = await collectWorkspaceFiles();
    return vo.architectureScan(files, 100000);
}
```

### Browser-Based IDE

```javascript
// In a web application (after bundling with webpack/vite)
import { VoyagerObservatory } from '@voyager-observatory/wasm';

const vo = new VoyagerObservatory();
const context = vo.scan(editorFiles, { lens: 'debug' });
sendToAI(context);
```

## Building from Source

```bash
# From the repository root
cd rust
wasm-pack build --target nodejs --features wasm
mv pkg ../bridges/wasm/
```

## Demo

Run the Planetarium Scan demo:

```bash
node planetarium_demo.js
```

## License

MIT - See [LICENSE](../../LICENSE)

---

*"The engine is tested. The optics are clean. The Voyager can now explore any galaxy."*
