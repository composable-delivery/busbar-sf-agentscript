# summary

Parse an AgentScript file and output its AST

# description

Parse an AgentScript file using the WASM-based parser and output the Abstract Syntax Tree (AST). The output can be in JSON format for programmatic use or in a pretty-printed format for human readability.

# examples

- Parse a file and display pretty output:

  <%= config.bin %> <%= command.id %> --file path/to/agent.agent

- Parse a file and output JSON:

  <%= config.bin %> <%= command.id %> --file path/to/agent.agent --format json

# flags.file.summary

Path to the AgentScript file to parse

# flags.file.description

The path to the AgentScript (.agent) file that you want to parse. The file must exist and be readable.

# flags.format.summary

Output format for the parsed AST

# flags.format.description

Choose how to display the parsed Abstract Syntax Tree. 'pretty' provides a human-readable summary, while 'json' outputs the full AST in JSON format suitable for programmatic use.

# error.parseFailure

Failed to parse AgentScript file: %s
