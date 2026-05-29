---
phase: 03-git-state
fixed_at: 2026-05-30T00:37:00Z
review_path: .planning/phases/03-git-state/03-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 7
skipped: 5
status: partial
---

# Phase 03: Code Review Fix Report

**Fixed at:** 2026-05-30
**Source review:** `.planning/phases/03-git-state/03-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 12
- Fixed: 7
- Skipped: 5 (already applied in source before this fix pass)

## Fixed Issues

### IN-03: No index on `repos(source)` for stale-path collection query

**Files modified:** `crates/workpot-core/src/infra/migrations/004_repos_source_index.sql`, `crates/workpot-core/src/infra/migrations.rs`, `crates/workpot-core/tests/bootstrap_test.rs`
**Commit:** fa0ce8f
**Applied fix:** Added migration 004 creating `idx_repos_source_excluded`; updated bootstrap test to expect user_version 4.

### WR-04: `eprintln!` in library code — no logging abstraction

**Files modified:** `crates/workpot-core/Cargo.toml`, `Cargo.lock`, `crates/workpot-core/src/infra/git.rs`, `crates/workpot-core/src/services/index.rs`
**Commit:** b72cb6a
**Applied fix:** Added `log = "0.4"` dependency; replaced four `eprintln!` calls with `log::warn!`.

### WR-06: `started_at` parameter silently unused in `finish_index_run`

**Files modified:** `crates/workpot-core/src/services/index.rs`
**Commit:** fb9b116
**Applied fix:** Removed unused `started_at` parameter from `finish_index_run` and its call site.

### WR-05: `git_state_error` not cleared on successful re-refresh

**Files modified:** `crates/workpot-core/src/services/git_state.rs`, `crates/workpot-core/src/lib.rs`
**Commit:** d2494a3
**Applied fix:** Added `persist_git_state`, `refresh_and_persist` service functions, and `AppContext::refresh_and_persist_git_state`; documented read-only behavior of `refresh_git_state`.

### IN-01: Dead helper `modify_and_commit` in test file

**Files modified:** `crates/workpot-core/tests/git_state_test.rs`
**Commit:** 63f136d (import only; removal landed in a80a43b amend with IN-04)
**Applied fix:** Removed unused `modify_and_commit` test helper.

### IN-02: `"manual"` / `"scan"` / `"unborn"` are magic strings with no shared constant

**Files modified:** `crates/workpot-core/src/domain/repo.rs`, `crates/workpot-core/src/domain/mod.rs`, `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/src/services/index.rs`, `crates/workpot-core/src/services/roots.rs`, `crates/workpot-core/src/infra/git.rs`, `crates/workpot-core/tests/git_state_test.rs`
**Commit:** ff2fa0b
**Applied fix:** Added `SOURCE_MANUAL`, `SOURCE_SCAN`, `BRANCH_UNBORN` constants; parameterized SQL queries and catalog inserts to use them.

### IN-04: `AppContext::connection` exposes raw `&Connection` to callers

**Files modified:** `crates/workpot-core/src/lib.rs`, `crates/workpot-core/tests/excludes_test.rs`
**Commit:** a80a43b
**Applied fix:** Changed `connection()` to `pub(crate)`; updated integration test to use `run_index()` instead of direct connection access.

## Skipped Issues

### CR-01: `IndexCapExceeded` skips audit-log write in `run_full`

**File:** `crates/workpot-core/src/services/index.rs:34-44`
**Reason:** Already applied — `run_full` handles cap-exceeded audit logging in the outer match; `run_full_inner` no longer calls `record_cap_exceeded_run`.
**Original issue:** Duplicate or missing audit-log row when cap-exceeded path fails mid-write.

### CR-02: `i64 as u32` truncation when projected repo count exceeds `u32::MAX`

**File:** `crates/workpot-core/src/services/index.rs:99`
**Reason:** Already applied — uses `u32::try_from(projected).unwrap_or(u32::MAX)` and `i64::try_from(paths.len()).unwrap_or(i64::MAX)`.
**Original issue:** Silent numeric truncation on large repo counts.

### WR-01: Double-canonicalize in `refresh_git_state` → `open_and_query`

**File:** `crates/workpot-core/src/services/git_state.rs:17-22`
**Reason:** Already applied — `open_and_query` uses `debug_assert!(path.is_absolute())` and no inner `canonicalize`; service layer canonicalizes once.
**Original issue:** Redundant syscall and error-context loss from double canonicalization.

### WR-02: `usize as i64` truncation for `ahead`/`behind`

**File:** `crates/workpot-core/src/infra/git.rs:183`
**Reason:** Already applied — uses `i64::try_from(ahead).unwrap_or(i64::MAX)` and same for `behind`.
**Original issue:** Silent wrap on extreme divergence counts.

### WR-03: Negative `git_refreshed_at` causes panic in `format_age`

**File:** `crates/workpot-cli/src/main.rs:155`
**Reason:** Already applied — `format_age` returns `"unknown"` when `git_refreshed_at <= 0`.
**Original issue:** Nonsensical age strings for zero/negative timestamps.

---

_Fixed: 2026-05-30_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
