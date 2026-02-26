# busbar-sf-agentscript

[![Crates.io](https://img.shields.io/crates/v/busbar-sf-agentscript.svg)](https://crates.io/crates/busbar-sf-agentscript)
[![docs.rs](https://docs.rs/busbar-sf-agentscript/badge.svg)](https://docs.rs/busbar-sf-agentscript)
[![CI](https://github.com/composable-delivery/busbar-sf-agentscript/actions/workflows/ci.yml/badge.svg)](https://github.com/composable-delivery/busbar-sf-agentscript/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

AgentScript parser, graph analysis, and LSP for [Salesforce Agentforce](https://www.salesforce.com/agentforce/).

Part of the [busbar](https://busbar.dev) suite of Salesforce Rust crates alongside
[busbar-sf-api](https://crates.io/crates/busbar-sf-api) and [busbar-sf-types](https://crates.io/crates/busbar-sf-types).

## Crates

| Crate | Version | Description |
|-------|---------|-------------|
| [`busbar-sf-agentscript`](https://crates.io/crates/busbar-sf-agentscript) | [![crates.io](https://img.shields.io/crates/v/busbar-sf-agentscript.svg)](https://crates.io/crates/busbar-sf-agentscript) | Umbrella crate — select features |
| [`busbar-sf-agentscript-parser`](https://crates.io/crates/busbar-sf-agentscript-parser) | [![crates.io](https://img.shields.io/crates/v/busbar-sf-agentscript-parser.svg)](https://crates.io/crates/busbar-sf-agentscript-parser) | Lexer, AST, serializer, validator |
| [`busbar-sf-agentscript-graph`](https://crates.io/crates/busbar-sf-agentscript-graph) | [![crates.io](https://img.shields.io/crates/v/busbar-sf-agentscript-graph.svg)](https://crates.io/crates/busbar-sf-agentscript-graph) | Reference graph, cycle detection, dead code |

## Quick Start

```toml
[dependencies]
# Everything (default)
busbar-sf-agentscript = "0.1"

# Parser only
busbar-sf-agentscript = { version = "0.1", default-features = false, features = ["parser"] }

# Or depend on sub-crates directly
busbar-sf-agentscript-parser = "0.1"
busbar-sf-agentscript-graph  = "0.1"
```

```rust
use busbar_sf_agentscript::{parse, graph::RefGraph};

let source = std::fs::read_to_string("my-agent.agent").unwrap();
let ast = parse(&source).unwrap();

// Graph analysis
let graph = RefGraph::from_ast(&ast).unwrap();
println!("Topics: {}", graph.topic_count());
println!("Unreachable topics: {:?}", graph.unreachable_topics());
```

## Features

### Parser (`busbar-sf-agentscript-parser`)

- Indentation-sensitive (3-space) lexer
- Full AST with source-location spans (`Spanned<T>`)
- Round-trip serializer — parse → AST → AgentScript source
- Semantic validator (undefined references, missing required blocks, etc.)
- WASM support (`features = ["wasm"]`)

### Graph (`busbar-sf-agentscript-graph`)

- Reference resolution — validates `@variables.*`, `@actions.*`, `@topic.*`
- Cycle detection — ensures topic transitions form a DAG
- Reachability analysis — finds unreachable topics from `start_agent`
- Dead code detection — unused actions and variables
- GraphML export

### LSP (`busbar-sf-agentscript-lsp`)

> Experimental — `publish = false` for now.

A Language Server Protocol server providing diagnostics, hover, and semantic tokens for `.agent` files.
Binary: `busbar-sf-agentscript-lsp`

## AgentScript Language

AgentScript (`.agent`) is Salesforce's YAML-like, indentation-sensitive (3-space) language for defining AI agent behaviour in Agentforce. Example:

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

## Workspace

```
crates/
  parser/   busbar-sf-agentscript-parser
  graph/    busbar-sf-agentscript-graph
  lsp/      busbar-sf-agentscript-lsp  (experimental)

tree-sitter-agentscript/   Tree-sitter grammar
packages/vscode-agentscript/  VS Code extension
zed-extension/             Zed editor extension
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions are welcome.

## License

Licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
