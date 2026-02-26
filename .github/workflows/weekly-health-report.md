---
description: "Weekly repository health report: issues, PRs, parser gaps, and test status"
labels: ["reporting", "automation"]

on:
  schedule:
    - cron: "0 8 * * 1"  # Every Monday at 08:00 UTC
  workflow_dispatch:

permissions:
  contents: read
  issues: write

tools:
  github:
    toolsets: [issues, pull-requests, repo, actions]
  bash:
    - "cargo"
    - "grep"
    - "wc"
    - "find"

safe-outputs:
  create-issue: {}
---

# Weekly Health Report

You are a repository health reporting agent for `busbar-sf-agentscript`. Every Monday,
generate a concise health report and create a GitHub issue to track it.

## Report sections

### 1. Parser Gap Status
Count:
- Total `#[ignore]` tests in the test suite (run `grep -rc '#\[ignore' crates/parser/tests/`)
- Open issues labeled `new-syntax`
- Parser gaps closed (ignored tests removed) in the last 7 days

### 2. Open Issues Summary
From GitHub Issues API:
- Total open issues by label
- Issues opened in the last 7 days
- Issues closed in the last 7 days
- Oldest open issue (potential stale)

### 3. Pull Request Activity
- PRs merged in the last 7 days (titles and authors)
- PRs open and waiting for review
- Any PRs open > 14 days

### 4. CI Health
- Last 5 CI runs on `main`: pass/fail status
- Any recurring failures (same job failing > 2 times in last week)

### 5. Test Count
Run: `cargo test --workspace --all-features -- --list 2>&1 | grep "test$" | wc -l`
Report: total runnable tests, total ignored tests, ratio

### 6. Agentic Workflow Activity
Summarize activity from other agentic workflows this week:
- Issues created by recipe-gap-tracker
- PRs opened by test-coverage-improver
- Issues created by parser-limitation-tracker

## Output

Create a GitHub issue titled: `[Weekly Health] busbar-sf-agentscript — <YYYY-MM-DD>`

Body: the report in Markdown with the sections above. Keep it scannable — use tables
and bullet points, not prose paragraphs. Close the previous week's health issue if it
is still open.

Label the issue: `report` (skip labeling if the label doesn't exist).
