# sf-plugin-busbar-agency

Salesforce CLI plugin for parsing and validating AgentScript files using WebAssembly.

## Overview

This plugin provides commands to work with Salesforce AgentScript files. It uses a high-performance WASM-based parser to parse, validate, and analyze AgentScript code directly from the Salesforce CLI.

## Features

- 🚀 **Fast WASM-based parsing** - Built with Rust for maximum performance
- ✅ **Syntax validation** - Quickly validate AgentScript files
- 🔍 **AST inspection** - View and query the Abstract Syntax Tree
- 📋 **Element listing** - List topics, variables, actions, and messages
- 🔎 **AST queries** - Extract specific parts of the AST using path notation
- 🛠️ **SF CLI integration** - Works seamlessly with the Salesforce CLI

## Installation

### Install as SF CLI Plugin

```bash
sf plugins install sf-plugin-busbar-agency
```

### Development Installation

From the plugin directory:

```bash
npm install
npm run build
sf plugins link .
```

## Commands

### `sf agency parse`

Parse an AgentScript file and output its Abstract Syntax Tree (AST).

**Usage:**

```bash
sf agency parse --file <path-to-agent-file>
```

**Flags:**

- `-f, --file <path>` (required) - Path to the AgentScript file to parse
- `-o, --format <format>` - Output format: `pretty` (default) or `json`

**Examples:**

```bash
# Parse and display pretty output
sf agency parse --file path/to/MyAgent.agent

# Parse and output JSON
sf agency parse --file path/to/MyAgent.agent --format json
```

### `sf agency validate`

Validate an AgentScript file for syntax errors.

**Usage:**

```bash
sf agency validate --file <path-to-agent-file>
```

**Flags:**

- `-f, --file <path>` (required) - Path to the AgentScript file to validate

**Examples:**

```bash
sf agency validate --file path/to/MyAgent.agent
```

### `sf agency validate platform`

Validate an AgentScript file against the Salesforce platform.

**Usage:**

```bash
sf agency validate platform --file <path-to-agent-file>
```

### `sf agency version`

Display the version of the AgentScript parser.

**Usage:**

```bash
sf agency version
```

### `sf agency list`

List specific elements from an AgentScript file.

**Usage:**

```bash
sf agency list --file <path-to-agent-file> --type <element-type>
```

**Flags:**

- `-f, --file <path>` (required) - Path to the AgentScript file
- `-t, --type <type>` (required) - Type of elements to list: `topics`, `variables`, `actions`, or `messages`
- `-o, --format <format>` - Output format: `pretty` (default) or `json`

**Examples:**

```bash
# List all topics
sf agency list --file path/to/MyAgent.agent --type topics

# List all variables
sf agency list --file path/to/MyAgent.agent --type variables

# List actions in JSON format
sf agency list --file path/to/MyAgent.agent --type actions --format json
```

### `sf agency query`

Query and extract specific parts of the AST using dot-notation paths.

**Usage:**

```bash
sf agency query --file <path-to-agent-file> --path <ast-path>
```

**Flags:**

- `-f, --file <path>` (required) - Path to the AgentScript file
- `-p, --path <path>` (required) - Dot-notation path to the AST element (e.g., `config.agent_name`)
- `-o, --format <format>` - Output format: `pretty` (default) or `json`

**Examples:**

```bash
# Query the agent name
sf agency query --file path/to/MyAgent.agent --path config.agent_name

# Query all topics
sf agency query --file path/to/MyAgent.agent --path topics
```

### `sf agency actions`

Extract action interface definitions from an AgentScript file.

**Usage:**

```bash
sf agency actions --file <path-to-agent-file>
```

**Flags:**

- `-f, --file <path>` (required) - Path to the AgentScript file
- `--format <format>` - Output format: `table` (default), `json`, `typescript`, or `markdown`
- `--target <type>` - Filter by target type: `all`, `flow`, `apex`, or `prompt`

**Examples:**

```bash
sf agency actions --file path/to/MyAgent.agent
sf agency actions --file path/to/MyAgent.agent --format typescript
```

### `sf agency graph`

Export the topic reference graph.

**Usage:**

```bash
sf agency graph --file <path-to-agent-file>
```

### `sf agency deps`

Extract Salesforce org dependencies from an AgentScript file.

**Usage:**

```bash
sf agency deps --file <path-to-agent-file>
```

## AgentScript Language

AgentScript is Salesforce's language for defining AI agent behavior. It features:

- YAML-like indentation-based syntax
- Configuration blocks for agent metadata
- Variable declarations with type safety
- System instructions and messages
- Topic-based conversation flow
- Action definitions with reasoning

Example AgentScript file:

```agentscript
config:
   agent_name: "HelloWorld"
   agent_version: "1.0"

variables:
   greeting: mutable string = "Hello"

system:
   messages:
      welcome: "Welcome to the agent!"
   instructions: "Be helpful and friendly."

start_agent topic_selector:
   description: "Entry point"
   reasoning:
      actions:
         start: @utils.transition to @topic.main

topic main:
   description: "Main conversation topic"
   reasoning:
      instructions:|
         Greet the user warmly.
```

## Development

### Build

```bash
npm run build
```

### Clean

```bash
npm run clean
```

## Technical Details

### WASM Parser

This plugin uses a Rust-based parser compiled to WebAssembly for:

- **Performance**: Native-speed parsing in Node.js
- **Portability**: Works across all platforms without native dependencies
- **Safety**: Memory-safe Rust implementation
- **Maintainability**: Shared parser logic with other tools

### Architecture

```
plugin-agency/
├── src/
│   ├── commands/
│   │   └── agency/
│   │       ├── parse.ts           # sf agency parse
│   │       ├── validate.ts        # sf agency validate
│   │       ├── validate/
│   │       │   └── platform.ts    # sf agency validate platform
│   │       ├── actions.ts         # sf agency actions
│   │       ├── deps.ts            # sf agency deps
│   │       ├── graph.ts           # sf agency graph
│   │       ├── list.ts            # sf agency list
│   │       ├── query.ts           # sf agency query
│   │       └── version.ts         # sf agency version
│   └── index.ts
├── messages/                      # Command help messages
├── lib/                           # Compiled JavaScript (generated)
└── package.json
```

## License

MIT

## Contributing

Contributions are welcome! Please see the main repository at [composable-delivery/busbar-sf-agentscript](https://github.com/composable-delivery/busbar-sf-agentscript).

## Support

For issues and questions, please file an issue on the [GitHub repository](https://github.com/composable-delivery/busbar-sf-agentscript/issues).
