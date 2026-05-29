---
phase: 01-core-persistence
fixed_at: 2026-05-30T21:30:00Z
review_path: .planning/phases/01-core-persistence/01-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-05-30T21:30:00Z
**Source review:** `.planning/phases/01-core-persistence/01-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 2
- Fixed: 2
- Skipped: 0

**Tests:** `cargo test -p workpot-core` — all passed (59 run, 1 ignored)

## Fixed Issues

### WR-01: Basename lookup — SQL LIKE metacharacters

**Files modified:** `crates/workpot-core/src/services/catalog.rs`
**Commit:** `f721a5c`
**Applied fix:** Added `escape_like()` helper and updated `resolve_repo_path_key` to escape `%`, `_`, and `\` in basename before building the suffix pattern; SQL now uses `path LIKE '%' || ?2 ESCAPE '\\'`.

### IN-01: Regression test for stale config.tmp cleanup

**Files modified:** `crates/workpot-core/tests/bootstrap_test.rs`
**Commit:** `5f66826`
**Applied fix:** Added `open_removes_stale_config_tmp` integration test — seeds valid `config.toml` plus orphan `config.tmp`, opens via `AppContext::open_with_paths`, asserts tmp file is removed.

---

_Fixed: 2026-05-30T21:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
