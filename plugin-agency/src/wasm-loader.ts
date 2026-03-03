import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

// @ts-ignore
import { initSync } from '@muselab/busbar-sf-agentscript';
// @ts-ignore
export * from '@muselab/busbar-sf-agentscript';

// Initialize WASM synchronously using the .wasm binary next to the bundled file.
// import.meta.url resolves to the actual bundled file location at runtime.
const wasmPath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  'busbar_sf_agentscript_bg.wasm'
);
initSync({ module: fs.readFileSync(wasmPath) });
