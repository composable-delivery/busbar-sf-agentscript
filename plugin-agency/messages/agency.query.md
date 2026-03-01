# summary

Query specific parts of an AgentScript AST

# description

Extract and display specific parts of an AgentScript file's Abstract Syntax Tree using a path-based query syntax. This allows you to inspect configuration values, topic details, variable definitions, and other AST elements without viewing the entire tree.

# examples

- Query the agent name:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --path config.agent_name

- Query all topics:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --path topics

- Query a specific topic's description:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --path topics.main.description

- Query with JSON output:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --path system.messages --format json

- Query variables:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --path variables

# flags.file.summary

Path to the AgentScript file to query

# flags.file.description

The path to the AgentScript (.agent) file that you want to query. The file must exist and be readable.

# flags.path.summary

Dot-notation path to the AST element to query

# flags.path.description

A dot-separated path to the element you want to extract from the AST. For example: 'config.agent_name' or 'topics.main.description'. Use array indices for arrays: 'topics.0.name'.

# flags.format.summary

Output format for the query result

# flags.format.description

Choose how to display the query result. 'pretty' provides a human-readable summary, while 'json' outputs the raw data in JSON format suitable for programmatic use.

# error.queryFailure

Failed to query AgentScript AST: %s
