# summary

Select agent files to use as the default target for all commands.

# description

Interactively select which .agent files commands should target by default (when --file is not specified). Use --all to select all found agents, or --none to clear the selection. The selection is saved to disk and used by all other agency commands as the default source of files.

# examples

- Select agents interactively: `sf agency agents select`
- Select all agents in current directory: `sf agency agents select --all`
- Select all agents in a specific path: `sf agency agents select --all --path force-app`
- Clear selection: `sf agency agents select --none`

# flags.path.summary

Directory to scan for agent files (default: current directory).

# flags.path.description

Recursively searches this directory for .agent files when selecting agents.

# flags.all.summary

Select all found agent files without prompting.

# flags.all.description

Selects all .agent files found under --path without showing an interactive prompt.

# flags.none.summary

Clear the current selection without prompting.

# flags.none.description

Removes all agents from the selection. Subsequent commands will fall back to scanning the directory.

# error.requireAllOrNone

Non-interactive mode detected. Use --all to select all agents or --none to clear the selection.

# error.noAgentsFound

No .agent files found in %s.

# error.selectFailure

Failed to save agent selection: %s
