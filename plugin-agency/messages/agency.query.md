# summary

Query specific parts of an AgentScript file

# description

Query topics, variables, actions, or raw AST elements in an AgentScript file using a path-based syntax.

Use semantic paths to inspect well-known constructs:
- `/topics/<name>` — incoming references and outgoing transitions for a topic
- `/variables/<name>` — readers and writers for a variable
- `/actions/<name>` — action definition and the reasoning steps that invoke it

Use dot-notation paths for raw AST access:
- `config.agent_name` — read a specific AST property
- `topics.0.name` — index into arrays

# examples

- Query a topic's incoming and outgoing connections:

  <%= config.bin %> <%= command.id %> /topics/fraud_review --file MyAgent.agent

- Query variable readers and writers:

  <%= config.bin %> <%= command.id %> /variables/accountId --file MyAgent.agent

- Query who invokes an action:

  <%= config.bin %> <%= command.id %> /actions/checkCredit --file MyAgent.agent

- Query a topic across all agents in the repo:

  <%= config.bin %> <%= command.id %> /topics/fraud_review --path ./agents

- Query raw AST with JSON output:

  <%= config.bin %> <%= command.id %> config.agent_name --file MyAgent.agent --format json

# flags.file.summary

Path to the AgentScript file to query

# flags.file.description

The path to the AgentScript (.agent) file that you want to query. The file must exist and be readable.

# flags.format.summary

Output format (pretty or json)

# flags.format.description

Choose how to display the query result. 'pretty' provides a human-readable summary, while 'json' outputs the raw data in JSON format suitable for programmatic use.

# error.queryFailure

Failed to query AgentScript: %s
