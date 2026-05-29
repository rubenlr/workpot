---
phase: 02-repo-discovery
reviewed: 2026-05-29T22:03:31Z
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
  warning: 2
  info: 1
  total: 3
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-05-29T22:03:31Z  
**Depth:** standard  
**Files Reviewed:** 18  
**Status:** issues_found

## Summary

Fresh adversarial pass on Phase 02 repo discovery after the fix iteration documented in `02-REVIEW-FIX.md`. All seven prior findings (WR-01–WR-05, IN-01–IN-02) were re-verified in source and are **resolved**: manual `max_repos` enforcement, `roots add` rollback with prune, orphan scan cleanup, shared `paths::path_under_root`, `GitUnavailable` on failed `git_common_dir`, and `WorkpotError::Config` for validation failures.

No critical security or data-loss defects in normal happy paths. SQL remains parameterized; git paths are canonicalized before `git2` open. Two warnings remain in edge paths (basename lookup after delete, compensating prune error handling). One info item on exclude-glob construction.

## Prior findings (re-verification)

| ID | Status | Evidence |
|----|--------|----------|
| WR-01 (manual cap) | Fixed | `catalog.rs:38-48` returns `IndexCapExceeded` before insert |
| WR-02 (roots add orphans) | Fixed | `roots.rs:29-32` pops root and calls `prune_scan_repos_under_root` on save failure |
| WR-03 (prune missing paths) | Fixed | `paths::path_under_root` canonicalize-or-prefix fallback; `roots.rs:92-94` |
| WR-04 (orphan scan rows) | Fixed | `index.rs:90-91` `collect_orphan_scan_paths` merged into remove pass |
| WR-05 (path prefix helper) | Fixed | `services/paths.rs` shared by `index.rs` and `roots.rs` |
| IN-01 (empty git_common_dir) | Fixed | `catalog.rs:56` propagates `resolve_git_common_dir` error |
| IN-02 (Config vs LimitsExceeded) | Fixed | `lib.rs:175-177` maps `validate()` to `WorkpotError::Config` |

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Basename repo lookup uses substring `LIKE`, not path suffix

**File:** `crates/workpot-core/src/services/catalog.rs:141-152`  
**Issue:** When the repo directory is gone, `resolve_repo_path_key` falls back to `path LIKE '%' || ?2` with `?2 = "/{basename}"`. SQL `LIKE` treats `%` as “any characters,” so `/tmp/foo-extra` matches suffix `/foo` (the `/foo` substring appears inside `foo-extra`). A single spurious match can make `repo remove` delete the wrong row after the user passes only a directory name.  
**Fix:** Match path suffix only, e.g. `path = ?1 OR path LIKE '%/' || ?2` with `?2` = basename only (no leading slash in pattern), or resolve in Rust: filter `SELECT path` candidates with `Path::new(p).file_name() == Some(name)` and require exactly one match.

### WR-02: Compensating prune errors are discarded on `roots add` rollback

**File:** `crates/workpot-core/src/services/roots.rs:29-32`  
**Issue:** When `save_config` fails after a successful `run_full`, rollback pops the in-memory root and calls `prune_scan_repos_under_root`, but the result is dropped (`let _ = ...`). If prune fails (DB locked, I/O), scan rows from the failed add remain while config omits the watch root—partially undoing WR-02’s fix.  
**Fix:** Propagate prune failure (e.g. `prune_scan_repos_under_root(...)?` after pop) or log at error level and return a compound error so the operator knows config and DB may diverge.

## Info

### IN-01: `remove_repo_with_exclude` embeds raw path segments in globs

**File:** `crates/workpot-core/src/services/catalog.rs:193-194`  
**Issue:** Exclude globs are built as `{parent}/{name}/**` using `display()` / `to_string_lossy()` without escaping glob metacharacters (`*`, `?`, `[`). Unusual directory names can produce invalid globs or broader-than-intended excludes. Low likelihood on typical macOS paths.  
**Fix:** Escape glob-special characters in path segments, or store the canonical path key in a dedicated config field instead of deriving a glob.

---

_Reviewed: 2026-05-29T22:03:31Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
