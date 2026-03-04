# Changesets

This directory contains pending changesets — descriptions of changes that have
not yet been included in a release.

## Adding a changeset

When you make a change that should result in a new version of one or more
packages, run:

```bash
npx changeset
```

This walks you through selecting which packages changed and what kind of
version bump is needed (patch / minor / major), then creates a `.md` file in
this directory.

**Commit that file with your PR.** The Changesets bot will then open (or update)
a "Version PR" that bumps the relevant `package.json` versions and
`CHANGELOG.md` files. Merging the Version PR triggers automatic publishing to
npm via the `changesets.yml` workflow.

## Packages tracked

| Package | Directory | Registry |
|---|---|---|
| `@muselab/busbar-sf-agentscript` | `pkg/` | npm |
| `@muselab/sf-plugin-busbar-agency` | `plugin-agency/` | npm |
| `@muselab/tree-sitter-agentscript` | `tree-sitter-agentscript/` | npm |

> **Note:** `@muselab/busbar-sf-agentscript` and `@muselab/sf-plugin-busbar-agency`
> are **linked** — they always receive the same version bump. If the Rust core
> changes, bump WASM and the plugin gets pulled along automatically.

## Binary packages (VS Code, LSP, Zed)

These are **not** managed by Changesets. Release them by pushing a scoped tag:

```bash
git tag vscode-v0.1.0 && git push origin vscode-v0.1.0   # VS Code + LSP binaries
git tag lsp-v0.1.0    && git push origin lsp-v0.1.0      # LSP binaries + crates.io only
git tag v0.1.0        && git push origin v0.1.0          # all binaries together
```
