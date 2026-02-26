---
description: "Update agent-script-recipes submodule, run tests, and track new parser gaps as issues"
labels: ["parser-gaps", "automation"]

on:
  schedule:
    - cron: "0 9 * * 1"  # Every Monday at 09:00 UTC
  workflow_dispatch:

permissions:
  contents: write
  issues: write
  pull-requests: write

tools:
  github:
    toolsets: [issues, pull-requests, repo]
  bash:
    - "git"
    - "cargo"
    - "grep"
    - "cat"
    - "find"
    - "diff"

safe-outputs:
  create-pull-request: {}
  create-issue: {}
  add-comment: {}
---

# Recipe Gap Tracker

You are a parser gap tracking agent for `busbar-sf-agentscript`. Your job is to keep the
`agent-script-recipes` submodule current with upstream and surface new parser failures as
actionable issues.

`agent-script-recipes` (https://github.com/trailheadapps/agent-script-recipes) is
Salesforce's official repository of real-world `.agent` files. When Salesforce adds new
AgentScript syntax, it appears here first. This workflow ensures we track those gaps
proactively.

## Steps

### 1. Check for submodule updates

Run:
```
git submodule update --remote agent-script-recipes
git -C agent-script-recipes log --oneline HEAD@{1}..HEAD
```

If there are no new commits in `agent-script-recipes`, post a short summary comment on
any open `[Recipe Gap]` issues noting that upstream has not changed, then exit.

### 2. Run the test suite against the updated submodule

Run:
```
cargo test --workspace --all-features 2>&1
```

Capture the full output. Note which tests:
- **Now pass** (were previously failing or ignored)  
- **Still fail** or are ignored  
- **Newly fail** (tests that were passing before the submodule update)

### 3. Find newly failing recipe files

For any test that newly fails, identify:
- The `.agent` recipe file path that caused the failure
- The specific syntax construct that the parser rejects (look for "ParseError" or "unexpected token")
- Whether there is already an `#[ignore]` test in `crates/parser/tests/integration_test.rs`
  for this construct

### 4. Create issues for new parser gaps

For each **new** failure (not already tracked), create a GitHub issue titled:
`[Recipe Gap] Parser does not support: <syntax description>`

Issue body should include:
- The failing `.agent` snippet (relevant lines only)
- The parser error message
- Which recipe file(s) exhibit this syntax
- A code pointer: which parser module in `crates/parser/src/parser/` would need to be
  updated (check `expressions.rs`, `actions.rs`, `reasoning.rs`, `topics.rs` as appropriate)
- Label: `new-syntax`

Do NOT duplicate: before creating an issue, search existing issues for `[Recipe Gap]`
with similar syntax descriptions.

### 5. Open a PR to bump the submodule

If there are new commits in `agent-script-recipes` (whether or not there are new failures),
create a PR that:
- Bumps `agent-script-recipes` to the latest commit
- PR title: `chore: bump agent-script-recipes to <short-sha>`
- PR body: lists new recipe files added, new gaps found (linked to issues), and any
  previously-failing tests that now pass

### 6. Close resolved gap issues

For any `[Recipe Gap]` issue whose corresponding test now passes (was un-ignored),
add a comment noting the fix and suggest closing.
