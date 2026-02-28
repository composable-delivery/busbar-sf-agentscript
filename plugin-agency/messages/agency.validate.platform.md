# summary

Validate an AgentScript file against the Salesforce platform compiler.

# description

Sends an AgentScript file to the Salesforce platform for validation using the same Python-based compiler that runs during deployment. This catches issues that local WASM validation cannot detect, such as platform-specific constraints and runtime compatibility.

The command automatically:
- Creates a temporary DX project structure around the agent file
- Injects `default_agent_user` if missing (using the target org's username)
- Runs `sf agent validate authoring-bundle` against the target org
- Combines platform results with local WASM validation
- Cleans up temporary files

Requires a Salesforce org with Agentforce enabled (Einstein1AIPlatform feature).

# examples

- Validate against a scratch org:

  <%= config.bin %> <%= command.id %> --file path/to/agent.agent --target-org my-scratch-org

- Validate with JSON output:

  <%= config.bin %> <%= command.id %> --file agent.agent --target-org my-org --json

- Skip local WASM validation:

  <%= config.bin %> <%= command.id %> --file agent.agent --target-org my-org --skip-local

# flags.file.summary

Path to the AgentScript file to validate.

# flags.file.description

The path to the AgentScript (.agent) file that you want to validate against the Salesforce platform.

# flags.target-org.summary

Salesforce org to validate against.

# flags.target-org.description

The Salesforce org to use for platform validation. Must have Agentforce features enabled (Einstein1AIPlatform).

# flags.skip-local.summary

Skip local WASM validation.

# flags.skip-local.description

By default, both local WASM validation and platform validation are run. Use this flag to skip the local validation and only run platform validation.

# error.noAgentName

Could not extract agent_name from the agent file. Ensure the config block has an agent_name field.

# error.platformValidation

Platform validation command failed: %s

# error.tempProject

Failed to create temporary project structure: %s
