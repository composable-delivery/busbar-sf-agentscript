# summary

Extract Salesforce org dependencies from an AgentScript file

# description

Analyze an AgentScript file to identify all Salesforce org dependencies including SObjects, Flows, Apex classes, Knowledge bases, Connections, and more. This is useful for deployment planning and impact analysis.

# examples

- Extract all dependencies from an AgentScript file:

  <%= config.bin %> <%= command.id %> --file path/to/agent.agent

- Get dependencies as JSON:

  <%= config.bin %> <%= command.id %> --file agent.agent --format json

- Get only Flow dependencies:

  <%= config.bin %> <%= command.id %> --file agent.agent --type flows

- Get a summary of dependency counts:

  <%= config.bin %> <%= command.id %> --file agent.agent --format summary

- Group by dependency across all agents (see which agents share each dep):

  <%= config.bin %> <%= command.id %> --path ./agents --group dependency

- Group by dependency as JSON:

  <%= config.bin %> <%= command.id %> --path ./agents --group dependency --format json

# flags.file.summary

Path to the AgentScript file to analyze

# flags.file.description

The path to the AgentScript (.agent) file that you want to analyze for dependencies. The file must exist and be readable.

# flags.format.summary

Output format (json, table, or summary)

# flags.format.description

Specify the output format. Use 'json' for machine-readable output, 'table' for human-readable lists, or 'summary' for a count overview.

# flags.type.summary

Filter by dependency type

# flags.type.description

Filter the output to show only specific types of dependencies: 'all' (default), 'sobjects', 'flows', 'apex', 'knowledge', or 'connections'.

# flags.group.summary

Group output by file or by dependency

# flags.group.description

Control how results are organized. 'file' (default) shows dependencies per agent file. 'dependency' inverts the view — shows each unique dependency and which agent files use it.

# flags.retrieve.summary

Retrieve dependent metadata from the target org.

# flags.retrieve.description

After extracting dependencies, run sf project retrieve start to pull all dependent metadata from the org. Requires --target-org.

# flags.target-org.summary

Target org alias or username for retrieval.

# flags.target-org.description

The org alias or username to retrieve metadata from. Required when using --retrieve.

# error.retrieveRequiresOrg

The --retrieve flag requires --target-org to be specified.

# error.extractionFailure

Failed to extract dependencies: %s
