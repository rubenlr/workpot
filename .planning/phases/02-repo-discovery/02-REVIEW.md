---
phase: 02-repo-discovery
reviewed: 2026-05-29T12:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/workpot-core/src/infra/migrations/002_discovery.sql
  - crates/workpot-core/src/infra/git.rs
  - crates/workpot-core/src/services/discovery.rs
  - crates/workpot-core/src/services/index.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/roots.rs
  - crates/workpot-core/src/services/excludes.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/error.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-cli/src/main.rs
  - crates/workpot-core/tests/discovery_test.rs
  - crates/workpot-core/tests/index_test.rs
  - crates/workpot-core/tests/roots_test.rs
  - crates/workpot-core/tests/excludes_test.rs
  - crates/workpot-core/tests/catalog_test.rs
  - crates/workpot-cli/tests/cli_smoke.rs
findings:
  critical: 0
  warning: 5
  info: 3
  total: 8
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-05-29T12:00:00Z  
**Depth:** standard  
**Files Reviewed:** 18  
**Status:** issues_found

## Summary

Phase 02 delivers discovery (`ignore` walk + excludes), transactional `run_full` with caps and audit tables, roots/excludes CLI, and solid integration coverage (45 workspace tests per summaries). SQL migrations and git subprocess usage are parameterized and shell-safe. No critical security or data-loss defects found in normal paths.

Five warnings remain: root prune fragility on missing paths, soft `max_watch_roots` not enforced at load/scan, config-before-index ordering on `roots add`, orphan scan rows after `roots remove --skip-prune`, and inconsistent canonicalization for bare worktree expansion. Three info items cover unused schema status, deferred `git_common_dir` on manual add, and redundant CLI cap handling.

## Warnings

### WR-01: `roots remove` prune aborts when any scan path is missing on disk

**File:** `crates/workpot-core/src/services/roots.rs:81-85`  
**Issue:** `prune_scan_repos_under_root` calls `repo_path.canonicalize()` for every scan row. If the user deleted repo directories but has not run `workpot index` yet, `canonicalize` fails and the entire `roots remove` returns `InvalidPath` instead of deleting reachable rows.  
**Fix:** Treat canonicalize failure as “not under root” (or delete by stored path key without re-canonicalizing):

```rust
fn repo_under_root(repo_path: &Path, root_canon: &Path) -> Result<bool> {
    let Ok(repo_canon) = repo_path.canonicalize() else {
        return Ok(false);
    };
    Ok(repo_canon.starts_with(root_canon))
}
```

Alternatively delete by `path` prefix using stored canonical strings only (no second canonicalize).

### WR-02: Soft `max_watch_roots` bypass via hand-edited config

**File:** `crates/workpot-core/src/domain/config.rs:46-59`, `crates/workpot-core/src/services/index.rs:35-49`  
**Issue:** `Config::validate` only checks that limit *fields* stay below hard caps (5000/20000). It does not reject `watch_roots.len() > limits.max_watch_roots`. `run_full` scans every entry in `watch_roots`, so a user (or bad merge) can list thousands of roots while `[limits] max_watch_roots = 100`, undermining D-22/D-24 intent. Only `roots add` enforces the soft cap.  
**Fix:** Extend `validate()`:

```rust
if self.watch_roots.len() > self.limits.max_watch_roots as usize {
    return Err(format!(
        "watch_roots count {} exceeds max_watch_roots {}",
        self.watch_roots.len(),
        self.limits.max_watch_roots
    ));
}
```

Reject on `load_config` / `save_config` the same way as hard caps.

### WR-03: `roots add` persists config before index succeeds

**File:** `crates/workpot-core/src/services/roots.rs:24-28`  
**Issue:** `save_config` runs before `index::run_full`. If index fails (cap exceeded, DB error, git walk error), the watch root remains in `config.toml` but repos under it may be unindexed. Caller sees an error with a partially applied config change.  
**Fix:** Run `run_full` first against an in-memory config clone, or wrap config write + index in a compensating rollback (remove root from config on index failure).

### WR-04: Orphan `source=scan` rows after `roots remove --skip-prune`

**File:** `crates/workpot-core/src/services/index.rs:199-221`, `crates/workpot-core/src/services/roots.rs:46-48`  
**Issue:** `collect_stale_scan_paths` only considers scan repos still under a *current* watch root. Repos discovered under a removed root (with `--skip-prune`) are never stale and are not removed by `collect_missing_paths` if directories still exist. They linger until manual `repo remove` or config exclude. D-21 documents `--skip-prune`; operability gap is easy to miss.  
**Fix:** Document in CLI help, or add `index` hygiene: remove scan rows whose path is not under any configured watch root (optional flag).

### WR-05: Bare worktree paths may stay non-canonical in discovery

**File:** `crates/workpot-core/src/infra/git.rs:71-74`, `crates/workpot-core/src/services/discovery.rs:77-79`  
**Issue:** `list_worktree_paths` uses `canonicalize(...).unwrap_or(resolved)`, while `scan_root` pushes fully canonical paths. Mixed canonical keys can break dedup (`seen_paths`), exclude matching, and `path_under_root` checks if the same worktree is reached twice with different string forms.  
**Fix:** Propagate canonicalize errors or skip non-resolvable worktrees with a logged warning; do not insert non-canonical paths into the candidate list.

## Info

### IN-01: `index_runs.status = 'error'` is never written

**File:** `crates/workpot-core/src/infra/migrations/002_discovery.sql:7`, `crates/workpot-core/src/services/index.rs:298-329`  
**Issue:** Schema allows `'error'` but orchestration only records `'ok'` and `'cap_exceeded'`. Failed transactions surface as `WorkpotError` without a matching history row.  
**Fix:** On `run_full` failure after `insert_index_run`, update run to `error` with message in a `catch` path, or drop `'error'` from the CHECK until Phase 3.

### IN-02: Manual `repo add` leaves `git_common_dir` empty until first index

**File:** `crates/workpot-core/src/services/catalog.rs:42-44`, `crates/workpot-core/src/services/index.rs:155-196`  
**Issue:** `register_manual` inserts `git_common_dir = ''`. Acceptable for Phase 2 (backfill on index), but `repo list` / future git-state consumers see incomplete rows if user never runs `workpot index`.  
**Fix:** Resolve `git_common_dir` in `register_manual` when `git` is available (same as scan path), or document that index is required for full metadata.

### IN-03: Duplicate `IndexCapExceeded` handling in CLI

**File:** `crates/workpot-cli/src/main.rs:64-71`, `96-99`  
**Issue:** `Commands::Index` maps cap errors into `anyhow`, then `main` matches `WorkpotError::IndexCapExceeded` again for exit code 1. Behavior is correct; structure is redundant.  
**Fix:** Return cap error directly from `run()` without re-wrapping, or handle exit code only in the `Index` arm.

---

_Reviewed: 2026-05-29T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
