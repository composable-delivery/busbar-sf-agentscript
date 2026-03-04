/**
 * Pre-publish build script for `changeset publish`.
 *
 * Builds only the packages whose local version isn't yet on npm.
 * Runs automatically via `npm run release` in the root package.json.
 */
import { execSync } from 'child_process';
import { readFileSync } from 'fs';

const run = (cmd, opts = {}) => execSync(cmd, { stdio: 'inherit', ...opts });

function publishedVersion(pkgName) {
  try {
    return execSync(`npm view ${pkgName} version 2>/dev/null`, { encoding: 'utf8' }).trim();
  } catch {
    return null;
  }
}

const wasm   = JSON.parse(readFileSync('pkg/package.json',                    'utf8'));
const plugin = JSON.parse(readFileSync('plugin-agency/package.json',          'utf8'));
const ts     = JSON.parse(readFileSync('tree-sitter-agentscript/package.json','utf8'));

// ── WASM (Rust → wasm-bindgen → pkg/) ────────────────────────────────────────
if (publishedVersion(wasm.name) !== wasm.version) {
  console.log(`\n⚙  Building WASM ${wasm.version}…`);
  run('cargo build --lib --release --target wasm32-unknown-unknown --features wasm,graph');
  run('wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/busbar_sf_agentscript.wasm');
} else {
  console.log(`✓  WASM ${wasm.version} already on npm, skipping build`);
}

// ── SF Plugin ─────────────────────────────────────────────────────────────────
if (publishedVersion(plugin.name) !== plugin.version) {
  console.log(`\n⚙  Building SF plugin ${plugin.version}…`);
  run('npm ci',         { cwd: 'plugin-agency' });
  run('npm run build',  { cwd: 'plugin-agency' });
} else {
  console.log(`✓  SF plugin ${plugin.version} already on npm, skipping build`);
}

// ── Tree-sitter ───────────────────────────────────────────────────────────────
if (publishedVersion(ts.name) !== ts.version) {
  console.log(`\n⚙  Building tree-sitter ${ts.version}…`);
  run('npm ci',         { cwd: 'tree-sitter-agentscript' });
  run('npm run build',  { cwd: 'tree-sitter-agentscript' });
} else {
  console.log(`✓  Tree-sitter ${ts.version} already on npm, skipping build`);
}

console.log('\n✓  Build complete — running changeset publish…\n');
