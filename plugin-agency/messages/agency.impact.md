# summary

Scan agent files to find which ones depend on a given Salesforce resource.

# description

Recursively searches a directory for AgentScript (.agent) files and checks each one for dependencies on the specified Salesforce resource. Useful for impact analysis before modifying a Flow, Apex class, SObject, or Prompt Template.

# examples

- Find all agents that use a Flow: `sf agency impact --resource MyFlow`
- Scan a specific directory: `sf agency impact --resource MyFlow --dir force-app`
- Filter by resource type: `sf agency impact --resource MyFlow --type flow`
- Get output as JSON: `sf agency impact --resource MyFlow --format json`

# flags.resource.summary

Salesforce resource name to search for.

# flags.resource.description

The name of the Salesforce resource to check for (e.g., a Flow API name, Apex class name, or SObject name).

# flags.type.summary

Resource type filter (flow, apex, sobject, prompt, or all).

# flags.type.description

Filter the dependency check to a specific resource type. Use 'all' to check all dependency types.

# flags.path.summary

Directory to scan for agent files (default: current directory).

# flags.path.description

The root directory to recursively search for .agent files.

# flags.format.summary

Output format (json or pretty).

# flags.format.description

Specify the output format. Use 'json' for machine-readable output or 'pretty' for a human-readable table.

# error.impactFailure

Failed to run impact analysis: %s
