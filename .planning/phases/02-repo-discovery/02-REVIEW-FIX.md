---
phase: 02-repo-discovery
fixed_at: 2026-05-30T18:00:00Z
review_path: .planning/phases/02-repo-discovery/02-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-05-30T18:00:00Z  
**Source review:** `.planning/phases/02-repo-discovery/02-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 7
- Fixed: 7
- Skipped: 0

## Fixed Issues

### WR-01: `register_manual` bypasses `max_repos`

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/src/lib.rs`, `crates/workpot-core/tests/index_test.rs`  
**Commit:** be987be  
**Applied fix:** Thread `&Config` into `register_manual`; count non-excluded repos and return `IndexCapExceeded` before insert when at cap.

### WR-02: `roots add` — index committed but `save_config` failure leaves orphan scan rows

**Files modified:** `crates/workpot-core/src/services/roots.rs`  
**Commit:** 251d879  
**Applied fix:** On `save_config` failure after successful index, pop the in-memory watch root and run `prune_scan_repos_under_root` for the added root path.

### WR-03: `roots remove` prune skips deleted paths under removed root

**Files modified:** `crates/workpot-core/src/services/roots.rs`  
**Commit:** 77b3cb8  
**Applied fix:** `repo_under_root` falls back to lexical `starts_with` when `canonicalize` fails on the repo path.

### WR-04: Index never prunes scan repos outside all configured watch roots

**Files modified:** `crates/workpot-core/src/services/index.rs`  
**Commit:** 138a4cd  
**Applied fix:** Added `collect_orphan_scan_paths` and merged into `run_full` remove pass.

### WR-05: `path_under_root` skips canonicalization unlike `repo_under_root`

**Files modified:** `crates/workpot-core/src/services/paths.rs`, `crates/workpot-core/src/services/mod.rs`, `crates/workpot-core/src/services/index.rs`, `crates/workpot-core/src/services/roots.rs`  
**Commit:** e8cb865  
**Applied fix:** Shared `paths::path_under_root` with canonicalize-or-prefix fallback; used by index stale/orphan checks and roots prune.

### IN-01: Manual register still allows empty `git_common_dir` on git failure

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`  
**Commit:** 02f389b (tests: f529705)  
**Applied fix:** Propagate `resolve_git_common_dir` errors as `GitUnavailable`; catalog tests use real `git init` / worktree fixtures.

### IN-02: Config validation errors always map to `LimitsExceeded`

**Files modified:** `crates/workpot-core/src/lib.rs`, `crates/workpot-core/tests/roots_test.rs`  
**Commit:** 0169781 (tests: f529705)  
**Applied fix:** `load_config` / `save_config` map `validate()` failures to `WorkpotError::Config`.

---

_Fixed: 2026-05-30T18:00:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
