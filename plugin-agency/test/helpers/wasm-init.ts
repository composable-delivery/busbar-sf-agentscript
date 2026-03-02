import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

let initialized = false;

export async function initWasm(): Promise<void> {
  if (initialized) return;
  const wasmPath = path.resolve(__dirname, '..', '..', '..', 'pkg', 'busbar_sf_agentscript_bg.wasm');
  const wasmBytes = fs.readFileSync(wasmPath);
  const { initSync } = await import('@muselab/busbar-sf-agentscript');
  initSync({ module: wasmBytes });
  initialized = true;
}
