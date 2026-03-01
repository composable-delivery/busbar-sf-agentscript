# Usage Examples for sf-plugin-busbar-agency

This document provides practical examples of using the AgentScript CLI plugin commands.

## Prerequisites

1. Build the WASM package (from repository root):
   ```bash
   cargo build --lib --release --target wasm32-unknown-unknown --features wasm
   ```

2. Install and link the plugin:
   ```bash
   cd plugin-agency
   npm install
   npm run build
   sf plugins link .
   ```

## Command Examples

### `sf agency version`

Display the version of the AgentScript parser.

```bash
$ sf agency version
AgentScript Parser Version: 0.1.0
```

**Use Case**: Verify which parser version you're using, helpful for debugging or reporting issues.

---

### `sf agency validate`

Validate the syntax of an AgentScript file.

#### ✅ Valid File

```bash
$ sf agency validate --file examples/HelloWorld.agent
✓ examples/HelloWorld.agent is valid AgentScript
```

#### ❌ Invalid File

```bash
$ sf agency validate --file examples/broken.agent
✗ Validation failed: found System at 175..181 expected end of input
```

**Use Cases**:
- Pre-commit hooks to ensure all AgentScript files are valid
- CI/CD pipelines to catch syntax errors early
- Quick syntax check during development
- Validate files before deployment to Salesforce

---

### `sf agency parse`

Parse an AgentScript file and display its structure.

#### Pretty Format (Default)

```bash
$ sf agency parse --file examples/HelloWorld.agent

Successfully parsed: examples/HelloWorld.agent

Configuration:
  Agent Name: HelloWorld
  Agent Label: HelloWorld
  Description: A minimal agent that greets users and engages using poems

System:
  Messages: 2 defined
  Instructions: defined

Topics:
  - greeting

✓ Parse successful
```

**Use Cases**:
- Quick overview of an agent's structure
- Verify configuration values
- Check topic names and counts
- Understand agent structure at a glance

#### JSON Format

```bash
$ sf agency parse --file examples/HelloWorld.agent --format json
```

Output (abbreviated):
```json
{
  "config": {
    "node": {
      "agent_name": {
        "node": "HelloWorld",
        "span": { "start": 356, "end": 368 }
      },
      "agent_label": {
        "node": "HelloWorld",
        "span": { "start": 389, "end": 401 }
      },
      "description": {
        "node": "A minimal agent that greets users...",
        "span": { "start": 420, "end": 480 }
      }
    }
  },
  "system": {
    "node": {
      "messages": {
        "welcome": {
          "node": "Hello! I'm a simple agent here to say hi.",
          "span": { "start": 603, "end": 649 }
        }
      }
    }
  },
  "topics": {
    "greeting": { ... }
  }
}
```

**Use Cases**:
- Programmatic analysis of AgentScript files
- Extract specific configuration values for automation
- Generate documentation from AgentScript files
- Build tools that process or transform AgentScript
- Integration with other systems via JSON

---

## Advanced Workflows

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Validate all AgentScript files before commit

AGENT_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '\.agent$')

if [ -z "$AGENT_FILES" ]; then
  exit 0
fi

echo "Validating AgentScript files..."
for file in $AGENT_FILES; do
  if ! sf agency validate --file "$file"; then
    echo "❌ Validation failed for $file"
    exit 1
  fi
done

echo "✅ All AgentScript files are valid"
exit 0
```

### CI/CD Pipeline (GitHub Actions)

```yaml
name: Validate AgentScript Files

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Install Salesforce CLI
        run: npm install -g @salesforce/cli
      
      - name: Install AgentScript Plugin
        run: |
          sf plugins install sf-plugin-busbar-agency
      
      - name: Validate AgentScript Files
        run: |
          find . -name "*.agent" -exec sf agency validate --file {} \;
```

### Extract Configuration Values

Use `jq` with JSON output:

```bash
# Get all agent names
find . -name "*.agent" -exec sh -c \
  'sf agency parse --file "$1" --format json | jq -r ".config.node.agent_name.node"' \
  _ {} \;

# Get topic names for a specific agent
sf agency parse --file MyAgent.agent --format json | \
  jq -r '.topics | keys[]'

# Get all variable names and types
sf agency parse --file MyAgent.agent --format json | \
  jq -r '.variables | to_entries[] | "\(.key): \(.value.node.var_type.node)"'
```

### Batch Validation

```bash
#!/bin/bash
# Validate all AgentScript files in a directory

TOTAL=0
VALID=0
INVALID=0

for file in $(find . -name "*.agent"); do
  TOTAL=$((TOTAL + 1))
  if sf agency validate --file "$file" > /dev/null 2>&1; then
    VALID=$((VALID + 1))
    echo "✅ $file"
  else
    INVALID=$((INVALID + 1))
    echo "❌ $file"
  fi
done

echo ""
echo "Summary:"
echo "  Total:   $TOTAL"
echo "  Valid:   $VALID"
echo "  Invalid: $INVALID"
```

### Documentation Generation

```bash
#!/bin/bash
# Generate markdown documentation from AgentScript files

echo "# AgentScript Catalog"
echo ""

for file in $(find . -name "*.agent"); do
  echo "## $(basename $file .agent)"
  echo ""
  
  # Parse and extract key information
  JSON=$(sf agency parse --file "$file" --format json)
  
  NAME=$(echo "$JSON" | jq -r '.config.node.agent_name.node')
  DESC=$(echo "$JSON" | jq -r '.config.node.description.node')
  TOPICS=$(echo "$JSON" | jq -r '.topics | keys[]' | wc -l)
  
  echo "- **Name**: $NAME"
  echo "- **Description**: $DESC"
  echo "- **Topics**: $TOPICS"
  echo ""
done
```

---

## Tips and Best Practices

### Performance

- **Validation is fast**: The WASM parser can validate hundreds of files per second
- **Batch operations**: Use shell loops for batch processing
- **JSON parsing**: Pipe JSON output directly to `jq` for efficient processing

### Error Handling

- **Exit codes**: Commands return non-zero exit codes on failure, perfect for scripts
- **Error messages**: Parse errors include position information for debugging
- **Validation vs Parse**: Use `validate` for simple checks, `parse` when you need the AST

### Integration

- **CI/CD**: Add validation to your CI pipeline to catch errors early
- **Git hooks**: Use pre-commit hooks to prevent invalid files from being committed
- **Build tools**: Integrate with build tools using JSON output format
- **Documentation**: Generate docs automatically using the parse command

### Debugging

If you encounter issues:

1. Verify the file exists and is readable
2. Check the file has valid AgentScript syntax
3. Ensure you're using the latest parser version: `sf agency version`
4. Try parsing with JSON format to see the full AST structure
5. Compare with working examples from agent-script-recipes

---

## Getting Help

- View command help: `sf agency parse --help`
- Report issues: https://github.com/composable-delivery/busbar-sf-agentscript/issues
- Examples: See `agent-script-recipes/` in the repository
- Documentation: See [PLUGIN.md](PLUGIN.md) and [README.md](README.md)

---

### `sf agency list`

List specific elements from an AgentScript file.

#### List Topics

```bash
$ sf agency list --file examples/CustomerService.agent --type topics

Topics (3):
  • start_agent: topic_selector
  • support: Handle customer support requests
  • billing: Process billing inquiries
```

#### List Variables

```bash
$ sf agency list --file examples/CustomerService.agent --type variables

Variables (2):
  • customer_name: mutable string
  • issue_resolved: mutable boolean
```

#### List Actions

```bash
$ sf agency list --file examples/CustomerService.agent --type actions

Actions (3):
  • start_agent.go_to_support
  • support.reasoning.resolve_issue
  • support.actions.resolve_issue
```

#### List Messages

```bash
$ sf agency list --file examples/CustomerService.agent --type messages

Messages (2):
  • welcome: Hello! How can I help you today?
  • error: I apologize, something went wrong.
```

**Use Cases**:
- Quick inventory of agent components
- Generate documentation about agent structure
- Validate that expected elements exist
- Compare agents to find differences

---

### `sf agency query`

Query and extract specific parts of the AST using dot-notation paths.

#### Query Configuration

```bash
$ sf agency query --file examples/MyAgent.agent --path config.agent_name

Query: config.agent_name

Result: "MyAgent"
```

#### Query System Instructions

```bash
$ sf agency query --file examples/MyAgent.agent --path system.instructions

Query: system.instructions

Result: "You are a helpful customer service agent."
```

#### Query Topic Description

```bash
$ sf agency query --file examples/MyAgent.agent --path topics.support.description

Query: topics.support.description

Result: "Handle customer support requests"
```

#### Query with JSON Output

```bash
$ sf agency query --file examples/MyAgent.agent --path topics --format json
{
  "support": {
    "node": {
      "description": { "node": "Handle support requests" },
      "reasoning": { ... }
    }
  }
}
```

**Use Cases**:
- Extract specific configuration values for scripts
- Validate expected values exist
- Generate reports or documentation
- Compare values across multiple agents
- CI/CD automation and validation

---

## Advanced AST Workflows

### Extract All Agent Names from Multiple Files

```bash
#!/bin/bash
# Extract agent names from all .agent files

for file in force-app/**/*.agent; do
  name=$(sf agency query --file "$file" --path config.agent_name --format json | jq -r '.data')
  echo "$file: $name"
done
```

### Generate Agent Inventory

```bash
#!/bin/bash
# Create a markdown inventory of all agents

echo "# Agent Inventory" > AGENTS.md
echo "" >> AGENTS.md

for file in $(find . -name "*.agent"); do
  name=$(sf agency query --file "$file" --path config.agent_name 2>/dev/null)
  topics=$(sf agency list --file "$file" --type topics --format json 2>/dev/null | jq -r '.items | length')
  vars=$(sf agency list --file "$file" --type variables --format json 2>/dev/null | jq -r '.items | length')
  
  echo "## $name" >> AGENTS.md
  echo "- File: \`$file\`" >> AGENTS.md
  echo "- Topics: $topics" >> AGENTS.md
  echo "- Variables: $vars" >> AGENTS.md
  echo "" >> AGENTS.md
done
```

### Validate Agent Structure

```bash
#!/bin/bash
# Ensure all agents have required elements

for file in $(find . -name "*.agent"); do
  # Check for required config
  if ! sf agency query --file "$file" --path config.agent_name &>/dev/null; then
    echo "❌ $file: Missing agent_name"
  fi
  
  # Check for system instructions
  if ! sf agency query --file "$file" --path system.instructions &>/dev/null; then
    echo "⚠️ $file: Missing system instructions"
  fi
  
  # Check for at least one topic
  topic_count=$(sf agency list --file "$file" --type topics --format json | jq -r '.items | length')
  if [ "$topic_count" -eq 0 ]; then
    echo "❌ $file: No topics defined"
  else
    echo "✅ $file: Valid structure"
  fi
done
```

### Compare Agent Configurations

```bash
#!/bin/bash
# Compare configurations of two agents

FILE1=$1
FILE2=$2

echo "Comparing $FILE1 vs $FILE2"
echo ""

echo "Agent Names:"
echo "  File 1: $(sf agency query --file "$FILE1" --path config.agent_name)"
echo "  File 2: $(sf agency query --file "$FILE2" --path config.agent_name)"

echo ""
echo "Topic Counts:"
echo "  File 1: $(sf agency list --file "$FILE1" --type topics --format json | jq -r '.items | length')"
echo "  File 2: $(sf agency list --file "$FILE2" --type topics --format json | jq -r '.items | length')"
```

