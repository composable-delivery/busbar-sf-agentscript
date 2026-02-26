---
description: "Weekly scan of #[ignore] tests to ensure every parser limitation has a tracking issue"
labels: ["parser-gaps", "automation"]

on:
  schedule:
    - cron: "0 10 * * 2"  # Every Tuesday at 10:00 UTC
  workflow_dispatch:

permissions:
  contents: read
  issues: write

tools:
  github:
    toolsets: [issues, repo]
  bash:
    - "grep"
    - "cat"
    - "find"

safe-outputs:
  create-issue: {}
  add-comment: {}
---

# Parser Limitation Tracker

You are a tracking agent for `busbar-sf-agentscript`. Your job is to ensure every
`#[ignore]` test in the codebase has a corresponding GitHub issue so that parser gaps
are visible and prioritized.

## Steps

### 1. Collect all ignored tests

Run:
```
grep -n '#\[ignore' crates/parser/tests/integration_test.rs crates/parser/tests/test_reasoning_minimal.rs
```

For each ignored test, extract:
- The test function name
- The ignore reason string (e.g., `"Known parser limitation: ... does not support ..."`)
- The `.agent` snippet being tested (the `include_str!` path or inline content)

### 2. Check for existing tracking issues

Search GitHub Issues for each ignore reason. An issue "covers" an ignored test if:
- The issue title contains `[Parser Gap]` and the syntax keyword from the ignore message, OR
- The issue body mentions the test function name

### 3. Create missing tracking issues

For each ignored test that has NO corresponding issue, create one titled:
`[Parser Gap] <ignore reason summary>`

Issue body:
- Test file and function name
- The ignore reason verbatim
- The AgentScript syntax construct that fails
- Relevant source location in `crates/parser/src/parser/` where the fix would go
  (look at `expressions.rs` for expression-level gaps, `actions.rs` for action gaps, etc.)
- Label: `new-syntax`, `good-first-issue` (if the fix seems self-contained)

### 4. Report summary

After scanning, post a comment (or create a pinned discussion if none exists) summarizing:
- Total ignored tests: N
- Tests with tracking issues: N
- Tests newly given issues: N
- Any ignored tests that look like they might actually pass now (the ignore reason
  references a feature that has since been implemented in the parser)
