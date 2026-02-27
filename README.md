# busbar-sf-agentscript

[![Crates.io](https://img.shields.io/crates/v/busbar-sf-agentscript.svg)](https://crates.io/crates/busbar-sf-agentscript)
[![docs.rs](https://docs.rs/busbar-sf-agentscript/badge.svg)](https://docs.rs/busbar-sf-agentscript)
[![CI](https://github.com/composable-delivery/busbar-sf-agentscript/actions/workflows/ci.yml/badge.svg)](https://github.com/composable-delivery/busbar-sf-agentscript/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/composable-delivery/busbar-sf-agentscript/branch/main/graph/badge.svg)](https://codecov.io/gh/composable-delivery/busbar-sf-agentscript)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

AgentScript parser, graph analysis, and LSP for [Salesforce Agentforce](https://www.salesforce.com/agentforce/).

AgentScript (`.agent`) is Salesforce's indentation-sensitive language for defining AI agent behavior in Agentforce. This project provides tooling for authoring, validating, and analyzing `.agent` files.

## Getting Started

Choose the option that fits your workflow:

| | Best for | What you get |
|---|---|---|
| [SF CLI Plugin](#sf-cli-plugin) | Salesforce developers | `sf agentscript` commands for validation, graph export, and CI integration |
| [VS Code Extension](#vs-code-extension) | VS Code users | Syntax highlighting, real-time diagnostics, and topic graph visualization |
| [LSP Server](#lsp-server) | Neovim, Helix, and other editors | Full language server for any LSP-capable editor |
| [Rust Crates](#rust-crates) | Rust developers | Parser, graph analysis library, and WASM support |

---

## SF CLI Plugin

Install with the [Salesforce CLI](https://developer.salesforce.com/tools/salesforcecli):

```sh
sf plugins install @composable-delivery/sf-agentscript
```

Validate an AgentScript file:

```sh
sf agentscript validate --file my-agent.agent
```

Export the topic reference graph:

```sh
sf agentscript graph --file my-agent.agent --format graphml --output graph.xml
```

Check for unreachable topics and dead code:

```sh
sf agentscript analyze --file my-agent.agent
```

The plugin exits non-zero on errors, making it suitable for CI pipelines.

---

## VS Code Extension

Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=composable-delivery.vscode-agentscript):

```sh
code --install-extension composable-delivery.vscode-agentscript
```

Or download the `.vsix` from [GitHub Releases](https://github.com/composable-delivery/busbar-sf-agentscript/releases) and install manually:

```sh
code --install-extension vscode-agentscript-<version>.vsix
```

**Features:**
- Syntax highlighting for `.agent` files
- Real-time diagnostics — undefined references, cycle detection, unreachable topics
- Hover documentation
- Semantic token highlighting
- Topic graph visualization (`AgentScript: Show Topic Graph`)
- Agent simulation (`AgentScript: Simulate Agent`)
- AgentScript Dependencies panel in the Explorer sidebar

**Settings:**

| Setting | Default | Description |
|---|---|---|
| `agentscript.lsp.serverPath` | auto | Path to a custom `busbar-sf-agentscript-lsp` binary |
| `agentscript.maxNumberOfProblems` | `100` | Maximum diagnostics shown per file |
| `agentscript.trace.server` | `off` | LSP communication tracing (`off`, `messages`, `verbose`) |

---

## LSP Server

The `busbar-sf-agentscript-lsp` binary powers the VS Code extension and works with any LSP-capable editor.

### Install

Download the pre-built binary for your platform from [GitHub Releases](https://github.com/composable-delivery/busbar-sf-agentscript/releases):

| Platform | Binary |
|---|---|
| macOS (Apple Silicon) | `busbar-sf-agentscript-lsp-aarch64-apple-darwin` |
| macOS (Intel) | `busbar-sf-agentscript-lsp-x86_64-apple-darwin` |
| Linux x86\_64 | `busbar-sf-agentscript-lsp-x86_64-unknown-linux-gnu` |
| Linux aarch64 | `busbar-sf-agentscript-lsp-aarch64-unknown-linux-gnu` |
| Windows x86\_64 | `busbar-sf-agentscript-lsp-x86_64-pc-windows-msvc.exe` |

Or install from source:

```sh
cargo install --git https://github.com/composable-delivery/busbar-sf-agentscript busbar-sf-agentscript-lsp
```

### Neovim

Using [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig):

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.agentscript then
  configs.agentscript = {
    default_config = {
      cmd = { 'busbar-sf-agentscript-lsp' },
      filetypes = { 'agentscript' },
      root_dir = lspconfig.util.root_pattern('sfdx-project.json', '.git'),
    },
  }
end

lspconfig.agentscript.setup {}
```

Add to `~/.config/nvim/init.lua` (or your config's `ftdetect`):

```lua
vim.filetype.add({ extension = { agent = 'agentscript' } })
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "agentscript"
scope = "source.agentscript"
file-types = ["agent"]
roots = ["sfdx-project.json", ".git"]
language-servers = ["agentscript-lsp"]

[language-server.agentscript-lsp]
command = "busbar-sf-agentscript-lsp"
```

### Other Editors

Any editor supporting LSP can be configured to launch `busbar-sf-agentscript-lsp` as a stdio language server for files with the `.agent` extension.

---

## Rust Crates

Add to `Cargo.toml`:

```toml
[dependencies]
# Parser + graph analysis (default)
busbar-sf-agentscript = "0.1"

# Parser only
busbar-sf-agentscript = { version = "0.1", default-features = false, features = ["parser"] }

# Parser + graph analysis
busbar-sf-agentscript = { version = "0.1", default-features = false, features = ["graph"] }
```

### Parser

```rust
use busbar_sf_agentscript::parse;

let source = std::fs::read_to_string("my-agent.agent").unwrap();
let ast = parse(&source).unwrap();
println!("{} topics defined", ast.topics.len());
```

### Graph Analysis

```rust
use busbar_sf_agentscript::{parse, graph::RefGraph};

let source = std::fs::read_to_string("my-agent.agent").unwrap();
let ast = parse(&source).unwrap();
let graph = RefGraph::from_ast(&ast).unwrap();

println!("Topics: {}", graph.topic_count());
println!("Unreachable: {:?}", graph.unreachable_topics());
println!("Unused actions: {:?}", graph.dead_actions());
```

### Individual Crates

The umbrella crate re-exports everything, but you can depend on individual crates directly:

| Crate | docs.rs | Description |
|---|---|---|
| `busbar-sf-agentscript-parser` | [![docs](https://docs.rs/busbar-sf-agentscript-parser/badge.svg)](https://docs.rs/busbar-sf-agentscript-parser) | Lexer, parser, AST, serializer, semantic validator |
| `busbar-sf-agentscript-graph` | [![docs](https://docs.rs/busbar-sf-agentscript-graph/badge.svg)](https://docs.rs/busbar-sf-agentscript-graph) | Reference resolution, cycle detection, reachability, GraphML export |

---

## AgentScript Language

AgentScript (`.agent`) is an indentation-sensitive (3-space) YAML-like language for defining Agentforce agent behavior. Example:

```
config:
   agent_name: MyAgent

variables:
   customerName: ""

start_agent:
   message: Hello! How can I help you today?

topics:
   - topic: SupportTopic
     instructions: Handle customer support requests.

     actions:
        - action: LookupCase
          type: FlowAction
          api_name: Look_Up_Case
```

See [`agent-script-recipes`](https://github.com/trailheadapps/agent-script-recipes) for real-world examples.

---

## Workspace

```
crates/
  parser/   busbar-sf-agentscript-parser    — lexer, AST, serializer, validator
  graph/    busbar-sf-agentscript-graph     — reference graph and analysis
  lsp/      busbar-sf-agentscript-lsp       — LSP server binary (experimental)

packages/                                   — VS Code extension
tree-sitter-agentscript/                    — Tree-sitter grammar
zed-extension/                              — Zed editor extension
agent-script-recipes/                       — test fixtures (git submodule)
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions are welcome.

### Local Setup

```bash
git clone https://github.com/composable-delivery/busbar-sf-agentscript
cd busbar-sf-agentscript
git submodule update --init         # recipe test fixtures
git config core.hooksPath .githooks # enable pre-commit checks
```

The pre-commit hook runs `cargo fmt --check` and `cargo clippy -- -D warnings` before every commit, matching CI exactly.

---

## License

Licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
