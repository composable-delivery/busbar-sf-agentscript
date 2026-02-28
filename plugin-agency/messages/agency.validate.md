# summary

Validate an AgentScript file

# description

Validate an AgentScript file to check if it has valid syntax according to the AgentScript language specification. This command uses the WASM-based parser to perform syntax validation without executing the script.

# examples

- Validate an AgentScript file:

  <%= config.bin %> <%= command.id %> --file path/to/agent.agent

# flags.file.summary

Path to the AgentScript file to validate

# flags.file.description

The path to the AgentScript (.agent) file that you want to validate. The file must exist and be readable.
