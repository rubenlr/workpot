---
phase: 01-core-persistence
fixed_at: 2026-05-30T18:00:00Z
review_path: .planning/phases/01-core-persistence/01-REVIEW.md
iteration: 1
fix_scope: all
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-05-30T18:00:00Z
**Source review:** `.planning/phases/01-core-persistence/01-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 7
- Fixed: 7
- Skipped: 0

## Fixed Issues

### WR-01: Gitdir-file worktrees accept any `gitdir:` prefix without validating target

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`
**Commit:** 80ca54b
**Applied fix:** Resolve gitdir path (absolute or relative) and require `HEAD` before accepting gitfile worktrees. Added regression test for invalid gitdir targets.

### WR-02: `repo remove` fails when the directory no longer exists

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`
**Commit:** 9a6e559
**Applied fix:** Added `resolve_repo_location` / `repo_path_key` helpers with `NotFound` fallback; applied to `remove_repo` and `remove_repo_with_exclude`. Added test removing after directory deletion.

### WR-03: Non-atomic config writes risk corruption on crash

**Files modified:** `crates/workpot-core/src/lib.rs`
**Commit:** 17d4081
**Applied fix:** Introduced `write_atomic` (temp file + fsync + rename) used by `ensure_default_config` and `save_config`.

### WR-04: D-12 offline enforcement not on macOS CI matrix

**Files modified:** `.github/workflows/ci.yml`
**Commit:** f15ff77
**Applied fix:** macOS matrix job now runs `cargo fetch`, `check-no-network-deps.sh`, and `cargo test --offline --workspace --all-targets`.

### WR-05: `remove_repo_with_exclude` commits DELETE before config save

**Files modified:** `crates/workpot-core/src/services/catalog.rs`
**Commit:** 0f12db3
**Applied fix:** Persist exclude globs via cloned config before DELETE; only mutate in-memory config after successful save; delegate row deletion to `remove_repo`.

### IN-01: `catalog::remove_repo` is dead code

**Files modified:** `crates/workpot-core/src/services/catalog.rs`
**Commit:** 0f12db3
**Applied fix:** `remove_repo_with_exclude` now calls `remove_repo` for the DELETE half (consolidated with WR-05).

### IN-02: `register_manual` silently stores empty `git_common_dir` when git2 open fails

**Files modified:** `crates/workpot-core/src/services/catalog.rs`
**Commit:** a5633b8
**Applied fix:** Replaced silent `unwrap_or_default` with `log::warn!` when `resolve_git_common_dir` fails; still registers with empty string for filesystem-valid minimal fixtures.

---

_Fixed: 2026-05-30T18:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
