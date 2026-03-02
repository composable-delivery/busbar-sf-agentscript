# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.0.2] — 2026-03-02

### SF CLI Plugin (`@muselab/sf-plugin-busbar-agency`)

#### Added
- **`sf agency agents list`** — List all `.agent` files in a directory with their parsed agent names and current selection state.
- **`sf agency agents select`** — Interactively select (or `--all` / `--none`) which agent files subsequent commands should target. Selection is persisted and used automatically by all commands.
- **Multi-file support for all commands** — Every command now accepts `--path <dir>` (recursive `.agent` scan) in addition to `--file`. A saved agent selection (from `sf agency agents select`) is used automatically when neither flag is provided.
- **`file` field in JSON output** — When running against multiple files, every result object now includes a `file` field so consumers can identify which agent file each result came from.
- **`sf agency deps --group dependency`** — New `--group` flag (`file` | `dependency`). `--group dependency` inverts the view: instead of deps per agent, shows each unique dependency with the list of agent files that use it.
- **`sf agency query <path>` unified command** — Replaces the three separate `sf agency query topic`, `sf agency query variable`, and `sf agency query action` subcommands with a single path-based interface:
  - `/topics/<name>` — incoming references and outgoing transitions for a topic
  - `/variables/<name>` — readers and writers for a variable
  - `/actions/<name>` — action definition and the reasoning steps that invoke it
  - `dot.notation.path` — raw AST traversal (unchanged)

#### Changed
- `sf agency parse` JSON output is now `{ file, ast }` instead of the raw AST object, so the source file is always identifiable.
- `sf agency impact` flag renamed: `--dir` → `--path` for consistency with other commands.
- `sf agency query` now uses a positional argument for the query path instead of `--path`, and `--path` is now the directory scan flag (consistent with all other commands).

#### Removed
- `sf agency query topic <name>` subcommand (use `sf agency query /topics/<name>`)
- `sf agency query variable <name>` subcommand (use `sf agency query /variables/<name>`)
- `sf agency query action <name>` subcommand (use `sf agency query /actions/<name>`)

#### Docs
- Rewrote `plugin-agency/README.md` to focus entirely on the SF CLI plugin with a full command reference.
- Added `plugin-agency/CONTRIBUTING.md` covering local build setup, linking to `sf` CLI, and conventions for adding new commands.

---

## [0.0.1-beta.3] — 2025-12-01

### SF CLI Plugin

#### Added
- `sf agency graph` — ASCII, GraphML, Mermaid, and HTML topic flow graph export with optional `--stats`.
- `sf agency paths` — DFS enumeration of all execution paths through the topic graph, with cycle detection.
- `sf agency impact` — Scan a directory for agents that depend on a specific Salesforce resource.
- `sf agency deps --retrieve` — Retrieve dependent metadata from a target org after extracting dependencies.
- `sf agency query topic`, `sf agency query variable`, `sf agency query action` — Semantic queries for specific AST constructs.

### Core Parser (`busbar-sf-agentscript`)

#### Added
- WASM build published to npm as `@muselab/busbar-sf-agentscript`.
- Graph analysis functions: `export_graph_json`, `export_graphml`, `render_graph`, `get_graph_stats`.
- Semantic query functions: `find_topic_usages`, `find_topic_transitions`, `find_variable_usages`.
- Dependency extraction: `extract_dependencies`.

---

## [0.0.1] — 2025-11-01

Initial public release.

### Added
- AgentScript parser (Rust + WASM).
- `sf agency parse`, `sf agency validate`, `sf agency list`, `sf agency query`, `sf agency actions`, `sf agency deps`.
- VS Code extension with syntax highlighting and real-time diagnostics.
- LSP server for Neovim, Helix, and other editors.
- Tree-sitter grammar (`@muselab/tree-sitter-agentscript`).
