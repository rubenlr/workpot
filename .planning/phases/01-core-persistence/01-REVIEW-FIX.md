---
phase: 01-core-persistence
fixed_at: 2026-05-30T23:45:00Z
review_path: .planning/phases/01-core-persistence/01-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-05-30T23:45:00Z  
**Source review:** `.planning/phases/01-core-persistence/01-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 2
- Fixed: 2
- Skipped: 0

## Fixed Issues

### IN-01: No regression test for SQL LIKE escape on repo basename

**Files modified:** `crates/workpot-core/tests/catalog_test.rs`  
**Commit:** ba7889e  
**Applied fix:** Added `remove_repo_by_basename_with_like_metacharacters_in_name` — repo dir `foo%bar`, register, delete directory, `remove_repo` by basename only; asserts empty list and zero rows in DB.

### IN-02: `upsert_scan` does not enforce `max_repos`

**Files modified:** `crates/workpot-core/src/services/catalog.rs`  
**Commit:** ef5a890  
**Applied fix:** Doc-comment on `upsert_scan` stating cap enforcement is the caller's responsibility (`index::run_full` projects count and returns `IndexCapExceeded`). No cap logic added per Phase 1 scope.

---

_Fixed: 2026-05-30T23:45:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
