# summary

Open an interactive terminal UI to explore AgentScript files.

# description

Launches a full-screen interactive TUI for browsing and analyzing AgentScript files.
Navigate between files with arrow keys, switch between Graph, Validate, Deps, and Topics tabs.

Keybindings: ↑↓ navigate | Tab switch tab | f focus files | r reload | q quit

# examples

- Open TUI in current directory:
  <%= config.bin %> <%= command.id %>
- Open a specific file:
  <%= config.bin %> <%= command.id %> --file path/to/agent.agent
- Scan a specific directory:
  <%= config.bin %> <%= command.id %> --path ./force-app/agents

# flags.file.summary

Path to a specific .agent file to open.

# flags.file.description

Open directly to this file. If omitted, scans for .agent files in the directory.

# flags.path.summary

Directory to scan for agent files (default: current directory).

# flags.path.description

Recursively searches this directory for .agent files when --file is not specified.
