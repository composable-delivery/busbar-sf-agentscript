# Contributing to sf-plugin-busbar-agency

## Prerequisites

- Node.js 18+
- Salesforce CLI (`sf`) — [install guide](https://developer.salesforce.com/tools/salesforcecli)
- `npm` (comes with Node.js)

The plugin uses a Rust/WebAssembly parser published as `@muselab/busbar-sf-agentscript`. You do not need a Rust toolchain to work on the plugin itself — the WASM binary is installed as an npm dependency.

---

## Project Structure

```
plugin-agency/
├── src/
│   ├── commands/agency/       # One file per sf command
│   │   ├── parse.ts
│   │   ├── validate.ts
│   │   ├── validate/
│   │   │   └── platform.ts
│   │   ├── agents/
│   │   │   ├── list.ts
│   │   │   └── select.ts
│   │   ├── query.ts
│   │   ├── actions.ts
│   │   ├── deps.ts
│   │   ├── graph.ts
│   │   ├── list.ts
│   │   ├── paths.ts
│   │   ├── impact.ts
│   │   └── version.ts
│   ├── lib/
│   │   └── agent-files.ts     # Shared file scanning and state utilities
│   └── wasm-loader.ts         # WASM initialization
├── messages/                  # Help text for each command (agency.<command>.md)
├── test/
│   ├── commands/agency/       # Tests mirror src/commands/agency/
│   │   └── fixtures/          # .agent fixture files for tests
│   └── helpers/               # WASM setup for vitest
├── esbuild.config.mjs         # Build script (bundles + copies WASM)
├── package.json
└── oclif.manifest.json        # Generated — do not edit manually
```

---

## Building

```bash
cd plugin-agency
npm install
node esbuild.config.mjs    # bundle TypeScript + copy WASM files
npx oclif manifest         # regenerate oclif.manifest.json
```

Or use the package script which does both:

```bash
npm run build
```

The build output goes to `lib/`. This directory is gitignored — it is generated on every build.

---

## Linking to the Salesforce CLI Locally

After building, link the plugin to your local `sf` installation so you can run commands directly:

```bash
# From the plugin-agency/ directory
sf plugins link .
```

This registers the plugin with your local `sf` CLI. Changes to source require a rebuild before they take effect:

```bash
node esbuild.config.mjs && sf agency parse --file some.agent
```

To unlink when done:

```bash
sf plugins unlink @muselab/sf-plugin-busbar-agency
# or
sf plugins unlink .
```

---

## Running Tests

```bash
cd plugin-agency
npx vitest run
```

Tests use vitest and run against the TypeScript source directly (no build step needed for tests). WASM is initialized via a global setup hook that copies the `.wasm` file from `node_modules` to `src/` before tests run.

To run a specific test file:

```bash
npx vitest run test/commands/agency/deps.test.ts
```

To run tests in watch mode during development:

```bash
npx vitest
```

---

## Adding a New Command

1. Create `src/commands/agency/<name>.ts` following the pattern of existing commands.
2. Create `messages/agency.<name>.md` with `summary`, `description`, `examples`, and flag messages.
3. Add the entry point to `esbuild.config.mjs` in the `entryPoints` array.
4. If your command lives in a new subdirectory, add a WASM copy step in `copyWasmFiles()`.
5. Add tests in `test/commands/agency/<name>.test.ts`.
6. Run `npm run build` to rebuild and regenerate the manifest.

### WASM functions available

Import the WASM module:

```typescript
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from '../../wasm-loader.js';
```

Key functions:

| Function | Returns |
|----------|---------|
| `parse_agent(source)` | AST object |
| `export_graph_json(source)` | JSON string (GraphExport) |
| `export_graphml(source)` | GraphML XML string |
| `render_graph(source, view)` | ASCII art string |
| `get_graph_stats(source)` | stats object |
| `extract_dependencies(source)` | DependencyReport object |
| `find_topic_usages(source, name)` | NodeRepr[] |
| `find_topic_transitions(source, name)` | NodeRepr[] |
| `find_variable_usages(source, name)` | JSON string `{readers, writers}` |
| `validate_agent_semantic(source)` | `{errors, warnings}` |
| `validate_graph(source)` | `{errors, warnings}` |

WASM functions for topics and variables throw (not return null) when the named element is not found. Wrap calls in `try/catch` and return empty results gracefully.

### Multi-file support pattern

All commands support `--file` (single file) and `--path` (directory scan). Use `resolveTargetFiles` from `src/lib/agent-files.ts`:

```typescript
import { resolveTargetFiles } from '../../lib/agent-files.js';

const files = resolveTargetFiles({
  file: flags.file,
  scanPath: flags.path,
  dataDir: this.config.dataDir,
});

const results: MyResult[] = [];

for (const filePath of files) {
  if (files.length > 1) {
    this.log(ansis.bold.dim(`\n─── ${path.relative(process.cwd(), filePath)} ───`));
  }
  const file = path.relative(process.cwd(), filePath);
  const source = fs.readFileSync(filePath, 'utf-8');
  // ... process source ...
  results.push({ file, ...resultData });
}

return files.length === 1 ? results[0] : results;
```

Always include `file: string` in your result interface so `--json` output identifies each file.

---

## Conventions

- **Messages**: All user-facing strings live in `messages/agency.<command>.md`, not in source code. Use `messages.getMessage('key')` and `messages.createError('key', [args])`.
- **Positional args**: Use `Args.string()` from `@oclif/core` — plain objects `{ required: true }` cause a runtime error.
- **Output**: Use `this.log()` for human-readable output and return structured data from `run()` for `--json` support.
- **No build artifacts in PRs**: The `lib/` directory and `oclif.manifest.json` are auto-generated; do not commit them as part of feature changes.

---

## Releasing

Releases are handled by the CI workflow. To bump the version manually:

```bash
npm version patch   # or minor / major
npm run build
```

The `prepack` script ensures a clean build before publish.
