# End-to-End Tests for GitHub Pages Site

This directory contains Playwright tests for the AgentScript Editor GitHub Pages site.

## Overview

These tests verify that:
- The editor page loads correctly
- WASM module loads successfully
- Monaco editor initializes properly
- Recipe dropdown works
- Plugin documentation page loads

## Running Tests Locally

1. Build the WASM package and prepare the dist folder:
   ```bash
   wasm-pack build --target web --out-dir pkg --features wasm
   mkdir -p dist/recipes
   cp docs/index.html dist/index.html
   cp docs/plugin.html dist/plugin.html
   cp pkg/sf_agentscript.js dist/
   cp pkg/sf_agentscript_bg.wasm dist/
   find agent-script-recipes -name "*.agent" -exec sh -c 'cp "$1" dist/recipes/$(basename "$1" .agent).agent' _ {} \;
   ```

2. Install dependencies:
   ```bash
   npm install
   npx playwright install --with-deps chromium
   ```

3. Run tests (with built-in web server):
   ```bash
   npm test
   ```

4. Or run tests with UI mode:
   ```bash
   npm run test:ui
   ```

## CI Integration

The tests run automatically on pull requests via the `.github/workflows/test-pages.yml` workflow. The workflow:
1. Builds the WASM package
2. Prepares the dist folder
3. Starts a local HTTP server
4. Runs Playwright tests against the local server
5. Uploads test reports and screenshots as artifacts

## Test Structure

- `editor.spec.ts` - Tests for the main editor page
- `plugin.spec.ts` - Tests for the plugin documentation page

## Configuration

See `playwright.config.ts` in the root directory for Playwright configuration.
