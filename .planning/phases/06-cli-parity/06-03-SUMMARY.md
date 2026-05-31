---
phase: 06-cli-parity
plan: "03"
subsystem: cli
tags: [rust, cli, list, priority, display]
dependency_graph:
  requires:
    - 06-01
  provides:
    - workpot-list-command
    - list-display-formatter
  affects:
    - crates/workpot-cli/src/main.rs
tech_stack:
  added: []
  patterns:
    - Priority-ordered flat list: Pinned > Dirty > Recent > Rest (mirrors TypeScript sectionSort)
    - Home-dir shortening for parent directory display
    - Emoji prefix per priority section (📌/🟡/🔥/⬜)
key_files:
  created:
    - crates/workpot-cli/src/list_display.rs
  modified:
    - crates/workpot-cli/src/main.rs
    - crates/workpot-cli/tests/cli_smoke.rs
decisions:
  - List command is top-level (workpot list) not under repo subcommand per D-01
  - Emoji icons per D-02 and D-04 (macOS-only v1, all terminals support)
  - Row format: icon parent_dir name branch tags per D-03
  - Ordering algorithm mirrors TypeScript sectionSort exactly (window + minRecentCount padding)
metrics:
  duration_minutes: 25
  completed_date: "2026-05-31"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 2
---

# Phase 06 Plan 03: workpot list Command Summary

Priority-ordered `workpot list` with emoji-prefixed rows matching tray default view: `list_display.rs` formatter module and `Commands::List` top-level variant with cli_smoke coverage.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | list_display formatter | 7811278 | `list_display.rs` (created), `main.rs` (mod added) |
| 2 | workpot list command | bd19038 | `main.rs` (Commands::List + run_list), `cli_smoke.rs` (+2 tests) |

## What Was Built

### list_display.rs

New module providing:

- `PrioritySection` enum: `Pinned`, `Dirty`, `Recent`, `Rest`
- `priority_icon(section) -> &'static str` — 📌/🟡/🔥/⬜
- `shorten_parent_dir(path) -> String` — replaces `$HOME` prefix with `~` for the parent directory
- `format_list_row(repo, icon) -> String` — `icon parent_dir name branch [tags...]`, branch `—` if None
- `flat_tray_ordered_with_icons(repos, config, now_secs) -> Vec<(RepoRecord, &'static str)>` — full priority sort:
  - Pinned (sorted by `pin_order`)
  - Dirty non-pinned (sorted by `last_opened_at` desc)
  - Recent non-pinned non-dirty (within `max_recent_days` window, padded to `min_recent_count`)
  - Rest (alphabetical)
- 11 unit tests covering shorten_parent_dir, format_list_row snapshot, ordering

### Commands::List in main.rs

- Top-level `List` variant in `Commands` enum (not under `Repo`)
- `run_list()` handler: opens AppContext, calls `flat_tray_ordered_with_icons`, prints each row to stdout
- Existing `workpot repo list` unchanged (legacy format preserved)

### cli_smoke.rs

Two new integration tests:
- `list_empty_index_exits_zero`: `workpot list` on empty index exits 0 with no output
- `list_registered_repo_shows_icon_and_name`: registered repo appears with name and ⬜ or 📌 icon

## Verification

- `cargo test -p workpot-cli`: 43 tests pass (19 unit + 24 integration)
- All pre-existing tests preserved (repo list, index, roots, tags, excludes)

## Decisions Made

1. **Ordering mirrors TypeScript exactly** — `flat_tray_ordered_with_icons` implements the same Pinned>Dirty>Recent>Rest algorithm as `src/lib/sort.ts` `sectionSort`, using `max_recent_days` window + `min_recent_count` padding from Config. This satisfies CLI-03 (CLI list must match tray ordering).

2. **`workpot list` is top-level** — Added as `Commands::List` per D-01. The old `workpot repo list` is preserved unchanged so no existing tests break.

3. **`home_dir()` uses `$HOME` env var** — Uses `std::env::var_os("HOME")` which is compatible with the test helper's `cmd.env("HOME", home)` isolation pattern in cli_smoke, ensuring tests don't read the real home directory.

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- list_display.rs exists: FOUND
- main.rs contains `Commands::List`: FOUND
- Commits 7811278 and bd19038: FOUND (verified via git log)
- 43 tests pass: PASSED
