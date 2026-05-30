---
phase: 02-repo-discovery
fixed_at: 2026-05-30T18:00:00Z
review_path: .planning/phases/02-repo-discovery/02-REVIEW.md
iteration: 3
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-05-30T18:00:00Z
**Source review:** `.planning/phases/02-repo-discovery/02-REVIEW.md`
**Iteration:** 3

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: `remove_repo_with_exclude` deletes the row before excludes are persisted

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`
**Commit:** 371dc75
**Applied fix:** Append exclude globs and `save_config` before `remove_repo`; on `remove_repo` failure, remove added globs from config and save again. Regression test `remove_repo_with_exclude_persists_excludes_before_row_removed`.

### WR-02: `roots remove` prunes DB before config save; save failure leaves divergence

**Files modified:** `crates/workpot-core/src/services/roots.rs`
**Commit:** 150987f
**Applied fix:** `save_config` runs before `prune_scan_repos_under_root`; on prune failure, restore the watch root in memory and on disk via `save_config`.

### WR-03: `run_full` returns `Err` after merge commit when git refresh fails

**Files modified:** `crates/workpot-core/src/services/index.rs`
**Commit:** 7670817
**Applied fix:** After successful index merge, `git_tx.commit` failure logs a warning, folds unpersisted refresh into `git_errors`, and returns `Ok(summary)` so CLI exits 0 with partial-success counts.

### IN-01: `roots add` crash window drops indexed repos on next index

**Files modified:** `crates/workpot-core/src/services/roots.rs`, `crates/workpot-core/tests/roots_test.rs`
**Commit:** 83d7497
**Applied fix:** `save_config` before `run_full`; on scan failure, pop root, save config, and `prune_scan_repos_under_root`. Regression test `roots_add_persists_watch_root_on_disk`.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-05-30T18:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
