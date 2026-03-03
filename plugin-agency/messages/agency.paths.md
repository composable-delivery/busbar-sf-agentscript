# summary

Enumerate all execution paths through an AgentScript agent.

# description

Performs a depth-first traversal of the agent's topic graph to enumerate all possible execution paths from start_agent through topics. Detects cycles, unreachable topics, and shows whether transitions are regular (→) or delegate (⇒) style.

# examples

- Show all execution paths: `sf agency paths -f agent.agent`
- Get paths as JSON: `sf agency paths -f agent.agent --format json`
- Limit traversal depth: `sf agency paths -f agent.agent --max-depth 5`

# flags.file.summary

Path to the AgentScript file to analyze.

# flags.file.description

The AgentScript (.agent) file to enumerate paths through.

# flags.format.summary

Output format (json or pretty).

# flags.format.description

Specify the output format. Use 'json' for machine-readable output or 'pretty' for human-readable path listing.

# flags.max-depth.summary

Maximum path depth (default: 20).

# flags.max-depth.description

Limit the maximum number of hops in a single path to avoid very deep traversals.

# error.pathsFailure

Failed to enumerate paths: %s
