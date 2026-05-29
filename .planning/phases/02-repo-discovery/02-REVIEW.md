---
phase: 02-repo-discovery
reviewed: 2026-05-29T22:11:24Z
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
  critical: 1
  warning: 4
  info: 0
  total: 5
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-05-29T22:11:24Z  
**Depth:** standard  
**Files Reviewed:** 18  
**Status:** issues_found

## Summary

Second-pass adversarial review after the fix iteration in `02-REVIEW-FIX.md` (commits `03c3933`, `f2c6cf0`, `24d356e`). The three findings from the 2026-05-29 review are **fixed in source** (basename suffix lookup, prune error propagation on `roots add` rollback, glob escaping on exclude paths).

Five **new** issues remain: one critical data-loss path when a watch root cannot be canonicalized during `run_full`, plus four warnings around non-atomic config/DB ordering and a lexical path-prefix fallback. SQL stays parameterized; git paths are canonicalized before `git2` open. No new hardcoded secrets or injection surfaces in scope.

## Prior findings (re-verification from 2026-05-29)

| ID | Status | Evidence |
|----|--------|----------|
| WR-01 (basename `LIKE` substring) | **Fixed** | `catalog.rs:147-174` — `%/{basename}` + `escape_like` + `file_name()` filter; regression `remove_repo_by_basename_does_not_match_similar_directory_name` |
| WR-02 (prune errors discarded on `roots add` rollback) | **Fixed** | `roots.rs:31` — `prune_scan_repos_under_root(...)?` after config pop |
| IN-01 (raw path segments in exclude globs) | **Fixed** | `catalog.rs:226-257` — `escape_glob_literal` / `path_to_exclude_glob_prefix`; regression `remove_repo_with_exclude_escapes_glob_metacharacters_in_path` |

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Skipped watch roots trigger orphan purge of all repos under that root

**File:** `crates/workpot-core/src/services/index.rs:205-217`, `286-305`  
**Issue:** `canonical_watch_roots` drops any watch root whose `canonicalize()` fails (logs a warning and continues). `collect_orphan_scan_paths` then deletes every `source=scan` row that is not under **any** successfully canonicalized root. A transient failure (permission blip, unmounted volume, EMFILE) on `~/code` removes every indexed repo previously discovered under `~/code` on the next `workpot index` or `roots add` scan—without the user removing the root from config.  
**Fix:** Treat failed canonicalization as “root still configured, skip orphan/stale logic for repos that lexically belong to that root,” or abort the run with an error instead of silently shrinking the active root set. Minimal guard:

```rust
// In collect_orphan_scan_paths: only orphan if path is not under ANY config.watch_roots entry
for root in &config.watch_roots {
    if paths::path_under_root(path, root) {
        return false; // still covered by a configured root
    }
}
```

Prefer using the same `path_under_root` helper against **configured** roots (with explicit error if a configured root cannot be canonicalized) rather than only the canonicalized subset.

## Warnings

### WR-01: Lexical `str::starts_with` in `path_under_root` when canonicalize fails

**File:** `crates/workpot-core/src/services/paths.rs:10-12`  
**Issue:** When `path.canonicalize()` fails, the fallback uses `path.starts_with(&root_canon)` on `Path`, which degrades to **byte/string** prefix matching. `/tmp/foo-bar/repo` is treated as under `/tmp/foo`, so `roots remove` prune or stale/orphan logic can target the wrong rows when the repo directory is missing but a sibling path shares a string prefix (verified: `Path::starts_with` is false for this pair, but `to_string_lossy().starts_with` is true).  
**Fix:** After `starts_with`, require a boundary: `path == root || path.starts_with(root.join(""))` or compare parent components; never use raw string prefix without a following `/` or end-of-path.

### WR-02: `roots remove` persists config before prune succeeds

**File:** `crates/workpot-core/src/services/roots.rs:53-58`  
**Issue:** `save_config` runs before `prune_scan_repos_under_root`. If prune fails (DB locked, I/O), the watch root is already gone from `config.toml` while scan rows under that root remain—config/DB divergence opposite to the fixed `roots add` rollback path.  
**Fix:** Prune first (or run both in one logical unit): on prune failure, do not save config; or save config only after successful prune unless `skip_prune` is set.

### WR-03: `remove_repo_with_exclude` writes excludes before deleting the repo row

**File:** `crates/workpot-core/src/services/catalog.rs:210-223`  
**Issue:** `save_config` appends exclude globs, then `remove_repo` runs. If `remove_repo` fails (ambiguous basename, DB error), config contains exclude globs for a repo that is still indexed—re-scan may skip the path while the row remains.  
**Fix:** Delete the row inside the same logical operation first, then append excludes and save config; or roll back config on `remove_repo` failure.

### WR-04: `roots add` does not compensate DB when `run_full` fails after the index transaction commits

**File:** `crates/workpot-core/src/services/roots.rs:26-38`, `crates/workpot-core/src/services/index.rs:153-200`  
**Issue:** `run_full` commits the merge transaction before the git-refresh transaction. If `git_tx.commit()` (or any post-commit step) returns `Err`, `add_root` pops the in-memory watch root and returns without saving config—but scan rows from the new root are already in SQLite while `config.toml` never gained the root. Orphan cleanup on a later index heals this, but the failure mode is silent partial apply.  
**Fix:** On `run_full` error after a successful merge, call `prune_scan_repos_under_root` for the added root (same as config-save rollback), or make `run_full` a single atomic unit including git refresh.

---

_Reviewed: 2026-05-29T22:11:24Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
