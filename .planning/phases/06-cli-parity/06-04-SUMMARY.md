---
phase: 06-cli-parity
plan: "04"
subsystem: cli
tags: [search, fuzzy, cli, integration-test]
dependency_graph:
  requires: [06-01, 06-02, 06-03]
  provides: [workpot-search-command, search-smoke-tests]
  affects: [crates/workpot-cli/src/main.rs, crates/workpot-cli/tests/cli_smoke.rs]
tech_stack:
  added: []
  patterns: [fuzzy-filter-before-priority-sort, reuse-list_display-helpers]
key_files:
  created: []
  modified:
    - crates/workpot-cli/src/main.rs
    - crates/workpot-cli/tests/cli_smoke.rs
decisions:
  - "Use fuzzy_match trim gate before retain loop — empty query skips the filter entirely rather than relying on fuzzy_match score=1 path, keeping code intent explicit"
  - "named_git_fixture helper added to cli_smoke.rs to create repos with specific names (alpha, beta, myrepo) rather than the default 'sample-repo' from git_fixture"
metrics:
  duration: "~10 minutes"
  completed: "2026-05-31"
  tasks_completed: 2
  files_modified: 2
---

# Phase 6 Plan 4: workpot search command — Summary

`workpot search <query>` fuzzy-filters the repo index and prints results in Pinned > Dirty > Recent > Rest order using the same row format as `workpot list`.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | workpot search command | 5d8ea54 | crates/workpot-cli/src/main.rs |
| 2 | cli_smoke search tests | e633a9f | crates/workpot-cli/tests/cli_smoke.rs |

## What Was Built

### Task 1: workpot search command (5d8ea54)

Added `Commands::Search { query: String }` to the CLI and a `run_search` handler in `crates/workpot-cli/src/main.rs`:

- Imports `workpot_core::services::repo_fuzzy::fuzzy_match`
- Handler: `ctx.list_repos()` → `repos.retain(|r| fuzzy_match(trimmed, r))` (skipped for empty query) → `flat_tray_ordered_with_icons(repos, config, now)` → `format_list_row` per row
- Empty/whitespace query retains all repos — output is identical to `workpot list` for same index
- No `#tag` parsing (D-07); documented in command doc comment
- Exit 0 regardless of match count; no matches → silent empty stdout (grep-friendly, D-05)

### Task 2: cli_smoke search tests (e633a9f)

Added two integration tests to `crates/workpot-cli/tests/cli_smoke.rs`:

- `search_filters_by_fuzzy_query`: registers repos `alpha` and `beta`; `workpot search alpha` stdout contains "alpha" and not "beta"
- `search_empty_query_equals_list`: `workpot search ""` stdout byte-for-byte equals `workpot list` stdout for the same index
- `named_git_fixture` helper: creates a git repo at `parent/name` (named, vs `git_fixture`'s hardcoded `sample-repo`)

## Verification

- `cargo test -p workpot-cli` — 30/30 tests pass (19 unit + 11 integration → up from 28)
- `cargo test -p workpot-core fuzzy_golden` — SC#2: 2/2 golden vector tests pass
- `cargo build -p workpot-cli` — clean compile, no warnings

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — `workpot search` is a read-only query path (no writes, no network, no auth surface). Input is the query string passed through `fuzzy_match` which applies a 256-char cap (T-06-02-01).

## Self-Check: PASSED

- `crates/workpot-cli/src/main.rs` — FOUND (modified, contains `Search` variant and `run_search`)
- `crates/workpot-cli/tests/cli_smoke.rs` — FOUND (modified, contains `search_filters_by_fuzzy_query`)
- Commit `5d8ea54` — FOUND (feat(06-04): add workpot search command)
- Commit `e633a9f` — FOUND (test(06-04): add cli_smoke search integration tests)
- All 30 workpot-cli tests pass
- SC#2 fuzzy_golden tests pass
