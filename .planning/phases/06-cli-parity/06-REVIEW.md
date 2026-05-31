---
phase: 06-cli-parity
reviewed: 2026-05-31T12:00:00Z
depth: deep
files_reviewed: 12
files_reviewed_list:
  - crates/workpot-cli/src/list_display.rs
  - crates/workpot-cli/src/main.rs
  - crates/workpot-cli/tests/cli_smoke.rs
  - crates/workpot-core/Cargo.toml
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/launch.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/services/repo_fuzzy.rs
  - crates/workpot-core/src/services/repo_priority.rs
  - crates/workpot-core/tests/repo_fuzzy_test.rs
  - crates/workpot-core/tests/repo_priority_test.rs
  - src-tauri/src/launch.rs
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: issues_found
---

# Phase 06: Code Review Report (re-review after fix)

**Reviewed:** 2026-05-31 (post `--fix --auto` iteration 1)  
**Depth:** deep  
**Files Reviewed:** 12  
**Status:** issues_found (info only)

## Summary

Critical and warning findings from the initial review are resolved. CLI list/search ordering delegates to `repo_priority::section_sort`; fuzzy DoS guard uses scalar count; launch spawns are reaped; `workpot open` launch failures exit via `LaunchFailed` in `main` (code 2).

Three info-level items remain — optional cleanup, not blocking parity.

---

## Resolved (iteration 1)

| ID | Resolution |
|----|------------|
| CR-01 | `fuzzy_score` uses `q.chars().count()` |
| CR-02 | `flat_tray_ordered_with_icons` calls `section_sort` |
| WR-01 | Unified via CR-02 (`pin_order` sentinel 999 in core, matches TS) |
| WR-02 | `resolve_launch_program` simplified |
| WR-03 | Background `child.wait()` after spawn |
| WR-04 | `LaunchFailed` error type; exit 2 in `main` |

---

## Info (remaining)

### IN-01: Duplicate tag validation in CLI vs core

**File:** `crates/workpot-cli/src/main.rs` — `validate_tag_for_add`

CLI pre-validates tags before `ctx.add_tag`; core `org::normalize_tag` duplicates rules. Low risk of drift.

**Fix:** Remove `validate_tag_for_add`; map `WorkpotError::InvalidInput` in the error pipeline.

---

### IN-02: Path key match could use `OsStr` on POSIX

**File:** `crates/workpot-cli/src/main.rs` — `match_repo_path_key`

Now uses `path.to_str()` for comparison (improved). Non-UTF-8 paths still won't match string identifiers.

**Fix:** Optional `as_os_str()` compare for POSIX-only paths.

---

### IN-03: Core re-exports now used by CLI

**File:** `list_display.rs` → `repo_priority::section_sort`

Resolved by CR-02. `flat_tray_ordered*` re-exports remain for tray/tests.

---

_Reviewer: gsd-code-review --fix --auto (iteration 1 re-review)_  
_Depth: deep_
