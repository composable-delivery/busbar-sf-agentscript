# AgentScript for Zed

This extension adds support for Salesforce AgentScript (`.agent`) files to the Zed editor.

## Features

- **Syntax Highlighting**: Complete syntax highlighting for AgentScript files using Tree-sitter.
- **Language Server Protocol (LSP)**: Integrated LSP support for diagnostics and more via `agentscript-lsp`.
- **Model Context Protocol (MCP)**: Includes the `aslab-mcp` server for AgentScript graph analysis and runtime simulation within Zed's AI context.
- **Code Outline**: Structure view for easy navigation of blocks and definitions.
- **Code Folding**: Fold blocks, actions, and control flow structures.

## Installation

This extension connects to local build artifacts in the `sf-agentscript` monorepo. It is intended for development use.

1. Ensure you have the repository checked out and dependencies installed.

2. Build the required components:
   ```bash
   # Build LSP
   cd packages/agentscript-lsp
   npm install && npm run build

   # Build MCP Server
   cd ../../aslab-mcp
   npm install && npm run build
   ```

3. Open Zed.

4. Open the command palette (`Cmd-Shift-P`) and run `zed: install dev extension`.

5. Select the `sf-agentscript/zed-extension` directory.

## Requirements

- **Node.js**: Required to run the LSP and MCP servers.
- **Rust**: Required if you need to modify and rebuild the extension binary itself.

## Configuration

The extension currently hardcodes paths to the LSP and MCP server builds in `src/lib.rs`. Ensure these paths match your local setup if you are not using the standard directory structure.

## License

MIT