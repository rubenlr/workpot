---
phase: 03-git-state
reviewed: 2026-05-30T00:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - crates/workpot-core/src/domain/git_state.rs
  - crates/workpot-core/src/infra/migrations/003_git_state.sql
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
  critical: 2
  warning: 6
  info: 4
  total: 12
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-05-30
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found

## Summary

The git-state layer introduces `GitState` domain type, `infra/git.rs` (git2-based querying), `services/git_state.rs` (single + batch refresh), migration 003, and CLI display. Overall the design is sound: path canonicalization for traversal mitigation is applied consistently, bare repos and detached HEAD are handled explicitly, and the rayon parallel refresh correctly avoids sharing a `Repository` across threads.

Two blockers were found: a silent numeric truncation on large repo counts (i64 → u32 without bounds check), and an `IndexCapExceeded` error that bypasses the audit-log path in `run_full` while still claiming it records one. Six warnings cover a double-canonicalize in the service layer, `usize → i64` truncation for ahead/behind counts on extremely diverged branches, a negative timestamp cast that panics at runtime, `eprintln!` calls in library code, a dead function parameter, and a missing `git_state_error` reset when a subsequent refresh succeeds. Four info items cover dead code, a magic string, and a missing index.

---

## Critical Issues

### CR-01: `IndexCapExceeded` skips audit-log write in `run_full`

**File:** `crates/workpot-core/src/services/index.rs:34-44`

**Issue:** `run_full` has two paths for recording a failed run:
- `record_cap_exceeded_run` is called inside `run_full_inner` (line 97) before returning `Err(IndexCapExceeded)`.
- Back in `run_full`, the outer `match` arm for `IndexCapExceeded` (lines 36-39) re-raises the error **without** calling `record_error_run` — which is correct, because `record_cap_exceeded_run` already ran.

The bug is the inverse: any other error propagated out of `run_full_inner` (line 40-43) calls `record_error_run`, but `run_full_inner` **also** calls `record_cap_exceeded_run` with its own `INSERT` **before** returning. If `record_cap_exceeded_run` itself fails (e.g., the DB is full), `run_full_inner` propagates that DB error, not `IndexCapExceeded`. The outer `run_full` then calls `record_error_run` a second time for that DB error. This creates a duplicate audit-log row for a single index run.

More critically: the `IndexCapExceeded` match arm silently reconstructs the error without logging it at all at the outer level. If the inner `record_cap_exceeded_run` INSERT fails (returns `Err`), that error replaces `IndexCapExceeded` and travels to the `Err(e)` arm, which calls `record_error_run`. The cap-exceeded condition is then silently swallowed — the audit log shows a generic DB error, not a cap-exceeded event, and the CLI exits with code 1 showing a DB error message instead of the cap message.

**Fix:** Remove `record_cap_exceeded_run` from inside `run_full_inner`; let `run_full` handle all audit logging after inspecting the returned error variant:

```rust
pub fn run_full(conn: &Connection, config: &Config) -> Result<IndexSummary> {
    let started_at = now_secs();
    match run_full_inner(conn, config, started_at) {
        Ok(summary) => Ok(summary),
        Err(WorkpotError::IndexCapExceeded { projected, max }) => {
            let _ = record_cap_exceeded_run(conn, started_at, projected as i64, max);
            Err(WorkpotError::IndexCapExceeded { projected, max })
        }
        Err(e) => {
            let _ = record_error_run(conn, started_at, &e);
            Err(e)
        }
    }
}
```

Remove the `record_cap_exceeded_run(conn, started_at, projected, max_repos)?;` call at line 97 inside `run_full_inner`.

---

### CR-02: `i64 as u32` truncation when projected repo count exceeds `u32::MAX`

**File:** `crates/workpot-core/src/services/index.rs:99`

**Issue:** `projected_repo_count` returns `i64`. The conversion `projected as u32` silently wraps if `projected > 4_294_967_295`. While that is an astronomical number of repos, the code also uses `paths.len() as i64` (line 335) which itself wraps on 32-bit targets. More practically, a malicious or corrupted DB could produce a large `projected` value and the `as u32` cast makes the error message show a nonsensical (wrapped) count. There is no `>= 0` guard either — a bug causing `projected` to go negative would wrap to a large `u32` and pass the cap check.

**Fix:**

```rust
// After the projected > i64::from(max_repos) check:
let projected_u32 = u32::try_from(projected).unwrap_or(u32::MAX);
return Err(WorkpotError::IndexCapExceeded {
    projected: projected_u32,
    max: max_repos,
});
```

And in `projected_repo_count`:

```rust
Ok(i64::try_from(paths.len()).unwrap_or(i64::MAX))
```

---

## Warnings

### WR-01: Double-canonicalize in `refresh_git_state` → `open_and_query`

**File:** `crates/workpot-core/src/services/git_state.rs:17-22` and `crates/workpot-core/src/infra/git.rs:65-67`

**Issue:** `refresh_git_state` canonicalizes the path (lines 18-20) and then passes the result to `open_and_query` (line 21). `open_and_query` canonicalizes the already-canonical path a second time (lines 65-67). The second `canonicalize` call is wasted I/O (a syscall per refresh), but more importantly, if the first succeeded and the second fails (e.g., a symlink was removed between the two calls), `open_and_query` returns `WorkpotError::GitUnavailable` referencing the already-canonical path without the git2 error context, silently discarding the more informative error produced at line 70.

**Fix:** Either document that `open_and_query` requires a pre-canonicalized path and remove the redundant `canonicalize` from `refresh_git_state`, or remove the `canonicalize` from `open_and_query` and guarantee all call sites pre-canonicalize. The latter is safer:

```rust
// services/git_state.rs — keep canonicalize here, pass canonical path directly
pub fn refresh_git_state(path: &Path) -> Result<GitState> {
    let canonical = path
        .canonicalize()
        .map_err(|_| crate::error::WorkpotError::GitUnavailable(path.to_path_buf()))?;
    crate::infra::git::open_and_query(&canonical)
}

// infra/git.rs open_and_query — remove the inner canonicalize, add a debug_assert
pub fn open_and_query(path: &Path) -> Result<GitState> {
    debug_assert!(path.is_absolute(), "open_and_query requires an absolute canonical path");
    let repo = Repository::open(path)
        .map_err(|e| WorkpotError::GitUnavailable(format!("{}: {e}", path.display()).into()))?;
    // ... rest unchanged
}
```

---

### WR-02: `usize as i64` truncation for `ahead`/`behind` on 64-bit (silent on large divergence)

**File:** `crates/workpot-core/src/infra/git.rs:183`

**Issue:** `repo.graph_ahead_behind` returns `(usize, usize)`. The cast `ahead as i64` is defined behavior on all current targets (usize ≤ i64::MAX on 64-bit), but on a theoretical 128-bit future target or if a repo somehow accumulates more than `i64::MAX` commits ahead, this silently wraps. More pragmatically, `usize::MAX as i64` is `-1` on 64-bit, which would be stored in the DB as `-1` and displayed as `↑-1↓-1` in the CLI.

**Fix:** Use checked conversion to surface unexpected values:

```rust
let ahead_i64 = i64::try_from(ahead).unwrap_or(i64::MAX);
let behind_i64 = i64::try_from(behind).unwrap_or(i64::MAX);
Ok((Some(ahead_i64), Some(behind_i64)))
```

---

### WR-03: Negative `git_refreshed_at` causes panic in `format_age`

**File:** `crates/workpot-cli/src/main.rs:155`

**Issue:** `git_refreshed_at` is stored as `i64` in the DB and read as `i64` in `RepoRecord`. The value can legitimately be `0` (from the `unwrap_or(0)` fallback in `now_secs()`), or corrupted to be negative. `format_age` casts it to `u64`:

```rust
let refreshed = UNIX_EPOCH + Duration::from_secs(git_refreshed_at as u64);
```

A negative `i64` cast to `u64` wraps to a large value (e.g., `-1i64 as u64 == u64::MAX`). `UNIX_EPOCH + Duration::from_secs(u64::MAX)` does not panic (Duration arithmetic is saturating in std), but it produces a nonsensical age string like "584542046090 years". If the DB row was written with `unwrap_or(0)` fallback, `0 as u64 == 0` is fine, but the `None` guard in `format_git_state` (line 163) covers the `git_refreshed_at = None` case, not `git_refreshed_at = Some(0)`. A `Some(0)` timestamp would display "55 years ago" instead of "never refreshed".

**Fix:**

```rust
fn format_age(git_refreshed_at: i64) -> String {
    if git_refreshed_at <= 0 {
        return "unknown".to_string();
    }
    let refreshed = UNIX_EPOCH + Duration::from_secs(git_refreshed_at as u64);
    // ... rest unchanged
}
```

---

### WR-04: `eprintln!` in library code — no logging abstraction

**Files:**
- `crates/workpot-core/src/infra/git.rs:52`
- `crates/workpot-core/src/services/index.rs:77`, `204`, `240`

**Issue:** Library crates must not write directly to stderr. Callers (CLI, future tray) cannot suppress, redirect, or format these messages. The four `eprintln!` calls in `workpot-core` bypass any structured logging or error reporting the consumer might configure. This is a well-established Rust convention violation: libraries should use the `log` crate facade or propagate warnings through the return type.

**Fix:** Either add the `log` crate as a dependency and replace `eprintln!` with `log::warn!`, or thread an optional diagnostic callback into the affected functions. The `log` crate approach is less invasive:

```toml
# Cargo.toml
log = "0.4"
```

```rust
// Replace each eprintln!("warning: ...") with:
log::warn!("skip worktree {}: {e}", wt_path.display());
```

---

### WR-05: `git_state_error` not cleared on successful re-refresh

**File:** `crates/workpot-core/src/services/index.rs:177-191`

**Issue:** The UPDATE statement in the git state write-back unconditionally sets `git_state_error = ?6`. When `r.state.error` is `None` (a successful refresh), `?6` binds to SQL `NULL`, which does clear the error column correctly in this batch path.

However, `AppContext::refresh_git_state` (lib.rs line 127) returns a `GitState` to the caller but **never writes it back to the DB**. That method is documented as "Public API for Phase 4 tray (D-18)." When the tray calls `refresh_git_state` and gets a successful result, `git_state_error` in the DB remains whatever it was from the last failed `run_full`. Any consumer that reads from the DB (e.g., `list_repos`) would still show the stale error. The `AppContext::refresh_git_state` API is incomplete — it queries but does not persist.

**Fix:** Either rename it to make the read-only behavior explicit (`query_git_state`), or add a companion `AppContext::refresh_and_persist_git_state` method that calls the DB UPDATE after a successful query.

---

### WR-06: `started_at` parameter silently unused in `finish_index_run`

**File:** `crates/workpot-core/src/services/index.rs:379`, `395`

**Issue:** `finish_index_run` accepts a `started_at: i64` parameter but suppresses the unused warning with `let _ = started_at;` (line 395). The function was presumably intended to also log `started_at` in the UPDATE, but the SQL only updates `finished_at`. The dead parameter adds noise to every call site and the suppression masks what was likely an omission.

**Fix:** Remove the `started_at` parameter from `finish_index_run` and all three call sites, or actually use it in the SQL if the schema should store it redundantly:

```rust
fn finish_index_run(
    tx: &Transaction<'_>,
    run_id: i64,
    status: &str,
    summary: &IndexSummary,
    message: Option<&str>,
) -> Result<()> {
    // remove started_at from params list
    ...
}
```

---

## Info

### IN-01: Dead helper `modify_and_commit` in test file

**File:** `crates/workpot-core/tests/git_state_test.rs:49-69`

**Issue:** `modify_and_commit` is marked `#[allow(dead_code)]` but is never called by any test in the file. The `#[allow(dead_code)]` attribute is a suppression of a compiler warning that should instead prompt removal.

**Fix:** Remove the function, or add a test that exercises the "re-commit after modification" path to justify its existence.

---

### IN-02: `"manual"` / `"scan"` / `"unborn"` are magic strings with no shared constant

**Files:**
- `crates/workpot-core/src/services/catalog.rs:48`, `67`, `207`, `210`
- `crates/workpot-core/src/services/index.rs:210`
- `crates/workpot-core/src/infra/git.rs:117`

**Issue:** The literal strings `"manual"`, `"scan"`, and `"unborn"` appear in multiple places across services, infra, and SQL. If one is changed (e.g., the DB CHECK constraint in 001_init.sql) without updating all uses, queries silently stop matching or inserts fail at runtime.

**Fix:** Define `const SOURCE_MANUAL: &str = "manual";` and `const SOURCE_SCAN: &str = "scan";` in `domain/repo.rs` or a new `domain/constants.rs` and import them at each use site.

---

### IN-03: No index on `repos(source)` for stale-path collection query

**File:** `crates/workpot-core/src/infra/migrations/003_git_state.sql` (related: `001_init.sql`)

**Issue:** `collect_stale_scan_paths` queries `SELECT path FROM repos WHERE source = 'scan' AND excluded = 0`. The existing indexes are `idx_repos_registered_at` (on `registered_at`) and `idx_repos_git_common_dir` (on `git_common_dir`). There is no index on `(source, excluded)`, so this is a full table scan on every `run_full`. For the v1 target of ~100 repos this is negligible, but the index is cheap to add.

**Fix:** Add to a future migration (not necessarily 003):

```sql
CREATE INDEX idx_repos_source_excluded ON repos(source, excluded);
```

---

### IN-04: `AppContext::connection` exposes raw `&Connection` to callers

**File:** `crates/workpot-core/src/lib.rs:106-108`

**Issue:** `pub fn connection(&self) -> &Connection` gives any caller (including the CLI and future tray) direct access to the database connection, bypassing all service-layer invariants (caps, validation, audit logging). The method exists because `services/roots.rs` and integration tests need it internally, but it is `pub` to the entire crate API.

**Fix:** Change to `pub(crate)` to restrict access to within the crate:

```rust
pub(crate) fn connection(&self) -> &Connection {
    &self.conn
}
```

External callers that need DB access should use the `AppContext` service methods.

---

_Reviewed: 2026-05-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
