---
phase: 02-repo-discovery
fixed_at: 2026-05-30T18:45:00Z
review_path: .planning/phases/02-repo-discovery/02-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-05-30T18:45:00Z  
**Source review:** `.planning/phases/02-repo-discovery/02-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 3
- Fixed: 3
- Skipped: 0

## Fixed Issues

### WR-01: Basename repo lookup uses substring `LIKE`, not path suffix

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`  
**Commit:** `03c3933`  
**Applied fix:** Replaced `LIKE '%' || '/basename'` with suffix pattern `%/{basename}` plus Rust `file_name()` filter; added `remove_repo_by_basename_does_not_match_similar_directory_name` regression test (`foo` vs `foo-extra`).

### WR-02: Compensating prune errors are discarded on `roots add` rollback

**Files modified:** `crates/workpot-core/src/services/roots.rs`  
**Commit:** `f2c6cf0`  
**Applied fix:** Propagate `prune_scan_repos_under_root` with `?` during rollback so config save failures do not leave orphan scan rows silently.

### IN-01: `remove_repo_with_exclude` embeds raw path segments in globs

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`  
**Commit:** `24d356e`  
**Applied fix:** Added per-segment `escape_glob_literal` / `path_to_exclude_glob_prefix` for exclude globs; added `remove_repo_with_exclude_escapes_glob_metacharacters_in_path` test for `*` in directory names.

## Skipped Issues

_None — all in-scope findings were fixed._

---

_Fixed: 2026-05-30T18:45:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
