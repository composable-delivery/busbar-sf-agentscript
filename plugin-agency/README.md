# @muselab/sf-plugin-busbar-agency

A Salesforce CLI plugin for parsing, validating, and analyzing AgentScript files. Powered by a Rust/WebAssembly parser for fast, portable execution.

## Installation

```bash
sf plugins install @muselab/sf-plugin-busbar-agency
```

Requires Salesforce CLI (`sf`) v2+.

---

## Commands

All commands support `--file` (target a single `.agent` file) or `--path` (scan a directory recursively, default: `.`). You can also save a persistent selection with `sf agency agents select` and subsequent commands will use it automatically.

When running against multiple files, JSON output includes a `file` field on each result so you can identify which file each result came from.

### Agent Management

#### `sf agency agents list`

List all `.agent` files in a directory along with their parsed names and selection status.

```bash
sf agency agents list --path ./agents
```

#### `sf agency agents select`

Interactively select which agent files subsequent commands should target.

```bash
# Interactive checkbox (TTY only)
sf agency agents select --path ./agents

# Select all agents
sf agency agents select --path ./agents --all

# Clear selection (revert to directory scan)
sf agency agents select --none
```

Selection is saved to the plugin's data directory and used automatically by all other commands until cleared.

---

### Parsing & Inspection

#### `sf agency parse`

Parse an AgentScript file and display its AST structure.

```bash
sf agency parse --file MyAgent.agent
sf agency parse --file MyAgent.agent --format json
sf agency parse --path ./agents
```

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `-o, --format` | `pretty` (default) or `json` |

---

#### `sf agency list`

List specific elements from an AgentScript file.

```bash
sf agency list --file MyAgent.agent --type topics
sf agency list --path ./agents --type actions --format json
```

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `-t, --type` | `topics`, `variables`, `actions`, or `messages` (required) |
| `-o, --format` | `pretty` (default) or `json` |

---

#### `sf agency query <path>`

Query topics, variables, actions, or raw AST elements using a path-based syntax.

```bash
# Semantic queries — returns structured data
sf agency query /topics/fraud_review --file MyAgent.agent
sf agency query /variables/accountId --file MyAgent.agent
sf agency query /actions/checkCredit --file MyAgent.agent

# Raw AST traversal — dot-notation
sf agency query config.agent_name --file MyAgent.agent --format json

# Across all agents in a directory
sf agency query /topics/fraud_review --path ./agents
```

**Semantic path formats:**

| Path | Returns |
|------|---------|
| `/topics/<name>` | Incoming references and outgoing transitions |
| `/variables/<name>` | All readers and writers |
| `/actions/<name>` | Action definition and reasoning steps that invoke it |
| `dot.notation.path` | Raw AST value at that path |

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `-o, --format` | `pretty` (default) or `json` |

---

### Visualization

#### `sf agency graph`

Visualize the topic flow graph.

```bash
sf agency graph --file MyAgent.agent
sf agency graph --file MyAgent.agent --format mermaid
sf agency graph --file MyAgent.agent --format html > graph.html
sf agency graph --path ./agents --stats
```

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `-v, --view` | `topics` (default), `actions`, or `full` |
| `--format` | `ascii` (default), `graphml`, `mermaid`, or `html` |
| `--stats` | Print topic/variable/action counts |

---

#### `sf agency paths`

Enumerate all execution paths through the agent's topic graph, detecting cycles.

```bash
sf agency paths --file MyAgent.agent
sf agency paths --path ./agents --format json
```

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `--format` | `pretty` (default) or `json` |
| `--max-depth` | Maximum path depth (default: 20) |

---

### Dependency Analysis

#### `sf agency deps`

Extract Salesforce org dependencies: SObjects, Flows, Apex classes, Knowledge bases, Connections, Prompt Templates, and External Services.

```bash
# Per-file dependency table
sf agency deps --file MyAgent.agent

# See which agents share each dependency
sf agency deps --path ./agents --group dependency

# Filter and format
sf agency deps --file MyAgent.agent --type flows --format json

# Retrieve dependent metadata from an org
sf agency deps --file MyAgent.agent --retrieve --target-org myOrg
```

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `-o, --format` | `table` (default), `json`, or `summary` |
| `-t, --type` | `all`, `sobjects`, `flows`, `apex`, `knowledge`, or `connections` |
| `--group` | `file` (default) or `dependency` — group by file or by dependency |
| `--retrieve` | Retrieve dependent metadata from the target org |
| `--target-org` | Org alias/username for `--retrieve` |

**`--group dependency` output:**

```
Flows (2)
  ▸ CreditCheckFlow
      • agents/billing.agent
      • agents/collections.agent
  ▸ OnboardingFlow
      • agents/onboarding.agent
```

---

#### `sf agency impact`

Scan a directory for agents that depend on a specific Salesforce resource.

```bash
sf agency impact --resource MyFlow__c --type flow --path ./agents
sf agency impact --resource Account --type sobject --path ./agents
```

| Flag | Description |
|------|-------------|
| `--path` | Directory to scan (default: `.`) |
| `--resource` | Resource name to search for (required) |
| `--type` | `flow`, `apex`, `sobject`, `knowledge`, or `any` (required) |

---

#### `sf agency actions`

Extract action interface definitions in multiple formats, useful for generating TypeScript types or API documentation.

```bash
sf agency actions --file MyAgent.agent
sf agency actions --file MyAgent.agent --format typescript
sf agency actions --file MyAgent.agent --format markdown
```

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to a single `.agent` file |
| `--path` | Directory to scan (default: `.`) |
| `-o, --format` | `table` (default), `json`, `typescript`, or `markdown` |
| `-t, --target` | `all`, `flow`, `apex`, or `prompt` |

---

### Validation

#### `sf agency validate`

Validate an AgentScript file for syntax and semantic errors.

```bash
sf agency validate --file MyAgent.agent
sf agency validate --path ./agents
```

Exit code is `1` if any errors are found.

---

#### `sf agency validate platform`

Validate an AgentScript file against the Salesforce platform (requires org connection).

```bash
sf agency validate platform --file MyAgent.agent --target-org myOrg
```

---

#### `sf agency version`

Display the AgentScript parser version.

```bash
sf agency version
```

---

## JSON Output

All commands accept oclif's `--json` flag for machine-readable output. In multi-file mode, each item in the returned array includes a `file` field:

```json
[
  {
    "file": "agents/billing.agent",
    "report": { ... },
    "summary": { ... }
  },
  {
    "file": "agents/collections.agent",
    "report": { ... },
    "summary": { ... }
  }
]
```

---

## License

MIT
