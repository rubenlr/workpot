---
phase: 02-repo-discovery
reviewed: 2026-05-30T12:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/workpot-cli/src/main.rs
  - crates/workpot-cli/tests/cli_smoke.rs
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/error.rs
  - crates/workpot-core/src/infra/git.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/discovery.rs
  - crates/workpot-core/src/services/excludes.rs
  - crates/workpot-core/src/services/index.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/services/roots.rs
  - crates/workpot-core/tests/catalog_test.rs
  - crates/workpot-core/tests/discovery_test.rs
  - crates/workpot-core/tests/excludes_test.rs
  - crates/workpot-core/tests/index_test.rs
  - crates/workpot-core/tests/roots_test.rs
  - crates/workpot-core/src/infra/migrations/002_discovery.sql
findings:
  critical: 0
  warning: 3
  info: 1
  total: 4
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-05-30T12:00:00Z  
**Depth:** standard  
**Files Reviewed:** 18  
**Status:** issues_found

## Summary

Third-pass review after the fix iteration in `02-REVIEW-FIX.md` (commits `3c43068`, `9886f29`, `431ea90`, `46fcb26`, `e00aa38`). All five findings from the 2026-05-29 review are **fixed in source** — orphan/stale purge respects configured roots, path prefix uses `strip_prefix`, `roots remove` prunes before save, `remove_repo_with_exclude` deletes the row before writing excludes, and `roots add` prunes on `run_full` failure.

Four **new** issues remain, all around non-atomic config/DB ordering or split transactions: config-save failures after destructive DB steps, and `run_full` reporting failure after a successful merge commit. Discovery, caps, cap-exceeded audit rows, parameterized SQL, and glob escaping look sound. No hardcoded secrets or injection surfaces in scope.

## Prior findings (re-verification from 02-REVIEW-FIX)

| ID | Status | Evidence |
|----|--------|----------|
| CR-01 (orphan purge when watch root canonicalize fails) | **Fixed** | `index.rs:297-317` — `collect_orphan_scan_paths` uses `configured_roots`; `collect_stale_scan_paths` skips when root not in `scan_roots` (`283-288`); unit tests `collect_orphan_scan_paths_honors_configured_roots_without_canonicalization`, `collect_stale_scan_paths_skips_repos_when_configured_root_not_scanned` |
| WR-01 (lexical string prefix in `path_under_root`) | **Fixed** | `paths.rs:16-18` — `path_starts_with_root` uses `strip_prefix`; tests `lexical_prefix_does_not_match_sibling_directory_name` |
| WR-02 (`roots remove` saved config before prune) | **Fixed** | `roots.rs:58-66` — prune before `save_config`; prune failure restores in-memory root |
| WR-03 (excludes saved before `remove_repo`) | **Fixed** | `catalog.rs:210-223` — `remove_repo` then `save_config`; regression `remove_repo_with_exclude_does_not_persist_excludes_when_remove_fails` |
| WR-04 (`roots add` no DB compensation on `run_full` error) | **Fixed** | `roots.rs:35-38` — `prune_scan_repos_under_root` on `run_full` `Err` |

| Prior audit item | Status | Evidence |
|------------------|--------|----------|
| Cap-exceeded audit row (D-18) | **OK** | `index.rs:36-40`, `392-405`; regression `index_cap_abort` asserts `cap_exceeded` run |

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: `remove_repo_with_exclude` deletes the row before excludes are persisted

**File:** `crates/workpot-core/src/services/catalog.rs:210-223`  
**Issue:** `remove_repo` runs first; `save_config` runs second. If `save_config` fails (disk full, permissions), the SQLite row is gone but exclude globs were never written. The next `workpot index` can rediscover the path under a watch root, violating D-10 (“rescan will not re-add”).  
**Fix:** Persist excludes first in a temp config, or delete the row only after a successful `save_config`; on save failure, re-insert the repo row (or wrap both in a single logical rollback).

### WR-02: `roots remove` prunes DB before config save; save failure leaves divergence

**File:** `crates/workpot-core/src/services/roots.rs:58-73`  
**Issue:** On successful prune, `save_config` can still fail. Rollback restores the in-memory watch root, but scan rows are already deleted while `config.toml` on disk still lists the root. A later `workpot index` re-adds repos the user believed were removed with the root.  
**Fix:** Save config first when `skip_prune`, or defer prune until after successful save (with compensating prune on save failure); mirror the `add_root` rollback pattern for the DB side when save fails after prune.

### WR-03: `run_full` returns `Err` after merge commit when git refresh fails

**File:** `crates/workpot-core/src/services/index.rs:147-201`  
**Issue:** The merge transaction commits and `index_runs` is finalized with status `ok` before the git-refresh transaction. If `git_tx.commit()` fails, the function returns `Err` and the CLI exits non-zero, even though adds/removes/skips were already applied. Users may treat the run as a full failure and retry unnecessarily.  
**Fix:** Return `Ok(summary)` with elevated `git_errors` when merge succeeded but refresh failed; or record a distinct run status / message and document partial success in CLI output.

## Info

### IN-01: `roots add` crash window drops indexed repos on next index

**File:** `crates/workpot-core/src/services/roots.rs:25-34`  
**Issue:** The watch root is only written to disk after a successful `run_full` + `save_config`. Process kill between those steps leaves scan rows in SQLite without the root in `config.toml`. The next `workpot index` runs `collect_orphan_scan_paths` and deletes those rows. Rare, but silent data loss from the user’s perspective.  
**Fix:** Save config before scan (with rollback on scan failure), or treat “repos under paths pending save” as protected until the root is persisted.

---

_Reviewed: 2026-05-30T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
