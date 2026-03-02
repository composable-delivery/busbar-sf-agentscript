import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export async function setup(): Promise<void> {
  const srcWasm = path.resolve(__dirname, '../../node_modules/@muselab/busbar-sf-agentscript/busbar_sf_agentscript_bg.wasm');
  const destWasm = path.resolve(__dirname, '../../src/busbar_sf_agentscript_bg.wasm');
  if (fs.existsSync(srcWasm) && !fs.existsSync(destWasm)) {
    fs.copyFileSync(srcWasm, destWasm);
  }

  // Remove oclif's SIGINT/SIGTERM handlers to prevent EEXIT:130 when vitest
  // tears down worker processes after test completion.
  process.removeAllListeners('SIGINT');
  process.removeAllListeners('SIGTERM');
}
