---
phase: 01-core-persistence
reviewed: 2026-05-30T22:30:00Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - .github/workflows/ci.yml
  - .gitignore
  - Cargo.toml
  - crates/workpot-cli/Cargo.toml
  - crates/workpot-cli/src/main.rs
  - crates/workpot-core/Cargo.toml
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/domain/mod.rs
  - crates/workpot-core/src/domain/repo.rs
  - crates/workpot-core/src/error.rs
  - crates/workpot-core/src/infra/migrations.rs
  - crates/workpot-core/src/infra/migrations/001_init.sql
  - crates/workpot-core/src/infra/mod.rs
  - crates/workpot-core/src/infra/paths.rs
  - crates/workpot-core/src/infra/store.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/tests/bootstrap_test.rs
  - crates/workpot-core/tests/catalog_test.rs
  - crates/workpot-core/tests/paths_test.rs
  - rust-toolchain.toml
  - scripts/check-no-network-deps.sh
findings:
  critical: 0
  warning: 0
  info: 2
  total: 2
status: clean
---

# Phase 1: Code Review Report

**Reviewed:** 2026-05-30T22:30:00Z  
**Depth:** standard  
**Files Reviewed:** 23  
**Status:** clean

## Summary

Fifth pass on Phase 1 core-persistence. Both open items from the fourth pass are **fixed** in current code: SQL `LIKE` metacharacters in deleted-repo basename lookup (`escape_like` + `ESCAPE '\\'` + Rust `file_name` filter), and a regression test for stale `config.tmp` cleanup (`open_removes_stale_config_tmp`).

Phase 1 persistence posture remains sound: parameterized SQL, path canonicalization on register, atomic config writes (temp + fsync + rename), WAL + busy timeout, macOS offline CI with banned-network dependency gate, locked macOS path layout.

Later-phase code in `catalog.rs` (`upsert_scan`, `remove_repo_with_exclude`, git columns in `list_repos`) was reviewed for correctness/security; no blockers found. Index-time `max_repos` enforcement lives in `services/index.rs` (Phase 2), not in `upsert_scan` alone.

No BLOCKER or WARNING findings for Phase 1 scope.

## Prior Finding Verification (fourth pass → fifth pass)

| ID | Status | Evidence |
|----|--------|----------|
| WR-01 (SQL LIKE metacharacters in basename lookup) | ✅ Fixed | `catalog.rs:125-165` `escape_like`, `LIKE ... ESCAPE '\\'`, ambiguity error; `catalog_test.rs:239-268` `foo` vs `foo-extra` |
| IN-01 (no regression test for tmp cleanup) | ✅ Fixed | `bootstrap_test.rs:54-69` `open_removes_stale_config_tmp` |

## Narrative Findings (AI reviewer)

## Info

### IN-01: No regression test for SQL LIKE escape on repo basename

**File:** `crates/workpot-core/src/services/catalog.rs:125-165`  
**Issue:** `escape_like` fixes WR-01, and `remove_repo_by_basename_does_not_match_similar_directory_name` covers suffix collision, but there is no test where the registered directory name contains `%` or `_` and removal uses basename after delete.  
**Fix:** Add a catalog test: repo dir named e.g. `foo%bar`, register, delete directory, `remove_repo` by basename only; assert the correct row is removed.

### IN-02: `upsert_scan` does not enforce `max_repos` (Phase 2+; not a Phase 1 gap)

**File:** `crates/workpot-core/src/services/catalog.rs:288-341`  
**Issue:** `register_manual` checks `max_repos`; `upsert_scan` does not. Direct callers could grow the table without cap.  
**Fix:** Not required for Phase 1 — `index::run_full` already projects count and returns `IndexCapExceeded` before bulk upsert (`services/index.rs`). Document or add a guard in `upsert_scan` only if a future caller bypasses the indexer.

---

_Reviewed: 2026-05-30T22:30:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
