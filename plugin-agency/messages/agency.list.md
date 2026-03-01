# summary

List elements from an AgentScript file

# description

List specific types of elements from an AgentScript file, such as topics, variables, actions, or system messages. This provides a quick overview of the key components in your agent without viewing the full AST.

# examples

- List all topics:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --type topics

- List all variables:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --type variables

- List all actions:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --type actions

- List system messages:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --type messages

- Output as JSON:

  <%= config.bin %> <%= command.id %> --file MyAgent.agent --type topics --format json

# flags.file.summary

Path to the AgentScript file to analyze

# flags.file.description

The path to the AgentScript (.agent) file that you want to list elements from. The file must exist and be readable.

# flags.type.summary

Type of elements to list

# flags.type.description

Specify which type of elements to list from the AgentScript file. Options are: 'topics' (conversation topics), 'variables' (agent variables), 'actions' (defined actions), or 'messages' (system messages).

# flags.format.summary

Output format for the list

# flags.format.description

Choose how to display the list. 'pretty' provides a human-readable formatted list, while 'json' outputs the data in JSON format suitable for programmatic use.

# error.listFailure

Failed to list elements from AgentScript file: %s
