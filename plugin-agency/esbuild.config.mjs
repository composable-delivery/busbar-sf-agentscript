import * as esbuild from 'esbuild';
import { copyFileSync, mkdirSync, existsSync, readdirSync, cpSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Copy messages directory to lib/commands/agency for bundled code
function copyMessages() {
  const srcMessages = join(__dirname, 'messages');
  const destMessages = join(__dirname, 'lib', 'commands', 'agency', 'messages');

  if (existsSync(srcMessages)) {
    cpSync(srcMessages, destMessages, { recursive: true });
    console.log('Copied messages/ to lib/commands/agency/messages/');
  }
}

// Copy WASM files to lib directory and commands directory
function copyWasmFiles() {
  const wasmSources = [
    { pkg: 'busbar-sf-agentscript' },
    { pkg: 'busbar-sf-agentscript-graph' },
  ];

  const libDir = join(__dirname, 'lib');
  const commandsDir = join(__dirname, 'lib', 'commands', 'agency');
  const validateDir = join(__dirname, 'lib', 'commands', 'agency', 'validate');

  // Ensure directories exist
  for (const dir of [libDir, commandsDir, validateDir]) {
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }
  }

  for (const { pkg } of wasmSources) {
    const pkgDir = join(__dirname, 'node_modules', pkg);
    if (existsSync(pkgDir)) {
      const files = readdirSync(pkgDir);
      for (const file of files) {
        if (file.endsWith('.wasm')) {
          copyFileSync(join(pkgDir, file), join(libDir, file));
          console.log(`Copied ${file} to lib/`);
          copyFileSync(join(pkgDir, file), join(commandsDir, file));
          console.log(`Copied ${file} to lib/commands/agency/`);
          copyFileSync(join(pkgDir, file), join(validateDir, file));
          console.log(`Copied ${file} to lib/commands/agency/validate/`);
        }
      }
    }
  }
}

// Build configuration
const buildOptions = {
  entryPoints: [
    'src/index.ts',
    'src/commands/agency/parse.ts',
    'src/commands/agency/validate.ts',
    'src/commands/agency/version.ts',
    'src/commands/agency/query.ts',
    'src/commands/agency/list.ts',
    'src/commands/agency/deps.ts',
    'src/commands/agency/actions.ts',
    'src/commands/agency/graph.ts',
    'src/commands/agency/validate/platform.ts',
  ],
  bundle: true,
  platform: 'node',
  target: 'node18',
  format: 'cjs',
  outdir: 'lib',
  outExtension: { '.js': '.js' },
  sourcemap: true,
  // Keep oclif and salesforce packages external - they use __dirname for messages
  external: [
    // Don't bundle node built-ins
    'fs',
    'path',
    'url',
    'util',
    'os',
    'crypto',
    'stream',
    'events',
    'buffer',
    'child_process',
    'http',
    'https',
    'net',
    'tls',
    'zlib',
    'readline',
    'tty',
    'assert',
    // Don't bundle Salesforce/oclif packages - they're provided at runtime
    '@oclif/*',
    '@salesforce/*',
  ],
  loader: {
    '.wasm': 'file',
  },
  // Handle WASM imports by treating them as external and loading at runtime
  plugins: [
    {
      name: 'wasm-loader',
      setup(build) {
        // Mark .wasm files as external - they'll be loaded at runtime
        build.onResolve({ filter: /\.wasm$/ }, (args) => {
          return { path: args.path, external: true };
        });
      },
    },
  ],
};

async function build() {
  try {
    // Build with esbuild
    console.log('Bundling with esbuild...');
    await esbuild.build(buildOptions);

    // Copy WASM files
    console.log('Copying WASM files...');
    copyWasmFiles();

    // Copy messages
    console.log('Copying messages...');
    copyMessages();

    console.log('Build complete!');
  } catch (error) {
    console.error('Build failed:', error);
    process.exit(1);
  }
}

build();
