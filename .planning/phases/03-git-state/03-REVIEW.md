---
phase: 03-git-state
reviewed: 2026-05-30T12:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/workpot-core/src/domain/git_state.rs
  - crates/workpot-core/src/infra/migrations/003_git_state.sql
  - crates/workpot-core/src/infra/migrations/004_repos_source_index.sql
  - crates/workpot-core/Cargo.toml
  - crates/workpot-cli/Cargo.toml
  - crates/workpot-core/src/domain/mod.rs
  - crates/workpot-core/src/infra/git.rs
  - crates/workpot-core/src/infra/migrations.rs
  - crates/workpot-core/tests/bootstrap_test.rs
  - crates/workpot-core/src/services/git_state.rs
  - crates/workpot-core/tests/git_state_test.rs
  - crates/workpot-core/tests/git_state_perf_test.rs
  - crates/workpot-core/src/domain/repo.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/index.rs
  - crates/workpot-cli/src/main.rs
findings:
  critical: 0
  warning: 3
  info: 1
  total: 4
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-05-30
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Re-review after fix pass. All twelve prior findings (CR-01..CR-02, WR-01..WR-06, IN-01..IN-04) are verified fixed in the current codebase: cap-exceeded audit logging is centralized in `run_full`, numeric casts use `try_from`, double-canonicalize removed, `format_age` guards non-positive timestamps, `log::warn!` replaces `eprintln!`, `refresh_and_persist_git_state` added, constants extracted, source index migration 004 landed, and `connection()` is `pub(crate)`.

Three new warnings remain: `detect_ahead_behind` failures are indistinguishable from "no upstream", `log::warn!` calls are no-ops without a CLI logger backend (regression from prior `eprintln!` behavior), and audit-log INSERT failures are silently dropped. One info item notes the batch git write-back does not verify row match count.

---

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: `detect_ahead_behind` failures silently masquerade as "no upstream"

**File:** `crates/workpot-core/src/infra/git.rs:100`

**Issue:** After a successful dirty check, `open_and_query` calls `detect_ahead_behind(&repo).unwrap_or((None, None))`. Any git2 error (missing objects in shallow clone, corrupt ref, `graph_ahead_behind` failure) is converted to `(None, None)` with `error` left `None`. The CLI then omits ahead/behind entirely — identical to the legitimate D-04 "no upstream configured" case. Users cannot tell a repo query failed from a repo that simply lacks tracking.

**Fix:** Propagate ahead/behind failures into `GitState.error` (same pattern as `detect_dirty`):

```rust
let (ahead, behind) = match detect_ahead_behind(&repo) {
    Ok(pair) => pair,
    Err(e) => {
        return Ok(GitState {
            branch,
            is_dirty,
            ahead: None,
            behind: None,
            error: Some(e.to_string()),
        });
    }
};
```

---

### WR-02: `log::warn!` diagnostics are dropped — CLI never initializes a logger

**Files:**
- `crates/workpot-core/src/infra/git.rs:52`
- `crates/workpot-core/src/services/index.rs:76`, `200`, `236`
- `crates/workpot-cli/src/main.rs` (no logger init)

**Issue:** WR-04 fix replaced `eprintln!` with `log::warn!`, but neither `workpot-cli` nor `workpot-core` registers a log backend (`env_logger`, `tracing-subscriber`, etc.). Per the `log` crate contract, macros are no-ops when no logger is set. Skipped watch roots, unavailable git repos during index, and skipped worktrees produce zero user-visible output — a regression from the previous stderr behavior.

**Fix:** Initialize a simple logger in the CLI entry point:

```toml
# crates/workpot-cli/Cargo.toml
env_logger = "0.11"
```

```rust
// crates/workpot-cli/src/main.rs — top of main()
fn main() -> ExitCode {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).try_init();
    // ...
}
```

Alternatively, propagate skip counts into `IndexSummary` so `workpot index` prints them regardless of logging.

---

### WR-03: Audit-log INSERT failures silently ignored in `run_full` error paths

**File:** `crates/workpot-core/src/services/index.rs:37-42`

**Issue:** Both error arms use `let _ = record_*_run(...)`, discarding DB write failures. If the SQLite file is read-only, full, or corrupted, the user still receives `IndexCapExceeded` or the original error, but no row is written to `index_runs`. Operational history becomes incomplete with no indication to the caller.

**Fix:** Log audit failures at minimum; optionally surface as a chained warning:

```rust
Err(WorkpotError::IndexCapExceeded { projected, max }) => {
    if let Err(e) = record_cap_exceeded_run(conn, started_at, i64::from(projected), max) {
        log::warn!("failed to record cap-exceeded audit row: {e}");
    }
    Err(WorkpotError::IndexCapExceeded { projected, max })
}
```

---

## Info

### IN-01: Batch git write-back does not verify `rows_affected`

**File:** `crates/workpot-core/src/services/index.rs:172-186`

**Issue:** The post-refresh UPDATE loop increments `summary.git_refreshed` based on in-memory results before writing to SQLite, and never checks whether `execute` matched a row. A path-key mismatch (e.g., legacy non-canonical path in DB) would report success in the summary while leaving the DB row stale.

**Fix:** Check `git_tx.execute(...)?` return value; increment `git_errors` (or a new `git_skipped` counter) when `rows_affected == 0`:

```rust
let updated = git_tx.execute(/* ... */)?;
if updated == 0 {
    log::warn!("git refresh: no repo row matched path {}", r.path);
}
```

---

## Prior Findings — Verified Fixed

| ID | Status |
|----|--------|
| CR-01 | Fixed — cap-exceeded audit in outer `run_full` match only |
| CR-02 | Fixed — `u32::try_from` / `i64::try_from` with `MAX` fallback |
| WR-01 (prior) | Fixed — single canonicalize in service layer; `debug_assert` in infra |
| WR-02 (prior) | Fixed — checked `usize → i64` conversion for ahead/behind |
| WR-03 (prior) | Fixed — `format_age` returns `"unknown"` for `<= 0` |
| WR-04 (prior) | Fixed — `log::warn!` (see new WR-02 for backend gap) |
| WR-05 (prior) | Fixed — `refresh_and_persist_git_state` + docs on read-only API |
| WR-06 (prior) | Fixed — `started_at` removed from `finish_index_run` |
| IN-01 (prior) | Fixed — dead `modify_and_commit` helper removed |
| IN-02 (prior) | Fixed — `SOURCE_*` / `BRANCH_UNBORN` constants |
| IN-03 (prior) | Fixed — migration 004 `idx_repos_source_excluded` |
| IN-04 (prior) | Fixed — `connection()` is `pub(crate)` |

---

_Reviewed: 2026-05-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
