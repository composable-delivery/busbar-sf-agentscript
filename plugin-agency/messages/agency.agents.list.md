# summary

List all AgentScript files found in a directory.

# description

Recursively scans a directory for .agent files and displays a table showing their relative path, agent name (parsed from the config block), and whether they are in the current selection.

# examples

- List agents in current directory: `sf agency agents list`
- List agents in a specific path: `sf agency agents list --path force-app`

# flags.path.summary

Directory to scan for agent files (default: current directory).

# flags.path.description

Recursively searches this directory for .agent files. Defaults to the current working directory.

# error.listFailure

Failed to list agent files: %s
