---
phase: 02-repo-discovery
fixed_at: 2026-05-29T19:20:00Z
review_path: .planning/phases/02-repo-discovery/02-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-05-29T19:20:00Z  
**Source review:** `.planning/phases/02-repo-discovery/02-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 8
- Fixed: 8
- Skipped: 0

## Fixed Issues

### WR-01: `roots remove` prune aborts when any scan path is missing on disk

**Files modified:** `crates/workpot-core/src/services/roots.rs`  
**Commit:** `2b9a7c5`  
**Applied fix:** `repo_under_root` returns `Ok(false)` when `canonicalize` fails instead of propagating `InvalidPath`.

### WR-02: Soft `max_watch_roots` bypass via hand-edited config

**Files modified:** `crates/workpot-core/src/domain/config.rs`  
**Commit:** `79e3cc4`  
**Applied fix:** `Config::validate()` rejects `watch_roots.len() > limits.max_watch_roots` on load/save.

### WR-03: `roots add` persists config before index succeeds

**Files modified:** `crates/workpot-core/src/services/roots.rs`  
**Commit:** `2b9a7c5`  
**Applied fix:** Run `index::run_full` before `save_config`; pop the in-memory root on index failure. (Same commit as WR-01 — both `roots.rs` hunks landed atomically.)

### WR-04: Orphan `source=scan` rows after `roots remove --skip-prune`

**Files modified:** `crates/workpot-cli/src/main.rs`  
**Commit:** `328c248`  
**Applied fix:** Expanded `--skip-prune` help to document orphan scan rows until `workpot index` or `repo remove`.

### WR-05: Bare worktree paths may stay non-canonical in discovery

**Files modified:** `crates/workpot-core/src/infra/git.rs`  
**Commit:** `06faabd`  
**Applied fix:** Skip worktrees that fail `canonicalize` with a stderr warning instead of inserting non-canonical paths.

### IN-01: `index_runs.status = 'error'` is never written

**Files modified:** `crates/workpot-core/src/services/index.rs`  
**Commit:** `1322ceb`  
**Applied fix:** `run_full` wrapper records an `error` index run (except `IndexCapExceeded`, which keeps its dedicated row).

### IN-02: Manual `repo add` leaves `git_common_dir` empty until first index

**Files modified:** `crates/workpot-core/src/services/catalog.rs`  
**Commit:** `1995519`  
**Applied fix:** `register_manual` resolves `git_common_dir` via `resolve_git_common_dir` when git is available.

### IN-03: Duplicate `IndexCapExceeded` handling in CLI

**Files modified:** `crates/workpot-cli/src/main.rs`, `crates/workpot-core/src/error.rs`  
**Commit:** `328c248`, `e009fc9`  
**Applied fix:** `Commands::Index` uses `?` propagation; cap exit code handled only in `main`. `IndexCapExceeded` display message aligned with prior CLI wording.

---

_Fixed: 2026-05-29T19:20:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
