---
phase: 03-git-state
plan: 03
subsystem: services + cli
tags: [index-pipeline, git-state, cli-output, humantime, IndexSummary]

# Dependency graph
requires:
  - phase: 03-git-state
    plan: 01
    provides: "git2 dependency, open_and_query, GitState domain struct, migration 003"
  - phase: 03-git-state
    plan: 02
    provides: "services/git_state.rs with refresh_all, RepoRecord six git fields, catalog::list_repos extended"
provides:
  - "IndexSummary.git_refreshed and IndexSummary.git_errors fields populated by index run (D-17)"
  - "index::run_full second pass: queries all non-excluded paths, calls git_state::refresh_all, writes batch UPDATE in separate git_tx"
  - "workpot index output: 'index: +N -M skipped K / git: P refreshed, Q errors'"
  - "workpot repo list output: name  path  branch  dirty/clean/N/A  [↑N↓N]  Xm ago"
  - "format_git_state helper: D-06 never-refreshed='?', D-09 error='ERROR:', D-13 bare='N/A', D-04 no ahead/behind when no upstream"
  - "format_age helper: humantime::format_duration for staleness age (D-07)"
  - "pub use RepoRecord in workpot-core public API surface"
affects: [04-tray-finder]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Git refresh second pass: rayon collect completes before opening git_tx (Pitfall 6 — no cross-thread DB borrow)"
    - "is_dirty stored as Option<i64>: None=NULL, Some(true)=1, Some(false)=0 — cast via .map(|b| b as i64)"
    - "refresh_time set once before loop — same timestamp for all repos in batch"
    - "format_git_state uses let-else for git_refreshed_at: None -> early return '?'"
    - "ahead/behind uses Unicode arrows: ↑ = \\u{2191}, ↓ = \\u{2193} — matches D-04 spec"

key-files:
  created: []
  modified:
    - crates/workpot-core/src/services/index.rs
    - crates/workpot-core/src/lib.rs
    - crates/workpot-cli/src/main.rs

key-decisions:
  - "pub use RepoRecord added to workpot-core public API: lib.rs had a private local `use crate::domain::{Config, RepoRecord}` but no pub re-export; format_git_state function signature requires it at the CLI boundary"
  - "git_tx is a separate transaction from the discovery tx per Pitfall 6: rayon::collect must complete before any DB borrow, and keeping separate transactions avoids long-held locks during parallel git2 operations"
  - "git stats (git_refreshed, git_errors) are NOT stored in index_runs table per RESEARCH.md deferred decision — output to CLI only; finish_index_run signature unchanged"

patterns-established:
  - "Two-pass index: discovery tx (upsert/delete) completes first, then git refresh tx writes git state columns — clean separation of concerns"
  - "format_git_state pattern: guard-clauses for special cases (never-refreshed, error) then normal path"

requirements-completed: [GIT-01, GIT-02, GIT-03, GIT-04]

# Metrics
duration: 8min
completed: 2026-05-29
---

# Phase 3 Plan 03: Index Integration and CLI Git State Output Summary

**Git refresh second pass wired into index pipeline and CLI repo list extended with branch/dirty/ahead-behind/age per repo — completes Phase 3 observable behavior**

## Performance

- **Duration:** ~8 min
- **Started:** ~2026-05-29T20:54:00Z
- **Completed:** 2026-05-29T21:02:03Z
- **Tasks:** 2 (Task 3 is a human-verify checkpoint — returned below)
- **Files modified:** 3

## Accomplishments

- Extended IndexSummary with `git_refreshed: u32` and `git_errors: u32` (D-17)
- Added `use crate::services::git_state` import to index.rs
- Added git refresh second pass in `run_full_inner` after discovery `tx.commit()`:
  - Queries all non-excluded repo paths from DB
  - Calls `git_state::refresh_all(all_paths)` (rayon parallel, outside any DB borrow)
  - Counts git_refreshed / git_errors from results
  - Writes batch UPDATE in separate `git_tx` (Pitfall 6 compliance)
  - `refresh_time` set once before loop (consistent timestamp across batch)
  - `is_dirty` stored as `Option<i64>` (None=NULL, 1=dirty, 0=clean)
- Updated `workpot index` CLI output: `"index: +N -M skipped K / git: P refreshed, Q errors"`
- Added `format_git_state(repo: &workpot_core::RepoRecord) -> String` helper:
  - None git_refreshed_at → `"?"` (D-06)
  - Some(err) git_state_error → `"ERROR: {err}"` (D-09)
  - Normal: `"{branch}  {dirty/clean/N/A}[  ↑N↓N]  {age}"`
- Added `format_age(git_refreshed_at: i64) -> String` using `humantime::format_duration` (D-07)
- Updated `workpot repo list` handler: `println!("{}  {}  {}", repo.name, repo.path.display(), format_git_state(&repo))`
- Added `pub use crate::domain::RepoRecord` to workpot-core public API (required for CLI function signature)
- All 63 workspace tests pass (55 workpot-core + 8 workpot-cli); cargo build --workspace exits 0

## Task Commits

1. **Task 1: Extend IndexSummary and add git refresh second pass** - `f4dca28`
2. **Task 2: Update workpot index and repo list CLI output for git state** - `4edff96`

## Files Created/Modified

- `crates/workpot-core/src/services/index.rs` - Added git_refreshed/git_errors to IndexSummary; added git refresh second pass with separate git_tx
- `crates/workpot-core/src/lib.rs` - Added `pub use crate::domain::RepoRecord`; removed duplicate private import
- `crates/workpot-cli/src/main.rs` - Extended index output line; updated repo list handler; added format_git_state and format_age helpers

## Decisions Made

- **pub use RepoRecord:** The plan said to use `workpot_core::RepoRecord` in the format_git_state function signature. lib.rs had a private import of RepoRecord but no pub re-export. Rather than using a domain-path workaround (`workpot_core::domain::repo::RepoRecord`), added `pub use crate::domain::RepoRecord` to the public API surface — this is the correct pattern matching how `GitState` was exported in Plan 01.

- **Separate git_tx design:** Placing the rayon::collect + git batch UPDATE after the discovery tx.commit() ensures no cross-thread DB borrow. The discovery and git refresh are logically independent operations. Keeping them in separate transactions also means a git refresh failure doesn't roll back the discovery work.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Added pub use RepoRecord to workpot-core public API**
- **Found during:** Task 2 (cargo build failed with E0603: RepoRecord is private)
- **Issue:** `format_git_state(repo: &workpot_core::RepoRecord)` in main.rs requires `RepoRecord` to be in workpot-core's public API. lib.rs only had a private `use crate::domain::{Config, RepoRecord}` — no pub re-export.
- **Fix:** Added `pub use crate::domain::RepoRecord;` and removed `RepoRecord` from the private import (changed to `use crate::domain::Config;`)
- **Files modified:** `crates/workpot-core/src/lib.rs`
- **Verification:** `cargo build --workspace` exits 0; `cargo test --workspace` all pass
- **Committed in:** 4edff96 (Task 2 commit)

## Known Stubs

None — all git state columns are wired from real git2 queries via the second pass. format_git_state reads live DB data.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. Batch UPDATE uses parameterized rusqlite::params![] (T-03-07 mitigation confirmed). Path traversal protection from Plan 01 (T-03-04) applies as paths flow through refresh_git_state canonicalize before reaching git2.

## Verification Results

- `cargo test --workspace` exits 0 (all 63 tests pass, 1 ignored — perf scaffold)
- `cargo build --workspace` exits 0
- `grep -r "Command::new" crates/workpot-core/src --include="*.rs"` returns zero matches (D-02 confirmed)
- `grep -v '^[[:space:]]*//' crates/workpot-core/src/services/index.rs | grep -c "git_refreshed"` returns 3
- IndexSummary contains `pub git_refreshed: u32` and `pub git_errors: u32`
- index.rs contains `git_state::refresh_all`, `git_refreshed_at=?5`, `git_tx.commit()`
- main.rs contains `fn format_git_state`, `fn format_age`, `humantime::format_duration`, `git: {} refreshed, {} errors`

## Checkpoint Pending

**Task 3 (checkpoint:human-verify)** has been surfaced to the user for end-to-end verification. See checkpoint message below.

## Self-Check

### Files exist:
- [x] `crates/workpot-core/src/services/index.rs` contains `pub git_refreshed: u32`
- [x] `crates/workpot-core/src/services/index.rs` contains `pub git_errors: u32`
- [x] `crates/workpot-core/src/services/index.rs` contains `git_state::refresh_all`
- [x] `crates/workpot-core/src/services/index.rs` contains `git_refreshed_at=?5`
- [x] `crates/workpot-core/src/services/index.rs` contains `git_tx.commit()`
- [x] `crates/workpot-core/src/lib.rs` contains `pub use crate::domain::RepoRecord`
- [x] `crates/workpot-cli/src/main.rs` contains `fn format_git_state`
- [x] `crates/workpot-cli/src/main.rs` contains `fn format_age`
- [x] `crates/workpot-cli/src/main.rs` contains `humantime::format_duration`
- [x] `crates/workpot-cli/src/main.rs` contains `git: {} refreshed, {} errors`
- [x] `crates/workpot-cli/src/main.rs` contains `format_git_state(&repo)` in list handler

### Commits exist:
- [x] f4dca28 - feat(03-03): extend IndexSummary with git stats and add git refresh second pass
- [x] 4edff96 - feat(03-03): extend CLI output with git state per repo

## Self-Check: PASSED

---
*Phase: 03-git-state*
*Completed: 2026-05-29*
