---
phase: 02-repo-discovery
fixed_at: 2026-05-30T00:00:00Z
review_path: .planning/phases/02-repo-discovery/02-REVIEW.md
iteration: 2
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-05-30T00:00:00Z
**Source review:** `.planning/phases/02-repo-discovery/02-REVIEW.md`
**Iteration:** 2

**Summary:**
- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### CR-01: Skipped watch roots trigger orphan purge of all repos under that root

**Files modified:** `crates/workpot-core/src/services/index.rs`
**Commit:** 3c43068
**Applied fix:** Orphan, stale, and manual-outside-root cleanup now checks `config.watch_roots` (configured paths) via `paths::path_under_root`, while stale removal only applies when the repo is under a successfully scanned canonical root. Added unit tests for orphan/stale preservation when canonical roots are empty.

### WR-01: Lexical `str::starts_with` in `path_under_root` when canonicalize fails

**Files modified:** `crates/workpot-core/src/services/paths.rs`
**Commit:** 9886f29
**Applied fix:** Replaced `Path::starts_with` fallback with `strip_prefix`-based `path_starts_with_root` for explicit component-boundary matching. Added unit tests for sibling vs child paths.

### WR-02: `roots remove` persists config before prune succeeds

**Files modified:** `crates/workpot-core/src/services/roots.rs`
**Commit:** 431ea90
**Applied fix:** `remove_root` prunes scan repos before `save_config` and restores the in-memory watch root on prune or save failure.

### WR-03: `remove_repo_with_exclude` writes excludes before deleting the repo row

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`
**Commit:** 46fcb26
**Applied fix:** `remove_repo` runs before exclude globs are appended and saved. Added regression test ensuring ambiguous basename removal leaves excludes unchanged.

### WR-04: `roots add` does not compensate DB when `run_full` fails after the index transaction commits

**Files modified:** `crates/workpot-core/src/services/roots.rs`
**Commit:** e00aa38
**Applied fix:** On `run_full` error, `add_root` pops the in-memory watch root and calls `prune_scan_repos_under_root` for the added root, matching the config-save rollback path.

---

_Fixed: 2026-05-30T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
