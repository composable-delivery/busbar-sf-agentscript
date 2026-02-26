---
description: "Weekly: run cargo bench, track performance trends, surface regressions vs prior week"
labels: ["perf", "report"]

on:
  schedule:
    - cron: "weekly on monday"
  workflow_dispatch:

permissions:
  contents: read
  issues: read

tools:
  github:
    toolsets: [repos, issues]
  bash:
    - "cargo"
    - "git"
    - "grep"
    - "cat"
    - "awk"
    - "sed"
    - "date"
    - "find"

safe-outputs:
  create-issue: {}
  add-comment: {}
---

# Weekly Benchmark Tracker

You are a performance tracking agent for `busbar-sf-agentscript`. Every Monday you run
`cargo bench`, record the results, and compare them against last week's run to detect
any slow drift that wouldn't trigger the PR regression checker.

## Steps

### 1. Initialize the repository

```bash
git submodule update --init --recursive
```

### 2. Check for a prior baseline

Look for Criterion's saved baseline from last week:

```bash
find target/criterion -name "estimates.json" 2>/dev/null | head -5
```

If baseline files exist from a `weekly` saved baseline, we can compare.

### 3. Run benchmarks with baseline comparison (if prior baseline exists)

If a `weekly` baseline exists:

```bash
cargo bench --bench parse_recipes -- --baseline weekly 2>&1
```

If no baseline exists yet (first run), save one and skip the comparison:

```bash
cargo bench --bench parse_recipes -- --save-baseline weekly 2>&1
```

After the comparison run, save the new baseline for next week:

```bash
cargo bench --bench parse_recipes -- --save-baseline weekly 2>&1 | tail -100
```

### 4. Collect timing data

Parse the Criterion output for all benchmark groups:
- `parse_all_recipes` — total throughput parsing all recipes
- `parse_individual` — per-recipe timings
- `parse_by_size` — small/medium/large file categories
- `comprehensive_demo` — full-feature file timings

For each benchmark, extract: benchmark name, median time (middle of the three-value
range), and throughput (MB/s) if shown.

### 5. Detect significant weekly drift

Flag any benchmark where the median time increased by more than **10%** compared to
last week's baseline. This threshold is higher than the PR checker (5%) to avoid
false positives from CI environment variability across weeks.

### 6. Search for existing open performance issues

Search GitHub issues with label `perf-regression` that are still open. If one already
exists for a particular benchmark, add a comment rather than opening a duplicate.

### 7. Report

**If regressions ≥10% are found:**

Open a new issue (or comment on an existing one) titled:
`perf: Weekly benchmark regression detected — YYYY-MM-DD`

Body:
- Date of this run
- Table of regressed benchmarks: name, last week median, this week median, % change
- Suggested investigation steps (check recent commits, profile with `cargo flamegraph`)
- Link to the bench file: `crates/parser/benches/parse_recipes.rs`

**If no regressions are found:**

Search for an existing open issue titled `Weekly Benchmark Health` (or similar).
If found, add a brief comment: "Week of YYYY-MM-DD: all benchmarks within 10% of
baseline. Fastest: <name> at <time>."

If no such issue exists, create one titled `📈 Weekly Benchmark Health Log` and
post the first entry. This issue becomes a running log of weekly performance.

**If benchmarks could not run** (submodule missing, compile error, etc.):

Post a comment on the `Weekly Benchmark Health Log` issue (create it if needed)
noting the failure and what prevented the run. Do not open a regression issue.

### 8. Summary format for the health log

```
### Week of YYYY-MM-DD

| Benchmark | Median | Throughput | vs Last Week |
|-----------|--------|------------|--------------|
| parse_all_recipes/parse | 1.23 ms | 45.6 MB/s | -1.2% ✅ |
| comprehensive_demo/parse | 456 µs | 12.3 MB/s | +2.1% ✅ |
| ... | | | |

Recipe count: N files, total X KB
```
