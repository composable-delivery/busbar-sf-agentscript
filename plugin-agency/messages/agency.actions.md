# summary

Extract action interface definitions from an AgentScript file.

# description

Parses an AgentScript file and extracts all action definitions with their complete interfaces, including inputs, outputs, and target information. This is useful for understanding the Salesforce org dependencies and generating type definitions.

# examples

- Extract actions in table format: `sf agency actions -f agent.agent`
- Output as JSON: `sf agency actions -f agent.agent --format json`
- Generate TypeScript interfaces: `sf agency actions -f agent.agent --format typescript`
- Generate Markdown documentation: `sf agency actions -f agent.agent --format markdown`
- Filter to only flows: `sf agency actions -f agent.agent --target flow`

# flags.file.summary

Path to the AgentScript file to analyze.

# flags.file.description

The AgentScript (.agent) file to extract action interfaces from.

# flags.format.summary

Output format (json, table, typescript, markdown).

# flags.format.description

Controls how the action interfaces are displayed. Use 'json' for machine-readable output, 'table' for human-readable summary, 'typescript' to generate TypeScript interface definitions, or 'markdown' for documentation.

# flags.target.summary

Filter by target type (all, flow, apex, prompt).

# flags.target.description

Only show actions with the specified target type. Use 'flow' for Salesforce Flows, 'apex' for Apex classes, 'prompt' for Prompt Templates, or 'all' to show everything.

# error.extractionFailure

Failed to extract action interfaces: %s
