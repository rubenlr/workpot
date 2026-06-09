---
phase: "06"
plan: "01"
subsystem: workpot-core
tags: [ordering, repo-priority, cli-parity, sort, tray]
dependency_graph:
  requires:
    - crates/workpot-core/src/domain/repo.rs  # RepoRecord
    - crates/workpot-core/src/domain/config.rs  # Config.max_recent_days, min_recent_count
  provides:
    - crates/workpot-core/src/services/repo_priority.rs  # section_sort, flat_tray_ordered_repos
  affects:
    - crates/workpot-core/src/lib.rs  # re-exports SectionedRepos + priority functions
tech_stack:
  added: []
  patterns:
    - Pure Rust port of TypeScript sort.ts four-tier ordering (no external deps)
    - HashSet-based dedup for Recent/Rest partition
key_files:
  created:
    - crates/workpot-core/src/services/repo_priority.rs
    - crates/workpot-core/tests/repo_priority_test.rs
  modified:
    - crates/workpot-core/src/services/mod.rs
    - crates/workpot-core/src/lib.rs
decisions:
  - "section_sort uses HashSet<String> (path-based) for dedup; avoids PartialEq derive on RepoRecord"
  - "cmp_last_opened_desc mirrors byLastOpenedDesc: higher timestamp first, null last, name tie-break"
  - "pin_order None treated as 999 (matches sort.ts pin_order ?? 999)"
metrics:
  duration: "4m"
  completed: "2026-05-31T14:50:26Z"
  tasks_completed: 2
  files_changed: 4
---

# Phase 6 Plan 01: repo_priority Module Summary

**One-liner:** Rust four-tier repo ordering (Pinned > Dirty > Recent > Rest) porting `sectionSort + flatSectioned` from TypeScript `sort.ts` with 11 golden-vector tests.

## What Was Built

Added `crates/workpot-core/src/services/repo_priority.rs` with:

- `SectionedRepos { pinned, dirty, recent, rest }` — mirrors `SectionedRepos` interface from `sort.ts`
- `section_sort(repos, config, now_seconds) -> SectionedRepos` — exact port of `sectionSort`
- `flat_tray_ordered(sectioned) -> Vec<RepoRecord>` — mirrors `flatSectioned` from `trayList.ts`
- `flat_tray_ordered_repos(repos, config, now_seconds) -> Vec<RepoRecord>` — convenience wrapper

Decision rules honored:
- **D-19:** Four-tier partition: Pinned → Dirty → Recent → Rest
- **D-20:** Dirty wins over recent — a dirty+recently-opened repo lands in Dirty, not Recent
- **D-21:** `last_opened_at IS NULL` repos go to Rest; they cannot pad Recent
- **D-22:** Recent is padded to `min_recent_count` from outside-window repos with `last_opened_at IS NOT NULL`

All functions exported from `lib.rs` public API for use by `workpot-cli`.

## Test Coverage

`crates/workpot-core/tests/repo_priority_test.rs` — 11 tests, 0 ignored:

| Test | Coverage |
|------|----------|
| `pinned_repos_land_only_in_pinned_section` | Pinned isolation |
| `dirty_repo_lands_in_dirty_not_recent` | D-20 dirty beats recent |
| `recent_padded_to_min_recent_count_from_outside_window_d22` | D-22 padding floor |
| `never_opened_repos_land_in_rest_not_recent_d21` | D-21 null → rest |
| `padding_never_uses_never_opened_repos_d21` | D-21 no null padding |
| `every_repo_appears_exactly_once` | Partition completeness |
| `pinned_sorted_by_pin_order_ascending` | pin_order sort |
| `pinned_none_pin_order_treated_as_999` | None → 999 fallback |
| `rest_sorted_alphabetically` | Rest alphabetical |
| `flat_output_follows_pinned_dirty_recent_rest_order` | Flat concat order |
| `dirty_beats_recent_in_flat_output_d20` | D-20 in flat context |

All 11 pass; `cargo clippy -p workpot-core -- -D warnings` clean.

## Verification

```
cargo test -p workpot-core --test repo_priority_test
running 11 tests
test result: ok. 11 passed; 0 failed; 0 ignored
```

Note: the plan's verify command `cargo test -p workpot-core repo_priority` uses "repo_priority" as a test-name filter which matches 0 function names (consistent with other test files like `org_test.rs`). The correct invocation is `--test repo_priority_test`. Both exit 0.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None — `repo_priority` is pure in-memory sort with no I/O or trust boundaries (T-06-01-01: accept).

## Self-Check: PASSED

- [x] `crates/workpot-core/src/services/repo_priority.rs` — exists
- [x] `crates/workpot-core/tests/repo_priority_test.rs` — exists
- [x] Task 1 commit `81e81ae` — exists
- [x] Task 2 commit `69c4388` — exists
